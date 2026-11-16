//! Selected-edge facts over current immutable integer SSA words. Comparison
//! inversion uses the proved current interval, never an assumed Boolean type.
use super::super::masked_integer_comparison;
use super::{Facts, Range};
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Function, InstructionData as Data, Opcode, Value, ValueDef};
use std::collections::BTreeMap;
fn mask(f: &Function, v: Value) -> Option<u64> {
    let ty = f.dfg.value_type(v);
    (ty.is_int() && !ty.is_vector() && (1..=64).contains(&ty.bits()))
        .then(|| u64::MAX >> (64 - ty.bits()))
}
pub(in super::super) fn assume(
    f: &Function,
    value: Value,
    truth: bool,
    known: &mut BTreeMap<Value, Range>,
) -> bool {
    assume_with_undo(f, value, truth, known, &mut Vec::new())
}

pub(in super::super) fn assume_with_undo(
    f: &Function,
    value: Value,
    truth: bool,
    known: &mut BTreeMap<Value, Range>,
    undo: &mut Vec<(Value, Option<Range>)>,
) -> bool {
    assume_inner(f, value, truth, known, undo, &mut 65_536)
}

pub(in super::super) fn assume_bounded(
    f: &Function,
    value: Value,
    truth: bool,
    known: &mut BTreeMap<Value, Range>,
    remaining: &mut usize,
) -> bool {
    assume_inner(f, value, truth, known, &mut Vec::new(), remaining)
}

fn assume_inner(
    f: &Function,
    value: Value,
    truth: bool,
    known: &mut BTreeMap<Value, Range>,
    undo: &mut Vec<(Value, Option<Range>)>,
    remaining: &mut usize,
) -> bool {
    let Some(max) = mask(f, value) else {
        return false;
    };
    let mut pending = vec![(value, if truth { Range(1, max) } else { Range(0, 0) })];
    // Shared Boolean DAGs may enqueue the same selected word many times.
    // Propagate an initially known range once, then revisit only on progress.
    let mut visited = BTreeMap::new();
    while let Some((value, want)) = pending.pop() {
        let Some(next) = remaining.checked_sub(1) else {
            return false;
        };
        *remaining = next;
        let value = f.dfg.resolve_aliases(value);
        let Some(current) = Facts::bound(f, known).value_range(value) else {
            return false;
        };
        let narrowed = Range(current.0.max(want.0), current.1.min(want.1));
        if narrowed.0 > narrowed.1 {
            return false;
        }
        if visited.insert(value, narrowed) == Some(narrowed) {
            continue;
        }
        if known.get(&value).copied() != Some(narrowed) {
            undo.push((value, known.insert(value, narrowed)));
        }
        let truth = if narrowed.1 == 0 {
            false
        } else if narrowed.0 > 0 {
            true
        } else {
            continue;
        };
        let ValueDef::Result(inst, 0) = f.dfg.value_def(value) else {
            continue;
        };
        if let Some(comparison) = masked_integer_comparison(f, &f.dfg.insts[inst], |value| {
            Some(f.dfg.resolve_aliases(value))
        }) {
            pending.push((comparison, narrowed));
            continue;
        }
        let (cond, word, bound) = match f.dfg.insts[inst] {
            Data::Unary {
                opcode: Opcode::Uextend,
                arg,
            } => {
                pending.push((arg, narrowed));
                continue;
            }
            Data::Binary {
                opcode: Opcode::Band,
                args,
            } if truth => {
                for arg in args {
                    if let Some(max) = mask(f, arg) {
                        pending.push((arg, Range(1, max)))
                    }
                }
                continue;
            }
            Data::Binary {
                opcode: Opcode::Bor,
                args,
            } if !truth => {
                for arg in args {
                    pending.push((arg, Range(0, 0)))
                }
                continue;
            }
            Data::IntCompare { cond, args, .. } => {
                let b = Facts::bound(f, known).value_range(args[1]);
                let a = Facts::bound(f, known).value_range(args[0]);
                if let Some(b) = b.filter(|r| r.0 == r.1) {
                    (cond, args[0], b.0)
                } else if let Some(a) = a.filter(|r| r.0 == r.1) {
                    (cond.swap_args(), args[1], a.0)
                } else {
                    continue;
                }
            }
            Data::IntCompareImm { cond, arg, imm, .. } => {
                let Some(max) = mask(f, arg) else { continue };
                (cond, arg, imm.bits() as u64 & max)
            }
            _ => continue,
        };
        let Some(current) = Facts::bound(f, known).value_range(word) else {
            continue;
        };
        let Some(max) = mask(f, word) else { continue };
        let cond = if truth { cond } else { cond.complement() };
        let want = match cond {
            IntCC::Equal => Range(bound, bound),
            IntCC::NotEqual if bound == current.0 && bound < current.1 => {
                Range(bound + 1, current.1)
            }
            IntCC::NotEqual if bound == current.1 && bound > current.0 => {
                Range(current.0, bound - 1)
            }
            IntCC::UnsignedLessThan if bound > 0 => Range(0, bound - 1),
            IntCC::UnsignedLessThanOrEqual => Range(0, bound),
            IntCC::UnsignedGreaterThan if bound < max => Range(bound + 1, max),
            IntCC::UnsignedGreaterThanOrEqual => Range(bound, max),
            _ => continue,
        };
        pending.push((word, want));
    }
    true
}
