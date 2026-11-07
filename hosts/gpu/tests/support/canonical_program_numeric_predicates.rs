//! Correlation of repeated current-visit Boolean branches in numeric leaves.
use super::{allocation_probe, artifacts, inputs};
use mech_core::ValueDataDraft as Data;
use mech_gpu::native_artifact::{JitProgramArtifact, NativeProgramLimitMode, emit_program_object};

const PREFIX: &str = "put(xs<[u64]>, at<u64>, value<u64>) => (out<[u64]>) :=\n  ~next := xs\n  next[at] = value\n  out := next.\nstep(xs<[u64]>, used<u64>, i<u64>) => (out<([u64],u64)>) :=\n  old := xs[i + 1u64]\n  next := put(xs, i + 1u64, used)\n  out := (next, used + old).\nscan(xs<[u64]>, n<u64>, used<u64>, i<u64>) => (out<([u64],u64)>) :=\n  more := i < n\n  next := (more? | true => step(xs, used, i) | false => (xs, used))\n  (ys, total) := next\n  out := (more? | true => scan(ys, n, total, i + 1u64) | false => (xs, used)).\n(result, total) := scan(data<[u64]>, steps<u64>, seed<u64>, 1u64)\n(result, total + 0u64)\n";

fn sources() -> [(String, String); 2] {
    [
        ("generic-prefix".into(), PREFIX.into()),
        (
            "pinned-cumulative".into(),
            format!(
                "{}\n(result, total) := huff-cumulative(data<[u64]>, steps<u64>, seed<u64>, 1u64)\n(result, total + 0u64)\n",
                include_str!("../fixtures/native/deflate-huffman-level12.mec")
            ),
        ),
    ]
}

