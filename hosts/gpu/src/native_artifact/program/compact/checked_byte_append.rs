//! Use uniquely owned backing directly for checked little-endian byte appends.
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::{
    Context,
    ir::{
        Block, BlockCall, Endianness, FuncRef, Function, Inst, InstBuilder,
        InstructionData as Data, MemFlags, Opcode as Op, Value, ValueDef, condcodes::IntCC, types,
    },
    isa::CallConv,
};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Module};

#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct ShortAppend {
    // Parameter positions in the proved continuation signature.
    count: usize,
    length: usize,
    capacity: usize,
    owner: usize,
    data: usize,
    value: usize,
    shift: usize,
    owner_flags: MemFlags,
    store_flags: MemFlags,
    pub(super) maximum: u8,
}

fn definition(f: &Function, value: Value) -> Option<(Inst, usize)> {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Result(inst, result) => Some((inst, result)),
        _ => None,
    }
}

fn literal(f: &Function, value: Value) -> Option<i64> {
    let (inst, 0) = definition(f, value)? else {
        return None;
    };
    match f.dfg.insts[inst] {
        Data::UnaryImm {
            opcode: Op::Iconst,
            imm,
        } => Some(imm.bits()),
        _ => None,
    }
}

fn arguments(f: &Function, call: BlockCall) -> Option<Vec<Value>> {
    call.args(&f.dfg.value_lists)
        .map(|arg| Some(f.dfg.resolve_aliases(arg.as_value()?)))
        .collect()
}

fn plain_edge(f: &Function, call: BlockCall) -> Option<Block> {
    arguments(f, call)?
        .is_empty()
        .then_some(call.block(&f.dfg.value_lists))
}

fn branch(f: &Function, block: Block) -> Option<(Value, [BlockCall; 2])> {
    match f.dfg.insts[f.layout.last_inst(block)?] {
        Data::Brif { arg, blocks, .. } => Some((f.dfg.resolve_aliases(arg), blocks)),
        _ => None,
    }
}

fn jump(f: &Function, block: Block) -> Option<Block> {
    match f.dfg.insts[f.layout.last_inst(block)?] {
        Data::Jump { destination, .. } => plain_edge(f, destination),
        _ => None,
    }
}

fn setup(f: &Function, block: Block, extra: &[Op]) -> bool {
    let end = f.layout.last_inst(block);
    f.layout
        .block_insts(block)
        .take_while(|inst| Some(*inst) != end)
        .all(|inst| {
            let op = f.dfg.insts[inst].opcode();
            matches!(op, Op::Iconst | Op::Icmp) || extra.contains(&op)
        })
}

fn binary(f: &Function, value: Value, op: Op) -> Option<[Value; 2]> {
    let (inst, 0) = definition(f, value)? else {
        return None;
    };
    match f.dfg.insts[inst] {
        Data::Binary { opcode, args } if opcode == op => {
            Some(args.map(|v| f.dfg.resolve_aliases(v)))
        }
        _ => None,
    }
}

fn compare(f: &Function, value: Value, cc: IntCC) -> Option<[Value; 2]> {
    let (inst, 0) = definition(f, value)? else {
        return None;
    };
    match f.dfg.insts[inst] {
        Data::IntCompare {
            opcode: Op::Icmp,
            cond,
            args,
        } if cond == cc => Some(args.map(|v| f.dfg.resolve_aliases(v))),
        _ => None,
    }
}

fn compare_literal(f: &Function, value: Value, cc: IntCC, expected: i64) -> Option<Value> {
    let [left, right] = compare(f, value, cc)?;
    (literal(f, right)? == expected).then_some(left)
}

