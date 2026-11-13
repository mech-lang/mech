//! Compose two defaulting readonly reads under a checked compact phase entry.
//! A two-valued selector chooses a bank. The current input packet fixes both
//! captured views, and the caller admits the cost extent before construction.
use super::{
    window_bounds::comparison,
    word_ranges::{Facts, constant_offset, literal},
};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Endianness, Function, Inst, InstBuilder, InstructionData as Data, MemFlags,
    Opcode as Op, StackSlotData, StackSlotKind, Value, ValueDef, condcodes::IntCC, types,
};
use cranelift_module::{DataDescription, DataId, Linkage, Module};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

#[derive(Clone)]
pub(in crate::native_artifact) struct Literal {
    pub length: u64,
    pub maximum: u64,
    pub bytes: Option<Arc<[u8]>>,
}
#[derive(Clone)]
pub(in crate::native_artifact) struct View {
    pub array: usize,
    pub pointer_word: usize,
    pub length_word: usize,
    pub width: u8,
    pub literal: Option<Literal>,
}
pub(in crate::native_artifact) struct Capture<'a> {
    pub packet_loads: &'a BTreeMap<Value, (Inst, usize)>,
    pub views: &'a [View],
    pub header: Block,
    pub countdown_word: usize,
}
#[derive(Clone, Copy, serde::Serialize)]
pub(in crate::native_artifact) struct Admission {
    pub cache: usize,
    pub cursor_word: usize,
    pub costs: usize,
    pub minimum_costs: u64,
    pub cells_per_bank: u64,
    pub second_bias: u64,
    pub minimum_records: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub immutable_code_bytes: Option<u64>,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord)]
struct CodeKey {
    bytes: Arc<[u8]>,
    input_length: u64,
    high_shift: u64,
    bias: u8,
}

impl CodeKey {
    fn encode(&self) -> Box<[u8]> {
        let split = self.bytes.len() as u64;
        (0..self.input_length)
            .map(|word| {
                let upper = word > split;
                let shift = if upper { self.high_shift } else { 0 };
                let index = word.wrapping_sub(1) >> shift;
                let slot = if index < split {
                    self.bytes[index as usize]
                } else {
                    0
                };
                slot + if upper { self.bias } else { 0 }
            })
            .collect::<Vec<_>>()
            .into_boxed_slice()
    }
}

struct DenseCodes {
    word: Value,
    key: CodeKey,
}

#[derive(Default)]
pub(in crate::native_artifact) struct DenseTables(BTreeMap<CodeKey, DataId>);

impl DenseTables {
    fn intern<M: Module>(
        &mut self,
        module: &mut M,
        key: &CodeKey,
    ) -> Result<DataId, NativeCompileError> {
        if let Some(id) = self.0.get(key) {
            return Ok(*id);
        }
        let id = module
            .declare_data(
                &format!("mech_compact_cost_codes_{}", self.0.len()),
                Linkage::Local,
                false,
                false,
            )
            .map_err(backend)?;
        let mut data = DataDescription::new();
        data.set_align(16);
        data.define(key.encode());
        module.define_data(id, &data).map_err(backend)?;
        self.0.insert(key.clone(), id);
        Ok(id)
    }
}
#[derive(Clone)]
pub(super) struct Read {
    branch: Inst,
    pub(super) index: Value,
    pub(super) view: View,
    pub(super) join: Block,
    pub(super) result: Value,
    arms: Vec<Block>,
}

fn definition(f: &Function, value: Value) -> Option<(Inst, Data)> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return None;
    };
    Some((inst, f.dfg.insts[inst]))
}

fn incoming(
    f: &Function,
    cfg: &ControlFlowGraph,
    block: Block,
    slot: usize,
) -> Option<Vec<(Block, Value)>> {
    let mut arguments = Vec::new();
    for edge in cfg.pred_iter(block) {
        for call in
            f.dfg.insts[edge.inst].branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
        {
            if call.block(&f.dfg.value_lists) == block {
                arguments.push((
                    edge.block,
                    call.args(&f.dfg.value_lists).nth(slot)?.as_value()?,
                ));
            }
        }
    }
    (!arguments.is_empty()).then_some(arguments)
}

