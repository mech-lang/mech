//! Merge distinct activation entries before a natural loop, not on its backedge.
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{BlockCall, Function, InstBuilder};
use std::collections::BTreeSet;

pub(super) fn normalize(
    function: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    // Snapshot original incoming instructions before changing the CFG. Edges
    // to other headers may share an instruction, but retain their destinations.
    let candidates = dom
        .cfg_postorder()
        .iter()
        .copied()
        .filter_map(|header| {
            if Some(header) == function.layout.entry_block()
                || function.dfg.block_params(header).is_empty()
                || !cfg
                    .pred_iter(header)
                    .any(|pred| dom.block_dominates(header, pred.block))
            {
                return None;
            }
            let incoming = cfg
                .pred_iter(header)
                .filter(|pred| !dom.block_dominates(header, pred.block))
                .map(|pred| pred.inst)
                .collect::<BTreeSet<_>>();
            let mut edges = 0;
            for &inst in &incoming {
                for call in function.dfg.insts[inst]
                    .branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
                {
                    if call.block(&function.dfg.value_lists) == header {
                        // Exceptional edge-only results keep their original
                        // destination and landing convention.
                        if call
                            .args(&function.dfg.value_lists)
                            .any(|arg| arg.as_value().is_none())
                        {
                            return None;
                        }
                        edges += 1;
                    }
                }
            }
            (edges > 1).then_some((header, incoming))
        })
        .collect::<Vec<_>>();
    for (header, incoming) in &candidates {
        let types = function
            .dfg
            .block_params(*header)
            .iter()
            .map(|&value| function.dfg.value_type(value))
            .collect::<Vec<_>>();
        let entry = function.dfg.make_block();
        function.layout.insert_block(entry, *header);
        let args = types
            .into_iter()
            .map(|ty| function.dfg.append_block_param(entry, ty).into())
            .collect::<Vec<_>>();
        FuncCursor::new(function)
            .at_bottom(entry)
            .ins()
            .jump(*header, &args);
        for &inst in incoming {
            let dfg = &mut function.dfg;
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                if call.block(&dfg.value_lists) == *header {
                    let args = call.args(&dfg.value_lists).collect::<Vec<_>>();
                    *call = BlockCall::new(entry, args, &mut dfg.value_lists);
                }
            }
        }
    }
    candidates.len()
}
