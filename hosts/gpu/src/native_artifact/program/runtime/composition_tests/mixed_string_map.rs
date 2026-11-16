//! Recursive String-matrix carriers retain their own shape and closed Map owner.
use super::*;
use mech_core::snapshot::MapEntryDraft;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/mixed-string-map-carrier.mec");

fn assert_private_string_carrier(jit: &JitProgramArtifact) {
    use mech_engine::native_artifact::NativeBufferFrameLane;
    let plan = jit.descriptor().plan();
    let frames = plan.facts().buffer_frames();
    assert!(
        plan.values().iter().enumerate().any(|(slot, ty)| {
            let mech_core::SchemaBody::Tuple(fields) = ty.definition().body() else {
                return false;
            };
            matches!(fields.first(), Some(mech_core::SchemaBody::Matrix { element, .. })
            if matches!(element.as_ref(), mech_core::SchemaBody::String))
                && frames.fields_at(slot).is_some_and(|fields| {
                    matches!(
                        fields.first().map(|field| field.lane()),
                        Some(NativeBufferFrameLane::Managed { .. })
                    )
                })
        }),
        "the String-matrix argument or changing carrier must retain a private managed lane"
    );
}

fn seed() -> Data {
    Data::Map(
        [("old", 91), ("shared", 99)]
            .map(|(key, value)| MapEntryDraft {
                items: [Data::String(key.into()), Data::U64(value)].into(),
            })
            .into(),
    )
}

fn words(keys: &[&str]) -> Data {
    Data::Matrix(keys.iter().map(|key| Data::String((*key).into())).collect())
}

fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    keys: &[&str],
    steps: u64,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("words", words(keys)),
            ("initial", seed()),
            ("steps", Data::U64(steps)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn expected(keys: &[&str], steps: usize, divisor: u64) -> Data {
    let mut index = std::collections::BTreeMap::from([("old", 91), ("shared", 99)]);
    for (at, key) in keys.iter().take(steps).enumerate() {
        index.insert(*key, (at as u64 + 1) / divisor);
    }
    let result = Data::Map(
        index
            .into_iter()
            .map(|(key, value)| MapEntryDraft {
                items: [Data::String(key.into()), Data::U64(value)].into(),
            })
            .collect(),
    );
    Data::Tuple(vec![result, seed(), words(keys)].into())
}

fn last(outputs: &[Option<Value>]) -> Data {
    outputs
        .last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}

#[test]
fn string_map_recursion_keeps_empty_unicode_duplicate_shapes_aliases_and_restoration() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_private_string_carrier(&jit);
        assert!(
            jit.descriptor()
                .metadata()
                .map_owner_words
                .as_ref()
                .is_some_and(|layout| !layout.regions().is_empty()),
            "the unchanged String-matrix argument must no longer block the native Map owner"
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let long = "x".repeat(65);
            for keys in [vec![], vec!["shared"], vec!["é", "界", "é", "", &long, "🛰"]] {
                for divisor in [1, 2] {
                    let input = args(&artifact, &memory, &keys, keys.len() as u64, divisor);
                    let aliases = input.clone();
                    let before = contents(&input);
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, observed) = if restored {
                        observe::run_restored(&jit, &input, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &input, &mut output, mode.execution(), &memory)
                    };
                    result.unwrap();
                    assert_eq!(
                        observed
                            .helpers
                            .iter()
                            .filter(|((_, operation), _)| *operation
                                == GlobalHelper::MapOwnerCall.raw())
                            .map(|(_, count)| *count)
                            .sum::<u64>(),
                        keys.len() as u64,
                        "each actual insertion must execute through the native owner"
                    );
                    assert_eq!(last(&output), expected(&keys, keys.len(), divisor));
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
fn string_map_recursive_bounds_and_arithmetic_failures_keep_prior_outputs_and_retry() {
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
            for (steps, divisor, error_kind) in [
                (3, 1, None),
                (4, 1, Some("bounds")),
                (3, 0, Some("arithmetic")),
                (3, 2, None),
            ] {
                let keys = ["shared", "界", "é"];
                let input = args(&artifact, &memory, &keys, steps, divisor);
                let aliases = input.clone();
                let before = contents(&input);
                let prior = output.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = if restored {
                    observe::run_restored(&jit, &input, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &input, &mut output, mode.execution(), &memory).0
                };
                if let Some(kind) = error_kind {
                    match (kind, result.unwrap_err()) {
                        ("bounds", NativeProgramExecutionError::Library { .. })
                        | (
                            "arithmetic",
                            NativeProgramExecutionError::Control(
                                NativeIterationExecutionError::Arithmetic { .. },
                            ),
                        )
                        | (
                            "arithmetic",
                            NativeProgramExecutionError::Base(NativeExecutionError::Arithmetic {
                                ..
                            }),
                        ) => {}
                        (_, error) => panic!("wrong {kind} failure: {error:?}"),
                    }
                    assert!(output.iter().zip(&prior).all(|(now, old)| {
                        now.as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    result.unwrap();
                    assert_eq!(last(&output), expected(&keys, steps as usize, divisor));
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
fn string_map_recursive_carriers_preserve_six_exact_finite_boundaries() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        args(artifact, memory, &["shared", "界", "é"], 3, 1)
    });
}

#[test]
fn changing_shadowed_and_multi_edge_matrix_arguments_are_not_promoted() {
    for (transition, swapped) in [
        ("walk(b, a, at + 1u64, end, next)", true),
        (
            "((at == 0u64)? | true => walk(b, b, at + 1u64, end, next) | false => walk(a, b, at + 1u64, end, next))",
            false,
        ),
        ("(b? | a => walk(a, b, at + 1u64, end, next))", false),
    ] {
        let source = format!(
            r#"+> map
walk(a<[string]>, b<[string]>, at<u64>, end<u64>, index<{{string:u64}}>) => (out<{{string:u64}}>) :=
  more := at < end
  next := (more? | true => map/insert(index, a[1u64], at) | false => index)
  out := (more? | true => {transition} | false => index).
answer := walk(a<[string]>, b<[string]>, 0u64, 3u64, initial<{{string:u64}}>)
answer
"#
        );
        for artifact in artifacts(&source) {
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
                    .map_owner_words
                    .as_ref()
                    .is_none_or(|layout| layout.regions().is_empty()),
                "a changing matrix must remain in its checked carrier"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let input = inputs(
                    &artifact,
                    &[
                        ("a", words(&["a"])),
                        ("b", words(&["b", "extra"])),
                        ("initial", seed()),
                    ],
                    &memory,
                );
                let aliases = input.clone();
                let before = contents(&input);
                let mut output = vec![None; artifact.outputs().len()];
                let result = if restored {
                    observe::run_restored(&jit, &input, &mut output, mode.execution(), &memory).0
                } else {
                    observe::run(&jit, &input, &mut output, mode.execution(), &memory).0
                };
                result.unwrap();
                let expected = Data::Map(
                    [
                        ("a", if swapped { 2 } else { 0 }),
                        ("b", if swapped { 1 } else { 2 }),
                        ("old", 91),
                        ("shared", 99),
                    ]
                    .map(|(key, value)| MapEntryDraft {
                        items: [Data::String(key.into()), Data::U64(value)].into(),
                    })
                    .into(),
                );
                assert_eq!(last(&output), expected);
                assert_eq!(contents(&input), before);
                assert_eq!(contents(&aliases), before);
                drop((input, aliases, output));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

fn argument_error(error: NativeProgramExecutionError) -> String {
    match error {
        NativeProgramExecutionError::Library { error, .. } => format!("library/{error:?}"),
        NativeProgramExecutionError::Control(NativeIterationExecutionError::Arithmetic {
            ..
        })
        | NativeProgramExecutionError::Base(NativeExecutionError::Arithmetic { .. }) => {
            "arithmetic".into()
        }
        other => panic!("unexpected argument failure: {other:?}"),
    }
}

#[test]
fn invariant_argument_capture_preserves_all_argument_errors_before_zero_iteration_calls() {
    let tail = r#"keys := words<[string]>
answer := intern([keys[bad<u64>]], 1u64 / divisor<u64>, steps<u64>, initial<{string:u64}>, 1u64)
(answer, initial, keys)
"#;
    let definition = SOURCE.split("answer :=").next().unwrap();
    let optimized = format!("{definition}{tail}");
    let signature = SOURCE
        .lines()
        .find(|line| line.starts_with("intern("))
        .unwrap();
    // The nonrecursive reference already retains all its argument captures,
    // including unused ones, and cannot select this recursive transformation.
    let reference = format!("+> map\n+> sequence\n{signature}\n  out := index.\n{tail}");
    for (artifact, reference_artifact) in
        artifacts(&optimized).into_iter().zip(artifacts(&reference))
    {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let reference_jit = JitProgramArtifact::compile_with_limits(
            &reference_artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for (bad, divisor) in [(0, 1), (1, 0), (0, 0), (1, 1)] {
                let named = [
                    ("words", words(&["valid"])),
                    ("bad", Data::U64(bad)),
                    ("divisor", Data::U64(divisor)),
                    ("steps", Data::U64(0)),
                    ("initial", seed()),
                ];
                let mut outcomes = Vec::new();
                for (program, artifact) in
                    [(&jit, &artifact), (&reference_jit, &reference_artifact)]
                {
                    let input = inputs(artifact, &named, &memory);
                    let before = contents(&input);
                    let mut output = vec![None; artifact.outputs().len()];
                    let result = if restored {
                        observe::run_restored(
                            program,
                            &input,
                            &mut output,
                            mode.execution(),
                            &memory,
                        )
                        .0
                    } else {
                        observe::run(program, &input, &mut output, mode.execution(), &memory).0
                    };
                    if bad == 1 && divisor == 1 {
                        result.unwrap();
                        assert_eq!(
                            last(&output),
                            Data::Tuple(vec![seed(), seed(), words(&["valid"])].into())
                        );
                        outcomes.push(None);
                    } else {
                        outcomes.push(Some(argument_error(result.unwrap_err())));
                        assert!(output.iter().all(Option::is_none));
                    }
                    assert_eq!(contents(&input), before);
                }
                assert_eq!(
                    outcomes[0], outcomes[1],
                    "argument errors must retain their reference order"
                );
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
