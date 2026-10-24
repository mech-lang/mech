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

#[test]
fn control_group_generated_keeps_all_lanes_empty_bits_and_eight_byte_extent() {
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut context = module.make_context();
    context.func.signature.params.extend([AbiParam::new(types::I64); 3]);
    let id = module.declare_function("packed_control_group", Linkage::Local,
        &context.func.signature).unwrap();
    let mut fb = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
        let entry = b.create_block(); b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let (matches, empty) = control_group(&mut b, args[0], args[1]);
        b.ins().store(MemFlags::new(), matches, args[2], 0);
        b.ins().store(MemFlags::new(), empty, args[2], 8);
        b.ins().return_(&[]); b.seal_all_blocks(); b.finalize();
    }
    let loads = context.func.layout.blocks().flat_map(|block| context.func.layout.block_insts(block))
        .filter(|&inst| context.func.dfg.insts[inst].opcode() == cranelift_codegen::ir::Opcode::Load)
        .collect::<Vec<_>>();
    assert_eq!(loads.len(), 1, "one bounded control-group read");
    let value = context.func.dfg.inst_results(loads[0])[0];
    assert_eq!(context.func.dfg.value_type(value).bytes(), 8, "no 16-byte vector overread");
    module.define_function(id, &mut context).unwrap(); module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u8, u64, *mut u64);
    // SAFETY: the module owns this exact synchronous signature. Every source
    // pointer has eight readable bytes and each output has two writable words.
    let entry = unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    let mut controls = [0u8; 24];
    for tag in 0..128u64 {
        for matching in 0..256u16 {
            let offset = matching as usize % 8;
            let empties = matching.rotate_left(3) & !matching;
            let mut expected = [0u64; 2];
            for lane in 0..8 {
                let byte = if matching & (1 << lane) != 0 { tag as u8 }
                    else if empties & (1 << lane) != 0 { 0x80 }
                    else { (tag as u8 + 1) & 0x7f };
                controls[offset + lane] = byte;
                expected[0] |= u64::from(byte == tag as u8) << (lane * 8);
                expected[1] |= u64::from(byte & 0x80) << (lane * 8);
            }
            let mut actual = [u64::MAX; 2];
            unsafe { entry(controls.as_ptr().add(offset), tag, actual.as_mut_ptr()); }
            assert_eq!(actual, expected, "tag={tag}, matching={matching}, offset={offset}");
        }
    }
}
