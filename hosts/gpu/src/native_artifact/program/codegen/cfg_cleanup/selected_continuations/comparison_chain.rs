//! Short-circuit a branch-only conjunction after a shared Boolean decision.

use super::{compound, jump};
use crate::native_artifact::program::codegen::cfg_cleanup::window_bounds::sole_predecessor;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockCall, Function, Inst, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

const MAX_CORRIDOR_BLOCKS: usize = 32;

struct Site {
    blocks: BTreeSet<Block>,
    shared: Value,
    tests: [(Inst, Value); 3],
    branch: Inst,
    last_test: Value,
    failure: Block,
}

fn definition(f: &Function, value: Value) -> Option<Inst> {
    match f.dfg.value_def(f.dfg.resolve_aliases(value)) {
        ValueDef::Result(inst, 0) => Some(inst),
        _ => None,
    }
}

fn peel_test(f: &Function, word: Value) -> Option<(Value, Value)> {
    let Data::Binary {
        opcode: Opcode::Band,
        args: [rest, extended],
    } = f.dfg.insts[definition(f, word)?]
    else {
        return None;
    };
    let Data::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = f.dfg.insts[definition(f, extended)?]
    else {
        return None;
    };
    let test = f.dfg.resolve_aliases(arg);
    (f.dfg.value_type(test) == types::I8
        && matches!(
            f.dfg.insts[definition(f, test)?].opcode(),
            Opcode::Icmp | Opcode::IcmpImm
        ))
    .then_some((rest, test))
}

fn pure(f: &Function, block: Block, end: Inst) -> bool {
    f.layout
        .block_insts(block)
        .take_while(|&inst| inst != end)
        .all(|inst| {
            matches!(
                f.dfg.insts[inst].opcode(),
                Opcode::Icmp
                    | Opcode::IcmpImm
                    | Opcode::Uextend
                    | Opcode::Band
                    | Opcode::BandImm
                    | Opcode::BxorImm
                    | Opcode::Isub
                    | Opcode::Iadd
                    | Opcode::IaddImm
            )
        })
}

fn candidate(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    block: Block,
) -> Option<Site> {
    let branch = f.layout.last_inst(block)?;
    let Data::Brif {
        arg,
        blocks: [success, failure],
        ..
    } = f.dfg.insts[branch]
    else {
        return None;
    };
    let Data::BinaryImm64 {
        opcode: Opcode::BandImm,
        arg: conjunction,
        imm,
    } = f.dfg.insts[definition(f, arg)?]
    else {
        return None;
    };
    if imm.bits() != 1
        || success.len(&f.dfg.value_lists) != 0
        || failure.len(&f.dfg.value_lists) != 0
    {
        return None;
    }
    let (rest, last_test) = peel_test(f, conjunction)?;
    let (rest, second) = peel_test(f, rest)?;
    let (inverted, first) = peel_test(f, rest)?;
    let inverted = f.dfg.resolve_aliases(inverted);
    let Data::BinaryImm64 {
        opcode: Opcode::BxorImm,
        arg: shared,
        imm,
    } = f.dfg.insts[definition(f, inverted)?]
    else {
        return None;
    };
    if imm.bits() != 1 {
        return None;
    }
    // An arbitrary word XOR 1 is not its logical negation. Require the
    // existing two-comparison proof of a canonical zero-or-one decision.
    compound(f, shared)?;
    let shared = f.dfg.resolve_aliases(shared);
    let [start, first_test, second_test] =
        [inverted, first, second].map(|value| f.layout.inst_block(definition(f, value)?));
    let positions = [start?, first_test?, second_test?];
    let failure = failure.block(&f.dfg.value_lists);
    let success = success.block(&f.dfg.value_lists);
    let mut visited = BTreeSet::new();
    let mut cuts = [(branch, inverted), (branch, first), (branch, second)];
    let mut current = positions[0];
    let mut next = 0;
    for _ in 0..MAX_CORRIDOR_BLOCKS {
        let end = f.layout.last_inst(current)?;
        if !visited.insert(current)
            || current == failure
            || current == success
            || !pure(f, current, end)
        {
            return None;
        }
        if current == block {
            return (next == cuts.len()
                && dom.dominates(definition(f, last_test)?, branch, &f.layout))
            .then_some(Site {
                blocks: visited,
                shared,
                tests: cuts,
                branch,
                last_test,
                failure,
            });
        }
        if positions.get(next) == Some(&current) {
            cuts[next].0 = end;
            next += 1;
        }
        let (_, edge) = jump(f, current)?;
        current = edge.block(&f.dfg.value_lists);
        if Some(current) == f.layout.entry_block()
            || edge.len(&f.dfg.value_lists) != 0
            || !f.dfg.block_params(current).is_empty()
            || sole_predecessor(cfg, current)?.1 != end
        {
            return None;
        }
    }
    None
}

#[cfg(all(test, feature = "jit"))]
#[path = "comparison_chain_tests.rs"]
mod tests;

pub(in crate::native_artifact) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let sites: Vec<_> = f
        .layout
        .blocks()
        .filter(|&block| dom.is_reachable(block))
        .filter_map(|block| candidate(f, cfg, dom, block))
        .collect();
    if sites.is_empty() {
        return 0;
    }
    let mut occupied = BTreeSet::new();
    if !sites
        .iter()
        .all(|site| site.blocks.iter().all(|&block| occupied.insert(block)))
    {
        return 0;
    }
    let mut shared_uses: BTreeMap<_, usize> = sites.iter().map(|site| (site.shared, 0)).collect();
    let mut bypassed = BTreeMap::new();
    for (index, site) in sites.iter().enumerate() {
        let first = f.layout.inst_block(site.tests[0].0).unwrap();
        for &block in &site.blocks {
            if block != first {
                for inst in f.layout.block_insts(block) {
                    for &value in f.dfg.inst_results(inst) {
                        bypassed.insert(value, index);
                    }
                }
            }
        }
    }
    // New false edges skip these definitions. Every original use must be
    // within the corridor, including successor arguments and unwind contexts.
    let mut escapes = vec![false; sites.len()];
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            for value in f.dfg.inst_values(inst).map(|v| f.dfg.resolve_aliases(v)) {
                if let Some(uses) = shared_uses.get_mut(&value) {
                    *uses += 1;
                }
                if let Some(&site) = bypassed.get(&value) {
                    escapes[site] |= !sites[site].blocks.contains(&block);
                }
            }
        }
    }
    let mut changed = 0;
    for (site, escapes) in sites.into_iter().zip(escapes) {
        if escapes || shared_uses[&site.shared] < 2 {
            continue;
        }
        for (inst, value) in site.tests {
            let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
                unreachable!();
            };
            let failure = BlockCall::new(site.failure, [], &mut f.dfg.value_lists);
            f.dfg.insts[inst] = Data::Brif {
                opcode: Opcode::Brif,
                arg: value,
                blocks: [destination, failure],
            };
        }
        let Data::Brif { ref mut arg, .. } = f.dfg.insts[site.branch] else {
            unreachable!();
        };
        *arg = site.last_test;
        changed += 1;
    }
    changed
}
