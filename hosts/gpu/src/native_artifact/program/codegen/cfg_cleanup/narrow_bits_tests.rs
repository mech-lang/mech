//! Native differential coverage of low costs and simultaneously live full words.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Type, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn ordered_reduction_narrow_costs_preserve_full_packed_words() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut entries = Vec::new();
    for ty in [types::I8, types::I16, types::I32] {
        for shift in [
            None,
            Some(0u64),
            Some(7),
            Some(8),
            Some(15),
            Some(16),
            Some(31),
            Some(32),
            Some(63),
            Some(64),
            Some(96),
        ] {
            for operation in 0..3 {
                for immediate in [false, true] {
                    for folded in [false, true] {
                        let mut context = module.make_context();
                        context.func.signature.params = vec![AbiParam::new(types::I64); 4];
                        context.func.signature.returns = vec![AbiParam::new(types::I64)];
                        let mut builder = FunctionBuilderContext::new();
                        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
                        let entry = b.create_block();
                        b.append_block_params_for_function_params(entry);
                        b.switch_to_block(entry);
                        let [a, high, dynamic, output] = b.block_params(entry)[..] else {
                            unreachable!()
                        };
                        let high = if let Some(shift) = shift {
                            if immediate {
                                b.ins().ishl_imm(high, shift as i64)
                            } else {
                                // Include the actual phase's constant expression,
                                // rather than requiring a literal shift node.
                                let amount = b.ins().iconst(types::I64, shift as i64);
                                let amount = b.ins().band_imm(amount, 63);
                                b.ins().ishl(high, amount)
                            }
                        } else {
                            b.ins().ishl(high, dynamic)
                        };
                        let packed = match operation {
                            0 => b.ins().bor(a, high),
                            1 => b.ins().bxor(high, a),
                            _ => b.ins().band(a, high),
                        };
                        b.ins().store(MemFlags::new(), packed, output, 0);
                        let low = b.ins().ireduce(ty, packed);
                        let low = b.ins().uextend(types::I64, low);
                        b.ins().return_(&[low]);
                        b.seal_all_blocks();
                        b.finalize();
                        if folded {
                            assert_eq!(
                                super::fold(&mut context.func),
                                usize::from(
                                    shift.is_some_and(|s| (s & 63) >= u64::from(ty.bits()))
                                )
                            );
                        }
                        context.verify(module.isa()).unwrap();
                        let id = module
                            .declare_function(
                                &format!("narrow_packed_{}", entries.len()),
                                Linkage::Local,
                                &context.func.signature,
                            )
                            .unwrap();
                        module.define_function(id, &mut context).unwrap();
                        entries.push((id, ty, shift, operation));
                    }
                }
            }
        }
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64;
    let entries = entries
        .into_iter()
        .map(|(id, ty, shift, operation)| {
            // SAFETY: the live finalized JIT owns this exact four-lane C ABI.
            let call = unsafe {
                std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id))
            };
            (call, ty, shift, operation)
        })
        .collect::<Vec<_>>();
    let mut seed = 0x6ef1_3892_d745_0abcu64;
    let mut calls = 0;
    for sample in 0..256u64 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let a = match sample % 8 {
            0 => 0,
            1 => u64::MAX,
            2 => 1u64 << 63,
            3 => u64::from(u32::MAX),
            _ => next(),
        };
        let high_word = next();
        for &(call, ty, shift, operation) in &entries {
            let shift = shift.unwrap_or(sample);
            let high = high_word.wrapping_shl(shift as u32);
            let expected = match operation {
                0 => a | high,
                1 => high ^ a,
                _ => a & high,
            };
            let mut output = [0xa913_739d_fec8_1672, 0, 0x3948_70af_6591_23de];
            // SAFETY: arguments are scalar words and the disjoint initialized
            // aligned output has exactly one live exclusive word for the store.
            let low = unsafe { call(a, high_word, shift, output.as_mut_ptr().add(1)) };
            assert_eq!(low, expected & mask(ty));
            assert_eq!(
                output,
                [0xa913_739d_fec8_1672, expected, 0x3948_70af_6591_23de]
            );
            calls += 1;
        }
    }
    assert_eq!(calls, 101_376);
}

fn mask(ty: Type) -> u64 {
    u64::MAX >> (64 - ty.bits())
}
