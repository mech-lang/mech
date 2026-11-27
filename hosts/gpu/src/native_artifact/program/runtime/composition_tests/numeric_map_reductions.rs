//! Ordered scalar reductions reuse native map DAGs and their current loans.
use super::numeric_maps::{Flags, run, typed_arguments};
use super::*;
use mech_core::snapshot::F64Bits;

const SOURCE: &str =
    "+> stats\nstats/sum/column([value * factor<f64> | value <- values<[f64]>])[1u64]\n";
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    bits: &[u64],
    factor: u64,
) -> Vec<Value> {
    typed_arguments(
        artifact,
        memory,
        &[
            (
                "values",
                Data::Matrix(
                    bits.iter()
                        .copied()
                        .map(|b| Data::F64(F64Bits::from_bits(b)))
                        .collect::<Vec<_>>()
                        .into(),
                ),
                &[1, bits.len() as u64],
            ),
            ("factor", Data::F64(F64Bits::from_bits(factor)), &[]),
        ],
    )
}
#[test]
fn numeric_map_reductions_preserve_order_bits_empty_inputs_and_native_scalar_completion() {
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let metadata = jit.descriptor().metadata();
        let region = metadata
            .numeric_maps
            .regions
            .iter()
            .find(|r| r.reduction.is_some());
        if region.is_none() {
            for (i, b) in metadata.plan.blocks().iter().enumerate() {
                eprintln!("{i}: {b:?}");
            }
            for c in metadata.plan.collections() {
                eprintln!("collection {c:?}");
            }
        }
        let region = region.expect("actual sole-consumer ordered F64 reduction");
        let reduction = region.reduction.as_ref().unwrap();
        crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        let cases = vec![
            vec![],
            vec![1u64 << 63, 1u64 << 63],
            vec![1e16f64.to_bits(), 1.0f64.to_bits(), (-1e16f64).to_bits()],
            vec![
                0x7ff8_0000_0000_0037,
                0xfff0_0000_0000_0000,
                0x7ff0_0000_0000_0000,
            ],
            vec![
                1.0f64.to_bits(),
                2.0f64.to_bits(),
                (-3.0f64).to_bits(),
                4.0f64.to_bits(),
            ],
        ];
        for restored in [false, true] {
            for cells in &cases {
                let arguments = args(&artifact, &memory, cells, 1.0f64.to_bits());
                let aliases = arguments.clone();
                let before = contents(&arguments);
                let mut fast = vec![None; artifact.outputs().len()];
                let mut ordinary = fast.clone();
                let (a, counts) = run(
                    &jit,
                    &arguments,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        numeric_only: true,
                        ..Default::default()
                    },
                );
                let (b, _) = run(
                    &jit,
                    &arguments,
                    &mut ordinary,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                a.unwrap();
                b.unwrap();
                assert_eq!(
                    contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                    contents(&ordinary.iter().flatten().cloned().collect::<Vec<_>>()),
                    "exact source fold order and F64 bits {cells:x?}"
                );
                assert_eq!(counts.selected, 1, "real native map reduction: {counts:?}");
                assert_eq!(counts.complete, 1);
                for site in [reduction.sum, reduction.select] {
                    assert_eq!(
                        counts
                            .helpers
                            .get(&(u64::from(site), GlobalHelper::Library.raw()))
                            .copied()
                            .unwrap_or(0),
                        0,
                        "elide both source library calls"
                    );
                }
                assert_eq!(
                    counts
                        .helpers
                        .get(&(
                            u64::from(reduction.finish),
                            GlobalHelper::NumericReductionFinish.raw()
                        ))
                        .copied()
                        .unwrap_or(0),
                    1
                );
                assert_eq!(contents(&arguments), before);
                assert_eq!(contents(&aliases), before);
            }
        }
    }
}

