//! Reuse a caller-admitted immutable span for a unit-stride loop cursor.
//! A proof miss changes neither the graph nor its entry contract. The private
//! caller must check begin <= end <= length before any phase effects.
mod induction;
use super::bounded_byte_histogram::{fixed_word, offset};
use super::window_bounds::{comparison, sole_predecessor};
use cranelift_codegen::Context;
use cranelift_codegen::ir::condcodes::{CondCode, IntCC};
use cranelift_codegen::ir::{Inst, InstBuilder, InstructionData as Data, Value, ValueDef, types};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
pub(in crate::native_artifact) struct Admission {
    pub begin_word: usize,
    pub end_word: usize,
    pub length_word: usize,
    pub checks_removed: usize,
}

// Only exact entry packet lengths of readonly byte views are supplied here.
// The owner/current-view contract remains the caller's responsibility.
pub(in crate::native_artifact) fn fold(
    ctx: &mut Context,
    loads: &BTreeMap<Value, (Inst, usize)>,
    lengths: &BTreeSet<usize>,
) -> Vec<Admission> {
    let f = &ctx.func;
    if lengths.is_empty()
        || f.layout.blocks().count() > 4096
        || f.layout
            .blocks()
            .flat_map(|b| f.layout.block_insts(b))
            .count()
            > 32768
    {
        return Vec::new();
    }
    ctx.compute_cfg();
    ctx.compute_domtree();
    let f = &ctx.func;
    let mut changes = BTreeMap::new();
    let mut admissions = Vec::new();
    let mut attempts = 0;
    for block in f.layout.blocks() {
        let Some(branch) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif { arg, blocks, .. } = f.dfg.insts[branch] else {
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
        let edge = match cond {
            IntCC::Equal => 1,
            IntCC::NotEqual => 0,
            _ => continue,
        };
        let body = blocks[edge].block(&f.dfg.value_lists);
        if sole_predecessor(&ctx.cfg, body).map(|(_, inst)| inst) != Some(branch) {
            continue;
        }
        for (cursor, end) in [(a, b), (b, a)] {
            let cursor = f.dfg.resolve_aliases(cursor);
            if !matches!(f.dfg.value_def(cursor), ValueDef::Param(..))
                || f.dfg.value_type(cursor) != types::I64
            {
                continue;
            }
            let Some(end_word) = fixed_word(f, &ctx.cfg, loads, end) else {
                continue;
            };
            attempts += 1;
            if attempts > 64 {
                break;
            }
            let Some(begin_word) = induction::prove(ctx, loads, cursor, body) else {
                continue;
            };
            let mut found = BTreeMap::<usize, Vec<_>>::new();
            for owner in f.layout.blocks() {
                if !ctx.domtree.block_dominates(body, owner) {
                    continue;
                }
                for inst in f.layout.block_insts(owner) {
                    let Data::IntCompare {
                        cond,
                        args: [left, right],
                        ..
                    } = f.dfg.insts[inst]
                    else {
                        continue;
                    };
                    if changes.contains_key(&inst) {
                        continue;
                    }
                    for (index, length, condition) in
                        [(left, right, cond), (right, left, cond.swap_args())]
                    {
                        let Some(length_word) = fixed_word(f, &ctx.cfg, loads, length) else {
                            continue;
                        };
                        if !lengths.contains(&length_word)
                            || f.dfg.value_type(index) != types::I64
                            || offset(f, &ctx.cfg, index, cursor, &mut BTreeSet::new(), &mut 512)
                                != Some(0)
                        {
                            continue;
                        }
                        let value = match condition {
                            IntCC::UnsignedLessThan
                            | IntCC::UnsignedLessThanOrEqual
                            | IntCC::NotEqual => 1,
                            IntCC::UnsignedGreaterThan
                            | IntCC::UnsignedGreaterThanOrEqual
                            | IntCC::Equal => 0,
                            _ => continue,
                        };
                        found.entry(length_word).or_default().push((inst, value));
                        break;
                    }
                }
            }
            for (length_word, tests) in found {
                admissions.push(Admission {
                    begin_word,
                    end_word,
                    length_word,
                    checks_removed: tests.len(),
                });
                changes.extend(tests);
            }
        }
    }
    for (inst, value) in changes {
        let ty = ctx.func.dfg.value_type(ctx.func.dfg.inst_results(inst)[0]);
        ctx.func.dfg.replace(inst).iconst(ty, value);
    }
    admissions
}

#[cfg(all(test, feature = "jit", target_arch = "aarch64"))]
mod tests;
