//! Demand starts at effects and control, not at loop-carried arguments. A
//! phi-only cycle or a dead private load has no observable result by itself.
//! Trapping loads, checked arithmetic, calls, stores and all CFG edges remain.
use cranelift_codegen::ir::{BlockCall, Function, Inst, InstructionData, Opcode, ValueDef};
use std::collections::BTreeMap;

#[derive(Default)]
pub(super) struct Removed {
    pub parameters: usize,
    pub instructions: usize,
    pub loads: usize,
}

fn removable(function: &Function, inst: Inst) -> bool {
    let opcode = function.dfg.insts[inst].opcode();
    if opcode.is_call()
        || opcode.is_branch()
        || opcode.is_terminator()
        || opcode.is_return()
        || opcode.can_trap()
        || opcode.other_side_effects()
        || opcode.can_store()
    {
        return false;
    }
    match function.dfg.insts[inst] {
        InstructionData::Load {
            opcode: Opcode::Load,
            flags,
            ..
        } => flags.notrap(),
        InstructionData::StackLoad { .. } => true,
        _ => !opcode.can_load(),
    }
}

pub(super) fn eliminate(function: &mut Function) -> Removed {
    // Exceptional edge-only results and context operands retain the original
    // lowering until this pass has an explicit contract for those edges.
    if function.layout.blocks().any(|block| {
        function
            .layout
            .block_insts(block)
            .any(|inst| function.dfg.insts[inst].exception_table().is_some())
    }) {
        return Removed::default();
    }
    // Demand resolves each value below. Keep the alias records themselves:
    // later native range proofs also retain producer-derived SSA references.
    // resolve_all_aliases rewrites IR uses but invalidates those records.
    let mut incoming = BTreeMap::new();
    let mut live_values = vec![false; function.dfg.num_values()];
    let mut live_insts = vec![false; function.dfg.num_insts()];
    let mut pending = Vec::new();
    for block in function.layout.blocks() {
        for inst in function.layout.block_insts(block) {
            let data = &function.dfg.insts[inst];
            for call in
                data.branch_destination(&function.dfg.jump_tables, &function.dfg.exception_tables)
            {
                let formals = function
                    .dfg
                    .block_params(call.block(&function.dfg.value_lists));
                let args = call.args(&function.dfg.value_lists).collect::<Vec<_>>();
                if args.len() != formals.len() || args.iter().any(|arg| arg.as_value().is_none()) {
                    return Removed::default();
                }
                for (&formal, arg) in formals.iter().zip(args) {
                    incoming
                        .entry(formal)
                        .or_insert_with(Vec::new)
                        .push(arg.as_value().unwrap());
                }
            }
            if !removable(function, inst) {
                live_insts[inst.as_u32() as usize] = true;
                // Ordinary instruction operands exclude destination arguments.
                // A branch demands its condition; a destination demands only
                // the arguments of its live formals, below.
                pending.extend(function.dfg.inst_args(inst).iter().copied());
            }
        }
    }
    if let Some(entry) = function.layout.entry_block() {
        // The declared native function ABI never changes, even for unused
        // parameters. Interior block signatures are private compiler details.
        pending.extend(function.dfg.block_params(entry).iter().copied());
    }
    while let Some(value) = pending.pop() {
        let value = function.dfg.resolve_aliases(value);
        if core::mem::replace(&mut live_values[value.as_u32() as usize], true) {
            continue;
        }
        match function.dfg.value_def(value) {
            ValueDef::Result(inst, _) => {
                if !core::mem::replace(&mut live_insts[inst.as_u32() as usize], true) {
                    pending.extend(function.dfg.inst_args(inst).iter().copied());
                }
            }
            ValueDef::Param(..) => {
                if let Some(args) = incoming.get(&value) {
                    pending.extend(args);
                }
            }
            ValueDef::Union(..) => return Removed::default(),
        }
    }
    let mut removed = Removed::default();
    let blocks = function
        .layout
        .blocks()
        .filter(|&block| Some(block) != function.layout.entry_block())
        .filter_map(|block| {
            let params = function.dfg.block_params(block);
            params
                .iter()
                .any(|value| !live_values[value.as_u32() as usize])
                .then(|| (block, params.to_vec()))
        })
        .collect::<BTreeMap<_, _>>();
    for block in function.layout.blocks().collect::<Vec<_>>() {
        for inst in function.layout.block_insts(block).collect::<Vec<_>>() {
            if !live_insts[inst.as_u32() as usize] {
                debug_assert!(removable(function, inst));
                removed.loads += usize::from(matches!(
                    function.dfg.insts[inst],
                    InstructionData::Load { .. } | InstructionData::StackLoad { .. }
                ));
                function.layout.remove_inst(inst);
                removed.instructions += 1;
                continue;
            }
            let dfg = &mut function.dfg;
            for call in dfg.insts[inst]
                .branch_destination_mut(&mut dfg.jump_tables, &mut dfg.exception_tables)
            {
                let target = call.block(&dfg.value_lists);
                if let Some(params) = blocks.get(&target) {
                    let args = call.args(&dfg.value_lists).collect::<Vec<_>>();
                    let kept = args.into_iter().zip(params).filter_map(|(arg, formal)| {
                        live_values[formal.as_u32() as usize].then_some(arg)
                    });
                    *call = BlockCall::new(target, kept, &mut dfg.value_lists);
                }
            }
        }
    }
    for params in blocks.values() {
        for &value in params.iter().rev() {
            if !live_values[value.as_u32() as usize] {
                function.dfg.remove_block_param(value);
                removed.parameters += 1;
            }
        }
    }
    removed
}

#[cfg(test)]
mod tests;
