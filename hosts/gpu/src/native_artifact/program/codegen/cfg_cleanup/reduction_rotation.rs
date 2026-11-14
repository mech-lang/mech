//! Move a nested loop's pure equality test to its initial and recurring
//! incoming edges. Keep the current visit's typed values on both exits.
use super::composed_reads::read_comparison;
use super::window_bounds::sole_predecessor;
use cranelift_codegen::cursor::{Cursor, FuncCursor};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, BlockArg, BlockCall, Function, Inst, InstBuilder, InstructionData as Data, Opcode,
    Value, ValueDef,
};
use cranelift_codegen::loop_analysis::{Loop, LoopAnalysis};
use std::collections::{BTreeMap, BTreeSet};

#[cfg(all(test, feature = "jit"))]
#[path = "reduction_rotation_tests.rs"]
mod tests;

struct Incoming {
    jump: Inst,
    args: Vec<BlockArg>,
}

struct Plan {
    header: Block,
    test: Inst,
    body: BlockCall,
    exit: Block,
    cursor: usize,
    stop: Value,
    incoming: [Incoming; 2],
    live_exit: BTreeMap<Value, Vec<Inst>>,
}

fn next_test(
    f: &Function,
    cfg: &ControlFlowGraph,
    loops: &LoopAnalysis,
    region: Loop,
    header: Block,
) -> Option<(Inst, BTreeSet<Block>)> {
    let mut block = header;
    let mut path = BTreeSet::new();
    while path.insert(block) {
        let last = f.layout.last_inst(block)?;
        // These operations are total modulo their scalar width. In
        // particular the corridor contains no load, checked add or debit.
        if !f
            .layout
            .block_insts(block)
            .take_while(|&inst| inst != last)
            .all(|inst| {
                matches!(
                    f.dfg.insts[inst].opcode(),
                    Opcode::IaddImm | Opcode::Icmp | Opcode::Uextend | Opcode::BandImm
                )
            })
        {
            return None;
        }
        let Data::Jump { destination, .. } = f.dfg.insts[last] else {
            return Some((last, path));
        };
        let next = destination.block(&f.dfg.value_lists);
        if destination.len(&f.dfg.value_lists) != 0
            || !loops.is_in_loop(next, region)
            || !f.dfg.block_params(next).is_empty()
            || sole_predecessor(cfg, next) != Some((block, last))
        {
            return None;
        }
        block = next;
    }
    None
}

