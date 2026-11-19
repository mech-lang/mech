use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};
fn make(
    mode: u8,
    kind: u8,
) -> (
    JITModule,
    unsafe extern "C" fn(*mut u64, u64, *const u32, u64) -> u64,
) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("enable_verifier", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut c = module.make_context();
    c.func.signature.params = vec![AbiParam::new(types::I64); 4];
    c.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut fc = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
    let entry = b.create_block();
    let header = b.create_block();
    let check = b.create_block();
    let read_block = b.create_block();
    let even = b.create_block();
    let odd = b.create_block();
    let out = b.create_block();
    let bad = b.create_block();
    b.append_block_params_for_function_params(entry);
    for _ in 0..2 {
        b.append_block_param(header, types::I64);
    }
    b.append_block_param(out, types::I64);
    b.switch_to_block(entry);
    let args = b.block_params(entry).to_vec();
    let zero = b.ins().iconst(types::I64, 0);
    b.ins().jump(header, &[zero.into(), args[1].into()]);
    b.switch_to_block(header);
    let state = b.block_params(header).to_vec();
    let more = b.ins().icmp(IntCC::UnsignedLessThan, state[0], args[3]);
    b.ins().brif(more, check, &[], out, &[state[1].into()]);
    b.switch_to_block(check);
    let nonempty = b.ins().icmp_imm(IntCC::NotEqual, state[1], 0);
    b.ins().brif(nonempty, read_block, &[], bad, &[]);
    b.switch_to_block(bad);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.switch_to_block(read_block);
    let prior = b.ins().iadd_imm(state[1], -1);
    let at = b.ins().ishl_imm(prior, 3);
    let at = b.ins().iadd(args[0], at);
    let physical = match kind {
        0 => b.ins().load(types::I64, MemFlags::new(), at, 0),
        1 => b.ins().uload32(MemFlags::new(), at, 0),
        _ => b.ins().load(types::I32, MemFlags::new(), at, 0),
    };
    let read_inst = b.func.dfg.value_def(physical).inst().unwrap();
    let read = if kind == 2 {
        b.ins().uextend(types::I64, physical)
    } else {
        physical
    };
    let at = b.ins().ishl_imm(state[0], 2);
    let at = b.ins().iadd(args[2], at);
    let price = b.ins().uload32(MemFlags::new(), at, 0);
    let low = b.ins().ireduce(types::I32, read);
    let price = b.ins().ireduce(types::I32, price);
    let low = b.ins().iadd(low, price);
    let low = b.ins().uextend(types::I64, low);
    let high = b.ins().ushr_imm(read, 32);
    let high = b.ins().bxor(high, state[0]);
    let high = b.ins().ishl_imm(high, 32);
    let word = b.ins().bor(low, high);
    let at = b.ins().ishl_imm(state[1], 3);
    let at = b.ins().iadd(args[0], at);
    let next = b.ins().iadd_imm(state[0], 1);
    let length = b.ins().iadd_imm(state[1], 1);
    let odd_flag = b.ins().band_imm(state[0], 1);
    b.ins().brif(odd_flag, odd, &[], even, &[]);
    b.switch_to_block(even);
    let a = b.ins().store(MemFlags::new(), word, at, 0);
    b.ins().jump(header, &[next.into(), length.into()]);
    b.switch_to_block(odd);
    let word = b.ins().bxor_imm(word, 0x1234_5678_0000_0000);
    let d = b.ins().store(MemFlags::new(), word, at, 0);
    b.ins().jump(header, &[next.into(), length.into()]);
    b.switch_to_block(out);
    let result = b.block_params(out)[0];
    b.ins().return_(&[result]);
    b.seal_all_blocks();
    b.finalize();
    c.compute_cfg();
    c.compute_domtree();
    if mode > 0 {
        assert!(
            super::forward(
                &mut c,
                read_inst,
                &[a, d],
                if mode == 2 || mode == 4 {
                    Some((args[0], args[1]))
                } else {
                    None
                },
                mode >= 3,
            )
            .is_ok()
        );
    }
    let id = module
        .declare_function("append_previous", Linkage::Local, &c.func.signature)
        .unwrap();
    module.define_function(id, &mut c).unwrap();
    module.finalize_definitions().unwrap();
    // SAFETY: The live finalized module owns this exact C signature.
    let call = unsafe { std::mem::transmute(module.get_finalized_function(id)) };
    (module, call)
}
#[test]
fn forward_last_append_preserves_initial_word_wraps_branches_and_resets() {
    let mut tested = 0;
    for mode in 1..=4 {
        for kind in 0..3 {
            let (_a, a) = make(0, kind);
            let (_b, b) = make(mode, kind);
            for seed in [
                0u64,
                1,
                u64::MAX,
                0x8000_0000_8000_0000,
                0x0123_4567_ffff_ffff,
            ] {
                for initial in [0usize, 1, 2, 4] {
                    for count in [0usize, 1, 2, 3, 7, 16] {
                        for price in [0u32, 1, 0x8000_0000, u32::MAX] {
                            let mut before = [0xfeed_face_dead_beefu64; 32];
                            before[..initial].fill(seed);
                            let mut after = before;
                            let prices =
                                std::array::from_fn::<_, 16, _>(|i| price.wrapping_add(i as u32));
                            // SAFETY: Both live modules use this exact signature; the arrays hold every admitted read and append.
                            let (x, y) = unsafe {
                                (
                                    a(
                                        before.as_mut_ptr(),
                                        initial as u64,
                                        prices.as_ptr(),
                                        count as u64,
                                    ),
                                    b(
                                        after.as_mut_ptr(),
                                        initial as u64,
                                        prices.as_ptr(),
                                        count as u64,
                                    ),
                                )
                            };
                            assert_eq!(x, y);
                            assert_eq!(before, after);
                            assert_eq!(
                                x,
                                if initial == 0 && count > 0 {
                                    u64::MAX
                                } else {
                                    (initial + count) as u64
                                }
                            );
                            tested += 1;
                        }
                    }
                }
            }
        }
    }
    assert_eq!(tested, 5760);
}

