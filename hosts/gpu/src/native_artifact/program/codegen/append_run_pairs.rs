//! Pair two checked visits of the proved scalar append run.
use super::{Compare, Step, Trace, Window, Word, block, mapped, trace::Walk};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockArg, Endianness, Inst, InstBuilder, InstructionData as Data, MemFlags, Opcode,
    Value, ValueDef, types,
};
use std::collections::BTreeMap;

#[derive(Clone, Copy)]
pub(super) struct Pair {
    cache: Value,
    literals: Value,
    literal_limit: Word,
    output: Value,
    selector_bits: u32,
    item_shift: u32,
    item_tag: u64,
    header_flags: MemFlags,
    cost_flags: MemFlags,
    output_flags: MemFlags,
}

fn unsigned_32_load(walk: &Walk<'_>, inst: Inst) -> Option<(Value, MemFlags, Value)> {
    let f = walk.function;
    let result = f.dfg.first_result(inst);
    let Data::Load {
        opcode: Opcode::Uload32,
        arg,
        flags,
        offset,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    (i32::from(offset) == 0
        && f.dfg.value_type(result) == types::I64
        && flags.endianness(Endianness::Little) == Endianness::Little)
        .then_some((walk.value(arg), flags, result))
}

fn constant(walk: &Walk<'_>, value: Value) -> Option<i64> {
    if let Word::Imm(value) = walk.word(value) {
        return Some(value);
    }
    let Data::BinaryImm64 {
        opcode: Opcode::BandImm,
        arg,
        imm,
    } = *walk.result(value)?
    else {
        return None;
    };
    let Word::Imm(value) = walk.word(arg) else {
        return None;
    };
    Some(value & i64::from(imm))
}

fn constant_binary(walk: &Walk<'_>, result: Value, opcode: Opcode) -> Option<(Value, i64)> {
    if let Data::BinaryImm64 {
        opcode: immediate,
        arg,
        imm,
    } = *walk.result(result)?
    {
        return matches!(
            (opcode, immediate),
            (Opcode::Band, Opcode::BandImm)
                | (Opcode::Bor, Opcode::BorImm)
                | (Opcode::Ushr, Opcode::UshrImm)
                | (Opcode::Ishl, Opcode::IshlImm)
        )
        .then_some((walk.value(arg), imm.into()));
    }
    let [left, right] = walk.pair(result, opcode)?;
    if let Some(value) = constant(walk, right) {
        Some((left, value))
    } else if matches!(opcode, Opcode::Band | Opcode::Bor) {
        Some((right, constant(walk, left)?))
    } else {
        None
    }
}

fn masked_word(walk: &Walk<'_>, value: Value) -> Option<Value> {
    let (source, mask) = constant_binary(walk, value, Opcode::Band)?;
    (mask == i64::from(u32::MAX)).then_some(source)
}

impl Pair {
    pub(super) fn derive(
        context: &Context,
        trace: &Trace,
        params: &[Value],
    ) -> Result<Self, &'static str> {
        let window = trace.window.ok_or("missing readable window")?;
        // The trace's operand bindings are the values on the empty edge.
        // Interpret the original instructions with exactly those bindings.
        let f = &context.func;
        let aliases = trace
            .steps
            .iter()
            .filter_map(|step| match step {
                Step::Copy { inst, args } => Some(f.dfg.inst_args(*inst).iter().copied().zip(args)),
                Step::Guard(_) => None,
            })
            .flatten()
            .map(|(value, &selected)| (f.dfg.resolve_aliases(value), selected))
            .collect();
        let walk = Walk {
            function: f,
            aliases,
        };
        let mut loads = trace.steps.iter().filter_map(|step| match step {
            Step::Copy { inst, .. } if matches!(f.dfg.insts[*inst], Data::Load { .. }) => {
                Some(*inst)
            }
            _ => None,
        });
        let (cache_addr, header_flags, header) = loads
            .next()
            .and_then(|load| unsigned_32_load(&walk, load))
            .ok_or("unsupported header load")?;
        let (cost_addr, cost_flags, cost) = loads
            .next()
            .and_then(|load| unsigned_32_load(&walk, load))
            .ok_or("unsupported cost load")?;
        let previous = f
            .dfg
            .first_result(loads.next().ok_or("missing previous word load")?);
        if loads.next().is_some() || previous != trace.previous {
            return Err("extra read or different previous word");
        }

        let (cache, cache_cursor, -1) = walk
            .scaled_address(cache_addr, 4)
            .ok_or("unsupported header address")?
        else {
            return Err("cache ordinal is not cursor minus one");
        };
        let (literals, literal, 0) = walk
            .scaled_address(cost_addr, 4)
            .ok_or("unsupported cost address")?
        else {
            return Err("cost ordinal has an offset");
        };
        let (packed, bits) =
            constant_binary(&walk, literal, Opcode::Ushr).ok_or("missing literal extraction")?;
        let selector_bits = u32::try_from(bits & 63)
            .ok()
            .filter(|bits| (1..32).contains(bits))
            .ok_or("invalid selector width")?;
        let (selector_source, mask) =
            constant_binary(&walk, trace.selector, Opcode::Band).ok_or("missing selector mask")?;
        if cache_cursor != params[window.cursor]
            || packed != header
            || selector_source != header
            || mask != (1_i64 << selector_bits) - 1
        {
            return Err("header cursor or selector differs");
        }

        let Compare {
            cc: IntCC::UnsignedLessThan,
            left: Word::Value(checked_literal),
            right: literal_limit,
        } = trace
            .steps
            .iter()
            .find_map(|step| match step {
                Step::Guard(check)
                    if *check != window.cache_guard && *check != window.previous_guard =>
                {
                    (check.left == Word::Value(literal)).then_some(*check)
                }
                _ => None,
            })
            .ok_or("missing literal bound")?
        else {
            return Err("literal guard sense differs");
        };
        let literal_guard = Compare::new(
            IntCC::UnsignedLessThan,
            Word::Value(checked_literal),
            literal_limit,
        );
        let count_guard = Compare::new(
            IntCC::NotEqual,
            Word::Value(params[window.count]),
            Word::Imm(0),
        );
        if trace.steps.iter().any(|step| {
            matches!(step, Step::Guard(check)
                if *check != count_guard && *check != window.cache_guard
                    && *check != window.previous_guard && *check != literal_guard)
        }) {
            return Err("an additional guard is required");
        }

        let [low, high] = walk
            .pair(trace.stored, Opcode::Bor)
            .ok_or("append word does not combine item and cost")?;
        let (item, shift) =
            constant_binary(&walk, high, Opcode::Ishl).ok_or("missing item placement")?;
        if shift & 63 != 32 {
            return Err("item placement differs");
        }
        let (item_bits, item_tag) = constant_binary(
            &walk,
            masked_word(&walk, item).ok_or("item word is not masked")?,
            Opcode::Bor,
        )
        .ok_or("missing item tag")?;
        let (item_literal, item_shift) =
            constant_binary(&walk, item_bits, Opcode::Ishl).ok_or("missing item shift")?;
        let item_shift = u32::try_from(item_shift & 63)
            .ok()
            .filter(|shift| *shift <= selector_bits)
            .ok_or("item exceeds word width")?;
        let item_tag = u32::try_from(item_tag).map_err(|_| "item tag exceeds word width")?;
        if masked_word(&walk, item_literal).ok_or("missing literal word mask")? != literal {
            return Err("packed item uses a different literal");
        }

        let low = masked_word(&walk, low).ok_or("missing cost word mask")?;
        let (sum, narrow) = match *walk.result(low).ok_or("missing cost word")? {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } if f.dfg.value_type(arg) == types::I32 => (arg, true),
            _ => (low, false),
        };
        let [cost_input, previous_input] = walk
            .pair(sum, Opcode::Iadd)
            .ok_or("missing wrapping cost addition")?;
        let input = |value| {
            if !narrow {
                return Some(value);
            }
            let Data::Unary {
                opcode: Opcode::Ireduce,
                arg,
            } = *walk.result(value)?
            else {
                return None;
            };
            Some(walk.value(arg))
        };
        if input(cost_input) != Some(cost)
            || input(previous_input).and_then(|word| masked_word(&walk, word)) != Some(previous)
        {
            return Err("cost sum differs");
        }
        let store = trace
            .steps
            .iter()
            .find_map(|step| match step {
                Step::Copy { inst, .. } if f.dfg.insts[*inst].opcode() == Opcode::Store => {
                    Some(*inst)
                }
                _ => None,
            })
            .ok_or("missing append store")?;
        let Data::Store {
            opcode: Opcode::Store,
            args: [_, address],
            flags: output_flags,
            offset,
        } = f.dfg.insts[store]
        else {
            return Err("append store differs");
        };
        let (output, write_length, 0) = walk
            .scaled_address(address, 8)
            .ok_or("unsupported append address")?
        else {
            return Err("append ordinal has an offset");
        };
        if i32::from(offset) != 0
            || output_flags.endianness(Endianness::Little) != Endianness::Little
            || write_length != params[window.length]
            || params.len() < 3
            || [window.count, window.cursor, window.length] != [0, 1, 2]
            || trace.next[3..]
                .iter()
                .any(|&next| next != trace.next[window.length])
            || trace.next[0] == trace.next[1]
            || trace.next[0] == trace.next[2]
            || trace.next[1] == trace.next[2]
        {
            return Err("unsupported scalar recurrence state");
        }
        // A value cloned inside the visit cannot be used by pair entry.
        let entry = f.layout.entry_block().ok_or("missing phase entry")?;
        let packet_values = f.dfg.block_params(entry);
        let packet = *packet_values.get(1).ok_or("missing phase packet")?;
        let available = |value: Value| {
            let ValueDef::Result(inst, _) = f.dfg.value_def(value) else {
                return false;
            };
            matches!(f.dfg.insts[inst], Data::Load { opcode: Opcode::Load, arg, .. } if arg == packet)
                && context
                    .domtree
                    .dominates(inst, trace.append_term, &f.layout)
        };
        if ![cache, literals, output].into_iter().all(available)
            || matches!(literal_limit, Word::Value(value) if !available(value))
        {
            return Err("packet words are unavailable at pair entry");
        }
        Ok(Self {
            cache,
            literals,
            literal_limit,
            output,
            selector_bits,
            item_shift,
            item_tag: u64::from(item_tag),
            header_flags,
            cost_flags,
            output_flags,
        })
    }

    pub(super) fn emit(
        self,
        b: &mut FuncCursor<'_>,
        window: Window,
        seed: &[Value],
        previous: Value,
        descriptors: &BTreeMap<Value, Value>,
        scalar: Block,
        scalar_args: &[BlockArg],
    ) {
        let [count, cache_cursor, length] =
            [window.count, window.cursor, window.length].map(|i| seed[i]);
        let cache = mapped(descriptors, self.cache);
        let literals = mapped(descriptors, self.literals);
        let output = mapped(descriptors, self.output);
        let literal_limit = match self.literal_limit {
            Word::Value(value) => mapped(descriptors, value),
            Word::Imm(limit) => b.ins().iconst(types::I64, limit),
        };
        let start = block(b.func);
        for _ in 0..4 {
            b.func.dfg.append_block_param(start, types::I64);
        }
        let load = block(b.func);
        let check_literals = block(b.func);
        let append = block(b.func);

        // The existing window proves nonwrapping length + count and a
        // readable cache prefix. Check the byte geometry independently so
        // the first append cannot change either visit's input reads.
        let end = b.ins().iadd(length, count);
        let (out_end, out_valid) = span(b, output, end, 8);
        let (cache_end, cache_valid) = span(b, cache, cache_cursor, 4);
        let (lit_end, lit_valid) = span(b, literals, literal_limit, 4);
        let cache_clear = disjoint(b, output, out_end, cache, cache_end);
        let lit_clear = disjoint(b, output, out_end, literals, lit_end);
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, count, 8);
        let valid = b.ins().band(out_valid, cache_valid);
        let valid = b.ins().band(valid, lit_valid);
        let separated = b.ins().band(cache_clear, lit_clear);
        let valid = b.ins().band(valid, separated);
        let valid = b.ins().band(valid, enough);
        let first = [count, cache_cursor, length, previous].map(BlockArg::Value);
        b.ins().brif(valid, start, &first, scalar, scalar_args);

        b.goto_bottom(start);
        let [count, cache_cursor, length, previous] =
            b.func.dfg.block_params(start).try_into().unwrap();
        let resume = [count, cache_cursor, length, previous].map(BlockArg::Value);
        let enough = b
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, count, 2);
        b.ins().brif(enough, load, &[], scalar, &resume);

        b.goto_bottom(load);
        let next_cache = b.ins().iadd_imm(cache_cursor, -2);
        let address = indexed(b, cache, next_cache, 4);
        // The adjacent source words may start four bytes off an I64
        // boundary. The AArch64 target supports the unaligned read.
        let flags = MemFlags::new()
            .with_endianness(Endianness::Little)
            .with_trap_code(self.header_flags.trap_code());
        let packed = b.ins().load(types::I64, flags, address, 0);
        let selector_mask = (1_i64 << self.selector_bits) - 1;
        let mask = selector_mask | (selector_mask << 32);
        let selectors = b.ins().band_imm(packed, mask);
        let empty = b.ins().icmp_imm(IntCC::Equal, selectors, 0);
        b.ins().brif(empty, check_literals, &[], scalar, &resume);

        b.goto_bottom(check_literals);
        let first = b.ins().ushr_imm(packed, i64::from(self.selector_bits + 32));
        let second = b.ins().ushr_imm(packed, i64::from(self.selector_bits));
        let second = b
            .ins()
            .band_imm(second, i64::from(u32::MAX >> self.selector_bits));
        let first_valid = b.ins().icmp(IntCC::UnsignedLessThan, first, literal_limit);
        let second_valid = b.ins().icmp(IntCC::UnsignedLessThan, second, literal_limit);
        let both = b.ins().band(first_valid, second_valid);
        b.ins().brif(both, append, &[], scalar, &resume);

        b.goto_bottom(append);
        let first_addr = indexed(b, literals, first, 4);
        let second_addr = indexed(b, literals, second, 4);
        let first_cost = b.ins().load(types::I32, self.cost_flags, first_addr, 0);
        let second_cost = b.ins().load(types::I32, self.cost_flags, second_addr, 0);
        let previous = b.ins().ireduce(types::I32, previous);
        let first_cost = b.ins().iadd(previous, first_cost);
        let second_cost = b.ins().iadd(first_cost, second_cost);
        let first_word = self.pack(b, first, first_cost);
        let second_word = self.pack(b, second, second_cost);
        let address = indexed(b, output, length, 8);
        b.ins().store(self.output_flags, first_word, address, 0);
        b.ins().store(self.output_flags, second_word, address, 8);
        let next_count = b.ins().iadd_imm(count, -2);
        let next_length = b.ins().iadd_imm(length, 2);
        let args = [next_count, next_cache, next_length, second_word].map(BlockArg::Value);
        b.ins().jump(start, &args);
    }

    fn pack(self, b: &mut FuncCursor<'_>, item: Value, cost: Value) -> Value {
        let cost = b.ins().uextend(types::I64, cost);
        let item = b.ins().ishl_imm(item, i64::from(self.item_shift + 32));
        let item = b.ins().bor_imm(item, (self.item_tag << 32) as i64);
        b.ins().bor(item, cost)
    }
}

fn indexed(b: &mut FuncCursor<'_>, base: Value, index: Value, stride: i64) -> Value {
    let delta = b.ins().imul_imm(index, stride);
    b.ins().iadd(base, delta)
}

fn span(b: &mut FuncCursor<'_>, base: Value, length: Value, stride: i64) -> (Value, Value) {
    let fits = b.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        length,
        (u64::MAX / stride as u64) as i64,
    );
    let end = indexed(b, base, length, stride);
    let ordered = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, base, end);
    (end, b.ins().band(fits, ordered))
}

fn disjoint(b: &mut FuncCursor<'_>, a: Value, a_end: Value, z: Value, z_end: Value) -> Value {
    let before = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, a_end, z);
    let after = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, z_end, a);
    b.ins().bor(before, after)
}
