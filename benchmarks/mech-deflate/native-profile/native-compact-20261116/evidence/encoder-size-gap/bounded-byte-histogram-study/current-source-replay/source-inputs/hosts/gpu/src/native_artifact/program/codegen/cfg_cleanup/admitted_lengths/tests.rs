use super::*;
use cranelift_codegen::ir::AbiParam;
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module, default_libcall_names};

#[derive(Clone, Copy, Debug)]
enum Case {
    Exact,
    Strict,
    Reversed,
    Wrapped,
    Stricter,
    Other,
    Mutable,
    Signed,
    Equal,
    SignedNegative,
    Masked,
    MaskedReversed,
    MaskedRoundTrip,
    MaskedReject,
    MaskedInclusive,
    MaskedEdge,
    MaskedSigned,
    MaskedWrap,
    MaskedDifferent,
    MaskedChanged,
}

type Run = unsafe extern "C" fn(u64, u64, u64) -> u64;

fn compile(case: Case, optimize: bool) -> (JITModule, Run) {
    use IntCC::*;
    let mut module = JITModule::new(
        JITBuilder::with_flags(
            &[("opt_level", "speed"), ("regalloc_checker", "true")],
            default_libcall_names(),
        )
        .unwrap(),
    );
    let mut context = module.make_context();
    context.func.signature.params = vec![AbiParam::new(types::I64); 3];
    context.func.signature.returns = vec![AbiParam::new(types::I64)];
    let mut builder = FunctionBuilderContext::new();
    let length;
    {
        let mut b = FunctionBuilder::new(&mut context.func, &mut builder);
        let entry = b.create_block();
        let prior = b.create_block();
        let admitted = b.create_block();
        let end = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(admitted, types::I64);
        b.append_block_param(end, types::I64);
        b.switch_to_block(entry);
        let [current, other, arithmetic_ok] = b.block_params(entry).try_into().unwrap();
        let earlier_rejection = b.ins().iconst(types::I64, 10);
        b.ins()
            .brif(arithmetic_ok, prior, &[], end, &[earlier_rejection.into()]);
        b.switch_to_block(prior);
        let guard = b.ins().icmp_imm(UnsignedGreaterThanOrEqual, current, 8);
        let original_fallback = b.ins().iconst(types::I64, 11);
        b.ins().brif(
            guard,
            admitted,
            &[current.into()],
            end,
            &[original_fallback.into()],
        );
        b.switch_to_block(admitted);
        let current = b.block_params(admitted)[0];
        length = current;
        let check = match case {
            Case::Masked
            | Case::MaskedReversed
            | Case::MaskedRoundTrip
            | Case::MaskedReject
            | Case::MaskedInclusive
            | Case::MaskedEdge
            | Case::MaskedSigned
            | Case::MaskedWrap
            | Case::MaskedDifferent
            | Case::MaskedChanged => {
                let mask = if matches!(case, Case::MaskedInclusive | Case::MaskedEdge) {
                    8
                } else {
                    7
                };
                let mut index = b.ins().band_imm(other, mask);
                if matches!(case, Case::MaskedRoundTrip) {
                    let one_based = b.ins().iadd_imm(index, 1);
                    index = b.ins().iadd_imm(one_based, -1);
                }
                if matches!(case, Case::MaskedWrap) {
                    index = b.ins().iadd_imm(index, -1);
                }
                let length = if matches!(case, Case::MaskedDifferent) {
                    arithmetic_ok
                } else if matches!(case, Case::MaskedChanged) {
                    b.ins().iadd_imm(current, -1)
                } else {
                    current
                };
                let condition = match case {
                    Case::MaskedReject => UnsignedGreaterThanOrEqual,
                    Case::MaskedInclusive => UnsignedLessThanOrEqual,
                    Case::MaskedSigned => SignedLessThan,
                    _ => UnsignedLessThan,
                };
                if matches!(case, Case::MaskedReversed) {
                    b.ins().icmp(UnsignedGreaterThan, length, index)
                } else {
                    b.ins().icmp(condition, index, length)
                }
            }
            Case::Exact => b.ins().icmp_imm(UnsignedGreaterThanOrEqual, current, 8),
            Case::Strict => b.ins().icmp_imm(UnsignedGreaterThan, current, 7),
            Case::Reversed => {
                let eight = b.ins().iconst(types::I64, 8);
                b.ins().icmp(UnsignedLessThanOrEqual, eight, current)
            }
            Case::Wrapped => {
                let cmp = b.ins().icmp_imm(UnsignedGreaterThanOrEqual, current, 8);
                let wide = b.ins().uextend(types::I64, cmp);
                let bit = b.ins().band_imm(wide, 1);
                b.ins().icmp_imm(Equal, bit, 1)
            }
            Case::Stricter => b.ins().icmp_imm(UnsignedGreaterThan, current, 8),
            Case::Other => b.ins().icmp_imm(UnsignedGreaterThanOrEqual, other, 8),
            Case::Mutable => {
                let changed = b.ins().iadd_imm(current, -1);
                b.ins().icmp_imm(UnsignedGreaterThanOrEqual, changed, 8)
            }
            Case::Signed => b.ins().icmp_imm(SignedGreaterThanOrEqual, current, 8),
            Case::Equal => b.ins().icmp_imm(Equal, current, 8),
            Case::SignedNegative => b.ins().icmp_imm(UnsignedGreaterThanOrEqual, current, -1),
        };
        let rejection = b.ins().iconst(types::I64, 12);
        b.ins()
            .brif(check, end, &[current.into()], end, &[rejection.into()]);
        b.switch_to_block(end);
        let result = b.block_params(end)[0];
        b.ins().return_(&[result]);
        b.seal_all_blocks();
        b.finalize();
    }
    if optimize {
        let floors = BTreeMap::from([(length, 8)]);
        if matches!(
            case,
            Case::Masked
                | Case::MaskedReversed
                | Case::MaskedRoundTrip
                | Case::MaskedReject
                | Case::MaskedInclusive
                | Case::MaskedEdge
                | Case::MaskedSigned
                | Case::MaskedWrap
                | Case::MaskedDifferent
                | Case::MaskedChanged
        ) {
            let (literals, ranged) = fold_index_ranges(&mut context.func, &floors);
            assert_eq!(literals, 0, "{case:?}");
            assert_eq!(
                ranged,
                usize::from(matches!(
                    case,
                    Case::Masked
                        | Case::MaskedReversed
                        | Case::MaskedRoundTrip
                        | Case::MaskedReject
                        | Case::MaskedInclusive
                )),
                "{case:?}"
            );
        } else {
            assert_eq!(
                fold(&mut context.func, &floors),
                usize::from(matches!(
                    case,
                    Case::Exact | Case::Strict | Case::Reversed | Case::Wrapped
                )),
                "{case:?}"
            );
        }
    }
    context.compute_cfg();
    context.compute_domtree();
    context.verify(module.isa()).unwrap();
    let id = module
        .declare_function("floor", Linkage::Local, &context.func.signature)
        .unwrap();
    module.define_function(id, &mut context).unwrap();
    module.finalize_definitions().unwrap();
    // The function has the declared three-word ABI and its module stays alive.
    let f = unsafe { core::mem::transmute::<*const u8, Run>(module.get_finalized_function(id)) };
    (module, f)
}

