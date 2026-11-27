//! A formal receiving one dominating SSA value is an alias, even when that
//! value is a changing outer-loop phi. No effect or instruction is moved.
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{Function, ValueDef};
use std::collections::BTreeMap;

pub(in super::super) fn coalesce(
    function: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    if function.preferred_fallthrough.is_some()
        || function.dfg.values_labels.is_some()
        || !function.debug_tags.is_empty()
    {
        return 0;
    }
    // The proof only understands attached SSA. Egraph extraction can reveal
    // parameter uses that are not visible through the current instruction.
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            if function.dfg.inst_values(inst).any(|value| {
                matches!(
                    function.dfg.value_def(function.dfg.resolve_aliases(value)),
                    ValueDef::Union(..)
                )
            }) {
                return 0;
            }
        }
    }
    let mut replacements = BTreeMap::new();
    for block in function.layout.blocks() {
        if Some(block) == function.layout.entry_block() || !dom.is_reachable(block) {
            continue;
        }
        for (slot, &formal) in function.dfg.block_params(block).iter().enumerate() {
            let mut candidate = None;
            let mut valid = true;
            for (_, value) in super::incoming(function, cfg, block, slot) {
                let Some(value) = value else {
                    valid = false;
                    break;
                };
                if function.dfg.value_type(value) != function.dfg.value_type(formal) {
                    valid = false;
                    break;
                }
                if value == formal {
                    continue;
                }
                if candidate.is_some_and(|source| source != value) {
                    valid = false;
                    break;
                }
                candidate = Some(value);
            }
            let Some(source) = candidate.filter(|_| valid) else {
                continue;
            };
            let dominates = match function.dfg.value_def(source) {
                ValueDef::Param(owner, _) => {
                    owner != block && dom.is_reachable(owner) && dom.block_dominates(owner, block)
                }
                ValueDef::Result(inst, _) => dom.dominates(inst, block, &function.layout),
                _ => false,
            };
            if dominates {
                replacements.insert(formal, source);
            }
        }
    }
    super::apply(function, cfg, &replacements);
    replacements.len()
}
