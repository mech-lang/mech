//! Forward a current compound stop decision through a pure loop continuation.
//! Selected arms still execute their original effects before entering the route.

pub(in crate::native_artifact) mod branch_form;
pub(in crate::native_artifact) mod comparison_chain;

use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode,
    Value, ValueDef, types,
};
use std::collections::{BTreeMap, BTreeSet};

const ROUTE_LIMIT: usize = 96;

#[derive(Clone, Copy)]
enum Word {
    Value(Value),
    Known(bool),
}

#[derive(Debug)]
struct Edit {
    from: Inst,
    block: Block,
    target: Block,
    args: Vec<BlockArg>,
}

struct Route {
    edit: Edit,
    skipped: BTreeSet<Block>,
    selected_arm: BTreeSet<Block>,
    loop_entry: bool,
}

fn canonical_comparison(f: &Function, value: Value) -> Option<()> {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(extension, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let Data::Unary {
        opcode: Opcode::Uextend,
        arg,
    } = f.dfg.insts[extension]
    else {
        return None;
    };
    if !f.dfg.value_type(arg).is_int() {
        return None;
    }
    let ValueDef::Result(test, 0) = f.dfg.value_def(f.dfg.resolve_aliases(arg)) else {
        return None;
    };
    matches!(f.dfg.insts[test].opcode(), Opcode::Icmp | Opcode::IcmpImm).then_some(())
}

fn unmask(f: &Function, value: Value) -> Value {
    let value = f.dfg.resolve_aliases(value);
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return value;
    };
    match f.dfg.insts[inst] {
        Data::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } if imm.bits() == 1 => f.dfg.resolve_aliases(arg),
        _ => value,
    }
}

fn compound(f: &Function, value: Value) -> Option<Value> {
    let value = unmask(f, value);
    if f.dfg.value_type(value) != types::I64 {
        return None;
    }
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let Data::Binary {
        opcode: Opcode::Bor,
        args: [a, b],
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    canonical_comparison(f, a)?;
    canonical_comparison(f, b)?;
    Some(value)
}

fn jump(f: &Function, block: Block) -> Option<(Inst, BlockCall)> {
    let inst = f.layout.last_inst(block)?;
    let Data::Jump { destination, .. } = f.dfg.insts[inst] else {
        return None;
    };
    Some((inst, destination))
}

// The selected effects stay on their original path. Prove that the
// terminating jump can only be entered with this branch edge selected.
fn arm_end(
    f: &Function,
    cfg: &ControlFlowGraph,
    initial: Inst,
    call: BlockCall,
) -> Option<(Inst, BlockCall, BTreeSet<Block>)> {
    if call.args(&f.dfg.value_lists).next().is_some() {
        return None;
    }
    let mut block = call.block(&f.dfg.value_lists);
    let mut incoming = initial;
    let mut selected_arm = BTreeSet::new();
    for _ in 0..ROUTE_LIMIT {
        if Some(block) == f.layout.entry_block()
            || !f.dfg.block_params(block).is_empty()
            || !selected_arm.insert(block)
        {
            return None;
        }
        let mut predecessors = cfg.pred_iter(block);
        if predecessors.next()?.inst != incoming || predecessors.next().is_some() {
            return None;
        }
        let (inst, call) = jump(f, block)?;
        if call.args(&f.dfg.value_lists).next().is_some() {
            return Some((inst, call, selected_arm));
        }
        incoming = inst;
        block = call.block(&f.dfg.value_lists);
    }
    None
}

fn available(f: &Function, dom: &DominatorTree, value: Value, origin: Inst) -> bool {
    match f.dfg.value_def(value) {
        ValueDef::Param(block, _) => dom.dominates(block, origin, &f.layout),
        ValueDef::Result(inst, _) if f.layout.inst_block(inst).is_some() => {
            inst != origin && dom.dominates(inst, origin, &f.layout)
        }
        _ => false,
    }
}

fn forwarded(f: &Function, words: &BTreeMap<Value, Word>, value: Value) -> Word {
    let value = f.dfg.resolve_aliases(value);
    words.get(&value).copied().unwrap_or(Word::Value(value))
}

fn truth(f: &Function, word: Word) -> Option<bool> {
    let value = match word {
        Word::Known(value) => return Some(value),
        Word::Value(value) => value,
    };
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return None;
    };
    let Data::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
    } = f.dfg.insts[inst]
    else {
        return None;
    };
    match imm.bits() {
        0 => Some(false),
        1 => Some(true),
        _ => None,
    }
}