// Invariant packet fields can pass through a self edge or be reloaded by an
// alternate entry. All external definitions must identify the same word.
pub(super) fn packet_word(
    f: &Function,
    cfg: &ControlFlowGraph,
    loads: &BTreeMap<Value, (Inst, usize)>,
    value: Value,
) -> Option<usize> {
    let mut pending = vec![value];
    let mut seen = BTreeSet::new();
    let mut result = None;
    while let Some(value) = pending.pop() {
        let value = f.dfg.resolve_aliases(value);
        if !seen.insert(value) {
            continue;
        }
        if let Some(&(_, word)) = loads.get(&value) {
            if result.is_some_and(|old| old != word) {
                return None;
            }
            result = Some(word);
        } else if let ValueDef::Param(block, slot) = f.dfg.value_def(value) {
            if Some(block) == f.layout.entry_block() {
                return None;
            }
            pending.extend(
                incoming(f, cfg, block, slot)?
                    .into_iter()
                    .map(|(_, value)| value),
            );
        } else {
            return None;
        }
    }
    result
}

pub(super) fn scalar(op: Op) -> bool {
    matches!(
        op,
        Op::Iconst
            | Op::Iadd
            | Op::IaddImm
            | Op::Isub
            | Op::ImulImm
            | Op::Ishl
            | Op::IshlImm
            | Op::Ushr
            | Op::UshrImm
            | Op::Band
            | Op::BandImm
            | Op::Uextend
            | Op::Ireduce
            | Op::Icmp
            | Op::IcmpImm
    )
}

// Follow a single-entry arm to its merge. Exactly one typed load is allowed
// in the selected arm and no effects are allowed in the default arm.
fn arm(
    f: &Function,
    cfg: &ControlFlowGraph,
    mut block: Block,
) -> Option<(Block, Value, Vec<Block>, Vec<Inst>)> {
    let mut blocks = Vec::new();
    let mut loads = Vec::new();
    loop {
        if cfg.pred_iter(block).count() != 1
            || !f.dfg.block_params(block).is_empty()
            || blocks.contains(&block)
        {
            return None;
        }
        blocks.push(block);
        let last = f.layout.last_inst(block)?;
        for inst in f.layout.block_insts(block).take_while(|&inst| inst != last) {
            let op = f.dfg.insts[inst].opcode();
            if matches!(op, Op::Load | Op::Uload8 | Op::Uload32) {
                loads.push(inst);
            } else if !scalar(op) {
                return None;
            }
        }
        let Data::Jump { destination, .. } = f.dfg.insts[last] else {
            return None;
        };
        block = destination.block(&f.dfg.value_lists);
        if cfg.pred_iter(block).count() > 1 {
            let mut args = destination.args(&f.dfg.value_lists);
            let result = args.next()?.as_value()?;
            return (args.next().is_none()).then_some((block, result, blocks, loads));
        }
        if destination.args(&f.dfg.value_lists).next().is_some() {
            return None;
        }
    }
}

fn read_comparison(f: &Function, condition: Value) -> Option<Inst> {
    let compare = comparison(f, condition)?;
    let Data::BinaryImm64 {
        opcode: Op::BandImm,
        arg,
        imm,
    } = f.dfg.insts[compare]
    else {
        return Some(compare);
    };
    if imm.bits() != 1 || f.dfg.value_type(arg) != types::I8 {
        return None;
    }
    // A compact branch may expose the low bit of a comparison for native
    // selection. An arbitrary byte mask does not establish a read bound.
    let (inner, data) = definition(f, arg)?;
    matches!(data, Data::IntCompare { .. }).then_some(inner)
}

