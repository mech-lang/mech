use super::*;
use crate::native_artifact::codegen::options::{
    CompilationControls, OptimizationOptions, OptimizationPass,
};
use cranelift_codegen::ir::{AbiParam, InstBuilder, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[test]
fn boolean_conditions_preserve_raw_words_and_every_loop_entry_and_backedge() {
    let mut module = cranelift_jit::JITModule::new(
        cranelift_jit::JITBuilder::with_flags(&[("opt_level", "speed")], default_libcall_names())
            .unwrap(),
    );
    let mut functions = Vec::new();
    for mode in 0..8 {
        for cc in [IntCC::Equal, IntCC::NotEqual] {
            for wanted in [0, 1] {
                for enabled in [false, true] {
                    let mut context = module.make_context();
                    context.func.signature.params = vec![AbiParam::new(types::I64); 2];
                    context.func.signature.returns = vec![AbiParam::new(types::I64)];
                    let id = module
                        .declare_function(
                            &format!("canonical_boolean_{}", functions.len()),
                            Linkage::Local,
                            &context.func.signature,
                        )
                        .unwrap();
                    let mut fc = FunctionBuilderContext::new();
                    {
                        let mut b = FunctionBuilder::new(&mut context.func, &mut fc);
                        let entry = b.create_block();
                        let header = b.create_block();
                        let judge = b.create_block();
                        let yes = b.create_block();
                        let no = b.create_block();
                        let join = b.create_block();
                        let exit = b.create_block();
                        b.append_block_params_for_function_params(entry);
                        for _ in 0..3 {
                            b.append_block_param(header, types::I64);
                        }
                        b.append_block_param(yes, types::I64);
                        b.append_block_param(no, types::I64);
                        b.append_block_param(join, types::I64);
                        b.switch_to_block(entry);
                        let args = b.block_params(entry).to_vec();
                        let zero = b.ins().iconst(types::I64, 0);
                        let one = b.ins().iconst(types::I64, 1);
                        let predicate = b.ins().icmp_imm(IntCC::NotEqual, args[0], 0);
                        let canonical = b.ins().uextend(types::I64, predicate);
                        let flag = match mode {
                            0 | 6 => canonical,
                            1 => b.ins().band_imm(args[0], 1),
                            2 => b.ins().bxor_imm(canonical, 1),
                            3 => args[0],
                            4 => {
                                let byte = b.ins().ireduce(types::I8, args[0]);
                                b.ins().uextend(types::I64, byte)
                            }
                            5 => b.ins().select(predicate, zero, one),
                            7 => {
                                let mask = b.ins().iconst(types::I64, 1);
                                b.ins().band(args[0], mask)
                            }
                            _ => unreachable!(),
                        };
                        b.ins()
                            .jump(header, &[zero.into(), flag.into(), zero.into()]);
                        b.switch_to_block(header);
                        let params = b.block_params(header).to_vec();
                        let more = b.ins().icmp(IntCC::UnsignedLessThan, params[0], args[1]);
                        b.ins().brif(more, judge, &[], exit, &[]);
                        b.switch_to_block(judge);
                        let test = if mode % 2 == 0 {
                            b.ins().icmp_imm(cc, params[1], wanted)
                        } else {
                            let literal = b.ins().iconst(types::I64, wanted);
                            b.ins().icmp(cc, literal, params[1])
                        };
                        let wide = b.ins().uextend(types::I64, test);
                        let yes_payload = b.ins().iconst(types::I64, 0x23);
                        let no_payload = b.ins().iconst(types::I64, 0xa7);
                        b.ins()
                            .brif(wide, yes, &[yes_payload.into()], no, &[no_payload.into()]);
                        b.switch_to_block(yes);
                        let payload = b.block_params(yes)[0];
                        let added = b.ins().iadd_imm(payload, 1);
                        b.ins().jump(join, &[added.into()]);
                        b.switch_to_block(no);
                        let payload = b.block_params(no)[0];
                        let added = b.ins().iadd_imm(payload, 2);
                        b.ins().jump(join, &[added.into()]);
                        b.switch_to_block(join);
                        let added = b.block_params(join)[0];
                        let tripled = b.ins().imul_imm(params[2], 3);
                        let sum = b.ins().iadd(tripled, added);
                        let next = b.ins().iadd_imm(params[0], 1);
                        let flag = if mode == 6 {
                            // A canonical entry cannot justify a loop that
                            // later produces 2, 3, ... on its backedge.
                            b.ins().iadd_imm(params[1], 1)
                        } else {
                            b.ins().bxor_imm(params[1], 1)
                        };
                        b.ins()
                            .jump(header, &[next.into(), flag.into(), sum.into()]);
                        b.switch_to_block(exit);
                        b.ins().return_(&[params[2]]);
                        b.seal_all_blocks();
                        b.finalize();
                    }
                    let options = if enabled {
                        OptimizationOptions::default()
                    } else {
                        OptimizationOptions::without(OptimizationPass::BooleanConditions)
                    };
                    let report = super::super::optimize(
                        &CompilationControls::new(options),
                        &mut context,
                        module.isa(),
                    )
                    .unwrap();
                    assert_eq!(
                        report.boolean_conditions_simplified,
                        usize::from(enabled && !matches!(mode, 3 | 4 | 6)),
                        "mode={mode}, cc={cc:?}, wanted={wanted}"
                    );
                    module.define_function(id, &mut context).unwrap();
                    functions.push((id, mode, cc, wanted as u64));
                }
            }
        }
    }
    module.finalize_definitions().unwrap();
    for (id, mode, cc, wanted) in functions {
        // SAFETY: every owned test function has exactly this two-word ABI.
        let run: unsafe extern "C" fn(u64, u64) -> u64 =
            unsafe { core::mem::transmute(module.get_finalized_function(id)) };
        for raw in [0, 1, 2, 3, 127, 255, 256, 257, 1 << 63, u64::MAX] {
            for count in [0, 1, 2, 3, 17, 33] {
                let canonical = u64::from(raw != 0);
                let mut flag = match mode {
                    0 | 6 => canonical,
                    1 | 7 => raw & 1,
                    2 | 5 => canonical ^ 1,
                    3 => raw,
                    4 => u64::from(raw as u8),
                    _ => unreachable!(),
                };
                let mut expected = 0u64;
                for _ in 0..count {
                    let yes = (flag == wanted) == (cc == IntCC::Equal);
                    expected = expected
                        .wrapping_mul(3)
                        .wrapping_add(if yes { 0x24 } else { 0xa9 });
                    flag = if mode == 6 {
                        flag.wrapping_add(1)
                    } else {
                        flag ^ 1
                    };
                }
                assert_eq!(
                    unsafe { run(raw, count) },
                    expected,
                    "mode={mode}, cc={cc:?}, wanted={wanted}, raw={raw}, count={count}"
                );
            }
        }
    }
}
