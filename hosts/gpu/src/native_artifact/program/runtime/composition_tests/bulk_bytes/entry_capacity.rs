//! Insufficient spare capacity must decline before opening a native loan.
use super::*;

#[test]
fn insufficient_bulk_capacity_declines_before_numeric_setup_and_recovers() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert_eq!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .filter(|r| r
                    .numeric
                    .as_ref()
                    .is_some_and(|p| p.arrays.iter().any(|a| a.appends)))
                .count(),
            1
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = vec![None; artifact.outputs().len()];
            for (capacity, count, steps) in [
                (128, 0, 3),
                (0, 0, 3),
                (5, 2, 3),
                (7, 9, 1),
                (0, 255, 0),
                (8, 0, 3),
                (13, 0, 3),
                (128, 8, 3),
            ] {
                let args = arguments(
                    &artifact,
                    &memory,
                    0x0102_0304_0506_0708,
                    count,
                    steps,
                    capacity,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let previous = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, observed) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                };
                if count > 8 && steps > 0 {
                    assert!(format!("{:?}", result.unwrap_err()).contains("Library"));
                    for (current, old) in output.iter().zip(&previous) {
                        assert!(
                            current
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
                        contents(&[output.last().unwrap().as_ref().unwrap().clone()]),
                        [expected(0x0102_0304_0506_0708, count.min(8), steps)]
                    );
                }
                if capacity < 8 {
                    assert_eq!(
                        observed.closed_entry_decisions.get(&0),
                        Some(&1),
                        "an unfundable entry must decline before opening a loan: {observed:?}"
                    );
                    assert!(!observed.closed_entry_decisions.contains_key(&1));
                } else if count == 0 {
                    assert_eq!(
                        observed.closed_entry_decisions.get(&1),
                        Some(&1),
                        "funded zero-count visits must retain the native route"
                    );
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