#[test]
fn predicate_versions_observe_generic_and_pinned_cumulative() {
    for (name, source) in sources() {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let memory = mode.memory_budget();
            let mut out = vec![None; artifact.outputs().len()];
            for (steps, length) in [
                (0, 288),
                (1, 288),
                (2, 288),
                (8, 288),
                (19, 19),
                (32, 32),
                (64, 64),
                (288, 288),
            ] {
                let original = (0..length).map(|i| i % 7 + 1).collect::<Vec<_>>();
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            Data::Matrix(original.iter().copied().map(Data::U64).collect()),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(5)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = args
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>();
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                let started = std::time::Instant::now();
                let (run, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                });
                let report = run.unwrap();
                let mut expected = original;
                let mut used = 5;
                for i in 1..steps {
                    let old = expected[i as usize];
                    expected[i as usize] = used;
                    used += old;
                }
                assert_eq!(
                    out.last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Matrix(expected.into_iter().map(Data::U64).collect()),
                            Data::U64(used)
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
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                println!(
                    "{}",
                    serde_json::json!({"witness":"current-predicate-versions","case":name,"form":form,"end":steps,"length":length,"seconds":started.elapsed().as_secs_f64(),"requests":requests,"requested_bytes":bytes,"visits":report.visits,"work":report.work,"allocator":"System/probe; excluded from primary timing"})
                );
            }
            drop(out);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn predicate_versions_select_generic_and_unchanged_cumulative_leaf() {
    let mut missing = Vec::new();
    for (name, source) in sources() {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata();
            let regions = metadata["compiled_numeric_write_loops"].as_array().unwrap();
            println!(
                "{}",
                serde_json::json!({"witness":"predicate-selection", "case":name, "form":form,
                "regions":regions, "candidates":metadata["readonly_numeric_candidates"]})
            );
            if regions.is_empty() {
                missing.push(format!("{name}/{form}"));
            }
            for region in regions {
                assert_eq!(region["successful_backedge_helpers"], 0);
                assert!(!region["typed_stores"].as_array().unwrap().is_empty());
            }
        }
    }
    assert!(
        missing.is_empty(),
        "correlated current-version proof missing: {missing:?}"
    );
}

fn selected(source: &str, expected: bool) {
    for artifact in artifacts(source).0 {
        let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap()
            .metadata();
        let regions = metadata["compiled_numeric_write_loops"].as_array().unwrap();
        assert_eq!(!regions.is_empty(), expected, "{source}\n{metadata}");
        for region in regions {
            assert_eq!(region["successful_backedge_helpers"], 0);
        }
    }
}

fn matrix(values: impl IntoIterator<Item = Data>) -> Data {
    Data::Matrix(values.into_iter().collect())
}

fn result(out: &[Option<mech_core::Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}

#[test]
fn predicate_versions_identity_copies_and_false_first_arms() {
    let copied = PREFIX
        .replace(
            "  more := i < n\n",
            "  more := i < n\n  copied := more\n  again := copied\n",
        )
        .replace("next := (more?", "next := (again?")
        .replace("out := (more?", "out := (copied?");
    let reversed = PREFIX
        .replace(
            "(more? | true => step(xs, used, i) | false => (xs, used))",
            "(more? | false => (xs, used) | true => step(xs, used, i))",
        )
        .replace(
            "(more? | true => scan(ys, n, total, i + 1u64) | false => (xs, used))",
            "(more? | false => (xs, used) | true => scan(ys, n, total, i + 1u64))",
        );
    for source in [copied, reversed] {
        selected(&source, true);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            for steps in [1, 3, 1, 2, 3] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "data",
                            matrix(vec![Data::U64(2), Data::U64(3), Data::U64(5)]),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(7)),
                    ],
                    &memory,
                );
                jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                    .unwrap();
                super::equal(
                    &artifact,
                    &out,
                    &super::reference_result(&artifact, &args).unwrap(),
                );
                if steps == 3 {
                    assert_eq!(
                        result(&out),
                        Data::Tuple(
                            vec![
                                matrix(vec![Data::U64(2), Data::U64(7), Data::U64(10)]),
                                Data::U64(15)
                            ]
                            .into()
                        )
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
fn predicate_versions_boolean_carrier_changes_on_every_recursive_visit() {
    let put = PREFIX.split("step(xs").next().unwrap();
    let source = format!(
        "{put}\nwalk(xs<[u64]>, left<u64>, flag<bool>) => (out<[u64]>) :=\n  more := flag == true\n  next := (more? | true => put(xs, 1u64, xs[1u64] + 1u64) | false => xs)\n  out := ((left > 0u64)? | true => (more? | true => walk(next, left - 1u64, false) | false => walk(xs, left - 1u64, true)) | false => (more? | true => next | false => xs)).\nwalk(data<[u64]>, steps<u64>, flag<bool>)\n"
    );
    selected(&source, true);
    for artifact in artifacts(&source).0 {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut out = vec![None; artifact.outputs().len()];
        for (flag, steps, word) in [
            (false, 0, 7),
            (true, 0, 11),
            (false, 1, 13),
            (true, 2, 17),
            (false, 7, 19),
            (true, 32, 23),
            (false, 0, 29),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("data", matrix(vec![Data::U64(word)])),
                    ("steps", Data::U64(steps)),
                    ("flag", Data::Bool(flag)),
                ],
                &memory,
            );
            let aliases = args.clone();
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &out,
                &super::reference_result(&artifact, &args).unwrap(),
            );
            let writes = (0..=steps).filter(|i| (*i % 2 == 0) == flag).count() as u64;
            assert_eq!(result(&out), matrix(vec![Data::U64(word + writes)]));
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            assert!(
                args.iter()
                    .any(|a| a.canonical_data_draft().unwrap() == matrix(vec![Data::U64(word)]))
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
fn predicate_versions_stale_reads_and_discarded_updates_stay_ordinary() {
    let discarded = PREFIX.replace(
        "scan(ys, n, total, i + 1u64)",
        "scan(xs, n, total, i + 1u64)",
    );
    let read = PREFIX
        .replace(
            "  (ys, total) := next\n",
            "  (ys, total) := next\n  old := xs[1u64]\n",
        )
        .replace(
            "scan(ys, n, total, i + 1u64)",
            "scan(ys, n, total + old, i + 1u64)",
        );
    for source in [discarded, read] {
        selected(&source, false);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    (
                        "data",
                        matrix(vec![Data::U64(2), Data::U64(3), Data::U64(5)]),
                    ),
                    ("steps", Data::U64(3)),
                    ("seed", Data::U64(7)),
                ],
                &memory,
            );
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &out,
                &super::reference_result(&artifact, &args).unwrap(),
            );
            drop((out, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn predicate_versions_distinct_predicates_and_phi_alternatives_stay_ordinary() {
    for predicate in ["flag", "chosen"] {
        let source = PREFIX
            .replace(
                "used<u64>, i<u64>) => (out<([u64],u64)>) :=\n  more",
                "used<u64>, i<u64>, flag<bool>, other<bool>) => (out<([u64],u64)>) :=\n  more",
            )
            .replace(
                "  (ys, total) := next\n",
                "  (ys, total) := next\n  chosen := (flag? | true => more | false => other)\n",
            )
            .replace(
                "out := (more? | true => scan(ys, n, total, i + 1u64)",
                &format!(
                    "out := ({predicate}? | true => scan(ys, n, total, i + 1u64, flag, other)"
                ),
            )
            .replace(
                "seed<u64>, 1u64)",
                "seed<u64>, 1u64, flag<bool>, other<bool>)",
            );
        selected(&source, false);
        for artifact in artifacts(&source).0 {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    (
                        "data",
                        matrix(vec![Data::U64(2), Data::U64(3), Data::U64(5)]),
                    ),
                    ("steps", Data::U64(3)),
                    ("seed", Data::U64(7)),
                    ("flag", Data::Bool(false)),
                    ("other", Data::Bool(false)),
                ],
                &memory,
            );
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut out = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut out, mode.execution(), &memory)
                .unwrap();
            super::equal(
                &artifact,
                &out,
                &super::reference_result(&artifact, &args).unwrap(),
            );
            assert_eq!(
                result(&out),
                Data::Tuple(
                    vec![
                        matrix(vec![Data::U64(2), Data::U64(3), Data::U64(5)]),
                        Data::U64(7)
                    ]
                    .into()
                )
            );
            drop((out, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[path = "canonical_program_numeric_predicate_recovery.rs"]
mod recovery;
