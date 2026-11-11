//! Nested managed leaves retain structural identity and current owner versions.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = r#"+> sequence
nested-step(tables<([u64],([u16],([u8],u64)))>, at<u64>, output<[u64]>) => (out<[u64]>) :=
  right := tables.2.1
  a := tables.1[at + 1u64]
  b<u64> := right[(at % sequence/length(right)) + 1u64]
  value := a + b + tables.2.2.2
  out := sequence/append(output, value).
nested-many(tables<([u64],([u16],([u8],u64)))>, at<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := at < end
  next := (more? | true => nested-step(tables, at, output) | false => output)
  out := (more? | true => nested-many(tables, at + 1u64, end, next) | false => output).
left-data := left<[u64]>
right-data := right<[u16]>
spare-data := spare<[u8]>
tables := (left-data, (right-data, (spare-data, salt<u64>)))
selected := nested-many(tables, 0u64, end<u64>, sequence/with-capacity(0u64, capacity<u64>))
(selected, left-data, right-data, spare-data)
"#;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    left: &[u64],
    right: &[u16],
    end: u64,
    capacity: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "left",
                Data::Matrix(left.iter().copied().map(Data::U64).collect()),
            ),
            (
                "right",
                Data::Matrix(right.iter().copied().map(Data::U16).collect()),
            ),
            (
                "spare",
                Data::Matrix(vec![Data::U8(9), Data::U8(7), Data::U8(5)].into()),
            ),
            ("salt", Data::U64(11)),
            ("end", Data::U64(end)),
            ("capacity", Data::U64(capacity)),
        ],
        memory,
    )
}

