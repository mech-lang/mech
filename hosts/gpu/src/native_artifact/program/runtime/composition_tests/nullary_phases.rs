//! Payload-free states and exact scalar-word exports at nested fuel boundaries.
use super::*;

const SOURCE: &str =
    include_str!("../../../../../tests/fixtures/native/numeric-nullary-phases.mec");

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    stop: u64,
    steps: u64,
    flag: bool,
    allowed: bool,
    divisor: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", Data::Matrix([3, 5, 7].map(Data::U32).into())),
            ("stop", Data::U64(stop)),
            ("steps", Data::U64(steps)),
            ("flag", Data::Bool(flag)),
            ("allowed", Data::Bool(allowed)),
            ("divisor", Data::U64(divisor)),
        ],
        memory,
    )
}

fn unchanged(published: &[Option<Value>], prior: &[Option<Value>]) {
    for (actual, previous) in published.iter().zip(prior) {
        let (actual, previous) = (actual.as_ref().unwrap(), previous.as_ref().unwrap());
        assert!(actual.shares_frozen_storage(previous));
        assert_eq!(
            actual.canonical_data_draft().unwrap(),
            previous.canonical_data_draft().unwrap()
        );
    }
}

#[test]
fn nullary_states_pack_only_the_tag_and_keep_nonzero_terminal_tags() {
    let source = "#Choose(flag<bool>) => <u64>\n  | :Start\n  | :Yes\n  | :No.\n#Choose(flag) -> :Start\n  :Start\n    | flag -> :Yes\n    | * -> :No\n  :Yes => 7u64\n  :No => 9u64.\n#Choose(flag<bool>)\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let mech_engine::native_artifact::NativeControlLoop::Tail(root) =
            &metadata.plan.iterations()[0]
        else {
            panic!("synchronous state machine")
        };
        assert_eq!(
            metadata.scalar_aggregates.slot(root.carrier()).unwrap().1,
            1
        );
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for flag in [true, false, false, true] {
                let args = inputs(&artifact, &[("flag", Data::Bool(flag))], &memory);
                let (result, _) = if restored {
                    observe::run_restored(&jit, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut output, mode.execution(), &memory)
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
                    Data::U64(if flag { 7 } else { 9 })
                );
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn nullary_multistate_child_stays_native_and_recovers_current_arguments() {
    use mech_engine::native_artifact::NativeControlLoop;
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = jit.descriptor().metadata();
        let NativeControlLoop::Tail(child) = &metadata.plan.iterations()[1] else {
            panic!("synchronous scalar state machine")
        };
        assert_eq!(
            metadata.scalar_aggregates.slot(child.carrier()).unwrap().1,
            3
        );
        let region = &metadata.closed_tails.regions[0];
        let proof = region.numeric.as_ref().unwrap();
        let phase = proof.phase.as_ref().unwrap();
        assert_eq!(phase.loops.len(), 2);
        assert!(phase.checked_children);
        assert!(phase.cold.is_empty());
        assert!(proof.exits.iter().all(|exit| {
            !matches!(
                exit.kind,
                crate::native_artifact::program::prepare::numeric_plan::ExitKind::Unsupported
            )
        }));
        // Every live private enum at an exact child boundary uses its scalar
        // snapshot lanes, rather than inventing a managed buffer binding.
        assert!(proof.exits.iter().any(|exit| {
            exit.values.contains(&child.carrier().get())
                && !exit.frames.contains(&child.carrier().get())
                && !exit
                    .aliases
                    .iter()
                    .any(|&(v, _)| v == child.carrier().get())
        }));
        for restored in [false, true] {
            let mut published = vec![None; artifact.outputs().len()];
            for (stop, steps, flag, allowed, divisor, expected, rejection) in [
                (3, 3, true, true, 1, 145, None),
                (2, 2, false, true, 2, 108, None),
                (4, 3, true, false, 0, 0, Some("Integrity")),
                (3, 3, true, true, 0, 0, Some("Arithmetic")),
                (4, 3, true, true, 1, 0, Some("IndexOutOfRange")),
                (3, 1, false, true, 1, 115, None),
                (3, 0, true, false, 0, 0, None),
                (0, 2, true, false, 1, 100, None),
                (3, 3, true, true, 1, 145, None),
            ] {
                let args = arguments(&artifact, &memory, stop, steps, flag, allowed, divisor);
                let aliases = args.clone();
                let before = contents(&args);
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, mode.execution(), &memory)
                } else {
                    observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                };
                if let Some(rejection) = rejection {
                    let error = format!("{:?}", result.unwrap_err());
                    assert!(error.contains(rejection), "{error}");
                    unchanged(&published, &prior);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                } else {
                    result.unwrap();
                    assert_eq!(counts.phases, 1);
                    assert!(
                        counts
                            .admits
                            .keys()
                            .all(|site| { !proof.sites.contains(&(*site as usize)) })
                    );
                    assert_eq!(
                        published
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(expected)
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
            let args = arguments(&artifact, &memory, 3, 32, true, true, 1);
            let mut full = mode.execution();
            let (result, counts) = observe::run(&jit, &args, &mut published, full, &memory);
            let report = result.unwrap();
            let entry_spent = u64::MAX - counts.phase_initial_fuel[0];
            let inside_child = entry_spent + phase.loops[0].compute + 3 * phase.loops[1].compute;
            let mut reached_child_exit = 0;
            for limit in [0, 1, inside_child, report.work / 2, report.work - 1] {
                let prior = published.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                full.execution.max_work = limit;
                let (result, counts) = if restored {
                    observe::run_restored(&jit, &args, &mut published, full, &memory)
                } else {
                    observe::run(&jit, &args, &mut published, full, &memory)
                };
                let error = format!("{:?}", result.unwrap_err());
                assert!(error.contains("Work"), "{error}");
                reached_child_exit += counts
                    .child_fuel_exits
                    .iter()
                    .filter(|(_, header)| *header)
                    .count();
                unchanged(&published, &prior);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                jit.evaluate_into(&args, &mut published, mode.execution(), &memory)
                    .unwrap();
                assert_eq!(
                    published
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(2080)
                );
            }
            assert!(
                reached_child_exit > 0,
                "must execute a real scalar-state child fuel exit"
            );
            drop((args, published));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