pub(super) fn defaulted_read(
    f: &Function,
    cfg: &ControlFlowGraph,
    packet_loads: &BTreeMap<Value, (Inst, usize)>,
    views: &[View],
    branch: Inst,
    endianness: Endianness,
) -> Option<Read> {
    let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
        return None;
    };
    let compare = read_comparison(f, arg)?;
    let Data::IntCompare {
        cond: IntCC::UnsignedLessThan,
        args: [index, length],
        ..
    } = f.dfg.insts[compare]
    else {
        return None;
    };
    let length = packet_word(f, cfg, packet_loads, length)?;
    let view = views
        .iter()
        .find(|view| view.length_word == length)?
        .clone();
    let (join, yes, mut arms, loads) = arm(f, cfg, blocks[0].block(&f.dfg.value_lists))?;
    let (other_join, no, no_arm, no_loads) = arm(f, cfg, blocks[1].block(&f.dfg.value_lists))?;
    let [result] = *f.dfg.block_params(join) else {
        return None;
    };
    if join != other_join
        || cfg.pred_iter(join).count() != 2
        || literal(f, no) != Some(0)
        || !no_loads.is_empty()
        || f.dfg.value_type(result) != types::I64
        || f.dfg.value_type(index) != types::I64
    {
        return None;
    }
    let [load] = *loads.as_slice() else {
        return None;
    };
    let Data::Load {
        opcode,
        arg: address,
        offset,
        flags,
    } = f.dfg.insts[load]
    else {
        return None;
    };
    if i32::from(offset) != 0 || flags.endianness(endianness) != endianness {
        return None;
    }
    let loaded = f.dfg.first_result(load);
    let returned = f.dfg.resolve_aliases(yes);
    let same_result = match (view.width, opcode) {
        (1, Op::Uload8) | (4, Op::Uload32) => loaded == returned,
        (8, Op::Load) => loaded == returned && flags == MemFlags::new(),
        (width @ (1 | 4), Op::Load) => {
            let expected = if width == 1 { types::I8 } else { types::I32 };
            f.dfg.value_type(loaded) == expected
                && matches!(definition(f, returned), Some((_, Data::Unary {
                    opcode: Op::Uextend, arg,
                })) if f.dfg.resolve_aliases(arg) == loaded)
        }
        _ => false,
    };
    if !same_result || f.dfg.value_type(returned) != types::I64 {
        return None;
    }
    let (
        _,
        Data::Binary {
            opcode: Op::Iadd,
            args,
        },
    ) = definition(f, address)?
    else {
        return None;
    };
    let expected = (f.dfg.resolve_aliases(index), 0);
    let matches = |pointer: Value, bytes: Value| {
        if packet_word(f, cfg, packet_loads, pointer) != Some(view.pointer_word) {
            return false;
        }
        match definition(f, bytes) {
            Some((
                _,
                Data::BinaryImm64 {
                    opcode: Op::ImulImm,
                    arg,
                    imm,
                },
            )) if imm.bits() == i64::from(view.width) => constant_offset(f, arg) == expected,
            Some((
                _,
                Data::BinaryImm64 {
                    opcode: Op::IshlImm,
                    arg,
                    imm,
                },
            )) if imm.bits() == i64::from(view.width.ilog2()) => {
                constant_offset(f, arg) == expected
            }
            _ => view.width == 1 && constant_offset(f, bytes) == expected,
        }
    };
    if !matches(args[0], args[1]) && !matches(args[1], args[0]) {
        return None;
    }
    arms.extend(no_arm);
    Some(Read {
        branch,
        index,
        view,
        join,
        result,
        arms,
    })
}

pub(super) fn between(
    f: &Function,
    cfg: &ControlFlowGraph,
    mut block: Block,
) -> Option<(Inst, Vec<Block>)> {
    let mut blocks = Vec::new();
    loop {
        if blocks.contains(&block) || (!blocks.is_empty() && cfg.pred_iter(block).count() != 1) {
            return None;
        }
        blocks.push(block);
        let end = f.layout.last_inst(block)?;
        if f.layout
            .block_insts(block)
            .take_while(|&inst| inst != end)
            .any(|inst| !scalar(f.dfg.insts[inst].opcode()))
        {
            return None;
        }
        match f.dfg.insts[end] {
            Data::Brif { .. } => return Some((end, blocks)),
            Data::Jump { destination, .. }
                if destination.args(&f.dfg.value_lists).next().is_none() =>
            {
                block = destination.block(&f.dfg.value_lists);
            }
            _ => return None,
        }
    }
}