#[test]
fn nested_numeric_leaves_execute_reads_opaque_captures_errors_and_cold_growth() {
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
        let regions = &native.descriptor().metadata().closed_tails.regions;
        assert!(
            regions
                .iter()
                .filter_map(|region| region.numeric.as_ref())
                .any(|proof| {
                    proof.arrays.len() >= 4
                        && proof
                            .arrays
                            .iter()
                            .any(|a| a.width == 0 && a.witness == usize::MAX)
                }),
            "nested numeric leaves and an opaque retained owner must select native lowering"
        );
        for restored in [false, true] {
            for capacity in [0, 1, 4, 9] {
                let memory = mode.memory_budget();
                let mut outputs = [
                    vec![None; artifact.outputs().len()],
                    vec![None; artifact.outputs().len()],
                ];
                for (left, right, end, accepts) in [
                    (vec![2, 3, 5, 7], vec![13, 17], 4, true),
                    (vec![2, 3, 5, 7], vec![13, 17], 0, true),
                    (vec![2, 3, 5, 7], vec![13, 17], 5, false),
                    (vec![u64::MAX, 3, 5, 7], vec![13, 17], 4, false),
                    (vec![2, 3, 5, 7], vec![], 4, false),
                    (vec![2, 3, 5, 7], vec![13, 17], 3, true),
                ] {
                    let args = arguments(&artifact, &memory, &left, &right, end, capacity);
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
                        if core::ptr::eq(jit, &native) && accepts && end > 0 && capacity >= end {
                            assert!(
                                !counts.closed_tail_exits.is_empty(),
                                "the nested source must execute the selected native region: {counts:?}"
                            );
                        }

                        if accepts {
                            let selected = left[..end as usize]
                                .iter()
                                .enumerate()
                                .map(|(i, &a)| {
                                    Data::U64(a + u64::from(right[i % right.len()]) + 11)
                                })
                                .collect();
                            let expected = Data::Tuple(
                                vec![
                                    Data::Matrix(selected),
                                    Data::Matrix(left.iter().copied().map(Data::U64).collect()),
                                    Data::Matrix(right.iter().copied().map(Data::U16).collect()),
                                    Data::Matrix(
                                        vec![Data::U8(9), Data::U8(7), Data::U8(5)].into(),
                                    ),
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
                            assert!(output.iter().zip(prior).all(|(now, old)| {
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
fn nested_numeric_finite_boundaries_preserve_atomic_output_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, &[2, 3, 5, 7], &[13, 17], 4, 1)
    });
}

#[test]
fn nested_numeric_output_caps_preserve_first_error_and_publication() {
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
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, &[2, 3, 5, 7], &[13, 17], 4, 4);
        let before = contents(&args);
        for cap in 0..12 {
            let mut policy = mode.execution();
            policy.library.max_output_elements = Some(cap);
            let mut actual = Vec::new();
            for jit in [&native, &ordinary] {
                let mut output = vec![None; artifact.outputs().len()];
                actual.push(
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                        .map(|_| contents(&output.iter().flatten().cloned().collect::<Vec<_>>()))
                        .map_err(|e| format!("{e:?}")),
                );
            }
            assert_eq!(actual[0], actual[1], "cap={cap}");
            assert_eq!(contents(&args), before);
        }
        drop(args);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

const WRITE_SOURCE: &str = r#"+> sequence
set-word(values<[u64]>, value<u64>, at<u64>) => (out<[u64]>) :=
  ~next := values
  next[at] = value
  out := next.
write-nested(state<([u64],([u64],u64))>, at<u64>, end<u64>) => (out<([u64],([u64],u64))>) :=
  more := at < end
  next := (more? | true => set-word(state.1, state.1[at + 1u64] + state.2.1[at + 1u64] + state.2.2, at + 1u64) | false => state.1)
  out := (more? | true => write-nested((next, state.2), at + 1u64, end) | false => state).
original := data<[u64]>
result := write-nested((original,(original,1u64)),0u64,end<u64>)
(result.1,result.2.1,result.2.2,original)
"#;

#[test]
fn nested_numeric_writes_preserve_distinct_functional_aliases_and_atomic_failure() {
    for artifact in artifacts(WRITE_SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
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
                .any(|r| r
                    .numeric
                    .as_ref()
                    .is_some_and(|p| p.arrays.len() == 2 && p.writes()))
        );
        for restored in [false, true] {
            let mut outputs = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for end in [3, 0, 4, 2] {
                let data = Data::Matrix(vec![Data::U64(3), Data::U64(5), Data::U64(7)].into());
                let args = inputs(
                    &artifact,
                    &[("data", data.clone()), ("end", Data::U64(end))],
                    &memory,
                );
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
                    if end <= 3 {
                        result.unwrap();
                        let new = Data::Matrix(
                            [3u64, 5, 7]
                                .into_iter()
                                .enumerate()
                                .map(|(i, x)| {
                                    Data::U64(if (i as u64) < end { x * 2 + 1 } else { x })
                                })
                                .collect(),
                        );
                        let expected =
                            Data::Tuple(vec![new, data.clone(), Data::U64(1), data.clone()].into());
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
                        if core::ptr::eq(jit, &native) && end > 0 {
                            assert!(!counts.closed_tail_exits.is_empty(), "{counts:?}");
                        }
                    } else {
                        errors.push(format!("{:?}", result.unwrap_err()));
                        assert!(output.iter().zip(prior).all(|(now, old)| {
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
                if end > 3 {
                    assert_eq!(errors[0], errors[1]);
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(outputs);
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn nested_numeric_owner_bindings_and_shape_caps_are_rederived_from_images() {
    use crate::native_artifact::program::image::NativeProgramDescriptor;
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&native.descriptor().encode_build_owned_image().unwrap())
                .unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let at = regions
            .iter()
            .position(|r| {
                r["numeric"]["arrays"]
                    .as_array()
                    .is_some_and(|a| a.len() >= 4)
            })
            .unwrap();
        for field in ["root", "field", "physical", "schema", "width", "witness"] {
            let mut changed = image.clone();
            let array =
                &mut changed["descriptor"]["closed_tails"]["regions"][at]["numeric"]["arrays"][0];
            let word = if matches!(field, "field" | "physical") {
                &mut array["storage"]["Carrier"][field]
            } else {
                &mut array[field]
            };
            *word = word.as_u64().unwrap().wrapping_add(1).into();
            // SAFETY: wire-only negative; a forged image is never invoked.
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
        let frames = regions[at]["numeric"]["frames"].as_array().unwrap();
        let frame = frames
            .iter()
            .position(|f| !f["array_bindings"].as_array().unwrap().is_empty())
            .unwrap();
        for column in [0, 1] {
            let mut changed = image.clone();
            let word = &mut changed["descriptor"]["closed_tails"]["regions"][at]["numeric"]["frames"]
                [frame]["array_bindings"][0][column];
            *word = (word.as_u64().unwrap() + 1).into();
            // SAFETY: wire-only negative; a forged image is never invoked.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&changed).unwrap(),
                        &catalog,
                    )
                }
                .is_err()
            );
        }
        let mut changed = image.clone();
        changed["descriptor"]["closed_tails"]["regions"][at]["numeric"]["frames"][frame]["maximum_arity"] =
            0.into();
        // SAFETY: wire-only negative; a forged image is never invoked.
        assert!(
            unsafe {
                NativeProgramDescriptor::decode_build_owned_image(
                    &serde_json::to_vec(&changed).unwrap(),
                    &catalog,
                )
            }
            .is_err()
        );
    }
}

#[test]
fn nested_numeric_swapped_recursive_roots_keep_the_ordinary_path() {
    let source = WRITE_SOURCE.replace(
        "write-nested((next, state.2)",
        "write-nested((state.2.1,(next,state.2.2))",
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
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
                .all(|r| r.numeric.is_none()),
            "swapped root identities cannot authorize native mutation"
        );
        let original = vec![3u64, 5, 7];
        let data = Data::Matrix(original.iter().copied().map(Data::U64).collect());
        let args = inputs(
            &artifact,
            &[("data", data.clone()), ("end", Data::U64(3))],
            &memory,
        );
        let before = contents(&args);
        let (mut left, mut right) = (original.clone(), original.clone());
        for i in 0..3 {
            let mut next = left.clone();
            next[i] = left[i] + right[i] + 1;
            left = right;
            right = next;
        }
        let expected = Data::Tuple(
            vec![
                Data::Matrix(left.into_iter().map(Data::U64).collect()),
                Data::Matrix(right.into_iter().map(Data::U64).collect()),
                Data::U64(1),
                data,
            ]
            .into(),
        );
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, mode.execution(), &memory)
            .unwrap();
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
        assert_eq!(contents(&args), before);
        drop((args, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

fn cold_source() -> String {
    let outer = r#"nested-outer(tables<([u64],([u16],([u8],u64)))>, round<u64>, rounds<u64>, end<u64>, output<[u64]>) => (out<[u64]>) :=
  more := round < rounds
  forwarded := (tables.1,(tables.2.1,(tables.2.2.1,tables.2.2.2 + round)))
  next := (more? | true => nested-many(forwarded,0u64,end,output) | false => output)
  finished := (more? | true => sequence/append(next,sequence/length(forwarded.1)) | false => output)
  out := (more? | true => nested-outer(tables,round + 1u64,rounds,end,finished) | false => output).
left-data :="#;
    SOURCE.replace("left-data :=", outer).replace(
        "selected := nested-many(tables, 0u64, end<u64>,",
        "selected := nested-outer(tables,0u64,2u64,end<u64>,",
    )
}

#[test]
fn nested_numeric_cold_child_continuations_reify_current_managed_frames() {
    use crate::native_artifact::program::image::NativeProgramDescriptor;
    for artifact in artifacts(&cold_source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let memory = mode.memory_budget();
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
                .any(|r| r.numeric_tree.is_some()
                    && r.numeric
                        .as_ref()
                        .is_some_and(|p| !p.cold_frames.is_empty())),
            "the exact ancestor managed frame must have a cold recipe"
        );
        for restored in [false, true] {
            for capacity in [0, 1, 5, 10] {
                let args = arguments(&artifact, &memory, &[2, 3, 5, 7], &[13, 17], 4, capacity);
                let before = contents(&args);
                let selected = (0..2)
                    .flat_map(|round| {
                        [2u64, 3, 5, 7]
                            .into_iter()
                            .enumerate()
                            .map(move |(i, a)| Data::U64(a + [13, 17][i % 2] + 11 + round))
                            .chain([Data::U64(4)])
                    })
                    .collect();
                let expected = Data::Tuple(
                    vec![
                        Data::Matrix(selected),
                        Data::Matrix(
                            vec![Data::U64(2), Data::U64(3), Data::U64(5), Data::U64(7)].into(),
                        ),
                        Data::Matrix(vec![Data::U16(13), Data::U16(17)].into()),
                        Data::Matrix(vec![Data::U8(9), Data::U8(7), Data::U8(5)].into()),
                    ]
                    .into(),
                );
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
                    if core::ptr::eq(jit, &native) {
                        assert!(!counts.closed_tail_exits.is_empty(), "{counts:?}");
                        if capacity <= 1 {
                            assert!(
                                counts.closed_tail_exits.iter().any(|exit| exit[3] == 1),
                                "exercise a real ordinary header continuation: {counts:?}"
                            );
                        }
                    }
                }
                assert_eq!(contents(&args), before);
            }
        }
        let image: serde_json::Value =
            serde_json::from_slice(&native.descriptor().encode_build_owned_image().unwrap())
                .unwrap();
        let regions = image["descriptor"]["closed_tails"]["regions"]
            .as_array()
            .unwrap();
        let at = regions
            .iter()
            .position(|r| {
                r["numeric"]["cold_frames"]
                    .as_array()
                    .is_some_and(|f| !f.is_empty())
            })
            .unwrap();
        for path in [vec![0], vec![1, 0, 0], vec![1, 0, 1]] {
            let mut changed = image.clone();
            let mut word = &mut changed["descriptor"]["closed_tails"]["regions"][at]["numeric"]["cold_frames"]
                [0];
            for index in path {
                word = &mut word[index];
            }
            *word = (word.as_u64().unwrap() + 1).into();
            // SAFETY: wire-only negative; forged owner recipes are never invoked.
            assert!(
                unsafe {
                    NativeProgramDescriptor::decode_build_owned_image(
                        &serde_json::to_vec(&changed).unwrap(),
                        &catalog,
                    )
                }
                .is_err()
            );
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
