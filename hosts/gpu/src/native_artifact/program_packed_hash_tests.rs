//! Execute the production hash emitter, including unsigned high multiplication.
use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::FunctionBuilderContext;
use cranelift_module::{Linkage, Module, default_libcall_names};

#[path = "program_packed_probe_group_codegen_tests.rs"]
mod probe_group_tests;

#[test]
fn direct_lookup_cache_generated_keeps_absence_zero_values_and_hash_fallback() {
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut functions = Vec::new();
    for direct in [false, true] {
        let mut context = module.make_context();
        context.func.signature.params.push(AbiParam::new(types::I64));
        context.func.signature.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function(&format!("direct_lookup_cache_{direct}"),
            Linkage::Local, &context.func.signature).unwrap();
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            let absent = b.create_block();
            let bad = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let frame = FrameState::memory(pointer, Domain::ascii()).with_specialized_width(Some(1));
            let result = probe_fresh_core_to(&mut b, &frame, true, bad, direct.then_some(absent));
            b.ins().return_(&[result]);
            if direct {
                b.switch_to_block(absent);
                let zero = b.ins().iconst(types::I64, 0);
                b.ins().return_(&[zero]);
            }
            b.switch_to_block(bad);
            let rejected = b.ins().iconst(types::I64, -1);
            b.ins().return_(&[rejected]);
            b.seal_all_blocks(); b.finalize();
        }
        module.define_function(id, &mut context).unwrap();
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    let mut slots = vec![Slot::default(); 128];
    let mut controls = vec![0x80; 128 + GROUP - 1];
    let mut cache = vec![0u64; 128];
    for (key, value) in [(0, 0), (17, u64::MAX), (63, 0), (127, 91)] {
        let hash = crate::native_artifact::program_packed_map::hash(key, 0);
        let mut at = hash as usize & 120;
        while controls[at] != 0x80 { at = (at + 1) & 127; }
        slots[at] = Slot { low: key, high: 0, value };
        controls[at] = (hash >> 57) as u8;
        cache[key as usize] = &slots[at] as *const Slot as u64;
    }
    for lane in 0..GROUP - 1 { controls[128 + lane] = controls[lane]; }
    type Entry = unsafe extern "C" fn(*mut Frame) -> u64;
    for id in functions {
        // SAFETY: module owns this exact synchronous signature; the frame,
        // table, controls and complete-range cache remain live across calls.
        let entry = unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
        for cached in [false, true] {
            for key in 0..128u64 {
                let mut frame = Frame {
                    slots: slots.as_mut_ptr() as u64, capacity: 128, len: 4,
                    fuel: 0, updates: 0, key_low: key, key_high: 0,
                    entry_pointer: 0, entry_known: 0, source_ptr: 0, source_len: 0,
                    width: 1, previous_start: 0, low_mask: 127, high_mask: 0,
                    range_call_allowed: 0, read_call_allowed: 0, write_call_allowed: 0,
                    controls: controls.as_ptr() as u64,
                    lookup_cache: if cached { cache.as_ptr() as u64 } else { 0 },
                };
                assert_eq!(unsafe { entry(&mut frame) }, cache[key as usize]);
                assert_eq!((frame.entry_pointer, frame.entry_known), (cache[key as usize], 1));
            }
        }
    }
}

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
                expected[0] |= (if byte == tag as u8 { 0xff } else { 0 }) << (lane * 8);
                expected[1] |= u64::from(byte & 0x80) << (lane * 8);
            }
            let mut actual = [u64::MAX; 2];
            unsafe { entry(controls.as_ptr().add(offset), tag, actual.as_mut_ptr()); }
            assert_eq!(actual, expected, "tag={tag}, matching={matching}, offset={offset}");
        }
    }
}