fn plan(
    f: &Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
    loops: &LoopAnalysis,
    region: Loop,
) -> Option<Plan> {
    loops.loop_parent(region)?;
    let header = loops.loop_header(region);
    let (test, corridor) = next_test(f, cfg, loops, region, header)?;
    let Data::Brif {
        arg,
        blocks: [body, exit],
        ..
    } = f.dfg.insts[test]
    else {
        return None;
    };
    let body_block = body.block(&f.dfg.value_lists);
    let exit_block = exit.block(&f.dfg.value_lists);
    let test_block = f.layout.inst_block(test)?;
    if !loops.is_in_loop(body_block, region)
        || corridor.contains(&body_block)
        || sole_predecessor(cfg, body_block) != Some((test_block, test))
        || loops.is_in_loop(exit_block, region)
        || exit.len(&f.dfg.value_lists) != 0
        || !f.dfg.block_params(exit_block).is_empty()
        || sole_predecessor(cfg, exit_block) != Some((test_block, test))
    {
        return None;
    }
    let compare = read_comparison(f, arg)?;
    let Data::IntCompare {
        cond: IntCC::NotEqual,
        args: [position, stop],
        ..
    } = f.dfg.insts[compare]
    else {
        return None;
    };
    let position = f.dfg.resolve_aliases(position);
    let ValueDef::Param(cursor_block, cursor) = f.dfg.value_def(position) else {
        return None;
    };
    let stop = f.dfg.resolve_aliases(stop);
    let stop_block = match f.dfg.value_def(stop) {
        ValueDef::Param(block, _) => block,
        ValueDef::Result(inst, _) => f.layout.inst_block(inst)?,
        ValueDef::Union(..) => return None,
    };
    let parameters = f.dfg.block_params(header);
    if cursor_block != header
        || parameters.get(cursor) != Some(&position)
        || loops.is_in_loop(stop_block, region)
    {
        return None;
    }
    let mut preds = cfg.pred_iter(header);
    let first = preds.next()?;
    let second = preds.next()?;
    if preds.next().is_some()
        || loops.is_in_loop(first.block, region) == loops.is_in_loop(second.block, region)
    {
        return None;
    }
    let incoming = [first, second].map(|edge| {
        let Data::Jump { destination, .. } = f.dfg.insts[edge.inst] else {
            return None;
        };
        let available = match f.dfg.value_def(stop) {
            ValueDef::Param(block, _) => dom.dominates(block, edge.inst, &f.layout),
            ValueDef::Result(inst, _) => {
                inst != edge.inst && dom.dominates(inst, edge.inst, &f.layout)
            }
            ValueDef::Union(..) => false,
        };
        let args = destination.args(&f.dfg.value_lists).collect::<Vec<_>>();
        (available
            && destination.block(&f.dfg.value_lists) == header
            && args.len() == parameters.len()
            && args.iter().all(|word| word.as_value().is_some()))
        .then_some(Incoming {
            jump: edge.inst,
            args,
        })
    });
    let [Some(first), Some(second)] = incoming else {
        return None;
    };
    if corridor.contains(&f.layout.inst_block(first.jump)?)
        || corridor.contains(&f.layout.inst_block(second.jump)?)
    {
        return None;
    }

    // Any use made reachable on the new exit without going through the
    // corridor must receive its current SSA argument. Shared joins and
    // non-parameter live-outs require a different proof.
    let mut live_exit = BTreeMap::<Value, Vec<Inst>>::new();
    for block in f
        .layout
        .blocks()
        .filter(|block| !corridor.contains(block) && !dom.block_dominates(body_block, *block))
    {
        for inst in f.layout.block_insts(block) {
            for value in f
                .dfg
                .inst_values(inst)
                .map(|word| f.dfg.resolve_aliases(word))
            {
                match f.dfg.value_def(value) {
                    ValueDef::Param(def, _)
                        if def == header && dom.block_dominates(exit_block, block) =>
                    {
                        if f.dfg.insts[inst].exception_table().is_some()
                            || f.dfg.insts[inst].opcode() == Opcode::BrTable
                        {
                            return None;
                        }
                        let uses = live_exit.entry(value).or_default();
                        if uses.last() != Some(&inst) {
                            uses.push(inst);
                        }
                    }
                    ValueDef::Param(def, _) if corridor.contains(&def) => return None,
                    ValueDef::Result(def, _)
                        if f.layout
                            .inst_block(def)
                            .is_some_and(|def| corridor.contains(&def)) =>
                    {
                        return None;
                    }
                    _ => {}
                }
            }
        }
    }
    Some(Plan {
        header,
        test,
        body,
        exit: exit_block,
        cursor,
        stop,
        incoming: [first, second],
        live_exit,
    })
}

fn apply(f: &mut Function, plan: Plan) {
    let mut exit_lanes = Vec::new();
    for (old, insts) in plan.live_exit {
        let ValueDef::Param(_, lane) = f.dfg.value_def(old) else {
            unreachable!()
        };
        let ty = f.dfg.value_type(old);
        let new = f.dfg.append_block_param(plan.exit, ty);
        exit_lanes.push(lane);
        for inst in insts {
            let aliases = f
                .dfg
                .inst_values(inst)
                .filter(|&word| f.dfg.resolve_aliases(word) == old)
                .collect::<Vec<_>>();
            f.dfg.map_inst_values(
                inst,
                |word| if aliases.contains(&word) { new } else { word },
            );
        }
    }
    let body = plan.body.block(&f.dfg.value_lists);
    let args = plan.body.args(&f.dfg.value_lists).collect::<Vec<_>>();
    f.dfg.replace(plan.test).jump(body, &args);
    let location = f.srcloc(plan.test);
    for Incoming { jump, args } in plan.incoming {
        let mut cursor = FuncCursor::new(f).at_inst(jump);
        cursor.set_srcloc(location);
        let current = args[plan.cursor].as_value().expect("ordinary loop edge");
        let complete = cursor.ins().icmp(IntCC::Equal, current, plan.stop);
        let finished = exit_lanes.iter().map(|&i| args[i]).collect::<Vec<_>>();
        cursor
            .func
            .dfg
            .replace(jump)
            .brif(complete, plan.exit, &finished, plan.header, &args);
    }
}

pub(in crate::native_artifact) fn rotate(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> bool {
    if f.dfg.values_labels.is_some() {
        return false;
    }
    let mut loops = LoopAnalysis::new();
    loops.compute(f, cfg, dom);
    // One innermost eligible reduction per body. Moving its predecessor
    // tests invalidates the original CFG and dominance snapshot.
    let selected = loops
        .loops()
        .filter_map(|region| {
            let plan = plan(f, cfg, dom, &loops, region)?;
            Some((loops.loop_level(plan.header), plan))
        })
        .max_by_key(|(level, _)| level.level());
    if let Some((_, plan)) = selected {
        apply(f, plan);
        true
    } else {
        false
    }
}
