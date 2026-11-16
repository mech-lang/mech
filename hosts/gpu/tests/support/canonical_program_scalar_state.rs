//! Exact private scalar state used by real mixed-kind numeric loops.
use super::{artifacts, equal, inputs, limits, reference};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_engine::native_artifact::NativeControlLoop;
use mech_gpu::native_artifact::{JitProgramArtifact, emit_program_object};

const SIGNED_PAIR: &str = "walk(left<i64>, total<i64>) => <i64>\n  | (left, total) => ((left > 0i64)? | true => walk(left - 1i64, total + 3i64) | false => total).\nwalk(steps<i64>, seed<i64>)\n";
const MIXED_ENUM: &str = "#Choose(n<u64>, best<u32>, item<u32>) => <u32>\n  | :Try(left<u64>, best<u32>, item<u32>)\n  | :Done(value<u32>).\n#Choose(n, best, item) -> :Try(n, best, item)\n  :Try(left, best, item)\n    | left > 0u64 -> :Try(left - 1u64, item, best)\n    | * -> :Done(best)\n  :Done(value) => value.\n#Choose(steps<u64>, best<u32>, item<u32>)\n";

fn check_scalar_loop(source: &str, signed: bool) {
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = jit.descriptor().plan();
        let aggregates = plan.scalar_aggregates();
        let carrier = plan
            .iterations()
            .iter()
            .find_map(|control| match control {
                NativeControlLoop::Tail(t) => Some(t.carrier()),
                _ => None,
            })
            .unwrap();
        assert!(
            aggregates.slot(carrier).is_some(),
            "mixed scalar carrier must use private native words: signed={signed}"
        );
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        for steps in [0, 1, 2, 64] {
            let named = if signed {
                vec![("steps", Data::I64(steps)), ("seed", Data::I64(-211))]
            } else {
                vec![
                    ("steps", Data::U64(steps as u64)),
                    ("best", Data::U32(0xfedc_ba98)),
                    ("item", Data::U32(0x8765_4321)),
                ]
            };
            let args = inputs(&artifact, &named, &memory);
            let expected = reference(&artifact, &args);
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            equal(&artifact, &output, &expected);
            let actual = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            assert_eq!(
                actual,
                if signed {
                    Data::I64(-211 + 3 * steps)
                } else {
                    Data::U32(if steps % 2 == 0 {
                        0xfedc_ba98
                    } else {
                        0x8765_4321
                    })
                }
            );
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn signed_pair_loop_selects_existing_native_word_storage() {
    check_scalar_loop(SIGNED_PAIR, true);
}

#[test]
fn mixed_enum_loop_selects_existing_native_word_storage() {
    check_scalar_loop(MIXED_ENUM, false);
}

pub(super) const ALL_KINDS: &str = "#Carry(n<u64>, flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>) => <(bool,u8,u16,u32,u64,i64,f64)>\n  | :Try(left<u64>, flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>)\n  | :Done(flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>).\n#Carry(n, flag, byte, half, word, wide, signed, fraction) -> :Try(n, flag, byte, half, word, wide, signed, fraction)\n  :Try(left, flag, byte, half, word, wide, signed, fraction)\n    | left > 0u64 -> :Try(left - 1u64, flag == false, byte, half, word, wide, signed, fraction)\n    | * -> :Done(flag, byte, half, word, wide, signed, fraction)\n  :Done(flag, byte, half, word, wide, signed, fraction) => (flag, byte, half, word, wide, signed, fraction).\n#Carry(steps<u64>, flag<bool>, byte<u8>, half<u16>, word<u32>, wide<u64>, signed<i64>, fraction<f64>)\n";

#[test]
fn mixed_scalar_state_preserves_bits_fresh_flags_and_zero_allocation_slope() {
    use mech_core::snapshot::F64Bits;
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(ALL_KINDS).0)
    {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let plan = jit.descriptor().plan();
        let aggregates = plan.scalar_aggregates();
        assert!(aggregates.words() > 0);
        for i in 0..plan.transport_output_count() {
            assert!(aggregates.slot(plan.transport_output_value(i)).is_none());
        }
        let object = emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(
            object.descriptor().typed_ir_sha256(),
            jit.descriptor().typed_ir_sha256()
        );
        for (signed, bits) in [
            (i64::MIN, 0x8000_0000_0000_0000),
            (i64::MAX, 0x7ff8_0000_0000_0042),
        ] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut observations = Vec::new();
            for steps in [0, 1, 2, 63, 64] {
                let args = inputs(
                    &artifact,
                    &[
                        ("steps", Data::U64(steps)),
                        ("flag", Data::Bool(true)),
                        ("byte", Data::U8(0xef)),
                        ("half", Data::U16(0xdcba)),
                        ("word", Data::U32(0xfedc_ba98)),
                        ("wide", Data::U64(0xfedc_ba98_7654_3210)),
                        ("signed", Data::I64(signed)),
                        ("fraction", Data::F64(F64Bits::from_bits(bits))),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let expected = reference(&artifact, &args);
                let mut output = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut output, limits(), &memory)
                    .unwrap();
                let (result, requests, requested_bytes) =
                    super::allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, limits(), &memory)
                    });
                result.unwrap();
                equal(&artifact, &output, &expected);
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Bool(steps % 2 == 0),
                            Data::U8(0xef),
                            Data::U16(0xdcba),
                            Data::U32(0xfedc_ba98),
                            Data::U64(0xfedc_ba98_7654_3210),
                            Data::I64(signed),
                            Data::F64(F64Bits::from_bits(bits)),
                        ]
                        .into()
                    )
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                println!(
                    "{}",
                    serde_json::json!({"witness":"mixed-scalar-state","artifact":form,"steps":steps,"signed":signed,"float_bits":bits,"requests":requests,"requested_bytes":requested_bytes})
                );
                observations.push((requests, requested_bytes));
            }
            assert!(
                observations.iter().all(|row| *row == observations[0]),
                "private visits must not allocate: {observations:?}"
            );
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn checked_mixed_visits_keep_error_order_publication_and_recovery() {
    for arithmetic_first in [false, true] {
        let earlier = if arithmetic_first {
            "  earlier := 12i64 / divisor\n"
        } else {
            ""
        };
        let source = format!(
            "walk(left<u64>, allowed<bool>, next<bool>, divisor<i64>) => (out<i64>) :=\n{earlier}  enabled! := allowed\n  out := ((left > 0u64)? | true => walk(left - 1u64, next, next, divisor) | false => 12i64 / divisor).\nwalk(3u64, true, allowed<bool>, divisor<i64>)\n"
        );
        for artifact in artifacts(&source).0 {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(jit.descriptor().plan().scalar_aggregates().words() > 0);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut output = vec![None; artifact.outputs().len()];
            for (allowed, divisor) in [(true, 3), (false, 0), (true, 0), (true, 2)] {
                let args = inputs(
                    &artifact,
                    &[
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::I64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = output.clone();
                let before = memory.used_bytes();
                let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
                let resident = super::reference_result(&artifact, &args);
                if allowed && divisor != 0 {
                    result.unwrap();
                    equal(&artifact, &output, &resident.unwrap());
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::I64(12 / divisor)
                    );
                } else {
                    let native = format!("{:?}", result.unwrap_err());
                    let resident = resident.unwrap_err();
                    let assertion = !allowed && !arithmetic_first;
                    assert_eq!(native.contains("Integrity"), assertion, "{native}");
                    assert_eq!(
                        resident.contains("LexicalIntegrity"),
                        assertion,
                        "{resident}"
                    );
                    assert_eq!(memory.used_bytes(), before);
                    assert!(output.iter().zip(&prior).all(|(v, p)| {
                        v.as_ref()
                            .unwrap()
                            .shares_frozen_storage(p.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            let args = inputs(
                &artifact,
                &[("allowed", Data::Bool(true)), ("divisor", Data::I64(3))],
                &memory,
            );
            let report = jit
                .evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            let prior = output.clone();
            let retained = memory.used_bytes();
            let mut denied = limits();
            denied.execution.max_work = report.work - 1;
            assert!(
                jit.evaluate_into(&args, &mut output, denied, &memory)
                    .is_err()
            );
            assert_eq!(memory.used_bytes(), retained);
            assert!(output.iter().zip(&prior).all(|(v, p)| {
                v.as_ref()
                    .unwrap()
                    .shares_frozen_storage(p.as_ref().unwrap())
            }));
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
            drop((args, prior, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn mixed_enum_constants_and_unsupported_nested_payloads_keep_exact_semantics() {
    let constant = "#Pick(n<u64>) => <i64>\n  | :Try(left<u64>, flag<bool>, value<i64>)\n  | :Done(value<i64>).\n#Pick(n) -> :Try(n, false, -211i64)\n  :Try(left, flag, value)\n    | left > 0u64 -> :Try(left - 1u64, flag == false, value)\n    | * -> :Done(value)\n  :Done(value) => value.\n#Pick(steps<u64>)\n";
    for artifact in artifacts(constant).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        assert!(jit.descriptor().plan().scalar_aggregates().words() > 0);
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(&artifact, &[("steps", Data::U64(4))], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
    }
    let nested = "#Pick(n<u64>, value<(u32,u32)>) => <u32>\n  | :Try(left<u64>, pair<(u32,u32)>)\n  | :Done(value<u32>).\n#Pick(n, value) -> :Try(n, value)\n  :Try(left, pair)\n    | left > 0u64 -> :Try(left - 1u64, pair)\n    | * -> { (a,b) := pair } -> :Done(a)\n  :Done(value) => value.\n#Pick(steps<u64>, (best<u32>, item<u32>))\n";
    for artifact in artifacts(nested).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = jit.descriptor().plan();
        for control in plan.iterations() {
            if let NativeControlLoop::Tail(t) = control {
                assert!(plan.scalar_aggregates().slot(t.carrier()).is_none());
            }
        }
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                ("steps", Data::U64(4)),
                ("best", Data::U32(u32::MAX)),
                ("item", Data::U32(19)),
            ],
            &memory,
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(&artifact, &output, &reference(&artifact, &args));
    }
}
