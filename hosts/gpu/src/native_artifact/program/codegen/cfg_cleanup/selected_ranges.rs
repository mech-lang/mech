//! Only a unique selected CFG edge introduces a fact. Facts are scoped to its
//! dominator subtree and name immutable SSA words, never memory or permissions.
use super::word_ranges::{Facts, Range};
use cranelift_codegen::dominator_tree::DominatorTree;
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{
    Block, Function, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef,
};
use std::collections::BTreeMap;

fn mask(f: &Function, value: Value) -> Option<u64> {
    let ty = f.dfg.value_type(value);
    (ty.is_int() && !ty.is_vector() && (1..=64).contains(&ty.bits()))
        .then(|| u64::MAX >> (64 - ty.bits()))
}
fn constant(f: &Function, value: Value) -> Option<u64> {
    let value = f.dfg.resolve_aliases(value);
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
    Some(imm.bits() as u64 & mask(f, value)?)
}
fn constrain(f: &Function, value: Value, range: Range, facts: &mut Vec<(Value, Range)>) {
    let value = f.dfg.resolve_aliases(value);
    facts.push((value, range));
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return;
    };
    // Invert a bounded modular offset only when its whole translated interval
    // fits. Otherwise it can wrap into two intervals and supplies no base fact.
    if let Data::BinaryImm64 {
        opcode: Opcode::IaddImm,
        arg,
        imm,
    } = f.dfg.insts[inst]
    {
        let Some(maximum) = mask(f, value) else {
            return;
        };
        let start = 0u64.wrapping_sub(imm.bits() as u64) & maximum;
        if let (Some(lo), Some(hi)) = (start.checked_add(range.0), start.checked_add(range.1)) {
            if hi <= maximum {
                facts.push((f.dfg.resolve_aliases(arg), Range(lo, hi)));
            }
        }
    }
}
fn selected(
    f: &Function,
    value: Value,
    truth: bool,
    depth: usize,
    known: &BTreeMap<Value, Range>,
    facts: &mut Vec<(Value, Range)>,
) {
    if depth == 0 {
        return;
    }
    let value = f.dfg.resolve_aliases(value);
    let Some(maximum) = mask(f, value) else {
        return;
    };
    facts.push((
        value,
        if truth {
            Range(1, maximum)
        } else {
            Range(0, 0)
        },
    ));
    let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
        return;
    };
    let (cond, word, bound) = match f.dfg.insts[inst] {
        Data::Unary {
            opcode: Opcode::Uextend,
            arg,
        } => return selected(f, arg, truth, depth - 1, known, facts),
        Data::BinaryImm64 {
            opcode: Opcode::BandImm,
            arg,
            imm,
        } if (imm.bits() as u64 & maximum) == 1
            && Facts::selected(f, known).canonical_boolean(arg) =>
        {
            // Direct Boolean lowering exposes a low-bit test. It is an
            // identity only for a separately proved zero-or-one SSA word;
            // an unrestricted odd word must never become the value one.
            return selected(f, arg, truth, depth - 1, known, facts);
        }
        Data::Binary {
            opcode: Opcode::Band,
            args,
        } if truth => {
            selected(f, args[0], true, depth - 1, known, facts);
            return selected(f, args[1], true, depth - 1, known, facts);
        }
        Data::Binary {
            opcode: Opcode::Bor,
            args,
        } if !truth => {
            selected(f, args[0], false, depth - 1, known, facts);
            return selected(f, args[1], false, depth - 1, known, facts);
        }
        Data::IntCompare { cond, args, .. } => {
            if let Some(bound) = constant(f, args[1]) {
                (cond, args[0], bound)
            } else if let Some(bound) = constant(f, args[0]) {
                (cond.swap_args(), args[1], bound)
            } else {
                return;
            }
        }
        Data::IntCompareImm { cond, arg, imm, .. } => {
            let Some(maximum) = mask(f, arg) else {
                return;
            };
            (cond, arg, imm.bits() as u64 & maximum)
        }
        _ => return,
    };
    let Some(maximum) = mask(f, word) else {
        return;
    };
    let cond = if truth { cond } else { cond.complement() };
    let range = match cond {
        IntCC::Equal => Range(bound, bound),
        IntCC::NotEqual if bound == 0 => Range(1, maximum),
        IntCC::NotEqual if bound == maximum => Range(0, maximum - 1),
        IntCC::UnsignedLessThan if bound > 0 => Range(0, bound - 1),
        IntCC::UnsignedLessThanOrEqual => Range(0, bound),
        IntCC::UnsignedGreaterThan if bound < maximum => Range(bound + 1, maximum),
        IntCC::UnsignedGreaterThanOrEqual => Range(bound, maximum),
        _ => return,
    };
    constrain(f, word, range, facts);
}

