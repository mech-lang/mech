use super::*;
use cranelift_codegen::ir::{AbiParam, MemFlags};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Case {
    Exact,
    WrappedPredicate,
    OtherPredicate,
    OtherMinimum,
    ExtraFirstEntry,
    ObservableMiddle,
    ObserveMaximumAfterMerge,
    LiveRepeatedPredicate,
}

type Entry = unsafe extern "C" fn(u64, u64, u64, u64, u64, *mut u64) -> u64;

fn compile(case: Case, optimize: bool) -> (JITModule, Entry) {
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 6];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        let early = b.create_block();
        let first = b.create_block();
        let take = b.create_block();
        let keep = b.create_block();
        let maximum = b.create_block();
        let repeat = b.create_block();
        let near = b.create_block();
        let lower = b.create_block();
        let higher = b.create_block();
        let minimum = b.create_block();
        let far = b.create_block();
        let finish = b.create_block();
        b.append_block_params_for_function_params(entry);
        for join in [maximum, minimum, finish] {
            b.append_block_param(join, types::I64);
        }
        b.switch_to_block(entry);
        let [remaining, max, nice, limit, route, out] = b.block_params(entry).try_into().unwrap();
        let early_test = b.ins().icmp_imm(IntCC::Equal, route, 0);
        b.ins().brif(early_test, early, &[], first, &[]);
        b.switch_to_block(early);
        if case == Case::ExtraFirstEntry {
            b.ins().jump(maximum, &[max.into()]);
        } else {
            let rejected = b.ins().iconst(types::I64, 91);
            b.ins().return_(&[rejected]);
        }
        b.switch_to_block(first);
        let test = b.ins().icmp(IntCC::UnsignedLessThan, remaining, limit);
        let test = if case == Case::WrappedPredicate {
            let wide = b.ins().uextend(types::I64, test);
            b.ins().band_imm(wide, 1)
        } else {
            test
        };
        b.ins().brif(test, take, &[], keep, &[]);
        b.switch_to_block(take);
        b.ins().jump(maximum, &[remaining.into()]);
        b.switch_to_block(keep);
        b.ins().jump(maximum, &[max.into()]);
        b.switch_to_block(maximum);
        let max_value = b.block_params(maximum)[0];
        b.ins().jump(repeat, &[]);
        b.switch_to_block(repeat);
        let repeat_limit = if case == Case::OtherPredicate {
            max
        } else {
            limit
        };
        let check = b
            .ins()
            .icmp(IntCC::UnsignedLessThan, remaining, repeat_limit);
        if case == Case::ObservableMiddle {
            b.ins().store(MemFlags::new(), max_value, out, 8);
        }
        b.ins().brif(check, near, &[], far, &[]);
        b.switch_to_block(near);
        let compare_right = if case == Case::OtherMinimum {
            max
        } else {
            max_value
        };
        let lo = b.ins().icmp(IntCC::UnsignedLessThan, nice, compare_right);
        b.ins().brif(lo, lower, &[], higher, &[]);
        b.switch_to_block(lower);
        b.ins().jump(minimum, &[nice.into()]);
        b.switch_to_block(higher);
        b.ins().jump(minimum, &[max_value.into()]);
        b.switch_to_block(minimum);
        let minimum_value = b.block_params(minimum)[0];
        b.ins().jump(finish, &[minimum_value.into()]);
        b.switch_to_block(far);
        b.ins().jump(finish, &[nice.into()]);
        b.switch_to_block(finish);
        let result = b.block_params(finish)[0];
        b.ins().store(MemFlags::new(), max_value, out, 0);
        b.ins().store(MemFlags::new(), result, out, 16);
        if case == Case::LiveRepeatedPredicate {
            let repeated = b.ins().uextend(types::I64, check);
            b.ins().store(MemFlags::new(), repeated, out, 24);
        }
        if case == Case::ObserveMaximumAfterMerge {
            // Both selected values remain available after the new merge.
            let before_compare = b.ins().icmp(IntCC::Equal, max_value, nice);
            let observed = b.ins().uextend(types::I64, before_compare);
            b.ins().store(MemFlags::new(), observed, out, 32);
        }
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
    }
    if optimize {
        context.flowgraph();
        let changed = fold(&mut context.func, &context.cfg, &context.domtree);
        assert_eq!(
            changed,
            usize::from(matches!(
                case,
                Case::Exact | Case::WrappedPredicate | Case::ObserveMaximumAfterMerge
            )),
            "{case:?}"
        );
    }
    context.flowgraph();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("shared_clamp", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The declared ABI matches this signature and the owning module stays alive.
    let function =
        unsafe { core::mem::transmute::<*const u8, Entry>(module.get_finalized_function(id)) };
    (module, function)
}

#[test]
fn reuse_current_clamp_decision_and_forward_both_results() {
    let mut count = 0;
    for case in [
        Case::Exact,
        Case::WrappedPredicate,
        Case::OtherPredicate,
        Case::OtherMinimum,
        Case::ExtraFirstEntry,
        Case::ObservableMiddle,
        Case::ObserveMaximumAfterMerge,
        Case::LiveRepeatedPredicate,
    ] {
        let (_old, before) = compile(case, false);
        let (_new, after) = compile(case, true);
        for remaining in [0, 1, 257, 258, 259, i64::MIN as u64, u64::MAX] {
            for max in [0, 3, 257, 258, u64::MAX] {
                for nice in [0, 2, 257, 258, 259, u64::MAX] {
                    for limit in [0, 258, u64::MAX] {
                        for route in [0, 1] {
                            let mut a = [33; 5];
                            let mut b = a;
                            let old = unsafe {
                                before(remaining, max, nice, limit, route, a.as_mut_ptr())
                            };
                            let new = unsafe {
                                after(remaining, max, nice, limit, route, b.as_mut_ptr())
                            };
                            assert_eq!(
                                (old, a),
                                (new, b),
                                "{case:?} {remaining} {max} {nice} {limit} {route}"
                            );
                            if matches!(case, Case::Exact | Case::WrappedPredicate) {
                                if route == 0 {
                                    assert_eq!((new, b), (91, [33; 5]));
                                } else {
                                    let (maximum, minimum) = if remaining < limit {
                                        (remaining, remaining.min(nice))
                                    } else {
                                        (max, nice)
                                    };
                                    assert_eq!((new, b), (minimum, [maximum, 33, minimum, 33, 33]));
                                }
                            }
                            count += 2;
                        }
                    }
                }
            }
        }
    }
    println!("{count} compiled clamp invocations");
}
