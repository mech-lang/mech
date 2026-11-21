//! Reuse a caller-checked lower bound for the same immutable entry length.
use super::window_bounds::comparison;
use super::word_ranges::literal;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Function, InstBuilder, InstructionData as Data, Value, types};
use std::collections::BTreeMap;

pub(in crate::native_artifact) fn fold(f: &mut Function, floors: &BTreeMap<Value, u64>) -> usize {
    let mut changes = Vec::new();
    for block in f.layout.blocks() {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
            continue;
        };
        let Some(test) = comparison(f, arg) else {
            continue;
        };
        let (word, bound, condition) = match f.dfg.insts[test] {
            Data::IntCompare {
                cond, args: [a, b], ..
            } => {
                if let Some(bound) = literal(f, b) {
                    (a, bound, cond)
                } else if let Some(bound) = literal(f, a) {
                    (b, bound, cond.swap_args())
                } else {
                    continue;
                }
            }
            Data::IntCompareImm { cond, arg, imm, .. } => (arg, imm.bits() as u64, cond),
            _ => continue,
        };
        let word = f.dfg.resolve_aliases(word);
        if f.dfg.value_type(word) != types::I64 {
            continue;
        }
        let Some(&floor) = floors.get(&word) else {
            continue;
        };
        if !match condition {
            IntCC::UnsignedGreaterThanOrEqual => floor >= bound,
            IntCC::UnsignedGreaterThan => floor > bound,
            _ => false,
        } {
            continue;
        }
        let call = blocks[0];
        changes.push((
            inst,
            call.block(&f.dfg.value_lists),
            call.args(&f.dfg.value_lists).collect::<Vec<_>>(),
        ));
    }
    for (inst, dst, args) in &changes {
        f.dfg.replace(*inst).jump(*dst, args);
    }
    changes.len()
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;

// Reuse an admitted current length for an independently bounded scalar index.
// The private caller supplies stable view geometry. Pure value ranges are
// independent of the selected route; unknown or wrapping ranges decline.
pub(in crate::native_artifact) fn fold_index_ranges(
    f: &mut Function,
    floors: &BTreeMap<Value, u64>,
) -> (usize, usize) {
    use super::word_ranges::Facts;
    let literals = fold(f, floors);
    let mut ranges = Facts::new(f);
    let mut changes = Vec::new();
    for block in f.layout.blocks() {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
            continue;
        };
        let Some(test) = comparison(f, arg) else {
            continue;
        };
        let Data::IntCompare {
            cond, args: [a, b], ..
        } = f.dfg.insts[test]
        else {
            continue;
        };
        for (index, length, condition) in [(a, b, cond), (b, a, cond.swap_args())] {
            let length = f.dfg.resolve_aliases(length);
            if f.dfg.value_type(index) != types::I64 || f.dfg.value_type(length) != types::I64 {
                continue;
            }
            let Some(&floor) = floors.get(&length) else {
                continue;
            };
            let (base, offset) = super::word_ranges::constant_offset(f, index);
            let bounded = if offset == 0 { base } else { index };
            let Some(range) = ranges.value_range(bounded) else {
                continue;
            };
            let edge = match condition {
                IntCC::UnsignedLessThan if range.1 < floor => 0,
                IntCC::UnsignedLessThanOrEqual if range.1 <= floor => 0,
                IntCC::UnsignedGreaterThan if range.1 <= floor => 1,
                IntCC::UnsignedGreaterThanOrEqual if range.1 < floor => 1,
                _ => continue,
            };
            let call = blocks[edge];
            changes.push((
                inst,
                call.block(&f.dfg.value_lists),
                call.args(&f.dfg.value_lists).collect::<Vec<_>>(),
            ));
            break;
        }
    }
    drop(ranges);
    for (inst, target, args) in &changes {
        f.dfg.replace(*inst).jump(*target, args);
    }
    (literals, changes.len())
}