fn incoming(
    f: &Function,
    cfg: &ControlFlowGraph,
    block: Block,
    known: &BTreeMap<Value, Range>,
) -> Vec<(Value, Range)> {
    if Some(block) == f.layout.entry_block() {
        return Vec::new();
    }
    let mut predecessors = cfg.pred_iter(block);
    let Some(pred) = predecessors.next() else {
        return Vec::new();
    };
    if predecessors.next().is_some() {
        return Vec::new();
    }
    let Data::Brif { arg, blocks, .. } = &f.dfg.insts[pred.inst] else {
        return Vec::new();
    };
    let yes = blocks[0].block(&f.dfg.value_lists);
    let no = blocks[1].block(&f.dfg.value_lists);
    if yes == no {
        return Vec::new();
    }
    let mut facts = Vec::new();
    selected(f, *arg, block == yes, 16, known, &mut facts);
    facts
}

pub(super) fn fold(
    f: &mut Function,
    cfg: &ControlFlowGraph,
    dom: &DominatorTree,
) -> (usize, usize) {
    if f.layout.blocks().any(|b| {
        f.layout
            .block_insts(b)
            .any(|i| f.dfg.insts[i].exception_table().is_some())
    }) {
        return (0, 0);
    }
    let Some(entry) = f.layout.entry_block() else {
        return (0, 0);
    };
    enum Step {
        Enter(Block),
        Leave(Vec<(Value, Option<Range>)>),
    }
    let mut steps = vec![Step::Enter(entry)];
    let mut known = super::boolean_words::derive(f, cfg);
    let boolean_phi_values = known.len();
    let mut changes = Vec::new();
    while let Some(step) = steps.pop() {
        match step {
            Step::Leave(undo) => {
                for (value, previous) in undo.into_iter().rev() {
                    if let Some(range) = previous {
                        known.insert(value, range);
                    } else {
                        known.remove(&value);
                    }
                }
            }
            Step::Enter(block) => {
                let mut undo = Vec::new();
                for (value, range) in incoming(f, cfg, block, &known) {
                    let current = known.get(&value).copied();
                    let range =
                        current.map_or(range, |a| Range(a.0.max(range.0), a.1.min(range.1)));
                    if range.0 <= range.1 {
                        undo.push((value, known.insert(value, range)));
                    }
                }
                if let Some(inst) = f.layout.last_inst(block) {
                    if let Data::Brif { arg, blocks, .. } = &f.dfg.insts[inst] {
                        if let Some(truth) = Facts::selected(f, &known).condition(*arg) {
                            let target = &blocks[usize::from(!truth)];
                            changes.push((
                                inst,
                                target.block(&f.dfg.value_lists),
                                target.args(&f.dfg.value_lists).collect::<Vec<_>>(),
                            ));
                        }
                    }
                }
                steps.push(Step::Leave(undo));
                steps.extend(dom.children(block).map(Step::Enter));
            }
        }
    }
    for (inst, target, args) in &changes {
        f.dfg.replace(*inst).jump(*target, args);
    }
    (changes.len(), boolean_phi_values)
}
