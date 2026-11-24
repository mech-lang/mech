use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Run = unsafe extern "C" fn(u64, *const u64, u64) -> u64;
const APPEND_MODES: [u8; 10] = [0, 1, 2, 3, 4, 5, 12, 13, 28, 29];

fn fixture(module: &JITModule, bias: i64, guard_bias: i64) -> (Context, Block) {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut frontend);
    let [
        entry,
        header,
        cache_bound,
        cache_read,
        cost_read,
        previous_read,
        append,
        latch,
        leave,
    ] = [(); 9].map(|()| b.create_block());
    b.append_block_params_for_function_params(entry);
    for _ in 0..3 {
        b.append_block_param(header, types::I64);
    }
    for _ in 0..2 {
        b.append_block_param(leave, types::I64);
    }
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let [
        cache,
        cache_len,
        costs,
        cost_len,
        output,
        count,
        cursor,
        length,
    ]: [Value; 8] = core::array::from_fn(|i| {
        b.ins()
            .load(types::I64, MemFlags::new(), packet, (i * 8) as i32)
    });
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let zero = b.ins().iconst(types::I64, 0);
    let errors: [Value; 4] = core::array::from_fn(|i| b.ins().iconst(types::I64, (i + 1) as i64));
    b.ins()
        .jump(header, &[count, cursor, length].map(BlockArg::Value));

    b.switch_to_block(header);
    let [count, cursor, length] = b.block_params(header).try_into().unwrap();
    let more = b.ins().icmp_imm(IntCC::NotEqual, count, 0);
    b.ins()
        .brif(more, cache_bound, &[], leave, &[length.into(), zero.into()]);

    b.switch_to_block(cache_bound);
    let ordinal = b.ins().iadd_imm(cursor, -1);
    let readable = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, cache_len);
    b.ins().brif(
        readable,
        cache_read,
        &[],
        leave,
        &[length.into(), errors[1].into()],
    );

    b.switch_to_block(cache_read);
    let offset = b.ins().imul_imm(ordinal, 4);
    let address = b.ins().iadd(cache, offset);
    let word = b.ins().uload32(flags, address, 0);
    let literal = b.ins().ushr_imm(word, 16);
    let index = b.ins().iadd_imm(literal, bias);
    let checked = if guard_bias == bias {
        index
    } else {
        b.ins().iadd_imm(literal, guard_bias)
    };
    let readable = b.ins().icmp(IntCC::UnsignedLessThan, checked, cost_len);
    b.ins().brif(
        readable,
        cost_read,
        &[],
        leave,
        &[length.into(), errors[2].into()],
    );

    b.switch_to_block(cost_read);
    let offset = b.ins().imul_imm(index, 4);
    let address = b.ins().iadd(costs, offset);
    let cost = b.ins().uload32(flags, address, 0);
    let previous_index = b.ins().iadd_imm(length, -1);
    let initialized = b
        .ins()
        .icmp(IntCC::UnsignedLessThan, previous_index, length);
    b.ins().brif(
        initialized,
        previous_read,
        &[],
        leave,
        &[length.into(), errors[3].into()],
    );

    b.switch_to_block(previous_read);
    let offset = b.ins().imul_imm(previous_index, 8);
    let address = b.ins().iadd(output, offset);
    let previous = b.ins().load(types::I64, flags, address, 0);
    let selector = b.ins().band_imm(word, 0xffff);
    let selected_cursor = b.ins().isub(cursor, selector);
    let empty = b.ins().icmp(IntCC::Equal, selected_cursor, cursor);
    b.ins().brif(
        empty,
        append,
        &[],
        leave,
        &[length.into(), errors[0].into()],
    );

    b.switch_to_block(append);
    let previous = b.ins().band_imm(previous, i64::from(u32::MAX));
    let sum = b.ins().iadd(cost, previous);
    let low = b.ins().band_imm(sum, i64::from(u32::MAX));
    let literal = b.ins().band_imm(literal, i64::from(u32::MAX));
    let item = b.ins().ishl_imm(literal, 9);
    let item = b.ins().bor_imm(item, 1);
    let item = b.ins().band_imm(item, i64::from(u32::MAX));
    let high = b.ins().ishl_imm(item, 32);
    let stored = b.ins().bor(low, high);
    let offset = b.ins().imul_imm(length, 8);
    let address = b.ins().iadd(output, offset);
    b.ins().store(flags, stored, address, 0);
    let next_count = b.ins().iadd_imm(count, -1);
    let next_cursor = b.ins().iadd_imm(selected_cursor, -1);
    let next_length = b.ins().iadd_imm(length, 1);
    b.ins().jump(latch, &[]);
    b.switch_to_block(latch);
    b.ins().jump(
        header,
        &[next_count, next_cursor, next_length].map(BlockArg::Value),
    );

    b.switch_to_block(leave);
    let [length, status] = b.block_params(leave).try_into().unwrap();
    let length = b.ins().ishl_imm(length, 32);
    let result = b.ins().bor(length, status);
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    (context, header)
}

