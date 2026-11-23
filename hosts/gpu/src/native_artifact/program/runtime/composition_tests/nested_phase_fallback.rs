//! Real ordinary continuations after native ownership has already advanced.
use super::*;
#[test]
fn nested_numeric_phase_capacity_fallback_completes_without_replaying_append() {
    let source =
        include_str!("../../../../../tests/fixtures/native/numeric-nested-phase-growing.mec")
            .replace("305257u64", "2u64");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = inputs(
            &artifact,
            &[
                ("nodes", Data::Matrix([Data::U64(9)].into())),
                ("steps", Data::U64(3)),
                ("allowed", Data::Bool(true)),
                ("deny-at", Data::U64(u64::MAX)),
                ("divisor", Data::U64(1)),
            ],
            &memory,
        );
        let aliases = args.clone();
        let before = contents(&args);
        for restored in [false, true] {
            let mut policy = mode.execution();
            policy.execution.max_work = 1_000_000_000;
            let mut published = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&jit, &args, &mut published, policy, &memory)
            } else {
                observe::run(&jit, &args, &mut published, policy, &memory)
            };
            result.unwrap();
            assert_eq!(counts.phases, 1);
            assert!(
                counts
                    .phase_settlements
                    .iter()
                    .any(|&(mode, roots, children, _)| mode == 5 && roots == 1 && children > 0)
            );
            assert!(
                counts
                    .phase_settlements
                    .iter()
                    .any(|&(mode, _, _, remaining)| mode == 1 && remaining > 0)
            );
            assert_eq!(
                published
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Matrix([9, 9, 28, 75].map(Data::U64).into())
            );
            assert_eq!(contents(&args), before);
            assert_eq!(contents(&aliases), before);
            drop(published);
        }
        drop((args, aliases));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn nested_numeric_phase_envelope_overflow_declines_before_source_checks() {
    let source = SOURCE.replace("end <= sequence/length(values)", "end <= divisor");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut published = vec![None; artifact.outputs().len()];
            for (steps, allowed, rejection) in [
                (0, true, None),
                (1, false, Some("Integrity")),
                (0, true, None),
            ] {
                let args = arguments(
                    &artifact,
                    &memory,
                    steps,
                    true,
                    allowed,
                    u64::MAX,
                    u64::MAX,
                    0,
                );
                let before = contents(&args);
                let previous = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                let root_header = jit
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .find(|region| region.tail.parent().is_none())
                    .unwrap()
                    .tail
                    .initialize();
                assert_eq!(counts.phase_sites.get(&u64::from(root_header)), Some(&1));
                assert!(
                    counts
                        .phase_settlements
                        .iter()
                        .any(|&(mode, roots, children, _)| mode == 1
                            && roots == 0
                            && children == 0)
                );
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    for (now, old) in published.iter().zip(&previous) {
                        assert!(
                            now.as_ref()
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
                }
                assert_eq!(contents(&args), before);
            }
            drop(published);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
