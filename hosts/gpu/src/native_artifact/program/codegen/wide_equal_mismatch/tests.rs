use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

fn fixture(module: &JITModule, variant: &str) -> (Context, Certificate) {
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 5];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fb = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut c.func, &mut fb);
    let entry = b.create_block();
    b.append_block_params_for_function_params(entry);
    let header = b.create_block();
    let body = b.create_block();
    let back = b.create_block();
    let mismatch = b.create_block();
    let exit = b.create_block();
    let join = b.create_block();
    b.append_block_param(header, types::I64);
    if variant == "shared_mismatch" {
        b.append_block_param(mismatch, types::I64);
        b.append_block_param(mismatch, types::I64);
    }
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let admitted = b.create_block();
    let refused = b.create_block();
    let valid = b
        .ins()
        .icmp(IntCC::UnsignedLessThanOrEqual, args[2], args[3]);
    b.ins().brif(valid, admitted, &[], refused, &[]);
    b.switch_to_block(refused);
    let returned = b.ins().bxor(args[2], args[4]);
    b.ins().return_(&[returned]);
    b.switch_to_block(admitted);
    if variant == "extra_entry" {
        let other = b.create_block();
        let choose = b.ins().band_imm(args[4], 1);
        b.ins().brif(choose, header, &[args[2].into()], other, &[]);
        b.switch_to_block(other);
    }
    b.ins().jump(header, &[args[2].into()]);
    b.switch_to_block(header);
    let at = b.block_params(header)[0];
    let next = b.ins().iadd_imm(at, 8);
    let fits = b.ins().icmp(IntCC::UnsignedLessThanOrEqual, next, args[3]);
    b.ins().brif(fits, body, &[], exit, &[]);
    b.switch_to_block(body);
    let left = b.ins().iadd(args[0], at);
    let right = b.ins().iadd(args[1], at);
    let flags = MemFlags::new().with_endianness(Endianness::Little);
    let left = b.ins().load(types::I64, flags, left, 0);
    let right = b.ins().load(types::I64, flags, right, 0);
    if variant == "store" {
        b.ins().store(MemFlags::new(), left, args[0], 0);
    }
    let payload = b.ins().iadd_imm(at, 11);
    let difference = b.ins().bxor(left, right);
    let zero = b.ins().iconst(types::I64, 0);
    let different = variant == "reversed";
    let condition = b.ins().icmp(
        if different {
            IntCC::NotEqual
        } else {
            IntCC::Equal
        },
        difference,
        zero,
    );
    let condition = if variant == "masked" {
        b.ins().band_imm(condition, 1)
    } else {
        condition
    };
    let mismatch_args = if variant == "shared_mismatch" {
        vec![at.into(), difference.into()]
    } else {
        Vec::new()
    };
    if different {
        b.ins().brif(condition, mismatch, &mismatch_args, back, &[]);
    } else {
        b.ins().brif(condition, back, &[], mismatch, &mismatch_args);
    }
    b.switch_to_block(back);
    let next = b.ins().iadd_imm(at, 8);
    b.ins().jump(header, &[next.into()]);
    b.switch_to_block(mismatch);
    if variant == "cross_exit" {
        b.ins().jump(join, &[]);
    } else {
        let (position_at, difference, payload) = if variant == "shared_mismatch" {
            let values = b.block_params(mismatch).to_vec();
            let payload = b.ins().iadd_imm(values[0], 11);
            (values[0], values[1], payload)
        } else {
            (at, difference, payload)
        };
        let bits = b.ins().ctz(difference);
        let bytes = b.ins().ushr_imm(bits, 3);
        let position = b.ins().iadd(position_at, bytes);
        let value = b.ins().bxor(
            position,
            if variant == "observed_word" {
                left
            } else {
                payload
            },
        );
        let value = b.ins().bxor(value, args[4]);
        b.ins().return_(&[value]);
    }
    b.switch_to_block(exit);
    if variant == "shared_mismatch" {
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().jump(mismatch, &[at.into(), zero.into()]);
        b.switch_to_block(join);
        let zero = b.ins().iconst(types::I64, 0);
        b.ins().return_(&[zero]);
    } else {
        b.ins().jump(join, &[]);
        b.switch_to_block(join);
        let value = b.ins().bxor(at, args[4]);
        b.ins().return_(&[value]);
    }
    b.seal_all_blocks();
    b.finalize();
    c.flowgraph();
    (
        c,
        Certificate {
            header,
            blocks: BTreeSet::from([header, body, back]),
            source_header: 7,
            reuse_scope: None,
        },
    )
}