fn output_data(output: &[Option<Value>]) -> Vec<Data> {
    contents(&output.iter().flatten().cloned().collect::<Vec<_>>())
}
fn limit(policy: &mut NativeProgramExecutionLimits, dimension: u8, cap: u64) {
    match dimension {
        0 => policy.library.max_output_elements = Some(cap),
        1 => policy.library.max_output_bytes = Some(cap),
        2 => policy.library.max_temporary_bytes = Some(cap),
        3 => policy.library.max_cloned_bytes = Some(cap),
        4 => policy.library.max_retained_nodes = Some(cap),
        5 => policy.library.max_compute_work = Some(cap),
        6 => policy.library.max_comparison_work = Some(cap),
        7 => policy.execution.max_work = cap,
        _ => policy.execution.max_transport_bytes = cap,
    }
}
#[test]
fn numeric_map_reductions_preserve_finite_logical_limits_atomic_outputs_and_cleanup() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let arguments = args(
            &artifact,
            &memory,
            &[1.0f64.to_bits(), 2.0f64.to_bits()],
            1.0f64.to_bits(),
        );
        let prior_arguments = args(&artifact, &memory, &[9.0f64.to_bits()], 3.0f64.to_bits());
        let mut prior = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &prior_arguments,
            &mut prior,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        drop(prior_arguments);
        let old = output_data(&prior);
        let mut oracle = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &arguments,
            &mut oracle,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        let expected = output_data(&oracle);
        assert_ne!(old, expected, "distinct atomic publication oracle");
        let before = contents(&arguments);
        for restored in [false, true] {
            for dimension in 0..9 {
                for cap in [0, 1, 2, 8, 64, 255, 4095, 65535, 1_000_000, 100_000_000] {
                    let mut policy = mode.execution();
                    limit(&mut policy, dimension, cap);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = prior.clone();
                    let mut cold = prior.clone();
                    let (a, _) = run(
                        &jit,
                        &arguments,
                        &mut fast,
                        &memory,
                        policy,
                        restored,
                        Flags {
                            numeric_only: true,
                            ..Default::default()
                        },
                    );
                    let (b, _) = run(
                        &jit,
                        &arguments,
                        &mut cold,
                        &memory,
                        policy,
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    if dimension < 2 {
                        assert_eq!(
                            a.is_ok(),
                            b.is_ok(),
                            "logical constructor limit {dimension} {cap}: {a:?} / {b:?}"
                        );
                    }
                    assert_eq!(
                        output_data(&fast),
                        if a.is_ok() {
                            expected.clone()
                        } else {
                            old.clone()
                        },
                        "dimension {dimension} cap {cap}: {a:?}"
                    );
                    assert_eq!(
                        output_data(&cold),
                        if b.is_ok() {
                            expected.clone()
                        } else {
                            old.clone()
                        }
                    );
                    drop((fast, cold));
                    assert_eq!(contents(&arguments), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            // Empty producer still has the original one-element sum output.
            let empty = args(&artifact, &memory, &[], 1.0f64.to_bits());
            let mut policy = mode.execution();
            policy.library.max_output_elements = Some(0);
            for flags in [
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
                Flags {
                    numeric_only: true,
                    ..Default::default()
                },
            ] {
                let mut out = prior.clone();
                assert!(
                    run(&jit, &empty, &mut out, &memory, policy, restored, flags)
                        .0
                        .is_err()
                );
                assert_eq!(output_data(&out), old);
            }
            let mut out = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &arguments,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    numeric_only: true,
                    ..Default::default()
                },
            )
            .0
            .unwrap();
            assert_eq!(output_data(&out), expected);
        }
        drop((arguments, prior, oracle));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_map_reductions_keep_nested_conditionals_axes_and_partial_source_errors() {
    for arithmetic in [false, true] {
        let term = if arithmetic {
            "term(value, divisor<u64>)"
        } else {
            "((enabled? | true => data[j,i] | false => data[1u64,i]))"
        };
        let source = if arithmetic {
            format!(
                "+> stats\nterm(x<u64>, divisor<u64>) => (out<f64>) :=\n  selected := ((x == 1u64)? | true => 100u64 / divisor | false => x)\n  number<f64> := selected\n  out := number.\nstats/sum/column([{term} | value <- words<[u64]>])[1u64] + 0.5\n"
            )
        } else {
            format!(
                "+> stats\ndata := words<[f64]:2,3>\nenabled := gate<bool>\nlast := end<u64>\n[stats/sum/column([{term} | i <- 1u64..=last])[1u64] + 0.5 | j <- 1u64..=2u64]\n"
            )
        };
        for artifact in artifacts(&source) {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let reductions: Vec<_> = jit
                .descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .filter(|r| r.reduction.is_some())
                .collect();
            assert_eq!(reductions.len(), 1, "{source}");
            for restored in [false, true] {
                for error in [false, true, false] {
                    let arguments = if arithmetic {
                        typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                (
                                    "words",
                                    Data::Matrix([2, 3, 1, 4].map(Data::U64).into()),
                                    &[1, 4],
                                ),
                                ("divisor", Data::U64(if error { 0 } else { 2 }), &[]),
                            ],
                        )
                    } else {
                        typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                (
                                    "words",
                                    Data::Matrix(
                                        [1., 2., 3., 4., 5., 6.]
                                            .map(|v| Data::F64(F64Bits::from_f64(v)))
                                            .into(),
                                    ),
                                    &[2, 3],
                                ),
                                ("end", Data::U64(if error { 4 } else { 3 }), &[]),
                                ("gate", Data::Bool(!error), &[]),
                            ],
                        )
                    };
                    let before = contents(&arguments);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, counts) = run(
                        &jit,
                        &arguments,
                        &mut fast,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            numeric_only: true,
                            ..Default::default()
                        },
                    );
                    let (b, _) = run(
                        &jit,
                        &arguments,
                        &mut cold,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    assert_eq!(a.is_err(), error, "{source}: {a:?}");
                    assert_eq!(a.is_err(), b.is_err());
                    assert!(counts.selected > 0, "real source loan {counts:?}");
                    if error {
                        assert_eq!(
                            format!("{:?}", a.err()),
                            format!("{:?}", b.err()),
                            "original failing source operation"
                        );
                    }
                    assert_eq!(output_data(&fast), output_data(&cold));
                    for region in &reductions {
                        for site in &region.body {
                            if !error {
                                assert!(
                                    !counts.helpers.keys().any(|(at, _)| *at == u64::from(*site)),
                                    "native inner DAG"
                                );
                            }
                        }
                    }
                    assert_eq!(contents(&arguments), before);
                    drop((fast, cold));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
        }
    }
}

