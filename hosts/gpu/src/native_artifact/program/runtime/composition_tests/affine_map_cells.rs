//! Public execution comparisons against the complete checked DAG, retaining
//! exact reports, source aliases, errors, nonfinite payloads and atomic output.
use super::ordered_map_dots::{compare, matrix, output};
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
fn pair(a: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    let fast = JitProgramArtifact::compile(a, &catalog).unwrap();
    let old = JitProgramArtifact::compile_for_test(a, &catalog, OptimizationPass::AffineMapCells)
        .unwrap();
    assert!(
        fast.descriptor()
            .metadata()
            .numeric_maps
            .regions
            .iter()
            .any(|r| {
                crate::native_artifact::program::prepare::numeric_maps::dots::CellProgram::derive(
                    &fast.descriptor().metadata().plan,
                    r,
                )
                .is_some()
            })
    );
    crate::native_artifact::emit_program_object(a, &catalog).unwrap();
    (fast, old)
}
fn f(v: f64) -> Data {
    Data::F64(F64Bits::from_f64(v))
}
#[test]
fn affine_map_cells_affine_reverse_and_linear_geometry_exact_words_errors_cleanup_retry() {
    let source = "a := left<[f64]>\nb := right<[f64]>\n[a[i * scale<u64> + shift<u64>] - gain<f64> * b[bound<u64> - i] | i <- first<u64>..last<u64>]\n";
    for artifact in artifacts(source) {
        let (fast, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for shape in [[8, 1], [1, 8], [2, 4]] {
            for (first, last, scale, shift, bound) in [
                (1, 7, 1, 0, 8),
                (1, 8, 1, 0, 9),
                (2, 7, 1, 0, 8),
                (1, 4, 2, 0, 8),
                (0, 3, 1, 0, 8),
                (1, 4, 1, 0, 1),
                (1, 4, u64::MAX, 2, 8),
                (4, 4, 1, 0, 0),
                (u64::MAX, u64::MAX, 1, 0, 0),
            ] {
                let bits = [
                    0.0f64.to_bits(),
                    (-0.0f64).to_bits(),
                    1,
                    0x0010_0000_0000_0000,
                    1.25f64.to_bits(),
                    (-2.0f64).to_bits(),
                    2.5f64.to_bits(),
                    3f64.to_bits(),
                ];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", matrix(&bits), &shape),
                        ("right", matrix(&bits), &shape),
                        ("gain", f(-0.75), &[]),
                        ("first", Data::U64(first), &[]),
                        ("last", Data::U64(last), &[]),
                        ("scale", Data::U64(scale), &[]),
                        ("shift", Data::U64(shift), &[]),
                        ("bound", Data::U64(bound), &[]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    compare(
                        &fast,
                        &old,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut actual,
                        &mut expected,
                    );
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
                assert_eq!(contents(&aliases), before);
                drop((args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
#[test]
fn affine_map_cells_piecewise_reverse_prefix_endpoint_and_nonfinite_partial_replay() {
    let source = "a := data<[f64]:7,1>\n[((i < cut<u64>)? | true => a[i] + gain<f64> * a[cut - i] | false => ((i == cut)? | true => gain | false => a[i])) | i <- 1u64..=7u64]'\n";
    for artifact in artifacts(source) {
        let (fast, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for cut in [0, 1, 2, 4, 7, 8, 9, u64::MAX] {
            for unusual in [
                1.5f64.to_bits(),
                f64::MAX.to_bits(),
                f64::INFINITY.to_bits(),
                0x7ff0_0000_0000_0123,
                0xfff8_1234_5678_9abc,
            ] {
                let words = [
                    1.0f64.to_bits(),
                    (-0.0f64).to_bits(),
                    1,
                    2.0f64.to_bits(),
                    3.0f64.to_bits(),
                    unusual,
                    4.0f64.to_bits(),
                ];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("data", matrix(&words), &[7, 1]),
                        ("gain", f(2.0), &[]),
                        ("cut", Data::U64(cut), &[]),
                    ],
                );
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    compare(
                        &fast,
                        &old,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut actual,
                        &mut expected,
                    );
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                }
                drop(args);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
#[test]
fn affine_map_cells_rectangular_relations_broadcast_streams_and_atomic_resource_denials() {
    for relation in ["==", "!=", "<", "<=", ">", ">="] {
        let source = format!(
            "a := left<[f64]:3,5>\nx := rows<[f64]:3,1>\ny := columns<[f64]:1,5>\n[[((j {relation} cut<u64>)? | true => a[i,j] - x[i] * y[j] | false => a[i,j]) | i <- 1u64..=3u64]' | j <- 1u64..=5u64]\n"
        );
        for artifact in artifacts(&source) {
            let (fast, old) = pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            for cut in [0, 1, 3, 5, 6, u64::MAX] {
                let words = (0..15)
                    .map(|n| (n as f64 * 0.25 - 1.0).to_bits())
                    .collect::<Vec<_>>();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", matrix(&words), &[3, 5]),
                        (
                            "rows",
                            matrix(&[1.5f64.to_bits(), (-0.0f64).to_bits(), (-2.5f64).to_bits()]),
                            &[3, 1],
                        ),
                        (
                            "columns",
                            matrix(&[
                                1.0f64.to_bits(),
                                2.0f64.to_bits(),
                                3.0f64.to_bits(),
                                4.0f64.to_bits(),
                                5.0f64.to_bits(),
                            ]),
                            &[1, 5],
                        ),
                        ("cut", Data::U64(cut), &[]),
                    ],
                );
                let mut previous = vec![None; artifact.outputs().len()];
                fast.evaluate_into(&args, &mut previous, mode.execution(), &memory)
                    .unwrap();
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    for cap in 0..8 {
                        let mut limits = mode.execution();
                        match cap {
                            0 => {}
                            1 => limits.execution.max_work = 1,
                            2 => limits.library.max_output_elements = Some(0),
                            3 => limits.library.max_temporary_bytes = Some(0),
                            4 => limits.library.max_cloned_bytes = Some(0),
                            5 => limits.library.max_compute_work = Some(0),
                            6 => limits.library.max_comparison_work = Some(0),
                            _ => limits.library.max_retained_nodes = Some(0),
                        }
                        let mut actual = previous.clone();
                        let mut expected = previous.clone();
                        compare(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            limits,
                            restored,
                            &mut actual,
                            &mut expected,
                        );
                        assert_eq!(output(&actual), output(&previous));
                        drop((actual, expected));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                }
                drop((args, previous));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn affine_map_cells_result_boundary_all_operators_distinct_nan_payloads_and_restored_replay() {
    for expression in [
        "-math/sqrt(1.0 / (a[i] * a[i])) + b[i]",
        "(a[i] + b[i]) * (a[i] - b[i]) / (a[i] + 1.0)",
    ] {
        let source = format!(
            "+> math\na := left<[f64]:7,1>\nb := right<[f64]:7,1>\n[{expression} | i <- 1u64..=7u64]'\n"
        );
        for artifact in artifacts(&source) {
            let (fast, old) = pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            for rotation in 0..7 {
                let words = [
                    f64::MAX.to_bits(),
                    f64::INFINITY.to_bits(),
                    (-0.0f64).to_bits(),
                    1,
                    0x7ff0_0000_0000_0123,
                    0x7ff8_9876_5432_1234,
                    0xfff8_1111_2222_3333,
                ];
                let left = (0..7)
                    .map(|i| words[(i + rotation) % 7])
                    .collect::<Vec<_>>();
                let right = (0..7)
                    .map(|i| words[(i + rotation + 3) % 7])
                    .collect::<Vec<_>>();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", matrix(&left), &[7, 1]),
                        ("right", matrix(&right), &[7, 1]),
                    ],
                );
                let before = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    compare(
                        &fast,
                        &old,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut actual,
                        &mut expected,
                    );
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        before
                    );
                }
                drop(args);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
