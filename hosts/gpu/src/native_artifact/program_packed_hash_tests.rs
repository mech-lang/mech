//! Execute the production hash emitter, including unsigned high multiplication.
use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn folded_hash_generated_matches_runtime_for_both_unsigned_words() {
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut context = module.make_context();
    context.func.signature.params.extend([AbiParam::new(types::I64); 2]);
    context.func.signature.returns.push(AbiParam::new(types::I64));
    let id = module.declare_function("packed_hash_words", Linkage::Local,
        &context.func.signature).unwrap();
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let value = hash_words(&mut b, args[0], args[1]);
        b.ins().return_(&[value]);
        b.seal_all_blocks();
        b.finalize();
    }
    let ir = context.func.display().to_string();
    assert!(ir.contains("umulhi"), "full unsigned product is required");
    assert!(!ir.contains("call ") && !ir.contains("call_indirect"));
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(u64, u64) -> u64;
    // SAFETY: this exact signature is defined above and module owns its code
    // for every synchronous call below. The function retains no arguments.
    let entry = unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    let mut words = vec![0, 1, u64::MAX, HASH_FIRST, HASH_SECOND];
    for bit in 0..64 { words.extend([1u64 << bit, !(1u64 << bit)]); }
    // Include odd/even, zero-product and high-bit operands, all key bits and
    // both narrow (high=0) and wide layouts. Distribution is not a proof of
    // equality; compare the actual emitted function with the runtime owner.
    for &low in &words {
        for &high in &words {
            let expected = crate::native_artifact::program_packed_map::hash(low, high);
            assert_eq!(unsafe { entry(low, high) }, expected, "{low:016x}/{high:016x}");
        }
    }
}
