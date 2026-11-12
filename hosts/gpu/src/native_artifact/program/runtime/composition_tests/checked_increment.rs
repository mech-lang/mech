//! Exercise the actual emitted optimization and its independent checked path.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"walk(a<[u64]>, i<u64>, end<u64>) => (out<[u64]>) :=
  old := a[i + 1u64]
  incoming := old + 1u64
  ~next := a
  next[i + 1u64] = incoming
  out := ((i + 1u64 < end)? | true => walk(next, i + 1u64, end) | false => next).
walk(values<[u64]>, 0u64, count<u64>)
"#;

const APPEND_SOURCE: &str = r#"+> sequence
<state> := <([u64],[u64])>
walk(a<[u64]>, keys<[u64]>, i<u64>, end<u64>) => (out<state>) :=
  next-keys := sequence/append(keys, i)
  old := a[1u64]
  incoming := old + 1u64
  ~next := a
  next[1u64] = incoming
  out := ((i + 1u64 < end)? | true => walk(next, next-keys, i + 1u64, end) | false => (next, next-keys)).
built := walk(values<[u64]>, sequence/with-capacity(0u64, 4u64), 0u64, count<u64>)
counts := built.1
keys := built.2
counts[1u64] + sequence/length(keys)
"#;

const INDEPENDENT_SOURCE: &str = r#"<state> := <([u64],[u64])>
walk(a<[u64]>, b<[u64]>, i<u64>, end<u64>) => (out<state>) :=
  old := a[i + 1u64]
  ~posted := b
  posted[i + 1u64] = i
  incoming := old + 1u64
  ~next := a
  next[i + 1u64] = incoming
  out := ((i + 1u64 < end)? | true => walk(next, posted, i + 1u64, end) | false => (next, posted)).
built := walk(values<[u64]>, postings<[u64]>, 0u64, count<u64>)
(built.1, built.2)
"#;

fn independent_args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    values: &[u64],
    postings: &[u64],
    count: u64,
    alias: bool,
) -> Vec<Value> {
    let mut args = inputs(
        artifact,
        &[
            (
                "values",
                Data::Matrix(values.iter().copied().map(Data::U64).collect()),
            ),
            (
                "postings",
                Data::Matrix(postings.iter().copied().map(Data::U64).collect()),
            ),
            ("count", Data::U64(count)),
        ],
        memory,
    );
    if alias {
        let index = |name| {
            artifact
                .inputs()
                .iter()
                .position(|input| {
                    mech_engine::decode_source_input_name(&input.name).unwrap() == name
                })
                .unwrap()
        };
        let a = index("values");
        let b = index("postings");
        assert_eq!(artifact.inputs()[a].schema, artifact.inputs()[b].schema);
        args[b] = args[a].clone();
        assert!(args[a].shares_frozen_storage(&args[b]));
    }
    args
}