#[test]
fn numeric_map_reductions_refuse_changed_loans_pending_epochs_and_unrelated_helpers() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let arguments = args(
            &artifact,
            &memory,
            &[2.0f64.to_bits(), 3.0f64.to_bits()],
            1.0f64.to_bits(),
        );
        let prior_arguments = args(&artifact, &memory, &[9.0f64.to_bits()], 1.0f64.to_bits());
        let mut prior = vec![None; artifact.outputs().len()];
        run(
            &jit,
            &prior_arguments,
            &mut prior,
            &memory,
            mode.execution(),
            false,
            Flags {
                ordinary: true,
                ..Default::default()
            },
        )
        .0
        .unwrap();
        drop(prior_arguments);
        let old = output_data(&prior);
        for restored in [false, true] {
            for corrupt in [1, 2, 3, 4, 6, 7, 8, 9, 10, 11, 12, 17, 18, 19, 20, 21] {
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut out = prior.clone();
                let (result, counts) = run(
                    &jit,
                    &arguments,
                    &mut out,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert_eq!(counts.selected, 1);
                assert!(result.is_err(), "mutation {corrupt}: {result:?}");
                assert_eq!(output_data(&out), old);
                drop(out);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                let mut retry = vec![None; artifact.outputs().len()];
                let (result, counts) = run(
                    &jit,
                    &arguments,
                    &mut retry,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        interleave: true,
                        ..Default::default()
                    },
                );
                result.unwrap();
                assert_eq!(
                    (counts.selected, counts.complete, counts.interleaved_refused),
                    (1, 1, 5)
                );
                assert_ne!(output_data(&retry), old);
                drop(retry);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
        }
        drop((arguments, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_map_reductions_reconstruct_source_proofs_and_reject_forged_completion() {
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        use serde::Deserialize;
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(
            object.descriptor().metadata().numeric_maps,
            jit.descriptor().metadata().numeric_maps
        );
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let original = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        for change in 0..8 {
            let mut forged = original.clone();
            let region = forged["descriptor"]["numeric_maps"]["regions"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .find(|r| !r["reduction"].is_null())
                .unwrap();
            let reduction = &mut region["reduction"];
            match change {
                0 => reduction["finish"] = 999.into(),
                1 => reduction["finish_path"][0] = 999.into(),
                2 => reduction["continuation"][0] = 999.into(),
                3 => reduction["sum"] = 999.into(),
                4 => reduction["select"] = 999.into(),
                5 => reduction["scalar"] = 999.into(),
                6 => reduction["elided_values"][0] = 999.into(),
                _ => region["reduction"] = serde_json::Value::Null,
            }
            let bytes = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&forged).unwrap(),
            )
            .unwrap();
            // SAFETY: diagnostic reconstruction only; forged metadata is never executed.
            assert!(
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&bytes, &catalog) }
                    .is_err(),
                "forged map reduction {change}"
            );
        }
    }
}

#[test]
fn numeric_map_reductions_keep_escaped_integer_wrong_axis_and_wrong_selector_sources_ordinary() {
    for source in [
        "+> stats\nmapped := [value * 2.0 | value <- values<[f64]>]\n(mapped, stats/sum/column(mapped)[1u64])\n",
        "+> stats\nstats/sum/column([value + 1u64 | value <- values<[u64]>])[1u64]\n",
        "+> stats\nstats/sum/row([value * 2.0 | value <- values<[f64]>])[1u64]\n",
        "+> stats\nstats/sum/column([value * 2.0 | value <- values<[f64]>])[2u64]\n",
        "+> stats\nsum := stats/sum/column([value * 2.0 | value <- values<[f64]>])\n(sum, sum[1u64])\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(
                jit.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .all(|r| r.reduction.is_none()),
                "{source}"
            );
        }
    }
}
