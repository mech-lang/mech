use super::*;
use cranelift_codegen::Context;
use cranelift_codegen::ir::{AbiParam, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

type Entry = unsafe extern "C" fn(u64, u64) -> u64;
const LIMIT: u64 = 258;

#[derive(Clone, Copy, Debug)]
enum Form {
    Direct,
    Masked,
    Signed,
    Raw,
    WrongMask,
    ChangedValue,
    Truncated,
}

fn fixture(
    module: &JITModule,
    cc: IntCC,
    reverse_compare: bool,
    reverse_values: bool,
    immediate: bool,
    form: Form,
) -> Context {
    let mut ctx = module.make_context();
    ctx.func.signature.params = vec![AbiParam::new(types::I64); 2];
    ctx.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut frontend = FunctionBuilderContext::new();
    let mut b = FunctionBuilder::new(&mut ctx.func, &mut frontend);
    let entry = b.create_block();
    let good = b.create_block();
    let bad = b.create_block();
    b.append_block_params_for_function_params(entry);
    b.switch_to_block(entry);
    let a = b.block_params(entry)[0];
    let raw = b.block_params(entry)[1];
    let limit = b.ins().iconst(types::I64, LIMIT as i64);
    let tested = if matches!(form, Form::Truncated) {
        b.ins().ireduce(types::I32, a)
    } else {
        a
    };
    let constant = if matches!(form, Form::Truncated) {
        b.ins().iconst(types::I32, LIMIT as i64)
    } else {
        limit
    };
    let cc = if matches!(form, Form::Signed) {
        IntCC::SignedLessThan
    } else {
        cc
    };
    let test = if immediate && !reverse_compare {
        b.ins().icmp_imm(cc, tested, LIMIT as i64)
    } else if reverse_compare {
        b.ins().icmp(cc, constant, tested)
    } else {
        b.ins().icmp(cc, tested, constant)
    };
    let test = match form {
        Form::Raw => raw,
        Form::Masked | Form::WrongMask => {
            let wide = b.ins().uextend(types::I64, test);
            b.ins().band_imm(
                wide,
                if matches!(form, Form::WrongMask) {
                    2
                } else {
                    1
                },
            )
        }
        _ => test,
    };
    let value = if matches!(form, Form::ChangedValue) {
        b.ins().iadd_imm(a, 1)
    } else {
        a
    };
    let chosen = if reverse_values {
        b.ins().select(test, limit, value)
    } else {
        b.ins().select(test, value, limit)
    };
    let minimum = matches!(cc, IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual)
        ^ reverse_compare
        ^ reverse_values;
    let valid = b.ins().icmp_imm(
        if minimum {
            IntCC::UnsignedLessThanOrEqual
        } else {
            IntCC::UnsignedGreaterThanOrEqual
        },
        chosen,
        LIMIT as i64,
    );
    b.ins().brif(valid, good, &[], bad, &[]);
    b.switch_to_block(good);
    b.ins().return_(&[chosen]);
    b.switch_to_block(bad);
    let error = b.ins().iconst(types::I64, -1);
    b.ins().return_(&[error]);
    b.seal_all_blocks();
    b.finalize();
    ctx
}

fn compile(
    module: &mut JITModule,
    name: &str,
    mut ctx: Context,
    expected: Option<(usize, bool)>,
) -> Entry {
    ctx.verify(module.isa()).unwrap();
    if let Some((expected, widths)) = expected {
        ctx.compute_cfg();
        ctx.compute_domtree();
        assert_eq!(
            if widths {
                super::super::known_widths::fold_selected(&mut ctx.func, &ctx.cfg, &ctx.domtree)
            } else {
                fold(&mut ctx.func, &ctx.cfg, &ctx.domtree)
            },
            expected,
            "{name}"
        );
        ctx.compute_cfg();
        ctx.compute_domtree();
        ctx.verify(module.isa()).unwrap();
    }
    let id = module
        .declare_function(name, Linkage::Local, &ctx.func.signature)
        .unwrap();
    module.define_function(id, &mut ctx).unwrap();
    module.finalize_definitions().unwrap();
    // The owning module stays live and each fixture has this exact two-word ABI.
    unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) }
}

