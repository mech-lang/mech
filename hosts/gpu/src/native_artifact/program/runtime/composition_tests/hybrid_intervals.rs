//! Nested fixed-size updates beside a real cold collection boundary.
use super::*;

fn source() -> String {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-collector-rebase.mec")
        .replacen("poke(nodes<[u16]>)", "poke-once(nodes<[u16]>)", 1);
    source.replace(
        "count(cache<[u32]>)",
        r#"poke-loop(nodes<[u16]>, at<u64>, end<u64>) => (out<[u16]>) :=
  more := at < end
  next := (more? | true => poke-once(nodes) | false => nodes)
  out := (more? | true => poke-loop(next, at + 1u64, end) | false => nodes).
poke(nodes<[u16]>) => (out<[u16]>) :=
  out := poke-loop(nodes, 0u64, 2u64).
count(cache<[u32]>)"#,
    )
}

#[test]
fn native_nested_updates_keep_a_cold_collection_exit() {
    for artifact in artifacts(&source()) {
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
                .closed_tails
                .regions
                .iter()
                .any(|region| region.tail.parent().is_none()
                    && region.numeric.as_ref().is_some_and(|proof| {
                        proof.phase.as_ref().is_some_and(|phase| phase.loops.len() > 1)
                            && proof.exits.iter().any(|exit| matches!(
                                exit.kind,
                                crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported
                            ))
                    })),
            "a cold collection must not disqualify the parent's supported child loops"
        );
    }
}

fn expected(steps: u64, checkpoint: u64, child_checkpoint: u64) -> Data {
    let mut nodes = [32768u16, 32769, 0];
    for at in 0..steps {
        for visit in 0..2 {
            nodes[0] = nodes[0].wrapping_add(1);
            if visit == child_checkpoint {
                nodes = nodes.map(|node| node.saturating_sub(32768));
            }
        }
        if at == checkpoint {
            nodes = nodes.map(|node| node.saturating_sub(32768));
        }
    }
    Data::Tuple(
        vec![
            Data::Matrix(nodes.map(Data::U16).into()),
            Data::Matrix([steps as u32, 0].map(Data::U32).into()),
            Data::U64(steps),
        ]
        .into(),
    )
}

