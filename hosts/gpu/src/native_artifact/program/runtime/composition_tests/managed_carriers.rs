//! Immutable record-matrix transport is distinct from numeric views.
use super::*;
use crate::native_artifact::program::prepare::composition::frames::Source;
use mech_core::{SchemaBody, snapshot::NamedValueDraft};

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-managed-carriers.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    seed: u32,
    steps: u64,
    allowed: bool,
    divisor: u64,
    empty: bool,
) -> Vec<Value> {
    let row = Data::Record(
        [
            ("litrunlen-and-length", Data::U32(seed)),
            ("offset", Data::U16(2)),
            ("offset-slot", Data::U16(3)),
        ]
        .map(|(name, value)| NamedValueDraft {
            name: name.into(),
            value,
        })
        .into(),
    );
    inputs(
        artifact,
        &[
            (
                "seqs",
                Data::Matrix(vec![row; seed as usize % 3 + 1].into()),
            ),
            (
                "words",
                Data::Matrix(if empty { vec![] } else { vec![Data::U64(7)] }.into()),
            ),
            ("steps", Data::U64(steps)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

#[test]
fn managed_carriers_forward_owners_without_numeric_views() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let selected = metadata.compositions.parents.iter().flat_map(|p| &p.segments)
            .filter(|piece| metadata.compositions.entry(piece.entry as usize).is_some())
            .filter(|piece| piece.frame_plan.as_ref().unwrap().sources.iter().any(|source| {
                let Source::Field { value, ordinal } = source else { return false };
                let fields = metadata.buffer_frames.fields_at(*value as usize).unwrap();
                let opaque = matches!(metadata.schemas.get(fields[*ordinal].schema()).unwrap().body(),
                    SchemaBody::Matrix { element, .. } if matches!(element.as_ref(), SchemaBody::Record(_)));
                if opaque {
                    let index = piece.frame_plan.as_ref().unwrap().sources.iter()
                        .position(|candidate| candidate == source).unwrap();
                    assert!(piece.numeric.as_ref().unwrap().views.iter()
                        .all(|view| view.source != index), "owner forwarding granted a numeric view");
                }
                opaque
            }))
            .map(|piece| u64::from(piece.entry)).collect::<std::collections::BTreeSet<_>>();
        assert!(
            !selected.is_empty(),
            "record-matrix forwarding still excludes native parent pieces"
        );
        for restored in [false, true] {
            let mut published = vec![None; artifact.outputs().len()];
            for (seed, steps, allowed, divisor, empty, rejection) in [
                (11, 2, true, 1, false, None),
                (23, 3, true, 2, false, None),
                (37, 2, false, 0, false, Some("Integrity")),
                (41, 2, true, 0, false, Some("Arithmetic")),
                (43, 1, true, 1, true, Some("Index")),
                (47, 0, false, 0, true, None),
                (53, 3, true, 1, false, None),
            ] {
                let args = arguments(&artifact, &memory, seed, steps, allowed, divisor, empty);
                let before = contents(&args);
                let aliases = args.clone();
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (actual, old) in published.iter().zip(&prior) {
                        assert!(
                            actual
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
                    assert!(
                        counts
                            .selected_sites
                            .iter()
                            .any(|(site, count)| selected.contains(site) && *count > 0)
                    );
                    let mut word = 7u64;
                    for _ in 0..steps {
                        for left in (0..=u64::from(seed % 3 + 1)).rev() {
                            word = (word + left) / divisor;
                        }
                    }
                    let expected =
                        Data::Matrix(if empty { vec![] } else { vec![Data::U64(word)] }.into());
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn managed_carriers_preserve_finite_bounds_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 7, 2, true, 1, false)
    });
}

#[test]
fn managed_carriers_keep_current_output_limits_at_ordinary_boundaries() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let args = arguments(&artifact, &memory, 11, 2, true, 1, false);
        let mut published = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
            .unwrap();
        let expected = published
            .last()
            .unwrap()
            .as_ref()
            .unwrap()
            .canonical_data_draft()
            .unwrap();
        for restored in [false, true] {
            for byte_limit in [true, false] {
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut policy = mode.execution();
                if byte_limit {
                    policy.library.max_output_bytes = Some(0);
                } else {
                    policy.library.max_output_elements = Some(0);
                }
                let (result, _) = if restored {
                    observe::run_restored(&jit, &args, &mut published, policy, &memory)
                } else {
                    observe::run(&jit, &args, &mut published, policy, &memory)
                };
                assert!(result.is_err());
                assert!(published.iter().zip(&prior).all(|(actual, prior)| {
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(prior.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
                    .unwrap();
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    expected
                );
            }
        }
        drop((args, published));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
