//! A failed native read must retain the private operand needed by its reader.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;

pub(super) const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-parent-admission-overlap.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    length: usize,
    steps: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("bytes", Data::Matrix(vec![Data::U8(97); length].into())),
            ("steps", Data::U64(steps)),
        ],
        memory,
    )
}

#[test]
fn parent_bounds_retry_keeps_private_operand_origin_publication_and_recovery() {
    for artifact in artifacts(SOURCE) {
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
        let metadata = native.descriptor().metadata();
        assert!(metadata.compositions.parents.iter().any(|parent| {
            parent.segments.iter().any(|piece| {
                metadata.compositions.entry(piece.entry as usize).is_some()
                    && piece.numeric.as_ref().is_some_and(|proof| {
                        proof.actions.iter().any(|(site, action)| {
                            let mech_engine::native_artifact::NativeIterationAction::LibraryCall {
                                operands,
                                ..
                            } = metadata.plan.blocks()[*site].action()
                            else {
                                return false;
                            };
                            matches!(action, Operation::Read { .. })
                                && piece
                                    .frame_plan
                                    .as_ref()
                                    .unwrap()
                                    .aliases
                                    .iter()
                                    .any(|alias| alias.value == operands[0])
                        })
                    })
            })
        }));
        let mut expected_errors = [None, None];
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (length, steps, expected_error) in [
                    (3, 2, None),
                    (0, 3, Some((0, "Index"))),
                    (5, 0, None),
                    (7, 4, None),
                    (1, u64::MAX, Some((1, "Arithmetic"))),
                    (2, 3, None),
                ] {
                    let args = arguments(&artifact, &memory, length, steps);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, observed) = if restored {
                        observe::run_restored(jit, &args, &mut published, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if let Some((error_index, name)) = expected_error {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(
                            error.contains(name),
                            "reference={reference} restored={restored} length={length} steps={steps}: {error}; {observed:?}"
                        );
                        assert_eq!(
                            error,
                            *expected_errors[error_index].get_or_insert(error.clone()),
                            "native retry must retain the ordinary reader's error and origin"
                        );
                        for (value, old) in published.iter().zip(&prior) {
                            assert!(
                                value
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
                        let mut expected = vec![Data::U8(97); length];
                        if length > 0 {
                            expected[0] = Data::U8(97 + (length as u8 + 1) * steps as u8);
                        }
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(expected.into())
                        );
                    }
                    if reference {
                        assert_eq!(observed.selected, 0);
                    } else {
                        assert!(
                            observed.selected > 0,
                            "the failed read must exercise a native piece"
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
}

#[test]
fn parent_bounds_retry_keeps_finite_funding_and_cleanup() {
    resources::finite_boundaries(SOURCE, |artifact, memory| arguments(artifact, memory, 3, 2));
}