fn shift(f: &Function, facts: &mut Facts<'_>, value: Value, right: bool) -> Option<(Value, u32)> {
    let (_, data) = definition(f, value)?;
    let (op, input, amount) = match data {
        Data::Binary {
            opcode,
            args: [input, amount],
        } => {
            let range = facts.value_range(amount)?;
            if range.0 != range.1 {
                return None;
            }
            (opcode, input, range.0)
        }
        Data::BinaryImm64 { opcode, arg, imm } => (opcode, arg, imm.bits() as u64),
        _ => return None,
    };
    let supported = if right {
        matches!(op, Op::Ushr | Op::UshrImm)
    } else {
        matches!(op, Op::Ishl | Op::IshlImm)
    };
    supported.then(|| {
        (
            input,
            (amount % u64::from(f.dfg.value_type(value).bits())) as u32,
        )
    })
}

#[derive(Clone, Copy)]
struct Selector {
    input: Value,
    threshold: u64,
    high: u64,
}

// The two intervals before the logical shift are the non-wrapping and the
// wrapping halves of (constant - x). Require each projected half to be exact.
fn selector(f: &Function, facts: &mut Facts<'_>, value: Value) -> Option<Selector> {
    let (mut reduced, shift) = self::shift(f, facts, value, true)?;
    let full_mask = |upper: u64| u64::MAX.checked_shr(upper.leading_zeros()).unwrap_or(0);
    let (a, b) = loop {
        match definition(f, reduced)?.1 {
            Data::Unary {
                opcode: Op::Uextend,
                arg,
            } => reduced = arg,
            Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } => {
                let left = facts.value_range(a)?;
                let right = facts.value_range(b)?;
                reduced = if right.0 == right.1 && right.0 & full_mask(left.1) == full_mask(left.1)
                {
                    a
                } else if left.0 == left.1 && left.0 & full_mask(right.1) == full_mask(right.1) {
                    b
                } else {
                    return None;
                };
            }
            Data::Binary {
                opcode: Op::Isub,
                args: [a, b],
            } => break (a, b),
            _ => return None,
        }
    };
    let bits = f.dfg.value_type(reduced).bits();
    if bits > 64 || shift >= bits {
        return None;
    }
    let mask = u64::MAX >> (64 - bits);
    let bias = facts.value_range(a)?;
    let input = facts.value_range(b)?;
    if bias.0 != bias.1 || input.0 > bias.0 || input.1 <= bias.0 {
        return None;
    }
    let low_max = (bias.0 - input.0) >> shift;
    let high_min = (mask - (input.1 - bias.0 - 1)) >> shift;
    let high_max = mask >> shift;
    (low_max == 0 && high_min == high_max && high_max != 0).then_some(Selector {
        input: b,
        threshold: bias.0,
        high: high_max,
    })
}

struct SplitIndex {
    selector: Selector,
}

// Both views must retain every possible bit of this same unsigned word.
fn same_unsigned_word(
    f: &Function,
    facts: &mut Facts<'_>,
    mut view: Value,
    word: Value,
) -> Option<()> {
    let word = f.dfg.resolve_aliases(word);
    let upper = facts.value_range(word)?.1;
    let needed = u64::MAX.checked_shr(upper.leading_zeros()).unwrap_or(0);
    loop {
        view = f.dfg.resolve_aliases(view);
        if view == word {
            return Some(());
        }
        let bits = f.dfg.value_type(view).bits();
        if bits > 64 || bits == 0 {
            return None;
        }
        let mask = u64::MAX >> (64 - bits);
        view = match definition(f, view)?.1 {
            Data::Unary {
                opcode: Op::Uextend,
                arg,
            } => arg,
            Data::Unary {
                opcode: Op::Ireduce,
                arg,
            } if needed <= mask => arg,
            Data::BinaryImm64 {
                opcode: Op::BandImm,
                arg,
                imm,
            } if imm.bits() as u64 & needed == needed => arg,
            Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } if literal(f, a).is_some_and(|mask| mask & needed == needed) => b,
            Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } if literal(f, b).is_some_and(|mask| mask & needed == needed) => a,
            _ => return None,
        };
    }
}

