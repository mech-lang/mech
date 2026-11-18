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
