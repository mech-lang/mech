//! Share the exclusive valid start of a fixed-width window. Both source
//! rejection paths must reach the same continuation with the same values.
pub(super) mod ordered_spans;
use super::word_ranges::literal;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value,
    ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

fn unextended(f: &Function, mut value: Value) -> Option<Inst> {
    loop {
        let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
            return None;
        };
        match f.dfg.insts[inst] {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => value = arg,
            _ => return Some(inst),
        }
    }
}

pub(super) fn comparison(f: &Function, value: Value) -> Option<Inst> {
    let mut inst = unextended(f, value)?;
    loop {
        let inner = match f.dfg.insts[inst] {
            Data::IntCompare {
                cond: IntCC::Equal,
                args: [a, b],
                ..
            } if literal(f, b) == Some(1) => Some(a),
            Data::IntCompare {
                cond: IntCC::Equal,
                args: [a, b],
                ..
            } if literal(f, a) == Some(1) => Some(b),
            Data::IntCompareImm {
                cond: IntCC::Equal,
                arg,
                imm,
                ..
            } if imm.bits() == 1 => Some(arg),
            _ => None,
        };
        // Equality to one is identity only for the result of a comparison.
        if let Some(next) = inner.and_then(|word| unextended(f, word)) {
            if matches!(
                f.dfg.insts[next],
                Data::IntCompare { .. } | Data::IntCompareImm { .. }
            ) {
                inst = next;
                continue;
            }
        }
        return Some(inst);
    }
}

fn length_test(f: &Function, condition: Value) -> Option<(Value, u64)> {
    let (length, width) = match f.dfg.insts[comparison(f, condition)?] {
        Data::IntCompare {
            cond: IntCC::UnsignedGreaterThanOrEqual,
            args: [length, width],
            ..
        }
        | Data::IntCompare {
            cond: IntCC::UnsignedLessThanOrEqual,
            args: [width, length],
            ..
        } => (length, literal(f, width)?),
        Data::IntCompareImm {
            cond: IntCC::UnsignedGreaterThanOrEqual,
            arg,
            imm,
            ..
        } => (arg, imm.bits() as u64),
        _ => return None,
    };
    let length = f.dfg.resolve_aliases(length);
    (width != 0 && f.dfg.value_type(length) == types::I64).then_some((length, width))
}

fn index_test(f: &Function, condition: Value, length: Value, width: u64) -> Option<Value> {
    let index_limit = match f.dfg.insts[comparison(f, condition)?] {
        Data::IntCompare {
            cond: IntCC::UnsignedLessThanOrEqual,
            args,
            ..
        } => args,
        Data::IntCompare {
            cond: IntCC::UnsignedGreaterThanOrEqual,
            args: [limit, index],
            ..
        } => [index, limit],
        _ => return None,
    };
    let [index, limit] = index_limit.map(|value| f.dfg.resolve_aliases(value));
    let ValueDef::Result(inst, 0) = f.dfg.value_def(limit) else {
        return None;
    };
    let base = match f.dfg.insts[inst] {
        Data::Binary {
            opcode: Opcode::Isub,
            args: [base, size],
        } if literal(f, size) == Some(width) => base,
        // CLIF canonicalizes subtraction by a literal into modular addition.
        // The original dominating length>=width guard still proves this is
        // the same nonwrapping window end on the selected route.
        Data::BinaryImm64 {
            opcode: Opcode::IaddImm,
            arg,
            imm,
        } if imm.bits() as u64 == width.wrapping_neg() => arg,
        _ => return None,
    };
    (f.dfg.resolve_aliases(base) == length).then_some(index)
}

pub(super) fn sole_predecessor(cfg: &ControlFlowGraph, block: Block) -> Option<(Block, Inst)> {
    let mut preds = cfg.pred_iter(block);
    let pred = preds.next()?;
    preds.next().is_none().then_some((pred.block, pred.inst))
}

fn in_prologue(f: &Function, cfg: &ControlFlowGraph, mut block: Block) -> Option<()> {
    // A reachable chain with one unconditional incoming edge per block runs
    // only once per activation. It cannot cross a merge or a loop backedge.
    while Some(block) != f.layout.entry_block() {
        let (prior, inst) = sole_predecessor(cfg, block)?;
        if !matches!(f.dfg.insts[inst], Data::Jump { .. }) {
            return None;
        }
        block = prior;
    }
    Some(())
}

fn only_successor_chain(
    f: &Function,
    cfg: &ControlFlowGraph,
    mut pred: Inst,
    mut block: Block,
) -> Option<Inst> {
    loop {
        if sole_predecessor(cfg, block)?.1 != pred || !f.dfg.block_params(block).is_empty() {
            return None;
        }
        let last = f.layout.last_inst(block)?;
        if !f
            .layout
            .block_insts(block)
            .take_while(|&inst| inst != last)
            .all(|inst| {
                matches!(
                    f.dfg.insts[inst].opcode(),
                    Opcode::Iconst
                        | Opcode::Icmp
                        | Opcode::IcmpImm
                        | Opcode::Uextend
                        | Opcode::Isub
                        | Opcode::IaddImm
                )
            })
        {
            return None;
        }
        match f.dfg.insts[last] {
            Data::Brif { .. } => return Some(last),
            Data::Jump { destination, .. } => {
                pred = last;
                block = destination.block(&f.dfg.value_lists);
            }
            _ => return None,
        }
    }
}

