use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags, condcodes::IntCC, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn boolean_chains_keep_eager_effects_raw_words_widths_and_same_target_payloads() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut functions = Vec::new();
    for ty in [types::I8, types::I16, types::I32, types::I64] {
        for mode in 0..6 {
            for enabled in [false, true] {
                let mut ctx = module.make_context();
                ctx.func.signature.params = vec![AbiParam::new(types::I64); 4];
                ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
                let id = module
                    .declare_function(
                        &format!("boolean_chain_{}", functions.len()),
                        Linkage::Local,
                        &ctx.func.signature,
                    )
                    .unwrap();
                let mut fc = FunctionBuilderContext::new();
                {
                    let mut b = FunctionBuilder::new(&mut ctx.func, &mut fc);
                    let entry = b.create_block();
                    let join = b.create_block();
                    b.append_block_params_for_function_params(entry);
                    b.append_block_param(join, types::I64);
                    b.switch_to_block(entry);
                    let args = b.block_params(entry).to_vec();
                    let values = args[..3]
                        .iter()
                        .map(|&v| {
                            if ty == types::I64 {
                                v
                            } else {
                                b.ins().ireduce(ty, v)
                            }
                        })
                        .collect::<Vec<_>>();
                    let predicates = values
                        .iter()
                        .map(|&v| b.ins().icmp_imm(IntCC::NotEqual, v, 0))
                        .collect::<Vec<_>>();
                    let [a, c, d] = <[Value; 3]>::try_from(predicates).unwrap();
                    let root = match mode {
                        0 => b.ins().band(a, c),
                        1 => b.ins().bor(a, c),
                        2 => {
                            let pair = b.ins().band(a, c);
                            b.ins().band(pair, d)
                        }
                        3 => {
                            let pair = b.ins().bor(a, c);
                            b.ins().bor(pair, d)
                        }
                        4 => {
                            let pair = b.ins().band(a, c);
                            b.ins().bor(pair, d)
                        }
                        5 => b.ins().band(values[0], values[1]),
                        _ => unreachable!(),
                    };
                    let root = if mode % 2 == 0 {
                        let wide = b.ins().uextend(types::I64, root);
                        b.ins().band_imm(wide, 1)
                    } else {
                        root
                    };
                    // These eager writes must remain visible even when the
                    // first predicate of a distributed chain is false.
                    let marker = b.ins().bxor(args[0], args[1]);
                    b.ins().store(MemFlags::new(), marker, args[3], 0);
                    let yes = b.ins().iconst(types::I64, 31);
                    let no = b.ins().iconst(types::I64, 97);
                    // Same target, different arguments: reachability alone
                    // must never substitute for preserving edge payloads.
                    b.ins().brif(root, join, &[yes.into()], join, &[no.into()]);
                    b.switch_to_block(join);
                    let selected = b.block_params(join)[0];
                    b.ins().return_(&[selected]);
                    b.seal_all_blocks();
                    b.finalize();
                }
                let count = if enabled { expand(&mut ctx.func) } else { 0 };
                assert_eq!(count, usize::from(enabled && mode != 5));
                if enabled {
                    assert_eq!(expand(&mut ctx.func), 0, "idempotent");
                }
                module.define_function(id, &mut ctx).unwrap();
                functions.push((id, ty.bits(), mode));
            }
        }
    }
    module.finalize_definitions().unwrap();
    for (id, bits, mode) in functions {
        // SAFETY: each retained function has the exact four-word signature.
        let run: unsafe extern "C" fn(u64, u64, u64, *mut u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        let mask = u64::MAX >> (64 - bits);
        for x in [
            0,
            1,
            2,
            3,
            127,
            128,
            255,
            256,
            257,
            1 << 31,
            1 << 63,
            u64::MAX,
        ] {
            for y in [0, 1, 2, 3, 128, 255, 256, u64::MAX] {
                for z in [0, 1, 2, 255, 256, u64::MAX] {
                    let [a, c, d] = [x, y, z].map(|v| v & mask != 0);
                    let yes = match mode {
                        0 => a && c,
                        1 => a || c,
                        2 => a && c && d,
                        3 => a || c || d,
                        4 => (a && c) || d,
                        5 => (x & y & mask) != 0,
                        _ => unreachable!(),
                    };
                    let mut marker = 0;
                    assert_eq!(
                        unsafe { run(x, y, z, &mut marker) },
                        if yes { 31 } else { 97 },
                        "bits={bits} mode={mode} x={x} y={y} z={z}"
                    );
                    assert_eq!(marker, x ^ y);
                }
            }
        }
    }
}

#[test]
fn boolean_chains_refuse_a_noncanonical_loop_backedge() {
    let mut f = Function::new();
    let mut fc = FunctionBuilderContext::new();
    f.signature.params = vec![AbiParam::new(types::I64)];
    f.signature.returns = vec![AbiParam::new(types::I64)];
    {
        let mut b = FunctionBuilder::new(&mut f, &mut fc);
        let entry = b.create_block();
        let header = b.create_block();
        let body = b.create_block();
        let exit = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(header, types::I64);
        b.switch_to_block(entry);
        let arg = b.block_params(entry)[0];
        let one = b.ins().iconst(types::I64, 1);
        b.ins().jump(header, &[one.into()]);
        b.switch_to_block(header);
        let raw = b.block_params(header)[0];
        let cmp = b.ins().icmp_imm(IntCC::UnsignedLessThan, raw, 7);
        let wide = b.ins().uextend(types::I64, cmp);
        let combined = b.ins().band(raw, wide);
        b.ins().brif(combined, body, &[], exit, &[]);
        b.switch_to_block(body);
        let next = b.ins().iadd(raw, arg);
        b.ins().jump(header, &[next.into()]);
        b.switch_to_block(exit);
        b.ins().return_(&[raw]);
        b.seal_all_blocks();
        b.finalize();
    }
    assert_eq!(expand(&mut f), 0);
}
