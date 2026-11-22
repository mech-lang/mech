//! Keep pure biased prefetch bases after egraph rematerialization.
use crate::cursor::{Cursor, FuncCursor};
use crate::dominator_tree::DominatorTree;
use crate::flowgraph::ControlFlowGraph;
use crate::ir::{
    Function, Inst, InstBuilder, InstructionData as Data, Opcode, Value, ValueDef, types,
};
use alloc::collections::{BTreeMap, BTreeSet};
use alloc::vec::Vec;

const MAX_BASE_CANDIDATES: usize = 64;

fn constant(f: &Function, value: Value) -> Option<i64> {
    let value = f.dfg.resolve_aliases(value);
    let inst = f.dfg.value_def(value).inst()?;
    match f.dfg.insts[inst] {
        Data::UnaryImm {
            opcode: Opcode::Iconst,
            imm,
        } => Some(imm.bits()),
        _ => None,
    }
}

fn biased_base(f: &Function, inst: Inst) -> Option<(Value, i64)> {
    let (base, bias) = match f.dfg.insts[inst] {
        Data::BinaryImm64 {
            opcode: Opcode::IaddImm,
            arg,
            imm,
        } => (arg, imm.bits()),
        Data::Binary {
            opcode: Opcode::Iadd,
            args: [a, b],
        } => {
            if let Some(n) = constant(f, b) {
                (a, n)
            } else {
                (b, constant(f, a)?)
            }
        }
        _ => return None,
    };
    (f.dfg.value_type(base) == types::I64).then(|| (f.dfg.resolve_aliases(base), bias))
}

fn replace_base(f: &mut Function, inst: Inst, replacement: Value) {
    let result = f.dfg.first_result(inst);
    f.dfg.detach_inst_results(inst);
    f.dfg.change_to_alias(result, replacement);
    f.layout.remove_inst(inst);
}

pub(crate) fn hoist(f: &mut Function, reuse_dominating_bases: bool) -> usize {
    let mut available = BTreeMap::<_, Vec<Inst>>::new();
    let mut candidates = BTreeSet::new();
    for block in f.layout.blocks() {
        for inst in f.layout.block_insts(block) {
            if let Some(key) = biased_base(f, inst) {
                let group = available.entry(key).or_default();
                if group.len() < MAX_BASE_CANDIDATES {
                    group.push(inst);
                }
            }
            let Data::Unary {
                opcode: Opcode::PrefetchWrite,
                arg,
            } = f.dfg.insts[inst]
            else {
                continue;
            };
            let pointer = f.dfg.resolve_aliases(arg);
            if f.dfg.value_type(pointer) != types::I64 {
                continue;
            }
            let Some(pointer_inst) = f.dfg.value_def(pointer).inst() else {
                continue;
            };
            let Data::Binary {
                opcode: Opcode::Iadd,
                args,
            } = f.dfg.insts[pointer_inst]
            else {
                continue;
            };
            for value in args {
                if let Some(base) = f.dfg.value_def(f.dfg.resolve_aliases(value)).inst()
                    && biased_base(f, base).is_some()
                {
                    candidates.insert(base);
                }
            }
        }
    }
    if candidates.is_empty() {
        return 0;
    }
    let cfg = ControlFlowGraph::with_function(f);
    let dom = DominatorTree::with_function(f, &cfg);
    let mut changed = 0;
    for inst in candidates {
        let Some(block) = f.layout.inst_block(inst) else {
            continue;
        };
        if !dom.is_reachable(block) {
            continue;
        }
        let Some(key @ (base, bias)) = biased_base(f, inst) else {
            continue;
        };
        let prior = available
            .get(&key)
            .into_iter()
            .flatten()
            .find(|&&prior| {
                prior != inst
                    && f.layout.inst_block(prior).is_some()
                    && dom.dominates(prior, inst, &f.layout)
            })
            .copied();
        let replacement = if let Some(prior) = prior {
            f.dfg.first_result(prior)
        } else {
            let (owner, anchor) = match f.dfg.value_def(base) {
                ValueDef::Param(owner, _) if dom.dominates(owner, inst, &f.layout) => {
                    (owner, f.layout.first_inst(owner))
                }
                ValueDef::Result(def, _) if dom.dominates(def, inst, &f.layout) => {
                    let Some(owner) = f.layout.inst_block(def) else {
                        continue;
                    };
                    (owner, f.layout.next_inst(def))
                }
                _ => continue,
            };
            // A value defined in this same block gains nothing from hoisting.
            // In particular, this keeps a second application stable.
            if owner == block {
                continue;
            }
            let Some(anchor) = anchor else {
                continue;
            };
            let mut cursor = FuncCursor::new(f);
            cursor.goto_inst(anchor);
            // Legalization has already run. Emit the legal binary form, not
            // iadd_imm, and do not depend on a late constant's dominance.
            let n = cursor.ins().iconst(types::I64, bias);
            let value = cursor.ins().iadd(base, n);
            let definition = cursor.func.dfg.value_def(value).inst().unwrap();
            let group = available.entry(key).or_default();
            if group.len() == MAX_BASE_CANDIDATES {
                group.pop();
            }
            group.insert(0, definition);
            value
        };
        // Only wrapping I64 addition moves. It cannot trap or access memory.
        // The base remains at its original definition; every existing shared
        // use is dominated. Hints, guards, offsets and effects do not move.
        replace_base(f, inst, replacement);
        changed += 1;
        if reuse_dominating_bases {
            let definition = f.dfg.value_def(replacement).inst().unwrap();
            for &duplicate in &available[&key] {
                // Equal wrapping additions can share the base already used by
                // prefetch when that definition dominates the duplicate.
                // Consumers and their memory ordering stay in place.
                if duplicate != definition
                    && f.layout.inst_block(duplicate).is_some()
                    && dom.dominates(definition, duplicate, &f.layout)
                {
                    replace_base(f, duplicate, replacement);
                    changed += 1;
                }
            }
        }
    }
    changed
}