#[test]
fn independent_view_updates_preserve_shared_backing_first_change_errors_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(INDEPENDENT_SOURCE) {
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let previous = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::IndependentViewUpdates,
        )
        .unwrap();
        let checked = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::CheckedIncrementWrites,
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &catalog,
            mode.compilation(),
        )
        .unwrap();
        let old_object = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            OptimizationPass::IndependentViewUpdates,
        )
        .unwrap();
        let count = |object: &crate::native_artifact::NativeProgramObject| {
            let metadata = object.metadata();
            [
                "compiled_numeric_write_loops",
                "compiled_readonly_numeric_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().unwrap())
            .map(|r| {
                r["proven_independent_view_increment_writes"]
                    .as_array()
                    .unwrap()
                    .len()
            })
            .sum::<usize>()
        };
        assert!(
            count(&object) > 0,
            "select the emitted cross-write optimization"
        );
        assert_eq!(count(&old_object), 0);
        for restored in [false, true] {
            for alias in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs =
                    std::array::from_fn::<_, 4, _>(|_| vec![None; artifact.outputs().len()]);
                for (values, postings, rounds, accepts) in [
                    (&[0, 2, 3][..], &[0, 2, 3][..], 3, true),
                    (&[1, u64::MAX, 3][..], &[0, 0, 0][..], 3, false),
                    (&[u64::MAX, 2, 3][..], &[9, 9, 9][..], 3, false),
                    (&[1, 2, 3][..], &[0, 0, 0][..], 4, false),
                    (&[1, 2, 3][..], &[0][..], 3, alias),
                    (&[7, 8, 9][..], &[0, 0, 0][..], 3, true),
                ] {
                    let args =
                        independent_args(&artifact, &memory, values, postings, rounds, alias);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut outcomes = Vec::new();
                    for (jit, output) in [&native, &previous, &checked, &ordinary]
                        .into_iter()
                        .zip(&mut outputs)
                    {
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, _) = if restored {
                            observe::run_restored(jit, &args, output, mode.execution(), &memory)
                        } else {
                            observe::run(jit, &args, output, mode.execution(), &memory)
                        };
                        assert_eq!(result.is_ok(), accepts, "{result:?}");
                        outcomes.push((
                            result.as_ref().copied().map_err(|e| format!("{e:?}")),
                            contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                        ));
                        if accepts {
                            let expected = Data::Tuple(
                                vec![
                                    Data::Matrix(values.iter().map(|v| Data::U64(v + 1)).collect()),
                                    Data::Matrix((0..rounds).map(Data::U64).collect()),
                                ]
                                .into(),
                            );
                            assert_eq!(
                                output
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                expected
                            );
                        } else {
                            assert!(output.iter().zip(prior).all(|(now, old)| match (now, old) {
                                (Some(a), Some(b)) => a.shares_frozen_storage(&b),
                                (None, None) => true,
                                _ => false,
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                    }
                    // The two emission-only ablations retain the identical
                    // numeric plan and all its charges. Disabling numeric
                    // emission altogether has a different ordinary fee plan.
                    assert!(
                        outcomes[..3].windows(2).all(|p| p[0] == p[1]),
                        "{outcomes:?}"
                    );
                    assert!(
                        outcomes.windows(2).all(|p| p[0].0.as_ref().map(|_| ())
                            == p[1].0.as_ref().map(|_| ())
                            && p[0].1 == p[1].1),
                        "{outcomes:?}"
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn independent_view_updates_preserve_finite_admission_and_publication() {
    resources::finite_boundaries(INDEPENDENT_SOURCE, |artifact, memory| {
        independent_args(artifact, memory, &[0, 2, 3], &[0, 2, 3], 3, true)
    });
}

fn counts(artifact: &ProgramArtifact) -> (usize, usize) {
    let object = crate::native_artifact::emit_program_object_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeProgramLimitMode::NoLimits.compilation(),
    )
    .unwrap();
    let metadata = object.metadata();
    [
        "compiled_numeric_write_loops",
        "compiled_readonly_numeric_loops",
    ]
    .into_iter()
    .flat_map(|key| metadata[key].as_array().unwrap())
    .fold((0, 0), |(proved, stores), region| {
        (
            proved
                + region["proven_checked_increment_writes"]
                    .as_array()
                    .unwrap()
                    .len(),
            stores + region["typed_stores"].as_array().unwrap().len(),
        )
    })
}

#[test]
fn checked_increment_writes_preserve_aliases_overflow_bounds_publication_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in [SOURCE, APPEND_SOURCE].into_iter().flat_map(artifacts) {
        assert!(
            counts(&artifact).0 > 0,
            "fixture must select the actual optimization"
        );
        let optimized = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let checked = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::CheckedIncrementWrites,
        )
        .unwrap();
        let mut outcomes = Vec::new();
        for jit in [&optimized, &checked] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            let mut actual = Vec::new();
            for (values, count) in [
                (vec![1, 2, 3], 3),
                (vec![1, u64::MAX, 3], 3),
                (vec![u64::MAX, 2, 3], 3),
                (vec![4, 5, 6], 4),
                (vec![7, 8, 9], 3),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "values",
                            Data::Matrix(values.into_iter().map(Data::U64).collect()),
                        ),
                        ("count", Data::U64(count)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let result = jit.evaluate_into(&args, &mut output, mode.execution(), &memory);
                assert_eq!(contents(&aliases), before);
                actual.push((
                    result
                        .as_ref()
                        .map(|r| (r.work, r.comparison_work))
                        .map_err(|e| format!("{e:?}")),
                    contents(&output.iter().flatten().cloned().collect::<Vec<_>>()),
                ));
            }
            drop(output);
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
            outcomes.push(actual);
        }
        assert_eq!(outcomes[0], outcomes[1]);
    }
    for source in [SOURCE.replace("incoming := old + 1u64", "incoming := 0u64"),
                   SOURCE.replace("next[i + 1u64] = incoming", "next[i + 2u64] = incoming"),
                   SOURCE.replace("  incoming := old + 1u64\n  ~next := a",
                       "  ~staged := a\n  staged[i + 1u64] = 0u64\n  incoming := old + 1u64\n  ~next := staged")] {
        for artifact in artifacts(&source) {
            let (proved, stores) = counts(&artifact);
            assert!(stores > 0, "negative fixture must retain native writes");
            assert_eq!(proved, 0, "unchanged or different-cell stores keep their equality/bounds checks");
        }
    }
}

fn narrow_source(source: &str, width: u8) -> String {
    let kind = if width == 1 { "u8" } else { "u32" };
    let changed = source
        .replace("[u64]", &format!("[{kind}]"))
        .replace("old + 1u64", &format!("old + 1{kind}"))
        .replace("  posted[i + 1u64] = i\n",
            &format!("  posting<{kind}> := i\n  posted[i + 1u64] = posting\n"))
        .replace(
            "counts[1u64] + sequence/length(keys)",
            "wide-count<u64> := counts[1u64]\nwide-count + sequence/length(keys)",
        );
    if source.contains("next-keys") {
        changed.replace(&format!("keys<[{kind}]>"), "keys<[u64]>")
            .replace(&format!("<([{kind}],[{kind}])>"), &format!("<([{kind}],[u64])>"))
    } else {
        changed
    }
}

fn narrow_word(width: u8, value: u64) -> Data {
    if width == 1 {
        Data::U8(u8::try_from(value).unwrap())
    } else {
        Data::U32(u32::try_from(value).unwrap())
    }
}

fn narrow_matrix(width: u8, values: &[u64]) -> Data {
    Data::Matrix(values.iter().map(|&v| narrow_word(width, v)).collect())
}

fn narrow_inputs(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    width: u8,
    values: &[u64],
    postings: Option<&[u64]>,
    count: u64,
    alias: bool,
) -> Vec<Value> {
    let mut data = vec![
        ("values", narrow_matrix(width, values)),
        ("count", Data::U64(count)),
    ];
    if let Some(postings) = postings {
        data.push(("postings", narrow_matrix(width, postings)));
    }
    let mut args = inputs(artifact, &data, memory);
    if alias {
        let index = |name| artifact.inputs().iter().position(|input| {
            mech_engine::decode_source_input_name(&input.name).unwrap() == name
        }).unwrap();
        let a = index("values");
        let b = index("postings");
        assert_eq!(artifact.inputs()[a].schema, artifact.inputs()[b].schema);
        args[b] = args[a].clone();
        assert!(args[a].shares_frozen_storage(&args[b]));
    }
    args
}

fn narrow_update_case(
    native: &[&JitProgramArtifact],
    args: &[Value],
    outputs: &mut [Vec<Option<Value>>],
    expected: Option<Data>,
    restored: bool,
    memory: &ManagedMemoryBudget,
) {
    let mode = NativeProgramLimitMode::NoLimits;
    let aliases = args.to_vec();
    let before = contents(args);
    let mut outcomes = Vec::new();
    for (jit, output) in native.iter().zip(outputs) {
        let prior = output.clone();
        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let (result, _) = if restored {
            observe::run_restored(jit, args, output, mode.execution(), memory)
        } else {
            observe::run(jit, args, output, mode.execution(), memory)
        };
        assert_eq!(result.is_ok(), expected.is_some(), "{result:?}");
        if let Some(expected) = &expected {
            assert_eq!(output.last().unwrap().as_ref().unwrap()
                .canonical_data_draft().unwrap(), *expected);
        } else {
            assert!(output.iter().zip(prior).all(|(now, old)| match (now, old) {
                (Some(a), Some(b)) => a.shares_frozen_storage(&b),
                (None, None) => true,
                _ => false,
            }));
            assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), retained);
        }
        outcomes.push((result.as_ref().copied().map_err(|e| format!("{e:?}")),
            contents(&output.iter().flatten().cloned().collect::<Vec<_>>())));
    }
    assert!(outcomes.windows(2).all(|p| p[0] == p[1]), "{outcomes:?}");
    assert_eq!(contents(args), before);
    assert_eq!(contents(&aliases), before);
}

#[test]
fn narrow_increment_writes_preserve_checked_errors_aliases_publication_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for width in [1, 4] {
        let max = if width == 1 { u8::MAX as u64 } else { u32::MAX as u64 };
        for (append, source) in [(false, SOURCE), (true, APPEND_SOURCE)] {
            let source = narrow_source(source, width);
            for artifact in artifacts(&source) {
                assert!(counts(&artifact).0 > 0, "actual narrow emitted update must be proved");
                let optimized = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation()).unwrap();
                let checked = JitProgramArtifact::compile_for_test(&artifact, &catalog, OptimizationPass::CheckedIncrementWrites).unwrap();
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut outputs = vec![vec![None; artifact.outputs().len()]; 2];
                    for (values, count) in [
                        (vec![1, 2, 3], 3), (vec![max, 2, 3], 3),
                        (vec![1, 2, max], 3), (vec![max - 1, 2, 3], 3),
                        (vec![0, 1], 3), (vec![], 1), (vec![7, 8, 9], 3),
                    ] {
                        let expected = if append {
                            values.first().and_then(|v| v.checked_add(count))
                                .filter(|&v| v <= max).map(|v| Data::U64(v + count))
                        } else if values.len() >= count as usize
                            && values[..count as usize].iter().all(|&v| v < max) {
                            let mut changed = values.clone();
                            for v in &mut changed[..count as usize] { *v += 1; }
                            Some(narrow_matrix(width, &changed))
                        } else { None };
                        let args = narrow_inputs(&artifact, &memory, width, &values, None, count, false);
                        narrow_update_case(&[&optimized, &checked], &args, &mut outputs, expected, restored, &memory);
                    }
                    drop(outputs);
                    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
                }
            }
            for negative in [
                source.replace(&format!("incoming := old + 1{}", if width == 1 { "u8" } else { "u32" }),
                               &format!("incoming := 0{}", if width == 1 { "u8" } else { "u32" })),
                source.replace("next[i + 1u64] = incoming", "next[i + 2u64] = incoming")
                    .replace("next[1u64] = incoming", "next[2u64] = incoming"),
            ] {
                for artifact in artifacts(&negative) {
                    let (proved, stores) = counts(&artifact);
                    assert!(stores > 0);
                    assert_eq!(proved, 0, "unrelated narrow stores keep their original guards");
                }
            }
        }
    }
}

