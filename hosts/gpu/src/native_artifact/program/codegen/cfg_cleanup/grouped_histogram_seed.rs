//! Reuse an admitted byte histogram and correct the bytes covered by matches.
//! The match and checked retry paths keep the original tally operations.
use super::super::{bounded_byte_histogram::fixed_word, word_ranges};
use super::*;
use cranelift_codegen::ir::{AbiParam, StackSlotData, StackSlotKind};
use std::collections::BTreeSet;

const PREFIX: u64 = 256;
const CHUNK: u64 = 64;
const MINIMUM_INPUT: i64 = 8 * PREFIX as i64;

pub(in crate::native_artifact) struct Seeded {
    original: Function,
    group: Group,
    initial: Inst,
    start_word: usize,
    top_word: usize,
    end_word: usize,
    lookups: BTreeSet<usize>,
    latches: Vec<Inst>,
    failures: Vec<Inst>,
}

fn edges(f: &Function, inst: Inst) -> &[BlockCall] {
    f.dfg.insts[inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
}

// Check the retained CFG as a DAG bounded by its original header. The
// replaced literal arm and the first visit's packet loads are outside it.
fn traversal(ctx: &Context, group: Group, first: Block, body: bool) -> Option<BTreeSet<Block>> {
    let f = &ctx.func;
    let mut pending = vec![first];
    let mut region = BTreeSet::new();
    let mut successors = BTreeMap::new();
    while let Some(at) = pending.pop() {
        if !region.insert(at) {
            continue;
        }
        let last = f.layout.last_inst(at)?;
        let destinations = match f.dfg.insts[last].opcode() {
            Op::Return => &[][..],
            Op::Jump | Op::Brif if body => {
                let calls = edges(f, last);
                if last == group.literal {
                    &calls[(1 - group.selected)..(2 - group.selected)]
                } else {
                    calls
                }
            }
            Op::Jump => edges(f, last),
            _ => return None,
        };
        let mut next = Vec::new();
        for to in destinations
            .iter()
            .map(|call| call.block(&f.dfg.value_lists))
        {
            if to == group.header {
                if !body {
                    return None;
                }
            } else {
                pending.push(to);
                next.push(to);
            }
        }
        successors.insert(at, next);
    }
    let mut incoming = region
        .iter()
        .map(|&b| (b, 0usize))
        .collect::<BTreeMap<_, _>>();
    for next in successors.values() {
        for b in next {
            *incoming.get_mut(b)? += 1;
        }
    }
    pending.extend(
        incoming
            .iter()
            .filter_map(|(&b, &degree)| (degree == 0).then_some(b)),
    );
    let mut count = 0;
    while let Some(b) = pending.pop() {
        count += 1;
        for next in &successors[&b] {
            let degree = incoming.get_mut(next)?;
            *degree -= 1;
            if *degree == 0 {
                pending.push(*next);
            }
        }
    }
    (count == region.len()).then_some(region)
}

fn forwarded(ctx: &Context, value: Value, expected: Value, fuel: &mut usize) -> Option<()> {
    *fuel = fuel.checked_sub(1)?;
    let f = &ctx.func;
    let value = f.dfg.resolve_aliases(value);
    if value == expected {
        return Some(());
    }
    let ValueDef::Param(block, slot) = f.dfg.value_def(value) else {
        return None;
    };
    let mut found = false;
    for pred in ctx.cfg.pred_iter(block) {
        for edge in edges(f, pred.inst) {
            if edge.block(&f.dfg.value_lists) == block {
                let word = edge.args(&f.dfg.value_lists).nth(slot)?.as_value()?;
                forwarded(ctx, word, expected, fuel)?;
                found = true;
            }
        }
    }
    found.then_some(())
}

fn latch(ctx: &Context, group: Group, last: Inst) -> Option<()> {
    let f = &ctx.func;
    let Data::Jump { destination, .. } = f.dfg.insts[last] else {
        return None;
    };
    let next = destination.args(&f.dfg.value_lists).next()?.as_value()?;
    let Data::Binary {
        opcode: Op::Iadd,
        args,
    } = definition(f, next)?
    else {
        return None;
    };
    let [a, b] = args.map(|v| f.dfg.resolve_aliases(v));
    let previous = if a == group.length {
        b
    } else if b == group.length {
        a
    } else {
        return None;
    };
    forwarded(ctx, previous, group.cursor, &mut 128)
}

fn counter_at(f: &Function, facts: &mut word_ranges::Facts<'_>, byte_index: Value) -> Option<()> {
    let Data::BinaryImm64 { opcode, arg, imm } = definition(f, byte_index)? else {
        return None;
    };
    if !matches!((opcode, imm.bits()), (Op::ImulImm, 4) | (Op::IshlImm, 2)) {
        return None;
    }
    let (base, bias) = word_ranges::constant_offset(f, arg);
    let span = facts.value_range(base)?;
    (span.0.checked_add(bias)? >= PREFIX && span.1.checked_add(bias)? <= u64::MAX / 4).then_some(())
}

fn access(f: &Function, inst: Inst) -> Option<(Value, bool, types::Type, Op)> {
    let opcode = f.dfg.insts[inst].opcode();
    match f.dfg.insts[inst] {
        Data::Load {
            arg, offset, flags, ..
        } if i32::from(offset) == 0
            && flags.endianness(Endianness::Little) == Endianness::Little =>
        {
            Some((
                arg,
                false,
                f.dfg.value_type(f.dfg.first_result(inst)),
                opcode,
            ))
        }
        Data::Store {
            args: [value, address],
            offset,
            flags,
            ..
        } if i32::from(offset) == 0
            && flags.endianness(Endianness::Little) == Endianness::Little =>
        {
            Some((address, true, f.dfg.value_type(value), opcode))
        }
        _ => None,
    }
}

pub(in crate::native_artifact) fn prepare(
    ctx: &mut Context,
    header: Block,
    loads: &BTreeMap<Value, (Inst, usize)>,
    nodes: &[View],
    bins: &[View],
    lookups: &[View],
) -> Option<Seeded> {
    ctx.compute_cfg();
    ctx.compute_domtree();
    let group = recognize(ctx, header, loads, nodes, bins)?;
    if (group.length_mask, group.key_shift, group.key_mask) != (511, 9, 255) {
        return None;
    }
    let f = &ctx.func;
    let entry = f.layout.entry_block()?;
    let [_, packet, out] = *f.dfg.block_params(entry) else {
        return None;
    };
    let initial = f.layout.last_inst(entry)?;
    let Data::Jump { destination, .. } = f.dfg.insts[initial] else {
        return None;
    };
    if destination.block(&f.dfg.value_lists) != header {
        return None;
    }
    let word = |v| fixed_word(f, &ctx.cfg, loads, v);
    let cursor = destination.args(&f.dfg.value_lists).next()?.as_value()?;
    let start_word = word(cursor)?;
    let top_word = word(group.top)?;
    let end_word = group.admission.end_word?;
    for inst in f
        .layout
        .block_insts(entry)
        .take_while(|&inst| inst != initial)
    {
        if !scalar(f.dfg.insts[inst].opcode()) && !loads.values().any(|&(load, _)| load == inst) {
            return None;
        }
    }
    let body = traversal(ctx, group, group.body, true)?;
    let done = traversal(ctx, group, group.done, false)?;
    if !body.is_disjoint(&done) {
        return None;
    }
    let mut facts = word_ranges::Facts::across_edges(f, &ctx.cfg, None);
    let mut failures = Vec::new();
    let mut latches = Vec::new();
    let mut captured = BTreeSet::new();
    for &block in &done {
        let last = f.layout.last_inst(block)?;
        if f.layout
            .block_insts(block)
            .take_while(|&inst| inst != last)
            .any(|inst| !scalar(f.dfg.insts[inst].opcode()))
        {
            return None;
        }
        if f.dfg.insts[last].opcode() == Op::Return
            && !f
                .dfg
                .inst_args(last)
                .iter()
                .all(|&v| constant(f, v) == Some(0))
        {
            return None;
        }
    }
    for &block in &body {
        let last = f.layout.last_inst(block)?;
        if f.dfg.insts[last].opcode() == Op::Return {
            let [status, _] = *f.dfg.inst_args(last) else {
                return None;
            };
            if facts.value_range(status)?.0 == 0 {
                return None;
            }
            failures.push(last);
        } else if edges(f, last)
            .iter()
            .any(|edge| edge.block(&f.dfg.value_lists) == header)
        {
            latch(ctx, group, last)?;
            latches.push(last);
        }
        for inst in f.layout.block_insts(block).take_while(|&inst| inst != last) {
            if scalar(f.dfg.insts[inst].opcode()) {
                continue;
            }
            let (address, store, ty, opcode) = access(f, inst)?;
            let address = f.dfg.resolve_aliases(address);
            if address == out && matches!((opcode, ty), (Op::Load | Op::Store, types::I64)) {
                // Isolate the checked status lane in the frame. The old
                // match may use it only on a path that ends in failure.
                let part = traversal(ctx, group, block, true)?;
                if part.iter().any(|&b| {
                    f.layout.last_inst(b).is_some_and(|i| {
                        edges(f, i)
                            .iter()
                            .any(|e| e.block(&f.dfg.value_lists) == header)
                    })
                }) {
                    return None;
                }
                continue;
            }
            let Data::Binary {
                opcode: Op::Iadd,
                args: [a, b],
            } = definition(f, address)?
            else {
                return None;
            };
            let [a, b] = [a, b].map(|v| f.dfg.resolve_aliases(v));
            if a == group.bins || b == group.bins {
                if !matches!(
                    (opcode, ty),
                    (Op::Uload32, types::I64) | (Op::Store, types::I32)
                ) {
                    return None;
                }
                counter_at(f, &mut facts, if a == group.bins { b } else { a })?;
            } else if !store
                && (a == group.nodes || b == group.nodes)
                && (opcode, ty) == (Op::Load, types::I64)
            {
                // The descending node read keeps its original bounds branch.
            } else if !store && matches!(opcode, Op::Uload8) {
                let base = [a, b].into_iter().find_map(|v| {
                    let pointer = word(v)?;
                    lookups
                        .iter()
                        .find(|view| {
                            view.width == 1
                                && view.pointer_word == pointer
                                && view.length_word == pointer + 1
                        })
                        .map(|_| pointer)
                })?;
                if !loads
                    .iter()
                    .any(|(&v, &(_, index))| index == base + 1 && word(v) == Some(index))
                {
                    return None;
                }
                captured.insert(base);
            } else {
                return None;
            }
        }
    }
    // Rewriting every retained latch makes the new progress test common to
    // literals and matches. No other entry may flow through old code first.
    if latches.is_empty()
        || ctx.cfg.pred_iter(header).any(|pred| {
            pred.inst != initial
                && !latches.contains(&pred.inst)
                && !ctx.domtree.block_dominates(header, pred.block)
        })
        || f.dfg.value_type(packet) != types::I64
    {
        return None;
    }
    Some(Seeded {
        original: f.clone(),
        group,
        initial,
        start_word,
        top_word,
        end_word,
        lookups: captured,
        latches,
        failures,
    })
}

fn packet(c: &mut FuncCursor<'_>, input: Value, word: usize) -> Value {
    c.ins().load(
        types::I64,
        MemFlags::new().with_notrap().with_readonly(),
        input,
        i32::try_from(word * 8).expect("proved phase input word"),
    )
}

fn disjoint(
    c: &mut FuncCursor<'_>,
    bins: Value,
    bin_end: Value,
    base: Value,
    length: Value,
    width: u64,
) -> Value {
    let length_fits = c.ins().icmp_imm(
        IntCC::UnsignedLessThanOrEqual,
        length,
        (u64::MAX / width) as i64,
    );
    let bytes = c.ins().imul_imm(length, width as i64);
    let end = c.ins().iadd(base, bytes);
    let address_fits = c.ins().icmp(IntCC::UnsignedGreaterThanOrEqual, end, base);
    let before = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, end, bins);
    let after = c.ins().icmp(IntCC::UnsignedLessThanOrEqual, bin_end, base);
    let separate = c.ins().bor(before, after);
    let valid = c.ins().band(length_fits, address_fits);
    c.ins().band(valid, separate)
}