fn compile(bias: i64, mode: u8) -> (JITModule, Run) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (mut context, header) = fixture(&module, bias, bias);
    if mode != 0 {
        context.compute_cfg();
        context.compute_domtree();
        let trace = super::super::trace::capture(&context, header).unwrap();
        assert!(trace.window.is_some());
        let params = context.func.dfg.block_params(header).to_vec();
        let pair = (mode >= 2).then(|| {
            let options = mode - 2;
            Pair::derive(
                &context,
                &trace,
                &params,
                Options {
                    allow_biased_costs: true,
                    rebase_costs: options & 1 != 0,
                    check_literal_bytes: options & 2 != 0,
                    limit_literal_span: options & 8 != 0,
                    group_four: options & 16 != 0,
                },
            )
            .unwrap()
        });
        super::super::emit(&mut context.func, header, &params, trace, None, pair);
    }
    context.flowgraph();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("append_pair", Linkage::Export, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    let run = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, run)
}

fn call(
    run: Run,
    cache: &[u32],
    cache_len: u64,
    costs: &[u32],
    count: u64,
    cursor: u64,
    length: u64,
) -> (u64, Vec<u64>) {
    // The caller supplies the same append capacity proved by NoGrowth in the
    // real emitter. Every scalar side exit remains within these actual views.
    let mut output = vec![0xdead_beef_ffff_ffc0; cache.len() + 4];
    let packet = [
        cache.as_ptr() as u64,
        cache_len,
        costs.as_ptr() as u64,
        costs.len() as u64,
        output.as_mut_ptr() as u64,
        count,
        cursor,
        length,
    ];
    let status = unsafe { run(0, packet.as_ptr(), 0) };
    (status, output)
}

#[test]
fn biased_pairs_preserve_wrapping_costs_ordered_failures_and_scalar_tails() {
    let mut calls = 0;
    for bias in [0, 1, 259, -1, -256, i64::MAX] {
        let compiled = APPEND_MODES.map(|mode| compile(bias, mode));
        let costs = (0..547)
            .map(|i| (i as u32).wrapping_mul(0xfedc_ba97).wrapping_add(u32::MAX))
            .collect::<Vec<_>>();
        for n in [0_usize, 1, 2, 8, 9, 10, 11, 12, 17, 19, 20, 21, 32, 63] {
            let floor = if bias < 0 {
                bias.wrapping_neg() as u32
            } else {
                0
            };
            let cache = (0..n)
                .map(|i| ((floor.wrapping_add(i as u32 % 32)) & 0xffff) << 16)
                .collect::<Vec<_>>();
            let mut inputs = vec![cache.clone()];
            for at in [
                0,
                n.saturating_sub(2),
                n.saturating_sub(3),
                n.saturating_sub(4),
                n.saturating_sub(5),
                n.saturating_sub(16),
                n.saturating_sub(17),
                n.saturating_sub(20),
                n.saturating_sub(21),
            ] {
                if at < n {
                    let mut bad = cache.clone();
                    bad[at] = 0xffff_0000;
                    inputs.push(bad);
                    let mut nonliteral = cache.clone();
                    nonliteral[at] |= 1;
                    inputs.push(nonliteral);
                }
            }
            for input in inputs {
                for length in [0, 1] {
                    for cache_len in [n as u64, (n as u64).saturating_sub(1)] {
                        let expected = call(
                            compiled[0].1,
                            &input,
                            cache_len,
                            &costs,
                            n as u64,
                            n as u64,
                            length,
                        );
                        calls += 1;
                        for (_, run) in &compiled[1..] {
                            let actual =
                                call(*run, &input, cache_len, &costs, n as u64, n as u64, length);
                            calls += 1;
                            assert_eq!(
                                actual, expected,
                                "bias {bias}, n {n}, length {length}, cache length {cache_len}"
                            );
                        }
                    }
                }
            }
        }
        let cache = vec![0x0100_0000; 8];
        let expected = call(compiled[0].1, &cache, 8, &costs, u64::MAX, 8, 1);
        calls += 1;
        for (_, run) in &compiled[1..] {
            assert_eq!(call(*run, &cache, 8, &costs, u64::MAX, 8, 1), expected);
            calls += 1;
        }
        if bias == 259 {
            let mut mixed = [0xff_0000; 16];
            mixed[13] = 0x100_0000;
            for cache in [[0; 16], [0xff_0000; 16], mixed] {
                for length in [258, 259, 260, 261, 514, 515, 516, 547] {
                    let costs = &costs[..length];
                    let expected = call(compiled[0].1, &cache, 16, costs, 16, 16, 1);
                    calls += 1;
                    for (_, run) in &compiled[1..] {
                        assert_eq!(call(*run, &cache, 16, costs, 16, 16, 1), expected);
                        calls += 1;
                    }
                }
            }
        }
    }
    eprintln!("biased append differential native calls: {calls}");
}

