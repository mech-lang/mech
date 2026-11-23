//! Execute the complete admitted entry, including scalar fallback and errors.
use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Run = unsafe extern "C" fn(u64, *const u64, *mut u64) -> u64;
type Fixture = (
    Context,
    Block,
    BTreeMap<Value, (Inst, usize)>,
    Vec<View>,
    Vec<View>,
);

fn fixture(module: &JITModule, shift: u32, key_mask: u32) -> Fixture {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 3];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64); 2];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
    let [
        entry,
        header,
        body,
        yes,
        no,
        join,
        selected,
        update,
        skip,
        done,
        error,
    ] = [(); 11].map(|()| b.create_block());
    b.append_block_params_for_function_params(entry);
    let at = b.append_block_param(header, types::I64);
    let raw = b.append_block_param(join, types::I64);
    b.switch_to_block(entry);
    let packet = b.block_params(entry)[1];
    let mut loads = BTreeMap::new();
    let fields: [Value; 7] = core::array::from_fn(|word| {
        let value = b
            .ins()
            .load(types::I64, MemFlags::new(), packet, (word * 8) as i32);
        loads.insert(value, (b.func.dfg.value_def(value).unwrap_inst(), word));
        value
    });
    let [nodes, node_len, bins, bin_len, top, end, start] = fields;
    b.ins().jump(header, &[start.into()]);
    b.switch_to_block(header);
    let finished = b.ins().icmp(IntCC::Equal, at, end);
    b.ins().brif(finished, done, &[], body, &[]);
    b.switch_to_block(body);
    let ordinal = b.ins().isub(top, at);
    let readable = b.ins().icmp(IntCC::UnsignedLessThan, ordinal, node_len);
    b.ins().brif(readable, yes, &[], no, &[]);
    b.switch_to_block(yes);
    let offset = b.ins().ishl_imm(ordinal, 3);
    let address = b.ins().iadd(nodes, offset);
    let word = b.ins().load(types::I64, MemFlags::new(), address, 0);
    b.ins().jump(join, &[word.into()]);
    b.switch_to_block(no);
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(join, &[zero.into()]);
    b.switch_to_block(join);
    let high = b.ins().ushr_imm(raw, 32);
    let packed = b.ins().band_imm(high, u32::MAX as i64);
    let length = b.ins().band_imm(packed, 0x1ff);
    let literal = b.ins().icmp_imm(IntCC::Equal, length, 1);
    b.ins().brif(literal, selected, &[], skip, &[]);
    b.switch_to_block(selected);
    let shifted = b.ins().ushr_imm(packed, i64::from(shift));
    let key = b.ins().band_imm(shifted, i64::from(key_mask));
    let fits = b.ins().icmp(IntCC::UnsignedLessThan, key, bin_len);
    b.ins().brif(fits, update, &[], error, &[]);
    b.switch_to_block(update);
    let offset = b.ins().ishl_imm(key, 2);
    let address = b.ins().iadd(bins, offset);
    let count = b.ins().uload32(MemFlags::new(), address, 0);
    let sum = b.ins().iadd_imm(count, 1);
    let wrapped = b.ins().band_imm(sum, u32::MAX as i64);
    let value = b.ins().ireduce(types::I32, wrapped);
    b.ins().store(MemFlags::new(), value, address, 0);
    let next = b.ins().iadd_imm(at, 1);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(skip);
    let next = b.ins().iadd_imm(at, 1);
    b.ins().jump(header, &[next.into()]);
    for (block, status) in [(done, 0), (error, 1)] {
        b.switch_to_block(block);
        let status = b.ins().iconst(types::I64, status);
        b.ins().return_(&[status, at]);
    }
    b.seal_all_blocks();
    b.finalize();
    let view = |array, pointer_word, width| View {
        array,
        pointer_word,
        length_word: pointer_word + 1,
        width,
        literal: None,
    };
    (ctx, header, loads, vec![view(0, 0, 8)], vec![view(1, 2, 4)])
}

