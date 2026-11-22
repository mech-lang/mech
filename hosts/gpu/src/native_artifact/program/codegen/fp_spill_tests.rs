#![cfg(all(target_arch = "aarch64", target_os = "macos"))]
use cranelift_codegen::ir::condcodes::IntCC;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{FuncId, Linkage, Module};

const SALT: u64 = 0x1e35a7bd;
const STEP: u64 = 0x9e3779b97f4a7c15;
const ERROR: u64 = 0xfeed;

extern "C" fn clobber_callback(seed: u64) -> u64 {
    // SAFETY: all written vector registers are caller-saved under the native
    // C ABI. There are no memory accesses, stack changes or flag changes.
    unsafe {
        std::arch::asm!(
            "movi v0.16b, #0",
            "movi v1.16b, #0",
            "movi v2.16b, #0",
            "movi v3.16b, #0",
            "movi v4.16b, #0",
            "movi v5.16b, #0",
            "movi v6.16b, #0",
            "movi v7.16b, #0",
            "movi v16.16b, #0",
            "movi v17.16b, #0",
            "movi v18.16b, #0",
            "movi v19.16b, #0",
            "movi v20.16b, #0",
            "movi v21.16b, #0",
            "movi v22.16b, #0",
            "movi v23.16b, #0",
            "movi v24.16b, #0",
            "movi v25.16b, #0",
            "movi v26.16b, #0",
            "movi v27.16b, #0",
            "movi v28.16b, #0",
            "movi v29.16b, #0",
            "movi v30.16b, #0",
            "movi v31.16b, #0",
            clobber_abi("C"),
            options(nomem, nostack, preserves_flags),
        );
    }
    seed.rotate_left(13).wrapping_add(STEP)
}

#[derive(Clone, Copy, Debug)]
struct Shape {
    integers: usize,
    floats: usize,
    call: bool,
}

fn compile(
    module: &mut JITModule,
    shape: Shape,
    slots: u8,
    forward: bool,
    keep_pairs: bool,
    name: &str,
) -> (FuncId, Vec<u8>) {
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 5];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    context.func.forward_spill_reloads = forward;
    context.func.aarch64_fp_spill_slots = slots;
    context.func.aarch64_fp_spill_keep_pairs = keep_pairs;
    context.set_disasm(true);
    let mut state = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut context.func, &mut state);
    let entry = b.create_block();
    let failed = b.create_block();
    let ready = b.create_block();
    let header = b.create_block();
    let body = b.create_block();
    let left = b.create_block();
    let right = b.create_block();
    let join = b.create_block();
    let done = b.create_block();
    let types = (0..shape.integers)
        .map(|_| types::I64)
        .chain((0..shape.floats).map(|_| types::F64))
        .collect::<Vec<_>>();
    for ty in [types::I64, types::I64]
        .into_iter()
        .chain(types.iter().copied())
    {
        b.append_block_param(header, ty);
    }
    for &ty in &types {
        b.append_block_param(join, ty);
    }
    b.append_block_param(done, types::I64);
    for &ty in &types {
        b.append_block_param(done, ty);
    }
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let [input, output, rounds, seed, callback] = b.block_params(entry).try_into().unwrap();
    let allowed = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, rounds, 4);
    let valid_in = b.ins().icmp_imm(IntCC::NotEqual, input, 0);
    let valid_out = b.ins().icmp_imm(IntCC::NotEqual, output, 0);
    let pointers = b.ins().band(valid_in, valid_out);
    let allowed = b.ins().band(allowed, pointers);
    b.ins().brif(allowed, ready, &[], failed, &[]);
    b.switch_to_block(failed);
    let error = b.ins().iconst(types::I64, ERROR as i64);
    b.ins().return_(&[error]);
    b.switch_to_block(ready);
    let mut initial = Vec::new();
    for (i, &ty) in types.iter().enumerate() {
        initial.push(b.ins().load(ty, MemFlags::new(), input, (i * 8) as i32));
    }
    let zero = b.ins().iconst(types::I64, 0);
    let arguments = [zero, seed]
        .into_iter()
        .chain(initial)
        .map(Into::into)
        .collect::<Vec<_>>();
    b.ins().jump(header, &arguments);
    b.switch_to_block(header);
    let values = b.block_params(header).to_vec();
    let counter = values[0];
    let current_seed = values[1];
    let more = b.ins().icmp(IntCC::UnsignedLessThan, counter, rounds);
    let finished = values[1..]
        .iter()
        .copied()
        .map(Into::into)
        .collect::<Vec<_>>();
    b.ins().brif(more, body, &[], done, &finished);
    b.switch_to_block(body);
    let next_seed = if shape.call {
        let mut signature = module.make_signature();
        signature.params.push(AbiParam::new(types::I64));
        signature.returns.push(AbiParam::new(types::I64));
        let sig = b.import_signature(signature);
        let call = b.ins().call_indirect(sig, callback, &[current_seed]);
        b.inst_results(call)[0]
    } else {
        current_seed
    };
    let parity = b.ins().bxor(counter, next_seed);
    let parity = b.ins().band_imm(parity, 1);
    b.ins().brif(parity, left, &[], right, &[]);
    for (block, is_left) in [(left, true), (right, false)] {
        b.switch_to_block(block);
        let mut next = Vec::new();
        for (i, &value) in values[2..].iter().enumerate() {
            let value = if i < shape.integers {
                let k = (i % 31 + 1) as i64;
                let (a, z) = if is_left {
                    (b.ins().ishl_imm(value, k), b.ins().ushr_imm(value, 64 - k))
                } else {
                    (b.ins().ushr_imm(value, k), b.ins().ishl_imm(value, 64 - k))
                };
                let rotated = b.ins().bor(a, z);
                let salt = b.ins().iadd_imm(next_seed, ((i as u64 + 1) * SALT) as i64);
                if is_left {
                    b.ins().iadd(rotated, salt)
                } else {
                    b.ins().bxor(rotated, salt)
                }
            } else {
                let step = b.ins().f64const(if is_left { 0.5 } else { -0.25 });
                b.ins().fadd(value, step)
            };
            next.push(value.into());
        }
        b.ins().jump(join, &next);
    }
    b.switch_to_block(join);
    let updated = b.block_params(join).to_vec();
    let incremented = b.ins().iadd_imm(counter, 1);
    let back = [incremented, next_seed]
        .into_iter()
        .chain(updated)
        .map(Into::into)
        .collect::<Vec<_>>();
    b.ins().jump(header, &back);
    b.switch_to_block(done);
    let result = b.block_params(done).to_vec();
    let mut checksum = result[0];
    for (i, &value) in result[1..].iter().enumerate() {
        b.ins()
            .store(MemFlags::new(), value, output, (i * 8) as i32);
        if i < shape.integers {
            checksum = b.ins().bxor(checksum, value);
        }
    }
    b.ins().return_(&[checksum]);
    b.seal_all_blocks();
    b.finalize();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function(name, Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    let bytes = context.compiled_code().unwrap().buffer.data().to_vec();
    (id, bytes)
}

