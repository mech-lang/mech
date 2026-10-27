//! Fold literal native branches after data optimization, then retire unreachable
//! blocks and their phi inputs before register allocation. No runtime predicate,
//! source operation, failure or work debit is assumed away.
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::CfgCleanup;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::backend;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{Function, InstBuilder, InstructionData, Opcode, Value, ValueDef};
use cranelift_codegen::isa::TargetIsa;

const MAX_ROUNDS: usize = 8;

fn counts(function: &Function) -> (usize, usize) {
    (
        function.layout.blocks().count(),
        function
            .layout
            .blocks()
            .map(|block| function.layout.block_insts(block).count())
            .sum(),
    )
}
fn constant_condition(function: &Function, value: Value) -> Option<bool> {
    let value = function.dfg.resolve_aliases(value);
    let ty = function.dfg.value_type(value);
    if !ty.is_int() || ty.bits() > 64 {
        return None;
    }
    let ValueDef::Result(inst, 0) = function.dfg.value_def(value) else {
        return None;
    };
    let InstructionData::UnaryImm {
        opcode: Opcode::Iconst,
        imm,
        ..
    } = function.dfg.insts[inst]
    else {
        return None;
    };
    // Iconst's immediate is sign extended; branch truth uses only the actual
    // integer type's low bits, including noncanonical nonzero boolean words.
    let mask = u64::MAX >> (64 - ty.bits());
    Some(imm.bits() as u64 & mask != 0)
}
fn fold(function: &mut Function) -> usize {
    let changes = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else {
                return None;
            };
            let selected = &blocks[usize::from(!constant_condition(function, *arg)?)];
            Some((
                inst,
                selected.block(&function.dfg.value_lists),
                selected.args(&function.dfg.value_lists).collect::<Vec<_>>(),
            ))
        })
        .collect::<Vec<_>>();
    for (inst, target, args) in &changes {
        function.dfg.replace(*inst).jump(*target, args);
    }
    changes.len()
}

pub(in crate::native_artifact) fn optimize(
    controls: &CompilationControls,
    context: &mut Context,
    isa: &dyn TargetIsa,
) -> Result<CfgCleanup, NativeCompileError> {
    let (before_blocks, before_instructions) = counts(&context.func);

    let enabled = !!controls.enabled(OptimizationPass::CfgCleanup);
    let mut report = CfgCleanup {
        enabled,
        rounds: 0,
        constant_branches_removed: 0,
        before_blocks,
        before_instructions,
        after_blocks: before_blocks,
        after_instructions: before_instructions,
        fixed_point: !enabled,
        pre_codegen_data_passes: 0,
    };
    if !enabled {
        return Ok(report);
    }
    // Retire literal source specializations BEFORE data optimization. Running
    // the egraph repeatedly here and once again in define_function can change
    // placement/rematerialization and increase register pressure. Leave exactly
    // one ordinary data/speed pass to the final backend compilation.
    for _ in 0..MAX_ROUNDS {
        context.compute_cfg();
        context.compute_domtree();
        context.eliminate_unreachable_code(isa).map_err(backend)?;
        context.remove_constant_phis(isa).map_err(backend)?;
        report.rounds += 1;
        let removed = fold(&mut context.func);
        report.constant_branches_removed += removed;
        if removed == 0 {
            report.fixed_point = true;
            break;
        }
    }
    context.compute_cfg();
    context.compute_domtree();
    context.eliminate_unreachable_code(isa).map_err(backend)?;
    context.remove_constant_phis(isa).map_err(backend)?;
    let (blocks, instructions) = counts(&context.func);
    report.after_blocks = blocks;
    report.after_instructions = instructions;

    Ok(report)
}

#[cfg(test)]
mod tests;