fn compile(shift: u32, key_mask: u32, tuning: Option<Options>) -> (JITModule, Run) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (mut ctx, header, loads, nodes, bins) = fixture(&module, shift, key_mask);
    if let Some(options) = tuning {
        let mut fast = module.make_context();
        fast.func = ctx.func.clone();
        let admission = optimize(&mut fast, header, &loads, &nodes, &bins, options)
            .expect("recognized descending packed-node histogram");
        assert_eq!(
            admission,
            Admission {
                node_word: 0,
                bin_word: 2,
                minimum_bins: u64::from(key_mask) + 1,
            }
        );
        fast.verify(module.isa()).unwrap();
        let id = module
            .declare_function("grouped", Linkage::Local, &fast.func.signature)
            .unwrap();
        module.define_function(id, &mut fast).unwrap();
        let call = module.declare_func_in_func(id, &mut ctx.func);
        guard_entry(&mut ctx, admission, call);
    }
    ctx.verify(module.isa()).unwrap();
    let id = module
        .declare_function("phase", Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    // Adapt two native return words explicitly, keeping the Rust ABI scalar.
    let mut wrapper = module.make_context();
    wrapper.func.signature.params = vec![AbiParam::new(types::I64); 3];
    wrapper.func.signature.returns = vec![AbiParam::new(types::I64)];
    let call = module.declare_func_in_func(id, &mut wrapper.func);
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut wrapper.func, &mut fc);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let called = b.ins().call(call, &args);
    let result = b.inst_results(called).to_vec();
    b.ins().store(MemFlags::new(), result[1], args[2], 0);
    b.ins().return_(&[result[0]]);
    b.seal_all_blocks();
    b.finalize();
    let id = module
        .declare_function("entry", Linkage::Local, &wrapper.func.signature)
        .unwrap();
    module.define_function(id, &mut wrapper).unwrap();
    module.finalize_definitions().unwrap();
    // The wrapper's declared three-word ABI returns one word; the module stays live.
    let run = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, run)
}

fn oracle(
    nodes: &[u64],
    bins: &mut [u32],
    top: u64,
    end: u64,
    mut at: u64,
    shift: u32,
    key_mask: u32,
) -> (u64, u64) {
    while at != end {
        let packed = nodes
            .get(top.wrapping_sub(at) as usize)
            .copied()
            .unwrap_or(0)
            >> 32;
        if packed & 0x1ff == 1 {
            let key = ((packed >> shift) & u64::from(key_mask)) as usize;
            let Some(count) = bins.get_mut(key) else {
                return (1, at);
            };
            *count = count.wrapping_add(1);
        }
        at = at.wrapping_add(1);
    }
    (0, at)
}

fn variants() -> impl Iterator<Item = Options> {
    [4, 8, 16].into_iter().flat_map(|visits| {
        [false, true].into_iter().flat_map(move |wide_keys| {
            [
                (false, false, false, false, false),
                (true, false, false, false, false),
                (true, true, false, false, false),
                (true, false, true, false, false),
                (true, true, true, false, false),
                (true, true, true, true, false),
                (true, true, true, false, true),
                (true, true, true, true, true),
            ]
            .into_iter()
            .filter(move |&(_, vector, stream, _, _)| {
                (!vector || cfg!(target_arch = "aarch64")) && (visits > 4 || !(vector || stream))
            })
            .map(
                move |(
                    progressive,
                    vector_chunks,
                    stream_chunks,
                    byte_keys,
                    pair_vector_chunks,
                )| {
                    Options {
                        visits,
                        wide_keys,
                        progressive,
                        vector_chunks,
                        stream_chunks,
                        byte_keys,
                        pair_vector_chunks,
                    }
                },
            )
        })
    })
}

