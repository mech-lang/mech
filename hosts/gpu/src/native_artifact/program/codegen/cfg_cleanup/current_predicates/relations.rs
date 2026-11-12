//! Reuse comparisons of current unsigned words in the selected dominator
//! subtree. A wrapping sum is keyed by its two current SSA operands.
use super::super::window_bounds::comparison;
use super::incoming;
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{
    Block, Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use std::collections::{BTreeMap, HashMap};

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Operand {
    Word(Value),
    Sum([Value; 2]),
}

impl Operand {
    fn current(f: &Function, value: Value) -> Self {
        let value = f.dfg.resolve_aliases(value);
        if let ValueDef::Result(inst, 0) = f.dfg.value_def(value)
            && let Data::Binary {
                opcode: Opcode::Iadd,
                args,
            } = f.dfg.insts[inst]
        {
            let mut args = args.map(|word| f.dfg.resolve_aliases(word));
            args.sort_unstable();
            return Self::Sum(args);
        }
        Self::Word(value)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
struct Relation {
    condition: IntCC,
    left: Operand,
    right: Operand,
}

impl Relation {
    fn of(f: &Function, inst: Inst) -> Option<Self> {
        let Data::IntCompare {
            cond,
            args: [left, right],
            ..
        } = f.dfg.insts[inst]
        else {
            return None;
        };
        if f.dfg.value_type(left) != types::I64 || f.dfg.value_type(right) != types::I64 {
            return None;
        }
        let (condition, left, right) = match cond {
            IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual => (cond, left, right),
            IntCC::UnsignedGreaterThan | IntCC::UnsignedGreaterThanOrEqual => {
                use cranelift_codegen::ir::condcodes::CondCode;
                (cond.swap_args(), right, left)
            }
            _ => return None,
        };
        let left = Operand::current(f, left);
        let right = Operand::current(f, right);
        // The first experiment is confined to recomputed endpoints. Keeping
        // leaves by SSA identity distinguishes a fresh read from a prior one.
        if !matches!(left, Operand::Sum(_)) && !matches!(right, Operand::Sum(_)) {
            return None;
        }
        Some(Self {
            condition,
            left,
            right,
        })
    }
}

pub(in super::super) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> usize {
    let Some(entry) = f.layout.entry_block() else {
        return 0;
    };
    if f.layout.blocks().any(|block| {
        f.layout
            .block_insts(block)
            .any(|inst| f.dfg.insts[inst].exception_table().is_some())
    }) {
        return 0;
    }
    enum Step {
        Enter(Block),
        Restore(Relation, Option<bool>),
    }
    let mut steps = vec![Step::Enter(entry)];
    let mut facts = HashMap::new();
    let mut edits = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Restore(key, Some(previous)) => {
                facts.insert(key, previous);
            }
            Step::Restore(key, None) => {
                facts.remove(&key);
            }
            Step::Enter(block) => {
                if let Some((predicate, truth)) = incoming(f, cfg, block)
                    && let Some(test) = comparison(f, predicate)
                    && let Some(key) = Relation::of(f, test)
                {
                    steps.push(Step::Restore(key, facts.insert(key, truth)));
                }
                for inst in f.layout.block_insts(block) {
                    if let Some(key) = Relation::of(f, inst)
                        && let Some(&truth) = facts.get(&key)
                    {
                        edits.push((inst, truth));
                    }
                }
                steps.extend(dom.children(block).map(Step::Enter));
            }
        }
    }
    replace_proved(f, &edits)
}

pub(super) fn replace_proved(f: &mut Function, edits: &[(Inst, bool)]) -> usize {
    // The final backend can still emit a branch on a constant widened
    // Boolean. Redirect only consumers of the comparisons proved above.
    let decided = edits.iter().copied().collect::<BTreeMap<_, _>>();
    let branches = f
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = f.layout.last_inst(block)?;
            let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
                return None;
            };
            let truth = *decided.get(&comparison(f, arg)?)?;
            let selected = blocks[usize::from(!truth)];
            Some((
                inst,
                selected.block(&f.dfg.value_lists),
                selected.args(&f.dfg.value_lists).collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    for &(inst, truth) in edits {
        let ty = f.dfg.value_type(f.dfg.first_result(inst));
        f.dfg.replace(inst).iconst(ty, i64::from(truth));
    }
    for (inst, target, args) in branches {
        f.dfg.replace(inst).jump(target, &args);
    }
    edits.len()
}

#[cfg(test)]
mod tests;