#[test]
fn direct_present_probe_keeps_collisions_wrapping_misses_and_bounded_failure() {
    let builder = cranelift_jit::JITBuilder::with_flags(
        &[("opt_level", "speed")], default_libcall_names()).unwrap();
    let mut module = cranelift_jit::JITModule::new(builder);
    let mut functions = Vec::new();
    for direct in [false, true] {
        let mut context = module.make_context();
        context.func.signature.params.push(AbiParam::new(types::I64));
        context.func.signature.returns.push(AbiParam::new(types::I64));
        let id = module.declare_function(&format!("packed_present_probe_{direct}"),
            Linkage::Local, &context.func.signature).unwrap();
        let mut fb = FunctionBuilderContext::new();
        {
            let mut b = FunctionBuilder::new(&mut context.func, &mut fb);
            let entry = b.create_block();
            let absent = b.create_block();
            let bad = b.create_block();
            b.append_block_params_for_function_params(entry);
            b.switch_to_block(entry);
            let pointer = b.block_params(entry)[0];
            let frame = FrameState::memory(pointer, Domain::ascii());
            let result = probe_fresh_core_to(&mut b, &frame, true, bad, direct.then_some(absent));
            b.ins().return_(&[result]);
            if direct {
                b.switch_to_block(absent);
                let zero = b.ins().iconst(types::I64, 0);
                b.ins().return_(&[zero]);
            }
            b.switch_to_block(bad);
            let rejected = b.ins().iconst(types::I64, -1);
            b.ins().return_(&[rejected]);
            b.seal_all_blocks();
            b.finalize();
        }
        module.define_function(id, &mut context).unwrap();
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*mut Frame) -> u64;
    for query in 0..16u64 {
        for high in [0, 1 << 31, 1 << 60] {
            let hash = crate::native_artifact::program_packed_map::hash(query, high);
            let start = hash as usize & 15;
            let tag = (hash >> 57) as u8;
            // Actual same-bucket/fingerprint collisions, rather than controls
            // inconsistent with their stored keys. These exercise more than
            // one matching byte, wraparound and a completely full first group.
            let collisions = (0..1_000_000u64).filter(|&key| key != query && {
                let other = crate::native_artifact::program_packed_map::hash(key, high);
                other as usize & 15 == start && (other >> 57) as u8 == tag
            }).take(16).collect::<Vec<_>>();
            assert_eq!(collisions.len(), 16, "bounded collision fixture");
            for (count, hit) in [(0, None), (1, Some(0)), (3, Some(2)),
                (9, Some(8)), (8, None), (16, None)]
            {
                let mut slots = vec![Slot::default(); 16];
                let mut controls = vec![0x80; 16 + GROUP - 1];
                for lane in 0..count {
                    let bucket = (start + lane) & 15;
                    slots[bucket] = Slot {
                        low: if hit == Some(lane) { query } else { collisions[lane] },
                        high,
                        value: lane as u64 + 11,
                    };
                    controls[bucket] = tag;
                }
                for lane in 0..GROUP - 1 { controls[16 + lane] = controls[lane]; }
                let expected = hit.map_or(if count == 16 { u64::MAX } else { 0 }, |lane|
                    &slots[(start + lane) & 15] as *const Slot as u64);
                for &id in &functions {
                    let mut frame = Frame {
                        slots: slots.as_mut_ptr() as u64, capacity: 16, len: count as u64,
                        fuel: 0, updates: 0, key_low: query, key_high: high,
                        entry_pointer: 0, entry_known: 0, source_ptr: 0, source_len: 0,
                        width: 18, previous_start: 0, low_mask: u64::MAX,
                        high_mask: (1 << 62) - 1, range_call_allowed: 0,
                        read_call_allowed: 0, write_call_allowed: 0,
                        controls: controls.as_ptr() as u64, lookup_cache: 0,
                    };
                    // SAFETY: this exact signature was emitted above. The
                    // module, frame, slots and mirrored control group all
                    // remain owned and live for the entire synchronous call.
                    let entry = unsafe { core::mem::transmute::<*const u8, Entry>(
                        module.get_finalized_function(id)) };
                    assert_eq!(unsafe { entry(&mut frame) }, expected,
                        "query={query}, high={high}, start={start}, count={count}, hit={hit:?}");
                    if expected == u64::MAX {
                        assert_eq!((frame.entry_pointer, frame.entry_known), (0, 0));
                    } else {
                        assert_eq!((frame.entry_pointer, frame.entry_known), (expected, 1));
                    }
                }
            }
        }
    }
}