fn split_index(
    f: &Function,
    facts: &mut Facts<'_>,
    index: Value,
    selection: Value,
    selector: Selector,
    slots: u64,
) -> Option<SplitIndex> {
    let (
        _,
        Data::Binary {
            opcode: Op::Ushr,
            args: [raw, count],
        },
    ) = definition(f, index)?
    else {
        return None;
    };
    if f.dfg.value_type(raw) != types::I64 || selector.high >= 64 || selector.threshold != slots {
        return None;
    }
    let (
        _,
        Data::Binary {
            opcode: Op::Isub,
            args: [word, one],
        },
    ) = definition(f, raw)?
    else {
        return None;
    };
    if literal(f, one)? != 1 {
        return None;
    }
    same_unsigned_word(f, facts, selector.input, word)?;
    let is_selector = |value| f.dfg.resolve_aliases(value) == f.dfg.resolve_aliases(selection);
    if !is_selector(count) {
        let mask = match definition(f, count)?.1 {
            Data::BinaryImm64 {
                opcode: Op::BandImm,
                arg,
                imm,
            } if is_selector(arg) => imm.bits() as u64,
            Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } if is_selector(a) => literal(f, b)?,
            Data::Binary {
                opcode: Op::Band,
                args: [a, b],
            } if is_selector(b) => literal(f, a)?,
            _ => return None,
        };
        if mask & selector.high != selector.high {
            return None;
        }
    }
    Some(SplitIndex { selector })
}

fn dependencies(
    f: &Function,
    cfg: &ControlFlowGraph,
    capture: &Capture<'_>,
    reads: &BTreeMap<Value, &Read>,
    input: Value,
    cache: Option<usize>,
) -> Option<BTreeSet<Value>> {
    let mut pending = vec![input];
    let mut seen = BTreeSet::new();
    let mut roots = BTreeSet::new();
    while let Some(value) = pending.pop() {
        let value = f.dfg.resolve_aliases(value);
        if !seen.insert(value) || literal(f, value).is_some() {
            continue;
        }
        if let Some(read) = reads.get(&value) {
            if let Some(array) = cache {
                if read.view.array != array {
                    return None;
                }
                pending.push(read.index);
            } else {
                roots.insert(value);
            }
            continue;
        }
        if cache.is_some() && packet_word(f, cfg, capture.packet_loads, value).is_some() {
            continue;
        }
        match f.dfg.value_def(value) {
            ValueDef::Param(block, _) if cache.is_some() && block == capture.header => {
                roots.insert(value);
            }
            ValueDef::Param(block, slot)
                if cache.is_some() && Some(block) != f.layout.entry_block() =>
            {
                pending.extend(incoming(f, cfg, block, slot)?.into_iter().map(|(_, v)| v));
            }
            ValueDef::Result(inst, 0) if scalar(f.dfg.insts[inst].opcode()) => {
                pending.extend(f.dfg.inst_values(inst));
            }
            _ => return None,
        }
    }
    Some(roots)
}

fn seed_word(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    capture: &Capture<'_>,
    value: Value,
) -> Option<usize> {
    let ValueDef::Param(block, slot) = f.dfg.value_def(value) else {
        return None;
    };
    let mut result = None;
    for (predecessor, argument) in incoming(f, cfg, block, slot)? {
        if dom.block_dominates(block, predecessor) {
            continue;
        }
        let word = packet_word(f, cfg, capture.packet_loads, argument)?;
        if result.is_some_and(|old| old != word) {
            return None;
        }
        result = Some(word);
    }
    result
}

struct Rewrite {
    slot: Read,
    cost: Read,
    selector: Value,
    bias_shift: u32,
    bank_bit: u32,
    dense_codes: Option<DenseCodes>,
    admission: Admission,
}

fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    capture: &Capture<'_>,
    slot: &Read,
    by_branch: &BTreeMap<Inst, Read>,
    by_result: &BTreeMap<Value, &Read>,
) -> Option<Rewrite> {
    let data = slot.view.literal.as_ref()?;
    let size = data.length.checked_mul(8)?;
    if slot.view.width != 1 || !data.length.is_power_of_two() || size > 2048 {
        return None;
    }
    let inspect = std::env::var_os("MECH_INSPECT_COMPOSED_READS").is_some();
    let (branch, mut region) = between(f, cfg, slot.join)?;
    if inspect {
        eprintln!(
            "compose literal read {} with following branch {branch}",
            slot.branch
        );
    }
    let cost = by_branch.get(&branch)?;
    if cost.view.width != 4 || cost.view.array == slot.view.array {
        return None;
    }
    let (
        _,
        Data::Binary {
            opcode: Op::Iadd,
            args: [left, right],
        },
    ) = definition(f, cost.index)?
    else {
        return None;
    };
    let bias = if f.dfg.resolve_aliases(left) == slot.result {
        right
    } else if f.dfg.resolve_aliases(right) == slot.result {
        left
    } else {
        return None;
    };
    let mut facts = Facts::across_edges(f, cfg, None);
    let (selection, bias_shift) = shift(f, &mut facts, bias, false)?;
    if inspect {
        eprintln!("compose biased read {} selector {selection}", cost.branch);
    }
    let classification = selector(f, &mut facts, selection)?;
    let classification_input = classification.input;
    let high = classification.high;
    if inspect {
        eprintln!("compose selector range 0 or {high} from {classification_input}");
    }
    let bank_delta = high.checked_shl(bias_shift)?;
    if bank_delta >> bias_shift != high {
        return None;
    }
    let floor = data.maximum.checked_add(bank_delta)?.checked_add(1)?;
    // The two read addresses scale by four and fit in the guarded view.
    if floor > (i64::MAX as u64) / 4 {
        return None;
    }
    let cache_reads = dependencies(f, cfg, capture, by_result, classification_input, None)?;
    let cache = by_result.get(cache_reads.iter().next()?)?;
    if cache_reads.len() != 1 || cache.view.width != 4 || cache.view.array == cost.view.array {
        return None;
    }
    let cursors = dependencies(
        f,
        cfg,
        capture,
        by_result,
        cache.index,
        Some(cache.view.array),
    )?;
    if inspect {
        eprintln!(
            "compose cache index {} current roots {cursors:?} at {}",
            cache.index, capture.header
        );
    }
    let cursor = seed_word(f, cfg, dom, capture, *cursors.iter().next()?)?;
    if cursors.len() != 1 || cursor == capture.countdown_word {
        return None;
    }
    region.extend(slot.arms.iter().chain(&cost.arms).copied());
    let region = region.into_iter().collect::<BTreeSet<_>>();
    let local = |value| match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Param(block, _) => region.contains(&block),
        ValueDef::Result(inst, _) => f
            .layout
            .inst_block(inst)
            .is_some_and(|block| region.contains(&block)),
        ValueDef::Union(..) => true,
    };
    if local(selection)
        || f.layout
            .blocks()
            .filter(|b| !region.contains(b))
            .any(|block| {
                f.layout
                    .block_insts(block)
                    .any(|inst| f.dfg.inst_values(inst).any(local))
            })
    {
        return None;
    }
    let (selection_inst, _) = definition(f, selection)?;
    if !dom.dominates(selection_inst, slot.branch, &f.layout) {
        return None;
    }
    let dense_codes = if std::env::var_os("MECH_COMPOSE_IMMUTABLE_COST_CODES").is_some() {
        (|| {
            let bytes = data.bytes.as_ref()?;
            if bytes.len() as u64 != data.length || floor > 256 {
                return None;
            }
            let split = split_index(
                f,
                &mut facts,
                slot.index,
                selection,
                classification,
                data.length,
            )?;
            let word = split.selector.input;
            let maximum = facts.value_range(word)?.1;
            if maximum > u16::MAX as u64 {
                return None;
            }
            Some(DenseCodes {
                word,
                key: CodeKey {
                    bytes: bytes.clone(),
                    input_length: maximum + 1,
                    high_shift: split.selector.high,
                    bias: bank_delta as u8,
                },
            })
        })()
    } else {
        None
    };
    let immutable_code_bytes = dense_codes.as_ref().map(|dense| dense.key.input_length);
    Some(Rewrite {
        slot: slot.clone(),
        cost: cost.clone(),
        selector: selection,
        bias_shift,
        bank_bit: high.trailing_zeros(),
        dense_codes,
        admission: Admission {
            cache: cache.view.array,
            cursor_word: cursor,
            costs: cost.view.array,
            minimum_costs: floor,
            cells_per_bank: data.length,
            second_bias: bank_delta,
            minimum_records: size,
            immutable_code_bytes,
        },
    })
}

