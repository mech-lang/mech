use super::*;
use cranelift_codegen::ir::{AbiParam, InstBuilder, MemFlags, types};
use cranelift_frontend::{FunctionBuilder, FunctionBuilderContext};

#[test]
fn cfg_cleanup_unsigned_interval_comparisons_are_universal() {
    use IntCC::*;
    for cc in [
        Equal,
        NotEqual,
        UnsignedLessThan,
        UnsignedLessThanOrEqual,
        UnsignedGreaterThan,
        UnsignedGreaterThanOrEqual,
    ] {
        for lo in 0..8 {
            for hi in lo..8 {
                for other_lo in 0..8 {
                    for other_hi in other_lo..8 {
                        let Some(proved) = compare(cc, Range(lo, hi), Range(other_lo, other_hi))
                        else {
                            continue;
                        };
                        for a in lo..=hi {
                            for b in other_lo..=other_hi {
                                let actual = match cc {
                                    Equal => a == b,
                                    NotEqual => a != b,
                                    UnsignedLessThan => a < b,
                                    UnsignedLessThanOrEqual => a <= b,
                                    UnsignedGreaterThan => a > b,
                                    UnsignedGreaterThanOrEqual => a >= b,
                                    _ => unreachable!(),
                                };
                                assert_eq!(
                                    proved, actual,
                                    "{cc:?} {lo}..{hi} {other_lo}..{other_hi}"
                                );
                            }
                        }
                    }
                }
            }
        }
    }
    assert_eq!(
        compare(
            UnsignedLessThan,
            Range(0, u64::MAX),
            Range(u64::MAX, u64::MAX)
        ),
        None
    );
    assert_eq!(
        compare(
            UnsignedLessThanOrEqual,
            Range(0, u64::MAX),
            Range(u64::MAX, u64::MAX)
        ),
        Some(true)
    );
    assert_eq!(
        compare(
            UnsignedGreaterThan,
            Range(u64::MAX, u64::MAX),
            Range(0, u64::MAX - 1)
        ),
        Some(true)
    );
}

#[test]
fn cfg_cleanup_uses_actual_widths_and_declines_phi_signed_vector_float_and_deep_facts() {
    let mut function = Function::new();
    function.signature.params = [types::I64, types::I8, types::I64X2, types::F64, types::I128]
        .into_iter()
        .map(AbiParam::new)
        .collect();
    let mut fc = FunctionBuilderContext::new();
    let (unknown, proved) = {
        let mut b = FunctionBuilder::new(&mut function, &mut fc);
        let entry = b.create_block();
        let yes = b.create_block();
        let no = b.create_block();
        let join = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.append_block_param(join, types::I64);
        b.switch_to_block(entry);
        let args = b.block_params(entry).to_vec();
        let input = args[0];
        let loaded = b.ins().load(types::I64, MemFlags::new(), input, 0);
        let mut deep = b.ins().band_imm(input, 255);
        for _ in 0..MAX_DEPTH + 2 {
            deep = b.ins().band_imm(deep, -1);
        }
        let deep_test = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, deep, 255);
        let truncated = b.ins().ireduce(types::I8, input);
        let signed = b.ins().icmp_imm(IntCC::SignedLessThan, truncated, 0);
        // I8 immediate -1 is 255 for an unsigned comparison, not U64::MAX.
        let byte_max = b
            .ins()
            .icmp_imm(IntCC::UnsignedLessThanOrEqual, args[1], -1);
        let byte_negative = b.ins().icmp_imm(IntCC::UnsignedGreaterThan, args[1], -128);
        let byte_literal = b.ins().iconst(types::I8, -128);
        let byte_zero = b.ins().iconst(types::I8, 0);
        let max = b.ins().iconst(types::I64, -1);
        let max_test = b.ins().icmp_imm(IntCC::UnsignedGreaterThanOrEqual, max, -1);
        let unknown = vec![
            input,
            args[2],
            args[3],
            args[4],
            loaded,
            deep_test,
            signed,
            byte_negative,
        ];
        b.ins().brif(input, yes, &[], no, &[]);
        b.switch_to_block(yes);
        let a = b.ins().iconst(types::I64, 1);
        b.ins().jump(join, &[a.into()]);
        b.switch_to_block(no);
        let a = b.ins().iconst(types::I64, 256);
        b.ins().jump(join, &[a.into()]);
        b.switch_to_block(join);
        let phi = b.block_params(join)[0];
        let phi_test = b.ins().icmp_imm(IntCC::UnsignedLessThanOrEqual, phi, 255);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
        (
            unknown
                .into_iter()
                .chain([phi, phi_test])
                .collect::<Vec<_>>(),
            [
                (byte_max, true),
                (byte_literal, true),
                (byte_zero, false),
                (max_test, true),
            ],
        )
    };
    let mut facts = Facts::new(&function);
    for value in unknown {
        assert_eq!(facts.condition(value), None, "{value}");
    }
    for (value, truth) in proved {
        assert_eq!(facts.condition(value), Some(truth), "{value}");
    }
}

#[test]
fn selected_fact_peels_only_low_bit_of_an_integer_comparison() {
    let mut function = cranelift_codegen::ir::Function::new();
    function.signature.params.push(AbiParam::new(types::I8));
    let mut frontend = FunctionBuilderContext::new();
    let (input, comparison, selected, lossy, raw) = {
        let mut b = FunctionBuilder::new(&mut function, &mut frontend);
        let entry = b.create_block();
        b.append_block_params_for_function_params(entry);
        b.switch_to_block(entry);
        let input = b.block_params(entry)[0];
        let comparison = b.ins().icmp_imm(IntCC::UnsignedLessThan, input, 8);
        let selected = b.ins().band_imm(comparison, 1);
        let lossy = b.ins().band_imm(comparison, 2);
        let raw = b.ins().band_imm(input, 1);
        b.ins().return_(&[]);
        b.seal_all_blocks();
        b.finalize();
        (input, comparison, selected, lossy, raw)
    };
    for (truth, expected) in [(true, Range(0, 7)), (false, Range(8, 255))] {
        let mut known = BTreeMap::new();
        assert!(condition::assume(&function, selected, truth, &mut known));
        assert_eq!(known.get(&input), Some(&expected));
    }
    for (word, truth) in [(lossy, false), (raw, true), (raw, false)] {
        let mut known = BTreeMap::new();
        assert!(condition::assume(&function, word, truth, &mut known));
        assert!(!known.contains_key(&comparison));
        assert!(!known.contains_key(&input));
    }
}
