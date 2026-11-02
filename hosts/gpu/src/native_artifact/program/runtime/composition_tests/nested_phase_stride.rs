//! The match extender advances eight bytes behind a checked length + 8 guard.
//! A positive constant stride must not require another child runtime boundary.
use super::*;

fn source(wrapping: bool) -> String {
    let increment = if wrapping {
        "bits/wrapping-add(at, 8u64)"
    } else {
        "at + 8u64"
    };
    SOURCE
        .replacen("+> sequence", "+> sequence\n+> bits", 1)
        .replacen("more := at < end", "more := at + 8u64 <= end", 1)
        .replacen(
            "phase-prefix(values, at + 1u64,",
            &format!("phase-prefix(values, {increment},"),
            1,
        )
}

fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    steps: u64,
    deny: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("values", Data::Matrix((1..=32).map(Data::U32).collect())),
            ("nodes", Data::Matrix([Data::U64(9)].into())),
            ("steps", Data::U64(steps)),
            ("flag", Data::Bool(true)),
            ("allowed", Data::Bool(true)),
            ("deny-at", Data::U64(deny)),
            ("divisor", Data::U64(1)),
            ("bias", Data::U64(0)),
        ],
        memory,
    )
}

#[test]
fn nested_numeric_phase_checked_offset_and_constant_stride_stay_native() {
    let sum: u64 = (1..=32)
        .map(|end| {
            (0..end)
                .step_by(8)
                .filter(|at| at + 8 <= end)
                .map(|at| at as u64 + 1)
                .sum::<u64>()
        })
        .sum();
    assert_eq!(sum, 356);
    for wrapping in [false, true] {
        for artifact in artifacts(&source(wrapping)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            qualify_nested_inventory(&jit);
            for restored in [false, true] {
                let mut output = vec![None; artifact.outputs().len()];
                for (steps, deny, fails) in [
                    (3, u64::MAX, false),
                    (3, 1, true),
                    (0, 0, false),
                    (1, u64::MAX, false),
                    (3, u64::MAX, false),
                ] {
                    let arguments = args(&artifact, &memory, steps, deny);
                    let aliases = arguments.clone();
                    let before = contents(&arguments);
                    let prior = output.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(
                            &jit,
                            &arguments,
                            &mut output,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &arguments, &mut output, mode.execution(), &memory)
                    };
                    if fails {
                        assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                        assert!(output.iter().zip(&prior).all(|(now, old)| {
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
                        assert_eq!(
                            counts.helpers.values().sum::<u64>(),
                            if steps == 0 { 7 } else { 8 }
                        );
                        let expected = std::iter::once(9)
                            .chain(
                                (0..steps).map(|at| sum + at + if at % 2 == 0 { 0 } else { 100 }),
                            )
                            .map(Data::U64)
                            .collect();
                        assert_eq!(
                            output
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::Matrix(expected)
                        );
                    }
                    assert_eq!(contents(&arguments), before);
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
}

#[test]
fn nested_numeric_phase_declines_wrapping_offset_zero_or_dynamic_stride() {
    for source in [
        source(true).replacen(
            "more := at + 8u64 <= end",
            "more := bits/wrapping-add(at, 8u64) <= end",
            1,
        ),
        source(false).replacen(
            "phase-prefix(values, at + 8u64,",
            "phase-prefix(values, at + 0u64,",
            1,
        ),
        source(false).replacen(
            "phase-prefix(values, at + 8u64,",
            "phase-prefix(values, at + divisor,",
            1,
        ),
        source(false).replacen(
            "phase-prefix(values, at + 8u64,",
            "phase-prefix(values, at + 18446744073709551615u64,",
            1,
        ),
    ] {
        for artifact in artifacts(&source) {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            assert!(
                !jit.descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .any(|r| r.iteration == 0
                        && r.numeric.as_ref().is_some_and(|p| p.phase.is_some()))
            );
        }
    }
}

#[test]
fn nested_numeric_phase_stride_keeps_finite_boundary_failures_and_retry() {
    resources::finite_boundaries(&source(true), |artifact, memory| {
        args(artifact, memory, 3, u64::MAX)
    });
}

#[test]
fn nested_numeric_phase_affine_guard_overflow_keeps_prior_errors_and_recovery() {
    let source = source(true).replacen(
        "phase-prefix(values, 0u64, end,",
        "phase-prefix(values, 18446744073709551612u64, end,",
        1,
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        qualify_nested_inventory(&jit);
        for restored in [false, true] {
            let mut output = vec![None; artifact.outputs().len()];
            for (steps, deny, rejection) in [
                (0, 0, None),
                (1, 0, Some("Integrity")),
                (1, u64::MAX, Some("Arithmetic")),
                (0, 0, None),
            ] {
                let arguments = args(&artifact, &memory, steps, deny);
                let before = contents(&arguments);
                let prior = output.clone();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let result = if restored {
                    observe::run_restored(&jit, &arguments, &mut output, mode.execution(), &memory)
                        .0
                } else {
                    observe::run(&jit, &arguments, &mut output, mode.execution(), &memory).0
                };
                if let Some(rejection) = rejection {
                    assert!(format!("{:?}", result.unwrap_err()).contains(rejection));
                    assert!(output.iter().zip(&prior).all(|(now, old)| {
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
                }
                assert_eq!(contents(&arguments), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
