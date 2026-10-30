//! Ordinary parent frames expose current scalar words around native children.
use super::super::{artifacts, equal, inputs, reference_result};
use mech_core::ValueDataDraft as Data;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const SOURCE: &str = r#"inner(xs<[u32]>, left<u64>, total<u64>) => (out<u64>) :=
  word<u64> := xs[1u64]
  out := ((left > 0u64)? | true => inner(xs, left - 1u64, total + word) | false => total).
outer(xs<[u32]>, left<u64>, total<u64>, flag<bool>) => (out<(u64,bool)>) :=
  next := inner(xs, 1u64, total)
  out := ((left > 0u64)? | true => outer(xs, left - 1u64, next, flag == false) | false => (total, flag)).
(total, flag) := outer(data<[u32]>, steps<u64>, seed<u64>, flag<bool>)
(total + 0u64, flag == true)
"#;

#[test]
fn ordinary_frame_scalar_reads_execute_without_projection_callbacks() {
    for artifact in artifacts(super::SCALAR_READ_PARENT).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        assert!(
            metadata["compiled_frame_scalar_reads"]
                .as_array()
                .is_some_and(|r| !r.is_empty()),
            "ordinary parent-frame scalar projections still use callbacks"
        );
    }
}

#[test]
fn ordinary_frame_scalar_reads_keep_fresh_recursive_and_repeated_arguments() {
    for artifact in artifacts(SOURCE).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        let mut rows = Vec::new();
        for (steps, flag, seed, word) in [
            (0, true, 17, 7),
            (1, false, 19, 11),
            (3, true, 29, 13),
            (8, false, 31, 17),
            (1, true, 37, 19),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix(vec![Data::U32(word)].into())),
                    ("steps", Data::U64(steps)),
                    ("seed", Data::U64(seed)),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let report = jit
                .evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::U64(seed + steps * u64::from(word)),
                        Data::Bool(flag ^ (steps % 2 == 1))
                    ]
                    .into()
                )
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            rows.push((steps, report.visits, report.work));
        }
        println!(
            "{}",
            serde_json::json!({"witness":"ordinary-frame-scalar-reads","observations":rows})
        );
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn ordinary_frame_scalar_reads_keep_all_scalar_kinds_and_exact_float_bits() {
    use mech_core::snapshot::F64Bits;
    let child = SOURCE.split("outer(xs").next().unwrap();
    let source = format!(
        "{child}\nwalk(xs<[u32]>, left<u64>, flag<bool>, byte<u8>, half<u16>, packed<u32>, signed<i64>, fraction<f64>) => (out<(bool,u8,u16,u32,i64,f64,u64)>) :=\n  tick := inner(xs, 1u64, 0u64)\n  out := ((left > 0u64)? | true => walk(xs,left - 1u64,flag == false,byte,half,packed,signed + 1i64,fraction) | false => (flag,byte,half,packed,signed,fraction,tick)).\n(a,b,c,d,e,f,g) := walk(data<[u32]>, steps<u64>, flag<bool>, byte<u8>, half<u16>, packed<u32>, signed<i64>, fraction<f64>)\n(a == true,b,c,d,e,f,g + 0u64)\n"
    );
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (steps, flag, signed, bits) in [
            (3, true, i64::MIN, 0x8000_0000_0000_0000),
            (2, false, -2, 0x7ff8_0000_0000_0042),
            (1, false, 11, 0x3ff0_0000_0000_0000),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", Data::Matrix(vec![Data::U32(7)].into())),
                    ("steps", Data::U64(steps)),
                    ("flag", Data::Bool(flag)),
                    ("byte", Data::U8(239)),
                    ("half", Data::U16(56506)),
                    ("packed", Data::U32(0x9abc_def0)),
                    ("signed", Data::I64(signed)),
                    ("fraction", Data::F64(F64Bits::from_bits(bits))),
                ],
                &memory,
            );
            let aliases = args.clone();
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Bool(flag ^ (steps % 2 == 1)),
                        Data::U8(239),
                        Data::U16(56506),
                        Data::U32(0x9abc_def0),
                        Data::I64(signed + steps as i64),
                        Data::F64(F64Bits::from_bits(bits)),
                        Data::U64(7)
                    ]
                    .into()
                )
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        super::super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn ordinary_frame_scalar_reads_preserve_error_order_publication_and_retry() {
    for arithmetic_first in [false, true] {
        let earlier = if arithmetic_first {
            "  earlier := total + 1u64\n"
        } else {
            ""
        };
        let source = SOURCE
            .replace("flag<bool>) =>", "flag<bool>, permit<bool>) =>")
            .replace(
                "  next := inner",
                &format!("{earlier}  allowed! := permit\n  next := inner"),
            )
            .replace("next, flag == false)", "next, flag == false, permit)")
            .replace(
                "seed<u64>, flag<bool>)",
                "seed<u64>, flag<bool>, permit<bool>)",
            );
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for (seed, permit, empty) in [
                (17, true, false),
                (u64::MAX, false, false),
                (19, true, false),
                (u64::MAX, true, false),
                (17, false, true),
                (17, true, true),
                (23, true, false),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            Data::Matrix(if empty { vec![] } else { vec![Data::U32(7)] }.into()),
                        ),
                        ("steps", Data::U64(2)),
                        ("seed", Data::U64(seed)),
                        ("flag", Data::Bool(true)),
                        ("permit", Data::Bool(permit)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let value = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let resident = reference_result(&artifact, &args);
                if permit && seed != u64::MAX && !empty {
                    value.unwrap();
                    equal(&artifact, &out, &resident.unwrap());
                } else {
                    let error = format!("{:?}", value.unwrap_err());
                    let oracle = resident.unwrap_err();
                    let assertion = !permit && !(arithmetic_first && seed == u64::MAX);
                    assert_eq!(error.contains("Integrity"), assertion, "{error}");
                    assert_eq!(oracle.contains("LexicalIntegrity"), assertion, "{oracle}");
                    assert_eq!(
                        before,
                        (memory.used_bytes(), memory.retained_snapshot_nodes())
                    );
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn ordinary_frame_scalar_reads_preserve_finite_denials_and_recovery() {
    for artifact in artifacts(SOURCE).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[
                ("data", Data::Matrix(vec![Data::U32(7)].into())),
                ("steps", Data::U64(3)),
                ("seed", Data::U64(17)),
                ("flag", Data::Bool(true)),
            ],
            &memory,
        );
        let mut out = vec![None; artifact.outputs().len()];
        let report = jit
            .evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        let mut failures = 0;
        for library in [false, true] {
            for budget in [
                report.work / 4,
                report.work / 2,
                report.work - 1,
                report.work,
                report.work + 2048,
            ] {
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut limits = mode.execution();
                if library {
                    limits.library.max_compute_work = Some(budget);
                } else {
                    limits.execution.max_work = budget;
                }
                match jit.evaluate_into(&args, &mut out, limits, &memory) {
                    Ok(_) => equal(
                        &artifact,
                        &out,
                        &reference_result(&artifact, &args).unwrap(),
                    ),
                    Err(error) => {
                        failures += 1;
                        assert!(format!("{error:?}").contains("Limit"), "{error:?}");
                        assert_eq!(
                            before,
                            (memory.used_bytes(), memory.retained_snapshot_nodes())
                        );
                        assert!(out.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                    }
                }
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
            }
        }
        assert!(failures > 0);
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
