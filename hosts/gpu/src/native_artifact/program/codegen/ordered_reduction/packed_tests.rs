//! Native bounded-key checks use the same production key and guard emitters.
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, Value, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn emit_loop(
    b: &mut FunctionBuilder<'_>,
    input: Value,
    start: Value,
    stop: Value,
    best: Value,
    packed: bool,
    carry: bool,
) -> (Value, Value) {
    let initial = if packed {
        super::packed_key::initial(b, best)
    } else {
        b.ins().splat(types::I32X4, best)
    };
    let missing = b.ins().iconst(types::I32, -1);
    let absent = b.ins().splat(types::I32X4, missing);
    let lanes = if packed {
        super::packed_key::halfwords(b, [1, 2, 3, 4, 0, 0, 0, 0])
    } else {
        super::cost::constant(b, [0, 1, 2, 3])
    };
    let header = b.create_block();
    for ty in [types::I64, types::I32X4] {
        b.append_block_param(header, ty);
    }
    if !packed {
        b.append_block_param(header, types::I32X4);
    }
    if packed && carry {
        b.append_block_param(header, types::I16X8);
    }
    let mut initial_args = vec![start.into(), initial.into()];
    if !packed {
        initial_args.push(absent.into());
    }
    if packed && carry {
        let first = b.ins().ireduce(types::I16, start);
        let first = b.ins().splat(types::I16X8, first);
        initial_args.push(b.ins().iadd(first, lanes).into());
    }
    b.ins().jump(header, &initial_args);
    b.switch_to_block(header);
    let parameters = b.block_params(header).to_vec();
    let k = parameters[0];
    let body = b.create_block();
    let done = b.create_block();
    let more = b.ins().icmp(IntCC::UnsignedLessThan, k, stop);
    b.ins().brif(more, body, &[], done, &[]);
    b.switch_to_block(body);
    let offset = b.ins().isub(k, start);
    let offset = b.ins().ishl_imm(offset, 2);
    let pointer = b.ins().iadd(input, offset);
    let costs = b.ins().load(types::I32X4, MemFlags::new(), pointer, 0);
    let next_values = if packed {
        let index = if carry {
            parameters[2]
        } else {
            let index = b.ins().ireduce(types::I16, k);
            let index = b.ins().splat(types::I16X8, index);
            b.ins().iadd(index, lanes)
        };
        let key = super::packed_key::key(b, costs, index);
        vec![b.ins().umin(key, parameters[1])]
    } else {
        let index = b.ins().ireduce(types::I32, k);
        let index = b.ins().splat(types::I32X4, index);
        let index = b.ins().iadd(index, lanes);
        let better = b.ins().icmp(IntCC::UnsignedLessThan, costs, parameters[1]);
        let cost = b.ins().umin(costs, parameters[1]);
        let index = b.ins().bitselect(better, index, parameters[2]);
        vec![cost, index]
    };
    let next = b.ins().iadd_imm(k, 4);
    let mut args = vec![next.into()];
    for value in next_values {
        args.push(value.into());
    }
    if packed && carry {
        let step = super::packed_key::halfwords(b, [4; 8]);
        args.push(b.ins().iadd(parameters[2], step).into());
    }
    b.ins().jump(header, &args);
    b.switch_to_block(done);
    let (cost, winner) = if packed {
        super::packed_key::finish(b, parameters[1])
    } else {
        let cost = super::emit::minimum_word(b, parameters[1]);
        let equal_cost = b.ins().splat(types::I32X4, cost);
        let equal = b.ins().icmp(IntCC::Equal, parameters[1], equal_cost);
        let positions = b.ins().bitselect(equal, parameters[2], absent);
        let winner = super::emit::minimum_word(b, positions);
        let winners = b.ins().splat(types::I32X4, winner);
        let cost = b.ins().extractlane(equal_cost, 0);
        let winner = b.ins().extractlane(winners, 0);
        (cost, winner)
    };

    (cost, winner)
}

