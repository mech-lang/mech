//! Existing scalar emission under one proved, funded admission frontier.
use super::{allocation_probe, artifacts, equal, inputs, reference_result};
use mech_core::{Value, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

pub(super) const SOURCE: &str = include_str!("../fixtures/native/scalar-path-admission.mec");
fn matrix(words: impl IntoIterator<Item = u64>) -> Data {
    Data::Matrix(words.into_iter().map(Data::U64).collect())
}
fn result(out: &[Option<Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn selected(artifact: &mech_engine::ProgramArtifact) {
    let metadata = emit_program_object(artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    assert!(
        metadata["ordinary_scalar_admission_groups"]
            .as_u64()
            .unwrap_or(0)
            > 0,
        "{metadata}"
    );
}

fn grouped_diamond(jit: &JitProgramArtifact) {
    use mech_engine::native_artifact::{
        NativeIterationTerminator as Terminator, NativeMatchTerminator as Term,
    };
    let image: serde_json::Value =
        serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
    let groups = image["descriptor"]["map_admission"]["groups"]
        .as_array()
        .unwrap();
    assert!(
        groups.iter().filter(|g| g["owner"].is_null()).any(|group| {
            let sites = group["sites"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<Vec<_>>();
            sites.iter().any(|&site| {
                match jit.descriptor().plan().blocks()[site as usize].terminator() {
                    Terminator::Scalar(
                        Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. },
                    ) => sites.contains(&yes) && sites.contains(&no),
                    _ => false,
                }
            })
        }),
        "fixture must fund both arms of one source branch, not merely a linear prelude"
    );
}

#[test]
fn scalar_paths_keep_current_branches_errors_publication_and_recovery() {
    for arithmetic_first in [false, true] {
        let source = if arithmetic_first {
            SOURCE.replace(
                "  allowed! := permit\n  advanced := word + 1u64",
                "  advanced := word + 1u64\n  allowed! := permit",
            )
        } else {
            SOURCE.into()
        };
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            let mut rows = Vec::new();
            for (word, flag, permit, divisor) in [
                (7, true, true, 1),
                (11, false, true, 2),
                (u64::MAX, true, false, 0),
                (7, true, false, 0),
                (7, false, true, 0),
                (u64::MAX / 3 - 3, true, true, 2),
                (u64::MAX / 3 - 3, false, true, 1),
                (19, true, true, 3),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("words", matrix([word, 99])),
                        ("flag", Data::Bool(flag)),
                        ("permit", Data::Bool(permit)),
                        ("divisor", Data::U64(divisor)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let oracle = reference_result(&artifact, &args);
                let model = word
                    .checked_add(1)
                    .and_then(|n| n.checked_mul(if flag { 3 } else { 7 }))
                    .and_then(|n| n.checked_add(if flag { 5 } else { 11 }))
                    .and_then(|n| n.checked_div(divisor))
                    .and_then(|n| n.checked_add(17));
                if permit && model.is_some() {
                    let report = run.unwrap();
                    equal(&artifact, &out, &oracle.unwrap());
                    assert_eq!(result(&out), matrix([model.unwrap(), 99]));
                    rows.push((requests, bytes, report.visits));
                    println!(
                        "{}",
                        serde_json::json!({"witness":"ordinary-scalar-path","form":form,"arithmetic_first":arithmetic_first,"word":word,"flag":flag,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
                    );
                } else {
                    let error = format!("{:?}", run.unwrap_err());
                    let oracle = oracle.unwrap_err();
                    let integrity = !permit && !(arithmetic_first && word == u64::MAX);
                    assert_eq!(error.contains("Integrity"), integrity, "{error}");
                    assert_eq!(oracle.contains("LexicalIntegrity"), integrity, "{oracle}");
                    if !integrity {
                        assert!(error.contains("Arithmetic"), "{error}");
                    }
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                }
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                assert!(
                    args.iter()
                        .any(|a| a.canonical_data_draft().unwrap() == matrix([word, 99]))
                );
            }
            selected(&artifact);
            grouped_diamond(&jit);
            super::record_buffer_qualification::assert_build_owned_image_round_trip(&jit);
            drop(out);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn pinned_huffman_reversal_uses_native_scalar_operations_inside_append_loop() {
    let source = format!(
        "{}\nhuff-build-reversal()\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&[], &mut out, mode.execution(), &memory)
            .unwrap();
        let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
            jit.evaluate_into(&[], &mut out, mode.execution(), &memory)
        });
        let report = run.unwrap();
        assert_eq!(
            result(&out),
            Data::Matrix((0u8..=255).map(|v| Data::U8(v.reverse_bits())).collect())
        );
        println!(
            "{}",
            serde_json::json!({"witness":"pinned-huffman-reversal-paths","form":form,"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work})
        );
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        // The complete recurrence now subsumes its former ordinary scalar
        // groups. Keep the exact table above and require native scalar work
        // plus capacity-guarded byte stores with no backedge helper calls.
        assert!(
            metadata["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .iter()
                .any(|region| region["scalar_operations"].as_u64().unwrap() > 0
                    && region["successful_backedge_helpers"] == 0
                    && region["capacity_guards"].as_array().unwrap().len() == 1
                    && region["append_sites"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|site| site[1] == 1)),
            "{metadata}"
        );
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn scalar_paths_repeated_pure_calls_and_finite_rejections_use_fresh_arguments() {
    let body = SOURCE.split("data :=").next().unwrap();
    let source = format!(
        "{body}\n(first,second) := (calculate(seed<u64>,flag<bool>,permit<bool>,divisor<u64>),calculate(seed + 3u64,flag == false,permit,divisor))\n(first + 0u64,second + 0u64)\n"
    );
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (seed, flag) in [(3, true), (17, false), (7, true)] {
            let args = inputs(
                &artifact,
                &[
                    ("seed", Data::U64(seed)),
                    ("flag", Data::Bool(flag)),
                    ("permit", Data::Bool(true)),
                    ("divisor", Data::U64(2)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            for low in [0, 1, 64] {
                let mut limits = mode.execution();
                limits.execution.max_work = low;
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                assert!(jit.evaluate_into(&args, &mut out, limits, &memory).is_err());
                assert!(out.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                equal(
                    &artifact,
                    &out,
                    &reference_result(&artifact, &args).unwrap(),
                );
            }
        }
        selected(&artifact);
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn scalar_paths_keep_signed_float_and_boolean_lanes_exact() {
    use mech_core::snapshot::F64Bits;
    let source = "calculate(signed<i64>, real<f64>, flag<bool>) => (out<(i64,f64,bool)>) :=\n  advanced := signed + 1i64\n  chosen := (flag? | true => advanced | false => advanced - 2i64)\n  number := real * 1.0\n  out := (chosen,number,flag == false).\n(a,b,c) := calculate(signed<i64>,real<f64>,flag<bool>)\n(a,b,c)\n";
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (flag, signed, bits) in [
            (true, i64::MIN, 0x8000000000000000),
            (false, i64::MAX - 2, 0x7ff8000000000042),
            (true, -100, 0x7ff0000000000000),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("signed", Data::I64(signed)),
                    ("real", Data::F64(F64Bits::from_bits(bits))),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![
                        Data::I64(signed + 1 - if flag { 0 } else { 2 }),
                        Data::F64(F64Bits::from_bits(bits)),
                        Data::Bool(!flag)
                    ]
                    .into()
                )
            );
        }
        selected(&artifact);
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
