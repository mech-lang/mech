//! Batch a selected unit-stride histogram arm of a checked numeric loop.
//! The caller admits the writable owners and the phase admits current extents.
//! Every group proves all four original visits before its first increment.
use super::composed_reads::{View, between, defaulted_read, scalar};
use super::window_bounds::{comparison, sole_predecessor};
use super::{MAX_CONSTANT_DEPTH, MAX_CONSTANT_STEPS, constant_integer};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::ir::{
    Block, BlockCall, Endianness, FuncRef, Function, Inst, InstBuilder, InstructionData as Data,
    MemFlags, Opcode as Op, Value, ValueDef, condcodes::IntCC, types,
};
use std::collections::BTreeMap;

#[path = "grouped_histogram_arm.rs"]
mod arm;

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(in crate::native_artifact) struct Admission {
    pub node_word: usize,
    pub bin_word: usize,
    pub minimum_bins: u64,
}

struct Group {
    header: Block,
    cursor: Value,
    end: Value,
    ordinal: Value,
    nodes: Value,
    bins: Value,
    packed: Value,
    length_mask: u32,
    key_mask: u32,
    key_shift: u32,
    literal: Inst,
    selected: usize,
    admission: Admission,
}

#[derive(Clone, Copy)]
enum Operand {
    Value(Value),
    Immediate(u64),
}

fn definition(f: &Function, value: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

fn constant(f: &Function, value: Value) -> Option<u64> {
    let mut steps = MAX_CONSTANT_STEPS;
    constant_integer(f, value, MAX_CONSTANT_DEPTH, &mut steps)
}

fn predicate(f: &Function, branch: Inst) -> Option<(IntCC, Value, Operand, [BlockCall; 2])> {
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
        return None;
    };
    let (cond, left, right) = match f.dfg.insts[comparison(f, arg)?] {
        Data::IntCompare { cond, args, .. } => (cond, args[0], Operand::Value(args[1])),
        Data::IntCompareImm { cond, arg, imm, .. } => {
            (cond, arg, Operand::Immediate(imm.bits() as u64))
        }
        _ => return None,
    };
    Some((cond, left, right, blocks))
}

fn with_constant(
    f: &Function,
    value: Value,
    opcode: Op,
    immediate: Op,
    commutative: bool,
) -> Option<(Value, u64)> {
    match definition(f, value)? {
        Data::BinaryImm64 { opcode, arg, imm } if opcode == immediate => {
            Some((f.dfg.resolve_aliases(arg), imm.bits() as u64))
        }
        Data::Binary {
            opcode: op,
            args: [a, b],
        } if op == opcode => constant(f, b)
            .map(|n| (f.dfg.resolve_aliases(a), n))
            .or_else(|| {
                if commutative {
                    constant(f, a).map(|n| (f.dfg.resolve_aliases(b), n))
                } else {
                    None
                }
            }),
        _ => None,
    }
}

fn invariant(ctx: &Context, value: Value, header: Block) -> bool {
    let f = &ctx.func;
    let value = f.dfg.resolve_aliases(value);
    if f.dfg.value_type(value) != types::I64 {
        return false;
    }
    let Some(first) = f.layout.first_inst(header) else {
        return false;
    };
    match f.dfg.value_def(value) {
        ValueDef::Result(inst, _) => inst != first && ctx.domtree.dominates(inst, first, &f.layout),
        ValueDef::Param(block, _) => {
            block != header && ctx.domtree.dominates(block, first, &f.layout)
        }
        ValueDef::Union(..) => false,
    }
}

