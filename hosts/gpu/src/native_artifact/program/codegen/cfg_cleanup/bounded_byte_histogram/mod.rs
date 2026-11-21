//! Hoist four bounds checks out of a fixed byte-indexed histogram loop.
//! The caller retains the unique-owner/current-view admission. A fresh entry
//! guard proves the entire source span and 256-bin extent; a proof miss leaves
//! the original graph unchanged. An optional counter-capacity admission also
//! proves that every existing bin can accept the entire remaining input span.
use super::composed_reads::{View, between, defaulted_read, definition, packet_word, scalar};
use super::window_bounds::{comparison, sole_predecessor};
use super::word_ranges::{constant_offset, literal};
use cranelift_codegen::Context;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockCall, Endianness, FuncRef, Function, Inst, InstBuilder, InstructionData as Data,
    MemFlags, Opcode, Value, ValueDef, condcodes::IntCC, types,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(in crate::native_artifact) struct Admission {
    pub input_word: usize,
    pub bin_word: usize,
    pub begin_word: usize,
    pub end_word: usize,
    pub cursor_word: usize,
}

fn branch(f: &Function, inst: Inst) -> Option<[BlockCall; 2]> {
    match f.dfg.insts[inst] {
        Data::Brif { blocks, .. } => Some(blocks),
        _ => None,
    }
}
fn test(f: &Function, inst: Inst) -> Option<(IntCC, Value, Value)> {
    let Data::Brif { arg, .. } = f.dfg.insts[inst] else {
        return None;
    };
    match f.dfg.insts[comparison(f, arg)?] {
        Data::IntCompare {
            cond, args: [a, b], ..
        } => Some((cond, f.dfg.resolve_aliases(a), f.dfg.resolve_aliases(b))),
        _ => None,
    }
}
fn next(f: &Function, cfg: &ControlFlowGraph, inst: Inst, edge: usize) -> Option<Block> {
    let call = branch(f, inst)?[edge];
    let block = call.block(&f.dfg.value_lists);
    (call.args(&f.dfg.value_lists).next().is_none() && sole_predecessor(cfg, block)?.1 == inst)
        .then_some(block)
}
fn same(f: &Function, a: Value, b: Value) -> bool {
    constant_offset(f, a) == constant_offset(f, b)
}
// Follow only pure copies/constant additions and unanimous block arguments.
// The loop cursor is an explicit leaf, so backedges cannot make this cyclic.
pub(super) fn offset(
    f: &Function,
    cfg: &ControlFlowGraph,
    value: Value,
    cursor: Value,
    active: &mut BTreeSet<Value>,
    fuel: &mut usize,
) -> Option<u64> {
    *fuel = fuel.checked_sub(1)?;
    let value = f.dfg.resolve_aliases(value);
    if value == cursor {
        return Some(0);
    }
    if !active.insert(value) {
        return None;
    }
    let result = match f.dfg.value_def(value) {
        ValueDef::Param(block, slot) => {
            let mut result = None;
            for pred in cfg.pred_iter(block) {
                for call in f.dfg.insts[pred.inst]
                    .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
                {
                    if call.block(&f.dfg.value_lists) != block {
                        continue;
                    }
                    let value = call.args(&f.dfg.value_lists).nth(slot)?.as_value()?;
                    let found = offset(f, cfg, value, cursor, active, fuel)?;
                    if result.is_some_and(|old| old != found) {
                        return None;
                    }
                    result = Some(found);
                }
            }
            result
        }
        ValueDef::Result(inst, 0) => match f.dfg.insts[inst] {
            Data::BinaryImm64 {
                opcode: Opcode::IaddImm,
                arg,
                imm,
            } => {
                offset(f, cfg, arg, cursor, active, fuel).map(|n| n.wrapping_add(imm.bits() as u64))
            }
            Data::Binary {
                opcode: Opcode::Iadd,
                args: [a, b],
            } => {
                if let Some(n) = literal(f, b) {
                    offset(f, cfg, a, cursor, active, fuel).map(|k| k.wrapping_add(n))
                } else if let Some(n) = literal(f, a) {
                    offset(f, cfg, b, cursor, active, fuel).map(|k| k.wrapping_add(n))
                } else {
                    None
                }
            }
            _ => None,
        },
        _ => None,
    };
    active.remove(&value);
    result
}
pub(super) fn fixed_word(
    f: &Function,
    cfg: &ControlFlowGraph,
    loads: &BTreeMap<Value, (Inst, usize)>,
    value: Value,
) -> Option<usize> {
    let word = packet_word(f, cfg, loads, value)?;
    let entry = f.layout.entry_block()?;
    let mut found = loads.iter().filter(|(_, (_, at))| *at == word);
    let (_, &(inst, _)) = found.next()?;
    (found.next().is_none() && f.layout.inst_block(inst) == Some(entry)).then_some(word)
}
struct Histogram {
    admission: Admission,
    bounds: [Inst; 4],
    increment: Inst,
    updated: Value,
    index: Value,
    bins: View,
}