#[test]
fn bounded_cost_ordinal_key_preserves_strict_ties_unsigned_costs_and_wide_fallback() {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut functions = Vec::new();
    for mode in 0..12 {
        let mut context = module.make_context();
        context.func.signature.params = vec![AbiParam::new(types::I64); 3];
        context.func.signature.returns = vec![AbiParam::new(types::I32)];
        let mut builder = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let [input, metadata, output] = b.block_params(entry)[..] else {
            unreachable!()
        };
        let count = b.ins().load(types::I64, MemFlags::new(), metadata, 0);
        let start = b.ins().load(types::I64, MemFlags::new(), metadata, 8);
        let best = b.ins().load(types::I32, MemFlags::new(), metadata, 16);
        let stop = b.ins().iadd(start, count);
        let done = b.create_block();
        for _ in 0..3 {
            b.append_block_param(done, types::I32);
        }
        if mode == 0 {
            let (cost, winner) = emit_loop(&mut b, input, start, stop, best, false, false);
            let route = b.ins().iconst(types::I32, 0);
            b.ins()
                .jump(done, &[cost.into(), winner.into(), route.into()]);
        } else {
            let mut fits = super::packed_key::fits(&mut b, best, stop, mode == 3);
            if mode >= 4 {
                let long = b
                    .ins()
                    .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, count, 16);
                fits = b.ins().band(fits, long);
            }
            let narrow = b.create_block();
            let wide = b.create_block();
            b.ins().brif(fits, narrow, &[], wide, &[]);
            for (block, packed) in [(wide, false), (narrow, true)] {
                b.switch_to_block(block);
                let (cost, winner) = if packed && mode >= 4 {
                    let keys = super::packed_loop::emit(
                        &mut b,
                        start,
                        stop,
                        best,
                        mode % 4 >= 2,
                        mode % 2 != 0,
                        mode >= 8,
                        |b, k, _| {
                            let offset = b.ins().isub(k, start);
                            let offset = b.ins().ishl_imm(offset, 2);
                            let pointer = b.ins().iadd(input, offset);
                            b.ins().load(types::I32X4, MemFlags::new(), pointer, 0)
                        },
                    );
                    super::packed_key::finish(&mut b, keys)
                } else {
                    emit_loop(&mut b, input, start, stop, best, packed, mode == 2)
                };
                let route = b.ins().iconst(types::I32, i64::from(packed));
                b.ins()
                    .jump(done, &[cost.into(), winner.into(), route.into()]);
            }
        }
        b.switch_to_block(done);
        let result = b.block_params(done).to_vec();
        for (at, &value) in result.iter().enumerate() {
            b.ins()
                .store(MemFlags::new(), value, output, (at * 4) as i32);
        }
        b.ins().return_(&[result[1]]);
        b.seal_all_blocks();
        b.finalize();
        let id = module
            .declare_function(
                &format!("bounded_key_{mode}"),
                Linkage::Local,
                &context.func.signature,
            )
            .unwrap();
        module.define_function(id, &mut context).unwrap();
        functions.push(id);
    }
    module.finalize_definitions().unwrap();
    type Entry = unsafe extern "C" fn(*const u32, *const u64, *mut u32) -> u32;
    let calls = functions
        .into_iter()
        .map(|id| {
            // SAFETY: the live finalized module owns each exact declared C entry.
            unsafe { std::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
        })
        .collect::<Vec<_>>();
    let mut seed = 0x8a72_4ef1_d63b_c905u64;
    let mut native_calls = 0;
    for sample in 0..8192usize {
        let mut next = || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed as u32
        };
        let count = [0, 4, 8, 16, 20, 24, 28, 32, 64, 128, 252, 256, 508, 512][sample % 14];
        let start = match sample % 8 {
            0 => 0,
            1 => 3,
            2 => 32767,
            3 => 65535 - count,
            4 => 65536 - count,
            5 => 65537 - count,
            6 => 0x8000_0000usize,
            _ => u32::MAX as usize - count,
        };
        let best = [
            0,
            1,
            7,
            32767,
            65534,
            65535,
            65536,
            0x8000_0000,
            u32::MAX,
            next(),
        ][sample % 10];
        let mut storage = vec![0xdead_beefu32; count + 2];
        for (at, value) in storage[1..count + 1].iter_mut().enumerate() {
            *value = match sample % 6 {
                0 => best,
                1 => best.saturating_sub(1),
                2 => u32::MAX,
                _ => {
                    if at % 3 == 0 {
                        next() & 65535
                    } else {
                        next()
                    }
                }
            };
        }
        let mut minimum = best;
        let mut winner = u32::MAX;
        for (at, &cost) in storage[1..count + 1].iter().enumerate() {
            if cost < minimum {
                minimum = cost;
                winner = (start + at) as u32;
            }
        }
        let metadata = [count as u64, start as u64, u64::from(best)];
        let original = storage.clone();
        for (mode, call) in calls.iter().enumerate() {
            // The fourth function models a static proof of stop <= 65535.
            if mode == 3 && start + count > 65535 {
                continue;
            }
            let route = u32::from(
                mode != 0 && best <= 65535 && start + count <= 65535 && (mode < 4 || count >= 16),
            );
            let mut output = [0xa5a5_a5a5u32; 5];
            // SAFETY: count is a multiple of four, both input extents are
            // initialized, output is disjoint and guarded, and stop <= U32::MAX.
            // The guarded route admits the narrower key only within its bounds.
            let actual = unsafe {
                call(
                    storage.as_ptr().add(1),
                    metadata.as_ptr(),
                    output.as_mut_ptr().add(1),
                )
            };
            native_calls += 1;
            assert_eq!(actual, winner, "mode {mode}, sample {sample}");
            assert_eq!(output, [0xa5a5_a5a5, minimum, winner, route, 0xa5a5_a5a5]);
            assert_eq!(storage, original);
        }
    }
    assert_eq!(native_calls, 94208);
}
