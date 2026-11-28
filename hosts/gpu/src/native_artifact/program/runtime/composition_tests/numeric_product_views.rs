//! Closed view prefixes reuse the current product-epilogue loan and suffix replay.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::numeric_binary::Kind;
use mech_core::snapshot::F64Bits;
const EXPRESSION: &str = "a := left<[f64]>\nb := right<[f64]>\nc := extra<[f64]>\ns := scale<f64>\ns * (a' ** b) + s * c\n";
fn matrix(words: &[u64], shape: [u64; 2]) -> Data {
    Data::Matrix(
        (0..shape[0] * shape[1])
            .map(|i| Data::F64(F64Bits::from_bits(words[i as usize % words.len()])))
            .collect(),
    )
}
fn outputs(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn recipe(
    jit: &JitProgramArtifact,
) -> &crate::native_artifact::program::prepare::numeric_product_epilogues::Candidate {
    let m = jit.descriptor().metadata();
    (0..m.plan.blocks().len())
        .find_map(|s| m.numeric_binary.transpose_epilogue(s))
        .unwrap()
}
#[test]
fn numeric_product_views_closed_graph_and_independent_suffix_objects() {
    for source in ["a<[f64]>' ** b<[f64]>\n", EXPRESSION] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let m = jit.descriptor().metadata();
            let views = (0..m.plan.blocks().len())
                .filter_map(|s| m.numeric_binary.transpose_epilogue(s))
                .collect::<Vec<_>>();
            if views.is_empty() {
                for (i, b) in m.plan.blocks().iter().enumerate() {
                    eprintln!("{i}: {b:?}");
                }
            }
            assert_eq!(views.len(), 1, "{source}");
            let r = views[0];
            assert_eq!(r.product_index(), Some(1));
            assert!(r.transposed_left());
            assert!(r.physical_roots().is_some());
            assert!(matches!(r.nodes[0].kind, Kind::Transpose { .. }));
            assert_eq!(r.nodes[1].kind, Kind::Product);
            assert!(r.elided_values.contains(&r.nodes[0].output.get()));
            if source == EXPRESSION {
                let suffix = m.numeric_binary.epilogue(r.nodes[1].site as usize).unwrap();
                assert!(!suffix.transposed_left());
                assert_eq!(suffix.product_index(), Some(0));
            }
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            assert!(
                object.metadata()["compiled_product_epilogues"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|v| v["entry"].as_u64() == Some(u64::from(r.entry)))
            );
        }
    }
}
#[test]
fn numeric_product_views_decline_escaping_reused_wrong_side_or_nonproduct_views() {
    for source in [
        "t := a<[f64]>'\n(t,t ** b<[f64]>)\n",
        "a<[f64]> ** b<[f64]>'\n",
        "t := a<[f64]>'\nt + b<[f64]>\n",
        "t := a<[f64]>'\n(t ** b<[f64]>) + t\n",
        "t := a<[f64]>'\n(t ** b<[f64]>, t ** c<[f64]>)\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let m = jit.descriptor().metadata();
            assert!(
                (0..m.plan.blocks().len())
                    .all(|s| m.numeric_binary.transpose_epilogue(s).is_none()),
                "{source}"
            );
        }
    }
}
#[test]
fn numeric_product_views_ordered_bits_broadcasts_empty_axes_and_actual_elided_banks_restored() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
        )
        .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericMatrixArithmetic,
        )
        .unwrap();
        let r = recipe(&native);
        assert_eq!(r.nodes.len(), 5);
        for (left, right, extra) in [
            ([3, 2], [3, 5], [2, 5]),
            ([3, 2], [3, 5], [1, 5]),
            ([3, 2], [3, 5], [2, 1]),
            ([3, 2], [3, 5], [1, 1]),
            ([7, 5], [7, 3], [5, 3]),
            ([5, 1], [5, 1], [1, 1]),
            ([0, 3], [0, 5], [3, 5]),
            ([3, 0], [3, 5], [0, 5]),
            ([3, 2], [3, 0], [2, 0]),
        ] {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let words = [
                0.0f64.to_bits(),
                (-0.0f64).to_bits(),
                1.25f64.to_bits(),
                (-0.5f64).to_bits(),
                1,
                0x0010000000000000,
            ];
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("left", matrix(&words, left), &left),
                    ("right", matrix(&words, right), &right),
                    ("extra", matrix(&words, extra), &extra),
                    ("scale", Data::F64(F64Bits::from_f64(-0.75)), &[]),
                ],
            );
            let before = contents(&args);
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut expected = vec![None; artifact.outputs().len()];
            ordinary
                .evaluate_into(&args, &mut expected, mode.execution(), &memory)
                .unwrap();
            let data = outputs(&expected);
            drop(expected);
            for restored in [false, true] {
                let mut output = vec![None; artifact.outputs().len()];
                let mut reference = output.clone();
                let (report, counts) = if restored {
                    observe::run_restored(&native, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&native, &args, &mut output, mode.execution(), &memory)
                };
                let report = report.unwrap();
                let (reference_report, _) = observe::run(
                    &materialized,
                    &args,
                    &mut reference,
                    mode.execution(),
                    &memory,
                );
                let reference_report = reference_report.unwrap();
                assert_eq!(outputs(&output), data, "{left:?}/{right:?}/{extra:?}");
                assert_eq!(outputs(&reference), data);
                assert_eq!(
                    counts.product_epilogues,
                    vec![(r.entry as usize, 5, (left[1] * right[1]) as usize, false)]
                );
                assert!(report.work >= reference_report.work);
                assert!(report.comparison_work >= reference_report.comparison_work);
                let suffix = native
                    .descriptor()
                    .metadata()
                    .numeric_binary
                    .epilogue(r.nodes[1].site as usize)
                    .unwrap();
                assert_eq!(
                    report.visits + (r.steps.len() - suffix.steps.len()) as u64,
                    reference_report.visits
                );
                let mut comparable = reference_report;
                comparable.work = report.work;
                comparable.comparison_work = report.comparison_work;
                comparable.visits = report.visits;
                assert_eq!(report, comparable);

                let aliases = output.clone();
                let (retry, counts) =
                    observe::run_restored(&native, &args, &mut output, mode.execution(), &memory);
                retry.unwrap();
                assert_eq!(outputs(&output), outputs(&aliases));
                assert!(!counts.product_epilogues.is_empty());
                drop((output, reference, aliases));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
            }
            assert_eq!(contents(&args), before);
            drop(args);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn numeric_product_views_nonfinite_inputs_or_terms_replay_without_leaking_partial_output() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let baseline = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
        )
        .unwrap();
        for nan in [false, true] {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let words = if nan {
                vec![1.0f64.to_bits(), 0x7ff8_1234_5678_9abc]
            } else {
                vec![f64::MAX.to_bits(), (-f64::MAX).to_bits()]
            };
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("left", matrix(&words, [3, 5]), &[3, 5]),
                    (
                        "right",
                        matrix(&[2.0f64.to_bits(), (-2.0f64).to_bits()], [3, 3]),
                        &[3, 3],
                    ),
                    ("extra", matrix(&[(-0.0f64).to_bits()], [5, 3]), &[5, 3]),
                    ("scale", Data::F64(F64Bits::from_f64(1.0)), &[]),
                ],
            );
            let mut actual = vec![None; artifact.outputs().len()];
            let mut expected = actual.clone();
            let (a, _) =
                observe::run_restored(&native, &args, &mut actual, mode.execution(), &memory);
            let (b, _) = observe::run(&baseline, &args, &mut expected, mode.execution(), &memory);
            a.unwrap();
            b.unwrap();
            assert_eq!(outputs(&actual), outputs(&expected));
            drop((actual, expected, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

fn gemm_args(
    artifact: &ProgramArtifact,
    memory: &mech_core::ManagedMemoryBudget,
    left: [u64; 2],
    right: [u64; 2],
    extra: [u64; 2],
    words: &[u64],
    alpha: f64,
    _beta: f64,
) -> Vec<Value> {
    let physical = [left[1], left[0]];
    numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("left", matrix(words, physical), &physical),
            ("right", matrix(words, right), &right),
            ("extra", matrix(words, extra), &extra),
            ("scale", Data::F64(F64Bits::from_f64(alpha)), &[]),
        ],
    )
}
#[test]
fn numeric_product_views_finite_caps_spent_comparison_physical_denials_cleanup_and_retry() {
    use crate::native_artifact::codegen::options::OptimizationPass;
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
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
                    assert_eq!(outputs(&output), outputs(&prior));
                }
                drop((output, reference));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
        }
        // An empty final domain does not erase the original nonempty
        // broadcast producer's source resource demand.
        for (left, right, extra, cap) in [
            ([0, 3], [3, 5], [1, 5], 0),
            ([2, 3], [3, 0], [2, 1], 1),
            ([0, 3], [3, 5], [1, 1], 0),
        ] {
            let empty = gemm_args(
                &artifact,
                &memory,
                left,
                right,
                extra,
                &[1.25f64.to_bits()],
                1.5,
                1.2,
            );
            let mut valid = vec![None; artifact.outputs().len()];
            let (r, counts) = observe::run(&native, &empty, &mut valid, mode.execution(), &memory);
            r.unwrap();
            assert_eq!(counts.product_epilogues.len(), 1);
            let empty_baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let mut bounded = mode.execution();
            bounded.library.max_output_elements = Some(cap);
            for restored in [false, true] {
                let mut output = valid.clone();
                let mut reference = valid.clone();
                let (a, counts) = if restored {
                    observe::run_restored(&native, &empty, &mut output, bounded, &memory)
                } else {
                    observe::run(&native, &empty, &mut output, bounded, &memory)
                };
                let b = materialized.evaluate_into(&empty, &mut reference, bounded, &memory);
                assert!(
                    a.is_err() && b.is_err(),
                    "logical producer {extra:?}, cap{cap}: {a:?}/{b:?}"
                );
                assert!(
                    counts
                        .product_epilogues
                        .iter()
                        .all(|x| x.0 != recipe(&native).entry as usize)
                );
                assert!(output.iter().zip(&valid).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                drop((output, reference));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    empty_baseline
                );
            }
            drop((valid, empty));
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        let mut spent = mode.execution();
        spent.library.max_comparison_work = Some(full.comparison_work - 1);
        let mut output = prior.clone();
        let (r, counts) = observe::run(&native, &args, &mut output, spent, &memory);
        r.unwrap();
        assert!(
            counts
                .product_epilogues
                .iter()
                .all(|x| x.0 != recipe(&native).entry as usize)
        );
        assert_eq!(outputs(&output), outputs(&prior));
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
                        assert_eq!(outputs(&output), outputs(&prior));
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
mod loan_checks {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper, NumericProductFrame};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};static CASE:Cell<u8>=const{Cell::new(0)};static CHECKED:Cell<u64>=const{Cell::new(0)};}
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
        if op == GlobalHelper::NumericProductPrepare.raw() && case == 19 {
            a ^= 1;
        }
        if op == GlobalHelper::NumericProductFinish.raw() {
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
                c = 2;
            }
            if case == 21 {
                c = 1;
            }
            if (3..=11).contains(&case) || case == 20 {
                // Arithmetic has completed. Generated code never observes the
                // forged pointer/geometry/word; only sealed finish checks do.
                let frame = unsafe { &mut *(a as *mut NumericProductFrame) };
                match case {
                    3 => frame.product.output ^= 1,
                    4 => frame.product.left ^= 1,
                    5 => frame.product.right ^= 1,
                    6 => frame.product.count += 1,
                    7 => frame.product.columns += 1,
                    8 => frame.product.inner += 1,
                    9 => frame.leaves[2].scalar ^= 1,
                    10 => frame.leaves[4].pointer ^= 1,
                    11 => frame.leaves[4].mode = 4,
                    20 => frame.leaves[7].pointer ^= 1,
                    _ => unreachable!(),
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            match case {
                12 => invocation.state.control.visits += 1,
                13 => invocation.state.control.current = Some(u32::MAX),
                14 => invocation.state.consumed = true,
                15 => invocation.state.control.previous = None,
                18 => {
                    let visits = invocation.state.control.visits;
                    assert!(invocation.visit(site).is_err());
                    assert!(
                        invocation
                            .helper(site, GlobalHelper::Move.raw(), [0; 3])
                            .is_err()
                    );
                    assert_eq!(invocation.state.control.visits, visits);
                }
                _ => {}
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_product_views_reject_forged_loans_epochs_context_interleaving_and_retry() {
        let catalog = mech_stdlib::source_catalog();
        for artifact in artifacts(EXPRESSION) {
            let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
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
            jit.evaluate_into(&args, &mut prior, mode.execution(), &memory)
                .unwrap();
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for case in 0..=21 {
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
                    case == 0 || case == 18,
                    "case{case}: {result:?}"
                );
                assert_eq!(CHECKED.with(Cell::get), u64::from(case != 19));
                if result.is_err() {
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                } else {
                    assert_eq!(outputs(&output), outputs(&prior));
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
                assert_eq!(counts.product_epilogues.len(), 1);
                drop(retry);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
                assert_eq!(contents(&args), before);
            }
            assert_eq!(contents(&aliases), before);
            drop((prior, args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
fn corrupt_product_operand(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(fields) => {
            if let Some(operands) = fields
                .get_mut("LibraryCall")
                .and_then(|c| c.get_mut("operands"))
                .and_then(|v| v.as_array_mut())
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
fn numeric_product_views_shape_failures_preserve_first_error_aliases_and_retry() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let baseline = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let words = [1.25f64.to_bits(), (-2.0f64).to_bits()];
        let good = gemm_args(
            &artifact,
            &memory,
            [2, 3],
            [3, 5],
            [2, 5],
            &words,
            0.75,
            1.0,
        );
        let mut prior = vec![None; artifact.outputs().len()];
        native
            .evaluate_into(&good, &mut prior, mode.execution(), &memory)
            .unwrap();
        for (left, right, extra) in [([2, 3], [4, 5], [2, 5]), ([2, 3], [3, 5], [3, 5])] {
            let args = gemm_args(&artifact, &memory, left, right, extra, &words, 0.75, 1.0);
            let before = contents(&args);
            let used = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for restored in [false, true] {
                let mut actual = prior.clone();
                let mut expected = prior.clone();
                let (a, _) = if restored {
                    observe::run_restored(&native, &args, &mut actual, mode.execution(), &memory)
                } else {
                    observe::run(&native, &args, &mut actual, mode.execution(), &memory)
                };
                let b = baseline.evaluate_into(&args, &mut expected, mode.execution(), &memory);
                assert_eq!(
                    format!("{:?}", a.unwrap_err()),
                    format!("{:?}", b.unwrap_err())
                );
                assert!(actual.iter().zip(&prior).all(|(a, p)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
                drop((actual, expected));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    used
                );
            }
            assert_eq!(contents(&args), before);
            drop(args);
        }
        let (r, counts) =
            observe::run_restored(&native, &good, &mut prior, mode.execution(), &memory);
        r.unwrap();
        assert!(
            counts
                .product_epilogues
                .iter()
                .any(|x| x.0 == recipe(&native).entry as usize)
        );
        drop((prior, good));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_product_views_partial_private_rows_are_discarded_before_nonfinite_replay() {
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(EXPRESSION) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let baseline = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let mut left = vec![1.0f64.to_bits(); 15];
        left[4] = f64::MAX.to_bits();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("left", matrix(&left, [3, 5]), &[3, 5]),
                ("right", matrix(&[2.0f64.to_bits()], [3, 3]), &[3, 3]),
                ("extra", matrix(&[0.0f64.to_bits()], [5, 3]), &[5, 3]),
                ("scale", Data::F64(F64Bits::from_f64(1.0)), &[]),
            ],
        );
        let mut actual = vec![None; artifact.outputs().len()];
        let mut expected = actual.clone();
        let (a, counts) =
            observe::run_restored(&native, &args, &mut actual, mode.execution(), &memory);
        let b = baseline.evaluate_into(&args, &mut expected, mode.execution(), &memory);
        a.unwrap();
        b.unwrap();
        assert!(
            counts
                .product_epilogues
                .iter()
                .any(|x| x.0 == recipe(&native).entry as usize)
        );
        assert_eq!(outputs(&actual), outputs(&expected));
        drop((actual, expected, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_product_views_rederive_private_recipes_and_scalar_capacity_from_genuine_images() {
    let catalog = mech_stdlib::source_catalog();
    let source = EXPRESSION;
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let m = jit.descriptor().metadata();
        let original = (0..m.plan.blocks().len())
            .filter_map(|s| m.numeric_binary.epilogue(s).cloned())
            .collect::<Vec<_>>();
        assert_eq!(original.iter().filter(|r| r.transposed_left()).count(), 1);
        assert_eq!(m.numeric_binary.epilogue_arguments(), 1);
        assert!(m.max_library_inputs >= 2);
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        for wire in [
            encoded.clone(),
            crate::native_artifact::program::image::packing::pack(&encoded).unwrap(),
        ] {
            // SAFETY: genuine metadata from this checked JIT, no invented entry.
            let decoded =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                    .unwrap();
            assert_eq!(decoded.encode_build_owned_image().unwrap(), encoded);
            let d = decoded.metadata();
            assert_eq!(
                (0..d.plan.blocks().len())
                    .filter_map(|s| d.numeric_binary.epilogue(s).cloned())
                    .collect::<Vec<_>>(),
                original
            );
        }
        let mut identity = image.clone();
        let byte = identity["descriptor"]["identity"][0].as_u64().unwrap();
        identity["descriptor"]["identity"][0] = ((byte + 1) % 256).into();
        // SAFETY: confined metadata-only inspection. The unsafe decoder trusts
        // caller provenance; this changed descriptor is never executed/bound.
        let decoded = unsafe {
            NativeProgramDescriptor::decode_build_owned_image(
                &serde_json::to_vec(&identity).unwrap(),
                &catalog,
            )
        }
        .unwrap();
        let d = decoded.metadata();
        assert_eq!(
            (0..d.plan.blocks().len())
                .filter_map(|s| d.numeric_binary.epilogue(s).cloned())
                .collect::<Vec<_>>(),
            original
        );
        for which in 0..4 {
            let mut changed = image.clone();
            match which {
                0 => {
                    changed["descriptor"]["numeric_product_epilogues"] =
                        serde_json::json!({"owns_source":true,"fees_paid":true})
                }
                1 => {
                    assert!(corrupt_product_operand(&mut changed));
                }
                2 => changed["descriptor"]["plan"] = serde_json::Value::Null,
                _ => changed["descriptor"]["max_library_inputs"] = 0.into(),
            }
            let bytes = serde_json::to_vec(&changed).unwrap();
            for wire in [
                bytes.clone(),
                crate::native_artifact::program::image::packing::pack(&bytes).unwrap(),
            ] {
                // SAFETY: wire-only negative decode; no candidate execution.
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                        .is_err(),
                    "forgery{which}"
                );
            }
        }
    }
}

#[test]
fn numeric_product_views_virtual_node_preserves_the_full_arithmetic_suffix_limit() {
    let mut expression = String::from("a' ** b");
    for _ in 0..20 {
        expression = format!("({expression}) * s");
    }
    let source = format!("a := left<[f64]>\nb := right<[f64]>\ns := scale<f64>\n{expression}\n");
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(&source) {
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let materialized = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::TransposeProductEpilogues,
        )
        .unwrap();
        let r = recipe(&native);
        assert_eq!(r.nodes.len(), 17);
        let suffix = native
            .descriptor()
            .metadata()
            .numeric_binary
            .epilogue(r.nodes[1].site as usize)
            .unwrap();
        assert_eq!(suffix.nodes.len(), 16);
        assert!(
            suffix
                .steps
                .iter()
                .all(|step| r.steps.iter().any(|ours| ours.site() == step.site()))
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                (
                    "left",
                    matrix(&[1.25f64.to_bits(), (-0.5f64).to_bits()], [3, 2]),
                    &[3, 2],
                ),
                ("right", matrix(&[2.0f64.to_bits()], [3, 5]), &[3, 5]),
                ("scale", Data::F64(F64Bits::from_f64(0.75)), &[]),
            ],
        );
        let mut output = vec![None; artifact.outputs().len()];
        let mut reference = output.clone();
        let (a, counts) =
            observe::run_restored(&native, &args, &mut output, mode.execution(), &memory);
        let b = materialized.evaluate_into(&args, &mut reference, mode.execution(), &memory);
        a.unwrap();
        b.unwrap();
        assert!(
            counts
                .product_epilogues
                .iter()
                .any(|x| x.0 == r.entry as usize && x.1 == 17)
        );
        assert_eq!(outputs(&output), outputs(&reference));
        drop((output, reference, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
