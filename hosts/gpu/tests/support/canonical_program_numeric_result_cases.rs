use super::super::{equal, reference_result};
use super::*;

fn matrix(words: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(words.into_iter().collect())
}
fn result(out: &[Option<mech_core::Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn selected(source: &str, expected: bool) {
    let artifact = artifacts(source).0.into_iter().next().unwrap();
    let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
        .unwrap()
        .metadata();
    assert_eq!(
        metadata["numeric_write_regions"]
            .as_array()
            .unwrap()
            .iter()
            .any(|r| r["proof"]["result"]["Frame"].is_object()),
        expected,
        "{metadata}"
    );
}

#[test]
fn mixed_result_terminal_scalar_kinds_preserve_exact_bits() {
    use mech_core::snapshot::F64Bits;
    let put = GENERIC.split("walk(xs").next().unwrap();
    let source = format!(
        "{put}\nwalk(xs<[u32]>, left<u64>, flag<bool>, byte<u8>, half<u16>, packed<u32>, signed<i64>, fraction<f64>) => <([u32],bool,u8,u16,u32,u64,i64,f64)>\n  | (xs, left, flag, byte, half, packed, signed, fraction) => ((left > 0u64)? | true => walk(put(xs, 1u64, packed), left - 1u64, flag, byte, half, packed, signed, fraction) | false => (xs, flag == false, byte, half, packed, left + 17u64, signed, fraction)).\n(a,b,c,d,e,f,g,h) := walk(data<[u32]>, 2u64, flag<bool>, byte<u8>, half<u16>, packed<u32>, signed<i64>, fraction<f64>)\n(a,b,c,d,e,f + 0u64,g,h)\n"
    );
    selected(&source, true);
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (flag, signed, bits) in [
            (true, i64::MIN, 0x8000_0000_0000_0000),
            (false, i64::MAX, 0x7ff8_0000_0000_0042),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix([Data::U32(7)])),
                    ("flag", Data::Bool(flag)),
                    ("byte", Data::U8(239)),
                    ("half", Data::U16(56506)),
                    ("packed", Data::U32(0x9abc_def0)),
                    ("signed", Data::I64(signed)),
                    ("fraction", Data::F64(F64Bits::from_bits(bits))),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![
                        matrix([Data::U32(0x9abc_def0)]),
                        Data::Bool(!flag),
                        Data::U8(239),
                        Data::U16(56506),
                        Data::U32(0x9abc_def0),
                        Data::U64(17),
                        Data::I64(signed),
                        Data::F64(F64Bits::from_bits(bits))
                    ]
                    .into()
                )
            );
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn readonly_mixed_result_retains_shared_owner_and_terminal_scalar() {
    let source = "walk(xs<[u64]>, i<u64>, total<u64>) => <([u64],u64)>\n  | (xs, i, total) => ((i < 3u64)? | true => walk(xs, i + 1u64, total + xs[i + 1u64]) | false => (xs, total + 17u64)).\n(a, b) := walk(data<[u64]>, 0u64, 5u64)\n(a, b + 0u64)\n";
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let regions = metadata["compiled_readonly_numeric_loops"]
            .as_array()
            .unwrap();
        assert!(!regions.is_empty());
        assert!(
            regions
                .iter()
                .all(|r| r["successful_backedge_helpers"] == 0)
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let args = inputs(
            &artifact,
            &[("data", matrix([Data::U64(2), Data::U64(3), Data::U64(5)]))],
            &memory,
        );
        let aliases = args.clone();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(
                vec![
                    matrix([Data::U64(2), Data::U64(3), Data::U64(5)]),
                    Data::U64(32)
                ]
                .into()
            )
        );
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert!(args[0].shares_frozen_storage(&aliases[0]));
        drop((args, aliases, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_results_exact_rectangular_shapes_and_fresh_views() {
    use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
    for artifact in artifacts(GENERIC).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (rows, cols) in [(2, 3), (3, 2), (2, 3)] {
            let mut args = inputs(
                &artifact,
                &[
                    ("data", matrix((1..=6).map(Data::U32))),
                    ("steps", Data::U64(1)),
                    ("seed", Data::U64(0)),
                ],
                &memory,
            );
            let at = artifact
                .inputs()
                .iter()
                .position(|p| mech_engine::decode_source_input_name(&p.name).unwrap() == "data")
                .unwrap();
            let schema = artifact.inputs()[at].schema;
            let shape = mech_core::shape_for_resolved_extents(
                artifact.schemas().get(schema).unwrap(),
                &[rows, cols],
            )
            .unwrap();
            let value = ValueDraft {
                schema,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data: args[at].canonical_data_draft().unwrap(),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut admission = memory
                .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                .unwrap();
            args[at] = value.into_memory_budget(&mut admission).unwrap();
            drop(admission);
            let before = args[at].canonical_data_draft().unwrap();
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            let mut expected = (1..=6).collect::<Vec<u32>>();
            expected[0] += 1;
            expected[cols as usize] += 1;
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![matrix(expected.into_iter().map(Data::U32)), Data::U64(19)].into()
                )
            );
            assert_eq!(args[at].canonical_data_draft().unwrap(), before);
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_results_actual_aliases_keep_readonly_array_versions_immutable() {
    let put = GENERIC.split("walk(xs").next().unwrap();
    let source = format!(
        "{put}\nread(a<[u32]>) => (out<u64>) :=\n  out<u64> := a[1u64].\nwalk(a<[u32]>, xs<[u32]>, i<u64>, total<u64>) => <([u32],u64)>\n  | (a, xs, i, total) => ((i < 4u64)? | true => walk(a, put(xs, 1u64, xs[1u64] + 1u32), i + 1u64, total + read(a)) | false => (xs, total + 17u64)).\n(result, sum) := walk(first<[u32]>, second<[u32]>, 0u64, 0u64)\n(result, sum + 0u64)\n"
    );
    selected(&source, true);
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut args = inputs(
            &artifact,
            &[
                ("first", matrix([Data::U32(5)])),
                ("second", matrix([Data::U32(5)])),
            ],
            &memory,
        );
        assert_eq!(args[0].schema(), args[1].schema());
        args[1] = args[0].clone();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        equal(
            &artifact,
            &out,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(
            result(&out),
            Data::Tuple(vec![matrix([Data::U32(9)]), Data::U64(37)].into())
        );
        assert!(args[0].shares_frozen_storage(&args[1]));
        assert_eq!(
            args[0].canonical_data_draft().unwrap(),
            matrix([Data::U32(5)])
        );
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_assign_results_preserve_short_reads_rejection_and_recovery() {
    let source = sources().into_iter().nth(1).unwrap().1;
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for len in [1, 0, 1] {
            let mut counts = vec![Data::U64(0); 17];
            counts[1] = Data::U64(1);
            counts[2] = Data::U64(1);
            let args = inputs(
                &artifact,
                &[
                    ("symbols", matrix([])),
                    ("lens", matrix(vec![Data::U8(7); len])),
                    ("counts", matrix(counts)),
                ],
                &memory,
            );
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let native = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
            if len == 1 {
                native.unwrap();
                equal(
                    &artifact,
                    &out,
                    &reference_result(&artifact, &args).unwrap(),
                );
                assert_eq!(result(&out), matrix([Data::U8(1)]));
            } else {
                assert!(native.is_err());
                assert!(reference_result(&artifact, &args).is_err());
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                assert!(out.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_results_refresh_same_shape_owners_and_current_parent_arguments() {
    selected(GENERIC, true);
    for artifact in artifacts(GENERIC).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = mode.memory_budget();
        let mut out = vec![None; artifact.outputs().len()];
        for (base, seed) in [(7, 0), (100, 1), (700, 0)] {
            let words = (base..base + 6).collect::<Vec<u32>>();
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix(words.iter().copied().map(Data::U32))),
                    ("steps", Data::U64(2)),
                    ("seed", Data::U64(seed)),
                ],
                &memory,
            );
            let before = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let mut expected = words;
            for start in [seed, seed + 1] {
                for i in start..start + 2 {
                    expected[i as usize] += 1;
                }
            }
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
                        matrix(expected.into_iter().map(Data::U32)),
                        Data::U64(seed + 20)
                    ]
                    .into()
                )
            );
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                before
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_results_zero_visits_and_unchanged_stores_keep_input_identity() {
    let body = GENERIC.split("outer(xs").next().unwrap();
    let source = format!("{body}\n(next, at) := walk(data<[u32]>, 0u64, steps<u64>)\nnext\n");
    for source in [
        source.clone(),
        source.replace("xs[i + 1u64] + 1u32", "xs[i + 1u64]"),
    ] {
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for steps in [0, 1, 3] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix(vec![Data::U32(7); 3])),
                        ("steps", Data::U64(steps)),
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
                if steps == 0 || !source.contains("+ 1u32") {
                    let input = args
                        .iter()
                        .find(|v| matches!(v.data(), mech_core::ValueData::Matrix(_)))
                        .unwrap();
                    assert!(
                        out.last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(input)
                    );
                }
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
fn mixed_results_stale_multiple_and_escaping_returns_keep_the_maintained_path() {
    let put = GENERIC.split("walk(xs").next().unwrap();
    let stale = format!(
        "{put}\nwalk(xs<[u32]>, left<u64>) => (out<([u32],u64)>) :=\n  next := put(xs, 1u64, xs[1u64] + 1u32)\n  out := ((left > 0u64)? | true => walk(next, left - 1u64) | false => (xs, 17u64)).\n(a, b) := walk(data<[u32]>, 2u64)\n(a, b + 0u64)\n"
    );
    let multiple = format!(
        "{put}\nwalk(xs<[u32]>, left<u64>) => (out<([u32],[u32],u64)>) :=\n  next := put(xs, 1u64, xs[1u64] + 1u32)\n  out := ((left > 0u64)? | true => walk(next, left - 1u64) | false => (next, next, 17u64)).\n(a, b, c) := walk(data<[u32]>, 2u64)\n(a, b, c + 0u64)\n"
    );
    let body = GENERIC.split("outer(xs").next().unwrap();
    let escaping = format!("{body}\nwalk(data<[u32]>, 0u64, 2u64)\n");
    for source in [stale, multiple, escaping] {
        selected(&source, false);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = inputs(
                &artifact,
                &[("data", matrix(vec![Data::U32(7); 3]))],
                &memory,
            );
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            equal(
                &artifact,
                &out,
                &reference_result(&artifact, &args).unwrap(),
            );
            drop((args, out));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn mixed_results_ordered_late_failures_preserve_publication_aliases_and_retry() {
    for order in [0, 1, 2] {
        let load = "  word := xs[at]\n";
        let check = "  enabled! := allowed\n";
        let arithmetic = "  divided := 12u64 / divisor\n";
        let checks = match order {
            0 => format!("{check}{load}{arithmetic}"),
            1 => format!("{load}{check}{arithmetic}"),
            _ => format!("{arithmetic}{check}{load}"),
        };
        let source = format!(
            "walk(xs<[u32]>, left<u64>, at<u64>, next-at<u64>, allowed<bool>, next-allowed<bool>, divisor<u64>, next-divisor<u64>) => (out<([u32],u64)>) :=\n{checks}  ~next := xs\n  next[at] = word + 1u32\n  out := ((left > 0u64)? | true => walk(next, left - 1u64, next-at, next-at, next-allowed, next-allowed, next-divisor, next-divisor) | false => (next, divided + 17u64)).\n(result, total) := walk(data<[u32]>, 1u64, 1u64, next-at<u64>, true, allowed<bool>, 3u64, divisor<u64>)\n(result, total + 0u64)\n"
        );
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for (allowed, divisor, at, word) in [
                (true, 3, 1, 7),
                (false, 0, 2, 7),
                (true, 2, 1, 9),
                (true, 0, 1, 7),
                (false, 3, 2, 7),
                (true, 3, 2, 7),
                (true, 3, 1, u32::MAX - 1),
                (true, 3, 1, 5),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        ("data", matrix([Data::U32(word)])),
                        ("allowed", Data::Bool(allowed)),
                        ("divisor", Data::U64(divisor)),
                        ("next-at", Data::U64(at)),
                    ],
                    &memory,
                );
                let before_args = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                let prior = out.clone();
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let native = jit.evaluate_into(&args, &mut out, mode.execution(), &memory);
                let resident = reference_result(&artifact, &args);
                if allowed && divisor != 0 && at == 1 && word < u32::MAX - 1 {
                    native.unwrap();
                    equal(&artifact, &out, &resident.unwrap());
                    assert_eq!(
                        result(&out),
                        Data::Tuple(
                            vec![matrix([Data::U32(word + 2)]), Data::U64(12 / divisor + 17)]
                                .into()
                        )
                    );
                } else {
                    let error = format!("{:?}", native.unwrap_err());
                    let oracle = resident.unwrap_err();
                    let assertion =
                        !allowed && !(order == 1 && at == 2) && !(order == 2 && divisor == 0);
                    assert_eq!(error.contains("Integrity"), assertion, "{error}");
                    assert_eq!(oracle.contains("LexicalIntegrity"), assertion, "{oracle}");
                    if allowed && at == 1 {
                        assert!(error.contains("Arithmetic"), "{error}");
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                    assert!(out.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                assert_eq!(
                    args.iter()
                        .map(|v| v.canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    before_args
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
fn mixed_results_exact_and_one_short_work_preserve_publication() {
    for artifact in artifacts(GENERIC).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for steps in [0, 1, 8, 64] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix((0..288).map(Data::U32))),
                    ("steps", Data::U64(steps)),
                    ("seed", Data::U64(3)),
                ],
                &memory,
            );
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let full = jit
                .evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            let mut exact = mode.execution();
            exact.execution.max_work = full.work;
            jit.evaluate_into(&args, &mut out, exact, &memory).unwrap();
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            exact.execution.max_work -= 1;
            assert!(jit.evaluate_into(&args, &mut out, exact, &memory).is_err());
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                before
            );
            assert!(out.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            // The independent whole-array model is also used by the 256-step
            // diagnostic. The maintained resident oracle's comparison budget
            // is 65536; two 64-step parent visits require 65888 comparisons.
            // Keep that allowance unchanged and verify every word directly.
            let mut expected = (0..288).collect::<Vec<u32>>();
            for start in [3, 4] {
                for i in start..start + steps as usize {
                    expected[i] += 1;
                }
            }
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![
                        matrix(expected.into_iter().map(Data::U32)),
                        Data::U64(steps + 21)
                    ]
                    .into()
                )
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn secondary_cumulative_retains_the_predicate_correlation_rejection() {
    let source = format!(
        "{}\n(next, used) := huff-cumulative(data<[u64]>, 3u64, 7u64, 1u64)\n(next, used + 0u64)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec")
    );
    for artifact in artifacts(&source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let candidates = metadata["readonly_numeric_candidates"].as_array().unwrap();
        println!(
            "{}",
            serde_json::json!({"witness":"secondary-cumulative-decline","candidates":candidates})
        );
        assert!(
            candidates.iter().any(|r| r["rejection"]
                .as_str()
                .is_some_and(|s| s.contains("older array version"))),
            "{metadata}"
        );
        assert!(
            metadata["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = inputs(
            &artifact,
            &[("data", matrix([Data::U64(2), Data::U64(3), Data::U64(5)]))],
            &memory,
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(
            result(&out),
            Data::Tuple(
                vec![
                    matrix([Data::U64(2), Data::U64(7), Data::U64(10)]),
                    Data::U64(15)
                ]
                .into()
            )
        );
        drop((args, out));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn mixed_result_frame_limits_keep_the_selected_failure_position() {
    let source = "+> sequence\nwalk(xs<[u32]>, left<u64>, allowed<bool>) => (out<([u32],u64,u64,u64,u64)>) :=\n  enabled! := allowed\n  word := xs[1u64] + 1u32\n  ~next := xs\n  next[1u64] = word\n  out := ((left > 0u64)? | true => walk(next, left - 1u64, allowed) | false => (next, 1u64, 2u64, 3u64, 4u64)).\n(a,b,c,d,e) := walk(data<[u32]>, 1u64, allowed<bool>)\nb + c + d + e + sequence/length(a)\n";
    selected(source, true);
    for artifact in artifacts(source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (allowed, bound) in [
            (true, None),
            (false, Some(3)),
            (true, Some(3)),
            (true, None),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix([Data::U32(7)])),
                    ("allowed", Data::Bool(allowed)),
                ],
                &memory,
            );
            let prior = out.clone();
            let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut limits = mode.execution();
            if let Some(bound) = bound {
                limits.library.max_output_elements = Some(bound);
            }
            let run = jit.evaluate_into(&args, &mut out, limits, &memory);
            if bound.is_none() {
                run.unwrap();
                assert_eq!(result(&out), Data::U64(11));
            } else {
                let error = format!("{:?}", run.unwrap_err());
                assert_eq!(error.contains("Integrity"), !allowed, "{error}");
                if allowed {
                    assert!(error.contains("OutputElements"), "{error}");
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    before
                );
                assert!(out.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
            }
            assert!(
                args.iter()
                    .filter(|v| matches!(v.data(), mech_core::ValueData::Matrix(_)))
                    .all(|v| v.canonical_data_draft().unwrap() == matrix([Data::U32(7)]))
            );
        }
        drop(out);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
