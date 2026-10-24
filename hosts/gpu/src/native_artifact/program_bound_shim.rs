//! Owned cold-call bridge. Only generated callers use PreserveAll; the
//! installed Rust helper retains its original native C ABI and result word.
use super::{NativeCompileError, NativeCompileLimits, compiler::{backend, compiled_stack_bytes, limit}};
use cranelift_codegen::{ir::{AbiParam, InstBuilder, MemFlags, types}, isa::CallConv};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{FuncId, Linkage, Module};

// AArch64 fixed-width instructions let the emitted code size independently
// bound the complete prologue, all register saves/restores and native call.
// This fits inside the existing 512-unit source-site work envelope; callers
// separately check their emitted instructions plus every bridge template.
pub(super) const TEMPLATE_BOUND: usize = 128;
// Eight input words and one distinct status word belong to the caller's
// invocation. Passing their address leaves only one argument register live at
// the cold bridge boundary. The original seven-argument native ABI is intact.
pub(super) const ARGUMENT_WORDS: usize = 8;
pub(super) const STATUS_OFFSET: i32 = ARGUMENT_WORDS as i32 * 8;
pub(super) const CALL_BYTES: u32 = (ARGUMENT_WORDS as u32 + 1) * 8;

#[derive(Clone, Copy)]
pub(super) struct Definition { pub function: FuncId, pub stack: u64, pub code: u64 }

#[cfg(test)]
thread_local! {
    static DISABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    static EMISSIONS: std::cell::Cell<usize> = const { std::cell::Cell::new(0) };
}
#[cfg(test)]
pub(super) fn take_emissions() -> usize { EMISSIONS.with(|v| v.replace(0)) }
pub(super) fn note_call() {
    #[cfg(test)]
    EMISSIONS.with(|v| v.set(v.get() + 1));
}
#[cfg(test)]
pub(super) fn without_preserving_calls<T>(f: impl FnOnce() -> T) -> T {
    struct Restore(bool);
    impl Drop for Restore { fn drop(&mut self) { DISABLED.with(|v| v.set(self.0)); } }
    let _restore = Restore(DISABLED.with(|v| v.replace(true)));
    f()
}

pub(super) fn define<M: Module>(module: &mut M, entry: &str, limits: NativeCompileLimits,
    report: &mut super::iteration_compiler::CodegenReport,
) -> Result<Option<Definition>, NativeCompileError> {
    if !cfg!(target_arch = "aarch64") { return Ok(None); }
    #[cfg(test)]
    if DISABLED.with(std::cell::Cell::get) { return Ok(None); }
    let symbol = format!("{entry}_preserving_bound_call");
    let mut signature = module.make_signature();
    signature.call_conv = CallConv::PreserveAll;
    // Caller-owned words: fn pointer, context, source site, operation, three
    // arguments, helper result pointer, status. No return registers are
    // permitted by PreserveAll. The local scratch pointer never escapes.
    signature.params.push(AbiParam::new(types::I64));
    let function = module.declare_function(&symbol, Linkage::Local, &signature).map_err(backend)?;
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let scratch = b.block_params(entry)[0];
        let args = (0..ARGUMENT_WORDS).map(|index|
            b.ins().load(types::I64, MemFlags::new(), scratch, (index * 8) as i32)
        ).collect::<Vec<_>>();
        let mut native = module.make_signature();
        native.params = vec![AbiParam::new(types::I64); 7];
        native.returns.push(AbiParam::new(types::I64));
        let native = b.import_signature(native);
        let call = b.ins().call_indirect(native, args[0], &args[1..8]);
        let status = b.inst_results(call)[0];
        b.ins().store(MemFlags::new(), status, scratch, STATUS_OFFSET);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(function, &mut context).map_err(backend)?;
    let compiled = context.compiled_code().ok_or_else(|| backend("preserving bridge has no code"))?;
    let stack = compiled_stack_bytes(&compiled.buffer)?;
    let code = u64::from(compiled.code_info().total_size);
    if code % 4 != 0 || code / 4 > TEMPLATE_BOUND as u64 {
        return Err(backend("preserving bridge exceeds fixed native instruction envelope"));
    }
    limit("Preserving bridge stack", stack, limits.max_stack_bytes)?;
    limit("Preserving bridge code", code, limits.max_code_bytes)?;
    report.bound_call_shim = Some(super::iteration_compiler::BoundCallShim {
        symbol, stack_bytes: stack, code_bytes: code, template_bound: TEMPLATE_BOUND,
    });
    Ok(Some(Definition { function, stack, code }))
}

#[cfg(test)]
mod tests {
    use super::*;

