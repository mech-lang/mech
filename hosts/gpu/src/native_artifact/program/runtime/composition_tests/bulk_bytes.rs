//! Selected bulk byte append must remain inside its ordinary numeric recurrence.
use super::*;

const SOURCE: &str = "+> sequence\n+> bits\nemit-bytes(bytes<[u8]>, word<u64>, count<u8>, i<u64>, end<u64>) => (out<[u8]>) :=\n  more := i < end\n  next := (more? | true => sequence/append-le-bytes(bytes, bits/wrapping-add(word, i), count) | false => bytes)\n  out := (more? | true => emit-bytes(next, word, count, i + 1u64, end) | false => bytes).\nemit-bytes(sequence/with-capacity(0u8, capacity<u64>), word<u64>, count<u8>, 0u64, steps<u64>)\n";

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    word: u64,
    count: u8,
    steps: u64,
    capacity: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("word", Data::U64(word)),
            ("count", Data::U8(count)),
            ("steps", Data::U64(steps)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

fn expected(word: u64, count: u8, steps: u64) -> Data {
    Data::Matrix(
        (0..steps)
            .flat_map(|i| {
                word.wrapping_add(i).to_le_bytes()[..usize::from(count)]
                    .iter()
                    .copied()
                    .map(Data::U8)
                    .collect::<Vec<_>>()
            })
            .collect(),
    )
}

#[test]
fn bulk_byte_append_selects_native_loop_without_per_visit_boundaries() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| {
                    r.numeric
                        .as_ref()
                        .is_some_and(|p| p.arrays.iter().any(|a| a.appends))
                }),
            "bulk append must retain its recurrence in the existing numeric emitter"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for count in 0..=8 {
                let mut boundary_count = None;
                for steps in [1, 8, 64] {
                    let args = arguments(&artifact, &memory, u64::MAX - 3, count, steps, 2048);
                    let (result, observed) = if restored {
                        observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                    };
                    result.unwrap();
                    assert_eq!(
                        contents(&[output.last().unwrap().as_ref().unwrap().clone()]),
                        [expected(u64::MAX - 3, count, steps)]
                    );
                    let count = (
                        observed.admits.values().sum::<u64>(),
                        observed.helpers.values().sum::<u64>(),
                    );
                    assert_eq!(count, *boundary_count.get_or_insert(count));
                }
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn bulk_byte_append_keeps_current_counts_publication_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (word, count, steps, capacity) in [
                (0x0102_0304_0506_0708, 2, 16, 128),
                (u64::MAX, 9, 1, 128),
                (0, 255, 0, 0),
                (0x0807_0605_0403_0201, 8, 8, 64),
                (7, 255, 2, 64),
                (99, 0, 64, 0),
                (u64::MAX - 3, 1, 16, 0),
                (0x0102_0304_0506_0708, 3, 2, 5),
            ] {
                let args = arguments(&artifact, &memory, word, count, steps, capacity);
                let aliases = args.clone();
                let before = contents(&args);
                let previous = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, _) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if count > 8 && steps > 0 {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Library"), "{error}");
                    for (current, old) in output.iter().zip(&previous) {
                        assert!(
                            current
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected(word, count.min(8), steps)
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn bulk_byte_append_keeps_finite_funding_and_cleanup() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 0x0102_0304_0506_0708, 3, 8, 128)
    });
}

const MIXED: &str = r#"+> sequence
+> bits
<step-state> := <{bytes<[u8]> stamp<u64>}>
<emitted> := <{bytes<[u8]> seen<u64>}>
one(bytes<[u8]>, words<[u64]>, counts<[u8]>, i<u64>, word<u64>, allowed<bool>, deny-at<u64>, divisor<u64>) => (out<step-state>) :=
  allowed-here! := allowed
  visit! := i != deny-at
  count := counts[i + 1u64]
  __ORDER__
  out := {bytes: next, stamp: part}.