#[test]
fn admitted_unsigned_length_retains_fallback_and_selected_values() {
    for case in [
        Case::Exact,
        Case::Strict,
        Case::Reversed,
        Case::Wrapped,
        Case::Stricter,
        Case::Other,
        Case::Mutable,
        Case::Signed,
        Case::Equal,
        Case::SignedNegative,
        Case::Masked,
        Case::MaskedReversed,
        Case::MaskedRoundTrip,
        Case::MaskedReject,
        Case::MaskedInclusive,
        Case::MaskedEdge,
        Case::MaskedSigned,
        Case::MaskedWrap,
        Case::MaskedDifferent,
        Case::MaskedChanged,
    ] {
        let (_old, original) = compile(case, false);
        let (_new, optimized) = compile(case, true);
        for current in [
            0,
            1,
            7,
            8,
            9,
            16,
            i64::MAX as u64,
            i64::MIN as u64,
            u64::MAX,
        ] {
            for other in [0, 1, 6, 7, 8, 9, i64::MIN as u64, u64::MAX] {
                for arithmetic_ok in [0, 1] {
                    assert_eq!(
                        unsafe { original(current, other, arithmetic_ok) },
                        unsafe { optimized(current, other, arithmetic_ok) },
                        "{case:?}: {current} {other} {arithmetic_ok}"
                    );
                }
            }
        }
    }
}