type Entry = unsafe extern "C" fn(*mut u8, *const u8, u64, u64, u64) -> u64;
fn compile(variant: &str, optimize: bool, probe_word: bool) -> (JITModule, Entry, bool) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[
                ("opt_level", "speed"),
                ("enable_verifier", "true"),
                ("regalloc_checker", "true"),
            ],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let (mut c, cert) = fixture(&module, variant);
    let prepared =
        candidate(&c, &cert).and_then(|change| plan(&c, &cert, change).map(|plan| (change, plan)));
    let eligible = prepared.is_some();
    if optimize && let Some((change, plan)) = prepared {
        let direct = plan.install(&mut c.func);
        let check = c.func.dfg.make_block();
        let at = c.func.dfg.append_block_param(check, types::I64);
        let vector = c.func.dfg.make_block();
        let advance = c.func.dfg.make_block();
        let failed = c.func.dfg.make_block();
        for block in [check, vector, advance, failed] {
            c.func.layout.append_block(block);
        }
        let entry = if probe_word {
            let entry = c.func.dfg.make_block();
            c.func.dfg.append_block_param(entry, types::I64);
            c.func.layout.append_block(entry);
            entry
        } else {
            check
        };
        c.func
            .rewrite_branch_destination(change.entry, cert.header, entry);
        let mut cursor = FuncCursor::new(&mut c.func).at_bottom(entry);
        if probe_word {
            let initial = cursor.func.dfg.block_params(entry)[0];
            let initial = direct.probe_word(&mut cursor, &cert, change, initial);
            cursor.ins().jump(check, &[initial.into()]);
            cursor.goto_bottom(check);
        }
        let left = cursor.ins().isub(change.limit, at);
        let enough = cursor
            .ins()
            .icmp_imm(IntCC::UnsignedGreaterThanOrEqual, left, 16);
        cursor
            .ins()
            .brif(enough, vector, &[], cert.header, &[at.into()]);
        cursor.goto_bottom(vector);
        let a = cursor.ins().iadd(change.bases[0], at);
        let z = cursor.ins().iadd(change.bases[1], at);
        let flags = MemFlags::new().with_endianness(Endianness::Little);
        let a = cursor.ins().load(types::I64X2, flags, a, 0);
        let z = cursor.ins().load(types::I64X2, flags, z, 0);
        let difference = cursor.ins().bxor(a, z);
        let low = cursor.ins().extractlane(difference, 0);
        let high = cursor.ins().extractlane(difference, 1);
        let different = cursor.ins().bor(low, high);
        cursor.ins().brif(different, failed, &[], advance, &[]);
        cursor.goto_bottom(failed);
        direct.emit16(&mut cursor, at, difference);
        cursor.goto_bottom(advance);
        let next = cursor.ins().iadd_imm(at, 16);
        cursor.ins().jump(check, &[next.into()]);
    }
    c.flowgraph();
    c.verify(module.isa()).unwrap();
    let id = module
        .declare_function("mismatch_forwarding", Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, &mut c).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: This live finalized module owns exactly the declared signature.
    let entry = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    (module, entry, eligible)
}

fn compare_mismatches(probe_word: bool) {
    let mut observations = 0;
    for variant in ["normal", "masked", "reversed", "shared_mismatch"] {
        let (_old, old, _) = compile(variant, false, false);
        let (_new, new, eligible) = compile(variant, true, probe_word);
        assert!(eligible);
        for offset in [0, 1, 3, 7] {
            for start in [0, 1, 7, 13] {
                for limit in [
                    0, 7, 8, 9, 15, 16, 17, 31, 32, 33, 63, 64, 65, 127, 128, 129, 255, 256, 257,
                ] {
                    for mismatch in 0..=264 {
                        for bit in [1u8, 128] {
                            let left = std::array::from_fn::<_, 288, _>(|i| {
                                (i as u8).wrapping_mul(73).wrapping_add(17)
                            });
                            let mut right = left;
                            if mismatch < 264 {
                                right[offset + mismatch] ^= bit;
                            }
                            for marker in [0, u64::MAX] {
                                let mut a = left;
                                let mut z = left;
                                // SAFETY: Both modules retain the exact entry signature;
                                // all admitted windows, including vector tails, fit the arrays.
                                let (x, y) = unsafe {
                                    (
                                        old(
                                            a.as_mut_ptr().add(offset),
                                            right.as_ptr().add(offset),
                                            start,
                                            limit,
                                            marker,
                                        ),
                                        new(
                                            z.as_mut_ptr().add(offset),
                                            right.as_ptr().add(offset),
                                            start,
                                            limit,
                                            marker,
                                        ),
                                    )
                                };
                                assert_eq!(
                                    x, y,
                                    "{variant} {offset} {start} {limit} {mismatch} {bit}"
                                );
                                assert_eq!(a, z);
                                observations += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(observations, 1288960);
}

#[test]
fn vector_mismatch_preserves_first_byte_bits_tails_positions_and_live_payloads() {
    compare_mismatches(false);
}

#[test]
fn first_word_probe_preserves_mismatches_and_the_following_vector_and_scalar_tails() {
    compare_mismatches(true);
}

#[test]
fn vector_mismatch_declines_effects_independently_observed_words_and_unsupported_exits() {
    for variant in ["store", "observed_word", "cross_exit", "extra_entry"] {
        let (_old, old, _) = compile(variant, false, false);
        let (_new, new, eligible) = compile(variant, true, true);
        assert!(!eligible, "{variant}");
        for limit in [
            0, 7, 8, 15, 16, 17, 63, 64, 65, 127, 128, 129, 255, 256, 257,
        ] {
            let mut a = [1u8; 288];
            let mut z = a;
            let mut right = a;
            right[11] = 7;
            // SAFETY: Both exact signatures remain owned by live modules,
            // with exclusive writable input and a disjoint readable array.
            let (x, y) = unsafe {
                (
                    old(a.as_mut_ptr(), right.as_ptr(), 0, limit, 17),
                    new(z.as_mut_ptr(), right.as_ptr(), 0, limit, 17),
                )
            };
            assert_eq!(x, y);
            assert_eq!(a, z);
        }
    }
}
