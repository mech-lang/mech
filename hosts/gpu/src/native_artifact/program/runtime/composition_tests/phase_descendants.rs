//! Current byte reads after native children and ordinary cold continuations.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/phase-descendant-pieces.mec");

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
