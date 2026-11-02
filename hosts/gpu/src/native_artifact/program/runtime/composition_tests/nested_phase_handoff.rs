//! A canonical cache/header update must not pin a dead array through the
//! following native first change. A genuinely observable old result stays live.
use super::*;

fn source(keep_old: bool) -> String {
    let original = include_str!("../../../../../tests/fixtures/native/numeric-nested-phase.mec");
    let main = original.lines().last().unwrap();
    let call = main.replace("sequence/reserve(nodes<[u64]>, 305257u64)", "prepared");
    let result = if keep_old {
        format!("answer := {call}\n(prepared, answer)")
    } else {
        call
    };
    original.replacen(
        main,
        &format!(
            "~prepared := sequence/reserve(nodes<[u64]>, 305257u64)\nprepared[1u64] = bias<u64>\n{result}"
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
                        let nodes = std::iter::once(17)
                            .chain((0..steps).map(|at| {
                                26 + at + if flag == (at % 2 == 0) { 0 } else { 100 } + 17
                            }))
                            .map(Data::U64)
                            .collect();
                        let expected = if keep_old {
                            Data::Tuple(
                                vec![Data::Matrix([Data::U64(17)].into()), Data::Matrix(nodes)]
                                    .into(),
                            )
                        } else {
                            Data::Matrix(nodes)
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