fn prove(
    ctx: &Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    input: &[View],
    bins: &[View],
) -> Option<Histogram> {
    let f = &ctx.func;
    let cfg = &ctx.cfg;
    if f.layout.blocks().count() > 512
        || f.layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .count()
            > 4096
    {
        return None;
    }
    let [cursor] = *f.dfg.block_params(header) else {
        return None;
    };
    if f.dfg.value_type(cursor) != types::I64 {
        return None;
    }
    let (boundary, _) = between(f, cfg, header)?;
    let (IntCC::UnsignedLessThan, left, end) = test(f, boundary)? else {
        return None;
    };
    if left != cursor {
        return None;
    }
    let end_word = fixed_word(f, cfg, loads, end)?;
    let mut initial = None;
    let mut backedges = 0;
    for pred in cfg.pred_iter(header) {
        for call in
            f.dfg.insts[pred.inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
        {
            if call.block(&f.dfg.value_lists) != header {
                continue;
            }
            let value = call.args(&f.dfg.value_lists).next()?.as_value()?;
            let step = offset(f, cfg, value, cursor, &mut BTreeSet::new(), &mut 512);
            if step == Some(1) {
                if !ctx.domtree.block_dominates(header, pred.block) {
                    return None;
                }
                backedges += 1;
            } else {
                let word = fixed_word(f, cfg, loads, value)?;
                if ctx.domtree.block_dominates(header, pred.block)
                    || initial.is_some_and(|old| old != word)
                {
                    return None;
                }
                initial = Some(word);
            }
        }
    }
    if backedges == 0 {
        return None;
    }
    let cursor_word = initial?;
    let (addition, _) = between(f, cfg, next(f, cfg, boundary, 0)?)?;
    let (IntCC::UnsignedGreaterThanOrEqual, position, begin) = test(f, addition)? else {
        return None;
    };
    let (
        _,
        Data::Binary {
            opcode: Opcode::Iadd,
            args,
        },
    ) = definition(f, position)?
    else {
        return None;
    };
    if args.map(|v| f.dfg.resolve_aliases(v)) != [begin, cursor]
        && args.map(|v| f.dfg.resolve_aliases(v)) != [cursor, begin]
    {
        return None;
    }
    let begin_word = fixed_word(f, cfg, loads, begin)?;
    let (read_branch, _) = between(f, cfg, next(f, cfg, addition, 0)?)?;
    let read = defaulted_read(f, cfg, loads, input, read_branch, Endianness::Little)?;
    if read.view.width != 1 || !same(f, read.index, position) {
        return None;
    }
    let (bin_branch, _) = between(f, cfg, read.join)?;
    let count = defaulted_read(f, cfg, loads, bins, bin_branch, Endianness::Little)?;
    if count.view.width != 4 || !same(f, count.index, read.result) {
        return None;
    }
    let (increment, _) = between(f, cfg, count.join)?;
    // Preserve the source increment/overflow branch verbatim. It is merely
    // the boundary from the defaulting count read to the checked write.
    let Data::Brif { arg, .. } = f.dfg.insts[increment] else {
        return None;
    };
    let condition = comparison(f, arg)?;
    let updated = match f.dfg.insts[condition] {
        Data::IntCompareImm {
            cond: IntCC::UnsignedLessThanOrEqual,
            arg,
            imm,
            ..
        } if imm.bits() as u64 == u64::from(u32::MAX) => arg,
        Data::IntCompare {
            cond: IntCC::UnsignedLessThanOrEqual,
            args: [value, limit],
            ..
        } if literal(f, limit) == Some(u64::from(u32::MAX)) => value,
        _ => return None,
    };
    let (base, amount) = constant_offset(f, updated);
    if base != f.dfg.resolve_aliases(count.result) || amount != 1 {
        return None;
    }
    let (write_branch, _) = between(f, cfg, next(f, cfg, increment, 0)?)?;
    let (IntCC::UnsignedLessThan, index, length) = test(f, write_branch)? else {
        return None;
    };
    if !same(f, index, read.result) {
        return None;
    }
    // The write must use exactly the count view's current length, rather than
    // a different view that happens to have the same width.
    if fixed_word(f, cfg, loads, length)? != count.view.length_word {
        return None;
    }
    for word in [
        read.view.pointer_word,
        read.view.length_word,
        count.view.pointer_word,
        count.view.length_word,
    ] {
        let value = loads
            .iter()
            .find_map(|(&v, &(_, at))| (at == word).then_some(v))?;
        if fixed_word(f, cfg, loads, value)? != word {
            return None;
        }
    }
    if read.view.length_word != read.view.pointer_word.checked_add(1)?
        || count.view.length_word != count.view.pointer_word.checked_add(1)?
    {
        return None;
    }
    Some(Histogram {
        admission: Admission {
            input_word: read.view.pointer_word,
            bin_word: count.view.pointer_word,
            begin_word,
            end_word,
            cursor_word,
        },
        bounds: [addition, read_branch, bin_branch, write_branch],
        increment,
        updated,
        index: read.result,
        bins: count.view,
    })
}

// The entry capacity bound is valid only if source work cannot change a bin
// before the loop, and each visit writes exactly its checked count + 1 once.
// The earlier recognition already admits only scalar setup and the two typed
// defaulting reads before this selected write arm.
fn counter_capacity(
    ctx: &Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    histogram: &Histogram,
) -> Option<()> {
    let f = &ctx.func;
    let cfg = &ctx.cfg;
    let mut at = f.layout.entry_block()?;
    let mut visited = BTreeSet::new();
    while at != header {
        if !visited.insert(at) {
            return None;
        }
        let last = f.layout.last_inst(at)?;
        for inst in f.layout.block_insts(at).take_while(|&i| i != last) {
            if !scalar(f.dfg.insts[inst].opcode()) && !loads.values().any(|&(load, _)| load == inst)
            {
                return None;
            }
        }
        let Data::Jump { destination, .. } = f.dfg.insts[last] else {
            return None;
        };
        at = destination.block(&f.dfg.value_lists);
    }

    let matches_address = |address| {
        let Some((
            _,
            Data::Binary {
                opcode: Opcode::Iadd,
                args,
            },
        )) = definition(f, address)
        else {
            return false;
        };
        let scaled = |pointer, bytes| {
            if packet_word(f, cfg, loads, pointer) != Some(histogram.bins.pointer_word) {
                return false;
            }
            match definition(f, bytes) {
                Some((
                    _,
                    Data::BinaryImm64 {
                        opcode: Opcode::ImulImm,
                        arg,
                        imm,
                    },
                )) if imm.bits() == 4 => same(f, arg, histogram.index),
                Some((
                    _,
                    Data::BinaryImm64 {
                        opcode: Opcode::IshlImm,
                        arg,
                        imm,
                    },
                )) if imm.bits() == 2 => same(f, arg, histogram.index),
                _ => false,
            }
        };
        scaled(args[0], args[1]) || scaled(args[1], args[0])
    };
    at = next(f, cfg, histogram.bounds[3], 0)?;
    visited.clear();
    let mut stores = 0;
    loop {
        if !visited.insert(at) {
            return None;
        }
        let last = f.layout.last_inst(at)?;
        for inst in f.layout.block_insts(at).take_while(|&i| i != last) {
            match f.dfg.insts[inst] {
                Data::Store {
                    args: [value, address],
                    offset,
                    flags,
                    ..
                } if i32::from(offset) == 0
                    && flags.endianness(Endianness::Little) == Endianness::Little
                    && f.dfg.value_type(value) == types::I32
                    && matches_address(address)
                    && matches!(definition(f, value), Some((_, Data::Unary {
                            opcode: Opcode::Ireduce, arg,
                        })) if f.dfg.resolve_aliases(arg) == f.dfg.resolve_aliases(histogram.updated)) =>
                {
                    stores += 1;
                }
                data if scalar(data.opcode()) => {}
                _ => return None,
            }
        }
        let Data::Jump { destination, .. } = f.dfg.insts[last] else {
            return None;
        };
        let target = destination.block(&f.dfg.value_lists);
        if target == header {
            return (stores == 1).then_some(());
        }
        if sole_predecessor(cfg, target)?.1 != last {
            return None;
        }
        at = target;
    }
}

fn remove_checks(ctx: &mut Context, checks: impl IntoIterator<Item = Inst>) {
    for inst in checks {
        let call = branch(&ctx.func, inst).expect("recognized histogram check")[0];
        let target = call.block(&ctx.func.dfg.value_lists);
        let args = call.args(&ctx.func.dfg.value_lists).collect::<Vec<_>>();
        ctx.func.dfg.replace(inst).jump(target, &args);
    }
    ctx.compute_cfg();
    ctx.compute_domtree();
}

pub(in crate::native_artifact) fn optimize(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    input: &[View],
    bins: &[View],
) -> Option<Admission> {
    ctx.compute_cfg();
    ctx.compute_domtree();
    let histogram = prove(ctx, header, loads, input, bins)?;
    remove_checks(ctx, histogram.bounds);
    Some(histogram.admission)
}

pub(in crate::native_artifact) fn optimize_counter_capacity(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    input: &[View],
    bins: &[View],
) -> Option<Admission> {
    optimize_capacity(ctx, header, loads, input, bins, false)
}

pub(in crate::native_artifact) fn optimize_grouped_counter_capacity(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    input: &[View],
    bins: &[View],
) -> Option<Admission> {
    optimize_capacity(ctx, header, loads, input, bins, true)
}

fn optimize_capacity(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    input: &[View],
    bins: &[View],
    grouped: bool,
) -> Option<Admission> {
    ctx.compute_cfg();
    ctx.compute_domtree();
    let histogram = prove(ctx, header, loads, input, bins)?;
    counter_capacity(ctx, header, loads, &histogram)?;
    remove_checks(
        ctx,
        histogram.bounds.into_iter().chain([histogram.increment]),
    );
    if grouped {
        group_visits(ctx, header, loads, histogram.admission);
    }
    Some(histogram.admission)
}

// Only the exact, effect-checked histogram reaches this construction. Four
// visits preserve read/increment/write order, even when the views alias. The
// original admitted loop handles the last zero to three bytes unchanged.
fn group_visits(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    a: Admission,
) {
    let f = &mut ctx.func;
    let field = |word| {
        loads
            .iter()
            .find_map(|(&v, &(_, at))| (word == at).then_some(v))
            .expect("proved invariant histogram field")
    };
    let begin = field(a.begin_word);
    let end = field(a.end_word);
    let input = field(a.input_word);
    let bins = field(a.bin_word);
    let cursor = f.dfg.block_params(header)[0];
    let original = f.layout.last_inst(header).expect("proved loop header");
    let tail = block(f);
    let group = block(f);
    f.layout.remove_inst(original);
    f.layout.append_inst(original, tail);
    let mut c = FuncCursor::new(f);
    c.goto_bottom(header);
    let remaining = c.ins().isub(end, cursor);
    let four = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 4);
    c.ins().brif(four, group, &[], tail, &[]);
    c.goto_bottom(group);
    let position = c.ins().iadd(begin, cursor);
    for offset in 0..4 {
        let position = c.ins().iadd_imm(position, offset);
        let source = c.ins().iadd(input, position);
        let byte = c.ins().uload8(types::I64, MemFlags::new(), source, 0);
        let bytes = c.ins().ishl_imm(byte, 2);
        let destination = c.ins().iadd(bins, bytes);
        let count = c.ins().uload32(MemFlags::new(), destination, 0);
        let increment = c.ins().iadd_imm(count, 1);
        let value = c.ins().ireduce(types::I32, increment);
        c.ins().store(MemFlags::new(), value, destination, 0);
    }
    let next = c.ins().iadd_imm(cursor, 4);
    c.ins().jump(header, &[next.into()]);
    ctx.compute_cfg();
    ctx.compute_domtree();
}