fn vector_zero(c: &mut FuncCursor<'_>) -> Value {
    let x = c.ins().iconst(types::I32, 0);
    c.ins().splat(types::I32X4, x)
}

fn scan_zero(c: &mut FuncCursor<'_>, bins: Value, bytes: i64, ready: Block, fallback: Block) {
    let scan = block(c.func);
    let checked = block(c.func);
    let offset = c.func.dfg.append_block_param(scan, types::I64);
    let running = c.func.dfg.append_block_param(scan, types::I32X4);
    let zero = c.ins().iconst(types::I64, 0);
    let none = vector_zero(c);
    c.ins().jump(scan, &[zero.into(), none.into()]);
    c.goto_bottom(scan);
    let address = c.ins().iadd(bins, offset);
    let mut bits = running;
    for index in (0..CHUNK as i32).step_by(16) {
        let word = c
            .ins()
            .load(types::I32X4, MemFlags::new().with_notrap(), address, index);
        bits = c.ins().bor(bits, word);
    }
    let next = c.ins().iadd_imm(offset, CHUNK as i64);
    let more = c.ins().icmp_imm(IntCC::UnsignedLessThan, next, bytes);
    c.ins()
        .brif(more, scan, &[next.into(), bits.into()], checked, &[]);
    c.goto_bottom(checked);
    let nonzero = c.ins().vany_true(bits);
    c.ins().brif(nonzero, fallback, &[], ready, &[]);
}

