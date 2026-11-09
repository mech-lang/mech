//! Completed source-key arithmetic keeps all original array/error boundaries.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::completed_element_ranges::tests::consumer_source;
fn source(seed: &str, checked: bool) -> String {
    consumer_source(
        seed,
        "key + 1u64",
        if checked {
            "counted(output,ids[at + 1u64])"
        } else {
            "(sequence/append(output.1,ids[at + 1u64]),output.2)"
        },
        "built.1",
    )
}
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    ids: &[u64],
    buckets: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "ids",
                Data::Matrix(ids.iter().copied().map(Data::U64).collect()),
            ),
            ("buckets", Data::U64(buckets)),
        ],
        memory,
    )
}
#[test]
fn completed_key_additions_keep_errors_partial_producers_aliases_and_retry() {
    for (text, eligible) in [
        (source("empty", true), true),
        (source("sequence/append(empty,18446744073709551615u64)", true), false),
        (source("empty", false), false),
        (consumer_source("empty", "key + 1u64", "((ids[at + 1u64] == 0u64)? | true => counted(output,ids[at + 1u64]) | false => (sequence/append(output.1,ids[at + 1u64]),output.2))", "built.1"), false),
    ] {
        for artifact in artifacts(&text) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let control = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::CompletedKeyAdditions,
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                let mut ordinary = output.clone();
                for (ids, buckets) in [
                    (&[0, 2, 1, 2][..], 3),
                    (&[][..], 0),
                    (&[0, u64::MAX][..], 3),
                    (&[3][..], 3),
                    (&[0, 1][..], 3),
                ] {
                    let input = args(&artifact, &memory, ids, buckets);
                    let aliases = input.clone();
                    let before = contents(&input);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let eval = |jit: &JitProgramArtifact, out: &mut [Option<Value>]| {
                        if restored {
                            observe::run_restored(jit, &input, out, mode.execution(), &memory).0
                        } else {
                            observe::run(jit, &input, out, mode.execution(), &memory).0
                        }
                    };
                    let a = eval(&native, &mut output);
                    let b = eval(&control, &mut ordinary);
                    match (a, b) {
                        (Ok(a), Ok(b)) => {
                            assert_eq!(a.work, b.work);
                            assert_eq!(a.visits, b.visits);
                            assert_eq!(
                                contents(
                                    &output
                                        .iter()
                                        .map(|v| v.as_ref().unwrap().clone())
                                        .collect::<Vec<_>>()
                                ),
                                contents(
                                    &ordinary
                                        .iter()
                                        .map(|v| v.as_ref().unwrap().clone())
                                        .collect::<Vec<_>>()
                                )
                            );
                            let expected = ids.iter().map(|n| n + 1).sum::<u64>();
                            assert_eq!(
                                output
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                Data::U64(expected)
                            );
                        }
                        (Err(a), Err(b)) => {
                            assert_eq!(format!("{a:?}"), format!("{b:?}"));
                            assert!(output.iter().zip(&prior).all(|(a, b)| match (a, b) {
                                (None, None) => true,
                                (Some(a), Some(b)) => a.shares_frozen_storage(b),
                                _ => false,
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                        (a, b) => panic!("default/control differ: {a:?} {b:?}"),
                    }
                    assert_eq!(contents(&input), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((output, ordinary));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let sites = |m: serde_json::Value| {
                [
                    "compiled_numeric_write_loops",
                    "compiled_readonly_numeric_loops",
                ]
                .into_iter()
                .flat_map(|k| m[k].as_array().unwrap().iter())
                .map(|r| {
                    r["proven_completed_key_additions"]
                        .as_array()
                        .unwrap()
                        .len()
                })
                .sum::<usize>()
            };
            assert_eq!(sites(object.metadata()) > 0, eligible);
            let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
                &artifact,
                &catalog,
                OptimizationPass::CompletedKeyAdditions,
            )
            .unwrap();
            assert_eq!(sites(old.metadata()), 0);
        }
    }
}
#[test]
fn completed_key_additions_keep_finite_admission_and_atomic_publication() {
    resources::finite_boundaries(&source("empty", true), |artifact, memory| {
        args(artifact, memory, &[0, 2, 1, 2], 3)
    });
}
#[test]
fn completed_key_additions_reject_earlier_object_identity() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&source("empty", true)) {
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let mut old = *object.identity().bytes();
        old[16..24].copy_from_slice(&308u64.to_le_bytes());
        assert!(
            crate::native_artifact::program::linked::prepare_descriptor(&artifact, &catalog, &old)
                .is_err()
        );
    }
}