#[test]
fn append_hint_requires_the_exact_view_and_declines_unknown_writes_before_mutation() {
    for (shift, unknown, admitted) in [(0, false, true), (8, false, false), (0, true, false)] {
        let module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
        let mut c = module.make_context();
        c.func.signature.params = vec![AbiParam::new(types::I64); 3];
        c.func.signature.returns.push(AbiParam::new(types::I64));
        let mut fc = FunctionBuilderContext::new();
        let mut b = FunctionBuilder::new(&mut c.func, &mut fc);
        let entry = b.create_block();
        let body = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[1];
        let ptr = b.ins().load(types::I64, MemFlags::new(), input, 0);
        let length = b.ins().load(types::I64, MemFlags::new(), input, 8);
        let other = b.ins().load(types::I64, MemFlags::new(), input, 16);
        let base = if shift == 0 {
            ptr
        } else {
            b.ins().iadd_imm(ptr, shift)
        };
        b.ins().jump(body, &[]);
        b.switch_to_block(body);
        let last = b.ins().iadd_imm(length, -1);
        let offset = b.ins().ishl_imm(last, 3);
        let at = b.ins().iadd(base, offset);
        let value = b.ins().load(types::I64, MemFlags::new(), at, 0);
        let read = b.func.dfg.value_def(value).inst().unwrap();
        let offset = b.ins().ishl_imm(length, 3);
        let at = b.ins().iadd(base, offset);
        let append = b.ins().store(MemFlags::new(), value, at, 0);
        if unknown {
            b.ins().store(MemFlags::new(), value, other, 0);
        }
        b.ins().return_(&[value]);
        b.seal_all_blocks();
        b.finalize();
        c.flowgraph();
        let loads = [ptr, length, other]
            .into_iter()
            .enumerate()
            .map(|(word, value)| (value, (c.func.dfg.value_def(value).inst().unwrap(), word)))
            .collect();
        let original = c.func.display().to_string();
        let hint = super::Hint::new(read, vec![append], base);
        let result = hint.apply(&mut c, &loads, 0, 1);
        assert_eq!(result.is_ok(), admitted, "shift={shift}, unknown={unknown}");
        if admitted {
            c.verify(module.isa()).unwrap();
        } else {
            assert_eq!(original, c.func.display().to_string());
        }
    }
}
