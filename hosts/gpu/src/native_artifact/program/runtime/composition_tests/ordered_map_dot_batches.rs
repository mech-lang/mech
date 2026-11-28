//! Independent output lanes, active-domain intersections and exact replay.
use super::ordered_map_dots::{compare, matrix, output, pair};
use super::*;
use mech_core::snapshot::F64Bits;
fn source(op: &str, reverse: bool) -> String {
    let dot = "stats/sum/column([weights[i] * values[i,j] | i <- first<u64>..=last<u64>])[1u64]";
    let alternative = "alternative<f64>";
    let (yes, no) = if reverse {
        (alternative, dot)
    } else {
        (dot, alternative)
    };
    format!(
        "+> stats\nvalues := data<[f64]:3,6>\nweights := coefficients<[f64]:3,1>\n[((j {op} bound<u64>)? | true => {yes} | false => {no}) | j <- outer-first<u64>..outer-last<u64>]\n"
    )
}
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    first: u64,
    last: u64,
    bound: u64,
    outer_first: u64,
    outer_last: u64,
    special: u64,
    alternative: u64,
) -> Vec<Value> {
    let bits = [
        1e16f64.to_bits(),
        0,
        1,
        2f64.to_bits(),
        3f64.to_bits(),
        4f64.to_bits(),
        1f64.to_bits(),
        0,
        2,
        (-3f64).to_bits(),
        4f64.to_bits(),
        5f64.to_bits(),
        (-1e16f64).to_bits(),
        0,
        3,
        4f64.to_bits(),
        5f64.to_bits(),
        special,
    ];
    numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("data", matrix(&bits), &[3, 6]),
            ("coefficients", matrix(&[1f64.to_bits(); 3]), &[3, 1]),
            ("first", Data::U64(first), &[]),
            ("last", Data::U64(last), &[]),
            ("bound", Data::U64(bound), &[]),
            ("outer-first", Data::U64(outer_first), &[]),
            ("outer-last", Data::U64(outer_last), &[]),
            (
                "alternative",
                Data::F64(F64Bits::from_bits(alternative)),
                &[],
            ),
        ],
    )
}
#[test]
fn ordered_map_dot_batches_all_comparisons_polarities_dynamic_domains_odd_lanes_and_payload_replay()
{
    for op in ["==", "!=", "<", "<=", ">", ">="] {
        for reverse in [false, true] {
            for artifact in artifacts(&source(op, reverse)) {
                let (fast, old) = pair(&artifact);
                assert!(fast.descriptor().metadata().numeric_maps.regions.iter().any(|r|crate::native_artifact::program::prepare::numeric_maps::dots::Batch::derive(&fast.descriptor().metadata().plan,r).is_some()),"{op} reversed={reverse}");
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                for restored in [false, true] {
                    let mut outputs = vec![None; artifact.outputs().len()];
                    let mut previous = outputs.clone();
                    for bound in [0, 1, 2, 3, 4, 6, 7, u64::MAX] {
                        for (outer_first, outer_last) in [(1, 7), (2, 7), (3, 7), (6, 7), (1, 1)] {
                            let values = args(
                                &artifact,
                                &memory,
                                1,
                                3,
                                bound,
                                outer_first,
                                outer_last,
                                6f64.to_bits(),
                                0x8000_0000_0000_0000,
                            );
                            compare(
                                &fast,
                                &old,
                                &values,
                                &memory,
                                mode.execution(),
                                restored,
                                &mut outputs,
                                &mut previous,
                            );
                            outputs.fill(None);
                            previous.fill(None);
                        }
                    }
                    for special in [
                        0x7ff8_0000_0000_0037,
                        0xfff0_0000_0000_0041,
                        0x7ff0_0000_0000_0000,
                        0x8000_0000_0000_0001,
                    ] {
                        let values = args(
                            &artifact,
                            &memory,
                            1,
                            3,
                            3,
                            1,
                            7,
                            special,
                            0x7ff8_0000_0000_0079,
                        );
                        compare(
                            &fast,
                            &old,
                            &values,
                            &memory,
                            mode.execution(),
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                        outputs.fill(None);
                        previous.fill(None);
                    }
                }
                assert_eq!(memory.used_bytes(), 0);
            }
        }
    }
}
#[test]
fn ordered_map_dot_batches_inactive_bad_child_ranges_partial_prefix_limits_forged_loan_and_retry() {
    for artifact in artifacts(&source(">", false)) {
        let (fast, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut outputs = vec![None; artifact.outputs().len()];
            let mut previous = outputs.clone();
            for (first, last, bound, outer_first, outer_last) in [
                (1, 3, 0, 1, 7),
                (9, 2, 6, 1, 7),
                (0, u64::MAX, 6, 1, 7),
                (1, 3, 2, 1, 7),
                (0, 3, 0, 1, 7),
                (1, 4, 0, 1, 7),
                (1, 3, 0, 1, 9),
                (1, 3, 0, 3, 1),
                (1, 3, 0, 1, 7),
            ] {
                let values = args(
                    &artifact,
                    &memory,
                    first,
                    last,
                    bound,
                    outer_first,
                    outer_last,
                    6f64.to_bits(),
                    0,
                );
                let (ok, _) = compare(
                    &fast,
                    &old,
                    &values,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                if bound == 6 {
                    assert!(ok, "inactive invalid range must stay unevaluated");
                }
                for dimension in 0..5 {
                    for cap in [0, 1, 65536, 1 << 36] {
                        let mut policy = mode.execution();
                        match dimension {
                            0 => policy.library.max_output_elements = Some(cap),
                            1 => policy.library.max_temporary_bytes = Some(cap),
                            2 => policy.library.max_compute_work = Some(cap),
                            3 => policy.execution.max_work = cap,
                            _ => policy.execution.max_transport_bytes = cap,
                        };
                        compare(
                            &fast,
                            &old,
                            &values,
                            &memory,
                            policy,
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                    }
                }
                compare(
                    &fast,
                    &old,
                    &values,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                outputs.fill(None);
                previous.fill(None);
            }
            let values = args(&artifact, &memory, 1, 3, 0, 1, 7, 6f64.to_bits(), 0);
            let mut out = vec![None; artifact.outputs().len()];
            numeric_maps::run(
                &fast,
                &values,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                numeric_maps::Flags::default(),
            )
            .0
            .unwrap();
            let before = output(&out);
            for corrupt in [1, 4, 14, 15, 16] {
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let (result, counts) = numeric_maps::run(
                    &fast,
                    &values,
                    &mut out,
                    &memory,
                    mode.execution(),
                    restored,
                    numeric_maps::Flags {
                        corrupt,
                        ..Default::default()
                    },
                );
                assert!(result.is_err());
                assert!(counts.selected > 0);
                assert_eq!(output(&out), before);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
            numeric_maps::run(
                &fast,
                &values,
                &mut out,
                &memory,
                mode.execution(),
                restored,
                numeric_maps::Flags::default(),
            )
            .0
            .unwrap();
            assert_eq!(output(&out), before);
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn ordered_map_dot_batches_strided_outer_outputs_linear_column_wrap_and_scalar_fallback() {
    for selection in ["values[i,7u64 - j]", "values[(j - 1u64) * 3u64 + i]"] {
        let source = source(">", false).replace("values[i,j]", selection);
        for artifact in artifacts(&source) {
            let (fast, old) = pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            assert!(
                fast.descriptor()
                    .metadata()
                    .numeric_maps
                    .regions
                    .iter()
                    .any(|r| {
                        crate::native_artifact::program::prepare::numeric_maps::dots::Batch::derive(
                            &fast.descriptor().metadata().plan,
                            r,
                        )
                        .is_some()
                    })
            );
            for restored in [false, true] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let mut previous = outputs.clone();
                for bound in [0, 2, 3, 5, 6] {
                    let values = args(&artifact, &memory, 1, 3, bound, 1, 7, 6f64.to_bits(), 0);
                    assert!(
                        compare(
                            &fast,
                            &old,
                            &values,
                            &memory,
                            mode.execution(),
                            restored,
                            &mut outputs,
                            &mut previous
                        )
                        .0
                    );
                    outputs.fill(None);
                    previous.fill(None);
                }
            }
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
