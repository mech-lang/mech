//! Ordinary admission, original/decoded/restored execution and atomic recovery.
use super::*;
use crate::native_artifact::{
    codegen::options::OptimizationPass, program::prepare::numeric_generators::banks,
};
use mech_core::snapshot::F64Bits;
fn source() -> String {
    super::numeric_generators::source("4u64").replace("]'\n(answer,image)", "]\n(answer,image)")
}
fn args(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget) -> Vec<Value> {
    numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            (
                "input",
                Data::Matrix(
                    [1., 2., 3., 4., 5., 6.]
                        .map(|v| Data::F64(F64Bits::from_f64(v)))
                        .into(),
                ),
                &[2, 3],
            ),
            ("a", Data::F64(F64Bits::from_f64(2.)), &[]),
        ],
    )
}
fn outputs(xs: &[Option<Value>]) -> Vec<Data> {
    xs.iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn finishes(counts: &observe::Counts) -> u64 {
    counts
        .helpers
        .iter()
        .filter(|((_, op), _)| *op == GlobalHelper::NumericGeneratorFinish.raw())
        .map(|(_, n)| n)
        .sum()
}
#[test]
fn numeric_generator_ordinary_original_decoded_and_restored_have_no_body_callbacks() {
    for artifact in artifacts(&source()) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericGenerators,
        )
        .unwrap();
        let selected = &native.descriptor().metadata().numeric_generators.regions;
        assert_eq!(
            selected.len(),
            1,
            "{:?}",
            native.descriptor().metadata().numeric_generators
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = args(&artifact, &memory);
        let before = args
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let mut expected = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&args, &mut expected, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            let mut actual = expected.clone();
            let (result, counts) = if restored {
                observe::run_restored(&native, &args, &mut actual, mode.execution(), &memory)
            } else {
                observe::run(&native, &args, &mut actual, mode.execution(), &memory)
            };
            result.unwrap();
            assert_eq!(outputs(&actual), outputs(&expected));
            assert_eq!(finishes(&counts), 1, "{counts:?}");
            for &site in &selected[0].region.sites {
                if site != selected[0].region.header {
                    assert!(
                        !counts.admits.contains_key(&(site as u64)),
                        "source callback {site}: {counts:?}"
                    );
                }
            }
            drop(actual);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        assert_eq!(
            args.iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>(),
            before
        );
        drop((expected, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_generator_finite_refusals_preserve_ordinary_outputs_and_retry_without_claims() {
    for artifact in artifacts(&source()) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericGenerators,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = args(&artifact, &memory);
        let mut prior = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&args, &mut prior, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let budget = &native.descriptor().metadata().numeric_generators.regions[0].budget;
        for field in 0..13 {
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
                12 => policy.library.max_compute_work = Some(budget.compute - 1),
                _ => unreachable!(),
            }
            let mut actual = prior.clone();
            let mut expected = prior.clone();
            let (a, counts) = observe::run(&native, &args, &mut actual, policy, &memory);
            let b = ordinary.evaluate_into(&args, &mut expected, policy, &memory);
            assert_eq!(a.is_ok(), b.is_ok(), "field {field}: {a:?} / {b:?}");
            assert_eq!(finishes(&counts), 0, "field {field}: {counts:?}");
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
        let mut actual = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut actual, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(finishes(&counts), 1);
        drop((actual, prior, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
#[test]
fn numeric_generator_source_bounds_miss_replays_the_original_error_without_publication() {
    let text = source().replace("a * source[at] + prior", "a * source[at + 1u64] + prior");
    for artifact in artifacts(&text) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericGenerators,
        )
        .unwrap();
        assert_eq!(
            native
                .descriptor()
                .metadata()
                .numeric_generators
                .regions
                .len(),
            1
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = args(&artifact, &memory);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        let mut actual = vec![None; artifact.outputs().len()];
        let mut expected = actual.clone();
        let (a, counts) = observe::run(&native, &args, &mut actual, mode.execution(), &memory);
        let b = ordinary.evaluate_into(&args, &mut expected, mode.execution(), &memory);
        assert!(a.is_err() && b.is_err());
        assert_eq!(
            format!("{:?}", a.unwrap_err()),
            format!("{:?}", b.unwrap_err())
        );
        assert_eq!(finishes(&counts), 1);
        assert!(actual.iter().all(Option::is_none));
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
mod loan_checks {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {
        static ORIGINAL: Cell<Option<Helper>> = const { Cell::new(None) };
        static CASE: Cell<u8> = const { Cell::new(0) };
        static CHECKED: Cell<u64> = const { Cell::new(0) };
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
        if op == GlobalHelper::NumericGeneratorPrepare.raw() && case == 14 {
            a = 1;
        }
        if op == GlobalHelper::NumericGeneratorFinish.raw() {
            CHECKED.with(|v| v.set(v.get() + 1));
            if case == 1 {
                a = 1;
            }
            if case == 2 {
                b = 2;
            }
            if (3..=11).contains(&case) {
                let frame = unsafe { &mut *(a as *mut banks::Frame) };
                match case {
                    3 => frame.output ^= 1,
                    4 => frame.scratch ^= 1,
                    5 => frame.inputs[0].input ^= 1,
                    6 => frame.inputs[0].rows += 1,
                    7 => frame.epoch += 1,
                    8 => frame.source_count += 1,
                    9 => frame.completed -= 1,
                    10 => frame.traversals[0] -= 1,
                    11 => frame.traversals[banks::MAX_LOOPS - 1] = 1,
                    _ => unreachable!(),
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            if case == 12 {
                invocation.state.control.previous = None;
            }
            if case == 13 {
                let visits = invocation.state.control.visits;
                assert!(invocation.visit(site).is_err());
                for tag in [
                    GlobalHelper::Move,
                    GlobalHelper::ClosedTailBegin,
                    GlobalHelper::HistogramPrepare,
                    GlobalHelper::ParallelRun,
                ] {
                    assert!(invocation.helper(site, tag.raw(), [0; 3]).is_err());
                }
                assert_eq!(invocation.state.control.visits, visits);
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_generator_loans_reject_forged_finish_and_interleaving_then_retry() {
        for artifact in artifacts(&source()) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let args = args(&artifact, &memory);
            let mut prior = vec![None; artifact.outputs().len()];
            observe::run(&jit, &args, &mut prior, mode.execution(), &memory)
                .0
                .unwrap();
            let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
            for case in 0..=14 {
                CASE.with(|v| v.set(case));
                CHECKED.with(|v| v.set(0));
                let mut actual = prior.clone();
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
                    &mut actual,
                    mode.execution(),
                    &memory,
                );
                assert_eq!(
                    result.is_ok(),
                    case == 0 || case == 13,
                    "case {case}: {result:?}"
                );
                assert_eq!(CHECKED.with(Cell::get), u64::from(case != 14));
                if result.is_err() {
                    for (a, p) in actual.iter().zip(&prior) {
                        assert!(
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(p.as_ref().unwrap())
                        );
                    }
                }
                drop(actual);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    baseline
                );
                let mut actual = prior.clone();
                observe::run(&jit, &args, &mut actual, mode.execution(), &memory)
                    .0
                    .unwrap();
                assert_eq!(outputs(&actual), outputs(&prior));
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
}

#[test]
fn numeric_generator_physical_byte_and_node_refusal_replays_and_releases_private_claims() {
    for artifact in artifacts(&source()) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericGenerators,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let budget = &native.descriptor().metadata().numeric_generators.regions[0].budget;
        let memory = ManagedMemoryBudget::new(
            budget
                .temporary
                .checked_mul(2)
                .unwrap()
                .checked_add(32 * 1024 * 1024)
                .unwrap(),
        );
        let args = args(&artifact, &memory);
        let mut prior = vec![None; artifact.outputs().len()];
        ordinary
            .evaluate_into(&args, &mut prior, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for dimension in 0..2 {
            let available = if dimension == 0 {
                memory.limit_bytes() - baseline.0
            } else {
                budget.nodes + 65536
            };
            let trial = |allowance| {
                let padding = if dimension == 0 {
                    Some(memory.reserve_capacity(available - allowance).unwrap())
                } else {
                    None
                };
                let capped = if dimension == 0 {
                    memory.clone()
                } else {
                    memory.with_limits(u64::MAX, baseline.1 + allowance)
                };
                let mut output = prior.clone();
                let result = ordinary.evaluate_into(&args, &mut output, mode.execution(), &capped);
                if result.is_err() {
                    for (a, p) in output.iter().zip(&prior) {
                        assert!(
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(p.as_ref().unwrap())
                        );
                    }
                }
                drop((output, padding));
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
                    high = mid;
                } else {
                    low = mid + 1;
                }
            }
            let padding = if dimension == 0 {
                Some(memory.reserve_capacity(available - low).unwrap())
            } else {
                None
            };
            let capped = if dimension == 0 {
                memory.clone()
            } else {
                memory.with_limits(u64::MAX, baseline.1 + low)
            };
            let mut actual = prior.clone();
            let (result, counts) =
                observe::run(&native, &args, &mut actual, mode.execution(), &capped);
            result.unwrap();
            assert_eq!(finishes(&counts), 0, "dimension {dimension}: {counts:?}");
            assert_eq!(outputs(&actual), outputs(&prior));
            drop((actual, padding));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        let mut actual = prior.clone();
        let (result, counts) = observe::run(&native, &args, &mut actual, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(finishes(&counts), 1);
        drop((actual, prior, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn numeric_generator_images_rederive_private_funding_and_reject_unpaid_headers() {
    use crate::native_artifact::NativeProgramDescriptor;
    for artifact in artifacts(&source()) {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let metadata = jit.descriptor().metadata();
        let selected = &metadata.numeric_generators.regions[0];
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        assert!(image["descriptor"].get("numeric_generators").is_none());
        // Exact unchanged metadata is confined to this retained compiled entry.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .unwrap();
        assert_eq!(
            serde_json::to_value(&restored.metadata().numeric_generators).unwrap(),
            serde_json::to_value(&metadata.numeric_generators).unwrap()
        );
        let mut unpaid = image;
        let header = selected.region.header as usize;
        let price = metadata.visits[header].work
            - crate::native_artifact::program::prepare::numeric_generators::HEADER_WORK;
        if let Some(bytes) = unpaid["descriptor"]["visit_work"].as_array_mut() {
            for (word, byte) in bytes[header * 8..header * 8 + 8]
                .iter_mut()
                .zip(price.to_le_bytes())
            {
                *word = byte.into();
            }
        } else {
            unpaid["descriptor"]["visits"][header]["work"] = price.into();
        }
        let encoded = serde_json::to_vec(&unpaid).unwrap();
        // Wire-only negative control: forged metadata is never executed.
        assert!(
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&encoded, &catalog) }
                .is_err()
        );
    }
}

#[test]
fn numeric_generator_complete_optimization_fixture_selects_with_fixed_benchmark_limits_and_matches_c()
 {
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/polybench/cases.json"
    ))
    .unwrap();
    let case = cases["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "deriche")
        .unwrap();
    let named = case["inputs"]
        .as_object()
        .unwrap()
        .iter()
        .map(|(name, value)| {
            let (data, extents) = if let Some(cells) = value["values"].as_array() {
                (
                    Data::Matrix(
                        cells
                            .iter()
                            .map(|v| Data::F64(F64Bits::from_f64(v.as_f64().unwrap())))
                            .collect(),
                    ),
                    value["shape"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap())
                        .collect::<Vec<_>>(),
                )
            } else if let Some(value) = value["u64"].as_u64() {
                (Data::U64(value), vec![])
            } else {
                (
                    Data::F64(F64Bits::from_f64(value.as_f64().unwrap())),
                    vec![],
                )
            };
            (name.as_str(), data, extents)
        })
        .collect::<Vec<_>>();
    for artifact in artifacts(include_str!(
        "../../../../../tests/fixtures/polybench/deriche.mec"
    )) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::NumericGenerators,
        )
        .unwrap();
        let mut policy = NativeProgramLimitMode::NoLimits.execution();
        let cap = 1_000_000_000_000_000;
        let bytes = 2 * 1024 * 1024 * 1024;
        policy.execution.max_work = cap;
        policy.execution.max_transport_bytes = bytes;
        policy.execution.max_native_stack_bytes = 64 * 1024 * 1024;
        policy.library = mech_core::MemoryBudgetLimits {
            max_output_elements: Some(100_000_000),
            max_output_bytes: Some(bytes),
            max_temporary_bytes: Some(bytes),
            max_cloned_bytes: Some(cap),
            max_retained_nodes: Some(100_000_000),
            max_comparison_work: Some(cap),
            max_compute_work: Some(cap),
            max_scalar_instructions: Some(cap),
            max_transfer_bytes: Some(cap),
            max_storage_buffer_bytes: Some(bytes),
            max_storage_bindings: Some(100_000),
        };
        let memory = ManagedMemoryBudget::new(bytes);
        let borrowed = named
            .iter()
            .map(|(n, d, e)| (*n, d.clone(), &e[..]))
            .collect::<Vec<_>>();
        let args = numeric_maps::typed_arguments(&artifact, &memory, &borrowed);
        let original = args
            .iter()
            .map(|v| v.canonical_data_draft().unwrap())
            .collect::<Vec<_>>();
        let mut expected = vec![None; artifact.outputs().len()];
        let replay = ordinary
            .evaluate_into(&args, &mut expected, policy, &memory)
            .unwrap();
        let selected = &native.descriptor().metadata().numeric_generators.regions;
        assert_eq!(selected.len(), 2);
        for region in selected {
            assert!(region.budget.replay > replay.work, "complete replay credit");
        }
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            let mut actual = vec![None; artifact.outputs().len()];
            let (result, counts) = if restored {
                observe::run_restored(&native, &args, &mut actual, policy, &memory)
            } else {
                observe::run(&native, &args, &mut actual, policy, &memory)
            };
            result.unwrap();
            assert_eq!(finishes(&counts), 2, "{counts:?}");
            for selected in selected {
                for &site in &selected.region.sites {
                    if site != selected.region.header {
                        assert!(
                            !counts.admits.contains_key(&(site as u64)),
                            "body callback {site}"
                        );
                    }
                }
            }
            assert_eq!(outputs(&actual), outputs(&expected));
            let Data::Matrix(values) = actual
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!("matrix output")
            };
            assert_eq!(values.len(), 4096);
            for (actual, expected) in values.iter().zip(case["expected"].as_array().unwrap()) {
                let Data::F64(actual) = actual else {
                    panic!("F64 output")
                };
                let actual = actual.to_f64();
                let expected = expected.as_f64().unwrap();
                assert!(
                    actual.is_finite()
                        && (actual - expected).abs() <= 1e-9 + 1e-10 * expected.abs()
                );
            }
            drop(actual);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        assert_eq!(
            args.iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>(),
            original
        );
        drop((args, expected));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