fn trace(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    (origin, mut call, selected_arm): (Inst, BlockCall, BTreeSet<Block>),
    original_test: Value,
    selected: bool,
) -> Option<Route> {
    let source = f.layout.inst_block(origin)?;
    let mut words = BTreeMap::from([(original_test, Word::Known(selected))]);
    let mut skipped = BTreeSet::new();
    let mut reused_decision = false;
    let mut inverted = false;
    for _ in 0..ROUTE_LIMIT {
        let block = call.block(&f.dfg.value_lists);
        let params = f.dfg.block_params(block);
        let loop_entry = dom.block_dominates(block, source);
        if loop_entry || (reused_decision && !params.is_empty() && cfg.pred_iter(block).count() > 1)
        {
            if !reused_decision || !inverted || skipped.contains(&block) {
                return None;
            }
            let args = call
                .args(&f.dfg.value_lists)
                .zip(params)
                .map(|(arg, &param)| {
                    let Word::Value(value) = forwarded(f, &words, arg.as_value()?) else {
                        return None;
                    };
                    (f.dfg.value_type(value) == f.dfg.value_type(param)
                        && available(f, dom, value, origin))
                    .then(|| value.into())
                })
                .collect::<Option<Vec<_>>>()?;
            if args.len() != params.len() || call.args(&f.dfg.value_lists).count() != params.len() {
                return None;
            }
            return Some(Route {
                edit: Edit {
                    from: origin,
                    block: source,
                    target: block,
                    args,
                },
                skipped,
                selected_arm,
                loop_entry,
            });
        }
        if !skipped.insert(block) || Some(block) == f.layout.entry_block() {
            return None;
        }
        let args = call
            .args(&f.dfg.value_lists)
            .map(|arg| Some(forwarded(f, &words, arg.as_value()?)))
            .collect::<Option<Vec<_>>>()?;
        if args.len() != params.len() {
            return None;
        }
        for (&param, word) in params.iter().zip(args) {
            words.insert(param, word);
        }
        let last = f.layout.last_inst(block)?;
        for inst in f.layout.block_insts(block).take_while(|&inst| inst != last) {
            let Data::BinaryImm64 { opcode, arg, imm } = f.dfg.insts[inst] else {
                return None;
            };
            if imm.bits() != 1 {
                return None;
            }
            let value = truth(f, forwarded(f, &words, arg))?;
            let value = match opcode {
                Opcode::BandImm => value,
                Opcode::BxorImm => {
                    inverted |= f.dfg.resolve_aliases(arg) == original_test;
                    !value
                }
                _ => return None,
            };
            words.insert(f.dfg.first_result(inst), Word::Known(value));
        }
        call = match f.dfg.insts[last] {
            Data::Jump { destination, .. } => destination,
            Data::Brif { arg, blocks, .. } => {
                let Word::Known(value) = forwarded(f, &words, arg) else {
                    return None;
                };
                reused_decision = true;
                blocks[usize::from(!value)]
            }
            _ => return None,
        };
    }
    None
}

// The proposed CFG must still visit each skipped definition before any
// use that has not been replaced by a destination parameter. Stop at its
// defining block: that visit executes the original definition again.
fn has_live_out(
    f: &Function,
    cfg: &ControlFlowGraph,
    uses: &BTreeMap<Value, BTreeSet<Block>>,
    edits: &BTreeMap<Block, Block>,
    route: &Route,
) -> bool {
    for &owner in &route.skipped {
        let outside: BTreeSet<_> = f
            .dfg
            .block_params(owner)
            .iter()
            .copied()
            .chain(
                f.layout
                    .block_insts(owner)
                    .flat_map(|inst| f.dfg.inst_results(inst).iter().copied()),
            )
            .filter_map(|value| uses.get(&value))
            .flatten()
            .copied()
            .collect();
        if outside.is_empty() {
            continue;
        }
        let mut seen = BTreeSet::new();
        let mut pending = vec![route.edit.target];
        while let Some(block) = pending.pop() {
            if block == owner || !seen.insert(block) {
                continue;
            }
            if outside.contains(&block) {
                return true;
            }
            if let Some(&target) = edits.get(&block) {
                pending.push(target);
            } else {
                pending.extend(cfg.succ_iter(block));
            }
        }
    }
    false
}

pub(in crate::native_artifact) fn optimize(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let mut pairs = Vec::new();
    for block in f.layout.blocks().filter(|&block| dom.is_reachable(block)) {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif {
            arg,
            blocks: [yes, no],
            ..
        } = f.dfg.insts[inst]
        else {
            continue;
        };
        if yes.block(&f.dfg.value_lists) == no.block(&f.dfg.value_lists) {
            continue;
        }
        let Some(test) = compound(f, arg) else {
            continue;
        };
        let Some(yes) = arm_end(f, cfg, inst, yes) else {
            continue;
        };
        let Some(no) = arm_end(f, cfg, inst, no) else {
            continue;
        };
        if yes.0 == no.0 || yes.1.block(&f.dfg.value_lists) != no.1.block(&f.dfg.value_lists) {
            continue;
        }
        let Some(taken) = trace(f, cfg, dom, yes, test, true) else {
            continue;
        };
        let Some(untaken) = trace(f, cfg, dom, no, test, false) else {
            continue;
        };
        if taken.loop_entry != untaken.loop_entry && taken.edit.target != untaken.edit.target {
            pairs.push([taken, untaken]);
        }
    }
    if pairs.is_empty() {
        return 0;
    }
    let routes = pairs.iter().flatten().collect::<Vec<_>>();
    let edits: BTreeMap<_, _> = routes
        .iter()
        .map(|route| (route.edit.block, route.edit.target))
        .collect();
    // A shortcut must not create a second entry to an arm whose original
    // predicate is assumed. Nor may it edit a jump in another proof's route.
    if edits.len() != routes.len()
        || routes.iter().any(|route| {
            edits.iter().any(|(source, target)| {
                route.skipped.contains(source) || route.selected_arm.contains(target)
            })
        })
    {
        return 0;
    }
    let mut uses = BTreeMap::<Value, BTreeSet<Block>>::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            if edits.contains_key(&block) && f.layout.last_inst(block) == Some(inst) {
                continue;
            }
            for value in f.dfg.inst_values(inst) {
                uses.entry(f.dfg.resolve_aliases(value))
                    .or_default()
                    .insert(block);
            }
        }
    }
    for route in &routes {
        for arg in &route.edit.args {
            let value = arg.as_value().expect("checked ordinary edge");
            uses.entry(value).or_default().insert(route.edit.block);
        }
    }
    if routes
        .iter()
        .any(|route| has_live_out(f, cfg, &uses, &edits, route))
    {
        return 0;
    }
    let count = routes.len();
    for route in pairs.into_iter().flatten() {
        let Edit {
            from, target, args, ..
        } = route.edit;
        f.dfg.replace(from).jump(target, &args);
    }
    count
}

#[cfg(all(test, feature = "jit"))]
mod tests;