impl Rewrite {
    fn emit(self, f: &mut Function) -> Option<Admission> {
        let entry = f.layout.entry_block()?;
        let &[_, input, _] = f.dfg.block_params(entry) else {
            return None;
        };
        let start = f.layout.last_inst(entry)?;
        let Data::Jump { destination, .. } = f.dfg.insts[start] else {
            return None;
        };
        let target = destination.block(&f.dfg.value_lists);
        if destination.args(&f.dfg.value_lists).next().is_some() {
            return None;
        }
        let slots_at = i32::try_from(self.slot.view.pointer_word.checked_mul(8)?).ok()?;
        let costs_at = i32::try_from(self.cost.view.pointer_word.checked_mul(8)?).ok()?;
        let slots = self.admission.cells_per_bank;
        let table = f.create_sized_stack_slot(StackSlotData::new(
            StackSlotKind::ExplicitSlot,
            (slots * 8) as u32,
            3,
        ));
        let prepare = f.dfg.make_block();
        let readable = f.dfg.make_block();
        let default = f.dfg.make_block();
        for block in [prepare, readable, default] {
            f.layout.append_block(block);
        }
        let index = f.dfg.append_block_param(prepare, types::I64);
        let flags = MemFlags::new();
        let mut c = FuncCursor::new(f).at_inst(start);
        let table = c.ins().stack_addr(types::I64, table, 0);
        let map = c.ins().load(types::I64, flags, input, slots_at);
        let costs = c.ins().load(types::I64, flags, input, costs_at);
        let zero = c.ins().iconst(types::I64, 0);
        c.func.dfg.replace(start).jump(prepare, &[zero.into()]);

        c.goto_bottom(prepare);
        let byte_address = c.ins().iadd(map, index);
        let raw = c.ins().load(types::I8, flags, byte_address, 0);
        let slot = c.ins().uextend(types::I64, raw);
        let byte = c.ins().ishl_imm(slot, 2);
        let address = c.ins().iadd(costs, byte);
        let a = c.ins().load(types::I32, flags, address, 0);
        let b = c.ins().load(
            types::I32,
            flags,
            address,
            (self.admission.second_bias * 4) as i32,
        );
        let at = c.ins().ishl_imm(index, 2);
        let address = c.ins().iadd(table, at);
        c.ins().store(flags, a, address, 0);
        c.ins().store(flags, b, address, (slots * 4) as i32);
        let next = c.ins().iadd_imm(index, 1);
        let more = c
            .ins()
            .icmp_imm(IntCC::UnsignedLessThan, next, slots as i64);
        c.ins().brif(more, prepare, &[next.into()], target, &[]);

        c.goto_inst(self.slot.branch);
        let bound = c
            .ins()
            .icmp_imm(IntCC::UnsignedLessThan, self.slot.index, slots as i64);
        c.func
            .dfg
            .replace(self.slot.branch)
            .brif(bound, readable, &[], default, &[]);

        c.goto_bottom(readable);
        let bit = c.ins().ushr_imm(self.selector, i64::from(self.bank_bit));
        let bank = c.ins().band_imm(bit, 1);
        let bank = c.ins().ishl_imm(bank, i64::from(slots.ilog2()));
        let cell = c.ins().bor(self.slot.index, bank);
        let byte = c.ins().ishl_imm(cell, 2);
        let address = c.ins().iadd(table, byte);
        let value = c.ins().load(types::I32, flags, address, 0);
        let value = c.ins().uextend(types::I64, value);
        c.ins().jump(self.cost.join, &[value.into()]);

        c.goto_bottom(default);
        let bias = c
            .ins()
            .ishl_imm(self.selector, i64::from(self.bias_shift + 2));
        let address = c.ins().iadd(costs, bias);
        let value = c.ins().load(types::I32, flags, address, 0);
        let value = c.ins().uextend(types::I64, value);
        c.ins().jump(self.cost.join, &[value.into()]);
        Some(self.admission)
    }
}

