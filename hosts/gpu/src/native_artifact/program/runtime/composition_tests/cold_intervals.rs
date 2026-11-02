//! Real collection boundaries, rather than a fabricated managed child result.
use super::*;

fn source() -> &'static str {
    include_str!("../../../../../tests/fixtures/native/numeric-collector-rebase.mec")
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    checkpoint: u64,
    reject: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "nodes",
                Data::Matrix([32768, 32769, 0].map(Data::U16).into()),
            ),
            ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
            ("steps", Data::U64(steps)),
            ("checkpoint", Data::U64(checkpoint)),
            ("reject", Data::U64(reject)),
        ],
        memory,
    )
}

#[test]
fn numeric_interval_collection_receives_current_writes_and_recovers() {
    for artifact in artifacts(source()) {
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
            .find(|r| r.iteration == 0)
            .unwrap_or_else(|| {
                let mech_engine::native_artifact::NativeControlLoop::Tail(tail) =
                    &metadata.plan.iterations()[0]
                else {
                    panic!()
                };
                panic!(
                    "native rebase interval missing: {:?}",
                    crate::native_artifact::program::prepare::readonly_numeric::derive_interval(
                        &metadata.plan,
                        tail,
                        &metadata.buffer_frames,
                        &metadata.scalar_aggregates,
                        &metadata.calls,
                        &metadata.visits
                    )
                )
            });
        let proof = root.numeric.as_ref().unwrap();
        assert!(!proof.prepaid_sites.is_empty());
        assert!(proof.prepaid_sites.len() > proof.sites.len());
        assert_eq!(proof.exits.len(), 1);
        let boundary = proof.exits[0].to as u64;
        assert!(matches!(
            metadata.plan.blocks()[boundary as usize].action(),
            mech_engine::native_artifact::NativeIterationAction::CollectionBegin { .. }
        ));
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (name, steps, checkpoint, reject, expected, rejected, cold) in [
                (
                    "normal",
                    3,
                    u64::MAX,
                    u64::MAX,
                    vec![32771, 32769, 0],
                    false,
                    false,
                ),
                (
                    "rebase-after-write",
                    3,
                    1,
                    u64::MAX,
                    vec![3, 1, 0],
                    false,
                    true,
                ),
                ("reject-after-rebase", 3, 1, 2, vec![], true, true),
                (
                    "zero-trip",
                    0,
                    0,
                    u64::MAX,
                    vec![32768, 32769, 0],
                    false,
                    false,
                ),
                (
                    "retry",
                    3,
                    u64::MAX,
                    u64::MAX,
                    vec![32771, 32769, 0],
                    false,
                    false,
                ),
            ] {
                let args = arguments(&artifact, &memory, steps, checkpoint, reject);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if rejected {
                    assert!(matches!(
                        result,
                        Err(NativeProgramExecutionError::Control(
                            crate::native_artifact::NativeIterationExecutionError::Integrity { .. }
                        ))
                    ));
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
                        Data::Tuple(
                            vec![
                                Data::Matrix(expected.into_iter().map(Data::U16).collect()),
                                Data::Matrix([steps as u32, 0].map(Data::U32).into()),
                                Data::U64(steps)
                            ]
                            .into()
                        )
                    );
                }
                assert_eq!(
                    counts.phase_settlements.iter().filter(|x| x.0 == 6).count(),
                    usize::from(cold),
                    "{name}: {counts:?}"
                );
                assert_eq!(
                    counts.admits.get(&boundary).copied().unwrap_or(0),
                    u64::from(cold)
                );
                if cold {
                    assert!(counts.cold_private_arrays.iter().all(|&n| n > 0));
                }
                assert_eq!(contents(&args), before);
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
