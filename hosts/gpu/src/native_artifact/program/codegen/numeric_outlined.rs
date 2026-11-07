//! Isolate a complete numeric phase in one synchronous native function. The
//! source body, current loan, private views and cold operations stay shared.
use super::{hooks::Hooks, readonly_numeric};
use crate::native_artifact::codegen::control::{CfgCleanup, CodegenReport, plan::CodePlan};
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::{backend, compiled_stack_bytes, limit};
use crate::native_artifact::program::prepare::tail::Layout;
use crate::native_artifact::{NativeCompileError, NativeCompileLimits};
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, Block, FuncRef, InstBuilder, Value, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

// Existing boundary funding must cover the caller's gate/call/dispatch and the
// callee's five return-stub instructions. No source price is increased.
pub(super) const ADDITIONAL_TEMPLATE_BOUND: usize = 64;

#[derive(Clone, Copy)]
pub(in crate::native_artifact) enum Entry {
    Inline,
    Call(FuncRef),
    Body,
}

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct Report {
    header: usize,
    symbol: String,
    stack_bytes: u64,
    code_bytes: u64,
    additional_template_bound: usize,
    cleanup: CfgCleanup,
}

pub(in crate::native_artifact) fn define_functions<M: Module>(
    controls: &CompilationControls,
    module: &mut M,
    entry_symbol: &str,
    code: CodePlan<'_>,
    closed: &Layout,
    limits: NativeCompileLimits,
    report: &mut CodegenReport,
) -> Result<Vec<(usize, FuncId, u64, u64)>, NativeCompileError> {
    if !controls.enabled(OptimizationPass::ScalarLoops)
        || !controls.enabled(OptimizationPass::LoopOutlining)
    {
        return Ok(Vec::new());
    }
    let mut functions = Vec::new();
    for (id, region) in closed.regions.iter().enumerate() {
        let site = region.tail.header() as usize;
        let Some(_) = region.numeric.as_ref().filter(|proof| {
            proof.exits.is_empty()
                && proof
                    .phase
                    .as_ref()
                    .is_some_and(|phase| phase.loops.len() > 1)
        }) else {
            continue;
        };
        // Secondary entries belong to a partially native ancestor's fallback.
        // Keep those inline; only the primary complete computation moves.
        if closed.sites[site] != Some(id) || region.cold_resume().is_some() {
            continue;
        }
        let symbol = format!("{entry_symbol}_numeric_phase_{site}");
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 7];
        // Original status and whether the existing source exit was reached.
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let mut context = module.make_context();
        context.func.signature = signature.clone();
        let mut fb = FunctionBuilderContext::new();
        let emitted;
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args: [Value; 7] = b.block_params(entry).try_into().unwrap();
            let zero = b.ins().iconst(types::I64, 0);
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            let done = b.create_block();
            let hooks = Hooks::from_numeric_outline(&mut b, module, args);
            let mut ordinary = vec![None; code.blocks().len()];
            // All cold source-continuation exits were excluded above. Bounds,
            // arithmetic, first change and root-funding failure remain inside
            // the original emitter and use the original current arguments.
            ordinary[region.tail.exit() as usize] = Some(done);
            emitted = readonly_numeric::emit_entry(
                controls,
                &mut b,
                args[0],
                args[1],
                code,
                closed,
                site,
                &hooks,
                &ordinary,
                error,
                Entry::Body,
                report,
            )?;
            b.ins().return_(&[zero, zero]);
            b.switch_to_block(done);
            let yes = b.ins().iconst(types::I64, 1);
            b.ins().return_(&[zero, yes]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status, zero]);
            b.seal_all_blocks();
            b.finalize();
        }
        if !emitted {
            continue;
        }
        // Do not leave an undefined module symbol when the unchanged finite
        // boundary envelope requires the original inline implementation.
        let function = module
            .declare_function(&symbol, Linkage::Local, &signature)
            .map_err(backend)?;
        let cleanup = super::cfg_cleanup::optimize(controls, &mut context, module.isa())?;
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("outlined numeric phase has no compiled code"))?;
        let stack = compiled_stack_bytes(&compiled.buffer)?;
        let bytes = u64::from(compiled.code_info().total_size);
        limit("Outlined numeric stack", stack, limits.max_stack_bytes)?;
        limit("Outlined numeric code", bytes, limits.max_code_bytes)?;
        report.numeric_outlines.push(Report {
            header: site,
            symbol,
            stack_bytes: stack,
            code_bytes: bytes,
            additional_template_bound: ADDITIONAL_TEMPLATE_BOUND,
            cleanup,
        });
        functions.push((site, function, stack, bytes));
    }
    Ok(functions)
}

pub(super) fn call(
    b: &mut FunctionBuilder<'_>,
    function: FuncRef,
    frame: Value,
    locals: Value,
    hooks: &Hooks,
    done: Block,
    ordinary: Block,
    error: Block,
    gate_work: usize,
) -> Result<(), NativeCompileError> {
    let before = b.func.dfg.num_insts();
    let call = b
        .ins()
        .call(function, &hooks.numeric_outline_arguments(frame, locals));
    let status = b.inst_results(call)[0];
    let completed = b.inst_results(call)[1];
    let good = b.create_block();
    let valid = b.ins().icmp_imm(IntCC::Equal, status, 0);
    b.ins().brif(valid, good, &[], error, &[status.into()]);
    b.switch_to_block(good);
    b.ins().brif(completed, done, &[], ordinary, &[]);
    if gate_work + b.func.dfg.num_insts() - before + 5 > ADDITIONAL_TEMPLATE_BOUND {
        return Err(backend(
            "numeric outline exceeds the unchanged boundary envelope",
        ));
    }
    b.switch_to_block(ordinary);
    Ok(())
}
