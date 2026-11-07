//! Fold native branches proved by immutable integer SSA definitions, then retire
//! unreachable blocks and their phi inputs before register allocation. Source
//! work debits and runtime predicates without a word proof remain unchanged.
mod dead_values;
mod local_phis;
mod preheaders;
mod word_ranges;
use crate::native_artifact::NativeCompileError;
use crate::native_artifact::codegen::control::CfgCleanup;
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::backend;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{Function, InstBuilder, InstructionData};
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
fn fold(function: &mut Function) -> usize {
    let mut facts = word_ranges::Facts::new(function);
    let changes = function
        .layout
        .blocks()
        .filter_map(|block| {
            let inst = function.layout.last_inst(block)?;
            let InstructionData::Brif { arg, blocks, .. } = &function.dfg.insts[inst] else {
                return None;
            };
            let selected = &blocks[usize::from(!facts.condition(*arg)?)];
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
        local_phi_parameters_removed: 0,
        local_phi_preheaders: 0,
        dead_parameters_removed: 0,
        dead_instructions_removed: 0,
        dead_loads_removed: 0,
    };
    if !enabled {
        return Ok(report);
    }
    // Retire proved source specializations BEFORE data optimization. Running
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
    report.local_phi_preheaders =
        preheaders::normalize(&mut context.func, &context.cfg, &context.domtree);
    if report.local_phi_preheaders != 0 {
        context.compute_cfg();
        context.compute_domtree();
    }
    report.local_phi_parameters_removed =
        local_phis::simplify(&mut context.func, &context.cfg, &context.domtree);
    let dead = dead_values::eliminate(&mut context.func);
    report.dead_parameters_removed = dead.parameters;
    report.dead_instructions_removed = dead.instructions;
    report.dead_loads_removed = dead.loads;
    let (blocks, instructions) = counts(&context.func);
    report.after_blocks = blocks;
    report.after_instructions = instructions;

    Ok(report)
}

#[cfg(test)]
mod tests;
