use super::super::cfg_cleanup::masked_integer_comparison;
use super::{Compare, Step, Trace, Window, Word};
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

struct Walk<'a> {
    function: &'a Function,
    aliases: BTreeMap<Value, Value>,
}

impl Walk<'_> {
    fn value(&self, value: Value) -> Value {
        let value = self.function.dfg.resolve_aliases(value);
        self.aliases.get(&value).copied().unwrap_or(value)
    }

    fn result(&self, value: Value) -> Option<&Data> {
        let ValueDef::Result(inst, _) = self.function.dfg.value_def(self.value(value)) else {
            return None;
        };
        Some(&self.function.dfg.insts[inst])
    }

    fn word(&self, value: Value) -> Word {
        let value = self.value(value);
        match self.result(value) {
            Some(Data::UnaryImm {
                opcode: Opcode::Iconst,
                imm,
            }) => Word::Imm((*imm).into()),
            _ => Word::Value(value),
        }
    }

    fn pair(&self, value: Value, opcode: Opcode) -> Option<[Value; 2]> {
        let Data::Binary { opcode: op, args } = self.result(value)? else {
            return None;
        };
        (*op == opcode).then(|| args.map(|v| self.value(v)))
    }

    // Arithmetic here is CLIF's modular integer arithmetic. Return the same
    // predicate sense while recognizing (cursor - width) == cursor as width == 0.
    fn test(&self, value: Value) -> Option<(Compare, Option<Value>)> {
        let instruction = self.result(value)?;
        if let Some(comparison) =
            masked_integer_comparison(self.function, instruction, |value| Some(self.value(value)))
        {
            return self.test(comparison);
        }
        let (cc, left, right) = match instruction {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => return self.test(*arg),
            Data::BinaryImm64 {
                opcode: Opcode::BandImm,
                arg,
                imm,
            } if imm.bits() == 1 && self.function.dfg.value_type(self.value(*arg)) == types::I8 => {
                // Resolve only the current path's incoming value. A masked
                // join parameter may differ on the original exit edge.
                return match self.result(*arg)? {
                    Data::UnaryImm {
                        opcode: Opcode::Iconst,
                        imm,
                    } => Some((
                        Compare::new(IntCC::NotEqual, Word::Imm(imm.bits() & 1), Word::Imm(0)),
                        None,
                    )),
                    _ => None,
                };
            }
            Data::IntCompare { cond, args, .. }
                if self.function.dfg.value_type(args[0]) == types::I64 =>
            {
                (*cond, self.word(args[0]), self.word(args[1]))
            }
            Data::IntCompareImm { cond, arg, imm, .. }
                if self.function.dfg.value_type(*arg) == types::I64 =>
            {
                (*cond, self.word(*arg), Word::Imm((*imm).into()))
            }
            _ => return None,
        };
        let (left, right) =
            if matches!(cc, IntCC::Equal | IntCC::NotEqual) && matches!(left, Word::Imm(_)) {
                (right, left)
            } else {
                (left, right)
            };
        if matches!(cc, IntCC::Equal | IntCC::NotEqual) {
            if let (Word::Value(left), Word::Imm(bit @ (0 | 1))) = (left, right) {
                if let Some((inner, selector)) = self.test(left) {
                    return Some((inner.sense((cc == IntCC::Equal) == (bit == 1)), selector));
                }
            }
            if let (Word::Value(left), Word::Value(right)) = (left, right) {
                for (difference, cursor) in [(left, right), (right, left)] {
                    if self.function.dfg.value_type(difference) != types::I64 {
                        continue;
                    }
                    if let Some([start, width]) = self.pair(difference, Opcode::Isub) {
                        if start == cursor {
                            return Some((
                                Compare::new(cc, self.word(width), Word::Imm(0)),
                                Some(width),
                            ));
                        }
                    }
                }
            }
        }
        Some((Compare::new(cc, left, right), None))
    }

    fn addend(&self, value: Value) -> (Value, i64) {
        self.addend_with_zero(value, None)
    }

    fn addend_with_zero(&self, value: Value, zero: Option<Value>) -> (Value, i64) {
        let value = self.value(value);
        if let Some(Data::BinaryImm64 {
            opcode: Opcode::IaddImm,
            arg,
            imm,
        }) = self.result(value)
        {
            let (base, delta) = self.addend_with_zero(*arg, zero);
            return (base, delta.wrapping_add((*imm).into()));
        }
        for opcode in [Opcode::Iadd, Opcode::Isub] {
            if let Some([a, b]) = self.pair(value, opcode) {
                let right = if zero == Some(b) {
                    Word::Imm(0)
                } else {
                    self.word(b)
                };
                if let Word::Imm(n) = right {
                    let (base, delta) = self.addend_with_zero(a, zero);
                    let n = if opcode == Opcode::Iadd {
                        n
                    } else {
                        n.wrapping_neg()
                    };
                    return (base, delta.wrapping_add(n));
                }
            }
        }
        (value, 0)
    }

    fn vector_address(&self, address: Value) -> Option<(Value, Value, i64)> {
        let [left, right] = self.pair(address, Opcode::Iadd)?;
        for (base, scaled) in [(left, right), (right, left)] {
            if let Some(Data::BinaryImm64 {
                opcode: Opcode::ImulImm,
                arg,
                imm,
            }) = self.result(scaled)
            {
                if i64::from(*imm) == 8 {
                    let (index, delta) = self.addend(*arg);
                    return Some((base, index, delta));
                }
            }
        }
        None
    }

    fn enter(&mut self, block: Block, args: &[Value]) {
        self.aliases.extend(
            self.function
                .dfg
                .block_params(block)
                .iter()
                .copied()
                .zip(args.iter().copied()),
        );
    }
}

