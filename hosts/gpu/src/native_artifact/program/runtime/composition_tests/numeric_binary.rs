//! Exact IEEE results, shared broadcasts, actual selection and funded fallback.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;

fn data(values: &[f64]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|v| Data::F64(F64Bits::from_f64(*v)))
            .collect(),
    )
}
fn completed(counts: &observe::Counts) -> u64 {
    counts
        .helpers
        .iter()
        .filter(|((_, op), _)| *op == GlobalHelper::NumericBinaryFinish.raw())
        .map(|(_, n)| n)
        .sum()
}
fn outputs(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}

#[test]
fn numeric_binary_broadcasts_simd_tails_and_dynamic_shapes_match_ordinary_bits() {
    let shapes = [
        ([2, 3], [2, 3]),
        ([3, 3], [3, 3]),
        ([2, 3], [1, 3]),
        ([2, 3], [2, 1]),
        ([1, 3], [2, 1]),
        ([2, 1], [1, 3]),
        ([1, 1], [2, 3]),
        ([2, 3], [1, 1]),
        ([1, 1], [1, 1]),
        ([0, 3], [0, 3]),
        ([2, 0], [2, 0]),
    ];
    for operator in ["+", "-", "*", "/"] {
        for scalar in [0, 1, 2] {
            let left = if scalar == 1 { "f64" } else { "[f64]" };
            let right = if scalar == 2 { "f64" } else { "[f64]" };
            let source = format!("input<{left}> {operator} peer<{right}>\n");
            for artifact in artifacts(&source) {
                let mode = NativeProgramLimitMode::NoLimits;
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let ordinary = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::NumericElementwise,
                )
                .unwrap();
                assert_eq!(
                    native
                        .descriptor()
                        .metadata()
                        .plan
                        .blocks()
                        .iter()
                        .enumerate()
                        .filter(|(i, _)| native
                            .descriptor()
                            .metadata()
                            .numeric_binary
                            .at(*i)
                            .is_some())
                        .count(),
                    1
                );
                for (ld, rd) in shapes {
                    let memory = mode.memory_budget();
                    let left = (0..ld[0] * ld[1])
                        .map(|i| if i % 3 == 0 { -0.0 } else { i as f64 + 0.25 })
                        .collect::<Vec<_>>();
                    let right = (0..rd[0] * rd[1])
                        .map(|i| if i % 2 == 0 { -2.0 } else { 0.0 })
                        .collect::<Vec<_>>();
                    let ldata = if scalar == 1 {
                        Data::F64(F64Bits::from_f64(-0.0))
                    } else {
                        data(&left)
                    };
                    let rdata = if scalar == 2 {
                        Data::F64(F64Bits::from_f64(2.0))
                    } else {
                        data(&right)
                    };
                    let args = numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", ldata, if scalar == 1 { &[] } else { &ld }),
                            ("peer", rdata, if scalar == 2 { &[] } else { &rd }),
                        ],
                    );
                    let before = contents(&args);
                    let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut expected = actual.clone();
                    let (a, counts) =
                        observe::run(&native, &args, &mut actual, mode.execution(), &memory);
                    let (b, ordinary_counts) =
                        observe::run(&ordinary, &args, &mut expected, mode.execution(), &memory);
                    assert_eq!(
                        a.is_ok(),
                        b.is_ok(),
                        "{source} {ld:?} {rd:?}: {a:?} / {b:?}"
                    );
                    if let (Ok(a), Ok(b)) = (a, b) {
                        assert_eq!(
                            outputs(&actual),
                            outputs(&expected),
                            "{source} {ld:?} {rd:?}"
                        );
                        assert_eq!(
                            completed(&counts),
                            1,
                            "selected {source} {ld:?} {rd:?}: {:?}",
                            counts.helpers
                        );
                        assert_eq!(completed(&ordinary_counts), 0);
                        assert!(a.work >= b.work, "complete source cost must not be reduced");
                        assert!(a.comparison_work >= b.comparison_work);
                        let aliases = actual.clone();
                        let (retry, counts) = observe::run_restored(
                            &native,
                            &args,
                            &mut actual,
                            mode.execution(),
                            &memory,
                        );
                        assert!(retry.is_ok(), "restored: {retry:?}");
                        assert_eq!(completed(&counts), 1);
                        assert_eq!(outputs(&actual), outputs(&aliases));
                    }
                    assert_eq!(contents(&args), before);
                    drop((actual, expected));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    drop(args);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}

#[test]
fn numeric_binary_input_nan_falls_back_and_generated_nan_keeps_exact_payload() {
    for operator in ["+", "-", "*", "/"] {
        for artifact in artifacts(&format!("input<[f64]> {operator} peer<[f64]>\n")) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::NumericElementwise,
            )
            .unwrap();
            for input_nan in [false, true] {
                let memory = mode.memory_budget();
                let left = if input_nan {
                    vec![
                        f64::from_bits(0xfff8000000000037),
                        f64::from_bits(0x7ff0000000000042),
                        0.0,
                    ]
                } else {
                    vec![f64::INFINITY, f64::NEG_INFINITY, 0.0]
                };
                let right = vec![f64::NEG_INFINITY, f64::NEG_INFINITY, 0.0];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", data(&left), &[1, 3]),
                        ("peer", data(&right), &[1, 3]),
                    ],
                );
                let mut actual = vec![None; artifact.outputs().len()];
                let mut expected = actual.clone();
                let (a, counts) =
                    observe::run(&native, &args, &mut actual, mode.execution(), &memory);
                let (b, _) =
                    observe::run(&ordinary, &args, &mut expected, mode.execution(), &memory);
                let a = a.unwrap();
                let b = b.unwrap();
                if input_nan {
                    // A refused native attempt must leave comparison credit
                    // for the complete ordinary source, just like compute.
                    let mut tight = mode.execution();
                    tight.library.max_comparison_work = Some(a.comparison_work - 1);
                    assert!(a.comparison_work > b.comparison_work);
                    let (retry, counts) = observe::run(&native, &args, &mut actual, tight, &memory);
                    retry.unwrap();
                    assert_eq!(completed(&counts), 0);
                }
                assert_eq!(
                    outputs(&actual),
                    outputs(&expected),
                    "{operator}, input_nan={input_nan}"
                );
                assert_eq!(completed(&counts), u64::from(!input_nan));
                drop((actual, expected, args));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_binary_finite_caps_decline_without_losing_aliases_or_retry() {
    for artifact in artifacts("input<[f64]> + peer<[f64]>\n") {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericElementwise,
        )
        .unwrap();
        let memory = mode.memory_budget();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("input", data(&[1., 2., 3.]), &[1, 3]),
                ("peer", data(&[4., 5., 6.]), &[1, 3]),
            ],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        observe::run(&native, &args, &mut prior, mode.execution(), &memory)
            .0
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for field in 0..12 {
            let mut policy = mode.execution();
            match field {
                0 => policy.library.max_output_elements = Some(0),
                1 => policy.library.max_output_bytes = Some(0),
                2 => policy.library.max_temporary_bytes = Some(0),
                3 => policy.library.max_cloned_bytes = Some(0),
                4 => policy.library.max_retained_nodes = Some(0),
                5 => policy.library.max_compute_work = Some(0),
                6 => policy.library.max_comparison_work = Some(0),
                7 => policy.library.max_scalar_instructions = None,
                8 => policy.library.max_transfer_bytes = None,
                9 => policy.library.max_storage_buffer_bytes = None,
                10 => policy.library.max_storage_bindings = None,
                11 => policy.execution.max_work = 1,
                _ => unreachable!(),
            }
            let mut actual = prior.clone();
            let mut expected = prior.clone();
            let (a, counts) = observe::run(&native, &args, &mut actual, policy, &memory);
            let (b, _) = observe::run(&ordinary, &args, &mut expected, policy, &memory);
            assert_eq!(a.is_ok(), b.is_ok(), "field {field}: {a:?} / {b:?}");
            assert_eq!(completed(&counts), 0);
            assert_eq!(outputs(&actual), outputs(&expected));
            if a.is_err() {
                for (a, p) in actual.iter().zip(&prior) {
                    assert!(
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(p.as_ref().unwrap())
                    );
                }
            }
            drop((actual, expected));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(completed(&counts), 1);
        assert_eq!(outputs(&output), outputs(&prior));
        drop((output, prior, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_binary_physical_refusal_replays_original_and_releases_private_claims() {
    for artifact in artifacts("input<[f64]> + peer<[f64]>\n") {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericElementwise,
        )
        .unwrap();
        let call = native
            .descriptor()
            .metadata()
            .calls
            .iter()
            .flatten()
            .find(|call| call.template().f64_matrix_binary_op().is_some())
            .unwrap();
        let (demand, _) = call.f64_matrix_binary_source_bound(6, 3).unwrap();
        // Size this test account for the complete declared envelope. The
        // binary search still denies that peak and proves ordinary replay at
        // its actual minimum; benchmark policies remain unchanged.
        let memory = ManagedMemoryBudget::new(
            demand
                .turn_peak_bytes
                .checked_mul(2)
                .unwrap()
                .checked_add(32 * 1024 * 1024)
                .unwrap(),
        );
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[
                ("input", data(&[1., 2., 3.]), &[1, 3]),
                ("peer", data(&[4., 5., 6.]), &[1, 3]),
            ],
        );
        let mut prior = vec![None; artifact.outputs().len()];
        observe::run(&ordinary, &args, &mut prior, mode.execution(), &memory)
            .0
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let available = memory.limit_bytes() - baseline.0;
        let trial = |allowance| {
            let padding = memory.reserve_capacity(available - allowance).unwrap();
            let mut output = prior.clone();
            let result = observe::run(&ordinary, &args, &mut output, mode.execution(), &memory).0;
            if result.is_err() {
                for (a, p) in output.iter().zip(&prior) {
                    assert!(
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(p.as_ref().unwrap())
                    );
                }
            }
            drop((padding, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
            result.is_ok()
        };
        let (mut low, mut high) = (0, available);
        assert!(!trial(low) && trial(high));
        while low < high {
            let mid = low + (high - low) / 2;
            if trial(mid) {
                high = mid
            } else {
                low = mid + 1
            }
        }
        let padding = memory.reserve_capacity(available - low).unwrap();
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(completed(&counts), 0);
        assert!(
            counts
                .helpers
                .keys()
                .any(|(_, op)| *op == GlobalHelper::NumericBinaryPrepare.raw())
        );
        assert_eq!(outputs(&output), outputs(&prior));
        drop((padding, output));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            baseline
        );
        let mut output = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut output, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(completed(&counts), 1);
        drop((output, prior, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
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
        site: u64,
        op: u64,
        mut a: u64,
        mut b: u64,
        c: u64,
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
            if (3..=8).contains(&case) {
                // The current frame belongs to this synchronous invocation;
                // mutations happen after arithmetic, before checked publication.
                let frame = unsafe { &mut *(a as *mut NumericBinaryFrame) };
                match case {
                    3 => frame.output ^= 1,
                    4 => frame.left ^= 1,
                    5 => frame.columns += 1,
                    6 => frame.left_mode = 4,
                    7 => frame.right_scalar ^= 1,
                    8 => frame.count += 1,
                    _ => unreachable!(),
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            if case == 9 {
                invocation.state.consumed = true;
            }
            if case == 10 {
                invocation.state.control.previous = None;
            }
            if case == 11 {
                let visits = invocation.state.control.visits;
                assert!(invocation.visit(site).is_err());
                assert!(
                    invocation
                        .helper(site, GlobalHelper::Move.raw(), [0; 3])
                        .is_err()
                );
                assert_eq!(invocation.state.control.visits, visits);
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_binary_loans_reject_forged_frames_and_interleaving_then_retry() {
        for artifact in artifacts("input<[f64]> + peer<[f64]>\n") {
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("input", data(&[1., 2., 3.]), &[1, 3]),
                    ("peer", data(&[4., 5., 6.]), &[1, 3]),
                ],
            );
            let mut prior = vec![None; artifact.outputs().len()];
            observe::run(&jit, &args, &mut prior, mode.execution(), &memory)
                .0
                .unwrap();
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for case in 0..=12 {
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
                    for (a, p) in output.iter().zip(&prior) {
                        assert!(
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(p.as_ref().unwrap())
                        );
                    }
                } else {
                    assert_eq!(outputs(&output), outputs(&prior));
                }
                drop(output);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
                let mut output = prior.clone();
                observe::run(&jit, &args, &mut output, mode.execution(), &memory)
                    .0
                    .unwrap();
                drop(output);
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
}
