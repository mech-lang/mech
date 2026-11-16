//! An admitted positive-capacity backing result has an owner. Transfer that
//! fact through exact SSA arguments while retaining all reference-count tests.
use super::inlining::module_callee;
use crate::native_artifact::program::codegen::cfg_cleanup::{
    incoming_block_arguments, positive_word,
};
use cranelift_codegen::{
    Context,
    ir::{
        Block, Function, InstructionData as Data, Opcode, Value, ValueDef, condcodes::IntCC, types,
    },
};
use cranelift_module::{Linkage, Module};
use std::collections::{BTreeMap, BTreeSet};

fn positive_literal(f: &Function, value: Value) -> bool {
    let ValueDef::Result(inst, 0) = f.dfg.value_def(f.dfg.resolve_aliases(value)) else {
        return false;
    };
    f.dfg.value_type(value) == types::I64
        && matches!(f.dfg.insts[inst],
            Data::UnaryImm { opcode: Opcode::Iconst, imm } if imm.bits() != 0)
}

fn admitted_backings<M: Module>(
    module: &M,
    context: &Context,
    ranges: bool,
) -> BTreeMap<Value, Block> {
    let f = &context.func;
    let mut results = BTreeMap::new();
    let positive =
        |value| positive_literal(f, value) || (ranges && positive_word(f, &context.cfg, value));
    for block in f
        .layout
        .blocks()
        .filter(|&block| context.domtree.is_reachable(block))
    {
        let Some(branch) = f.layout.last_inst(block) else {
            continue;
        };
        let Data::Brif {
            arg,
            blocks: [yes, no],
            ..
        } = f.dfg.insts[branch]
        else {
            continue;
        };
        let success = yes.block(&f.dfg.value_lists);
        if success == no.block(&f.dfg.value_lists)
            || !context
                .cfg
                .pred_iter(success)
                .map(|pred| pred.inst)
                .eq([branch])
        {
            continue;
        }
        let ValueDef::Result(call, 0) = f.dfg.value_def(f.dfg.resolve_aliases(arg)) else {
            continue;
        };
        let Data::Call {
            opcode: Opcode::Call,
            func_ref,
            ..
        } = f.dfg.insts[call]
        else {
            continue;
        };
        let Some(id) = module_callee(f, func_ref) else {
            continue;
        };
        let callee = module.declarations().get_function_decl(id);
        if callee.linkage != Linkage::Import {
            continue;
        }
        let nonempty = match (callee.name.as_deref(), f.dfg.inst_args(call)) {
            (Some("mech_compact_allocate_backing"), [_, capacity, stride, _, _]) => {
                positive(*capacity) && positive(*stride)
            }
            (Some("mech_compact_writable_scalar_backing"), [_, _, _, capacity, _, minimum, _]) => {
                positive(*capacity) || positive(*minimum)
            }
            _ => false,
        };
        let [_, owner] = f.dfg.inst_results(call) else {
            continue;
        };
        if nonempty && f.dfg.value_type(*owner) == types::I64 {
            results.insert(*owner, success);
        }
    }
    results
}

type Use = (Value, Block);

fn nonzero(context: &Context, seeds: &BTreeMap<Value, Block>, value: Value, at: Block) -> bool {
    let f = &context.func;
    let at = (f.dfg.resolve_aliases(value), at);
    let mut pending = vec![at];
    let mut seen = BTreeSet::new();
    let mut proved = Vec::<Use>::new();
    let mut users = BTreeMap::<Use, Vec<Use>>::new();
    while let Some(point @ (value, at)) = pending.pop() {
        if !seen.insert(point) {
            continue;
        }
        if seen.len() > 4096 {
            return false;
        }
        if seeds
            .get(&value)
            .is_some_and(|&success| context.domtree.block_dominates(success, at))
        {
            proved.push(point);
            continue;
        }
        let ValueDef::Param(block, ordinal) = f.dfg.value_def(value) else {
            return false;
        };
        let mut incoming = false;
        for (pred, arg) in incoming_block_arguments(f, &context.cfg, block, ordinal)
            .filter(|(pred, _)| context.domtree.is_reachable(*pred))
        {
            let Some(arg) = arg else { return false };
            let arg = f.dfg.resolve_aliases(arg);
            // The call may transport the owner directly on its admission
            // edge, before the predecessor alone would prove success.
            let from = (
                arg,
                if seeds.get(&arg) == Some(&block) {
                    block
                } else {
                    pred
                },
            );
            users.entry(from).or_default().push(point);
            pending.push(from);
            incoming = true;
        }
        if !incoming {
            return false;
        }
    }
    // Every closed copy cycle must have a seed on an actual entry edge.
    let mut initialized = BTreeSet::new();
    while let Some(point) = proved.pop() {
        if initialized.insert(point)
            && let Some(next) = users.get(&point)
        {
            proved.extend(next);
        }
    }
    initialized.len() == seen.len()
}

fn tested_word(f: &Function, test: Value) -> Option<(Value, bool)> {
    let test = f.dfg.resolve_aliases(test);
    if f.dfg.value_type(test) == types::I64 {
        return Some((test, true));
    }
    let ValueDef::Result(inst, 0) = f.dfg.value_def(test) else {
        return None;
    };
    let Data::IntCompareImm { cond, arg, imm, .. } = f.dfg.insts[inst] else {
        return None;
    };
    (imm.bits() == 0 && matches!(cond, IntCC::Equal | IntCC::NotEqual))
        .then_some((arg, cond == IntCC::NotEqual))
}

#[derive(Clone, Copy)]
pub(super) enum Scope {
    All,
    Direct,
    DirectRanges,
    Joins,
}

pub(super) fn fold<M: Module>(module: &M, context: &mut Context, scope: Scope) -> (usize, usize) {
    context.flowgraph();
    let seeds = admitted_backings(module, context, matches!(scope, Scope::DirectRanges));
    let f = &context.func;
    let mut proved = Vec::new();
    if !seeds.is_empty() {
        for block in f
            .layout
            .blocks()
            .filter(|&block| context.domtree.is_reachable(block))
        {
            let Some(inst) = f.layout.last_inst(block) else {
                continue;
            };
            let Data::Brif { arg, blocks, .. } = f.dfg.insts[inst] else {
                continue;
            };
            let Some((value, present)) = tested_word(f, arg) else {
                continue;
            };
            let direct = seeds.contains_key(&f.dfg.resolve_aliases(value));
            if matches!(scope, Scope::Direct | Scope::DirectRanges) && !direct
                || matches!(scope, Scope::Joins) && direct
            {
                continue;
            }
            if nonzero(context, &seeds, value, block) {
                proved.push((inst, blocks[usize::from(!present)]));
            }
        }
    }
    for &(inst, destination) in &proved {
        context.func.dfg.insts[inst] = Data::Jump {
            opcode: Opcode::Jump,
            destination,
        };
    }
    (seeds.len(), proved.len())
}

#[cfg(all(test, target_arch = "aarch64", target_os = "macos"))]
mod tests;
