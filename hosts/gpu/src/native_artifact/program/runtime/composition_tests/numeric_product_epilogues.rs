//! Source proofs and actual fused product publication, original/decoded/restored.
use super::*;
#[test]
fn numeric_product_epilogues_source_graphs() {
    for source in [
        "(a<[f64]> ** b<[f64]>) * scale<f64>\n",
        "alpha<f64> * (a<[f64]> ** b<[f64]>) + beta<f64> * c<[f64]>\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            let candidates = (0..m.plan.blocks().len())
                .filter_map(|site| m.numeric_binary.epilogue(site))
                .collect::<Vec<_>>();
            eprintln!("SOURCE {source} RECIPES {candidates:?}");
            if candidates.is_empty() {
                for (site, block) in m.plan.blocks().iter().enumerate() {
                    eprintln!("{site}: {block:?}");
                }
            }
            assert_eq!(candidates.len(), 1);
            assert_eq!(
                m.numeric_binary.epilogue_arguments(),
                candidates[0].scalars.len()
            );
            let object = crate::native_artifact::emit_program_object(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            assert_eq!(
                object.metadata()["compiled_product_epilogues"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
        }
    }
}
#[test]
fn numeric_product_epilogues_optimization_source_graphs() {
    for name in ["gemm", "2mm", "mvt"] {
        let source = std::fs::read_to_string(format!(
            "{}/tests/fixtures/polybench/{name}.mec",
            env!("CARGO_MANIFEST_DIR")
        ))
        .unwrap();
        for artifact in artifacts(&source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            let candidates = (0..m.plan.blocks().len())
                .filter_map(|site| m.numeric_binary.epilogue(site))
                .collect::<Vec<_>>();
            eprintln!("OPTIMIZATION {name} RECIPES {candidates:?}");
            assert!(!candidates.is_empty(), "{name}");
        }
    }
}
fn bits_matrix(words: &[u64], shape: [u64; 2]) -> Data {
    Data::Matrix(
        (0..shape[0] * shape[1])
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
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
const EXPRESSION: &str = "alpha<f64> * (a<[f64]> ** b<[f64]>) + beta<f64> * c<[f64]>\n";
#[test]
fn numeric_product_epilogues_decline_escaping_products_extra_products_and_unavailable_sources() {
    for source in [
        "p := a<[f64]> ** b<[f64]>\n(p, p * scale<f64>)\n",
        "p := a<[f64]> ** b<[f64]>\n(p * scale<f64>, p * other<f64>)\n",
        "(a<[f64]> ** b<[f64]>) ** c<[f64]>\n",
        "+> stats\np := a<[f64]> ** b<[f64]>\np + stats/sum/row(c<[f64]>)\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            assert_eq!(
                (0..m.plan.blocks().len())
                    .filter(|&s| m.numeric_binary.epilogue(s).is_some())
                    .count(),
                0,
                "{source}"
            );
        }
    }
}
#[test]
fn numeric_product_epilogues_ordered_bits_broadcasts_empty_axes_actual_banks_fees_and_restored_loans()
 {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ProductEpilogues,
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericMatrixArithmetic,
        )
        .unwrap();
        let m = native.descriptor().metadata();
        let recipe = (0..m.plan.blocks().len())
            .find_map(|s| m.numeric_binary.epilogue(s))
            .unwrap();
        assert_eq!(recipe.nodes.len(), 4);
        let words = [
            0.0f64.to_bits(),
            (-0.0f64).to_bits(),
            1.25f64.to_bits(),
            (-0.5f64).to_bits(),
            1,
            0x0010000000000000,
        ];
        for (left, right, extra) in [
            ([2, 3], [3, 5], [2, 5]),
            ([2, 3], [3, 5], [1, 5]),
            ([2, 3], [3, 5], [2, 1]),
            ([2, 3], [3, 5], [1, 1]),
            ([1, 3], [3, 1], [1, 1]),
            ([3, 0], [0, 5], [3, 5]),
            ([0, 3], [3, 5], [0, 5]),
            ([2, 3], [3, 0], [2, 0]),
        ] {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("a", bits_matrix(&words, left), &left),
                    ("b", bits_matrix(&words, right), &right),
                    ("c", bits_matrix(&words, extra), &extra),
                    (
                        "alpha",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(-0.75)),
                        &[],
                    ),
                    (
                        "beta",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(1.25)),
                        &[],
                    ),
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
                    let (reference_report, reference_counts) = observe::run(
                        &materialized,
                        &args,
                        &mut reference,
                        mode.execution(),
                        &memory,
                    );
                    let reference_report = reference_report.unwrap();
                    let mut output = vec![None; artifact.outputs().len()];
                    let (report, counts) = if restored {
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
                    let report = report.unwrap();
                    assert_eq!(
                        output_data(&output),
                        expected_data,
                        "{left:?}/{right:?}/{extra:?}"
                    );
                    assert_eq!(output_data(&reference), expected_data);
                    assert_eq!(
                        counts.product_epilogues,
                        vec![(
                            recipe.entry as usize,
                            4,
                            (left[0] * right[1]) as usize,
                            false
                        )],
                        "helpers {:?}",
                        counts.helpers
                    );
                    for step in recipe.steps.iter().skip(1) {
                        assert!(
                            reference_counts
                                .admission_callbacks
                                .get(&u64::from(step.site()))
                                .copied()
                                .unwrap_or(0)
                                > 0
                        );
                        assert_eq!(
                            counts
                                .admission_callbacks
                                .get(&u64::from(step.site()))
                                .copied()
                                .unwrap_or(0),
                            0
                        );
                    }
                    assert_eq!(
                        report.visits + recipe.steps.len() as u64 - 1,
                        reference_report.visits
                    );
                    assert!(
                        report.work >= reference_report.work,
                        "source work decreased"
                    );
                    assert!(report.comparison_work >= reference_report.comparison_work);
                    let mut comparable = reference_report;
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
fn gemm_args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    left: [u64; 2],
    right: [u64; 2],
    extra: [u64; 2],
    words: &[u64],
    alpha: f64,
    beta: f64,
) -> Vec<Value> {
    numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("a", bits_matrix(words, left), &left),
            ("b", bits_matrix(words, right), &right),
            ("c", bits_matrix(words, extra), &extra),
            (
                "alpha",
                Data::F64(mech_core::snapshot::F64Bits::from_f64(alpha)),
                &[],
            ),
            (
                "beta",
                Data::F64(mech_core::snapshot::F64Bits::from_f64(beta)),
                &[],
            ),
        ],
    )
}
#[test]
fn numeric_product_epilogues_nonfinite_bits_final_division_fixed_shapes_atomic_errors_and_expansion()
 {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for source in [
        EXPRESSION,
        "alpha<f64> * (a<[f64]:2,3> ** b<[f64]:3,5>) + beta<f64> * c<[f64]:2,5>\n",
    ] {
        for artifact in artifacts(source) {
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::NumericMatrixArithmetic,
            )
            .unwrap();
            for words in [
                vec![1.0e308f64.to_bits()],
                vec![f64::INFINITY.to_bits(), 0, (-0.0f64).to_bits()],
                vec![0x7ff8000000000042, 0x7ff0000000000031, 0xfff8000000000074],
            ] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let args = gemm_args(&artifact, &memory, [2, 3], [3, 5], [2, 5], &words, 0.0, 0.0);
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
                    result.unwrap();
                    assert_eq!(output_data(&output), expected_data);
                    assert!(counts.product_epilogues.len() <= 1);
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
                assert_eq!(contents(&aliases), before);
                drop((aliases, args));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
    for artifact in artifacts("(a<[f64]> ** b<[f64]>) / scale<f64>\n") {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericMatrixArithmetic,
        )
        .unwrap();
        for word in [0u64, 1.0f64.to_bits(), (-0.0f64).to_bits()] {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("a", bits_matrix(&[word], [2, 1]), &[2, 1]),
                    ("b", bits_matrix(&[1.0f64.to_bits()], [1, 3]), &[1, 3]),
                    (
                        "scale",
                        Data::F64(mech_core::snapshot::F64Bits::from_f64(0.0)),
                        &[],
                    ),
                ],
            );
            let mut a = vec![None; artifact.outputs().len()];
            let mut b = a.clone();
            let (r, counts) =
                observe::run_restored(&native, &args, &mut a, mode.execution(), &memory);
            r.unwrap();
            ordinary
                .evaluate_into(&args, &mut b, mode.execution(), &memory)
                .unwrap();
            assert_eq!(output_data(&a), output_data(&b));
            assert_eq!(counts.product_epilogues.len(), 1);
            drop((a, b, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ProductEpilogues,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let words = [1.25f64.to_bits(), (-2.0f64).to_bits()];
        let good = gemm_args(&artifact, &memory, [2, 3], [3, 5], [2, 5], &words, 1.5, 1.2);
        let mut prior = vec![None; artifact.outputs().len()];
        native
            .evaluate_into(&good, &mut prior, mode.execution(), &memory)
            .unwrap();
        for (left, right, extra, success) in [
            ([2, 3], [4, 5], [2, 5], false),
            ([2, 3], [3, 5], [3, 5], false),
            ([1, 3], [3, 5], [2, 5], true),
        ] {
            let args = gemm_args(&artifact, &memory, left, right, extra, &words, 1.5, 1.2);
            let before = contents(&args);
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for restored in [false, true] {
                let mut output = prior.clone();
                let mut reference = prior.clone();
                let (a, counts) = if restored {
                    observe::run_restored(&native, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&native, &args, &mut output, mode.execution(), &memory)
                };
                let b =
                    materialized.evaluate_into(&args, &mut reference, mode.execution(), &memory);
                assert_eq!(a.is_ok(), success);
                assert_eq!(b.is_ok(), success);
                assert!(counts.product_epilogues.is_empty());
                if success {
                    assert_eq!(output_data(&output), output_data(&reference));
                } else {
                    assert_eq!(
                        format!("{:?}", a.unwrap_err()),
                        format!("{:?}", b.unwrap_err())
                    );
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                drop((output, reference));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
            }
            assert_eq!(contents(&args), before);
            drop(args);
        }
        let mut retry = prior.clone();
        let (r, counts) =
            observe::run_restored(&native, &good, &mut retry, mode.execution(), &memory);
        r.unwrap();
        assert_eq!(counts.product_epilogues.len(), 1);
        drop((retry, prior, good));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_product_epilogues_finite_caps_spent_comparison_physical_denials_cleanup_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ProductEpilogues,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = gemm_args(
            &artifact,
            &memory,
            [2, 3],
            [3, 5],
            [2, 5],
            &[1.25f64.to_bits(), (-2.0f64).to_bits()],
            1.5,
            1.2,
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
        let (r, counts) = observe::run(&native, &args, &mut output, spent, &memory);
        r.unwrap();
        assert!(counts.product_epilogues.is_empty());
        assert_eq!(output_data(&output), output_data(&prior));
        drop(output);
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
                for &allowance in allowances {
                    let bounded = if dimension == "bytes" {
                        memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                    } else {
                        memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                    };
                    let mut output = prior.clone();
                    let (r, _) = if restored {
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
                    if r.is_ok() {
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
                    "{dimension}: denied{denied}, succeeded{succeeded}"
                );
            }
        }
        let mut output = prior.clone();
        let (r, counts) =
            observe::run_restored(&native, &args, &mut output, mode.execution(), &memory);
        r.unwrap();
        assert_eq!(counts.product_epilogues.len(), 1);
        drop(output);
        assert_eq!(contents(&aliases), before);
        drop((args, aliases, prior));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
