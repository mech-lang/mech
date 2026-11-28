//! Typed matrix transpose shares current loans and preserves every F64 bit.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
fn outputs(xs: &[Option<Value>]) -> Vec<Data> {
    xs.iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
#[test]
fn numeric_transpose_dynamic_and_fixed_axes_keep_nan_bits_empty_shapes_and_current_owners() {
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
        [64, 64],
    ];
    let words = [
        0,
        1u64 << 63,
        0x7ff8_0000_0000_0042,
        0x7ff0_0000_0000_0031,
        0xfff8_0000_0000_0064,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
        1.5f64.to_bits(),
        (-7.0f64).to_bits(),
    ];
    for [rows, columns] in shapes {
        for fixed in [false, true] {
            let source = if fixed {
                format!("input<[f64]:{rows},{columns}>'\n")
            } else {
                "input<[f64]>'\n".into()
            };
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let ordinary = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::NumericMatrixArithmetic,
                )
                .unwrap();
                assert_eq!(native.descriptor().metadata().numeric_binary.at(native.descriptor().metadata().plan.blocks().iter().position(|b| matches!(b.action(),mech_engine::native_artifact::NativeIterationAction::LibraryCall{..})).unwrap()).unwrap().operands.len(),1);
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let data = Data::Matrix(
                    (0..rows * columns)
                        .map(|i| Data::F64(F64Bits::from_bits(words[i as usize % words.len()])))
                        .collect(),
                );
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[("input", data.clone(), &[rows, columns])],
                );
                let mut expected = vec![None; artifact.outputs().len()];
                ordinary
                    .evaluate_into(&args, &mut expected, mode.execution(), &memory)
                    .unwrap();
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = expected.clone();
                    let (result, counts) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut actual,
                            mode.execution(),
                            &memory,
                        )
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
                    assert_eq!(finished, 1, "{rows}x{columns}, fixed {fixed}: {counts:?}");
                    assert_eq!(outputs(&actual), outputs(&expected));
                    assert_eq!(args[0].canonical_data_draft().unwrap(), data);
                    drop(actual);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
                drop((args, expected));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
