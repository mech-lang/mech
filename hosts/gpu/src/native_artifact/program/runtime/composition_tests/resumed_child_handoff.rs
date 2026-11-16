//! A denied first change must clean up after the secondary loan has settled.
use super::*;

#[test]
fn resumed_child_factory_denial_after_settlement_keeps_publication_and_recovery() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let (id, region) = metadata
            .closed_tails
            .regions
            .iter()
            .enumerate()
            .find(|(id, region)| {
                metadata.closed_tails.sites[region.tail.header() as usize] != Some(*id)
            })
            .unwrap();
        let write = region.numeric.as_ref().unwrap().actions.iter().find(|(_, action)| matches!(action, crate::native_artifact::program::prepare::readonly_numeric::Operation::Write { .. })).unwrap().0;
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut published = vec![None; artifact.outputs().len()];
            for reject in [false, true, false] {
                let args = super::super::phase_descendants::arguments(&artifact, &memory, 1, 2, 0);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if reject {
                    observe::run_with_secondary_copy_limit(
                        &jit,
                        &args,
                        &mut published,
                        mode.execution(),
                        &memory,
                        restored,
                        id,
                    )
                } else if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if reject {
                    let error = result.unwrap_err();
                    assert!(
                        matches!(error, NativeProgramExecutionError::Library { origin, error: mech_engine::native_artifact::NativeCollectionInvocationError::Plan(mech_core::MemoryPlanError::TargetLimitExceeded { ref violation }) } if origin == metadata.plan.blocks()[write].origin() && violation.dimension == mech_core::MemoryBudgetDimension::ClonedBytes && violation.limit == 0 && violation.required == 3),
                        "{error:?}"
                    );
                    assert_eq!(counts.settled_copy_rejections, [id]);
                    assert!(
                        counts
                            .tail_boundaries
                            .iter()
                            .any(|row| row.0 == write as u64
                                && row.2 == 5
                                && row.3.is_some()
                                && row.3 == row.4
                                && row.5 != 0),
                        "{counts:?}"
                    );
                    assert!(published.iter().zip(&prior).all(|(actual, old)| {
                        actual
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    }));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        held
                    );
                } else {
                    result.unwrap();
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
