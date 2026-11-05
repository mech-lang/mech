//! A complete child can exhaust finite fuel under a live outer continuation.
use super::*;

#[test]
fn resumed_child_header_bailouts_preserve_current_prefix_outer_cursor_and_retry() {
    let source = SOURCE.replace("leaf(nodes, 0u64, 1u64)", "leaf(nodes, 0u64, 32u64)");
    assert_ne!(source, SOURCE);
    for artifact in artifacts(&source) {
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
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = super::super::phase_descendants::arguments(&artifact, &memory, 1, 2, 0);
            let aliases = args.clone();
            let before = contents(&args);
            let mut published = vec![None; artifact.outputs().len()];
            let run = |published: &mut [Option<Value>], limits| {
                if restored {
                    observe::run_restored(&jit, &args, published, limits, &memory)
                } else {
                    observe::run(&jit, &args, published, limits, &memory)
                }
            };
            run(&mut published, mode.execution()).0.unwrap();
            let (result, counts) = run(&mut published, mode.execution());
            result.unwrap();
            let expected = published
                .iter()
                .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            for partial in [false, true] {
                let remaining = counts
                    .tail_funding
                    .iter()
                    .find(|row| row.0 == id && row.2 == partial)
                    .unwrap()
                    .3;
                let spent = u64::MAX - remaining;
                let mut limits = mode.execution();
                limits.execution.max_work =
                    spent + region.compute / 2 + if partial { region.compute } else { 0 };
                let prior = published.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, observed) = run(&mut published, limits);
                assert!(
                    observed
                        .tail_bailouts
                        .iter()
                        .any(|row| row.0 == id && row.1 == partial && row.2.is_some()),
                    "partial={partial} limit={} counts={observed:?}",
                    limits.execution.max_work
                );
                assert!(
                    observed
                        .tail_boundaries
                        .iter()
                        .any(|row| row.0 == region.tail.header() as u64
                            && row.2 == 1
                            && row.3.is_some()
                            && row.3 == row.4
                            && row.5 == 0),
                    "the bailout must settle successfully without changing its outer cursor: {observed:?}"
                );
                if let Err(error) = result {
                    let denied = format!("{error:?}");
                    assert!(denied.contains("ComputeWork") || denied.contains("WorkLimit") || matches!(error, NativeProgramExecutionError::Control(crate::native_artifact::iteration_runtime::NativeIterationExecutionError::Work { .. })), "unexpected failure after settlement: {denied}");
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
                }
                assert_eq!(
                    published
                        .iter()
                        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                        .collect::<Vec<_>>(),
                    expected
                );
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                run(&mut published, mode.execution()).0.unwrap();
            }
            drop((published, aliases, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