walk(bytes<[u8]>, words<[u64]>, counts<[u8]>, i<u64>, end<u64>, seen<u64>, word<u64>, allowed<bool>, deny-at<u64>, divisor<u64>) => (out<emitted>) :=
  more := i < end
  step := (more? | true => one(bytes, words, counts, i, word, allowed, deny-at, divisor) | false => {bytes: bytes, stamp: 0u64})
  out := (more? | true => walk(step.bytes, words, counts, i + 1u64, end, bits/wrapping-add(seen, step.stamp), word, allowed, deny-at, divisor) | false => {bytes: bytes, seen: seen}).
value := walk(sequence/reserve(bytes<[u8]>, capacity<u64>), words<[u64]>, counts<[u8]>, 0u64, steps<u64>, 0u64, word<u64>, allowed<bool>, deny-at<u64>, divisor<u64>)
(value.bytes, value.seen)
"#;

fn mixed_source(append_first: bool) -> String {
    MIXED.replace(
        "__ORDER__",
        if append_first {
            "next := sequence/append-le-bytes(bytes, word, count)\n  raw := words[i + 1u64]\n  part := raw / divisor"
        } else {
            "raw := words[i + 1u64]\n  part := raw / divisor\n  next := sequence/append-le-bytes(bytes, part, count)"
        },
    )
}

