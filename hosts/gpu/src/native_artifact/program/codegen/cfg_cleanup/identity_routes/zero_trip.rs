//! Forward the false edge of an admission test past the same loop-entry test.

use super::super::{window_bounds, word_ranges::Facts};
use super::route::pure;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockArg, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
    types,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_HOPS: usize = 32;
const MAX_CONJUNCTS: usize = 8;

fn test(f: &Function, condition: Value) -> Option<[Value; 2]> {
    let Data::IntCompare {
        cond: IntCC::UnsignedLessThan,
        args,
        ..
    } = f.dfg.insts[window_bounds::comparison(f, condition)?]
    else {
        return None;
    };
    let args = args.map(|v| f.dfg.resolve_aliases(v));
    (f.dfg.value_type(args[0]) == types::I64).then_some(args)
}

fn admission(f: &Function, condition: Value) -> Option<[Value; 2]> {
    let mut pending = vec![condition];
    let mut facts = Facts::new(f);
    let mut pair = None;
    for _ in 0..MAX_CONJUNCTS {
        let Some(word) = pending.pop() else {
            return pair;
        };
        let word = f.dfg.resolve_aliases(word);
        if facts.condition(word) == Some(true) {
            continue;
        }
        let ValueDef::Result(inst, 0) = f.dfg.value_def(word) else {
            return None;
        };
        if let Data::Binary {
            opcode: Opcode::Band,
            args,
        } = f.dfg.insts[inst]
        {
            if f.dfg.value_type(word) != types::I8
                || !args.iter().all(|&arg| facts.canonical_boolean(arg))
            {
                return None;
            }
            pending.extend(args);
        } else if pair.replace(test(f, word)?).is_some() {
            return None;
        }
    }
    None
}

fn available(f: &Function, dom: &DominatorTree, value: Value, before: Inst) -> bool {
    match f.dfg.value_def(value) {
        ValueDef::Param(block, _) => dom.dominates(block, before, &f.layout),
        ValueDef::Result(inst, _) => {
            inst != before
                && f.layout.inst_block(inst).is_some()
                && dom.dominates(inst, before, &f.layout)
        }
        ValueDef::Union(..) => false,
    }
}

struct Shortcut {
    source: Inst,
    target: Block,
    args: Vec<BlockArg>,
}

fn shortcut(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    branch: Inst,
) -> Option<Shortcut> {
    let Data::Brif {
        arg,
        blocks: [yes, no],
        ..
    } = f.dfg.insts[branch]
    else {
        return None;
    };
    let start = no.block(&f.dfg.value_lists);
    if yes.block(&f.dfg.value_lists) == start
        || Some(start) == f.layout.entry_block()
        || window_bounds::sole_predecessor(cfg, start)?.1 != branch
    {
        return None;
    }
    let expected = admission(f, arg)?;
    let inputs = no
        .args(&f.dfg.value_lists)
        .map(|arg| Some(f.dfg.resolve_aliases(arg.as_value()?)))
        .collect::<Option<Vec<_>>>()?;
    let entry_params = f.dfg.block_params(start);
    if entry_params.len() != inputs.len()
        || entry_params.iter().any(|&p| f.dfg.resolve_aliases(p) != p)
    {
        return None;
    }
    let source = f.layout.last_inst(start)?;
    let Data::Jump { destination, .. } = f.dfg.insts[source] else {
        return None;
    };
    let mut call = destination;
    let mut forwarded = BTreeMap::new();
    let mut visited = BTreeSet::new();
    let mut loop_entry = false;
    let mut resolved = false;
    for _ in 0..MAX_HOPS {
        let block = call.block(&f.dfg.value_lists);
        if dom.block_dominates(block, start) || visited.contains(&block) {
            return None;
        }
        let current = |word| {
            let word = f.dfg.resolve_aliases(word);
            forwarded.get(&word).copied().unwrap_or(word)
        };
        let args = call
            .args(&f.dfg.value_lists)
            .map(|arg| Some(current(arg.as_value()?)))
            .collect::<Option<Vec<_>>>()?;
        let params = f.dfg.block_params(block);
        if params.len() != args.len() || params.iter().any(|&p| f.dfg.resolve_aliases(p) != p) {
            return None;
        }
        if resolved && cfg.pred_iter(block).nth(1).is_some() {
            // An independently reachable join cannot use a skipped definition
            // before visiting that definition again. Its typed inputs carry
            // the exact zero-trip result of this edge.
            if visited
                .iter()
                .any(|&prior| dom.block_dominates(prior, block))
                || !args.iter().all(|&word| available(f, dom, word, source))
            {
                return None;
            }
            return Some(Shortcut {
                source,
                target: block,
                args: args.into_iter().map(BlockArg::from).collect(),
            });
        }
        let end = f.layout.last_inst(block)?;
        if !pure(f, block, end) {
            return None;
        }
        loop_entry |= cfg
            .pred_iter(block)
            .any(|pred| dom.block_dominates(block, pred.block));
        call = match f.dfg.insts[end] {
            Data::Jump { destination, .. } => destination,
            Data::Brif { arg, blocks, .. } if loop_entry && !resolved => {
                let got = test(f, arg)?.map(|word| {
                    let word = params
                        .iter()
                        .position(|&param| param == word)
                        .map_or_else(|| current(word), |index| args[index]);
                    entry_params
                        .iter()
                        .position(|&param| param == word)
                        .map_or(word, |index| inputs[index])
                });
                if got != expected {
                    return None;
                }
                resolved = true;
                blocks[1]
            }
            _ => return None,
        };
        visited.insert(block);
        forwarded.extend(params.iter().copied().zip(args));
    }
    None
}

pub(in crate::native_artifact) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> bool {
    let Some(site) = f
        .layout
        .blocks()
        .filter(|&block| dom.is_reachable(block))
        .find_map(|block| shortcut(f, cfg, dom, f.layout.last_inst(block)?))
    else {
        return false;
    };
    f.dfg.replace(site.source).jump(site.target, &site.args);
    true
}

#[cfg(all(test, feature = "jit"))]
#[path = "zero_trip_tests.rs"]
mod tests;
