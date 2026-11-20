//! Loop and multi-edge coverage of separately carried full and narrow words.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn ordered_reduction_parallel_costs_require_every_incoming_full_word() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut entries = Vec::new();
    for ty in [types::I8, types::I16, types::I32] {
        for mode in 0..4 {
            for folded in [false, true] {
                let mut context = module.make_context();
                context.func.signature.params = vec![AbiParam::new(types::I64); 4];
                context.func.signature.returns = vec![AbiParam::new(types::I64)];
                let mut builder = FunctionBuilderContext::new();
                let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
                let entry = b.create_block();
                let header = b.create_block();
                let body = b.create_block();
                let even = b.create_block();
                let odd = b.create_block();
                let finish = b.create_block();
                b.append_block_params_for_function_params(entry);
                for ty in [types::I64, types::I64, ty] {
                    b.append_block_param(header, ty);
                }
                b.switch_to_block(entry);
                let [initial, salt, count, output] = b.block_params(entry)[..] else {
                    unreachable!()
                };
                let low = b.ins().ireduce(ty, initial);
                let low = if mode == 1 {
                    b.ins().bxor_imm(low, 1)
                } else {
                    low
                };
                let zero = b.ins().iconst(types::I64, 0);
                b.ins()
                    .jump(header, &[zero.into(), initial.into(), low.into()]);
                b.switch_to_block(header);
                let [index, full, carried] = b.block_params(header)[..] else {
                    unreachable!()
                };
                let more = b.ins().icmp(IntCC::UnsignedLessThan, index, count);
                b.ins().brif(more, body, &[], finish, &[]);
                b.switch_to_block(body);
                let mask = u64::MAX >> (64 - ty.bits());
                let low = b.ins().band_imm(full, mask as i64);
                let low = b.ins().ireduce(ty, low);
                let addition = b.ins().ireduce(ty, salt);
                let cost = b.ins().iadd(low, addition);
                let ordinal = b.ins().ireduce(ty, index);
                let cost = b.ins().iadd(cost, ordinal);
                let parity = b.ins().band_imm(index, 1);
                let parity = b.ins().icmp_imm(IntCC::NotEqual, parity, 0);
                b.ins().brif(parity, odd, &[], even, &[]);
                for (block, is_odd) in [(even, false), (odd, true)] {
                    b.switch_to_block(block);
                    let high = if is_odd {
                        b.ins().bxor(salt, index)
                    } else {
                        b.ins().iadd(salt, index)
                    };
                    let shift = if mode == 3 {
                        b.ins().band_imm(salt, 63)
                    } else {
                        b.ins().iconst(types::I64, 32)
                    };
                    let high = b.ins().ishl(high, shift);
                    let word = b.ins().uextend(types::I64, cost);
                    let word = b.ins().bor(word, high);
                    let offset = b.ins().ishl_imm(index, 3);
                    let address = b.ins().iadd(output, offset);
                    b.ins().store(MemFlags::new(), word, address, 0);
                    let low = if mode == 3 {
                        // A dynamic high shift need not leave the low bits zero.
                        // Carrying the pre-pack cost is not an equality proof.
                        cost
                    } else {
                        b.ins().ireduce(ty, word)
                    };
                    let low = if mode == 2 && is_odd {
                        b.ins().bxor_imm(low, 1)
                    } else {
                        low
                    };
                    let next = b.ins().iadd_imm(index, 1);
                    b.ins()
                        .jump(header, &[next.into(), word.into(), low.into()]);
                }
                b.switch_to_block(finish);
                b.ins().store(MemFlags::new(), full, output, 16 * 8);
                let result = b.ins().uextend(types::I64, carried);
                b.ins().return_(&[result]);
                b.seal_all_blocks();
                b.finalize();
                if folded {
                    super::fold(&mut context.func);
                    assert_eq!(
                        super::fold_parallel(&mut context.func),
                        usize::from(mode == 0)
                    );
                }
                context.verify(module.isa()).unwrap();
                let id = module
                    .declare_function(
                        &format!("parallel_low_{}", entries.len()),
                        Linkage::Local,
                        &context.func.signature,
                    )
                    .unwrap();
                module.define_function(id, &mut context).unwrap();
                entries.push((id, ty.bits(), mode));
            }
        }
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64;
    let entries = entries
        .into_iter()
        .map(|(id, bits, mode)| {
            // SAFETY: the live finalized module owns this exact C signature.
            let call = unsafe {
                std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id))
            };
            (call, bits, mode)
        })
        .collect::<Vec<_>>();
    let mut seed = 0x5eaf_3c19_82d4_b671u64;
    let mut calls = 0;
    for sample in 0..256 {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        let initial = if sample % 3 == 0 { u64::MAX } else { seed };
        let salt = seed.rotate_left(23);
        for count in [0u64, 1, 2, 5, 16] {
            for &(call, bits, mode) in &entries {
                let mask = u64::MAX >> (64 - bits);
                let mut expected = [0x9a73_f021_6b4d_c185; 19];
                let mut full = initial;
                let mut low = (initial & mask) ^ u64::from(mode == 1);
                for index in 0..count {
                    let cost = (full & mask).wrapping_add(salt).wrapping_add(index) & mask;
                    let high = if index & 1 != 0 {
                        salt ^ index
                    } else {
                        salt.wrapping_add(index)
                    };
                    let shift = if mode == 3 { salt & 63 } else { 32 };
                    full = cost | high.wrapping_shl(shift as u32);
                    low = if mode == 3 { cost } else { full & mask };
                    if mode == 2 && index & 1 != 0 {
                        low ^= 1;
                    }
                    expected[1 + index as usize] = full;
                }
                expected[17] = full;
                let mut output = [0x9a73_f021_6b4d_c185; 19];
                // SAFETY: count <=16 admits the loop's writes to words0..15;
                // word16 is the final full value. All17 output words are
                // initialized, aligned, exclusive and enclosed by live guards.
                let actual = unsafe { call(initial, salt, count, output.as_mut_ptr().add(1)) };
                assert_eq!(actual, low);
                assert_eq!(output, expected);
                calls += 1;
            }
        }
    }
    assert_eq!(calls, 30_720);
}
