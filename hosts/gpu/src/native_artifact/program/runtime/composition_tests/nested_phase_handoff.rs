//! A canonical cache/header update must not pin a dead array through the
//! following native first change. A genuinely observable old result stays live.
use super::*;

fn source(keep_old: bool) -> String {
    let original = include_str!("../../../../../tests/fixtures/native/numeric-nested-phase.mec")
        .replace("next := (more? | true => sequence/append(nodes, number + at + tag + bias) | false => nodes)", "next := phase-store(nodes, number + at + tag + bias)")
        .replace("| false => nodes).", "| false => next).")
        .replacen("+> sequence", "+> sequence\nphase-store(nodes<[u64]>, word<u64>) => (out<[u64]>) :=\n  ~next := nodes\n  next[1u64] = word\n  out := next.\n", 1);
    let main = original.lines().last().unwrap();
    let signature = "values<[u32]>, nodes<[u64]>, steps<u64>, flag<bool>, allowed<bool>, deny-at<u64>, divisor<u64>, bias<u64>";
    let result = if keep_old {
        "(prepared, answer)"
    } else {
        "answer"
    };
    let kind = if keep_old { "([u64],[u64])" } else { "[u64]" };
    original.replacen(
        main,
        &format!(
            "write-header(nodes<[u64]>, head<u64>) => (out<[u64]>) :=\n  ~next := nodes\n  next[1u64] = head\n  out := next.\n\nhandoff({signature}) => (out<{kind}>) :=\n  reserved := sequence/reserve(nodes, 305257u64)\n  prepared := (allowed? | true => write-header(reserved, bias) | false => reserved)\n  answer := phase-scan(values, prepared, steps, 0u64, flag, allowed, deny-at, divisor, bias)\n  out := {result}.\n\nhandoff({signature})"
        ),
        1,
    )
}

#[test]
fn nested_numeric_phase_first_change_retires_only_dead_canonical_pins() {
    for keep_old in [false, true] {
        for artifact in artifacts(&source(keep_old)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
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
                    .filter_map(|r| r.numeric.as_ref().and_then(|p| p.phase.as_ref()))
                    .count(),
                1
            );
            for restored in [false, true] {
                let mut published = vec![None; artifact.outputs().len()];
                for (steps, flag, deny, fails) in [
                    (0, true, u64::MAX, false),
                    (3, true, u64::MAX, false),
                    (3, false, 1, true),
                    (1, false, u64::MAX, false),
                    (3, false, u64::MAX, false),
                ] {
                    let arguments =
                        nested_phase::arguments(&artifact, &memory, steps, flag, true, deny, 1, 17);
                    let aliases = arguments.clone();
                    let before = contents(&arguments);
                    let prior = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(
                            &jit,
                            &arguments,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &arguments, &mut published, mode.execution(), &memory)
                    };
                    if fails {
                        assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                        assert!(published.iter().zip(&prior).all(|(now, old)| {
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(counts.phases, 1);
                        let nodes = vec![Data::U64(
                            steps + if flag == (steps % 2 == 0) { 0 } else { 100 } + 17,
                        )];
                        let expected = if keep_old {
                            Data::Tuple(
                                vec![
                                    Data::Matrix([Data::U64(17)].into()),
                                    Data::Matrix(nodes.into()),
                                ]
                                .into(),
                            )
                        } else {
                            Data::Matrix(nodes.into())
                        };
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
                    assert_eq!(counts.first_change_backings.len(), usize::from(steps > 0));
                    for (old, current) in counts.first_change_backings {
                        assert_eq!(
                            old == current,
                            !keep_old,
                            "dead internal pins must not defeat first-change reuse; live earlier results must remain immutable"
                        );
                    }
                    assert_eq!(contents(&arguments), before);
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
}

#[test]
fn nested_numeric_phase_canonical_pin_handoff_keeps_finite_failures_and_retry() {
    resources::finite_boundaries(&source(false), |artifact, memory| {
        nested_phase::arguments(artifact, memory, 3, true, true, u64::MAX, 1, 17)
    });
}

#[test]
fn nested_numeric_phase_canonical_handoff_keeps_copy_denial_and_recovery() {
    for keep_old in [false, true] {
        for artifact in artifacts(&source(keep_old)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let mut published = vec![None; artifact.outputs().len()];
                for deny_copy in [false, true, false] {
                    let args =
                        nested_phase::arguments(&artifact, &memory, 3, true, true, u64::MAX, 1, 17);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = published.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if deny_copy {
                        observe::run_with_first_change_copy_limit(
                            &jit,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                            restored,
                            0,
                        )
                    } else if restored {
                        observe::run_restored(
                            &jit,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if keep_old && deny_copy {
                        assert!(format!("{:?}", result.unwrap_err()).contains("ClonedBytes"));
                        assert!(counts.first_change_backings.is_empty());
                        assert!(published.iter().zip(&prior).all(|(now, old)| {
                            now.as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    } else {
                        result.unwrap();
                        assert_eq!(counts.phases, 1);
                        assert_eq!(counts.first_change_backings.len(), 1);
                        assert_eq!(
                            counts.first_change_backings[0].0 == counts.first_change_backings[0].1,
                            !keep_old
                        );
                        if prior.iter().all(Option::is_some) {
                            assert_eq!(
                                published
                                    .iter()
                                    .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                                    .collect::<Vec<_>>(),
                                prior
                                    .iter()
                                    .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                                    .collect::<Vec<_>>()
                            );
                        }
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
}