fn recognize(
    ctx: &Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    nodes: &[View],
    bins: &[View],
) -> Option<Group> {
    let f = &ctx.func;
    let cfg = &ctx.cfg;
    let [cursor] = *f.dfg.block_params(header) else {
        return None;
    };
    if f.dfg.value_type(cursor) != types::I64 {
        return None;
    }
    let (boundary, _) = between(f, cfg, header)?;
    let (sense, left, Operand::Value(right), calls) = predicate(f, boundary)? else {
        return None;
    };
    let [left, right] = [left, right].map(|v| f.dfg.resolve_aliases(v));
    let end = match (left == cursor, right == cursor) {
        (true, false) => right,
        (false, true) => left,
        _ => return None,
    };
    if !matches!(sense, IntCC::Equal | IntCC::NotEqual) || !invariant(ctx, end, header) {
        return None;
    }
    let active = calls[usize::from(sense == IntCC::Equal)];
    let body = active.block(&f.dfg.value_lists);
    if active.args(&f.dfg.value_lists).next().is_some()
        || sole_predecessor(cfg, body)?.1 != boundary
    {
        return None;
    }
    let (read_branch, _) = between(f, cfg, body)?;
    let read = defaulted_read(f, cfg, loads, nodes, read_branch, Endianness::Little)?;
    let Data::Binary {
        opcode: Op::Isub,
        args: [top, index],
    } = definition(f, read.index)?
    else {
        return None;
    };
    if read.view.width != 8
        || f.dfg.resolve_aliases(index) != cursor
        || !invariant(ctx, top, header)
    {
        return None;
    }
    let (literal, _) = between(f, cfg, read.join)?;
    let (sense, mut length, right, calls) = predicate(f, literal)?;
    let expected = match right {
        Operand::Immediate(n) => n,
        Operand::Value(right) if constant(f, length) == Some(1) => {
            length = right;
            1
        }
        Operand::Value(right) => constant(f, right)?,
    };
    if expected != 1 || !matches!(sense, IntCC::Equal | IntCC::NotEqual) {
        return None;
    }
    let length = f.dfg.resolve_aliases(length);
    let (packed, length_mask) = with_constant(f, length, Op::Band, Op::BandImm, true)?;
    let (high, mask) = with_constant(f, packed, Op::Band, Op::BandImm, true)?;
    let (raw, 32) = with_constant(f, high, Op::Ushr, Op::UshrImm, false)? else {
        return None;
    };
    if [length, packed, high]
        .iter()
        .any(|&value| f.dfg.value_type(value) != types::I64)
    {
        return None;
    }
    // The default read produces zero and cannot select length one.
    if raw != read.result || mask != u64::from(u32::MAX) || length_mask & 1 == 0 {
        return None;
    }
    let selected = usize::from(sense == IntCC::NotEqual);
    let tail = arm::prove(
        ctx,
        calls[selected],
        header,
        cursor,
        length,
        packed,
        loads,
        bins,
    )?;
    // Four visits use the same snapshot even if a bin is updated in between.
    // Disallow packet reloads of the two views after the phase entry.
    let entry = f.layout.entry_block()?;
    let fixed = |view: &View| {
        [view.pointer_word, view.length_word]
            .into_iter()
            .all(|word| {
                loads
                    .iter()
                    .any(|(_, &(inst, at))| at == word && f.layout.inst_block(inst) == Some(entry))
                    && loads.iter().all(|(_, &(inst, at))| {
                        at != word || f.layout.inst_block(inst) == Some(entry)
                    })
            })
    };
    if !fixed(&read.view)
        || !fixed(tail.view)
        || read.view.length_word != read.view.pointer_word.checked_add(1)?
        || tail.view.length_word != tail.view.pointer_word.checked_add(1)?
    {
        return None;
    }
    let input = |word| {
        loads.iter().find_map(|(&value, &(_, index))| {
            (index == word && invariant(ctx, value, header)).then_some(value)
        })
    };
    Some(Group {
        header,
        cursor,
        end,
        ordinal: read.index,
        nodes: input(read.view.pointer_word)?,
        bins: input(tail.view.pointer_word)?,
        packed,
        length_mask: u32::try_from(length_mask).ok()?,
        key_mask: tail.key_mask,
        key_shift: tail.key_shift,
        literal,
        selected,
        admission: Admission {
            node_word: read.view.pointer_word,
            bin_word: tail.view.pointer_word,
            minimum_bins: u64::from(tail.key_mask) + 1,
        },
    })
}

fn block(f: &mut Function) -> Block {
    let block = f.dfg.make_block();
    f.layout.append_block(block);
    block
}