    unsafe extern "C" fn clobber(raw: u64, site: u64, operation: u64,
        first: u64, second: u64, third: u64, result: *mut u64) -> u64
    {
        // SAFETY: this test passes one live, exclusive 64-word array for the
        // complete synchronous call. The two result words are disjoint.
        let words = unsafe { &mut *(raw as *mut [u64; 64]) };
        let status = words[62];
        words[..34].fill(0);
        unsafe { *result = site.wrapping_mul(3).wrapping_add(operation.wrapping_mul(5))
            .wrapping_add(first.wrapping_mul(7)).wrapping_add(second.wrapping_mul(11))
            .wrapping_add(third.wrapping_mul(13)); }
        status
    }

    #[test]
    fn preserving_calls_bridge_keeps_live_scalar_vector_values_result_and_status() {
        if !cfg!(target_arch = "aarch64") { return; }
        let builder = cranelift_jit::JITBuilder::with_flags(&[
            ("opt_level", "speed"), ("preserve_frame_pointers", "true"), ("unwind_info", "true"),
        ], cranelift_module::default_libcall_names()).unwrap();
        let mut module = cranelift_jit::JITModule::new(builder);
        let mut report = super::super::iteration_compiler::CodegenReport::default();
        let shim = define(&mut module, "bridge_test", NativeCompileLimits::default(), &mut report).unwrap().unwrap();
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I64));
        let caller = module.declare_function("bridge_caller", Linkage::Export, &signature).unwrap();
        let mut context = module.make_context();
        context.func.signature = signature;
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let values = (0..32).map(|i| b.ins().load(types::I64, MemFlags::new(), pointer, i * 8)).collect::<Vec<_>>();
            let vector = b.ins().load(types::I64X2, MemFlags::new(), pointer, 32 * 8);
            let function = b.ins().iconst(types::I64, clobber as *const () as usize as i64);
            let constants = [0x55, 0x77, 0x123, 0x456, 0x789].map(|v| b.ins().iconst(types::I64, v));
            let result = b.ins().iadd_imm(pointer, 60 * 8);
            let status = b.ins().iadd_imm(pointer, 61 * 8);
            let slot = b.create_sized_stack_slot(cranelift_codegen::ir::StackSlotData::new(
                cranelift_codegen::ir::StackSlotKind::ExplicitSlot, CALL_BYTES, 3));
            let scratch = b.ins().stack_addr(types::I64, slot, 0);
            for (index, value) in [function, pointer, constants[0], constants[1],
                constants[2], constants[3], constants[4], result].into_iter().enumerate()
            {
                b.ins().store(MemFlags::new(), value, scratch, (index * 8) as i32);
            }
            let reference = module.declare_func_in_func(shim.function, b.func);
            b.ins().call(reference, &[scratch]);
            let returned_status = b.ins().load(types::I64, MemFlags::new(), scratch, STATUS_OFFSET);
            b.ins().store(MemFlags::new(), returned_status, status, 0);
            let mut total = b.ins().iconst(types::I64, 0);
            for value in values { total = b.ins().iadd(total, value); }
            for lane in 0..2 {
                let value = b.ins().extractlane(vector, lane);
                total = b.ins().iadd(total, value);
            }
            for output in [result, status] {
                let value = b.ins().load(types::I64, MemFlags::new(), output, 0);
                total = b.ins().iadd(total, value);
            }
            b.ins().return_(&[total]);
            b.seal_all_blocks();
            b.finalize();
        }
        module.define_function(caller, &mut context).unwrap();
        module.finalize_definitions().unwrap();
        // SAFETY: the checked signature is u64 pointer -> u64 and this owner
        // retains both generated definitions throughout every invocation.
        let call: unsafe extern "C" fn(*mut u64) -> u64 = unsafe { std::mem::transmute(module.get_finalized_function(caller)) };
        let helper_result = 0x55u64 * 3 + 0x77 * 5 + 0x123 * 7 + 0x456 * 11 + 0x789 * 13;
        for status in [0, 7, 0xffff_ffff_aabb_ccdd] {
            let mut words = core::array::from_fn::<_, 64, _>(|i| (i as u64).wrapping_mul(0x9e37_79b9_7f4a_7c15));
            let expected = words[..34].iter().fold(0u64, |sum, v| sum.wrapping_add(*v)).wrapping_add(helper_result).wrapping_add(status);
            words[60] = u64::MAX; words[61] = u64::MAX; words[62] = status;
            assert_eq!(unsafe { call(words.as_mut_ptr()) }, expected);
            assert!(words[..34].iter().all(|word| *word == 0));
            assert_eq!(words[60], helper_result);
            assert_eq!(words[61], status);
        }
    }
}
