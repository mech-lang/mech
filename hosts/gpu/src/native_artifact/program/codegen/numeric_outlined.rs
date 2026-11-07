//! One native function per admitted numeric region. The same emitter owns its
//! body, checked cold bridges and exact ordinary continuation.
use super::*;
use crate::native_artifact::compiler::{compiled_stack_bytes, limit};
use crate::native_artifact::program::prepare::tail::Region;
use crate::native_artifact::{NativeCompileLimits, codegen::options::OptimizationPass};
use cranelift_codegen::ir::{AbiParam, FuncRef};
use cranelift_frontend::{FunctionBuilderContext, Switch};
use cranelift_module::{FuncId, Linkage, Module};

#[derive(serde::Serialize)]
pub(in crate::native_artifact) struct OutlinedReport {
    pub header: usize,
    pub symbol: String,
    pub stack_bytes: u64,
    pub code_bytes: u64,
    pub caller_template_bound: usize,
    pub cleanup: crate::native_artifact::codegen::control::CfgCleanup,
    pub unsigned_guards: usize,
}

fn continuations(region: &Region) -> Vec<usize> {
    let mut sites = vec![region.tail.header() as usize, region.tail.exit() as usize];
    if let Some(tree) = &region.numeric_tree {
        sites.extend(
            tree.nodes
                .iter()
                .skip(1)
                .map(|node| node.tail.header() as usize),
        );
    }
    if let Some(proof) = &region.numeric {
        let mut seen = sites.iter().copied().collect::<BTreeSet<_>>();
        for exit in &proof.exits {
            if seen.insert(exit.to) {
                sites.push(exit.to);
            }
        }
    }
    sites
}