fn forwarded(f: &Function, mut call: BlockCall) -> Option<Vec<(Block, Vec<Value>)>> {
    let mut path = Vec::new();
    let mut values = BTreeMap::new();
    loop {
        let block = call.block(&f.dfg.value_lists);
        if path.iter().any(|(visited, _)| *visited == block) {
            return None;
        }
        let args = call
            .args(&f.dfg.value_lists)
            .map(|arg| {
                let value = f.dfg.resolve_aliases(arg.as_value()?);
                Some(values.get(&value).copied().unwrap_or(value))
            })
            .collect::<Option<Vec<_>>>()?;
        let params = f.dfg.block_params(block);
        if params.len() != args.len() {
            return None;
        }
        values.extend(params.iter().copied().zip(args.iter().copied()));
        path.push((block, args));
        let inst = f.layout.first_inst(block)?;
        match f.dfg.insts[inst] {
            Data::Jump { destination, .. } if f.layout.last_inst(block) == Some(inst) => {
                call = destination
            }
            _ => return Some(path),
        }
    }
}

fn same_rejection(f: &Function, a: BlockCall, b: BlockCall) -> Option<()> {
    let a = forwarded(f, a)?;
    // The first merge must agree. A later merge can hide different SSA values
    // from an earlier shared block even when its own parameters agree.
    for (block, args) in forwarded(f, b)? {
        if let Some((_, other)) = a.iter().find(|(here, _)| *here == block) {
            return (other == &args).then_some(());
        }
    }
    None
}

struct Window {
    first: Inst,
    second: Inst,
    index: Value,
    length: Value,
    width: u64,
}

fn candidate(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    first: Inst,
) -> Option<Window> {
    let Data::Brif {
        arg,
        blocks: [yes, no],
        ..
    } = f.dfg.insts[first]
    else {
        return None;
    };
    if yes.block(&f.dfg.value_lists) == no.block(&f.dfg.value_lists) {
        return None;
    }
    let (length, width) = length_test(f, arg)?;
    // Compute the shared limit after its original definition in the linear
    // activation prologue. A loop-local or reloaded length declines.
    let length_block = match f.dfg.value_def(length) {
        ValueDef::Param(block, _) => block,
        ValueDef::Result(inst, _) => f.layout.inst_block(inst)?,
        ValueDef::Union(_, _) => return None,
    };
    if !dom.is_reachable(length_block) {
        return None;
    }
    in_prologue(f, cfg, length_block)?;
    let second = only_successor_chain(f, cfg, first, yes.block(&f.dfg.value_lists))?;
    let Data::Brif {
        arg,
        blocks: [_, rejected],
        ..
    } = f.dfg.insts[second]
    else {
        return None;
    };
    let index = index_test(f, arg, length, width)?;
    let available = match f.dfg.value_def(index) {
        ValueDef::Param(block, _) => dom.dominates(block, first, &f.layout),
        ValueDef::Result(inst, _) => dom.dominates(inst, first, &f.layout),
        ValueDef::Union(_, _) => false,
    };
    if !available {
        return None;
    }
    same_rejection(f, no, rejected)?;
    Some(Window {
        first,
        second,
        index,
        length,
        width,
    })
}

fn limit(f: &mut Function, length: Value, width: u64) -> Value {
    if width == 1 {
        return length;
    }
    let mut cursor = match f.dfg.value_def(length) {
        ValueDef::Param(block, _) => FuncCursor::new(f).at_first_inst(block),
        ValueDef::Result(inst, _) => FuncCursor::new(f).after_inst(inst),
        ValueDef::Union(_, _) => unreachable!("candidate has a current length definition"),
    };
    let delta = width - 1;
    let enough = cursor
        .ins()
        .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, length, delta as i64);
    let end = cursor.ins().iadd_imm(length, delta.wrapping_neg() as i64);
    let zero = cursor.ins().iconst(types::I64, 0);
    cursor.ins().select(enough, end, zero)
}

pub(super) fn combine(f: &mut Function, cfg: &ControlFlowGraph, dom: &DominatorTree) -> usize {
    let mut groups: BTreeMap<_, Vec<Window>> = BTreeMap::new();
    let mut used = BTreeSet::new();
    for block in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        if let Some(window) = f
            .layout
            .last_inst(block)
            .and_then(|inst| candidate(f, cfg, dom, inst))
            && !used.contains(&window.first)
            && !used.contains(&window.second)
        {
            used.extend([window.first, window.second]);
            groups
                .entry((window.length, window.width))
                .or_default()
                .push(window);
        }
    }
    let mut removed = 0;
    for ((length, width), windows) in groups.into_iter().filter(|(_, windows)| windows.len() > 1) {
        // length >= width && index <= length - width exactly when
        // index < saturating_sub(length, width - 1).
        let exclusive_end = limit(f, length, width);
        for window in windows {
            let mut cursor = FuncCursor::new(f).at_inst(window.first);
            let valid = cursor
                .ins()
                .icmp(IntCC::UnsignedLessThan, window.index, exclusive_end);
            cursor.func.dfg.inst_args_mut(window.first)[0] = valid;
            let Data::Brif {
                blocks: [selected, _],
                ..
            } = f.dfg.insts[window.second]
            else {
                unreachable!("window comparison changed before emission");
            };
            let target = selected.block(&f.dfg.value_lists);
            let args = selected.args(&f.dfg.value_lists).collect::<Vec<_>>();
            f.dfg.replace(window.second).jump(target, &args);
            removed += 1;
        }
    }
    removed
}

#[cfg(test)]
mod tests;
