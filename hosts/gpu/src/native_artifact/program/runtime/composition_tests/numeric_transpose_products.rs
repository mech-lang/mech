//! Exact source consumer proofs before native view emission is enabled.
use super::*;
use crate::native_artifact::program::prepare::{numeric_binary::Kind, numeric_transpose_products};
use mech_engine::native_artifact::{
    NativeIterationTerminator as Terminator, NativeMatchTerminator as Term,
};

fn prove(source: &str, expected: usize, selected: Option<usize>) {
    for artifact in artifacts(source) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let metadata = jit.descriptor().metadata();
        let plan = &metadata.plan;
        let views = numeric_transpose_products::derive(plan).unwrap();
        println!(
            "transpose/product source proof: expected {expected}, found {}",
            views.len()
        );
        assert_eq!(views.len(), expected, "{source}");
        for view in views {
            let transpose = metadata.numeric_binary.at(view.entry as usize).unwrap();
            let product = metadata.numeric_binary.at(view.product as usize).unwrap();
            assert!(matches!(transpose.kind, Kind::Transpose { .. }));
            assert_eq!(product.kind, Kind::Product);
            assert_eq!(Some(view.transposed_operand), selected);
            assert_eq!(
                view.operands[view.transposed_operand],
                transpose.operands[0]
            );
            assert_eq!(
                view.operands[1 - view.transposed_operand],
                product.operands[1 - view.transposed_operand]
            );
            assert!(view.elided_values.contains(&transpose.output));
            assert!(
                view.elided_values
                    .contains(&product.operands[view.transposed_operand])
            );
            assert!(
                view.operands
                    .iter()
                    .all(|v| !view.elided_values.contains(v))
            );
            assert_eq!(view.continuation.last(), Some(&view.product));
            assert_eq!(
                plan.blocks()[view.product as usize].terminator(),
                Terminator::Scalar(Term::Jump(view.resume))
            );
            assert_eq!(view.output, product.output);
            assert_eq!(view.parent, plan.blocks()[view.entry as usize].iteration());
        }
    }
}

#[test]
fn numeric_transpose_products_prove_original_and_decoded_both_operands_and_aliases() {
    for (source, side) in [
        ("a<[f64]>' ** b<[f64]>\n", 0),
        ("a<[f64]> ** b<[f64]>'\n", 1),
        ("a := input<[f64]>\nt := a'\nu := t\nu ** peer<[f64]>\n", 0),
        ("a<[f64]:3,5>' ** b<[f64]:3,7>\n", 0),
        ("a<[f64]:3,5> ** b<[f64]:7,5>'\n", 1),
    ] {
        prove(source, 1, Some(side));
    }
}

#[test]
fn numeric_transpose_products_decline_escapes_multiple_uses_other_consumers_and_intervening_work() {
    for source in [
        "t := a<[f64]>'\n(t, t ** b<[f64]>)\n",
        "t := a<[f64]>'\n(t ** b<[f64]>, t ** c<[f64]>)\n",
        "t := a<[f64]>'\nt ** t\n",
        "a<[f64]>' + b<[f64]>\n",
        "a<[f64]>'\n",
        "a<[f64]>' ** (b<[f64]> ** c<[f64]>)\n",
        "t := a<[f64]>'\nflag<bool>? | true => t ** b<[f64]> | false => t ** c<[f64]>\n",
    ] {
        prove(source, 0, None);
    }
}

#[test]
fn numeric_transpose_products_prove_optimization_operator_graphs_and_bound_initial_scope() {
    let fixtures =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/polybench");
    for (name, expected) in [
        ("bicg", 1),
        ("mvt", 1),
        ("covariance", 1),
        ("correlation", 1),
        ("atax", 0),
    ] {
        let source = std::fs::read_to_string(fixtures.join(format!("{name}.mec"))).unwrap();
        println!("optimization source proof {name}");
        prove(&source, expected, (expected > 0).then_some(0));
    }
}

fn bits_matrix(words: &[u64], rows: u64, columns: u64) -> Data {
    Data::Matrix(
        (0..rows * columns)
            .map(|at| {
                Data::F64(mech_core::snapshot::F64Bits::from_bits(
                    words[at as usize % words.len()],
                ))
            })
            .collect(),
    )
}
fn output_data(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|value| value.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
#[test]
fn numeric_transpose_products_execute_one_final_bank_keep_order_bits_aliases_and_restored_loans() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    let shapes = [
        [0, 0, 0],
        [0, 3, 5],
        [3, 0, 5],
        [3, 5, 0],
        [1, 1, 1],
        [2, 3, 5],
        [3, 5, 7],
        [5, 7, 3],
    ];
    let words = [
        0,
        1u64 << 63,
        1,
        1.25f64.to_bits(),
        (-2.0f64).to_bits(),
        3.5f64.to_bits(),
    ];
    for side in [0, 1] {
        let source = if side == 0 {
            "a<[f64]>' ** b<[f64]>\n"
        } else {
            "a<[f64]> ** b<[f64]>'\n"
        };
        for artifact in artifacts(source) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let materialized = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::TransposeProductViews,
            )
            .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::NumericMatrixArithmetic,
            )
            .unwrap();
            let proof = numeric_transpose_products::derive(&native.descriptor().metadata().plan)
                .unwrap()
                .remove(0);
            for [rows, inner, columns] in shapes {
                let left = if side == 0 {
                    [inner, rows]
                } else {
                    [rows, inner]
                };
                let right = if side == 1 {
                    [columns, inner]
                } else {
                    [inner, columns]
                };
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("a", bits_matrix(&words, left[0], left[1]), &left),
                        ("b", bits_matrix(&words, right[0], right[1]), &right),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut expected = vec![None; artifact.outputs().len()];
                ordinary
                    .evaluate_into(&args, &mut expected, mode.execution(), &memory)
                    .unwrap();
                let expected_data = output_data(&expected);
                drop(expected);
                for restored in [false, true] {
                    for _ in 0..3 {
                        let mut reference = vec![None; artifact.outputs().len()];
                        let reference_report = materialized
                            .evaluate_into(&args, &mut reference, mode.execution(), &memory)
                            .unwrap();
                        let mut output = vec![None; artifact.outputs().len()];
                        let (result, counts) = if restored {
                            observe::run_restored(
                                &native,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(&native, &args, &mut output, mode.execution(), &memory)
                        };
                        let report = result.unwrap();
                        assert_eq!(
                            output_data(&output),
                            expected_data,
                            "side{side} {rows},{inner},{columns}"
                        );
                        assert_eq!(output_data(&reference), expected_data);
                        assert_eq!(
                            counts.transpose_products,
                            vec![(
                                proof.entry as usize,
                                left[0] * left[1] * (u64::from(side == 0))
                                    + right[0] * right[1] * (u64::from(side == 1)),
                                rows * columns,
                                false
                            )]
                        );
                        assert_eq!(
                            counts
                                .admission_callbacks
                                .get(&u64::from(proof.product))
                                .copied()
                                .unwrap_or(0),
                            0
                        );
                        assert_eq!(
                            report.visits + proof.continuation.len() as u64,
                            reference_report.visits
                        );
                        assert!(
                            report.work >= reference_report.work,
                            "source compute fee decreased"
                        );
                        assert!(
                            report.comparison_work >= reference_report.comparison_work,
                            "source comparison fee decreased"
                        );
                        let mut comparable = reference_report.clone();
                        comparable.work = report.work;
                        comparable.comparison_work = report.comparison_work;
                        comparable.visits = report.visits;
                        assert_eq!(report, comparable);
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        drop((output, reference));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            baseline
                        );
                    }
                }
                drop((args, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
