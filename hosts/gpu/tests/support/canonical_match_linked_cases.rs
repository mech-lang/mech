//! Wider inputs only; maintained resident/JIT execution supplies the oracles.
use super::*;
use mech_core::snapshot::F64Bits;

const SIGNED: &str = "(enabled<bool>? | true => left<i64> / divisor<i64> | false => left<i64>)\n";
const FLOATING: &str = "(value<f64>? | 0.0 => 7.0 | found => found)\n";

pub(super) fn fixtures() -> Vec<(String, String)> {
    let mut result = vec![
        ("canonical-match-signed".into(), SIGNED.into()),
        ("canonical-match-floating".into(), FLOATING.into()),
    ];
    result.extend(match_cast_cases::CASTS.iter().map(|cast| {
        let zero = match cast.output {
            NativeScalarKind::U8 => "0u8",
            NativeScalarKind::U16 => "0u16",
            NativeScalarKind::U32 => "0u32",
            NativeScalarKind::U64 => "0u64",
            NativeScalarKind::I64 => "0i64",
            NativeScalarKind::F64 => "0.0",
            NativeScalarKind::Bool => unreachable!(),
        };
        (
            cast.name.replace("canonical-linked", "canonical-match"),
            cast.source.replace(
                "\nresult\n",
                &if matches!(
                    cast.output,
                    NativeScalarKind::U8 | NativeScalarKind::U16 | NativeScalarKind::U32
                ) {
                    "\n(enabled<bool>? | true => result | false => result)\n".to_owned()
                } else {
                    format!("\n(enabled<bool>? | true => result | false => result + {zero})\n")
                },
            ),
        )
    }));
    result
}

pub(super) fn contains(name: &str) -> bool {
    fixtures().iter().any(|(fixture, _)| fixture == name)
}

fn literal_failure(name: &str, args: Vec<String>, diagnostic: &str) -> Value {
    let mut case = failure_case(name, args, diagnostic);
    case["stderr"] = json!(format!("{diagnostic}\n"));
    case
}

