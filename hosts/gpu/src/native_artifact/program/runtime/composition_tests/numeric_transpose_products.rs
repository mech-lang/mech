//! Source consumer proofs, ordered native views, source fees and atomic loans.
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
        let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
        assert_eq!(
            object.metadata()["compiled_transpose_product_views"]
                .as_array()
                .unwrap()
                .len(),
            expected
        );
        let ablated = crate::native_artifact::program::compiler::emit_program_object_for_test(
            &artifact,
            &catalog,
            crate::native_artifact::codegen::options::OptimizationPass::TransposeProductViews,
        )
        .unwrap();
        assert!(
            ablated.metadata()["compiled_transpose_product_views"]
                .as_array()
                .unwrap()
                .is_empty()
        );
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
                        let (reference_result, reference_counts) = observe::run(
                            &materialized,
                            &args,
                            &mut reference,
                            mode.execution(),
                            &memory,
                        );
                        let reference_report = reference_result.unwrap();
                        assert!(
                            reference_counts
                                .admission_callbacks
                                .get(&u64::from(proof.product))
                                .copied()
                                .unwrap_or(0)
                                > 0
                        );
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

#[test]
fn numeric_transpose_products_keep_nonfinite_payload_fallback_fixed_shapes_and_shape_errors_atomic()
{
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for side in [0, 1] {
        let left = if side == 0 { [3, 2] } else { [2, 3] };
        let right = if side == 1 { [5, 3] } else { [3, 5] };
        for fixed in [false, true] {
            let a = if fixed {
                format!("a<[f64]:{},{}>", left[0], left[1])
            } else {
                "a<[f64]>".into()
            };
            let b = if fixed {
                format!("b<[f64]:{},{}>", right[0], right[1])
            } else {
                "b<[f64]>".into()
            };
            let source = if side == 0 {
                format!("{a}' ** {b}\n")
            } else {
                format!("{a} ** {b}'\n")
            };
            for artifact in artifacts(&source) {
                let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let ordinary = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::NumericMatrixArithmetic,
                )
                .unwrap();
                for input_nan in [false, true] {
                    let words = if input_nan {
                        vec![
                            0x7ff8_0000_0000_0042,
                            0x7ff0_0000_0000_0031,
                            0xfff8_0000_0000_0064,
                            0,
                            1u64 << 63,
                            1,
                        ]
                    } else {
                        vec![
                            f64::INFINITY.to_bits(),
                            f64::NEG_INFINITY.to_bits(),
                            0,
                            1u64 << 63,
                            1,
                            2.0f64.to_bits(),
                        ]
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
                    for restored in [false, true] {
                        let mut expected = vec![None; artifact.outputs().len()];
                        ordinary
                            .evaluate_into(&args, &mut expected, mode.execution(), &memory)
                            .unwrap();
                        let mut output = vec![None; artifact.outputs().len()];
                        let (result, counts) = if restored {
                            observe::run_restored(
                                &jit,
                                &args,
                                &mut output,
                                mode.execution(),
                                &memory,
                            )
                        } else {
                            observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                        };
                        result.unwrap();
                        assert_eq!(output_data(&output), output_data(&expected));
                        assert_eq!(counts.transpose_products.len(), usize::from(!input_nan));
                        drop((expected, output));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            baseline
                        );
                    }
                    assert_eq!(contents(&args), before);
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
    for artifact in artifacts("a<[f64]>' ** b<[f64]>\n") {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductViews,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let words = [1.0f64.to_bits()];
        let good = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("a", bits_matrix(&words, 3, 2), &[3, 2]),
                ("b", bits_matrix(&words, 3, 5), &[3, 5]),
            ],
        );
        let bad = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("a", bits_matrix(&words, 3, 2), &[3, 2]),
                ("b", bits_matrix(&words, 4, 5), &[4, 5]),
            ],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&good, &mut prior, mode.execution(), &memory)
            .unwrap();
        let before = contents(&bad);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            let mut output = prior.clone();
            let mut reference = prior.clone();
            let (a, _) = if restored {
                observe::run_restored(&jit, &bad, &mut output, mode.execution(), &memory)
            } else {
                observe::run(&jit, &bad, &mut output, mode.execution(), &memory)
            };
            let b = materialized.evaluate_into(&bad, &mut reference, mode.execution(), &memory);
            assert!(a.is_err() && b.is_err());
            assert_eq!(
                format!("{:?}", a.unwrap_err()),
                format!("{:?}", b.unwrap_err())
            );
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            assert_eq!(contents(&bad), before);
            drop((output, reference));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
            let mut retry = prior.clone();
            jit.evaluate_into(&good, &mut retry, mode.execution(), &memory)
                .unwrap();
            drop(retry);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        drop((good, bad, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_transpose_products_keep_finite_source_obligations_physical_denials_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts("a<[f64]>' ** b<[f64]>\n") {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductViews,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let words = [1.25f64.to_bits(), (-2.0f64).to_bits(), 0];
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("a", bits_matrix(&words, 3, 2), &[3, 2]),
                ("b", bits_matrix(&words, 3, 5), &[3, 5]),
            ],
        );
        let aliases = args.clone();
        let before = contents(&args);
        let mut prior = vec![None; artifact.outputs().len()];
        let full = native
            .evaluate_into(&args, &mut prior, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        for case in 0..19 {
            let mut policy = mode.execution();
            match case {
                0 => policy.library.max_output_elements = None,
                1 => policy.library.max_output_elements = Some(0),
                2 => policy.library.max_output_bytes = None,
                3 => policy.library.max_output_bytes = Some(0),
                4 => policy.library.max_temporary_bytes = None,
                5 => policy.library.max_temporary_bytes = Some(0),
                6 => policy.library.max_cloned_bytes = None,
                7 => policy.library.max_cloned_bytes = Some(0),
                8 => policy.library.max_retained_nodes = None,
                9 => policy.library.max_retained_nodes = Some(0),
                10 => policy.library.max_compute_work = None,
                11 => policy.library.max_compute_work = Some(0),
                12 => policy.library.max_comparison_work = None,
                13 => policy.library.max_comparison_work = Some(0),
                14 => policy.library.max_scalar_instructions = None,
                15 => policy.library.max_transfer_bytes = None,
                16 => policy.library.max_storage_buffer_bytes = None,
                17 => policy.library.max_storage_bindings = None,
                _ => policy.execution.max_work = 1,
            }
            for restored in [false, true] {
                let mut output = prior.clone();
                let mut reference = prior.clone();
                let (a, _) = if restored {
                    observe::run_restored(&native, &args, &mut output, policy, &memory)
                } else {
                    observe::run(&native, &args, &mut output, policy, &memory)
                };
                let b = materialized.evaluate_into(&args, &mut reference, policy, &memory);
                assert_eq!(a.is_ok(), b.is_ok(), "cap{case} {a:?}/{b:?}");
                if a.is_err() {
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                } else {
                    assert_eq!(output_data(&output), output_data(&prior));
                }
                drop((output, reference));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
        }
        let mut spent = mode.execution();
        spent.library.max_comparison_work = Some(full.comparison_work - 1);
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, spent, &memory);
        result.unwrap();
        assert!(counts.transpose_products.is_empty());
        assert_eq!(output_data(&output), output_data(&prior));
        drop(output);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        for restored in [false, true] {
            for dimension in ["bytes", "nodes"] {
                let allowances: &[u64] = if dimension == "bytes" {
                    &[
                        0,
                        4096,
                        65536,
                        1_048_576,
                        16_777_216,
                        64_000_000,
                        1_000_000_000_000,
                    ]
                } else {
                    &[0, 16, 256, 4096, 65536, 1_000_000_000]
                };
                let mut denied = 0;
                let mut succeeded = 0;
                for allowance in allowances {
                    let bounded = if dimension == "bytes" {
                        memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                    } else {
                        memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                    };
                    let mut output = prior.clone();
                    let (result, _) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut output,
                            mode.execution(),
                            &bounded,
                        )
                    } else {
                        observe::run(&native, &args, &mut output, mode.execution(), &bounded)
                    };
                    if result.is_ok() {
                        succeeded += 1;
                        assert_eq!(output_data(&output), output_data(&prior));
                    } else {
                        denied += 1;
                        assert!(output.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_nodes()),
                        baseline,
                        "{dimension} {allowance}"
                    );
                    assert_eq!(contents(&args), before);
                }
                assert!(
                    denied > 0 && succeeded > 0,
                    "{dimension}: denied{denied} succeeded{succeeded}"
                );
            }
        }
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(counts.transpose_products.len(), 1);
        drop(output);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        assert_eq!(contents(&aliases), before);
        drop((args, aliases, prior));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}