fn emit(f: &mut Function, group: Group) {
    let Data::Brif {
        arg, mut blocks, ..
    } = f.dfg.insts[group.literal]
    else {
        unreachable!("group was derived from an integer branch")
    };
    let fallback = blocks[group.selected];
    let scalar = fallback.block(&f.dfg.value_lists);
    let scalar_args = fallback.args(&f.dfg.value_lists).collect::<Vec<_>>();
    let check = block(f);
    let read = block(f);
    let update = block(f);
    blocks[group.selected] = BlockCall::new(check, [], &mut f.dfg.value_lists);
    f.dfg.insts[group.literal] = Data::Brif {
        opcode: Op::Brif,
        arg,
        blocks,
    };
    let mut c = FuncCursor::new(f);
    c.goto_bottom(check);
    let before_end = c
        .ins()
        .icmp(IntCC::UnsignedLessThan, group.cursor, group.end);
    let remaining = c.ins().isub(group.end, group.cursor);
    let enough = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 4);
    let readable = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, group.ordinal, 3);
    let enough = c.ins().band(before_end, enough);
    let ready = c.ins().band(enough, readable);
    c.ins().brif(ready, read, &[], scalar, &scalar_args);

    c.goto_bottom(read);
    let offset = c.ins().ishl_imm(group.ordinal, 3);
    let address = c.ins().iadd(group.nodes, offset);
    let flags = MemFlags::new().with_notrap();
    let first = c.ins().ireduce(types::I32, group.packed);
    let next = [-4, -12, -20].map(|at| c.ins().load(types::I32, flags, address, at));
    let [a, b, d] = next.map(|value| c.ins().bxor_imm(value, 1));
    let other = c.ins().bor(a, b);
    let other = c.ins().bor(other, d);
    let other = c.ins().band_imm(other, i64::from(group.length_mask));
    let literals = c.ins().icmp_imm(IntCC::Equal, other, 0);
    c.ins().brif(literals, update, &[], scalar, &scalar_args);

    c.goto_bottom(update);
    for packed in [first, next[0], next[1], next[2]] {
        let key = c.ins().ushr_imm(packed, i64::from(group.key_shift));
        let key = c.ins().band_imm(key, i64::from(group.key_mask));
        let key = c.ins().uextend(types::I64, key);
        let offset = c.ins().ishl_imm(key, 2);
        let address = c.ins().iadd(group.bins, offset);
        // A later visit must load the new count when two keys agree.
        let count = c.ins().load(types::I32, flags, address, 0);
        let updated = c.ins().iadd_imm(count, 1);
        c.ins().store(flags, updated, address, 0);
    }
    let next = c.ins().iadd_imm(group.cursor, 4);
    c.ins().jump(group.header, &[next.into()]);
}

pub(in crate::native_artifact) fn optimize(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    nodes: &[View],
    bins: &[View],
) -> Option<Admission> {
    ctx.compute_cfg();
    ctx.compute_domtree();
    let group = recognize(ctx, header, loads, nodes, bins)?;
    let admission = group.admission;
    emit(&mut ctx.func, group);
    ctx.compute_cfg();
    ctx.compute_domtree();
    Some(admission)
}

// Check the current packet once on entry. The phase caller has already
// admitted exclusive writable owners and their initialized lengths.
pub(in crate::native_artifact) fn guard_entry(
    context: &mut Context,
    admission: Admission,
    fast: FuncRef,
) {
    let f = &mut context.func;
    let original = f.layout.entry_block().expect("numeric phase has an entry");
    let entry = f.dfg.make_block();
    f.layout.insert_block(entry, original);
    let ready = block(f);
    let inputs = [(); 3].map(|()| f.dfg.append_block_param(entry, types::I64));
    let [_, packet, _] = inputs;
    let mut cursor = FuncCursor::new(f);
    cursor.goto_bottom(entry);
    let mut load = |word| {
        cursor.ins().load(
            types::I64,
            MemFlags::new(),
            packet,
            i32::try_from(word * 8).expect("word was loaded from the input packet"),
        )
    };
    let start = load(admission.node_word);
    let len = load(admission.node_word + 1);
    let bins = load(admission.bin_word);
    let count = load(admission.bin_word + 1);
    let fits = cursor
        .ins()
        .icmp_imm(IntCC::UnsignedLessThanOrEqual, len, (u64::MAX / 8) as i64);
    let bytes = cursor.ins().ishl_imm(len, 3);
    let end = cursor.ins().iadd(start, bytes);
    let node_end = cursor
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, end, start);
    let enough = cursor.ins().icmp_imm(
        IntCC::UnsignedGreaterThanOrEqual,
        count,
        admission.minimum_bins as i64,
    );
    let bin_end = cursor
        .ins()
        .iadd_imm(bins, (admission.minimum_bins * 4) as i64);
    let bin_fits = cursor
        .ins()
        .icmp(IntCC::UnsignedGreaterThanOrEqual, bin_end, bins);
    let before = cursor.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, bins);
    let after = cursor
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, bin_end, start);
    let disjoint = cursor.ins().bor(before, after);
    let nodes = cursor.ins().band(fits, node_end);
    let counts = cursor.ins().band(enough, bin_fits);
    let admitted = cursor.ins().band(nodes, counts);
    let admitted = cursor.ins().band(admitted, disjoint);
    cursor
        .ins()
        .brif(admitted, ready, &[], original, &inputs.map(Into::into));
    cursor.goto_bottom(ready);
    let call = cursor.ins().call(fast, &inputs);
    let [status, at] = *cursor.func.dfg.inst_results(call) else {
        unreachable!("numeric phase alternatives have the same signature")
    };
    cursor.ins().return_(&[status, at]);
    context.compute_cfg();
    context.compute_domtree();
}
