use crate::native_artifact::program::codegen::hooks::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::Linkage;

fn answer(marker: u64, values: [u64; 5]) -> u64 {
    values
        .into_iter()
        .zip([3, 5, 7, 11, 13])
        .fold(marker, |value, (input, weight)| {
            value.wrapping_add(input.wrapping_mul(weight))
        })
}
unsafe extern "C" fn record(
    raw: u64,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    // SAFETY: each context is an exclusive three-word slice during the
    // synchronous call; the caller supplies a separate live result word.
    let words = unsafe { &mut *(raw as *mut [u64; 3]) };
    words[0] += 1;
    unsafe {
        *result = answer(words[1], [site, operation, first, second, third]);
    }
    words[2]
}
unsafe extern "C" fn record_other(
    raw: u64,
    site: u64,
    operation: u64,
    first: u64,
    second: u64,
    third: u64,
    result: *mut u64,
) -> u64 {
    // SAFETY: same context and result contract as record().
    let status = unsafe { record(raw, site, operation, first, second, third, result) };
    unsafe {
        *result = (*result).wrapping_add(1000);
    }
    status
}

#[test]
fn preserving_calls_hoist_only_exact_bindings_and_keep_changed_helpers_contexts_and_errors() {
    if !cfg!(target_arch = "aarch64") {
        return;
    }
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[
            ("opt_level", "speed"),
            ("preserve_frame_pointers", "true"),
            ("unwind_info", "true"),
        ],
        cranelift_module::default_libcall_names(),
    )
    .unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut report = crate::native_artifact::codegen::control::CodegenReport::default();
    let controls = crate::native_artifact::codegen::options::CompilationControls::new(
        crate::native_artifact::codegen::options::OptimizationOptions::default(),
    );
    let shim = crate::native_artifact::program::codegen::bound_shim::define(
        &controls,
        &mut module,
        "binding_test",
        crate::native_artifact::NativeCompileLimits::default(),
        &mut report,
    )
    .unwrap()
    .unwrap();
    let mut signature = module.make_signature();
    signature.params.push(AbiParam::new(types::I64));
    signature.returns.push(AbiParam::new(types::I64));
    let caller = module
        .declare_function("binding_caller", Linkage::Export, &signature)
        .unwrap();
    let mut context = module.make_context();
    context.func.signature = signature;
    let mut fb = FunctionBuilderContext::new();
    let inputs = [
        [2u64, 3, 5, 7, 11],
        [13, 17, 19, 23, 29],
        [31, 37, 41, 43, 47],
        [53, 59, 61, 67, 71],
    ];
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let pointer = b.block_params(entry)[0];
        let other_context = b.ins().iadd_imm(pointer, 24);
        let result = b.ins().iadd_imm(pointer, 48);
        let function = b
            .ins()
            .iconst(types::I64, record as *const () as usize as i64);
        let other_function = b
            .ins()
            .iconst(types::I64, record_other as *const () as usize as i64);
        let mut native = module.make_signature();
        native.params = vec![AbiParam::new(types::I64); 7];
        native.returns.push(AbiParam::new(types::I64));
        let zero = b.ins().iconst(types::I64, 0);
        let hooks = Hooks {
            signature: b.import_signature(native),
            function,
            context: pointer,
            result,
            arguments: pointer,
            argument_capacity: zero,
            bound_shim: None,
            bound_shim_calls: std::cell::Cell::new(0),
        }
        .with_bound_shim(&mut b, &mut module, shim.function, function, pointer);
        let error = b.create_block();
        b.append_block_param(error, types::I64);
        let mut total = zero;
        for (index, input) in inputs.iter().enumerate() {
            let arguments =
                [input[2], input[3], input[4]].map(|v| b.ins().iconst(types::I64, v as i64));
            hooks.bound_call(
                &mut b,
                if index == 2 { other_function } else { function },
                if index == 3 { other_context } else { pointer },
                input[0] as usize,
                OwnerOperation::decode(input[1]),
                arguments,
                error,
            );
            let value = b.ins().load(types::I64, MemFlags::new(), result, 0);
            total = b.ins().iadd(total, value);
        }
        assert_eq!(hooks.bound_shim_calls(), 2);
        b.ins().return_(&[total]);
        b.switch_to_block(error);
        let status = b.block_params(error)[0];
        b.ins().return_(&[status]);
        b.seal_all_blocks();
        b.finalize();
    }
    module.define_function(caller, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: this module owns the matching pointer -> u64 definition
    // throughout each synchronous call and every pointed-to word is live.
    let call: unsafe extern "C" fn(*mut u64) -> u64 =
        unsafe { std::mem::transmute(module.get_finalized_function(caller)) };
    for (first_status, second_status) in [(0, 0), (0, 7), (0, 0xffff_ffff_0000_0000), (7, 0)] {
        let mut words = [0, 123, first_status, 0, 456, second_status, u64::MAX];
        let expected = if first_status != 0 || second_status != 0 {
            crate::native_artifact::abi::STATUS_ADMISSION
        } else {
            inputs
                .iter()
                .enumerate()
                .map(|(index, input)| {
                    answer(if index == 3 { 456 } else { 123 }, *input)
                        + if index == 2 { 1000 } else { 0 }
                })
                .sum()
        };
        assert_eq!(unsafe { call(words.as_mut_ptr()) }, expected);
        assert_eq!(words[0], if first_status != 0 { 1 } else { 3 });
        assert_eq!(words[3], if first_status != 0 { 0 } else { 1 });
        assert_eq!(
            [words[1], words[2], words[4], words[5]],
            [123, first_status, 456, second_status]
        );
    }
}
