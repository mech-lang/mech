//! Numeric state and invocation-current immutable captures share one native loan.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::ArrayStorage;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/invariant-numeric-arguments.mec");

#[test]
fn guarded_numeric_loads_retire_unused_data_but_preserve_bounds_errors_and_retry() {
    const CHECK: &str = r#"
walk(data<[u64]>, at<u64>, end<u64>, total<u64>) => (out<u64>) :=
  more := at < end
  ignored := (more? | true => data[at + 1u64] | false => 0u64)
  next := (more? | true => total + at | false => total)
  out := (more? | true => walk(data, at + 1u64, end, next) | false => total).
walk(input<[u64]>, 0u64, steps<u64>, seed<u64>)
"#;
    for artifact in artifacts(CHECK) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let conservative = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::GuardedNumericLoads,
        )
        .unwrap();
        let no_demand = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::DemandLiveness,
        )
        .unwrap();
        let removed_loads = |metadata: serde_json::Value| {
            metadata["compiled_numeric_functions"]
                .as_array()
                .unwrap()
                .iter()
                .map(|function| function["cleanup"]["dead_loads_removed"].as_u64().unwrap())
                .sum::<u64>()
        };
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            OptimizationPass::GuardedNumericLoads,
        )
        .unwrap();
        assert!(
            removed_loads(object.metadata()) > removed_loads(old.metadata()),
            "remove an actual unused cell load, independently of its original source check"
        );
        for restored in [false, true] {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for (values, steps, seed, expected) in [
                (vec![], 0, 0, Some(0)),
                (vec![1, 2, 3], 3, 7, Some(10)),
                (vec![], 1, 0, None),
                (vec![1, 2], 3, 0, None),
                (vec![1, 2, 3], 3, u64::MAX, None),
                (vec![2, 3], 2, 5, Some(6)),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(values.into_iter().map(Data::U64).collect()),
                        ),
                        ("steps", Data::U64(steps)),
                        ("seed", Data::U64(seed)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let mut errors = Vec::new();
                for (jit, output) in [&native, &conservative, &no_demand]
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
                    assert_eq!(result.is_ok(), expected.is_some(), "{result:?}");
                    if let Some(expected) = expected {
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
                    } else {
                        errors.push(format!("{:?}", result.unwrap_err()));
                        assert!(output.iter().zip(prior).all(|(now, old)| match (now, old) {
                            (Some(now), Some(old)) => now.shares_frozen_storage(&old),
                            (None, None) => true,
                            _ => false,
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                }
                assert!(
                    errors.windows(2).all(|pair| pair[0] == pair[1]),
                    "{errors:?}"
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

#[test]
fn lifted_arrays_preserve_the_native_scalar_carrier_tree() {
    const CHECK: &str = r#"
contains-later(values<[u64]>, at<u64>, end<u64>, key<u64>) => (out<bool>) :=
  more := at < end
  equal := (more? | true => values[at + 1u64] == key | false => false)
  out := (more? | true => (equal? | true => true | false => contains-later(values, at + 1u64, end, key)) | false => false).
distinct(values<[u64]>, at<u64>, end<u64>) => (out<bool>) :=
  more := at < end
  duplicate := (more? | true => contains-later(values, at + 1u64, end, values[at + 1u64]) | false => false)
  out := (more? | true => (duplicate? | true => false | false => distinct(values, at + 1u64, end)) | false => true).
answer := distinct(input<[u64]>, 0u64, steps<u64>)
answer
"#;
    for artifact in artifacts(CHECK) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let region = native
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| {
                region.tail.parent().is_none()
                    && region
                        .numeric_tree
                        .as_ref()
                        .is_some_and(|t| t.nodes.len() == 2)
                    && region
                        .numeric
                        .as_ref()
                        .is_some_and(|p| p.arrays.iter().all(|a| a.physical().is_none()))
            })
            .expect("lifting arrays must retain the complete native scalar-state tree");
        assert!(
            native
                .descriptor()
                .metadata()
                .buffer_frames
                .fields(region.tail.carrier())
                .is_none()
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for (values, steps, expected) in [
                (vec![], 0, Some(true)),
                (vec![1], 1, Some(true)),
                (vec![1, 2, 3, 4], 4, Some(true)),
                (vec![1, 2, 1, 4], 4, Some(false)),
                (vec![u64::MAX, 1, 0, u64::MAX], 4, Some(false)),
                (vec![1, 2, 3], 4, None),
                (vec![8, 9, 10], 3, Some(true)),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "input",
                            Data::Matrix(values.into_iter().map(Data::U64).collect()),
                        ),
                        ("steps", Data::U64(steps)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                for (jit, output) in [&native, &ordinary].into_iter().zip(&mut outputs) {
                    let prior = contents(
                        output
                            .iter()
                            .flatten()
                            .cloned()
                            .collect::<Vec<_>>()
                            .as_slice(),
                    );
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, output, mode.execution(), &memory)
                    };
                    assert_eq!(result.is_ok(), expected.is_some(), "{result:?}");
                    if let Some(expected) = expected {
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Bool(expected)
                        );
                        if core::ptr::eq(jit, &native) {
                            assert!(
                                counts
                                    .closed_tail_exits
                                    .iter()
                                    .any(|row| row[0] == u64::from(region.tail.exit())
                                        && row[3] == 0),
                                "the complete root must execute natively: {counts:?}"
                            );
                        }
                    } else {
                        assert_eq!(
                            contents(
                                output
                                    .iter()
                                    .flatten()
                                    .cloned()
                                    .collect::<Vec<_>>()
                                    .as_slice()
                            ),
                            prior
                        );
                    }
                }
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

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    values: &[u64],
    steps: u64,
    capacity: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "input",
                Data::Matrix(values.iter().copied().map(Data::U64).collect()),
            ),
            ("steps", Data::U64(steps)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

#[test]
fn invariant_numeric_capture_executes_native_reads_growth_errors_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        assert!(
            native
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .filter_map(|r| r.numeric.as_ref())
                .any(|proof| {
                    proof.arrays.iter().any(|a| {
                        matches!(a.storage, ArrayStorage::Capture { .. })
                            && a.width == 8
                            && !a.writable
                    }) && proof
                        .arrays
                        .iter()
                        .any(|a| a.appends && a.physical().is_some())
                }),
            "the invariant source array must be a native immutable capture"
        );
        for restored in [false, true] {
            for capacity in [0, 1, 4, 9] {
                let memory = mode.memory_budget();
                let mut outputs = [
                    vec![None; artifact.outputs().len()],
                    vec![None; artifact.outputs().len()],
                ];
                for (values, steps, accepts) in [
                    (vec![2, 3, 5, 7], 4, true),
                    (vec![2, 3, 5, 7], 0, true),
                    (vec![2, 3, 5, 7], 5, false),
                    (vec![], 1, false),
                    (vec![u64::MAX, 0, 8], 3, true),
                    (vec![], 0, true),
                    (vec![11, 13, 17], 2, true),
                ] {
                    let args = arguments(&artifact, &memory, &values, steps, capacity);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let mut errors = Vec::new();
                    for (jit, output) in [&native, &ordinary].into_iter().zip(&mut outputs) {
                        let prior = output.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (result, counts) = if restored {
                            observe::run_restored(jit, &args, output, mode.execution(), &memory)
                        } else {
                            observe::run(jit, &args, output, mode.execution(), &memory)
                        };
                        assert_eq!(result.is_ok(), accepts, "{result:?}");
                        if core::ptr::eq(jit, &native) && accepts && steps > 0 && capacity >= steps
                        {
                            assert!(
                                !counts.closed_tail_exits.is_empty(),
                                "selected capture loop must execute: {counts:?}"
                            );
                        }
                        if accepts {
                            let expected = Data::Tuple(
                                vec![
                                    Data::Matrix(
                                        values[..steps as usize]
                                            .iter()
                                            .copied()
                                            .map(Data::U64)
                                            .collect(),
                                    ),
                                    Data::Matrix(values.iter().copied().map(Data::U64).collect()),
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
                            errors.push(format!("{:?}", result.unwrap_err()));
                            assert!(output.iter().zip(&prior).all(|(now, old)| {
                                now.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(old.as_ref().unwrap())
                            }));
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        }
                    }
                    if !accepts {
                        assert_eq!(errors[0], errors[1]);
                    }
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
fn invariant_numeric_capture_finite_boundaries_preserve_atomic_output_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[2, 3, 5, 7], 4, 1)
    });
}

fn projected_source() -> String {
    SOURCE
        .replace("gather(values<[u64]>", "gather(pair<([u64],(u64,[u64]))>")
        .replace("values[at + 1u64]", "pair.2.2[at + 1u64] + pair.2.1")
        .replace("gather(values, at + 1u64", "gather(pair, at + 1u64")
        .replace(
            "gather(values, 0u64",
            "gather((other<[u64]>, (salt<u64>, values)), 0u64",
        )
}

// Re-read the captured first cell after replacing an aliased output. Persistent
// replacement retains the original general-matrix schema and exercises COW
// independently of append's row-vector result schema.
const ALIASED: &str = r#"+> sequence
write-step(pair<([u64],(u64,[u64]))>, at<u64>, output<[u64]>) => (out<[u64]>) :=
  ~changed := output
  changed[at + 1u64] = pair.2.2[1u64] + pair.2.1
  out := changed.
gather(pair<([u64],(u64,[u64]))>, at<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => write-step(pair, at, output) | false => output)
  out := (more? | true => gather(pair, at + 1u64, end, next) | false => output).
values := input<[u64]>
answer := gather((other<[u64]>, (salt<u64>, values)), 0u64, steps<u64>, values)
(answer, values)
"#;

#[test]
fn invariant_numeric_nested_frame_capture_preserves_scalar_leaves_opaque_owners_and_aliases() {
    let projected = projected_source();
    for aliased in [false, true] {
        let source = if aliased {
            ALIASED.to_owned()
        } else {
            projected.clone()
        };
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            assert!(
                native
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .filter_map(|r| r.numeric.as_ref())
                    .any(|proof| {
                        proof.arrays.iter().any(|a| {
                            matches!(a.storage, ArrayStorage::FrameCapture { .. })
                                && a.width == 8
                                && !a.writable
                        }) && proof.arrays.iter().any(|a| {
                            matches!(a.storage, ArrayStorage::FrameCapture { .. })
                                && a.width == 0
                                && a.witness == usize::MAX
                        }) && !proof.scalar_inputs.is_empty()
                    }),
                "exact captured frame fields must select native reads"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                for capacity in [0, 1, 8] {
                    for steps in [0, 1, 4] {
                        let values = [2u64, 3, 5, 7];
                        let args = inputs(
                            &artifact,
                            &[
                                ("input", Data::Matrix(values.map(Data::U64).into())),
                                (
                                    "other",
                                    Data::Matrix(vec![Data::U64(u64::MAX), Data::U64(0)].into()),
                                ),
                                ("salt", Data::U64(11)),
                                ("steps", Data::U64(steps)),
                                ("capacity", Data::U64(capacity)),
                            ],
                            &memory,
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let mut answer = if aliased {
                            values.iter().copied().map(Data::U64).collect::<Vec<_>>()
                        } else {
                            Vec::new()
                        };
                        if aliased {
                            answer[..steps as usize].fill(Data::U64(values[0] + 11));
                        } else {
                            answer.extend(
                                values[..steps as usize].iter().map(|&v| Data::U64(v + 11)),
                            );
                        }
                        let expected = Data::Tuple(
                            vec![
                                Data::Matrix(answer.into()),
                                Data::Matrix(values.map(Data::U64).into()),
                            ]
                            .into(),
                        );
                        for jit in [&native, &ordinary] {
                            let mut output = vec![None; artifact.outputs().len()];
                            let (result, counts) = if restored {
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
                            result.unwrap();
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
                            if core::ptr::eq(jit, &native) && aliased && steps == 4 {
                                assert!(
                                    counts.closed_tail_exits.iter().any(|exit| exit[3] == 0),
                                    "aliased replacement must resume native execution after COW: {counts:?}"
                                );
                            }
                            if core::ptr::eq(jit, &native)
                                && !aliased
                                && steps > 0
                                && capacity >= steps
                            {
                                assert!(!counts.closed_tail_exits.is_empty(), "{counts:?}");
                            }
                        }
                        assert_eq!(contents(&args), before);
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
}

#[test]
fn invariant_numeric_capture_storage_is_rederived_from_build_owned_images() {
    use crate::native_artifact::program::image::NativeProgramDescriptor;
    let source = projected_source();
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&source) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let (region, array) = regions
            .iter()
            .enumerate()
            .find_map(|(i, r)| {
                r["numeric"]["arrays"]
                    .as_array()?
                    .iter()
                    .position(|a| a["storage"]["FrameCapture"].is_object() && a["width"] == 8)
                    .map(|a| (i, a))
            })
            .unwrap();
        // SAFETY: unchanged image from this same native emitter and catalog.
        unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }.unwrap();
        for field in [
            "value", "field", "physical", "root", "schema", "width", "witness",
        ] {
            let mut changed = image.clone();
            let owner = &mut changed["descriptor"]["closed_tails"]["regions"][region]["numeric"]["arrays"]
                [array];
            let word = if matches!(field, "value" | "field" | "physical") {
                &mut owner["storage"]["FrameCapture"][field]
            } else {
                &mut owner[field]
            };
            *word = word.as_u64().unwrap().wrapping_add(1).into();
            // SAFETY: mutated metadata is rejected before any entry is used.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&changed).unwrap(),
                        &catalog,
                    )
                }
                .is_err(),
                "{field}"
            );
        }
        for field in ["value", "word", "physical", "steps"] {
            let mut changed = image.clone();
            let word = &mut changed["descriptor"]["closed_tails"]["regions"][region]["numeric"]["scalar_captures"]
                [0][field];
            *word = word.as_u64().unwrap().wrapping_add(1).into();
            // SAFETY: independently rederived capture imports reject this wire mutation.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&changed).unwrap(),
                        &catalog,
                    )
                }
                .is_err(),
                "scalar capture {field}"
            );
        }
    }
}

const DUAL: &str = r#"+> sequence
gather-two(values<[u64]>, at<u64>, end<u64>, left<[u64]>, right<[u64]>) => (out<([u64],[u64])>) :=
  more := at < end
  next := (more? | true => (sequence/append(left, values[at + 1u64]), sequence/append(sequence/append(right, at), values[at + 1u64])) | false => (left, right))
  out := (more? | true => gather-two(values, at + 1u64, end, next.1, next.2) | false => next).
values := input<[u64]>
answer := gather-two(values, 0u64, steps<u64>, sequence/with-capacity(0u64, capacity<u64>), sequence/with-capacity(0u64, capacity))
(answer.1, answer.2, values)
"#;

#[test]
fn changing_ancestor_table_keeps_current_captured_versions_after_cold_growth_and_cow() {
    let source =
        include_str!("../../../../../tests/fixtures/native/numeric-changing-capture-tree.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        assert!(
            native
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|region| {
                    region
                        .numeric_tree
                        .as_ref()
                        .is_some_and(|tree| tree.nodes.len() == 3)
                        && region.numeric.as_ref().is_some_and(|proof| {
                            proof
                                .cold_aliases
                                .iter()
                                .any(|(_, aliases)| !aliases.is_empty())
                        })
                }),
            "a changing ancestor table must retain real cold alias restoration"
        );
        let mut table = [0u64, 1, 2, 3, 4];
        let mut expected = Vec::new();
        for a in 0..3 {
            for b in a + 1..4 {
                for c in b + 1..5 {
                    expected.push(Data::U64((table[a] * 5 + table[b]) * 5 + table[c]));
                }
            }
            table[4] = (table[4] + 1) % 5;
        }
        let expected = Data::Matrix(expected.into());
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for capacity in [0, 1, 16] {
                let args = inputs(
                    &artifact,
                    &[
                        ("ids", Data::Matrix([0, 1, 2, 3, 4].map(Data::U64).into())),
                        ("capacity", Data::U64(capacity)),
                        ("end", Data::U64(5)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                for jit in [&native, &ordinary] {
                    let mut output = vec![None; artifact.outputs().len()];
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory)
                    };
                    result.unwrap();
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
                    if core::ptr::eq(jit, &native) && capacity == 16 {
                        assert!(!counts.closed_tail_exits.is_empty(), "{counts:?}");
                    }
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

const NESTED: &str = r#"+> sequence
walk(values<[u64]>, at<u64>, end<u64>, state<(u64,(u64,u64))>) => (out<(u64,(u64,u64))>) :=
  more := at < end
  next := (more? | true => (state.1 + values[at + 1u64], (state.2.1 + 1u64, state.2.2)) | false => state)
  out := (more? | true => walk(values, at + 1u64, end, next) | false => state).
answer := walk(input<[u64]>, 0u64, steps<u64>, (0u64,(0u64,7u64)))
answer.1 + answer.2.1 + answer.2.2
"#;

#[test]
fn invariant_numeric_capture_keeps_independent_changing_dimensions_and_nested_results() {
    use crate::native_artifact::program::prepare::readonly_numeric::ResultRecipe;
    for source in [DUAL, NESTED] {
        for artifact in artifacts(source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            assert!(
                native
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .filter_map(|r| r.numeric.as_ref())
                    .any(|proof| {
                        proof
                            .arrays
                            .iter()
                            .any(|a| matches!(a.storage, ArrayStorage::Capture { .. }))
                            && match &proof.result {
                                ResultRecipe::Frame { arrays, .. } if source == NESTED => {
                                    arrays.is_empty()
                                }
                                ResultRecipe::Frame { arrays, .. } => arrays.len() == 2,
                                _ => false,
                            }
                    }),
                "changing independent dimensions and scalar frame results must remain native: {source}"
            );
            for restored in [false, true] {
                for capacity in [0, 1, 8] {
                    let memory = mode.memory_budget();
                    for steps in [0, 1, 3] {
                        let values = [2, 3, 5];
                        let args = arguments(&artifact, &memory, &values, steps, capacity);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let selected = values[..steps as usize]
                            .iter()
                            .copied()
                            .map(Data::U64)
                            .collect();
                        let expected = if source == NESTED {
                            Data::U64(values[..steps as usize].iter().sum::<u64>() + steps + 7)
                        } else {
                            let right = values[..steps as usize]
                                .iter()
                                .enumerate()
                                .flat_map(|(i, &value)| [Data::U64(i as u64), Data::U64(value)])
                                .collect();
                            Data::Tuple(
                                vec![
                                    Data::Matrix(selected),
                                    Data::Matrix(right),
                                    Data::Matrix(values.map(Data::U64).into()),
                                ]
                                .into(),
                            )
                        };
                        for jit in [&native, &ordinary] {
                            let mut output = vec![None; artifact.outputs().len()];
                            let (result, counts) = if restored {
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
                            result.unwrap();
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
                            if core::ptr::eq(jit, &native) && steps > 0 && capacity >= steps * 2 {
                                assert!(!counts.closed_tail_exits.is_empty(), "{counts:?}");
                            }
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
