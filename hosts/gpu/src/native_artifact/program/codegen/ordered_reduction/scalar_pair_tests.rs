//! Exercise the production paired control flow with guarded unaligned reads.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn paired_scalar_visits_preserve_unsigned_wrap_ties_full_items_and_odd_tails() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for stride in [4i64, 8] {
        for deferred in [false, true] {
            for sequential in [0u8, 1, 2] {
                let mut context = module.make_context();
                context.func.signature.params = vec![AbiParam::new(types::I64); 4];
                context.func.signature.returns = vec![AbiParam::new(types::I64)];
                let mut builder = FunctionBuilderContext::new();
                let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
                let entry = b.create_block();
                b.append_block_params_for_function_params(entry);
                b.switch_to_block(entry);
                let [front, back, state, output] = b.block_params(entry)[..] else {
                    unreachable!()
                };
                let flags = MemFlags::new();
                let index = b.ins().load(types::I64, flags, state, 0);
                let count = b.ins().load(types::I64, flags, state, 8);
                let best = b.ins().load(types::I32, flags, state, 16);
                let old = b.ins().load(types::I64, flags, state, 24);
                let offset = b.ins().load(types::I32, flags, state, 32);
                let addition = b.ins().load(types::I32, flags, state, 40);
                let end = b.ins().iadd_imm(back, 64 * stride);
                let bytes = b.ins().imul_imm(index, stride);
                let previous = b.ins().isub(end, bytes);
                let pair = b.create_block();
                let scalar = b.create_block();
                let finish = b.create_block();
                for block in [pair, scalar] {
                    for ty in [types::I64, types::I32, types::I64, types::I64, types::I64] {
                        b.append_block_param(block, ty);
                    }
                }
                b.append_block_param(finish, types::I32);
                b.append_block_param(finish, types::I64);
                b.ins().jump(
                    pair,
                    &[
                        index.into(),
                        best.into(),
                        old.into(),
                        previous.into(),
                        count.into(),
                    ],
                );
                let cost = |b: &mut FunctionBuilder<'_>, index: Value, previous: Value| {
                    let bytes = b.ins().ishl_imm(index, 2);
                    let address = b.ins().iadd(front, bytes);
                    let a = b.ins().load(types::I32, flags, address, 0);
                    let previous = b.ins().iadd_imm(previous, -stride);
                    let d = b.ins().load(types::I32, flags, previous, 0);
                    let sum = b.ins().iadd(a, d);
                    (b.ins().iadd(sum, addition), previous)
                };
                let item = |b: &mut FunctionBuilder<'_>, index| {
                    if deferred {
                        index
                    } else {
                        let index = b.ins().ireduce(types::I32, index);
                        let low = b.ins().bor(index, offset);
                        b.ins().uextend(types::I64, low)
                    }
                };
                if sequential != 0 {
                    super::scalar_pair::emit_sequential(
                        &mut b,
                        pair,
                        scalar,
                        finish,
                        |b, index, previous, best, old| {
                            let (value, previous) = cost(b, index, previous);
                            let better = b.ins().icmp(IntCC::UnsignedLessThan, value, best);
                            let minimum = b.ins().select(better, value, best);
                            let selected = item(b, index);
                            let choice = b.ins().select(better, selected, old);
                            (minimum, choice, previous)
                        },
                    );
                } else {
                    super::scalar_pair::emit(
                        &mut b,
                        pair,
                        scalar,
                        finish,
                        |b, index, previous| {
                            let (first, previous) = cost(b, index, previous);
                            let next = b.ins().iadd_imm(index, 1);
                            let (second, previous) = cost(b, next, previous);
                            (first, second, previous)
                        },
                        item,
                    );
                }
                b.switch_to_block(scalar);
                let args = b.block_params(scalar).to_vec();
                let (value, previous) = cost(&mut b, args[0], args[3]);
                let better = b.ins().icmp(IntCC::UnsignedLessThan, value, args[1]);
                let minimum = b.ins().select(better, value, args[1]);
                let selected = item(&mut b, args[0]);
                let choice = b.ins().select(better, selected, args[2]);
                if sequential == 2 {
                    b.ins().jump(finish, &[minimum.into(), choice.into()]);
                } else {
                    let index = b.ins().iadd_imm(args[0], 1);
                    let left = b.ins().iadd_imm(args[4], -1);
                    let more = b.ins().icmp_imm(IntCC::NotEqual, left, 0);
                    b.ins().brif(
                        more,
                        scalar,
                        &[
                            index.into(),
                            minimum.into(),
                            choice.into(),
                            previous.into(),
                            left.into(),
                        ],
                        finish,
                        &[minimum.into(), choice.into()],
                    );
                }
                b.switch_to_block(finish);
                let values = b.block_params(finish).to_vec();
                let minimum = b.ins().uextend(types::I64, values[0]);
                b.ins().store(flags, minimum, output, 0);
                b.ins().store(flags, values[1], output, 8);
                b.ins().return_(&[values[1]]);
                b.seal_all_blocks();
                b.finalize();
                let id = module
                    .declare_function(
                        &format!("paired_scalar_{stride}_{deferred}_{sequential}"),
                        Linkage::Local,
                        &context.func.signature,
                    )
                    .unwrap();
                module.define_function(id, &mut context).unwrap();
                functions.push((id, stride as usize, deferred));
            }
        }
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u8, *const u8, *const u64, *mut u64) -> u64;
    let calls = functions
        .into_iter()
        .map(|(id, stride, deferred)| {
            // SAFETY: the live finalized module owns this exact C signature.
            let call = unsafe {
                std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id))
            };
            (call, stride, deferred)
        })
        .collect::<Vec<_>>();
    let mut seed = 0x806d_351a_ab12_7f98u64;
    for sample in 0..128 {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        let mut front = std::array::from_fn::<_, 64, _>(|_| next() as u32);
        let mut back = std::array::from_fn::<_, 64, _>(|_| next() as u32);
        let mut addition = next() as u32;
        let mut best = next() as u32;
        match sample % 6 {
            0 => {
                front.fill(u32::MAX);
                back.fill(0);
                addition = 0;
                best = u32::MAX;
            }
            1 => {
                front.fill(0);
                back.fill(0);
                addition = 0;
                best = 0;
            }
            2 => {
                front.fill(7);
                back.fill(0);
                addition = 0;
                best = 8;
            }
            3 => {
                front = std::array::from_fn(|i| [9, 3, 3, 4][i % 4]);
                back.fill(0);
                addition = 0;
                best = 10;
            }
            4 => {
                front.fill(u32::MAX - 3);
                back.fill(10);
                addition = u32::MAX - 2;
            }
            _ => {}
        }
        let old = next();
        let offset = next() as u32;
        for start in 0..4 {
            for count in [1, 2, 3, 4, 7, 8, 9, 15, 16, 17, 31] {
                for &(call, stride, deferred) in &calls {
                    let mut a = vec![0xa5; 64 * 4 + 2];
                    let mut d = vec![0x5a; 64 * stride + 2];
                    for i in 0..64 {
                        a[1 + i * 4..5 + i * 4].copy_from_slice(&front[i].to_le_bytes());
                        d[1 + i * stride..5 + i * stride].copy_from_slice(&back[i].to_le_bytes());
                    }
                    let before = (a.clone(), d.clone());
                    let state = [
                        start as u64,
                        count as u64,
                        u64::from(best),
                        old,
                        u64::from(offset),
                        u64::from(addition),
                    ];
                    let mut minimum = best;
                    let mut expected = old;
                    for index in start..start + count {
                        let cost = front[index]
                            .wrapping_add(back[63 - index])
                            .wrapping_add(addition);
                        if cost < minimum {
                            minimum = cost;
                            expected = if deferred {
                                index as u64
                            } else {
                                u64::from(index as u32 | offset)
                            };
                        }
                    }
                    let guard = 0xc3c3_c3c3_c3c3_c3c3;
                    let mut output = [guard; 5];
                    // SAFETY: nonzero count and start admit every front and
                    // reverse low-word read within the initialized unaligned
                    // arrays. State is immutable and the guarded two-word
                    // output is separate, fixed and exclusive for this call.
                    let actual = unsafe {
                        call(
                            a.as_ptr().add(1),
                            d.as_ptr().add(1),
                            state.as_ptr(),
                            output.as_mut_ptr().add(1),
                        )
                    };
                    assert_eq!(
                        actual, expected,
                        "sample {sample}, start {start}, count {count}, stride {stride}, deferred {deferred}"
                    );
                    assert_eq!(output, [guard, u64::from(minimum), expected, guard, guard]);
                    assert_eq!((a, d), before);
                }
            }
        }
    }
}
