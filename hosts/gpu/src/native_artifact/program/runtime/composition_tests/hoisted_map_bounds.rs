//! Current map bounds, checked original fallback and empty sources.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn exercise(kind: &str, conditional: bool) {
    let mode = NativeProgramLimitMode::NoLimits;
    let catalog = mech_stdlib::source_catalog();
    for rows in [1u64, 3, 5] {
        let read = if conditional {
            "((j == 1u64)? | true => data[i,j] | false => data[i,pick<u64>])"
        } else {
            "data[i,j]"
        };
        let inner = if rows == 3 {
            format!("1u64..{}u64", rows + 1)
        } else {
            format!("1u64..={rows}u64")
        };
        let source = format!(
            "data := input<[{kind}]:{rows},5>\n[[{read} + gain<{kind}> | i <- {inner}]' | j <- 1u64..=count<u64>]\n"
        );
        for artifact in artifacts(&source) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let flat = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::VersionedMapBounds,
            )
            .unwrap();
            let memory = mode.memory_budget();
            let cells = (0..rows * 5)
                .map(|i| match kind {
                    "f64" => Data::F64(mech_core::snapshot::F64Bits::from_f64(i as f64 / 2.0)),
                    "u64" => Data::U64(i),
                    _ => unreachable!(),
                })
                .collect::<Vec<_>>();
            let gain = match kind {
                "f64" => Data::F64(mech_core::snapshot::F64Bits::from_f64(0.25)),
                "u64" => Data::U64(1),
                _ => unreachable!(),
            };
            for restored in [false, true] {
                let mut selected = 0;
                for pick in if conditional {
                    vec![0u64, 1, 5, 6, u64::MAX]
                } else {
                    vec![1]
                } {
                    for columns in [0u64, 1, 2, 5, 6] {
                        let extents = [rows, 5];
                        let mut inputs: Vec<(&str, Data, &[u64])> = vec![
                            ("input", Data::Matrix(cells.clone().into()), &extents),
                            ("gain", gain.clone(), &[]),
                            ("count", Data::U64(columns), &[]),
                        ];
                        if conditional {
                            inputs.push(("pick", Data::U64(pick), &[]));
                        }
                        let args = numeric_maps::typed_arguments(&artifact, &memory, &inputs);
                        let before = contents(&args);
                        let baseline = (memory.used_bytes(), memory.retained_nodes());
                        let mut a = vec![None; artifact.outputs().len()];
                        let mut b = a.clone();
                        let (new, counts) = numeric_maps::run(
                            &native,
                            &args,
                            &mut a,
                            &memory,
                            mode.execution(),
                            restored,
                            numeric_maps::Flags {
                                numeric_only: true,
                                ..Default::default()
                            },
                        );
                        let (old, _) = numeric_maps::run(
                            &flat,
                            &args,
                            &mut b,
                            &memory,
                            mode.execution(),
                            restored,
                            numeric_maps::Flags {
                                numeric_only: true,
                                ..Default::default()
                            },
                        );
                        selected += counts.direct_rectangles;
                        assert_eq!(
                            new.is_ok(),
                            old.is_ok(),
                            "{source}: {columns}: {new:?} / {old:?}"
                        );
                        assert_eq!(
                            contents(&a.iter().flatten().cloned().collect::<Vec<_>>()),
                            contents(&b.iter().flatten().cloned().collect::<Vec<_>>())
                        );
                        if let (Err(new), Err(old)) = (&new, &old) {
                            assert_eq!(new.to_string(), old.to_string());
                        }
                        if let (Ok(new), Ok(old)) = (new, old) {
                            {
                                assert_eq!(
                                    (
                                        new.work,
                                        new.comparison_work,
                                        new.visits,
                                        new.transport_bytes
                                    ),
                                    (
                                        old.work,
                                        old.comparison_work,
                                        old.visits,
                                        old.transport_bytes
                                    )
                                );
                            }
                            if !conditional {
                                assert!(columns <= 5);
                            }
                            let expected = (0..rows)
                                .flat_map(|i| {
                                    (0..columns).map(move |j| {
                                        let column =
                                            if conditional && j != 0 { pick } else { j + 1 };
                                        assert!((1..=5).contains(&column));
                                        match kind {
                                            "f64" => {
                                                Data::F64(mech_core::snapshot::F64Bits::from_f64(
                                                    (i * 5 + column - 1) as f64 / 2.0 + 0.25,
                                                ))
                                            }
                                            "u64" => Data::U64(i * 5 + column),
                                            _ => unreachable!(),
                                        }
                                    })
                                })
                                .collect::<Vec<_>>();
                            assert_eq!(
                                a.last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                Data::Matrix(expected.into())
                            );
                        } else {
                            assert!(a.iter().all(Option::is_none));
                        }
                        assert_eq!(contents(&args), before);
                        drop((a, b));
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                        drop(args);
                        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
                    }
                }
                if rows > 1 {
                    assert!(
                        selected > 0,
                        "exercise the actual admitted rectangle: {source}"
                    );
                }
            }
        }
    }
}
#[test]
fn numeric_map_hoisted_bounds_preserves_f64_axis_order_empty_ranges_errors_and_source_bills() {
    exercise("f64", false);
}
#[test]
fn numeric_map_hoisted_bounds_preserves_u64_axis_order_empty_ranges_errors_and_source_bills() {
    exercise("u64", false);
}

#[test]
fn numeric_map_hoisted_bounds_replay_unused_captures_and_preserve_source_error_order() {
    exercise("f64", true);
    exercise("u64", true);
}