fn available(context: &Context, value: Value, at: Inst) -> bool {
    let f = &context.func;
    match f.dfg.value_def(value) {
        ValueDef::Result(inst, _) => inst != at && context.domtree.dominates(inst, at, &f.layout),
        ValueDef::Param(block, _) => context.domtree.dominates(block, at, &f.layout),
        ValueDef::Union(_, _) => false,
    }
}

fn pure(opcode: Opcode) -> bool {
    matches!(
        opcode,
        Opcode::Iconst
            | Opcode::Iadd
            | Opcode::IaddImm
            | Opcode::Isub
            | Opcode::Imul
            | Opcode::ImulImm
            | Opcode::Band
            | Opcode::BandImm
            | Opcode::Bor
            | Opcode::BorImm
            | Opcode::Bxor
            | Opcode::BxorImm
            | Opcode::Ishl
            | Opcode::IshlImm
            | Opcode::Ushr
            | Opcode::UshrImm
            | Opcode::Uextend
            | Opcode::Ireduce
            | Opcode::Icmp
            | Opcode::IcmpImm
    )
}

fn entry_window(
    context: &Context,
    path: &Walk<'_>,
    header: Block,
    trace: &Trace,
    cache_load: Inst,
    previous_load: Inst,
    length: usize,
) -> Option<Window> {
    let f = &context.func;
    let params = f.dfg.block_params(header);
    let first = f.layout.first_inst(header)?;
    let slot = |value| params.iter().position(|&param| param == value);
    let before_load = |load| {
        trace
            .steps
            .iter()
            .position(|step| matches!(step, Step::Copy { inst, .. } if *inst == load))
    };
    let guard = |step: &Step| match step {
        Step::Guard(check) => Some(*check),
        Step::Copy { .. } => None,
    };
    let cache = before_load(cache_load)?;
    let previous = before_load(previous_load)?;
    let mut prefix = trace.steps[..cache].iter().filter_map(guard);
    let Compare {
        cc: IntCC::NotEqual,
        left: Word::Value(count),
        right: Word::Imm(0),
    } = prefix.next()?
    else {
        return None;
    };
    let cache_guard @ Compare {
        cc: IntCC::UnsignedLessThan,
        left: Word::Value(ordinal),
        right: limit,
    } = prefix.next()?
    else {
        return None;
    };
    if prefix.next().is_some() {
        return None;
    }
    if let Word::Value(limit) = limit {
        if slot(limit).is_some()
            || !available(context, limit, first)
            || !available(context, limit, trace.append_term)
        {
            return None;
        }
    }
    let (cursor, -1) = path.addend(ordinal) else {
        return None;
    };
    let count_slot = slot(count)?;
    let cursor_slot = slot(cursor)?;
    if path.addend(trace.next[count_slot]) != (count, -1)
        || path.addend_with_zero(trace.next[cursor_slot], Some(trace.selector)) != (cursor, -1)
    {
        return None;
    }
    let previous_guard @ Compare {
        cc: IntCC::UnsignedLessThan,
        left: Word::Value(previous_ordinal),
        right: Word::Value(current_length),
    } = trace
        .steps
        .get(cache..previous)?
        .iter()
        .rev()
        .find_map(guard)?
    else {
        return None;
    };
    if current_length != params[length] || path.addend(previous_ordinal) != (current_length, -1) {
        return None;
    }
    // The taken empty edge keeps these two unit decrements in step. The
    // existing append witness supplies length + 1 and the stored last word.
    Some(Window {
        count: count_slot,
        cursor: cursor_slot,
        length,
        limit,
        cache_guard,
        previous_guard,
    })
}