mod loan_checks {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper, NumericBinaryFrame};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {
        static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};
        static CASE:Cell<u8>=const{Cell::new(0)};
        static CHECKED:Cell<u64>=const{Cell::new(0)};
    }
    unsafe extern "C" fn helper(
        context: *mut c_void,
        mut site: u64,
        op: u64,
        mut a: u64,
        mut b: u64,
        mut c: u64,
        out: *mut u64,
    ) -> u64 {
        let case = CASE.with(Cell::get);
        if op == GlobalHelper::NumericBinaryPrepare.raw() && case == 12 {
            a ^= 1;
        }
        if op == GlobalHelper::NumericBinaryFinish.raw() {
            CHECKED.with(|n| n.set(n.get() + 1));
            if case == 1 {
                a = 1;
            }
            if case == 2 {
                b += 1;
            }
            if case == 16 {
                site += 1;
            }
            if case == 17 {
                c = 1;
            }
            if (3..=8).contains(&case) || case == 13 || case == 18 {
                // This valid current frame is mutated only after arithmetic;
                // no generated code observes the forged pointers or geometry.
                let frame = unsafe { &mut *(a as *mut NumericBinaryFrame) };
                match case {
                    3 => frame.output ^= 1,
                    4 => frame.left ^= 1,
                    5 => frame.columns += 1,
                    6 => frame.left_mode = 4,
                    7 => frame.right_scalar ^= 1,
                    8 => frame.count += 1,
                    13 => frame.inner += 1,
                    18 => frame.right ^= 1,
                    _ => unreachable!(),
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            match case {
                9 => invocation.state.consumed = true,
                10 => invocation.state.control.previous = None,
                11 => {
                    let visits = invocation.state.control.visits;
                    assert!(invocation.visit(site).is_err());
                    assert!(
                        invocation
                            .helper(site, GlobalHelper::Move.raw(), [0; 3])
                            .is_err()
                    );
                    assert_eq!(invocation.state.control.visits, visits);
                }
                14 => invocation.state.control.visits += 1,
                15 => invocation.state.control.current = Some(u32::MAX),
                _ => {}
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_transpose_products_reject_tampered_loans_epochs_context_and_interleaving_then_retry()
    {
        for source in ["a<[f64]>' ** b<[f64]>\n", "a<[f64]> ** b<[f64]>'\n"] {
            for artifact in artifacts(source) {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let words = [1.25f64.to_bits(), (-2.0f64).to_bits()];
                let left = if source.starts_with("a<[f64]>'") {
                    [3, 2]
                } else {
                    [2, 3]
                };
                let right = if source.starts_with("a<[f64]>'") {
                    [3, 5]
                } else {
                    [5, 3]
                };
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
                let mut prior = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut prior, mode.execution(), &memory)
                    .unwrap();
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for case in 0..=18 {
                    CASE.with(|v| v.set(case));
                    CHECKED.with(|v| v.set(0));
                    let mut output = prior.clone();
                    let entry = jit.test_checked_entry();
                    let result = execute(
                        jit.descriptor(),
                        |raw: *mut Frame| unsafe {
                            let old = (*raw).helper;
                            ORIGINAL.with(|v| v.set(Some(old)));
                            (*raw).helper = helper;
                            let status = entry(raw);
                            (*raw).helper = old;
                            ORIGINAL.with(|v| v.set(None));
                            status
                        },
                        &args,
                        &mut output,
                        mode.execution(),
                        &memory,
                    );
                    assert_eq!(
                        result.is_ok(),
                        case == 0 || case == 11,
                        "case{case}: {result:?}"
                    );
                    assert_eq!(CHECKED.with(Cell::get), u64::from(case != 12));
                    if result.is_err() {
                        assert!(output.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                    } else {
                        assert_eq!(output_data(&output), output_data(&prior));
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    let mut retry = prior.clone();
                    let (result, counts) =
                        observe::run_restored(&jit, &args, &mut retry, mode.execution(), &memory);
                    result.unwrap();
                    assert_eq!(counts.transpose_products.len(), 1);
                    drop(retry);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    assert_eq!(contents(&args), before);
                }
                assert_eq!(contents(&aliases), before);
                drop((args, aliases, prior));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

fn corrupt_product_operand(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(operands) = fields
                .get_mut("LibraryCall")
                .and_then(|call| call.get_mut("operands"))
                .and_then(|values| values.as_array_mut())
                && operands.len() == 2
            {
                operands[0] = (u32::MAX as u64).into();
                return true;
            }
            fields.values_mut().any(corrupt_product_operand)
        }
        serde_json::Value::Array(values) => values.iter_mut().any(corrupt_product_operand),
        _ => false,
    }
}
#[test]
fn numeric_transpose_products_rederive_private_views_from_genuine_json_and_packed_images() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts("a<[f64]>' ** b<[f64]>\n") {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let original =
            numeric_transpose_products::derive(&jit.descriptor().metadata().plan).unwrap();
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        for wire in [
            encoded.clone(),
            crate::native_artifact::program::image::packing::pack(&encoded).unwrap(),
        ] {
            // SAFETY: metadata emitted by this genuine checked JIT. No entry
            // is fabricated; changed descriptors below are never executed.
            let decoded =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                    .unwrap();
            assert_eq!(decoded.encode_build_owned_image().unwrap(), encoded);
            assert_eq!(
                numeric_transpose_products::derive(&decoded.metadata().plan).unwrap(),
                original
            );
        }
        let mut identity_only = image.clone();
        let byte = identity_only["descriptor"]["identity"][0].as_u64().unwrap();
        identity_only["descriptor"]["identity"][0] = ((byte + 1) % 256).into();
        // SAFETY: confined metadata-only inspection; this changed descriptor
        // is never bound to native code and never reaches execution.
        let changed = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(
                &serde_json::to_vec(&identity_only).unwrap(),
                &catalog,
            )
        }
        .unwrap();
        assert_eq!(
            numeric_transpose_products::derive(&changed.metadata().plan).unwrap(),
            original
        );
        for which in 0..3 {
            let mut changed = image.clone();
            match which {
                0 => {
                    changed["descriptor"]["numeric_transpose_products"] =
                        serde_json::json!({"skip_all":true,"owns_source":true})
                }
                1 => {
                    assert!(corrupt_product_operand(&mut changed));
                }
                _ => {
                    assert!(!changed["descriptor"]["plan"].is_null());
                    changed["descriptor"]["plan"] = serde_json::Value::Null;
                }
            }
            let bytes = serde_json::to_vec(&changed).unwrap();
            for wire in [
                bytes.clone(),
                crate::native_artifact::program::image::packing::pack(&bytes).unwrap(),
            ] {
                // SAFETY: independent wire-only negative decode, no execution.
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                        .is_err(),
                    "forgery{which}"
                );
            }
        }
    }
}
