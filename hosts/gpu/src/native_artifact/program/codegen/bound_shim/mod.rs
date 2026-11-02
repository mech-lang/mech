//! Owned cold-call bridge. Only generated callers use PreserveAll; the
//! installed Rust helper retains its original native C ABI and result word.
use crate::native_artifact::codegen::options::{CompilationControls, OptimizationPass};
use crate::native_artifact::compiler::{backend, compiled_stack_bytes, limit};
use crate::native_artifact::{NativeCompileError, NativeCompileLimits};
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_codegen::isa::CallConv;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

// AArch64 fixed-width instructions let the emitted code size independently
// bound the complete prologue, all register saves/restores and native call.
// This fits inside the existing 512-unit source-site work envelope; callers
// separately check their emitted instructions plus every bridge template.
pub(in crate::native_artifact) const TEMPLATE_BOUND: usize = 128;
// Eight input words and one distinct status word belong to the caller's
// invocation. Passing their address leaves only one argument register live at
// the cold bridge boundary. The original seven-argument native ABI is intact.
pub(in crate::native_artifact) const ARGUMENT_WORDS: usize = 8;
pub(in crate::native_artifact) const STATUS_OFFSET: i32 = ARGUMENT_WORDS as i32 * 8;
pub(in crate::native_artifact) const CALL_BYTES: u32 = (ARGUMENT_WORDS as u32 + 1) * 8;

#[derive(Clone, Copy)]
pub(in crate::native_artifact) struct Definition {
    pub function: FuncId,
    pub stack: u64,
    pub code: u64,
}

pub(in crate::native_artifact) fn define<M: Module>(
    controls: &CompilationControls,
    module: &mut M,
    entry: &str,
    limits: NativeCompileLimits,
    report: &mut crate::native_artifact::codegen::control::CodegenReport,
) -> Result<Option<Definition>, NativeCompileError> {
    if !cfg!(target_arch = "aarch64") {
        return Ok(None);
    }
    if !controls.enabled(OptimizationPass::PreservingCalls) {
        return Ok(None);
    }
    let symbol = format!("{entry}_preserving_bound_call");
    let mut signature = module.make_signature();
    signature.call_conv = CallConv::PreserveAll;
    // Caller-owned words: fn pointer, context, source site, operation, three
    // arguments, helper result pointer, status. No return registers are
    // permitted by PreserveAll. The local scratch pointer never escapes.
    signature.params.push(AbiParam::new(types::I64));
    let function = module
        .declare_function(&symbol, Linkage::Local, &signature)
        .map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let scratch = b.block_params(entry)[0];
        let args = (0..ARGUMENT_WORDS)
            .map(|index| {
                b.ins()
                    .load(types::I64, MemFlags::new(), scratch, (index * 8) as i32)
            })
            .collect::<Vec<_>>();
        let mut native = module.make_signature();
        native.params = vec![AbiParam::new(types::I64); 7];
        native.returns.push(AbiParam::new(types::I64));
        let native = b.import_signature(native);
        let call = b.ins().call_indirect(native, args[0], &args[1..8]);
        let status = b.inst_results(call)[0];
        b.ins()
            .store(MemFlags::new(), status, scratch, STATUS_OFFSET);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    module
        .define_function(function, &mut context)
        .map_err(backend)?;
    let compiled = context
        .compiled_code()
        .ok_or_else(|| backend("preserving bridge has no code"))?;
    let stack = compiled_stack_bytes(&compiled.buffer)?;
    let code = u64::from(compiled.code_info().total_size);
    if code % 4 != 0 || code / 4 > TEMPLATE_BOUND as u64 {
        return Err(backend(
            "preserving bridge exceeds fixed native instruction envelope",
        ));
    }
    limit("Preserving bridge stack", stack, limits.max_stack_bytes)?;
    limit("Preserving bridge code", code, limits.max_code_bytes)?;
    report.bound_call_shim = Some(crate::native_artifact::codegen::control::BoundCallShim {
        symbol,
        stack_bytes: stack,
        code_bytes: code,
        template_bound: TEMPLATE_BOUND,
    });
    Ok(Some(Definition {
        function,
        stack,
        code,
    }))
}

#[cfg(test)]
mod tests;