fn caller_bound(region: &Region) -> usize {
    64 + continuations(region).len() * 8
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
    if !controls.enabled(OptimizationPass::NumericOutlining)
        || !controls.enabled(OptimizationPass::ScalarLoops)
    {
        return Ok(Vec::new());
    }
    let mut functions = Vec::new();
    for region in &closed.regions {
        if functions.len() == 256 {
            break;
        }
        let Some(proof) = &region.numeric else {
            continue;
        };
        if region.numeric_subtree() && !controls.enabled(OptimizationPass::NumericTrees) {
            continue;
        }
        let resume = continuations(region);
        let mut signature = module.make_signature();
        signature.params = vec![AbiParam::new(types::I64); 3];
        signature.returns = vec![AbiParam::new(types::I64); 2];
        let mut context = module.make_context();
        context.func.signature = signature.clone();
        let mut fb = FunctionBuilderContext::new();
        let mut local = CodegenReport::default();
        let extra;
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            let hooks = Hooks::load(true, &mut b, module, args[0], args[2], error)?
                .ok_or_else(|| backend("outlined numeric body lost managed hooks"))?;
            // Explicitly include both native call/dispatch and callee loading/
            // return templates in the unchanged independent boundary envelope.
            extra = b.func.dfg.num_insts() + caller_bound(region) + resume.len() * 3 + 3;
            let mut ordinary = vec![None; code.blocks().len()];
            for &site in resume.iter().skip(1) {
                ordinary[site] = Some(b.create_block());
            }
            super::emit_entry(
                controls,
                &mut b,
                args[0],
                args[1],
                args[2],
                None,
                code,
                closed,
                region.tail.header() as usize,
                &hooks,
                &ordinary,
                error,
                &mut local,
            )?;
            let zero = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[zero, zero]);
            for (token, &site) in resume.iter().enumerate().skip(1) {
                b.switch_to_block(ordinary[site].unwrap());
                let zero = b.ins().iconst(types::I64, 0);
                let token = b.ins().iconst(types::I64, token as i64);
                b.ins().return_(&[zero, token]);
            }
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            let zero = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[status, zero]);
            b.seal_all_blocks();
            b.finalize();
        }
        let Some(receipt) = local
            .numeric_write_loops
            .last_mut()
            .or_else(|| local.readonly_numeric_loops.last_mut())
        else {
            continue;
        };
        let Some(boundary) = receipt.fixed_template_instructions.checked_add(extra) else {
            continue;
        };
        if boundary as u64 > proof.boundary_work {
            continue;
        }
        receipt.fixed_template_instructions = boundary;
        if controls.enabled(OptimizationPass::BitCounts) {
            crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
        }
        let mut cleanup =
            crate::native_artifact::program::codegen::cfg_cleanup::optimize_before_unsigned_guards(
                controls,
                &mut context,
                module.isa(),
            )?;
        if controls.enabled(OptimizationPass::BitCounts) {
            crate::native_artifact::codegen::common::bit_counts::optimize(&mut context.func);
        }
        if controls.enabled(OptimizationPass::ScalarSelects) {
            let selected = crate::native_artifact::codegen::common::scalar_selects::optimize(
                &mut context.func,
            );
            report.scalar_selects += selected;
            if selected > 0 {
                context.compute_cfg();
                context.compute_domtree();
            }
        }
        let unsigned_guards = if controls.enabled(OptimizationPass::UnsignedGuardFacts) {
            let bounds = local
                .numeric_write_loops
                .iter()
                .chain(&local.readonly_numeric_loops)
                .flat_map(|r| r.native_extent_bounds.iter().copied())
                .collect::<Vec<_>>();
            crate::native_artifact::codegen::common::unsigned_guard_facts::optimize_with_bounds(
                &mut context,
                &bounds,
            )
        } else {
            0
        };
        report.unsigned_guards += unsigned_guards;
        crate::native_artifact::program::codegen::cfg_cleanup::finish_boolean_conditions(
            controls,
            &mut context,
            &mut cleanup,
        );
        let header = region.tail.header() as usize;
        let symbol = format!("{entry_symbol}_numeric_{header}");
        let function = module
            .declare_function(&symbol, Linkage::Local, &signature)
            .map_err(backend)?;
        module
            .define_function(function, &mut context)
            .map_err(backend)?;
        let compiled = context
            .compiled_code()
            .ok_or_else(|| backend("numeric function has no code"))?;
        let stack = compiled_stack_bytes(&compiled.buffer)?;
        let bytes = u64::from(compiled.code_info().total_size);
        limit("Numeric function stack", stack, limits.max_stack_bytes)?;
        limit("Numeric function code", bytes, limits.max_code_bytes)?;
        report.numeric_write_loops.extend(local.numeric_write_loops);
        report
            .readonly_numeric_loops
            .extend(local.readonly_numeric_loops);
        report.numeric_functions.push(OutlinedReport {
            header,
            symbol,
            stack_bytes: stack,
            code_bytes: bytes,
            caller_template_bound: caller_bound(region),
            cleanup,
            unsigned_guards,
        });
        functions.push((header, function, stack, bytes));
    }
    Ok(functions)
}

pub(super) fn call(
    b: &mut FunctionBuilder<'_>,
    function: FuncRef,
    frame: Value,
    locals: Value,
    context: Value,
    region: &Region,
    ordinary_blocks: &[Option<Block>],
    ordinary: Block,
    error: Block,
) -> Result<(), NativeCompileError> {
    let before = b.func.dfg.num_insts();
    let call = b.ins().call(function, &[frame, locals, context]);
    let status = b.inst_results(call)[0];
    let token = b.inst_results(call)[1];
    let ok = b.ins().icmp_imm(IntCC::Equal, status, 0);
    let dispatch = b.create_block();
    b.ins().brif(ok, dispatch, &[], error, &[status.into()]);
    b.switch_to_block(dispatch);
    let invalid = b.create_block();
    b.set_cold_block(invalid);
    let mut switch = Switch::new();
    switch.set_entry(0, ordinary);
    for (token, &site) in continuations(region).iter().enumerate().skip(1) {
        switch.set_entry(
            token as u128,
            ordinary_blocks[site]
                .ok_or_else(|| backend("numeric native continuation is missing"))?,
        );
    }
    switch.emit(b, token, invalid);
    b.switch_to_block(invalid);
    let status = b.ins().iconst(types::I64, STATUS_ABI as i64);
    b.ins().jump(error, &[status.into()]);
    if b.func.dfg.num_insts() - before + 4 > caller_bound(region) {
        return Err(backend(
            "numeric caller exceeds independent boundary envelope",
        ));
    }
    Ok(())
}