#[test]
fn biased_pair_recognition_requires_the_exact_cost_index_guard() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for bias in [0, 259, -1, i64::MAX] {
        for guard in [bias, bias.wrapping_add(1)] {
            let (mut context, header) = fixture(&module, bias, guard);
            context.compute_cfg();
            context.compute_domtree();
            let trace = super::super::trace::capture(&context, header).unwrap();
            let params = context.func.dfg.block_params(header);
            let result = Pair::derive(
                &context,
                &trace,
                params,
                Options {
                    allow_biased_costs: true,
                    ..Options::default()
                },
            );
            assert_eq!(result.is_ok(), guard == bias);
            if guard == bias && bias != 0 {
                assert!(matches!(
                    Pair::derive(&context, &trace, params, Options::default()),
                    Err("cost ordinal has an offset")
                ));
            }
        }
    }
}

#[test]
fn biased_pairs_preserve_alias_fallbacks_and_adjacent_views() {
    for bias in [0, 259, -256] {
        let reference = compile(bias, 0);
        let candidates = APPEND_MODES[2..]
            .iter()
            .map(|&mode| compile(bias, mode))
            .collect::<Vec<_>>();
        for layout in 0..4 {
            let execute = |run: Run| {
                let mut storage = vec![0xfeed_cafe_ffff_ffc0_u64; 320];
                let floor = if bias < 0 {
                    bias.wrapping_neg() as u32
                } else {
                    0
                };
                let cache = (0..32).map(|i| (floor + i % 16) << 16).collect::<Vec<_>>();
                let costs = (0..547)
                    .map(|i| (i as u32).wrapping_mul(0x8765_4321))
                    .collect::<Vec<_>>();
                let words = storage.as_mut_ptr().cast::<u32>();
                let mut output = storage.as_mut_ptr();
                let (cache_ptr, cost_ptr) = match layout {
                    0 => {
                        for (i, &word) in cache.iter().enumerate() {
                            unsafe { words.add(i).write(word) };
                        }
                        (words.cast_const(), costs.as_ptr())
                    }
                    1 => {
                        for (i, &word) in costs.iter().enumerate() {
                            unsafe { words.add(i).write(word) };
                        }
                        (cache.as_ptr(), words.cast_const())
                    }
                    2 => {
                        // The output's maximum end exactly touches this cache.
                        for (i, &word) in cache.iter().enumerate() {
                            unsafe { words.add(66 + i).write(word) };
                        }
                        (unsafe { words.add(66) }.cast_const(), costs.as_ptr())
                    }
                    _ => {
                        // The output begins in the unused tail of a cost view.
                        for (i, &word) in costs.iter().enumerate() {
                            unsafe { words.add(i).write(word) };
                        }
                        let prefix = 256 + bias.max(0) as usize;
                        output = unsafe { output.add(prefix.div_ceil(2)) };
                        (cache.as_ptr(), words.cast_const())
                    }
                };
                let packet = [
                    cache_ptr as u64,
                    32,
                    cost_ptr as u64,
                    547,
                    output as u64,
                    32,
                    32,
                    1,
                ];
                let result = unsafe { run(0, packet.as_ptr(), 0) };
                (result, storage)
            };
            let expected = execute(reference.1);
            for (_, candidate) in &candidates {
                assert_eq!(
                    execute(*candidate),
                    expected,
                    "bias {bias}, layout {layout}"
                );
            }
        }
    }
}