pub(super) fn reference_cases(name: &str, artifacts: &[ProgramArtifact; 2]) -> Vec<Value> {
    use NativeScalarValue::{Bool, F64, I64, U64};
    let plans = artifacts.each_ref().map(|artifact| {
        lower_scalar_match_artifact(artifact, &mech_stdlib::source_catalog()).unwrap()
    });
    let jits = plans
        .each_ref()
        .map(|plan| JitMatchArtifact::compile(plan).unwrap());
    let cast = match_cast_cases::CASTS
        .into_iter()
        .find(|cast| cast.name.replace("canonical-linked", "canonical-match") == name);
    let (samples, good): (Vec<Vec<(&str, NativeScalarValue)>>, _) =
        if name == "canonical-match-signed" {
            let sample = |enabled, left, divisor| {
                vec![
                    ("enabled", Bool(enabled)),
                    ("left", I64(left)),
                    ("divisor", I64(divisor)),
                ]
            };
            (
                vec![
                    sample(false, i64::MIN, -1),
                    sample(true, i64::MIN, -1),
                    sample(false, 8, 0),
                    sample(true, 8, 0),
                    sample(true, i64::MIN, 1),
                    sample(true, -8, 2),
                    sample(true, i64::MAX, 1),
                ],
                sample(true, 8, 2),
            )
        } else if name == "canonical-match-floating" {
            let samples = [
                0,
                1u64 << 63,
                1,
                0x3ff0_0000_0000_0000,
                0x7ff0_0000_0000_0000,
                0xfff0_0000_0000_0000,
                0x7ff8_1234_5678_9abc,
                0x7ff0_0000_0000_0001,
            ]
            .into_iter()
            .map(|bits| vec![("value", F64(F64Bits::from_bits(bits)))])
            .collect();
            (samples, vec![("value", F64(F64Bits::from_bits(0)))])
        } else {
            let cast = cast.expect("known wider Match fixture");
            for plan in &plans {
                assert!(plan.blocks().iter().any(|block| matches!(
                    block.terminator(),
                    mech_engine::native_artifact::NativeMatchTerminator::LiteralBranch { .. }
                )));
                assert!(plan.blocks().iter().any(|block| matches!(block.action(),
                NativeMatchAction::Primitive { op, .. } if *op == cast.op)));
                assert!(
                    plan.outputs()
                        .iter()
                        .any(|output| output.ty().kind() == cast.output)
                );
            }
            let samples = [false, true]
                .into_iter()
                .flat_map(|enabled| {
                    match_cast_cases::samples(cast)
                        .into_iter()
                        .map(move |value| vec![("enabled", Bool(enabled)), ("value", value)])
                })
                .collect();
            let zero = match cast.input {
                NativeScalarKind::U8 => NativeScalarValue::U8(0),
                NativeScalarKind::U16 => NativeScalarValue::U16(0),
                NativeScalarKind::U32 => NativeScalarValue::U32(0),
                NativeScalarKind::U64 => U64(0),
                NativeScalarKind::I64 => I64(0),
                NativeScalarKind::F64 => F64(F64Bits::from_bits(0)),
                NativeScalarKind::Bool => unreachable!(),
            };
            (samples, vec![("enabled", Bool(true)), ("value", zero)])
        };
    let mut cases = Vec::new();
    for (index, named) in samples.iter().enumerate() {
        let args = inputs(&artifacts[0], named);
        let label = format!("numeric-{index:03}");
        match resident(&artifacts[0], &args) {
            Ok(expected) => {
                for variant in 0..2 {
                    assert_eq!(resident(&artifacts[variant], &args).unwrap(), expected);
                    assert_eq!(jit_success(&jits[variant], &args).0, expected);
                }
                cases.push(success_case(&label, arguments(&args), &expected));
            }
            Err(_) => {
                assert!(resident(&artifacts[1], &args).is_err());
                let error = jit_error(&jits[0], &args, NativeExecutionLimits::default());
                assert!(matches!(
                    error,
                    NativeMatchExecutionError::Arithmetic { .. }
                ));
                assert_eq!(
                    jit_error(&jits[1], &args, NativeExecutionLimits::default()),
                    error
                );
                cases.push(exact_failure_case(&label, arguments(&args), &error));
            }
        }
    }
    let args = inputs(&artifacts[0], &good);
    let expected = resident(&artifacts[0], &args).unwrap();
    let (recovered, work) = jit_success(&jits[0], &args);
    assert_eq!(recovered, expected);
    assert_eq!(jit_success(&jits[1], &args).0, expected);
    cases.push(success_case(
        "recovery-after-checked-failures",
        arguments(&args),
        &expected,
    ));
    cases.push(success_case(
        "explicit-boundary",
        std::iter::once("--".into())
            .chain(arguments(&args))
            .collect(),
        &expected,
    ));
    for (label, option, limits) in [
        (
            "setup-denial",
            "--work=0".to_owned(),
            NativeExecutionLimits {
                max_work: 0,
                ..NativeExecutionLimits::default()
            },
        ),
        (
            "completion-denial",
            format!("--work={}", work - 1),
            NativeExecutionLimits {
                max_work: work - 1,
                ..NativeExecutionLimits::default()
            },
        ),
        (
            "memory-denial",
            "--memory=0".to_owned(),
            NativeExecutionLimits {
                max_transport_bytes: 0,
                ..NativeExecutionLimits::default()
            },
        ),
    ] {
        let error = jit_error(&jits[0], &args, limits);
        assert_eq!(jit_error(&jits[1], &args, limits), error);
        cases.push(exact_failure_case(
            label,
            with_limit(&option, &args),
            &error,
        ));
    }
    let mut outputs = vec![U64(77); plans[0].outputs().len()];
    let budget = mech_core::ManagedMemoryBudget::new(0);
    let error = jits[0]
        .evaluate_into(
            &args,
            &mut outputs,
            NativeExecutionLimits::default(),
            Some(&budget),
        )
        .unwrap_err();
    assert!(matches!(
        error,
        NativeMatchExecutionError::Base(NativeExecutionError::Memory { .. })
    ));
    assert!(outputs.iter().all(|value| *value == U64(77)));
    assert_eq!(budget.used_bytes(), 0);
    cases.push(exact_failure_case(
        "shared-budget-denial",
        with_limit("--budget=0", &args),
        &error,
    ));
    // Object frame size is filled from that exact object's checked metadata by
    // the parent worker; it need not equal the JIT's frame size.
    cases.push(failure_case(
        "stack-denial",
        with_limit("--stack=0", &args),
        "StackLimit",
    ));
    for (ordinal, ty) in plans[0].inputs().iter().enumerate() {
        let (invalid, diagnostic) = match ty.declaration().ty().kind() {
            NativeScalarKind::Bool => ("TRUE", "invalid Bool argument"),
            NativeScalarKind::U8 => ("256", "invalid U8 argument"),
            NativeScalarKind::U16 => ("65536", "invalid U16 argument"),
            NativeScalarKind::U32 => ("4294967296", "invalid U32 argument"),
            NativeScalarKind::U64 => ("18446744073709551616", "invalid U64 argument"),
            NativeScalarKind::I64 => ("9223372036854775808", "invalid I64 argument"),
            NativeScalarKind::F64 => ("f64:000000000000000g", "invalid F64 bit argument"),
        };
        let mut invalid_args = arguments(&args);
        invalid_args[ordinal] = invalid.into();
        invalid_args.insert(0, "--".into());
        cases.push(literal_failure(
            &format!("invalid-typed-input-{ordinal}"),
            invalid_args,
            diagnostic,
        ));
    }
    cases.push(literal_failure(
        "missing-input",
        vec![],
        &format!("expected {} typed inputs", args.len()),
    ));
    let mut extra = arguments(&args);
    extra.push("tail".into());
    cases.push(literal_failure(
        "extra-input",
        extra,
        &format!("expected {} typed inputs", args.len()),
    ));
    if invalid_unicode_argument().is_some() {
        let mut invalid = literal_failure(
            "invalid-unicode-input",
            arguments(&args),
            "argument is not Unicode",
        );
        invalid["invalid_os_argument"] = json!(0);
        cases.push(invalid);
    }
    cases.push(success_case(
        "recovery-after-denials",
        arguments(&args),
        &expected,
    ));
    cases
}
