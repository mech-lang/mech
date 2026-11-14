//! Reuse a nonwrapping extent for two ordered bases in the same current view.
use super::{comparison, literal, sole_predecessor};
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, Function, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeSet;

fn current(f: &Function, value: Value) -> Value {
    f.dfg.resolve_aliases(value)
}

fn definition(f: &Function, value: Value) -> Option<Data> {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(current(f, value)) else {
        return None;
    };
    Some(f.dfg.insts[inst])
}

fn unsigned_order(f: &Function, flag: Value, selected: bool) -> Option<(Value, Value)> {
    let Data::IntCompare {
        cond,
        args: [left, right],
        ..
    } = f.dfg.insts[comparison(f, flag)?]
    else {
        return None;
    };
    if f.dfg.value_type(left) != types::I64 || f.dfg.value_type(right) != types::I64 {
        return None;
    }
    let cond = if selected { cond } else { cond.complement() };
    let left = current(f, left);
    let right = current(f, right);
    match cond {
        IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual => Some((left, right)),
        IntCC::UnsignedGreaterThan | IntCC::UnsignedGreaterThanOrEqual => Some((right, left)),
        _ => None,
    }
}

fn selected_edge(f: &Function, cfg: &ControlFlowGraph, block: Block) -> Option<(Value, bool)> {
    let (_, branch) = sole_predecessor(cfg, block)?;
    let Data::Brif {
        arg,
        blocks: [yes, no],
        ..
    } = f.dfg.insts[branch]
    else {
        return None;
    };
    let yes = yes.block(&f.dfg.value_lists);
    let no = no.block(&f.dfg.value_lists);
    (yes != no).then_some((arg, yes == block))
}

fn selected_orders(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    block: Block,
) -> BTreeSet<(Value, Value)> {
    let mut orders = BTreeSet::new();
    let mut pending = Vec::new();
    let mut seen = BTreeSet::new();
    let mut here = Some(block);
    while let Some(block) = here {
        if let Some(edge) = selected_edge(f, cfg, block) {
            pending.push(edge);
        }
        here = dom.idom(block);
    }
    while let Some((flag, selected)) = pending.pop() {
        if !seen.insert((current(f, flag), selected)) {
            continue;
        }
        if let Some(order) = unsigned_order(f, flag, selected) {
            orders.insert(order);
        } else if selected
            && let Some(inst) = comparison(f, flag)
            && let Data::Binary {
                opcode: Opcode::Band,
                args,
            } = f.dfg.insts[inst]
        {
            // A nonzero conjunction establishes each operand. A false
            // conjunction establishes no individual comparison.
            pending.extend(args.map(|arg| (arg, true)));
        }
    }
    orders
}

fn boolean_terms(f: &Function, condition: Value) -> Option<Vec<Value>> {
    let mut pending = vec![condition];
    let mut terms = Vec::new();
    let mut seen = BTreeSet::new();
    while let Some(value) = pending.pop() {
        let value = current(f, value);
        if !seen.insert(value) {
            continue;
        }
        if f.dfg.value_type(value) != types::I8 {
            return None;
        }
        match definition(f, value)? {
            Data::Binary {
                opcode: Opcode::Band,
                args,
            } => pending.extend(args.into_iter().rev()),
            Data::IntCompare { .. } | Data::IntCompareImm { .. } => terms.push(value),
            _ if literal(f, value).is_some_and(|n| n <= 1) => terms.push(value),
            _ => return None,
        }
    }
    Some(terms)
}

struct Window {
    base: Value,
    extent: Value,
    length: Value,
    base_check: Value,
    fit_check: Value,
}

fn window(f: &Function, terms: &[Value], fit_check: Value) -> Option<Window> {
    let Data::IntCompare {
        cond: IntCC::UnsignedLessThanOrEqual,
        args: [extent, available],
        ..
    } = definition(f, fit_check)?
    else {
        return None;
    };
    let Data::Binary {
        opcode: Opcode::Isub,
        args: [length, base],
    } = definition(f, available)?
    else {
        return None;
    };
    let [length, base, extent] = [length, base, extent].map(|v| current(f, v));
    let base_check = terms.iter().copied().find(|&term| {
        matches!(definition(f, term),
            Some(Data::IntCompare { cond: IntCC::UnsignedLessThanOrEqual, args: [a, b], .. })
                if current(f, a) == base && current(f, b) == length)
    })?;
    Some(Window {
        base,
        extent,
        length,
        base_check,
        fit_check,
    })
}

fn proves_pair(
    f: &Function,
    orders: &BTreeSet<(Value, Value)>,
    first: &Window,
    second: &Window,
) -> bool {
    orders.iter().any(|&(end, limit)| {
        if limit != first.length {
            return false;
        }
        let Some(Data::Binary {
            opcode: Opcode::Iadd,
            args: [left, right],
        }) = definition(f, end)
        else {
            return false;
        };
        let [left, right] = [left, right].map(|v| current(f, v));
        let last_base = if left == first.extent {
            right
        } else if right == first.extent {
            left
        } else {
            return false;
        };
        let nonwrapping =
            orders.contains(&(last_base, end)) || orders.contains(&(first.extent, end));
        // Both bases are no later than a checked start with the same extent.
        nonwrapping
            && [first.base, second.base]
                .into_iter()
                .all(|base| base == last_base || orders.contains(&(base, last_base)))
    })
}

pub(in super::super) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let changes = f
        .layout
        .blocks()
        .filter(|&block| dom.is_reachable(block))
        .filter_map(|block| {
            let branch = f.layout.last_inst(block)?;
            let Data::Brif { arg, .. } = f.dfg.insts[branch] else {
                return None;
            };
            let mut terms = boolean_terms(f, arg)?;
            let windows = terms
                .iter()
                .filter_map(|&term| window(f, &terms, term))
                .collect::<Vec<_>>();
            let [first, second] = windows.as_slice() else {
                return None;
            };
            if first.base == second.base
                || first.extent != second.extent
                || first.length != second.length
                || !proves_pair(f, &selected_orders(f, cfg, dom, block), first, second)
            {
                return None;
            }
            terms.retain(|term| {
                ![
                    first.base_check,
                    first.fit_check,
                    second.base_check,
                    second.fit_check,
                ]
                .contains(term)
            });
            Some((branch, terms))
        })
        .collect::<Vec<_>>();
    for (branch, terms) in &changes {
        let mut cursor = FuncCursor::new(f).at_inst(*branch);
        let condition = terms
            .iter()
            .copied()
            .reduce(|left, right| cursor.ins().band(left, right))
            .unwrap_or_else(|| cursor.ins().iconst(types::I8, 1));
        cursor.func.dfg.inst_args_mut(*branch)[0] = condition;
    }
    changes.len()
}

#[cfg(test)]
mod tests;