fn write_bins(
    c: &mut FuncCursor<'_>,
    bins: Value,
    seed: Option<Value>,
    bytes: i64,
    done: Block,
    args: &[cranelift_codegen::ir::BlockArg],
) {
    let write = block(c.func);
    let offset = c.func.dfg.append_block_param(write, types::I64);
    let zero = c.ins().iconst(types::I64, 0);
    let none = seed.is_none().then(|| vector_zero(c));
    c.ins().jump(write, &[zero.into()]);
    c.goto_bottom(write);
    let result = c.ins().iadd(bins, offset);
    let source = seed.map(|seed| c.ins().iadd(seed, offset));
    let flags = MemFlags::new().with_notrap();
    for index in (0..CHUNK as i32).step_by(16) {
        let word = if let Some(source) = source {
            c.ins()
                .load(types::I32X4, flags.with_readonly(), source, index)
        } else {
            none.expect("zeroing the bin span")
        };
        c.ins().store(flags, word, result, index);
    }
    let next = c.ins().iadd_imm(offset, CHUNK as i64);
    let more = c.ins().icmp_imm(IntCC::UnsignedLessThan, next, bytes);
    c.ins().brif(more, write, &[next.into()], done, args);
}

fn try_literals(c: &mut FuncCursor<'_>, group: Group, at: Value, raw: Value, vector: Block) {
    let compare = block(c.func);
    let remaining = c.ins().isub(group.end, at);
    let fits = c
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, remaining, 4);
    c.ins().brif(fits, compare, &[], group.header, &[at.into()]);
    c.goto_bottom(compare);
    let ordinal = c.ins().isub(group.top, at);
    let bytes = c.ins().ishl_imm(ordinal, 3);
    let nodes = c.ins().iadd(group.nodes, bytes);
    let packed = chunks::vector_words(c, nodes, 0);
    let address = c.ins().iadd(raw, at);
    let word = c.ins().load(
        types::I32,
        MemFlags::new().with_notrap().with_readonly(),
        address,
        0,
    );
    let word = c.ins().bswap(word);
    let bytes = c.ins().scalar_to_vector(types::I32X4, word);
    let bytes = c.ins().bitcast(
        types::I8X16,
        MemFlags::new().with_endianness(Endianness::Little),
        bytes,
    );
    let half = c.ins().uwiden_low(bytes);
    let wide = c.ins().uwiden_low(half);
    let shift = c.ins().ishl_imm(wide, i64::from(group.key_shift));
    let one = c.ins().iconst(types::I32, 1);
    let one = c.ins().splat(types::I32X4, one);
    let expected = c.ins().bor(shift, one);
    let difference = c.ins().bxor(expected, packed);
    let mask = c.ins().iconst(
        types::I32,
        i64::from(group.length_mask | (group.key_mask << group.key_shift)),
    );
    let mask = c.ins().splat(types::I32X4, mask);
    let difference = c.ins().band(mask, difference);
    let mismatch = c.ins().vany_true(difference);
    let next = c.ins().iadd_imm(at, 4);
    c.ins()
        .brif(mismatch, group.header, &[at.into()], vector, &[next.into()]);
}