#[test]
fn bulk_byte_append_mixed_results_keep_fresh_visits_error_order_and_aliases() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    for append_first in [false, true] {
        for artifact in artifacts(&mixed_source(append_first)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let mut reference_errors = std::collections::BTreeMap::new();
            for (is_reference, jit) in [(true, &ordinary), (false, &native)] {
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut output = vec![None; artifact.outputs().len()];
                    for (case, words, counts, steps, allowed, deny, divisor, capacity) in [
                        (
                            "initial",
                            vec![1, 2, 3, 4],
                            vec![0, 1, 8, 2],
                            4,
                            true,
                            u64::MAX,
                            1,
                            128,
                        ),
                        (
                            "reject-later",
                            vec![5, 6, 7, 8],
                            vec![1, 1, 9, 2],
                            4,
                            true,
                            u64::MAX,
                            1,
                            128,
                        ),
                        (
                            "arithmetic-versus-count",
                            vec![5],
                            vec![9],
                            1,
                            true,
                            u64::MAX,
                            0,
                            128,
                        ),
                        (
                            "load-versus-count",
                            vec![],
                            vec![9],
                            1,
                            true,
                            u64::MAX,
                            1,
                            128,
                        ),
                        ("two-assertions", vec![], vec![9], 1, false, 0, 0, 128),
                        (
                            "later-assertion",
                            vec![5, 6, 7],
                            vec![1, 1, 1],
                            3,
                            true,
                            1,
                            1,
                            128,
                        ),
                        ("unselected", vec![], vec![], 0, false, 0, 0, 0),
                        (
                            "retry",
                            vec![u64::MAX, 19, 27, 33],
                            vec![8, 0, 1, 3],
                            4,
                            true,
                            u64::MAX,
                            1,
                            128,
                        ),
                        (
                            "growth",
                            vec![9, 8, 7, 6],
                            vec![1, 2, 3, 8],
                            4,
                            true,
                            u64::MAX,
                            1,
                            0,
                        ),
                        (
                            "zero-counts",
                            vec![9, 8, 7, 6],
                            vec![0, 0, 0, 0],
                            4,
                            true,
                            u64::MAX,
                            1,
                            128,
                        ),
                    ] {
                        let word = 0x0102_0304_0506_0708u64;
                        let args = inputs(
                            &artifact,
                            &[
                                ("bytes", Data::Matrix([Data::U8(77), Data::U8(88)].into())),
                                (
                                    "words",
                                    Data::Matrix(words.iter().copied().map(Data::U64).collect()),
                                ),
                                (
                                    "counts",
                                    Data::Matrix(counts.iter().copied().map(Data::U8).collect()),
                                ),
                                ("word", Data::U64(word)),
                                ("steps", Data::U64(steps)),
                                ("allowed", Data::Bool(allowed)),
                                ("deny-at", Data::U64(deny)),
                                ("divisor", Data::U64(divisor)),
                                ("capacity", Data::U64(capacity)),
                            ],
                            &memory,
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let previous = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let expected_error = match case {
                            "reject-later" => Some("Library"),
                            "arithmetic-versus-count" => Some(if append_first {
                                "Library"
                            } else {
                                "Arithmetic"
                            }),
                            "load-versus-count" => {
                                Some(if append_first { "Library" } else { "Index" })
                            }
                            "two-assertions" | "later-assertion" => Some("Integrity"),
                            _ => None,
                        };
                        let (result, observed) = if restored {
                            observe::run_restored(
                                jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(jit, &args, &mut output, mode.execution(), &memory)
                        };
                        if let Some(name) = expected_error {
                            let error = format!("{:?}", result.unwrap_err());
                            assert!(error.contains(name), "{case}: {error}");
                            if is_reference {
                                reference_errors.insert(case, error.clone());
                            } else {
                                assert_eq!(
                                    error, reference_errors[case],
                                    "{case} restored={restored}"
                                );
                            }
                            for (current, old) in output.iter().zip(&previous) {
                                assert!(
                                    current
                                        .as_ref()
                                        .unwrap()
                                        .shares_frozen_storage(old.as_ref().unwrap())
                                );
                            }
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            result.unwrap();
                            let mut bytes = vec![Data::U8(77), Data::U8(88)];
                            let mut seen = 0u64;
                            for i in 0..steps as usize {
                                let part = words[i] / divisor;
                                let incoming = if append_first { word } else { part };
                                bytes.extend(
                                    incoming.to_le_bytes()[..counts[i] as usize]
                                        .iter()
                                        .copied()
                                        .map(Data::U8),
                                );
                                seen = seen.wrapping_add(part);
                            }
                            assert_eq!(
                                output
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                Data::Tuple([Data::Matrix(bytes.into()), Data::U64(seen)].into())
                            );
                            if !is_reference && capacity == 128 && steps > 0 {
                                assert!(
                                    !observed.first_change_backings.is_empty()
                                        || case == "zero-counts"
                                );
                                if case == "zero-counts" {
                                    assert!(observed.first_change_backings.is_empty());
                                }
                            }
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}

#[test]
fn bulk_byte_append_checks_remain_observable_with_unused_results() {
    let source = mixed_source(true).replace("(value.bytes, value.seen)", "7u64");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let mut output = vec![None; artifact.outputs().len()];
        for (allowed, count, rejection) in [
            (true, 1, None),
            (false, 1, Some("Integrity")),
            (true, 9, Some("Library")),
            (true, 0, None),
        ] {
            let args = inputs(
                &artifact,
                &[
                    ("bytes", Data::Matrix([Data::U8(7)].into())),
                    ("words", Data::Matrix([Data::U64(1)].into())),
                    ("counts", Data::Matrix([Data::U8(count)].into())),
                    ("word", Data::U64(19)),
                    ("steps", Data::U64(1)),
                    ("allowed", Data::Bool(allowed)),
                    ("deny-at", Data::U64(u64::MAX)),
                    ("divisor", Data::U64(1)),
                    ("capacity", Data::U64(128)),
                ],
                &memory,
            );
            let previous = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let result = jit.evaluate_into(&args, &mut output, mode.execution(), &memory);
            if let Some(rejection) = rejection {
                assert!(format!("{:?}", result.unwrap_err()).contains(rejection));
                for (current, old) in output.iter().zip(&previous) {
                    assert!(
                        current
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    );
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            } else {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(7)
                );
            }
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