fn skip_owner_update(f: &Function, block: Block, current: Value, previous: Value) -> Option<Block> {
    let (test, [_, no]) = branch(f, block)?;
    let (inst, 0) = definition(f, test)? else {
        return None;
    };
    let Data::Ternary {
        opcode: Op::Select,
        args: [same, zero, _],
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    let pair = compare(f, same, IntCC::Equal)?;
    if literal(f, zero)? != 0
        || (pair != [current, previous] && pair != [previous, current])
        || !setup(f, block, &[Op::Select])
    {
        return None;
    }
    plain_edge(f, no)
}

fn adding(f: &Function, value: Value, base: Value, offset: i64) -> bool {
    if let Some([a, b]) = binary(f, value, Op::Iadd) {
        return (a == base && literal(f, b) == Some(offset))
            || (b == base && literal(f, a) == Some(offset));
    }
    let Some((inst, 0)) = definition(f, value) else {
        return false;
    };
    matches!(f.dfg.insts[inst], Data::BinaryImm64 { opcode: Op::IaddImm, arg, imm }
        if f.dfg.resolve_aliases(arg) == base && imm.bits() == offset)
}

fn following_store(
    f: &Function,
    block: Block,
    data: Value,
    length: Value,
    word: Value,
    index: i64,
    expected_flags: MemFlags,
) -> Option<Block> {
    if !setup(
        f,
        block,
        &[
            Op::Iadd,
            Op::IaddImm,
            Op::Ushr,
            Op::UshrImm,
            Op::Ireduce,
            Op::Store,
        ],
    ) {
        return None;
    }
    let mut stores = f
        .layout
        .block_insts(block)
        .filter(|&i| f.dfg.insts[i].opcode() == Op::Store);
    let Data::Store {
        args: [byte, address],
        offset,
        flags,
        ..
    } = f.dfg.insts[stores.next()?]
    else {
        return None;
    };
    if stores.next().is_some() || flags != expected_flags || f.dfg.value_type(byte) != types::I8 {
        return None;
    }
    let (cast, 0) = definition(f, byte)? else {
        return None;
    };
    let Data::Unary {
        opcode: Op::Ireduce,
        arg: shifted,
    } = f.dfg.insts[cast]
    else {
        return None;
    };
    let shift = index * 8;
    if let Some([a, b]) = binary(f, shifted, Op::Ushr) {
        if a != word || literal(f, b) != Some(shift) {
            return None;
        }
    } else {
        let (inst, 0) = definition(f, shifted)? else {
            return None;
        };
        if !matches!(f.dfg.insts[inst], Data::BinaryImm64 { opcode: Op::UshrImm, arg, imm }
            if f.dfg.resolve_aliases(arg) == word && imm.bits() == shift)
        {
            return None;
        }
    }
    let [left, right] = binary(f, address, Op::Iadd)?;
    let other = if left == data {
        right
    } else if right == data {
        left
    } else {
        return None;
    };
    let exact = (other == length && i64::from(i32::from(offset)) == index)
        || (i32::from(offset) == 0 && adding(f, other, length, index));
    exact.then(|| jump(f, block)).flatten()
}

impl ShortAppend {
    pub(super) fn recognize(f: &Function) -> Option<Self> {
        if f.signature.call_conv != CallConv::AppleAarch64
            || f.signature.params.len() > 8
            || f.signature.returns.len() != 7
            || f.signature
                .params
                .iter()
                .chain(&f.signature.returns)
                .any(|param| *param != cranelift_codegen::ir::AbiParam::new(types::I64))
        {
            return None;
        }
        let entry = f.layout.entry_block()?;
        let params = f.dfg.block_params(entry);
        let slot = |v| params.iter().position(|&arg| arg == v);
        let width = jump(f, entry)?;
        let (tested, [yes, _]) = branch(f, width)?;
        let count = compare_literal(f, tested, IntCC::UnsignedLessThanOrEqual, 8)?;
        let addition = plain_edge(f, yes)?;
        let (valid, [yes, _]) = branch(f, addition)?;
        let carry = compare_literal(f, valid, IntCC::Equal, 0)?;
        let (add, 1) = definition(f, carry)? else {
            return None;
        };
        let [length, amount] = *f.dfg.inst_args(add) else {
            return None;
        };
        let length = f.dfg.resolve_aliases(length);
        let end = f.dfg.first_result(add);
        let extent = plain_edge(f, yes)?;
        let (fits, [yes, _]) = branch(f, extent)?;
        let [required, capacity] = compare(f, fits, IntCC::UnsignedLessThanOrEqual)?;
        let present = plain_edge(f, yes)?;
        let (owner, [yes, _]) = branch(f, present)?;
        let ownership = plain_edge(f, yes)?;
        let (unique, [reuse, _]) = branch(f, ownership)?;
        let refs = compare_literal(f, unique, IntCC::Equal, 1)?;
        let (load, 0) = definition(f, refs)? else {
            return None;
        };
        let Data::Load {
            opcode: Op::Load,
            arg,
            offset,
            flags: owner_flags,
        } = f.dfg.insts[load]
        else {
            return None;
        };
        let [data, kept_capacity, kept_owner] = arguments(f, reuse)?.try_into().ok()?;
        if f.dfg.insts[add].opcode() != Op::UaddOverflow
            || f.dfg.resolve_aliases(amount) != count
            || required != end
            || kept_capacity != capacity
            || kept_owner != owner
            || f.dfg.value_type(refs) != types::I64
            || f.layout.inst_block(load) != Some(ownership)
            || f.dfg.resolve_aliases(arg) != owner
            || i32::from(offset) != 0
            || ![entry, width, extent, present]
                .into_iter()
                .all(|at| setup(f, at, &[]))
            || !setup(f, addition, &[Op::UaddOverflow])
            || !setup(f, ownership, &[Op::Load])
            || f.layout
                .block_insts(ownership)
                .filter(|&inst| f.dfg.insts[inst].opcode() == Op::Load)
                .count()
                != 1
        {
            return None;
        }

        let first = reuse.block(&f.dfg.value_lists);
        let [current, available, current_owner] = *f.dfg.block_params(first) else {
            return None;
        };
        let (nonempty, [put, skip]) = branch(f, first)?;
        let write = plain_edge(f, put)?;
        let done = plain_edge(f, skip)?;
        let second = jump(f, write)?;
        let (more, [two, one]) = branch(f, second)?;
        if ![first, second].into_iter().all(|at| setup(f, at, &[]))
            || compare_literal(f, nonempty, IntCC::NotEqual, 0)? != count
            || compare_literal(f, more, IntCC::UnsignedGreaterThan, 1)? != count
            || plain_edge(f, one)? != done
            || !setup(f, write, &[Op::Iadd, Op::Ireduce, Op::Store])
        {
            return None;
        }
        let mut stores = f
            .layout
            .block_insts(write)
            .filter(|&inst| f.dfg.insts[inst].opcode() == Op::Store);
        let Data::Store {
            args: [byte, address],
            offset,
            flags: store_flags,
            ..
        } = f.dfg.insts[stores.next()?]
        else {
            return None;
        };
        let (narrow, 0) = definition(f, byte)? else {
            return None;
        };
        let Data::Unary {
            opcode: Op::Ireduce,
            arg: value,
        } = f.dfg.insts[narrow]
        else {
            return None;
        };
        let value = f.dfg.resolve_aliases(value);
        let address = binary(f, address, Op::Iadd)?;
        if stores.next().is_some()
            || f.dfg.value_type(byte) != types::I8
            || i32::from(offset) != 0
            || (address != [current, length] && address != [length, current])
        {
            return None;
        }

        let maximum = if std::env::var_os("MECH_COMPACT_SHORT_BYTE_APPENDS")
            .is_some_and(|mode| mode == "wide")
        {
            let mut next_store = plain_edge(f, two);
            let mut proved = 1;
            for index in 1..8 {
                let Some(after) = next_store.and_then(|block| {
                    following_store(f, block, current, length, value, index, store_flags)
                }) else {
                    break;
                };
                if after == done {
                    if index == 7 {
                        proved = 8;
                    }
                    break;
                }
                let Some((condition, [next, end])) = branch(f, after) else {
                    break;
                };
                if !setup(f, after, &[])
                    || compare_literal(f, condition, IntCC::UnsignedGreaterThan, index + 1)
                        != Some(count)
                    || plain_edge(f, end) != Some(done)
                {
                    break;
                }
                proved = (index + 1) as u8;
                next_store = plain_edge(f, next);
            }
            proved
        } else {
            1
        };

        // On the reuse edge the owner aliases its input exactly, so both
        // ownership updates select zero. All remaining instructions are pure.
        let release = skip_owner_update(f, done, current_owner, owner)?;
        let mut tail = skip_owner_update(f, release, current_owner, owner)?;
        for _ in 0..f.dfg.num_blocks() {
            let last = f.layout.last_inst(tail)?;
            if f.dfg.insts[last].opcode() == Op::Return {
                let [
                    tag,
                    status,
                    shifted,
                    result_end,
                    result_data,
                    result_capacity,
                    result_owner,
                ] = *f.dfg.inst_args(last)
                else {
                    return None;
                };
                let [word, mask] = binary(f, shifted, Op::Ushr)?;
                let [scaled, width] = binary(f, mask, Op::Band)?;
                let [shift, bits] = binary(f, scaled, Op::Ishl)?;
                if !setup(f, tail, &[Op::Ushr, Op::Band, Op::Ishl])
                    || literal(f, tag)? != 1
                    || literal(f, status)? != 0
                    || [result_end, result_data, result_capacity, result_owner]
                        .map(|v| f.dfg.resolve_aliases(v))
                        != [end, current, available, current_owner]
                    || word != value
                    || literal(f, width)? != 63
                    || literal(f, bits)? != 3
                {
                    return None;
                }
                return Some(Self {
                    count: slot(count)?,
                    length: slot(length)?,
                    capacity: slot(capacity)?,
                    owner: slot(owner)?,
                    data: slot(data)?,
                    value: slot(value)?,
                    shift: slot(shift)?,
                    owner_flags,
                    store_flags,
                    maximum,
                });
            }
            if !setup(f, tail, &[]) {
                return None;
            }
            tail = jump(f, tail)?;
        }
        None
    }

    pub(super) fn define<M: Module>(
        self,
        module: &mut M,
        function: FuncId,
        fallback: FuncId,
    ) -> Result<Context, NativeCompileError> {
        let mut context = module.make_context();
        context.func.signature = module
            .declarations()
            .get_function_decl(function)
            .signature
            .clone();
        context.func.aarch64_branch_on_zero = true;
        let fallback = module.declare_func_in_func(fallback, &mut context.func);
        self.emit(&mut context.func, fallback);
        context.verify(module.isa()).map_err(backend)?;
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        Ok(context)
    }

    fn emit_many(
        self,
        b: &mut FunctionBuilder<'_>,
        address: Value,
        word: Value,
        count: Value,
        done: Block,
    ) {
        let little = MemFlags::new().with_endianness(Endianness::Little);
        if self.maximum >= 4 {
            let wide = b.create_block();
            let half = b.create_block();
            let four = b
                .ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, count, 4);
            b.ins().brif(four, wide, &[], half, &[]);
            b.switch_to_block(wide);
            let offset = b.ins().iadd_imm(count, -4);
            let tail = b.ins().iadd(address, offset);
            let shift = b.ins().ishl_imm(offset, 3);
            let high = b.ins().ushr(word, shift);
            let low = b.ins().ireduce(types::I32, word);
            let high = b.ins().ireduce(types::I32, high);
            // The end stores cover four through eight bytes exactly.
            // Overlapping bytes receive the same bits.
            b.ins().store(little, low, address, 0);
            b.ins().store(little, high, tail, 0);
            b.ins().jump(done, &[]);
            b.switch_to_block(half);
        }

        let low = b.ins().ireduce(types::I16, word);
        b.ins().store(little, low, address, 0);
        if self.maximum >= 3 {
            let last = b.create_block();
            let three = b.ins().icmp_imm(IntCC::Equal, count, 3);
            b.ins().brif(three, last, &[], done, &[]);
            b.switch_to_block(last);
            let high = b.ins().ushr_imm(word, 16);
            let byte = b.ins().ireduce(types::I8, high);
            b.ins().store(little, byte, address, 2);
        }
        b.ins().jump(done, &[]);
    }

    fn emit(self, function: &mut Function, fallback: FuncRef) {
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(function, &mut builder);
        let entry = b.create_block();
        let extent = b.create_block();
        let owner_gate = b.create_block();
        let ownership = b.create_block();
        let nonempty = b.create_block();
        let write = b.create_block();
        let finish = b.create_block();
        let slow = b.create_block();
        b.set_cold_block(slow);
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let count = args[self.count];
        let length = args[self.length];
        let capacity = args[self.capacity];
        let owner = args[self.owner];
        let data = args[self.data];
        let word = args[self.value];
        let width = b.ins().icmp_imm(
            IntCC::UnsignedLessThanOrEqual,
            count,
            i64::from(self.maximum),
        );
        b.ins().brif(width, extent, &[], slow, &[]);
        b.switch_to_block(extent);
        let end = b.ins().iadd(length, count);
        let no_wrap = b.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, end, length);
        let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, capacity);
        let same_backing = b.ins().band(no_wrap, fits);
        b.ins().brif(same_backing, owner_gate, &[], slow, &[]);
        b.switch_to_block(owner_gate);
        b.ins().brif(owner, ownership, &[], slow, &[]);
        b.switch_to_block(ownership);
        let refs = b.ins().load(types::I64, self.owner_flags, owner, 0);
        let unique = b.ins().icmp_imm(IntCC::Equal, refs, 1);
        b.ins().brif(unique, nonempty, &[], slow, &[]);
        b.switch_to_block(nonempty);
        b.ins().brif(count, write, &[], finish, &[]);
        b.switch_to_block(write);
        let byte = b.ins().ireduce(types::I8, word);
        let address = b.ins().iadd(data, length);
        if self.maximum > 1 {
            let one = b.create_block();
            let many = b.create_block();
            let is_one = b.ins().icmp_imm(IntCC::Equal, count, 1);
            b.ins().brif(is_one, one, &[], many, &[]);
            b.switch_to_block(many);
            self.emit_many(&mut b, address, word, count, finish);
            b.switch_to_block(one);
        }
        b.ins().store(self.store_flags, byte, address, 0);
        b.ins().jump(finish, &[]);
        b.switch_to_block(finish);
        let bits = b.ins().ishl_imm(args[self.shift], 3);
        let bits = b.ins().band_imm(bits, 63);
        let rest = b.ins().ushr(word, bits);
        let success = b.ins().iconst(types::I64, 1);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins()
            .return_(&[success, zero, rest, end, data, capacity, owner]);
        b.switch_to_block(slow);
        let call = b.ins().call(fallback, &args);
        let outputs = b.inst_results(call).to_vec();
        b.ins().return_(&outputs);
        b.seal_all_blocks();
        b.finalize();
    }
}