impl Seeded {
    pub(in crate::native_artifact) fn function(&self) -> Function {
        self.original.clone()
    }

    pub(in crate::native_artifact) fn emit_entry(
        &self,
        f: &mut Function,
        attempt: FuncRef,
        fallback: FuncRef,
    ) {
        // Keep the large tally frame off the short-window edge.
        let entry = block(f);
        let short = block(f);
        let long = block(f);
        let args = [(); 8].map(|()| f.dfg.append_block_param(entry, types::I64));
        let mut c = FuncCursor::new(f);
        c.goto_bottom(entry);
        let end = packet(&mut c, args[1], self.end_word);
        let enough = c
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, end, MINIMUM_INPUT);
        c.ins().brif(enough, long, &[], short, &[]);
        for (block, target, values) in [(short, fallback, &args[..3]), (long, attempt, &args[..])] {
            c.goto_bottom(block);
            let call = c.ins().call(target, values);
            let [status, origin] = *c.func.dfg.inst_results(call) else {
                unreachable!("tally status pair")
            };
            c.ins().return_(&[status, origin]);
        }
    }

    pub(in crate::native_artifact) fn emit(
        &self,
        f: &mut Function,
        fallback_call: FuncRef,
        count: u64,
    ) -> Option<()> {
        let bytes = count.checked_mul(4)?;
        if count < PREFIX || bytes % CHUNK != 0 || bytes > i64::MAX as u64 {
            return None;
        }
        let group = self.group;
        let source_entry = f.layout.entry_block()?;
        let entry = f.dfg.make_block();
        f.layout.insert_block(entry, source_entry);
        f.signature.params.extend([AbiParam::new(types::I64); 5]);
        let [heap, input, out, raw_base, raw_len, begin, seed, seed_len] =
            [(); 8].map(|()| f.dfg.append_block_param(entry, types::I64));
        let fallback = block(f);
        let rollback = block(f);
        let spans = block(f);
        let scan = block(f);
        let copy = block(f);
        let vector = block(f);
        let literal = block(f);
        let matching = block(f);
        let subtract = block(f);
        let corrected = block(f);
        f.layout.set_cold(rollback);
        f.layout.set_cold(fallback);
        let at = f.dfg.append_block_param(vector, types::I64);
        let pointer = f.dfg.append_block_param(subtract, types::I64);
        let left = f.dfg.append_block_param(subtract, types::I64);
        let scratch =
            f.create_sized_stack_slot(StackSlotData::new(StackSlotKind::ExplicitSlot, 8, 3));
        let Data::Brif {
            arg, mut blocks, ..
        } = f.dfg.insts[group.literal]
        else {
            return None;
        };
        let old_match = blocks[1 - group.selected];
        blocks[group.selected] = BlockCall::new(literal, [], &mut f.dfg.value_lists);
        blocks[1 - group.selected] = BlockCall::new(matching, [], &mut f.dfg.value_lists);
        f.dfg.insts[group.literal] = Data::Brif {
            opcode: Op::Brif,
            arg,
            blocks,
        };
        let match_block = old_match.block(&f.dfg.value_lists);
        let match_args = old_match.args(&f.dfg.value_lists).collect::<Vec<_>>();
        for inst in std::iter::once(self.initial).chain(self.latches.iter().copied()) {
            let Data::Jump {
                opcode,
                destination,
            } = f.dfg.insts[inst]
            else {
                return None;
            };
            let args = destination.args(&f.dfg.value_lists).collect::<Vec<_>>();
            let destination = BlockCall::new(vector, args, &mut f.dfg.value_lists);
            f.dfg.insts[inst] = Data::Jump {
                opcode,
                destination,
            };
        }
        for &inst in &self.failures {
            f.dfg.replace(inst).jump(rollback, &[]);
        }
        let mut c = FuncCursor::new(f);
        c.goto_bottom(entry);
        let end = packet(&mut c, input, self.end_word);
        let enough_work = c
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, end, MINIMUM_INPUT);
        c.ins().brif(enough_work, spans, &[], fallback, &[]);

        c.goto_bottom(spans);
        let bins = packet(&mut c, input, group.admission.bin_word);
        let bin_len = packet(&mut c, input, group.admission.bin_word + 1);
        let nodes = packet(&mut c, input, group.admission.node_word);
        let node_len = packet(&mut c, input, group.admission.node_word + 1);
        let start = packet(&mut c, input, self.start_word);
        let top = packet(&mut c, input, self.top_word);
        let bin_end = c.ins().iadd_imm(bins, bytes as i64);
        let raw = c.ins().iadd(raw_base, begin);
        let enough_raw = c.ins().isub(raw_len, begin);
        let last = c.ins().iadd_imm(end, -1);
        let cmp = |c: &mut FuncCursor<'_>, cond, a, b| c.ins().icmp(cond, a, b);
        let mut ready = c.ins().icmp_imm(IntCC::Equal, start, 0);
        for test in [
            c.ins().icmp_imm(IntCC::Equal, bin_len, count as i64),
            c.ins()
                .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, seed_len, PREFIX as i64),
            c.ins()
                .icmp_imm(IntCC::UnsignedLessThanOrEqual, end, u32::MAX as i64),
            cmp(&mut c, IntCC::UnsignedLessThan, top, node_len),
            cmp(&mut c, IntCC::UnsignedLessThanOrEqual, last, top),
            cmp(&mut c, IntCC::UnsignedLessThanOrEqual, begin, raw_len),
            cmp(&mut c, IntCC::UnsignedLessThanOrEqual, end, enough_raw),
            cmp(&mut c, IntCC::UnsignedGreaterThanOrEqual, bin_end, bins),
        ] {
            ready = c.ins().band(ready, test);
        }
        for (base, len, width) in [
            (raw_base, raw_len, 1),
            (seed, seed_len, 4),
            (nodes, node_len, 8),
        ] {
            let separate = disjoint(&mut c, bins, bin_end, base, len, width);
            ready = c.ins().band(ready, separate);
        }
        for pointer in [raw_base, seed, nodes, bins] {
            let present = c.ins().icmp_imm(IntCC::NotEqual, pointer, 0);
            ready = c.ins().band(ready, present);
        }
        for &word in &self.lookups {
            let base = packet(&mut c, input, word);
            let len = packet(&mut c, input, word + 1);
            let separate = disjoint(&mut c, bins, bin_end, base, len, 1);
            ready = c.ins().band(ready, separate);
        }
        c.ins().brif(ready, scan, &[], fallback, &[]);
        c.goto_bottom(scan);
        scan_zero(&mut c, bins, bytes as i64, copy, fallback);
        c.goto_bottom(copy);
        let zero = c.ins().iconst(types::I64, 0);
        c.ins().stack_store(zero, scratch, 0);
        let shadow = c.ins().stack_addr(types::I64, scratch, 0);
        write_bins(
            &mut c,
            bins,
            Some(seed),
            (PREFIX * 4) as i64,
            source_entry,
            &[heap.into(), input.into(), shadow.into()],
        );

        c.goto_bottom(vector);
        try_literals(&mut c, group, at, raw, vector);
        c.goto_bottom(literal);
        let address = c.ins().iadd(raw, group.cursor);
        let byte = c.ins().uload8(
            types::I64,
            MemFlags::new().with_notrap().with_readonly(),
            address,
            0,
        );
        let payload = c.ins().ushr_imm(group.packed, i64::from(group.key_shift));
        let key = c.ins().band_imm(payload, i64::from(group.key_mask));
        let good = c.ins().icmp(IntCC::Equal, byte, key);
        let next = c.ins().iadd_imm(group.cursor, 1);
        c.ins().brif(good, vector, &[next.into()], rollback, &[]);

        c.goto_bottom(matching);
        let remaining = c.ins().isub(group.end, group.cursor);
        let nonzero = c
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThan, group.length, 1);
        let fits = c
            .ins()
            .icmp(IntCC::UnsignedLessThanOrEqual, group.length, remaining);
        let valid = c.ins().band(nonzero, fits);
        let address = c.ins().iadd(raw, group.cursor);
        c.ins().brif(
            valid,
            subtract,
            &[address.into(), group.length.into()],
            rollback,
            &[],
        );
        c.goto_bottom(subtract);
        let flags = MemFlags::new().with_notrap();
        let byte = c
            .ins()
            .uload8(types::I64, flags.with_readonly(), pointer, 0);
        let index = c.ins().ishl_imm(byte, 2);
        let address = c.ins().iadd(bins, index);
        let n = c.ins().load(types::I32, flags, address, 0);
        let n = c.ins().iadd_imm(n, -1);
        c.ins().store(flags, n, address, 0);
        let pointer = c.ins().iadd_imm(pointer, 1);
        let left = c.ins().iadd_imm(left, -1);
        c.ins().brif(
            left,
            subtract,
            &[pointer.into(), left.into()],
            corrected,
            &[],
        );
        c.goto_bottom(corrected);
        c.ins().jump(match_block, &match_args);

        c.goto_bottom(rollback);
        write_bins(&mut c, bins, None, bytes as i64, fallback, &[]);
        c.goto_bottom(fallback);
        let call = c.ins().call(fallback_call, &[heap, input, out]);
        let &[status, origin] = c.func.dfg.inst_results(call) else {
            return None;
        };
        c.ins().return_(&[status, origin]);
        Some(())
    }
}
