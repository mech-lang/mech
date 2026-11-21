//! Admit one native word of input, then reuse that lower bound for scalar
//! predicates such as the saturated byte-window limits. The same caller span
//! guard checks the extra floor; no body is cloned and short inputs refuse.
use super::super::word_ranges::literal;
use super::{Admission, fixed_word};
use cranelift_codegen::flowgraph::ControlFlowGraph;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Function, Inst, InstBuilder, InstructionData as Data, Value, types};
use std::collections::{BTreeMap, BTreeSet};

pub(in crate::native_artifact) fn admit(
    f: &mut Function,
    loads: &BTreeMap<Value, (Inst, usize)>,
    spans: &mut [Admission],
) -> usize {
    if spans.is_empty() {
        return 0;
    }
    let lengths = spans
        .iter()
        .map(|span| span.length_word)
        .collect::<BTreeSet<_>>();
    let cfg = ControlFlowGraph::with_function(f);
    let mut changes = Vec::new();
    let mut counts = BTreeMap::<usize, usize>::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            let (word, bound, condition) = match f.dfg.insts[inst] {
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
            if f.dfg.value_type(word) != types::I64 {
                continue;
            }
            let Some(at) = fixed_word(f, &cfg, loads, word).filter(|at| lengths.contains(at))
            else {
                continue;
            };
            let value = match condition {
                IntCC::UnsignedGreaterThanOrEqual if 8 >= bound => 1,
                IntCC::UnsignedGreaterThan if 8 > bound => 1,
                IntCC::UnsignedLessThan if 8 >= bound => 0,
                IntCC::UnsignedLessThanOrEqual | IntCC::Equal if 8 > bound => 0,
                IntCC::NotEqual if 8 > bound => 1,
                _ => continue,
            };
            let ty = f.dfg.value_type(f.dfg.inst_results(inst)[0]);
            changes.push((inst, ty, value));
            *counts.entry(at).or_default() += 1;
        }
    }
    for span in spans {
        if let Some(&count) = counts.get(&span.length_word) {
            span.minimum_length = span.minimum_length.max(8);
            span.checks_removed += count;
        }
    }
    for &(inst, ty, value) in &changes {
        f.dfg.replace(inst).iconst(ty, value);
    }
    changes.len()
}