#[test]
fn narrow_independent_updates_preserve_shared_backing_source_errors_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for width in [1, 4] {
        let max = if width == 1 { u8::MAX as u64 } else { u32::MAX as u64 };
        for artifact in artifacts(&narrow_source(INDEPENDENT_SOURCE, width)) {
            let optimized = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation()).unwrap();
            let previous = JitProgramArtifact::compile_for_test(&artifact, &catalog, OptimizationPass::IndependentViewUpdates).unwrap();
            let checked = JitProgramArtifact::compile_for_test(&artifact, &catalog, OptimizationPass::CheckedIncrementWrites).unwrap();
            let object = crate::native_artifact::emit_program_object_with_limits(&artifact, &catalog, mode.compilation()).unwrap();
            let metadata = object.metadata();
            let total = ["compiled_numeric_write_loops", "compiled_readonly_numeric_loops"].into_iter()
                .flat_map(|k| metadata[k].as_array().unwrap())
                .map(|r| r["proven_independent_view_increment_writes"].as_array().unwrap().len()).sum::<usize>();
            assert!(total > 0, "actual emitted cross-write optimization must be selected");
            for restored in [false, true] {
                for alias in [false, true] {
                    let memory = mode.memory_budget();
                    let mut outputs = vec![vec![None; artifact.outputs().len()]; 3];
                    for (values, postings, count) in [
                        (vec![0, 2, 3], vec![0, 2, 3], 3),
                        (vec![1, max, 3], vec![0, 0, 0], 3),
                        (vec![max, 2, 3], vec![9, 9, 9], 3),
                        (vec![1, 2, 3], vec![0, 0, 0], 4),
                        (vec![1, 2, 3], vec![0], 3),
                        (vec![7, 8, 9], vec![0, 0, 0], 3),
                    ] {
                        let original_posts = if alias { &values } else { &postings };
                        let expected = if values.len() >= count as usize
                            && original_posts.len() >= count as usize
                            && values[..count as usize].iter().all(|&v| v < max) {
                            let mut counts = values.clone();
                            let mut posts = original_posts.clone();
                            for i in 0..count as usize { counts[i] += 1; posts[i] = i as u64; }
                            Some(Data::Tuple(vec![narrow_matrix(width, &counts), narrow_matrix(width, &posts)].into()))
                        } else { None };
                        let args = narrow_inputs(&artifact, &memory, width, &values, Some(&postings), count, alias);
                        narrow_update_case(&[&optimized, &previous, &checked], &args, &mut outputs, expected, restored, &memory);
                    }
                    drop(outputs);
                    assert_eq!((memory.used_bytes(), memory.retained_snapshot_nodes()), (0, 0));
                }
            }
        }
    }
}

#[test]
fn narrow_increment_updates_preserve_finite_admission() {
    for width in [1, 4] {
        for source in [SOURCE, APPEND_SOURCE, INDEPENDENT_SOURCE] {
            resources::finite_boundaries(&narrow_source(source, width), |artifact, memory| {
                narrow_inputs(artifact, memory, width, &[0, 2, 3],
                    source.contains("postings<").then_some(&[0, 2, 3][..]), 3,
                    source.contains("postings<"))
            });
        }
    }
}