#[test]
fn selected_value_ranges_preserve_unsigned_minmax_at_full_width() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for cc in [
        IntCC::UnsignedLessThan,
        IntCC::UnsignedLessThanOrEqual,
        IntCC::UnsignedGreaterThan,
        IntCC::UnsignedGreaterThanOrEqual,
    ] {
        for reverse_compare in [false, true] {
            for reverse_values in [false, true] {
                for immediate in [false, true] {
                    for form in [Form::Direct, Form::Masked] {
                        let original = fixture(
                            &module,
                            cc,
                            reverse_compare,
                            reverse_values,
                            immediate,
                            form,
                        );
                        let candidate = fixture(
                            &module,
                            cc,
                            reverse_compare,
                            reverse_values,
                            immediate,
                            form,
                        );
                        let name = format!(
                            "minmax_{cc:?}_{reverse_compare}_{reverse_values}_{immediate}_{form:?}"
                        );
                        let before =
                            compile(&mut module, &(name.clone() + "_before"), original, None);
                        let after =
                            compile(&mut module, &(name + "_after"), candidate, Some((1, false)));
                        let minimum =
                            matches!(cc, IntCC::UnsignedLessThan | IntCC::UnsignedLessThanOrEqual)
                                ^ reverse_compare
                                ^ reverse_values;
                        for input in [
                            0,
                            1,
                            257,
                            258,
                            259,
                            511,
                            u32::MAX as u64,
                            1 << 32,
                            1 << 63,
                            u64::MAX,
                        ] {
                            let expected = if minimum {
                                input.min(LIMIT)
                            } else {
                                input.max(LIMIT)
                            };
                            for raw in [0, 1, 2, u64::MAX] {
                                assert_eq!(unsafe { before(input, raw) }, expected);
                                assert_eq!(unsafe { after(input, raw) }, expected);
                                checked += 1;
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(checked, 2560);
}

#[test]
fn selected_value_ranges_keep_signed_lossy_unrelated_and_changed_conditions() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for form in [
        Form::Signed,
        Form::Raw,
        Form::WrongMask,
        Form::ChangedValue,
        Form::Truncated,
    ] {
        let before = fixture(&module, IntCC::UnsignedLessThan, false, false, false, form);
        let after = fixture(&module, IntCC::UnsignedLessThan, false, false, false, form);
        let before = compile(
            &mut module,
            &format!("negative_{form:?}_before"),
            before,
            None,
        );
        let after = compile(
            &mut module,
            &format!("negative_{form:?}_after"),
            after,
            Some((0, false)),
        );
        for input in [0, 1, 257, 258, 259, 511, (1 << 32) + 1, 1 << 63, u64::MAX] {
            for raw in [0, 1, 2, u64::MAX] {
                let truth = match form {
                    Form::Signed => (input as i64) < LIMIT as i64,
                    Form::Raw => raw != 0,
                    Form::WrongMask => false,
                    Form::Truncated => (input as u32) < LIMIT as u32,
                    _ => input < LIMIT,
                };
                let value = if matches!(form, Form::ChangedValue) {
                    input.wrapping_add(1)
                } else {
                    input
                };
                let selected = if truth { value } else { LIMIT };
                // Signed uses the maximum guard in this fixture; all other negative forms use minimum.
                let valid = if matches!(form, Form::Signed) {
                    selected >= LIMIT
                } else {
                    selected <= LIMIT
                };
                let expected = if valid { selected } else { u64::MAX };
                assert_eq!(unsafe { before(input, raw) }, expected);
                assert_eq!(unsafe { after(input, raw) }, expected);
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 180);
}

#[test]
fn selected_value_ranges_width_solver_keeps_full_width_rejections() {
    let mut module = JITModule::new(JITBuilder::new(default_libcall_names()).unwrap());
    let mut checked = 0;
    for form in [
        Form::Direct,
        Form::Masked,
        Form::Signed,
        Form::Raw,
        Form::WrongMask,
        Form::ChangedValue,
        Form::Truncated,
    ] {
        let mut original = fixture(&module, IntCC::UnsignedLessThan, false, false, false, form);
        let entry = original.func.layout.entry_block().unwrap();
        let branch = original.func.layout.last_inst(entry).unwrap();
        let Data::Brif { arg, .. } = original.func.dfg.insts[branch] else {
            unreachable!()
        };
        let ValueDef::Result(inst, 0) = original.func.dfg.value_def(arg) else {
            unreachable!()
        };
        let Data::IntCompareImm { arg, .. } = original.func.dfg.insts[inst] else {
            unreachable!()
        };
        original
            .func
            .dfg
            .replace(inst)
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, arg, 511);
        let mut candidate = module.make_context();
        candidate.func = original.func.clone();
        let before = compile(
            &mut module,
            &format!("width_{form:?}_before"),
            original,
            None,
        );
        let expected = usize::from(matches!(form, Form::Direct | Form::Masked));
        let after = compile(
            &mut module,
            &format!("width_{form:?}_after"),
            candidate,
            Some((expected, true)),
        );
        for input in [0, 1, 257, 258, 259, 511, (1 << 32) + 1, 1 << 63, u64::MAX] {
            for raw in [0, 1, 2, u64::MAX] {
                assert_eq!(unsafe { after(input, raw) }, unsafe { before(input, raw) });
                checked += 1;
            }
        }
    }
    assert_eq!(checked, 252);
}
