//! Immutable record-matrix/String transport is distinct from numeric views.
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
            ("seqs", Data::Matrix(vec![row].into())),
            ("label", Data::String(format!("visit-{seed}").into())),
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
                matches!(metadata.schemas.get(fields[*ordinal].schema()).unwrap().body(),
                    SchemaBody::Matrix { element, .. } if matches!(element.as_ref(), SchemaBody::Record(_)))
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
                    let by_name = |name| {
                        before[artifact
                            .inputs()
                            .iter()
                            .position(|i| {
                                mech_engine::decode_source_input_name(&i.name).unwrap() == name
                            })
                            .unwrap()]
                        .clone()
                    };
                    let mut word = 7u64;
                    for _ in 0..steps {
                        word = ((word + 1) / divisor) / divisor;
                    }
                    let expected = Data::Tuple(
                        vec![
                            by_name("seqs"),
                            by_name("label"),
                            Data::Matrix(if empty { vec![] } else { vec![Data::U64(word)] }.into()),
                        ]
                        .into(),
                    );
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