fn block(f: &mut Function) -> Block {
    let b = f.dfg.make_block();
    f.layout.append_block(b);
    b
}

// These are runtime conditions on the original packet, before source effects.
// Source initialization is not assumed: overflowing existing counts still fail
// at the original checked increment in the specialized function.
pub(in crate::native_artifact) fn guard_entry(ctx: &mut Context, a: Admission, fast: FuncRef) {
    guard(ctx, a, fast, false, false);
}

pub(in crate::native_artifact) fn guard_counter_capacity(
    ctx: &mut Context,
    a: Admission,
    fast: FuncRef,
) {
    guard(ctx, a, fast, true, false);
}

pub(in crate::native_artifact) fn guard_vector_counter_capacity(
    ctx: &mut Context,
    a: Admission,
    fast: FuncRef,
) {
    guard(ctx, a, fast, true, true);
}

fn guard(ctx: &mut Context, a: Admission, fast: FuncRef, counters: bool, vector: bool) {
    let f = &mut ctx.func;
    let original = f.layout.entry_block().expect("numeric entry");
    let entry = f.dfg.make_block();
    f.layout.insert_block(entry, original);
    let ready = block(f);
    let inputs = [(); 3].map(|()| f.dfg.append_block_param(entry, types::I64));
    let [_, packet, _] = inputs;
    let mut c = FuncCursor::new(f);
    c.goto_bottom(entry);
    let mut load = |word: usize| {
        c.ins().load(
            types::I64,
            MemFlags::new(),
            packet,
            i32::try_from(word * 8).expect("admitted packet word"),
        )
    };
    let begin = load(a.begin_word);
    let end = load(a.end_word);
    let at = load(a.cursor_word);
    let input_len = load(a.input_word + 1);
    let bins = load(a.bin_word + 1);
    let start = c
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, begin, input_len);
    let remaining = c.ins().isub(input_len, begin);
    let span = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, remaining);
    let cursor = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, at, end);
    let histogram = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, bins, 256);
    let left = c.ins().band(start, span);
    let right = c.ins().band(cursor, histogram);
    let admitted = c.ins().band(left, right);
    if counters {
        // The span is end - cursor, not end - begin: begin is an input offset.
        // Bounds admission precedes all extra reads. Empty loops need no scan.
        let work = c.ins().isub(end, at);
        let fits = c
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, work, u32::MAX as i64);
        let admitted = c.ins().band(admitted, fits);
        let check = block(c.func);
        let scan = block(c.func);
        let step = block(c.func);
        let index = c.func.dfg.append_block_param(scan, types::I64);
        c.ins()
            .brif(admitted, check, &[], original, &inputs.map(Into::into));
        c.goto_bottom(check);
        let pointer = c.ins().load(
            types::I64,
            MemFlags::new(),
            packet,
            i32::try_from(a.bin_word * 8).expect("admitted bin packet word"),
        );
        let maximum = c.ins().iconst(types::I64, u32::MAX as i64);
        let limit = c.ins().isub(maximum, work);
        let zero = c.ins().iconst(types::I64, 0);
        let empty = c.ins().icmp_imm(IntCC::Equal, work, 0);
        if vector {
            let maximum = c.func.dfg.append_block_param(scan, types::I32X4);
            let final_maximum = c.func.dfg.append_block_param(step, types::I32X4);
            let zero32 = c.ins().iconst(types::I32, 0);
            let zeroes = c.ins().splat(types::I32X4, zero32);
            c.ins()
                .brif(empty, ready, &[], scan, &[zero.into(), zeroes.into()]);
            c.goto_bottom(scan);
            let offset = c.ins().ishl_imm(index, 4);
            let address = c.ins().iadd(pointer, offset);
            let counts = c.ins().load(types::I32X4, MemFlags::new(), address, 0);
            let maximum = c.ins().umax(maximum, counts);
            let next = c.ins().iadd_imm(index, 1);
            let complete = c.ins().icmp_imm(IntCC::Equal, next, 64);
            c.ins().brif(
                complete,
                step,
                &[maximum.into()],
                scan,
                &[next.into(), maximum.into()],
            );
            c.goto_bottom(step);
            let mut maximum = c.ins().extractlane(final_maximum, 0);
            for lane in 1..4 {
                let value = c.ins().extractlane(final_maximum, lane);
                maximum = c.ins().umax(maximum, value);
            }
            let maximum = c.ins().uextend(types::I64, maximum);
            let safe = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, maximum, limit);
            c.ins()
                .brif(safe, ready, &[], original, &inputs.map(Into::into));
        } else {
            c.ins().brif(empty, ready, &[], scan, &[zero.into()]);
            c.goto_bottom(scan);
            let offset = c.ins().ishl_imm(index, 2);
            let address = c.ins().iadd(pointer, offset);
            let count = c.ins().uload32(MemFlags::new(), address, 0);
            let safe = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, count, limit);
            let next = c.ins().iadd_imm(index, 1);
            c.ins()
                .brif(safe, step, &[], original, &inputs.map(Into::into));
            c.goto_bottom(step);
            let complete = c.ins().icmp_imm(IntCC::Equal, next, 256);
            c.ins().brif(complete, ready, &[], scan, &[next.into()]);
        }
    } else {
        c.ins()
            .brif(admitted, ready, &[], original, &inputs.map(Into::into));
    }
    c.goto_bottom(ready);
    let call = c.ins().call(fast, &inputs);
    let results = c.func.dfg.inst_results(call).to_vec();
    c.ins().return_(&results);
    ctx.compute_cfg();
    ctx.compute_domtree();
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;
