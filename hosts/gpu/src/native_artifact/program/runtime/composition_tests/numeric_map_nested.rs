//! Ordered child folds run in one admitted parent map without per-cell helpers.
use super::numeric_maps::{Flags, run, typed_arguments};
use super::*;
use mech_core::snapshot::F64Bits;

const SOURCE: &str = "+> stats\nvalues := data<[f64]:3,4>\nweights := coefficients<[f64]:3,1>\n[stats/sum/column([values[i,j] * weights[i] | i <- 1u64..=3u64])[1u64] | j <- 1u64..=4u64]\n";
fn output_data(output: &[Option<Value>]) -> Vec<Data> {
    contents(&output.iter().flatten().cloned().collect::<Vec<_>>())
}
fn matrix(xs: &[f64]) -> Data {
    Data::Matrix(
        xs.iter()
            .map(|&x| Data::F64(F64Bits::from_f64(x)))
            .collect(),
    )
}
#[test]
fn numeric_nested_map_reductions_share_one_native_loan_keep_order_current_values_and_restore() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .find(|r| !r.nested.is_empty())
            .expect("parent scalar map must really contain a proved child fold");
        assert_eq!(region.nested.len(), 1);
        assert!(
            jit.descriptor()
                .metadata()
                .nested_numeric
                .at(region)
                .is_some()
        );
        crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
            .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for seed in [1.0, -2.0, 1.0] {
                let data = [
                    1e16,
                    -0.0,
                    seed,
                    2.5,
                    1.0,
                    -0.0,
                    2.0 * seed,
                    -3.5,
                    -1e16,
                    -0.0,
                    3.0 * seed,
                    4.5,
                ];
                let args = typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("data", matrix(&data), &[3, 4]),
                        ("coefficients", matrix(&[1.0, 1.0, 1.0]), &[3, 1]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut fast = vec![None; artifact.outputs().len()];
                let mut ordinary = fast.clone();
                let (a, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                let (b, old) = run(
                    &jit,
                    &args,
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
                assert_eq!(counts.selected, 1, "one parent source loan: {counts:?}");
                assert_eq!(
                    counts.complete, 1,
                    "one complete native outer map: {counts:?}"
                );
                assert_eq!(old.complete, 0);
                assert_eq!(output_data(&fast), output_data(&ordinary));
                for site in region.nested[0].source.source_sites() {
                    assert!(
                        !counts.helpers.keys().any(|(at, _)| *at == u64::from(site)),
                        "nested helper at {site}: {counts:?}"
                    );
                }
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                drop((fast, ordinary));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                drop((args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

const DYNAMIC: &str = "+> stats\nvalues := data<[f64]:3,4>\nfirst := first<u64>\nlast := last<u64>\ndivisor := divisor<u64>\nskip := skip<u64>\n[((j > skip)? | true => stats/sum/column([values[i / divisor,j] | i <- first..=last])[1u64] | false => 0.0) | j <- 1u64..=4u64]\n";
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    first: u64,
    last: u64,
    divisor: u64,
    skip: u64,
) -> Vec<Value> {
    typed_arguments(
        artifact,
        memory,
        &[
            (
                "data",
                matrix(&[
                    1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
                ]),
                &[3, 4],
            ),
            ("first", Data::U64(first), &[]),
            ("last", Data::U64(last), &[]),
            ("divisor", Data::U64(divisor), &[]),
            ("skip", Data::U64(skip), &[]),
        ],
    )
}
#[test]
fn numeric_nested_map_reductions_keep_inactive_invalid_ranges_and_exact_error_replay() {
    for artifact in artifacts(DYNAMIC) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .any(|r| !r.nested.is_empty())
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut old = fast.clone();
            for (first, last, divisor, skip) in [
                (1, 3, 1, 0),
                (9, 2, 0, 4),
                (0, u64::MAX, 0, 4),
                (1, 3, 1, 2),
            ] {
                let args = arguments(&artifact, &memory, first, last, divisor, skip);
                let (a, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                let (b, _) = run(
                    &jit,
                    &args,
                    &mut old,
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
                assert_eq!(output_data(&fast), output_data(&old));
                assert_eq!(
                    counts.complete, 1,
                    "inactive source operations must stay inactive: {counts:?}"
                );
            }
            let prior = output_data(&fast);
            for (first, last, divisor) in [(9, 2, 0), (0, u64::MAX, 0), (1, 3, 0), (1, 4, 1)] {
                let args = arguments(&artifact, &memory, first, last, divisor, 0);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (a, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                let (b, _) = run(
                    &jit,
                    &args,
                    &mut old,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                assert_eq!(
                    format!("{:?}", a.unwrap_err()),
                    format!("{:?}", b.unwrap_err()),
                    "{first}..={last}/{divisor}"
                );
                assert!(
                    counts.selected > 0,
                    "native attempt must actually precede source replay"
                );
                assert_eq!(output_data(&fast), prior);
                assert_eq!(output_data(&old), prior);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            let args = arguments(&artifact, &memory, 1, 3, 1, 0);
            let (result, counts) = run(
                &jit,
                &args,
                &mut fast,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            );
            result.unwrap();
            assert_eq!(counts.complete, 1);
            drop((args, fast, old));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_nested_map_reductions_preserve_finite_fallback_denials_and_output_atomicity() {
    for artifact in artifacts(DYNAMIC) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, 1, 3, 1, 0);
        for restored in [false, true] {
            for dimension in 0..8 {
                for cap in [1, 64, 4096, 65536, 1 << 20, 1 << 36] {
                    let mut fast = vec![None; artifact.outputs().len()];
                    run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags::default(),
                    )
                    .0
                    .unwrap();
                    let mut old = vec![None; artifact.outputs().len()];
                    run(
                        &jit,
                        &args,
                        &mut old,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    )
                    .0
                    .unwrap();
                    let before = output_data(&fast);
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut limits = mode.execution();
                    match dimension {
                        0 => limits.library.max_output_elements = Some(cap),
                        1 => limits.library.max_output_bytes = Some(cap),
                        2 => limits.library.max_temporary_bytes = Some(cap),
                        3 => limits.library.max_cloned_bytes = Some(cap),
                        4 => limits.library.max_retained_nodes = Some(cap),
                        5 => limits.library.max_compute_work = Some(cap),
                        6 => limits.library.max_comparison_work = Some(cap),
                        _ => limits.execution.max_work = cap,
                    }
                    let (a, _) = run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        limits,
                        restored,
                        Flags::default(),
                    );
                    let (b, _) = run(
                        &jit,
                        &args,
                        &mut old,
                        &memory,
                        limits,
                        restored,
                        Flags {
                            ordinary: true,
                            ..Default::default()
                        },
                    );
                    assert_eq!(
                        a.is_ok(),
                        b.is_ok(),
                        "dimension{dimension} cap{cap}: {a:?} {b:?}"
                    );
                    assert_eq!(output_data(&fast), output_data(&old));
                    if a.is_err() {
                        assert_eq!(output_data(&fast), before);
                    }
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    run(
                        &jit,
                        &args,
                        &mut fast,
                        &memory,
                        mode.execution(),
                        restored,
                        Flags::default(),
                    )
                    .0
                    .unwrap();
                    assert_eq!(output_data(&fast), before);
                }
            }
        }
        drop(args);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_nested_map_reductions_images_rederive_sources_views_bounds_and_funds() {
    for artifact in artifacts(SOURCE) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(
            jit.descriptor().metadata().numeric_maps,
            object.descriptor().metadata().numeric_maps
        );
        use serde::Deserialize;
        let mut reader = mech_engine::native_artifact::BuildOwnedImageReader::from_image(
            object.native_metadata(),
        )
        .unwrap();
        let original = serde_json::Value::deserialize(&mut reader).unwrap();
        reader.end().unwrap();
        let at = original["descriptor"]["numeric_maps"]["regions"]
            .as_array()
            .unwrap()
            .iter()
            .position(|r| r["nested"].as_array().is_some_and(|xs| !xs.is_empty()))
            .unwrap();
        for alteration in 0..6 {
            let mut changed = original.clone();
            let child = &mut changed["descriptor"]["numeric_maps"]["regions"][at]["nested"][0];
            match alteration {
                0 => child["source"]["maximum"] = serde_json::json!(u64::MAX),
                1 => child["source"]["after"] = serde_json::json!(u32::MAX),
                2 => child["matrices"][0] = serde_json::json!(16),
                3 => {
                    child["source"]["region"]["sites"]
                        .as_array_mut()
                        .unwrap()
                        .pop();
                }
                4 => {
                    changed["descriptor"]["numeric_maps"]["regions"][at]
                        .as_object_mut()
                        .unwrap()
                        .remove("nested");
                }
                _ => {
                    changed["descriptor"]["nested_numeric"] =
                        serde_json::json!({"compute":0,"replay":0})
                }
            }
            let bytes = mech_engine::native_artifact::encode_native_metadata_image(
                &serde_json::to_vec(&changed).unwrap(),
            )
            .unwrap();
            // SAFETY: only this build-owned metadata is mutated; rejection is
            // the entire observation and no restored entry is executed.
            let decoded = unsafe {
                crate::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                    &bytes, &catalog,
                )
            };
            assert!(
                decoded.is_err(),
                "forged nested proof {alteration} was accepted"
            );
        }
    }
}

#[test]
fn numeric_nested_map_reductions_share_one_rectangular_loan_without_intermediate_rows() {
    let source = "+> stats\nvalues := data<[f64]:3,4>\nweights := coefficients<[f64]:3,2>\n[[stats/sum/column([values[k,j] * weights[k,i] | k <- 1u64..=3u64])[1u64] | i <- 1u64..=2u64]' | j <- 1u64..=4u64]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .find(|r| r.rectangle.is_some() && !r.nested.is_empty())
            .expect("complete nested native rectangle");
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = typed_arguments(
                &artifact,
                &memory,
                &[
                    (
                        "data",
                        matrix(&[
                            1e16, -0.0, 2.0, 2.5, 1.0, -0.0, 4.0, -3.5, -1e16, -0.0, 6.0, 4.5,
                        ]),
                        &[3, 4],
                    ),
                    (
                        "coefficients",
                        matrix(&[1.0, 2.0, 1.0, 2.0, 1.0, 2.0]),
                        &[3, 2],
                    ),
                ],
            );
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut fast = vec![None; artifact.outputs().len()];
            let mut ordinary = fast.clone();
            let (a, counts) = run(
                &jit,
                &args,
                &mut fast,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            );
            let (b, _) = run(
                &jit,
                &args,
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
            assert_eq!(output_data(&fast), output_data(&ordinary));
            assert_eq!(
                counts.selected, 1,
                "one complete rectangle loan: {counts:?}"
            );
            assert_eq!(counts.complete, 1);
            assert_eq!(counts.direct_rectangles, 1);
            assert_eq!(counts.block_permutations, 0);
            for child in &region.nested {
                for site in child.source.source_sites() {
                    assert!(
                        !counts.helpers.keys().any(|(at, _)| *at == u64::from(site)),
                        "nested rectangular helper at{site}"
                    );
                }
            }
            drop((fast, ordinary));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            drop(args);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_nested_map_reductions_discard_partial_outer_prefix_before_exact_source_replay() {
    let source = "+> stats\nvalues := data<[f64]:3,4>\nshift := offset<u64>\n[stats/sum/column([values[i,j + shift] | i <- 1u64..=3u64])[1u64] | j <- 1u64..=4u64]\n";
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let region = jit
            .descriptor()
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .find(|r| !r.nested.is_empty())
            .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args_for = |shift| {
                typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "data",
                            matrix(&[
                                1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0, 10.0, 11.0, 12.0,
                            ]),
                            &[3, 4],
                        ),
                        ("offset", Data::U64(shift), &[]),
                    ],
                )
            };
            let good = args_for(0);
            let mut fast = vec![None; artifact.outputs().len()];
            let mut old = fast.clone();
            run(
                &jit,
                &good,
                &mut fast,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            )
            .0
            .unwrap();
            run(
                &jit,
                &good,
                &mut old,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    ordinary: true,
                    ..Default::default()
                },
            )
            .0
            .unwrap();
            let before = output_data(&fast);
            for shift in [2, u64::MAX, 2] {
                let args = args_for(shift);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (a, counts) = run(
                    &jit,
                    &args,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags::default(),
                );
                let (b, _) = run(
                    &jit,
                    &args,
                    &mut old,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        ordinary: true,
                        ..Default::default()
                    },
                );
                assert_eq!(
                    format!("{:?}", a.unwrap_err()),
                    format!("{:?}", b.unwrap_err())
                );
                assert!(counts.selected > 0);
                assert!(
                    counts
                        .helpers
                        .get(&(
                            u64::from(region.header),
                            GlobalHelper::NumericMapFinish.raw()
                        ))
                        .copied()
                        .unwrap_or(0)
                        > 0
                );
                assert_eq!(output_data(&fast), before);
                assert_eq!(output_data(&old), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            let (result, counts) = run(
                &jit,
                &good,
                &mut fast,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            );
            result.unwrap();
            assert_eq!(counts.complete, 1);
            assert_eq!(output_data(&fast), before);
            drop((good, fast, old));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_nested_map_reductions_reject_corrupt_parent_loans_and_interleaved_helpers() {
    for artifact in artifacts(SOURCE) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = typed_arguments(
                &artifact,
                &memory,
                &[
                    ("data", matrix(&[1.0; 12]), &[3, 4]),
                    ("coefficients", matrix(&[1.0; 3]), &[3, 1]),
                ],
            );
            let mut out = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &args,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                Flags::default(),
            )
            .0
            .unwrap();
            let before = output_data(&out);
            for corrupt in [1, 4, 14, 15, 16] {
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = run(
                    &jit,
                    &args,
                    &mut out,
                    &memory,
                    mode.execution(),
                    restored,
                    Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert!(result.is_err(), "corrupt nested parent frame {corrupt}");
                assert!(counts.selected > 0);
                assert_eq!(output_data(&out), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            let (result, counts) = run(
                &jit,
                &args,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                Flags {
                    interleave: true,
                    ..Default::default()
                },
            );
            result.unwrap();
            assert_eq!(counts.complete, 1);
            assert!(counts.interleaved_refused >= 2);
            assert_eq!(output_data(&out), before);
            drop((args, out));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
