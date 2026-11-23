//! Current byte reads after native children and ordinary cold continuations.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/phase-descendant-pieces.mec");

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    nonce: u8,
    steps: u64,
    child_checkpoint: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "nodes",
                Data::Matrix([128 + nonce, 129, 7].map(Data::U8).into()),
            ),
            ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
            ("steps", Data::U64(steps)),
            ("checkpoint", Data::U64(u64::MAX)),
            ("child-checkpoint", Data::U64(child_checkpoint)),
            ("reject", Data::U64(u64::MAX)),
            ("child-reject", Data::U64(u64::MAX)),
            ("probe", Data::U64(0)),
        ],
        memory,
    )
}

#[test]
fn descendant_cold_routes_restore_child_markers_and_charge_current_source_work() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            for nonce in [1, 3] {
                let args = arguments(&artifact, &memory, nonce, 2, 0);
                let aliases = args.clone();
                let before = contents(&args);
                let (result, counts) = if restored {
                    observe::run_restored(&native, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&native, &args, &mut published, mode.execution(), &memory)
                };
                result.unwrap();
                let mut nodes = [128 + nonce, 129, 7];
                for _ in 0..2 {
                    for at in 0..2 {
                        nodes[0] = nodes[0].wrapping_add(1);
                        if at == 0 {
                            nodes = nodes.map(|n| n.saturating_sub(128));
                        }
                    }
                }
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Tuple(
                        vec![
                            Data::Matrix(nodes.map(Data::U8).into()),
                            Data::Matrix([2, 0].map(Data::U32).into()),
                            Data::U64(2)
                        ]
                        .into()
                    )
                );
                assert!(
                    counts.paid_segment_declines.values().sum::<u64>() > 0,
                    "{counts:?}"
                );
                assert!(
                    counts.segment_charges.iter().all(|row| row.1 == row.2),
                    "{counts:?}"
                );
                assert!(
                    counts.segment_charges.iter().any(|row| !row.3 && row.4),
                    "an unpaid piece must execute under the live ancestor token: {counts:?}"
                );
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
fn tracked_grandchild_fuel_handoffs_keep_active_edges_and_failure_recovery() {
    // This unselected checked branch makes a complete native traversal more
    // expensive than its actual terminating ordinary suffix. A finite account
    // can refuse the traversal while still funding synchronization and return.
    let mut expensive = String::from("expensive(nodes<[u8]>) => (out<u64>) :=\n");
    for at in 0..32 {
        expensive.push_str(&format!("  byte{at}<u64> := nodes[1u64]\n"));
    }
    expensive.push_str(&format!(
        "  out := {}.\n",
        (0..32)
            .map(|n| format!("byte{n}"))
            .collect::<Vec<_>>()
            .join(" + ")
    ));
    let source = SOURCE.replace("+> bits\n", &format!("+> bits\n{expensive}"))
        .replace("leaf(nodes, 0u64, 1u64)", "leaf(nodes, 0u64, 4u64)")
        .replace("child(nodes, 0u64, 2u64,", "child(nodes, 0u64, 1u64,")
        .replace("  next := (more? | true => poke(nodes) | false => nodes)",
            "  extra := ((at == 18446744073709551615u64)? | true => expensive(nodes) | false => 0u64)\n  unselected! := extra == 0u64\n  next := (more? | true => poke(nodes) | false => nodes)");
    assert_ne!(source, SOURCE);
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = native.descriptor().metadata();
        let root = metadata
            .closed_tails
            .regions
            .iter()
            .find(|r| {
                r.tail.parent().is_none() && r.numeric.as_ref().is_some_and(|p| p.phase.is_some())
            })
            .unwrap();
        let phase = root.numeric.as_ref().unwrap().phase.as_ref().unwrap();
        let leaf = phase
            .loops
            .iter()
            .rev()
            .find(|d| {
                metadata
                    .compositions
                    .boundary(d.initialize as usize)
                    .is_some_and(|(_, entry)| entry)
            })
            .unwrap();
        let parent = phase
            .loops
            .iter()
            .find(|d| Some(d.iteration) == leaf.parent)
            .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, 1, 1, u64::MAX);
            let aliases = args.clone();
            let before = contents(&args);
            let mut published = vec![None; artifact.outputs().len()];
            let run = |published: &mut [Option<Value>], limits| {
                if restored {
                    observe::run_restored(&native, &args, published, limits, &memory)
                } else {
                    observe::run(&native, &args, published, limits, &memory)
                }
            };
            run(&mut published, mode.execution()).0.unwrap();
            let expected = published
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            for header in [false, true] {
                let (result, counts) = run(&mut published, mode.execution());
                result.unwrap();
                let mut limits = mode.execution();
                limits.execution.max_work = if header {
                    u64::MAX - counts.phase_write_fuel[0] + 3 * leaf.compute + leaf.compute
                        - leaf.compute / 4
                } else {
                    u64::MAX - counts.phase_initial_fuel[0]
                        + phase.loops[0].compute
                        + parent.compute
                        + leaf.compute / 2
                };
                let prior = published.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = run(&mut published, limits);
                assert!(
                    counts
                        .child_fuel_exits
                        .iter()
                        .any(|(_, actual)| *actual == header),
                    "{counts:?}"
                );
                assert!(
                    counts.parent_marker_handoffs.iter().any(|row| row.4 == 7
                        && if header {
                            row.1 == leaf.iteration && row.3 == Some(leaf.iteration as usize)
                        } else {
                            row.1 == parent.iteration && row.3 != Some(leaf.iteration as usize)
                        }),
                    "{counts:?}"
                );
                println!(
                    "grandchild-fuel restored={restored} header={header} completed={} markers={:?} returned={:?}",
                    result.is_ok(),
                    counts.parent_marker_handoffs,
                    counts.completed_parent_children
                );
                if header {
                    assert!(
                        counts
                            .completed_parent_children
                            .contains(&(leaf.iteration, parent.iteration)),
                        "the restored child return must be consumed before any later resource denial: {counts:?}"
                    );
                }
                if let Err(error) = result {
                    let error = format!("{error:?}");
                    assert!(
                        error.contains("Work") && !error.contains("InvalidProgram"),
                        "{error}"
                    );
                    for (current, old) in published.iter().zip(&prior) {
                        assert!(
                            current
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
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
                run(&mut published, mode.execution()).0.unwrap();
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
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop((args, aliases, published));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

fn expected(steps: u64, checkpoint: u64, child_checkpoint: u64, nonce: u8) -> Data {
    let mut nodes = [128 + nonce, 129, 7];
    for at in 0..steps {
        for visit in 0..2 {
            nodes[0] = nodes[0].wrapping_add(1);
            if visit == child_checkpoint {
                nodes = nodes.map(|node| node.saturating_sub(128));
            }
        }
        if at == checkpoint {
            nodes = nodes.map(|node| node.saturating_sub(128));
        }
    }
    Data::Tuple(
        vec![
            Data::Matrix(nodes.map(Data::U8).into()),
            Data::Matrix([steps as u32, 0].map(Data::U32).into()),
            Data::U64(steps),
        ]
        .into(),
    )
}

#[test]
fn ordinary_phase_descendant_pieces_keep_their_full_source_price() {
    for artifact in artifacts(SOURCE) {
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = native.descriptor().metadata();
        let root = metadata
            .closed_tails
            .regions
            .iter()
            .find(|region| {
                region.tail.parent().is_none()
                    && region.numeric.as_ref().is_some_and(|p| p.phase.is_some())
            })
            .unwrap();
        let proof = root.numeric.as_ref().unwrap();
        for proof in metadata
            .closed_tails
            .regions
            .iter()
            .filter_map(|r| r.numeric.as_ref())
            .filter(|p| p.phase.is_some())
        {
            assert!(
                proof.exits.iter().all(|exit| metadata
                    .compositions
                    .boundary(exit.from)
                    .is_none_or(|(_, initialize)| initialize)),
                "a cold cut must not bypass a tracked completed-child edge"
            );
        }
        let pieces = metadata
            .compositions
            .parents
            .iter()
            .filter(|parent| parent.iteration != root.iteration)
            .flat_map(|parent| &parent.segments)
            .filter(|segment| {
                metadata
                    .compositions
                    .entry(segment.entry as usize)
                    .is_some()
                    && segment
                        .sites
                        .iter()
                        .all(|&site| proof.sites.binary_search(&(site as usize)).is_ok())
            })
            .collect::<Vec<_>>();
        assert!(
            !pieces.is_empty(),
            "ordinary native-phase descendants must retain parent pieces"
        );
        assert!(
            pieces
                .iter()
                .all(|segment| segment.prepaid_region.is_none())
        );
        assert!(
            pieces
                .iter()
                .any(|segment| segment.numeric.as_ref().unwrap().views.len() > 0)
        );
    }
}

#[test]
fn descendant_reads_keep_current_owners_errors_publication_and_retry() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(jit.descriptor().metadata().closed_tails.regions.iter().any(|r| {
            r.tail.parent().is_none()
                && r.numeric.as_ref().is_some_and(|p| {
                    p.phase.as_ref().is_some_and(|phase| phase.loops.len() >= 3)
                        && p.exits.iter().any(|e| matches!(e.kind,
                            crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported))
                })
        }));
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            for (
                nonce,
                (steps, checkpoint, child_checkpoint, reject, child_reject, probe, error),
            ) in [
                (3, u64::MAX, u64::MAX, u64::MAX, u64::MAX, 0, ""),
                (3, 0, u64::MAX, u64::MAX, u64::MAX, 0, ""),
                (3, u64::MAX, 0, u64::MAX, u64::MAX, 0, ""),
                (3, 1, 1, u64::MAX, u64::MAX, 0, ""),
                (3, 0, u64::MAX, u64::MAX, 0, u64::MAX, "Integrity"),
                (3, 0, u64::MAX, 0, u64::MAX, u64::MAX, "Integrity"),
                (3, 0, u64::MAX, u64::MAX, u64::MAX, 2, "Bounds"),
                (3, 0, u64::MAX, u64::MAX, u64::MAX, u64::MAX, "Arithmetic"),
                (0, 0, 0, u64::MAX, u64::MAX, u64::MAX, ""),
                (3, 0, u64::MAX, u64::MAX, u64::MAX, 0, ""),
            ]
            .into_iter()
            .enumerate()
            {
                let nonce = nonce as u8;
                let args = inputs(
                    &artifact,
                    &[
                        (
                            "nodes",
                            Data::Matrix([128 + nonce, 129, 7].map(Data::U8).into()),
                        ),
                        ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
                        ("steps", Data::U64(steps)),
                        ("checkpoint", Data::U64(checkpoint)),
                        ("child-checkpoint", Data::U64(child_checkpoint)),
                        ("reject", Data::U64(reject)),
                        ("child-reject", Data::U64(child_reject)),
                        ("probe", Data::U64(probe)),
                    ],
                    &memory,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if error.is_empty() {
                    result.unwrap();
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        expected(steps, checkpoint, child_checkpoint, nonce)
                    );
                    if steps > 0 && (checkpoint != u64::MAX || child_checkpoint != u64::MAX) {
                        assert!(
                            counts.phase_settlements.iter().any(|row| row.0 == 6),
                            "{counts:?}"
                        );
                    }
                } else {
                    let actual = format!("{:?}", result.unwrap_err());
                    assert!(!actual.contains("InvalidProgram"), "{actual}");
                    if error == "Bounds" {
                        assert!(actual.contains("IndexOutOfRange"), "{actual}");
                    } else {
                        assert!(actual.contains(error), "{actual}");
                    }
                    for (current, old) in published.iter().zip(&prior) {
                        assert!(
                            current
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        );
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
