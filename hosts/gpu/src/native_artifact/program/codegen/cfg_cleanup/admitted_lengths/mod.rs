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
mod guarded_tests;
#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;

// Reuse an admitted current length for an independently bounded scalar index.
// The private caller supplies stable view geometry. Pure value ranges are
// independent of the selected route; unknown or wrapping ranges decline.
// Callers retire newly unreachable blocks before recomputing dominance.
pub(in crate::native_artifact) fn fold_index_ranges(
    f: &mut Function,
    floors: &BTreeMap<Value, u64>,
) -> (usize, usize) {
    use super::word_ranges::Facts;
    let literals = fold(f, floors);
    let cfg = cranelift_codegen::flowgraph::ControlFlowGraph::with_function(f);
    let mut ranges = Facts::across_edges(f, &cfg, None);
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
            let Some(maximum) = index_maximum(f, &mut ranges, index) else {
                continue;
            };
            let edge = match condition {
                IntCC::UnsignedLessThan if maximum < floor => 0,
                IntCC::UnsignedLessThanOrEqual if maximum <= floor => 0,
                IntCC::UnsignedGreaterThan if maximum <= floor => 1,
                IntCC::UnsignedGreaterThanOrEqual if maximum < floor => 1,
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

// A zero-net add/subtract chain preserves its exact modular value. A nonzero
// offset only inherits a range when its complete maximum cannot wrap.
fn index_maximum(
    f: &Function,
    facts: &mut super::word_ranges::Facts<'_>,
    index: Value,
) -> Option<u64> {
    let (base, offset) = super::word_ranges::constant_offset(f, index);
    facts
        .value_range(base)
        .and_then(|range| range.1.checked_add(offset))
        .or_else(|| facts.value_range(index).map(|range| range.1))
}

// The caller must guard every returned floor before entering the optimized
// body. Only exact fixed packet lengths supplied by that caller participate.
// No path condition or source integer type is assumed here.
pub(in crate::native_artifact) fn required_index_floors(
    f: &Function,
    lengths: &std::collections::BTreeSet<Value>,
) -> BTreeMap<Value, u64> {
    // Bound speculative work even for unusually large generated functions.
    if f.layout.blocks().take(4097).count() > 4096
        || f.layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .take(32769)
            .count()
            > 32768
    {
        return BTreeMap::new();
    }
    let cfg = cranelift_codegen::flowgraph::ControlFlowGraph::with_function(f);
    let mut facts = super::word_ranges::Facts::across_edges(f, &cfg, None);
    let mut floors = BTreeMap::<Value, u64>::new();
    for block in f.layout.blocks() {
        let Some(inst) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, .. } = f.dfg.insts[inst] else {
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
            if !lengths.contains(&length) || f.dfg.value_type(index) != types::I64 {
                continue;
            }
            let Some(maximum) = index_maximum(f, &mut facts, index) else {
                continue;
            };
            let minimum = match condition {
                IntCC::UnsignedLessThan | IntCC::UnsignedGreaterThanOrEqual => {
                    maximum.checked_add(1)
                }
                IntCC::UnsignedLessThanOrEqual | IntCC::UnsignedGreaterThan => Some(maximum),
                _ => None,
            };
            // Limit new entry requirements to small fixed tables. A missing
            // guard must not accidentally demand a large input or workspace.
            if let Some(minimum @ 1..=64) = minimum {
                floors
                    .entry(length)
                    .and_modify(|floor| *floor = (*floor).max(minimum))
                    .or_insert(minimum);
            }
        }
    }
    floors
}
