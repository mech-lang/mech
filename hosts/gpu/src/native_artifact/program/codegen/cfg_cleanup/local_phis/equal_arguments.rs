//! Two formals can be equal on every incoming edge while their shared value
//! changes with the edge. Demand the proof at an integer compare or subtract.
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::{
    Block, Function, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::BTreeMap;

fn formals(f: &Function, left: Value, right: Value) -> Option<(Block, usize, usize)> {
    let ValueDef::Param(block, left_slot) = f.dfg.value_def(left) else {
        return None;
    };
    let ValueDef::Param(owner, right_slot) = f.dfg.value_def(right) else {
        return None;
    };
    (block == owner
        && Some(block) != f.layout.entry_block()
        && f.dfg.value_type(left) == types::I64
        && f.dfg.value_type(right) == types::I64)
        .then_some((block, left_slot, right_slot))
}

fn agree(
    f: &Function,
    cfg: &ControlFlowGraph,
    left: Value,
    right: Value,
    proofs: &mut BTreeMap<(Value, Value), bool>,
) -> bool {
    let pair = |a, b| {
        let a = f.dfg.resolve_aliases(a);
        let b = f.dfg.resolve_aliases(b);
        (a.min(b), a.max(b))
    };
    enum Step {
        Check((Value, Value)),
        Complete((Value, Value)),
    }
    let mut pending = vec![Step::Check(pair(left, right))];
    while let Some(step) = pending.pop() {
        let key = match step {
            Step::Complete(key) => {
                proofs.insert(key, true);
                continue;
            }
            Step::Check(key) if key.0 == key.1 => continue,
            Step::Check(key) => key,
        };
        if let Some(&proved) = proofs.get(&key) {
            if proved {
                continue;
            }
            return false;
        }
        // A recursive dependence supplies no evidence for the pair.
        proofs.insert(key, false);
        let Some((block, left_slot, right_slot)) = formals(f, key.0, key.1) else {
            return false;
        };
        pending.push(Step::Complete(key));
        let mut reached = false;
        for pred in cfg.pred_iter(block) {
            for call in f.dfg.insts[pred.inst]
                .branch_destination(&f.dfg.jump_tables, &f.dfg.exception_tables)
            {
                if call.block(&f.dfg.value_lists) != block {
                    continue;
                }
                let arg = |slot| call.args(&f.dfg.value_lists).nth(slot)?.as_value();
                let (Some(a), Some(b)) = (arg(left_slot), arg(right_slot)) else {
                    return false;
                };
                pending.push(Step::Check(pair(a, b)));
                reached = true;
            }
        }
        if !reached {
            return false;
        }
    }
    true
}

pub(in super::super) fn simplify(f: &mut Function, cfg: &ControlFlowGraph) -> usize {
    if f.dfg.values_labels.is_some() {
        return 0;
    }
    let mut proofs = BTreeMap::new();
    let mut replacements = BTreeMap::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            let [left, right] = match f.dfg.insts[inst] {
                Data::IntCompare { args, .. }
                | Data::Binary {
                    opcode: Opcode::Isub,
                    args,
                } => args.map(|v| f.dfg.resolve_aliases(v)),
                _ => continue,
            };
            let Some((_, a, b)) = formals(f, left, right) else {
                continue;
            };
            if a != b && agree(f, cfg, left, right, &mut proofs) {
                let (old, current) = if a < b { (right, left) } else { (left, right) };
                replacements.insert(old, current);
            }
        }
    }
    super::apply(f, cfg, &replacements);
    replacements.len()
}