impl Rewrite {
    fn emit_dense<M: Module>(
        &self,
        f: &mut Function,
        module: &mut M,
        tables: &mut DenseTables,
        dense: &DenseCodes,
    ) -> Result<Option<Admission>, NativeCompileError> {
        let Some(entry) = f.layout.entry_block() else {
            return Ok(None);
        };
        let &[_, input, _] = f.dfg.block_params(entry) else {
            return Ok(None);
        };
        let Some(start) = f.layout.last_inst(entry) else {
            return Ok(None);
        };
        let Some(costs_at) = self
            .cost
            .view
            .pointer_word
            .checked_mul(8)
            .and_then(|at| i32::try_from(at).ok())
        else {
            return Ok(None);
        };
        let id = tables.intern(module, &dense.key)?;
        let global = module.declare_data_in_func(id, f);
        let flags = MemFlags::new();
        let mut c = FuncCursor::new(f).at_inst(start);
        let codes = c.ins().global_value(types::I64, global);
        let costs = c.ins().load(types::I64, flags, input, costs_at);
        c.goto_inst(self.slot.branch);
        let word = if c.func.dfg.value_type(dense.word) == types::I64 {
            dense.word
        } else {
            c.ins().uextend(types::I64, dense.word)
        };
        let cell = c.ins().iadd(codes, word);
        let code = c.ins().uload8(types::I64, flags, cell, 0);
        let byte = c.ins().ishl_imm(code, 2);
        let address = c.ins().iadd(costs, byte);
        let value = c.ins().uload32(flags, address, 0);
        c.func
            .dfg
            .replace(self.slot.branch)
            .jump(self.cost.join, &[value.into()]);
        Ok(Some(self.admission))
    }
}

pub(super) fn compose<M: Module>(
    module: &mut M,
    tables: &mut DenseTables,
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    capture: &Capture<'_>,
) -> Result<Option<Admission>, NativeCompileError> {
    let endianness = module.isa().endianness();
    let reads = f
        .layout
        .blocks()
        .filter_map(|block| {
            let branch = f.layout.last_inst(block)?;
            defaulted_read(
                f,
                cfg,
                capture.packet_loads,
                capture.views,
                branch,
                endianness,
            )
            .filter(|read| matches!(read.view.width, 1 | 4))
            .map(|read| (branch, read))
        })
        .collect::<BTreeMap<_, _>>();
    let results = reads
        .values()
        .map(|read| (read.result, read))
        .collect::<BTreeMap<_, _>>();
    let Some(rewrite) = reads
        .values()
        .find_map(|first| plan(f, cfg, dom, capture, first, &reads, &results))
    else {
        return Ok(None);
    };
    if let Some(dense) = &rewrite.dense_codes {
        return rewrite.emit_dense(f, module, tables, dense);
    }
    if !rewrite
        .admission
        .second_bias
        .checked_mul(4)
        .is_some_and(|bytes| i32::try_from(bytes).is_ok())
    {
        return Ok(None);
    }
    Ok(rewrite.emit(f))
}

#[cfg(test)]
#[path = "composed_index_tests.rs"]
mod tests;
