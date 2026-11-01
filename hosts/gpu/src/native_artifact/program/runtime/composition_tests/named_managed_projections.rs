//! Constant named fields retain current owners without ordinary call crossings.
use super::*;
use mech_engine::native_artifact::{
    NativeBufferFrameOperation as F, NativeCollectionPrimitive as P, NativeIterationAction as A,
};
use std::collections::BTreeSet;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-parent-named-managed.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    length: usize,
    steps: u64,
    allowed: bool,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("bytes", Data::Matrix(vec![Data::U8(97); length].into())),
            ("steps", Data::U64(steps)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn selected_named_fields(jit: &JitProgramArtifact) -> BTreeSet<u64> {
    let metadata = jit.descriptor().metadata();
    let fields = metadata
        .plan
        .blocks()
        .iter()
        .enumerate()
        .filter_map(|(site, block)| {
            let A::LibraryCall { template, .. } = block.action() else {
                return None;
            };
            let F::Project { output, .. } = metadata.buffer_frames.action(site)?.operation() else {
                return None;
            };
            (template.primitive() == P::RecordFieldAccess
                && metadata.plan.values()[output.get() as usize]
                    .scalar_kind()
                    .is_none())
            .then_some(site as u64)
        })
        .collect::<BTreeSet<_>>();
    let selected = metadata
        .compositions
        .parents
        .iter()
        .flat_map(|parent| &parent.segments)
        .filter(|piece| metadata.compositions.entry(piece.entry as usize).is_some())
        .flat_map(|piece| piece.sites.iter().map(|&site| u64::from(site)))
        .collect::<BTreeSet<_>>();
    let fields = fields
        .intersection(&selected)
        .copied()
        .collect::<BTreeSet<_>>();
    assert!(
        !fields.is_empty(),
        "named managed fields still split native pieces"
    );
    fields
}

fn assert_unchanged(values: &[Option<Value>], prior: &[Option<Value>]) {
    for (value, before) in values.iter().zip(prior) {
        assert!(
            value
                .as_ref()
                .unwrap()
                .shares_frozen_storage(before.as_ref().unwrap())
        );
        assert_eq!(
            value.as_ref().unwrap().canonical_data_draft().unwrap(),
            before.as_ref().unwrap().canonical_data_draft().unwrap()
        );
    }
}

#[test]
fn named_managed_fields_stay_native_with_fresh_arguments_and_recovery() {
    let mut expected_work = [None; 5];
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let fields = selected_named_fields(&jit);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (case, (length, steps)) in [(3, 2), (0, 3), (5, 0), (7, 4), (2, 3)]
                .into_iter()
                .enumerate()
            {
                let args = arguments(&artifact, &memory, length, steps, true, 1);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if length == 0 && steps > 0 {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Index"), "{error}");
                    assert_unchanged(&output, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    let report = result.unwrap();
                    assert_eq!(report.work, *expected_work[case].get_or_insert(report.work));
                    let mut expected = vec![Data::U8(97); length];
                    if length > 0 {
                        expected[0] = Data::U8(97 + length as u8 * steps as u8);
                    }
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Matrix(expected.into())
                    );
                }
                assert!(counts.selected > 0, "{counts:?}");
                assert!(
                    counts
                        .helpers
                        .keys()
                        .all(|(site, _)| !fields.contains(site)),
                    "{counts:?}"
                );
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
fn named_managed_fields_preserve_finite_boundaries_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 3, 2, true, 1)
    });
}

fn controlled_source(assert_first: bool) -> String {
    let division = "division := ((left > 0u64)? | true => 1u64 / divisor | false => 0u64)";
    let checks = if assert_first {
        format!("permitted! := allowed\n  {division}")
    } else {
        format!("{division}\n  permitted! := allowed")
    };
    SOURCE
        .replace(
            "projection-parent(data<[u8]>, left<u64>)",
            "projection-parent(data<[u8]>, left<u64>, allowed<bool>, divisor<u64>)",
        )
        .replace("  state :=", &format!("  {checks}\n  state :="))
        .replace(
            "sequence/length(window)",
            "sequence/length(window) + division - division",
        )
        .replace(
            "projection-parent(child, left - 1u64)",
            "projection-parent(child, left - 1u64, allowed, divisor)",
        )
        .replace(
            "projection-parent(bytes<[u8]>, steps<u64>)",
            "projection-parent(bytes<[u8]>, steps<u64>, allowed<bool>, divisor<u64>)",
        )
}

#[test]
fn named_managed_fields_keep_lexical_errors_unselected_paths_and_publication() {
    for assert_first in [false, true] {
        for artifact in artifacts(&controlled_source(assert_first)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            selected_named_fields(&jit);
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (length, steps, allowed, divisor, error) in [
                    (3, 2, true, 1, None),
                    (
                        0,
                        2,
                        false,
                        0,
                        Some(if assert_first {
                            "Integrity"
                        } else {
                            "Arithmetic"
                        }),
                    ),
                    (0, 2, true, 1, Some("Index")),
                    (3, 2, false, 1, Some("Integrity")),
                    (3, 2, true, 0, Some("Arithmetic")),
                    (0, 0, true, 0, None),
                    (2, 3, true, 1, None),
                ] {
                    let args = arguments(&artifact, &memory, length, steps, allowed, divisor);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                    };
                    if let Some(error) = error {
                        let receipt = format!("{:?}", result.unwrap_err());
                        assert!(receipt.contains(error), "{receipt}");
                        assert_unchanged(&output, &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        let mut expected = vec![Data::U8(97); length];
                        if length > 0 {
                            expected[0] = Data::U8(97 + length as u8 * steps as u8);
                        }
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(expected.into())
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
}

#[test]
fn named_managed_fields_decline_missing_and_tight_declared_output_limits() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let fields = selected_named_fields(&jit);
        let args = arguments(&artifact, &memory, 3, 2, true, 1);
        let before = contents(&args);
        let aliases = args.clone();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
            .unwrap();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for (dimension, allowance) in [
            ("elements", Some(0)),
            ("bytes", Some(0)),
            ("bytes", Some(1024)),
            ("elements", None),
            ("bytes", None),
            ("temporary", None),
            ("clone", None),
            ("nodes", None),
            ("compute", None),
            ("comparison", None),
        ] {
            let mut policy = mode.execution();
            match dimension {
                "elements" => policy.library.max_output_elements = allowance,
                "bytes" => policy.library.max_output_bytes = allowance,
                "temporary" => policy.library.max_temporary_bytes = allowance,
                "clone" => policy.library.max_cloned_bytes = allowance,
                "nodes" => policy.library.max_retained_nodes = allowance,
                "compute" => policy.library.max_compute_work = allowance,
                "comparison" => policy.library.max_comparison_work = allowance,
                _ => unreachable!(),
            }
            let prior = output.clone();
            let (result, counts) = observe::run(&jit, &args, &mut output, policy, &memory);
            if dimension == "bytes" && allowance == Some(1024) {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix([103, 97, 97].map(Data::U8).into())
                );
            } else {
                assert!(result.is_err(), "{dimension} {allowance:?}");
                assert_unchanged(&output, &prior);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            if dimension == "bytes" && allowance == Some(1024) {
                assert!(
                    counts.helpers.keys().any(|(site, _)| fields.contains(site)),
                    "tight byte cap did not exercise ordinary fallback: {counts:?}"
                );
            }
            jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
                .unwrap();
        }
        drop((args, aliases, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
