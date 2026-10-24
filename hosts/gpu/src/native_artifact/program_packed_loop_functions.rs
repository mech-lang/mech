//! Isolate a proved closed loop from its parent's unrelated live values.
//! The function is emitted from the same body as the inline fallback. It is
//! called once per selected region, never once per source update.
use super::*;
use crate::native_artifact::{
    NativeCompileLimits,
    compiler::{compiled_stack_bytes, limit},
    iteration_compiler::OutlinedLoop,
};
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{FuncId, Linkage, Module};

// Includes the actual selector, selection receipt, direct call and return
// dispatch. The caller checks this bound at emission; every callee includes it
// in the existing, unchanged fixed per-traversal work envelope.
pub(super) const CALLER_TEMPLATE_BOUND: usize = 64;
const MAX_FUNCTIONS: usize = 256;

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(in crate::native_artifact) fn without_loop_outlining<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore {
        fn drop(&mut self) { DISABLED.with(|value| value.set(self.0)); }
    }
    let _restore = Restore(DISABLED.with(|value| value.replace(true)));
    f()
}
#[cfg(test)]
pub(in crate::native_artifact) fn take_outline_emissions() -> usize {
    EMISSIONS.with(|value| value.replace(0))
}

pub(in crate::native_artifact) fn define_functions<M: Module>(
    module: &mut M, entry_symbol: &str, code: CodePlan<'_>, closed: &Layout,
    owner: Option<&NativeMapOwnerWords>, aggregates: &NativeScalarAggregates,
    limits: NativeCompileLimits, report: &mut CodegenReport,
) -> Result<Vec<(usize, FuncId, u64, u64)>, NativeCompileError> {
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) { return Ok(Vec::new()); }
    let Some(owner) = owner else { return Ok(Vec::new()); };
    let mut functions = Vec::new();
    for (id, region) in closed.regions.iter().enumerate() {
        if functions.len() == MAX_FUNCTIONS { break; }
        let Some(slots) = eligible_slots(code, closed, id, owner, aggregates) else { continue; };
        let site = region.tail.header() as usize;
        let symbol = format!("{entry_symbol}_packed_loop_{site}");
        let mut signature = module.make_signature();
        // Root frame, source locals, private packed frame, installed private
        // helper, its context, and the original transport context. No pointer
        // or value outlives the original synchronous owned invocation.
        signature.params = vec![AbiParam::new(types::I64); 6];
        signature.returns.push(AbiParam::new(types::I64));
        let function = module.declare_function(&symbol, Linkage::Local, &signature).map_err(backend)?;
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let args = b.block_params(entry).to_vec();
            let error = b.create_block();
            b.append_block_param(error, types::I64);
            b.set_cold_block(error);
            let exit = b.create_block();
            let hooks = Hooks::load(true, &mut b, module, args[0], args[5], error)?
                .ok_or_else(|| backend("outlined packed loop requires installed hooks"))?;
            // Cover the loader and both final returns in addition to the
            // separately bounded caller. The body itself checks the sum.
            let prefix_work = b.func.dfg.num_insts() + CALLER_TEMPLATE_BOUND + 3;
            emit_body(&mut b, args[0], args[1], args[2], args[3], args[4], code,
                closed, id, owner, aggregates, slots, &hooks, exit, error,
                prefix_work, report)?;
            b.switch_to_block(exit);
            let ok = b.ins().iconst(types::I64, 0);
            b.ins().return_(&[ok]);
            b.switch_to_block(error);
            let status = b.block_params(error)[0];
            b.ins().return_(&[status]);
            b.seal_all_blocks();
            b.finalize();
        }
        let cleanup = super::super::cfg_cleanup::optimize(&mut context, module.isa())?;
        module.define_function(function, &mut context).map_err(backend)?;
        let compiled = context.compiled_code().ok_or_else(|| backend("outlined loop has no compiled code"))?;
        let stack = compiled_stack_bytes(&compiled.buffer)?;
        let bytes = u64::from(compiled.code_info().total_size);
        limit("Outlined packed loop stack", stack, limits.max_stack_bytes)?;
        limit("Outlined packed loop code", bytes, limits.max_code_bytes)?;
        report.outlined_loops.push(OutlinedLoop {
            header: site, symbol, stack_bytes: stack, code_bytes: bytes,
            caller_template_bound: CALLER_TEMPLATE_BOUND, cleanup,
        });
        functions.push((site, function, stack, bytes));
        #[cfg(test)]
        EMISSIONS.with(|value| value.set(value.get() + 1));
    }
    Ok(functions)
}