#[test]
fn fp_spills_preserve_bits_loops_aliases_guards_and_call_clobbers() {
    let mut seeds = vec![
        0,
        1,
        u64::MAX,
        1 << 63,
        0x7ff0000000000001,
        0x7ff80000deadbeef,
        0xfff0000000000001,
        0x0123456789abcdef,
    ];
    let mut random = 0x243f6a8885a308d3u64;
    while seeds.len() < 256 {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        seeds.push(random);
    }
    let mut calls = 0;
    let mut changed = 0;
    let mut exact_fallbacks = 0;
    let mut records = Vec::new();
    for (opt, keep_pairs) in ["none", "speed"].into_iter().flat_map(|opt| {
        [false, true]
            .into_iter()
            .map(move |keep_pairs| (opt, keep_pairs))
    }) {
        for forward in [false, true] {
            for shape in [
                Shape {
                    integers: 4,
                    floats: 0,
                    call: false,
                },
                Shape {
                    integers: 64,
                    floats: 0,
                    call: false,
                },
                Shape {
                    integers: 64,
                    floats: 8,
                    call: false,
                },
                Shape {
                    integers: 64,
                    floats: 40,
                    call: false,
                },
                Shape {
                    integers: 64,
                    floats: 8,
                    call: true,
                },
            ] {
                let mut module = JITModule::new(
                    JITBuilder::with_flags(
                        &[("opt_level", opt), ("regalloc_checker", "true")],
                        cranelift_module::default_libcall_names(),
                    )
                    .unwrap(),
                );
                let (original, baseline) =
                    compile(&mut module, shape, 0, forward, keep_pairs, "original");
                let mut entries = Vec::new();
                for slots in [1, 2, 8, 24, 255] {
                    let (id, bytes) = compile(
                        &mut module,
                        shape,
                        slots,
                        forward,
                        keep_pairs,
                        &format!("cache_{slots}"),
                    );
                    if shape.call || shape.integers == 4 {
                        assert_eq!(bytes, baseline, "exact fallback {shape:?} {opt} {forward}");
                        exact_fallbacks += 1;
                    }
                    if bytes != baseline {
                        changed += 1;
                    }
                    if !shape.call && shape.floats == 0 && shape.integers == 64 {
                        assert!(
                            bytes != baseline,
                            "cache must change a pressured native body"
                        );
                        assert!(
                            bytes
                                .chunks_exact(4)
                                .any(|b| u32::from_le_bytes(b.try_into().unwrap()) & 0xfffffc00
                                    == 0x9e670000),
                            "native GPR to FP transfer missing"
                        );
                        assert!(
                            bytes.chunks_exact(4).any(|b| {
                                u32::from_le_bytes(b.try_into().unwrap()) & 0xfffffc00 == 0x9e660000
                            }),
                            "native scalar FP to GPR transfer missing"
                        );
                    }
                    records.push(serde_json::json!({"opt":opt,"forward":forward,"keep_pairs":keep_pairs,"integers":shape.integers,"floats":shape.floats,"call":shape.call,"slots":slots,"baseline_body":baseline,"body":bytes}));
                    entries.push((slots, id));
                }
                module.finalize_definitions().unwrap();
                // SAFETY: retained finalized functions have exactly the declared
                // five-word C ABI. Every module remains live through all calls.
                let original: unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64 =
                    unsafe { std::mem::transmute(module.get_finalized_function(original)) };
                for (slots, id) in entries {
                    // SAFETY: this entry shares the exact declared ABI and owner.
                    let cached: unsafe extern "C" fn(u64, u64, u64, u64, u64) -> u64 =
                        unsafe { std::mem::transmute(module.get_finalized_function(id)) };
                    for (sample, &seed) in seeds.iter().enumerate() {
                        for rounds in 0..=4u64 {
                            let sentinel = 0xabcdeffedcba0123;
                            let mut input = [sentinel; 128];
                            let count = shape.integers + shape.floats;
                            for i in 0..shape.integers {
                                input[i + 1] = seed.rotate_left((i * 7 % 64) as u32)
                                    ^ ((i as u64).wrapping_mul(STEP));
                            }
                            for i in 0..shape.floats {
                                input[shape.integers + i + 1] = (i as f64 + 0.25).to_bits();
                            }
                            let mut expected = input;
                            let mut current_seed = seed;
                            for round in 0..rounds {
                                if shape.call {
                                    current_seed = current_seed.rotate_left(13).wrapping_add(STEP);
                                }
                                let left = (round ^ current_seed) & 1 != 0;
                                for i in 0..shape.integers {
                                    let v = expected[i + 1];
                                    let k = (i % 31 + 1) as u32;
                                    let salt = current_seed.wrapping_add((i as u64 + 1) * SALT);
                                    expected[i + 1] = if left {
                                        v.rotate_left(k).wrapping_add(salt)
                                    } else {
                                        v.rotate_right(k) ^ salt
                                    };
                                }
                                for i in 0..shape.floats {
                                    let v = f64::from_bits(expected[shape.integers + i + 1]);
                                    expected[shape.integers + i + 1] =
                                        (v + if left { 0.5 } else { -0.25 }).to_bits();
                                }
                            }
                            let checksum = expected[1..shape.integers + 1]
                                .iter()
                                .fold(current_seed, |a, &b| a ^ b);
                            let mut a = input;
                            let mut b = input;
                            let mut oa = [sentinel; 128];
                            let mut ob = oa;
                            let alias = (sample + rounds as usize) % 2 == 0;
                            let pa = a.as_mut_ptr();
                            let pb = b.as_mut_ptr();
                            let da = if alias { pa } else { oa.as_mut_ptr() };
                            let db = if alias { pb } else { ob.as_mut_ptr() };
                            // SAFETY: all accessed words are initialized inside live
                            // 128-word buffers. Exact overlap is allowed; all input
                            // loads precede output stores. The callback is C ABI.
                            let results = unsafe {
                                (
                                    original(
                                        pa.add(1) as u64,
                                        da.add(1) as u64,
                                        rounds,
                                        seed,
                                        clobber_callback as *const () as u64,
                                    ),
                                    cached(
                                        pb.add(1) as u64,
                                        db.add(1) as u64,
                                        rounds,
                                        seed,
                                        clobber_callback as *const () as u64,
                                    ),
                                )
                            };
                            calls += 2;
                            assert_eq!(
                                results,
                                (checksum, checksum),
                                "{shape:?} {opt} {forward} {slots} {sample} {rounds}"
                            );
                            assert_eq!(a, b);
                            assert_eq!(oa, ob);
                            let output = if alias { &a } else { &oa };
                            assert_eq!(&output[1..count + 1], &expected[1..count + 1]);
                            assert_eq!(output[0], sentinel);
                            assert!(output[count + 1..].iter().all(|&v| v == sentinel));
                            if !alias {
                                assert_eq!(a, input);
                            }
                        }
                    }
                    for (input, output, rounds) in
                        [(0, 0, 0), (0, 1, 1), (1, 0, 1), (0, 0, u64::MAX)]
                    {
                        // SAFETY: the guard rejects these values before all loads,
                        // stores and callback calls, including invalid pointers.
                        unsafe {
                            assert_eq!(original(input, output, rounds, 0, 0), ERROR);
                            assert_eq!(cached(input, output, rounds, 0, 0), ERROR);
                        }
                        calls += 2;
                    }
                }
            }
        }
    }
    assert_eq!(calls, 513600);
    assert_eq!(exact_fallbacks, 80);
    assert!(changed >= 40);
    if let Some(path) = std::env::var_os("MECH_FP_SPILL_PROBE") {
        std::fs::write(path,serde_json::to_vec(&serde_json::json!({"native_observations":calls,"changed_functions":changed,"exact_fallbacks":exact_fallbacks,"functions":records})).unwrap()).unwrap();
    }
    eprintln!(
        "FP spill native calls: {calls}; changed bodies: {changed}; exact fallbacks: {exact_fallbacks}"
    );
}
