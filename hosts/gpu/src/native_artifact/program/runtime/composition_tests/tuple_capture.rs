//! Canonical tuple captures keep independent current turn-local geometry.
use super::*;

const LENGTHS: &str = r#"+> sequence
pair := (left<[u64]>, right<[u64]>)
outer := {payload: (pair, 9u64)}
answer := (take-left<bool>?
  | true => sequence/length(outer.payload.1.1) + outer.payload.2
  | false => sequence/length(outer.payload.1.2))
(answer, outer.payload.1.1, outer.payload.1.2)
"#;
const ELEMENT: &str = r#"pair := (left<[u64]>, right<[u64]>)
answer := (take-left<bool>? | true => pair.1[at<u64>] | false => pair.2[at])
(answer, pair.1, pair.2)
"#;
const DIRECT_ELEMENT: &str = r#"lhs := left<[u64]>
rhs := right<[u64]>
slot := at<u64>
answer := (take-left<bool>? | true => lhs[slot] | false => rhs[slot])
(answer, lhs, rhs)
"#;
fn library_error_kind(error: NativeProgramExecutionError) -> String {
    match error {
        NativeProgramExecutionError::Library { error, .. } => format!("{error:?}"),
        other => panic!("expected selected library error: {other:?}"),
    }
}
fn matrix(xs: &[u64]) -> Data {
    Data::Matrix(xs.iter().copied().map(Data::U64).collect())
}
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    left: &[u64],
    right: &[u64],
    take_left: bool,
    at: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("left", matrix(left)),
            ("right", matrix(right)),
            ("take-left", Data::Bool(take_left)),
            ("at", Data::U64(at)),
        ],
        memory,
    )
}
fn last(output: &[Option<Value>]) -> Data {
    output
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
#[test]
fn tuple_matrix_captures_keep_independent_shapes_original_decoded_and_restored() {
    for artifact in artifacts(LENGTHS) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for (left, right) in [
                (vec![], vec![7, 8, 9]),
                (vec![2], vec![]),
                (vec![1, 2], vec![3, 4, 5]),
            ] {
                for take_left in [false, true] {
                    let input = args(&artifact, &memory, &left, &right, take_left, 1);
                    let aliases = input.clone();
                    let before = contents(&input);
                    let mut output = vec![None; artifact.outputs().len()];
                    let result = if restored {
                        observe::run_restored(&jit, &input, &mut output, mode.execution(), &memory)
                            .0
                    } else {
                        observe::run(&jit, &input, &mut output, mode.execution(), &memory).0
                    };
                    result.unwrap();
                    assert_eq!(
                        last(&output),
                        Data::Tuple(
                            vec![
                                Data::U64(if take_left {
                                    left.len() as u64 + 9
                                } else {
                                    right.len() as u64
                                }),
                                matrix(&left),
                                matrix(&right)
                            ]
                            .into()
                        )
                    );
                    assert_eq!(contents(&input), before);
                    assert_eq!(contents(&aliases), before);
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn tuple_matrix_capture_selected_bounds_failure_is_atomic_and_retryable() {
    for (artifact, direct_artifact) in artifacts(ELEMENT)
        .into_iter()
        .zip(artifacts(DIRECT_ELEMENT))
    {
        let mode = NativeProgramLimitMode::NoLimits;
        let direct = JitProgramArtifact::compile_with_limits(
            &direct_artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (take_left, at, fail) in [
                (false, 3, false),
                (true, 3, true),
                (true, 1, false),
                (false, 0, true),
                (false, 1, false),
            ] {
                let input = args(&artifact, &memory, &[11], &[20, 21, 22], take_left, at);
                let aliases = input.clone();
                let before = contents(&input);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = if restored {
                    observe::run_restored(&jit, &input, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &input, &mut output, mode.execution(), &memory).0
                };
                if fail {
                    let error = library_error_kind(result.unwrap_err());
                    let reference_args = args(
                        &direct_artifact,
                        &memory,
                        &[11],
                        &[20, 21, 22],
                        take_left,
                        at,
                    );
                    let mut reference_output = vec![None; direct_artifact.outputs().len()];
                    let reference_result = if restored {
                        observe::run_restored(
                            &direct,
                            &reference_args,
                            &mut reference_output,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(
                            &direct,
                            &reference_args,
                            &mut reference_output,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    };
                    assert_eq!(error, library_error_kind(reference_result.unwrap_err()));
                    drop((reference_args, reference_output));
                    assert!(output.iter().zip(&prior).all(|(now, old)| {
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    let expected = if take_left {
                        11
                    } else {
                        [20, 21, 22][at as usize - 1]
                    };
                    assert_eq!(
                        last(&output),
                        Data::Tuple(
                            vec![Data::U64(expected), matrix(&[11]), matrix(&[20, 21, 22])].into()
                        )
                    );
                }
                assert_eq!(contents(&input), before);
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
fn tuple_matrix_capture_preserves_finite_limits_and_publication() {
    resources::finite_boundaries(ELEMENT, |artifact, memory| {
        args(artifact, memory, &[11], &[20, 21, 22], true, 1)
    });
}
