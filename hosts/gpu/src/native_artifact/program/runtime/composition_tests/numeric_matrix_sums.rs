//! Ordered source reductions, exact bits and current synchronous owners.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;

fn outputs(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn check(
    axis: &str,
    extents: [u64; 2],
    fixed: bool,
    values: &[u64],
    expected: Option<&[u64]>,
    native_expected: bool,
) {
    let [rows, columns] = extents;
    let ty = if fixed {
        format!("[f64]:{rows},{columns}")
    } else {
        "[f64]".into()
    };
    let source = format!("+> stats\nstats/sum/{axis}(input<{ty}>)\n");
    for artifact in artifacts(&source) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericMatrixArithmetic,
        )
        .unwrap();
        let selected: Vec<_> = native
            .descriptor()
            .metadata()
            .plan
            .blocks()
            .iter()
            .enumerate()
            .filter_map(|(site, _)| native.descriptor().metadata().numeric_binary.at(site))
            .collect();
        assert_eq!(selected.len(), 1);
        assert!(matches!(
            selected[0].kind,
            crate::native_artifact::program::prepare::numeric_binary::Kind::Sum { .. }
        ));
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let data = Data::Matrix(
            values
                .iter()
                .map(|&v| Data::F64(F64Bits::from_bits(v)))
                .collect(),
        );
        let args =
            numeric_maps::typed_arguments(&artifact, &memory, &[("input", data.clone(), &extents)]);
        let mut prior = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&args, &mut prior, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            let mut actual = prior.clone();
            let (result, counts) = if restored {
                observe::run_restored(&native, &args, &mut actual, mode.execution(), &memory)
            } else {
                observe::run(&native, &args, &mut actual, mode.execution(), &memory)
            };
            result.unwrap();
            let finished: u64 = counts
                .helpers
                .iter()
                .filter(|((_, op), _)| *op == GlobalHelper::NumericBinaryFinish.raw())
                .map(|(_, n)| n)
                .sum();
            assert_eq!(
                finished,
                u64::from(native_expected),
                "axis{axis} {extents:?} fixed{fixed}: {counts:?}"
            );
            assert_eq!(outputs(&actual), outputs(&prior));
            if let Some(expected) = expected {
                let expected = Data::Matrix(
                    expected
                        .iter()
                        .map(|&v| Data::F64(F64Bits::from_bits(v)))
                        .collect(),
                );
                assert_eq!(outputs(&actual).last(), Some(&expected));
            }
            assert_eq!(args[0].canonical_data_draft().unwrap(), data);
            drop(actual);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        drop((prior, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_matrix_sums_fixed_dynamic_empty_rectangles_and_simd_tails_match_ordinary_bits() {
    let shapes = [
        [0, 0],
        [0, 3],
        [3, 0],
        [1, 1],
        [1, 7],
        [7, 1],
        [2, 3],
        [3, 2],
        [7, 9],
        [4, 17],
        [64, 64],
    ];
    let words = [
        0,
        1u64 << 63,
        1.0f64.to_bits(),
        (-3.0f64).to_bits(),
        1e16f64.to_bits(),
        (-1e16f64).to_bits(),
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        f64::MAX.to_bits(),
        f64::MIN_POSITIVE.to_bits(),
        1,
    ];
    for extents in shapes {
        let values = (0..extents[0] * extents[1])
            .map(|n| words[n as usize % words.len()])
            .collect::<Vec<_>>();
        for axis in ["row", "column"] {
            for fixed in [false, true] {
                check(axis, extents, fixed, &values, None, true);
            }
        }
    }
}
#[test]
fn numeric_matrix_sums_keep_positive_zero_seed_and_strict_cancellation_order() {
    let row = [1e16f64, 1e16, 1., 2., -1e16, -1e16, 4., 8.].map(f64::to_bits);
    let column = [1e16f64, 1., -1e16, 4., 1e16, 2., -1e16, 8.].map(f64::to_bits);
    let expected = [4.0f64.to_bits(), 10.0f64.to_bits()];
    for fixed in [false, true] {
        check("row", [4, 2], fixed, &row, Some(&expected), true);
        check("column", [2, 4], fixed, &column, Some(&expected), true);
        check("row", [3, 1], fixed, &[1u64 << 63; 3], Some(&[0]), true);
        check("column", [1, 3], fixed, &[1u64 << 63; 3], Some(&[0]), true);
    }
}
#[test]
fn numeric_matrix_sums_input_nan_payloads_replay_ordinary_without_publication_changes() {
    let words = [
        0x7ff8_0000_0000_0042,
        1.0f64.to_bits(),
        0x7ff0_0000_0000_0031,
        0xfff8_0000_0000_0064,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
    ];
    for axis in ["row", "column"] {
        for fixed in [false, true] {
            check(axis, [2, 3], fixed, &words, None, false);
        }
    }
}