pub(super) fn capture(context: &Context, header: Block) -> Result<Trace, &'static str> {
    let f = &context.func;
    let first = f.layout.first_inst(header).ok_or("missing header")?;
    let params = f.dfg.block_params(header);
    if params.is_empty() || params.iter().any(|&v| f.dfg.value_type(v) != types::I64) {
        return Err("unsupported loop state");
    }
    let mut path = Walk {
        function: f,
        aliases: BTreeMap::new(),
    };
    let mut suffix = Walk {
        function: f,
        aliases: BTreeMap::new(),
    };
    let mut defined = params.iter().copied().collect::<BTreeSet<_>>();
    let mut visited = BTreeSet::new();
    let mut steps = Vec::new();
    let mut selected = Vec::<Compare>::new();
    let mut seed_checks = Vec::new();
    let mut loads = Vec::new();
    let mut store = None;
    let mut selector = None;
    let mut block = header;
    let (next, seed) = loop {
        if !visited.insert(block) || visited.len() > 256 {
            return Err("selected path does not close one visit");
        }
        let term = f.layout.last_inst(block).ok_or("empty selected block")?;
        for inst in f.layout.block_insts(block).take_while(|&inst| inst != term) {
            let opcode = f.dfg.insts[inst].opcode();
            let args = f
                .dfg
                .inst_args(inst)
                .iter()
                .map(|&v| path.value(v))
                .collect::<Vec<_>>();
            if args
                .iter()
                .any(|v| !defined.contains(v) && !available(context, *v, first))
            {
                return Err("selected path reads an unavailable operand");
            }
            match opcode {
                // The final read supplies the original complete append word.
                // Packed-load folding can fuse either earlier U32 read.
                _ if store.is_none()
                    && matches!(
                        (opcode, loads.len()),
                        (Opcode::Load, 0..=2) | (Opcode::Uload32, 0..=1)
                    ) =>
                {
                    loads.push((inst, args[0]));
                }
                Opcode::Store if store.is_none() => store = Some((inst, term, args[0], args[1])),
                _ if pure(opcode) => {}
                _ => return Err("selected path has an unsupported effect"),
            }
            defined.extend(f.dfg.inst_results(inst).iter().copied());
            steps.push(Step::Copy { inst, args });
        }
        let edge = match &f.dfg.insts[term] {
            Data::Jump { destination, .. } => destination,
            Data::Brif { arg, blocks, .. } => {
                let (test, difference) = path.test(*arg).ok_or("unrecognized branch")?;
                if test
                    .values()
                    .any(|v| !defined.contains(&v) && !available(context, v, first))
                {
                    return Err("unavailable selected check");
                }
                let proven = test.constant().or_else(|| {
                    if selected.contains(&test) {
                        Some(true)
                    } else if selected.contains(&test.sense(false)) {
                        Some(false)
                    } else {
                        None
                    }
                });
                let take = if let Some(take) = proven {
                    if store.is_some() {
                        let (actual, _) = suffix.test(*arg).ok_or("unrecognized append suffix")?;
                        if actual.constant() != Some(take) {
                            seed_checks.push(actual.sense(take));
                        }
                    }
                    take
                } else {
                    if store.is_some() {
                        return Err("append suffix has a new check");
                    }
                    if let Some(width) = difference {
                        if selector.replace(width).is_some() {
                            return Err("multiple empty selectors");
                        }
                        test.cc == IntCC::Equal
                    } else {
                        if !matches!(test.cc, IntCC::UnsignedLessThan | IntCC::NotEqual) {
                            return Err("unsupported selected guard");
                        }
                        steps.push(Step::Guard(test));
                        true
                    }
                };
                selected.push(test.sense(take));
                &blocks[usize::from(!take)]
            }
            _ => return Err("selected path leaves the recurrence"),
        };
        let next_block = edge.block(&f.dfg.value_lists);
        let actual = edge
            .args(&f.dfg.value_lists)
            .map(|v| v.as_value())
            .collect::<Option<Vec<_>>>()
            .ok_or("non-value edge argument")?;
        let args = actual.iter().map(|&v| path.value(v)).collect::<Vec<_>>();
        let suffix_args = actual.iter().map(|&v| suffix.value(v)).collect::<Vec<_>>();
        if next_block == header {
            break (args, suffix_args);
        }
        path.enter(next_block, &args);
        if store.is_some() {
            suffix.enter(next_block, &suffix_args);
        }
        block = next_block;
    };

    let selector = selector.ok_or("no empty selector")?;
    let (_, append_term, stored, write_addr) = store.ok_or("no single append")?;
    let [(cache, _), _, (previous, read_addr)] = loads.as_slice() else {
        return Err("no three-load recurrence");
    };
    let [previous_value] = f.dfg.inst_results(*previous) else {
        return Err("previous load does not produce a scalar");
    };
    if selector == *previous_value {
        return Err("the selector must have its own emitted definition");
    }
    let Some(Step::Copy { .. }) = steps.iter().find(|step| {
        matches!(
            step, Step::Copy { inst, .. } if f.dfg.inst_results(*inst).contains(&selector)
        )
    }) else {
        return Err("selector is not computed within the visit");
    };
    // The prefix only executes the original guarded loads and pure operations.
    // Its exits therefore restart the original visit with unchanged state.
    let (append_inst, _, _, _) = store.unwrap();
    let raw = Walk {
        function: f,
        aliases: BTreeMap::new(),
    };
    let [raw_stored, raw_addr] = f.dfg.inst_args(append_inst) else {
        return Err("unsupported append operands");
    };
    let shape = raw.vector_address(*raw_addr);
    if raw.value(*raw_stored) != stored || shape != path.vector_address(write_addr) {
        return Err("append must use the same stored word and geometry at entry");
    }
    let (write_base, length, 0) = shape.ok_or("append address")? else {
        return Err("append index is offset");
    };
    if !available(context, write_base, first)
        || path.vector_address(*read_addr) != Some((write_base, length, -1))
    {
        return Err("previous read does not address the prior append word");
    }
    let i = params
        .iter()
        .position(|&p| p == length)
        .ok_or("length is not carried")?;
    if raw.addend(seed[i]) != (length, 1)
        || f.dfg.value_type(*previous_value) != types::I64
        || f.dfg.value_type(stored) != types::I64
    {
        return Err("incompatible next length or element width");
    }
    match (&f.dfg.insts[*previous], &f.dfg.insts[append_inst]) {
        (
            Data::Load {
                flags: read_flags,
                offset: read_offset,
                ..
            },
            Data::Store {
                flags: write_flags,
                offset: write_offset,
                ..
            },
        ) if read_flags == write_flags
            && i32::from(*read_offset) == 0
            && i32::from(*write_offset) == 0 => {}
        _ => return Err("incompatible previous load and append store"),
    }
    let Data::Jump { destination, .. } = &f.dfg.insts[append_term] else {
        return Err("append has no plain continuation");
    };
    if destination.args(&f.dfg.value_lists).next().is_some()
        || next != seed
        || seed
            .iter()
            .copied()
            .chain([selector, stored])
            .any(|v| !available(context, v, append_term))
        || seed_checks
            .iter()
            .any(|test| !test.values().all(|v| available(context, v, append_term)))
    {
        return Err("original append continuation does not supply actual next state");
    }
    let mut trace = Trace {
        steps,
        selector,
        stored,
        previous: *previous_value,
        append_term,
        continuation: destination.block(&f.dfg.value_lists),
        seed_checks,
        next,
        window: None,
    };
    trace.window = entry_window(context, &path, header, &trace, *cache, *previous, i);
    Ok(trace)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::native_artifact::program::codegen::numeric_words::load_unsigned_word;
    use cranelift_codegen::ir::{AbiParam, BlockArg, Endianness, InstBuilder, MemFlags, Type};
    use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

    #[test]
    fn append_guard_recognizes_only_a_canonical_comparison_mask() {
        let mut function = Function::new();
        function.signature.params = vec![
            AbiParam::new(types::I64),
            AbiParam::new(types::I64),
            AbiParam::new(types::I8),
        ];
        let mut context = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [cursor, end, byte] = b.block_params(entry).try_into().unwrap();
        let bound = b.ins().icmp(IntCC::NotEqual, cursor, end);
        let masked = b.ins().band_imm(bound, 1);
        let wide = b.ins().uextend(types::I64, bound);
        let extended_mask = b.ins().band_imm(wide, 1);
        let wrong_mask = b.ins().band_imm(bound, 2);
        let arbitrary_bit = b.ins().band_imm(byte, 1);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let walk = Walk {
            function: &function,
            aliases: BTreeMap::new(),
        };
        assert!(walk.test(bound).is_some());
        assert_eq!(walk.test(masked), walk.test(bound));
        assert_eq!(walk.test(extended_mask), walk.test(bound));
        assert_eq!(walk.test(wrong_mask), None);
        assert_eq!(walk.test(arbitrary_bit), None);
    }

    #[test]
    fn append_guard_resolves_only_the_selected_join_input() {
        let mut function = Function::new();
        function.signature.params = vec![
            AbiParam::new(types::I8),
            AbiParam::new(types::I8),
            AbiParam::new(types::I8),
        ];
        let mut context = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut function, &mut context);
        let entry = b.create_block();
        let other = b.create_block();
        let join = b.create_block();
        let latch = b.create_block();
        let exit = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(join, types::I8);

        b.switch_to_block(entry);
        let [select, select_other, input] = b.block_params(entry).try_into().unwrap();
        let one = b.ins().iconst(types::I8, 1);
        let from_append = b.ins().brif(select, join, &[one.into()], other, &[]);

        b.switch_to_block(other);
        let zero = b.ins().iconst(types::I8, 0);
        let from_other = b
            .ins()
            .brif(select_other, join, &[zero.into()], join, &[input.into()]);

        b.switch_to_block(join);
        let flag = b.block_params(join)[0];
        let masked = b.ins().band_imm(flag, 1);
        b.ins().brif(masked, latch, &[], exit, &[]);
        b.switch_to_block(latch);
        b.ins().return_(&[]);
        b.switch_to_block(exit);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let unresolved = Walk {
            function: &function,
            aliases: BTreeMap::new(),
        };
        assert_eq!(unresolved.test(masked), None);
        for (incoming, edge, expected) in [
            (from_append, 0, Some(true)),
            (from_other, 0, Some(false)),
            (from_other, 1, None),
        ] {
            let edges = function.dfg.insts[incoming]
                .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables);
            let incoming = &edges[edge];
            assert_eq!(incoming.block(&function.dfg.value_lists), join);
            let args = incoming
                .args(&function.dfg.value_lists)
                .map(|arg| arg.as_value().unwrap())
                .collect::<Vec<_>>();
            let mut path = Walk {
                function: &function,
                aliases: BTreeMap::new(),
            };
            path.enter(join, &args);
            assert_eq!(
                path.test(masked).and_then(|(test, _)| test.constant()),
                expected
            );
            if expected.is_none() {
                assert_eq!(path.test(masked), None);
            }
        }
    }

    #[derive(Clone, Copy)]
    enum Reads {
        Plain,
        Fused,
        Fused16,
        Signed32,
        FusedPrevious,
        ExtraBefore,
        ExtraAfter,
    }

    type Outcome = Result<(usize, usize, usize), &'static str>;
    type ClonedRead = (Opcode, MemFlags, i32, Type);

    // A checked append recurrence fed by the shared physical-word loader.
    fn witness(reads: Reads) -> (Outcome, Vec<ClonedRead>) {
        let mut context = Context::new();
        context.func.signature.params = vec![AbiParam::new(types::I64); 4];
        let mut builder_context = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder_context);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        let header = b.create_block();
        for _ in 0..3 {
            b.append_block_param(header, types::I64);
        }
        let visit = b.create_block();
        let append = b.create_block();
        let finish = b.create_block();
        let leave = b.create_block();

        b.switch_to_block(entry);
        let [base, count, cursor, length] = b.block_params(entry).try_into().unwrap();
        b.ins()
            .jump(header, &[count, cursor, length].map(BlockArg::Value));

        b.switch_to_block(header);
        let [count, cursor, length] = b.block_params(header).try_into().unwrap();
        let more = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
        let more = b.ins().band_imm(more, 1);
        b.ins().brif(more, visit, &[], leave, &[]);

        b.switch_to_block(visit);
        let flags = MemFlags::new().with_endianness(Endianness::Little);
        let safe_flags = flags.with_notrap();
        let cache_word = |b: &mut FunctionBuilder<'_>, offset: i32, flags: MemFlags| match reads {
            Reads::Plain => {
                let value = b.ins().load(types::I32, flags, base, offset);
                b.ins().uextend(types::I64, value)
            }
            Reads::Fused16 => load_unsigned_word(b, types::I16, flags, base, offset),
            Reads::Signed32 => b.ins().sload32(flags, base, offset),
            _ => load_unsigned_word(b, types::I32, flags, base, offset),
        };
        let header_word = cache_word(&mut b, 0, flags);
        let cost = cache_word(&mut b, 4, safe_flags);
        let prev_index = b.ins().iadd_imm(length, -1);
        let prev_bytes = b.ins().imul_imm(prev_index, 8);
        let prev_address = b.ins().iadd(base, prev_bytes);
        let previous = if matches!(reads, Reads::FusedPrevious) {
            load_unsigned_word(&mut b, types::I32, flags, prev_address, 0)
        } else {
            load_unsigned_word(&mut b, types::I64, flags, prev_address, 0)
        };
        if matches!(reads, Reads::ExtraBefore) {
            load_unsigned_word(&mut b, types::I32, flags, base, 8);
        }
        let width = b.ins().band_imm(header_word, 0xffff);
        let start = b.ins().isub(cursor, width);
        let empty = b.ins().icmp(IntCC::Equal, start, cursor);
        b.ins().brif(empty, append, &[], leave, &[]);

        b.switch_to_block(append);
        let word = b.ins().iadd(previous, cost);
        let bytes = b.ins().imul_imm(length, 8);
        let address = b.ins().iadd(base, bytes);
        b.ins().store(flags, word, address, 0);
        if matches!(reads, Reads::ExtraAfter) {
            load_unsigned_word(&mut b, types::I32, flags, base, 8);
        }
        let next_length = b.ins().iadd_imm(length, 1);
        let next_count = b.ins().iadd_imm(count, -1);
        b.ins().jump(finish, &[]);
        b.switch_to_block(finish);
        b.ins().jump(
            header,
            &[next_count, start, next_length].map(BlockArg::Value),
        );

        b.switch_to_block(leave);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();

        let old = context
            .func
            .layout
            .blocks()
            .flat_map(|block| context.func.layout.block_insts(block))
            .collect::<BTreeSet<_>>();
        let result = super::super::optimize(&mut context, header, None);
        let settings =
            cranelift_codegen::settings::Flags::new(cranelift_codegen::settings::builder());
        context.verify(&settings).unwrap();
        let f = &context.func;
        let cloned = f
            .layout
            .blocks()
            .flat_map(|block| f.layout.block_insts(block))
            .filter(|inst| !old.contains(inst))
            .filter_map(|inst| {
                let Data::Load {
                    opcode: opcode @ (Opcode::Load | Opcode::Uload32),
                    flags,
                    offset,
                    ..
                } = f.dfg.insts[inst]
                else {
                    return None;
                };
                Some((
                    opcode,
                    flags,
                    offset.into(),
                    f.dfg.value_type(f.dfg.first_result(inst)),
                ))
            })
            .collect();
        (result, cloned)
    }

    #[test]
    fn fused_cache_reads_clone_in_order_without_changing_width_or_flags() {
        let (result, cloned) = witness(Reads::Fused);
        assert_eq!(witness(Reads::Plain).0, Ok((4, 0, 0)));
        assert_eq!(result, Ok((4, 0, 0)));
        let flags = MemFlags::new().with_endianness(Endianness::Little);
        assert_eq!(
            cloned,
            [
                (Opcode::Uload32, flags, 0, types::I64),
                (Opcode::Uload32, flags.with_notrap(), 4, types::I64),
            ]
        );
    }

    #[test]
    fn unsupported_loads_leave_the_checked_append_path() {
        for reads in [
            Reads::Fused16,
            Reads::Signed32,
            Reads::FusedPrevious,
            Reads::ExtraBefore,
            Reads::ExtraAfter,
        ] {
            assert_eq!(
                witness(reads),
                (Err("selected path has an unsupported effect"), vec![]),
            );
        }
    }
}