#[test]
fn nested_cold_edges_preserve_current_buffers_publication_and_recovery() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-nested-rebase.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let root = metadata
            .closed_tails
            .regions
            .iter()
            .find(|region| region.tail.parent().is_none())
            .unwrap_or_else(|| {
                let reasons = metadata.plan.iterations().iter().filter_map(|loop_| {
                    let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = loop_ else { return None; };
                    tail.parent().is_none().then(|| crate::native_artifact::program::prepare::readonly_numeric::derive_hybrid(
                        &metadata.plan, tail, &metadata.buffer_frames, &metadata.scalar_aggregates, &metadata.calls, &metadata.visits,
                    ).map(|_| ()).err())
                }).collect::<Vec<_>>();
                panic!("nested cold root missing: {reasons:?}")
            });
        let proof = root.numeric.as_ref().unwrap();
        assert!(proof.phase.as_ref().unwrap().loops.len() >= 3);
        assert!(proof.exits.iter().any(|exit| matches!(
            exit.kind,
            crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported
        ) && metadata.plan.blocks()[exit.from].iteration()
            != Some(root.iteration)));
        let entry = proof.entry_words(root.tail.carrier().get()).unwrap();
        assert!(entry.len() < proof.words);
        assert!(entry.iter().all(|word| *word < proof.words));

        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (steps, checkpoint, child_checkpoint, reject, child_reject) in [
                (3, u64::MAX, u64::MAX, u64::MAX, u64::MAX),
                (3, 1, u64::MAX, u64::MAX, u64::MAX),
                (3, u64::MAX, 1, u64::MAX, u64::MAX),
                (3, 1, 1, u64::MAX, u64::MAX),
                (3, 1, u64::MAX, 2, u64::MAX),
                (3, u64::MAX, 0, u64::MAX, 1),
                (0, 0, 0, u64::MAX, u64::MAX),
                (3, u64::MAX, u64::MAX, u64::MAX, u64::MAX),
            ] {
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "nodes",
                            Data::Matrix([32768, 32769, 0].map(Data::U16).into()),
                        ),
                        ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
                        ("steps", Data::U64(steps)),
                        ("checkpoint", Data::U64(checkpoint)),
                        ("child-checkpoint", Data::U64(child_checkpoint)),
                        ("reject", Data::U64(reject)),
                        ("child-reject", Data::U64(child_reject)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if reject != u64::MAX || child_reject != u64::MAX {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains("Integrity"), "{error}");
                    for (actual, old) in output.iter().zip(&prior) {
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
                    assert_eq!(
                        output
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected(steps, checkpoint, child_checkpoint)
                    );
                    if steps > 0 && checkpoint == u64::MAX && child_checkpoint == u64::MAX {
                        assert_eq!(counts.phases, 1, "{counts:?}");
                        assert!(counts.cold_private_arrays.is_empty());
                    }
                }
                if steps > 0 && (checkpoint != u64::MAX || child_checkpoint != u64::MAX) {
                    assert!(
                        counts.cold_private_arrays.iter().any(|count| *count > 0),
                        "{counts:?}"
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(actual, alias)| actual.shares_frozen_storage(alias))
                );
            }
            drop(output);
        }
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn nested_cold_settlement_keeps_finite_denial_atomic_and_recovers() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-nested-rebase.mec");
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    (
                        "nodes",
                        Data::Matrix([32768, 32769, 0].map(Data::U16).into()),
                    ),
                    ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
                    ("steps", Data::U64(3)),
                    ("checkpoint", Data::U64(u64::MAX)),
                    ("child-checkpoint", Data::U64(0)),
                    ("reject", Data::U64(u64::MAX)),
                    ("child-reject", Data::U64(u64::MAX)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            let run = |output: &mut [Option<Value>], policy| {
                if restored {
                    observe::run_restored(&jit, &args, output, policy, &memory)
                } else {
                    observe::run(&jit, &args, output, policy, &memory)
                }
            };
            run(&mut output, mode.execution()).0.unwrap();
            // Match the same prior-publication state when deriving the exact
            // reached cold settlement's remaining current-account fuel.
            let (result, counts) = run(&mut output, mode.execution());
            result.unwrap();
            let remaining = counts
                .phase_settlements
                .iter()
                .find(|row| row.0 == 6)
                .unwrap()
                .3;
            let mut policy = mode.execution();
            policy.execution.max_work = (u64::MAX - remaining) + 1;
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (result, counts) = run(&mut output, policy);
            let error = format!("{:?}", result.unwrap_err());
            assert!(
                error.contains("Work") && !error.contains("InvalidProgram"),
                "{error}"
            );
            assert!(
                counts.phase_settlements.iter().any(|row| row.0 == 6),
                "{counts:?}"
            );
            assert!(
                counts.cold_private_arrays.iter().any(|count| *count > 0),
                "{counts:?}"
            );
            for (actual, old) in output.iter().zip(&prior) {
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
            run(&mut output, mode.execution()).0.unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected(3, u64::MAX, 0)
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            drop((args, aliases, output, prior));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn nested_cold_images_reconstruct_exact_edges_and_buffer_bindings() {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-nested-rebase.mec");
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let mut image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let regions = image
            .pointer("/descriptor/closed_tails/regions")
            .unwrap()
            .as_array()
            .unwrap();
        let index = regions
            .iter()
            .position(|region| region.pointer("/tail/parent") == Some(&serde_json::Value::Null))
            .unwrap();
        let exits = regions[index]
            .pointer("/numeric/exits")
            .unwrap()
            .as_array()
            .unwrap();
        let exit = exits
            .iter()
            .position(|exit| exit["kind"] == "Unsupported")
            .unwrap();
        for (suffix, replacement) in [
            (
                "phase/checked_children".to_owned(),
                serde_json::json!(false),
            ),
            (
                "phase/loops/1/parent".to_owned(),
                serde_json::json!(u32::MAX),
            ),
            ("arrays/0/field".to_owned(), serde_json::json!(u32::MAX)),
            (format!("exits/{exit}/from"), serde_json::json!(u32::MAX)),
            (format!("exits/{exit}/to"), serde_json::json!(0)),
        ] {
            let path = format!("/descriptor/closed_tails/regions/{index}/numeric/{suffix}");
            let old = image.pointer_mut(&path).unwrap();
            assert_ne!(*old, replacement);
            let original = core::mem::replace(old, replacement);
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&image).unwrap()
                ),
                "{path}"
            );
            *image.pointer_mut(&path).unwrap() = original;
        }
    }
}