#[test]
fn grouped_histogram_batches_preserve_tails_full_words_collisions_wrap_and_errors() {
    let mut calls = 0;
    for shift in [0, 9, 31] {
        let entries: Vec<_> = std::iter::once(None)
            .chain(variants().map(Some))
            .map(|tuning| compile(shift, 255, tuning))
            .collect();
        for pattern in 0..4 {
            let nodes: Vec<_> = (0..96u64)
                .map(|i| {
                    let key = if pattern == 0 {
                        17
                    } else {
                        (i * 197 + 3) & 255
                    };
                    let tag = if pattern == 2 && i % 7 == 0 { i % 3 } else { 1 };
                    let high = (0xff00_0000 | (key << 9) | tag) << 32;
                    high | (i.wrapping_mul(0xabcdef01) & u32::MAX as u64)
                })
                .collect();
            for len in [0, 1, 3, 4, 7, 8, 15, 16, 17, 31, 64, 96] {
                for (start, end) in [
                    (0u64, 0u64),
                    (0, 1),
                    (0, 3),
                    (0, 4),
                    (0, 7),
                    (0, 8),
                    (0, 15),
                    (0, 16),
                    (0, 17),
                    (0, 80),
                    (2, 39),
                    (u64::MAX - 7, 9),
                    (u64::MAX - 20, u64::MAX),
                ] {
                    for top in [0, 3, 15, 63, 95, u64::MAX] {
                        for cells in [0, 1, 17, 255, 256] {
                            let seed = if pattern == 3 { u32::MAX - 1 } else { 0 };
                            let mut expected = vec![seed; cells];
                            let expected_status =
                                oracle(&nodes[..len], &mut expected, top, end, start, shift, 255);
                            for (_, run) in &entries {
                                let guard = 0xabcdef01;
                                let mut actual = vec![seed; cells + 2];
                                actual[0] = guard;
                                actual[cells + 1] = guard;
                                let packet = [
                                    nodes.as_ptr() as u64,
                                    len as u64,
                                    actual[1..].as_mut_ptr() as u64,
                                    cells as u64,
                                    top,
                                    end,
                                    start,
                                ];
                                let mut observed_at = 0;
                                // The packet describes live initialized buffers and their exact logical lengths.
                                let status = unsafe { run(0, packet.as_ptr(), &mut observed_at) };
                                assert_eq!((status, observed_at), expected_status);
                                assert_eq!(&actual[1..cells + 1], &expected);
                                assert_eq!((actual[0], actual[cells + 1]), (guard, guard));
                                calls += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    eprintln!("grouped histogram differential native calls: {calls}");
}

#[test]
fn grouped_histogram_declines_unsupported_group_widths_without_mutation() {
    let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    for visits in [0, 1, 2, 3, 5, 32, usize::MAX] {
        let (mut ctx, header, loads, nodes, bins) = fixture(&module, 9, 255);
        let before = ctx.func.display().to_string();
        assert_eq!(
            optimize(
                &mut ctx,
                header,
                &loads,
                &nodes,
                &bins,
                Options {
                    visits,
                    wide_keys: true,
                    progressive: false,
                    vector_chunks: false,
                    stream_chunks: false,
                    byte_keys: false,
                    pair_vector_chunks: false,
                }
            ),
            None
        );
        assert_eq!(ctx.func.display().to_string(), before);
    }
}

#[cfg(all(target_arch = "aarch64", target_endian = "little"))]
#[test]
fn grouped_histogram_byte_keys_preserve_full_vector_keys_when_mask_is_wider() {
    let mask = 511;
    let (_module, run) = compile(
        9,
        mask,
        Some(Options {
            visits: 8,
            wide_keys: false,
            progressive: true,
            vector_chunks: true,
            stream_chunks: true,
            byte_keys: true,
            pair_vector_chunks: true,
        }),
    );
    let nodes: Vec<_> = (0..80u64)
        .rev()
        .map(|visit| {
            let key = 256 + ((visit * 197 + 3) & 255);
            let tag = if visit == 10 { 2 } else { 1 };
            ((key << 9) | tag) << 32
        })
        .collect();
    for end in [11, 12, 17, 20, 21, 79, 80] {
        let mut expected = [0u32; 512];
        let status = oracle(&nodes, &mut expected, 79, end, 0, 9, mask);
        let mut actual = [0u32; 512];
        let packet = [
            nodes.as_ptr() as u64,
            nodes.len() as u64,
            actual.as_mut_ptr() as u64,
            actual.len() as u64,
            79,
            end,
            0,
        ];
        let mut at = 0;
        // Both views are fully initialized and disjoint.
        let code = unsafe { run(0, packet.as_ptr(), &mut at) };
        assert_eq!((code, at), status);
        assert_eq!(actual, expected);
    }
}

#[test]
fn grouped_histogram_current_entry_preserves_overlapping_node_bin_observations() {
    let entries: Vec<_> = variants()
        .map(|options| compile(9, 255, Some(options)))
        .collect();
    let mut calls = 0;
    for bin_base in [0, 1, 96, 191, 192] {
        for end in [1, 4, 8, 16, 31, 64, 96] {
            let mut expected = vec![0u32; 512];
            for node in 0..96 {
                expected[2 * node] = (node as u32).wrapping_mul(0xabcdef01);
                expected[2 * node + 1] = 1 | ((((node * 197 + 3) & 255) as u32) << 9);
            }
            let initial = expected.clone();
            for at in 0..end {
                // Reads see all preceding bin writes, including changes to node tags and keys.
                let packed = expected[2 * (95 - at) + 1];
                if packed & 0x1ff == 1 {
                    let bin = bin_base + ((packed >> 9) & 255) as usize;
                    expected[bin] = expected[bin].wrapping_add(1);
                }
            }
            for (_, run) in &entries {
                let mut actual = initial.clone();
                let packet = [
                    actual.as_ptr() as u64,
                    96,
                    actual[bin_base..].as_mut_ptr() as u64,
                    256,
                    95,
                    end as u64,
                    0,
                ];
                let mut observed_at = 0;
                // One live initialized allocation intentionally backs both views; entry must fall back on overlap.
                let status = unsafe { run(0, packet.as_ptr(), &mut observed_at) };
                assert_eq!((status, observed_at), (0, end as u64));
                assert_eq!(actual, expected);
                calls += 1;
            }
        }
    }
    eprintln!("grouped histogram alias-entry native calls: {calls}");
}
