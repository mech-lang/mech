//! Real checked source closures, final-only banks and ordered bits on both axes.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
const EXPR: &str =
    "+> stats\nstats/sum/row((a<[f64]> * a + scale<f64> * b<[f64]>) / denominator<f64>)\n";
fn matrix(words: &[u64], shape: [u64; 2]) -> Data {
    Data::Matrix(
        (0..shape[0] * shape[1])
            .map(|i| Data::F64(F64Bits::from_bits(words[i as usize % words.len()])))
            .collect(),
    )
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    shape: [u64; 2],
    extra: [u64; 2],
    words: &[u64],
) -> Vec<Value> {
    numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("a", matrix(words, shape), &shape),
            ("b", matrix(words, extra), &extra),
            ("scale", Data::F64(F64Bits::from_f64(-0.75)), &[]),
            ("denominator", Data::F64(F64Bits::from_f64(1.25)), &[]),
        ],
    )
}
fn data(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn recipes(
    jit: &JitProgramArtifact,
) -> Vec<crate::native_artifact::program::prepare::numeric_product_epilogues::Candidate> {
    let m = jit.descriptor().metadata();
    (0..m.plan.blocks().len())
        .filter_map(|s| {
            m.numeric_binary
                .epilogue(s)
                .filter(|r| r.reduction_axis().is_some())
                .cloned()
        })
        .collect()
}
#[test]
fn numeric_reduction_producers_source_closures_and_actual_objects() {
    for (source, axis) in [
        ("+> stats\nstats/sum/row(a<[f64]> * a)\n", 0),
        (
            "+> stats\np := a<[f64]> * b<[f64]>\nstats/sum/column(p)\n",
            1,
        ),
        (EXPR, 0),
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let r = recipes(&jit);
            assert_eq!(r.len(), 1, "{source}");
            assert_eq!(r[0].reduction_axis(), Some(axis));
            assert!(r[0].nodes[..r[0].nodes.len() - 1].iter().all(|n| matches!(
                n.kind,
                crate::native_artifact::program::prepare::numeric_binary::Kind::Elementwise(_)
            )));
            let object = crate::native_artifact::emit_program_object(
                &artifact,
                &mech_stdlib::source_catalog(),
            )
            .unwrap();
            assert_eq!(
                object.metadata()["compiled_reduction_producers"]
                    .as_array()
                    .unwrap()
                    .len(),
                1
            );
            assert_eq!(
                object.metadata()["compiled_reduction_producers"][0]["axis"],
                axis
            );
            assert!(
                object.metadata()["compiled_product_epilogues"]
                    .as_array()
                    .unwrap()
                    .is_empty()
            );
        }
    }
    let source = std::fs::read_to_string(format!(
        "{}/tests/fixtures/polybench/correlation.mec",
        env!("CARGO_MANIFEST_DIR")
    ))
    .unwrap();
    for artifact in artifacts(&source) {
        let object =
            crate::native_artifact::emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap();
        assert_eq!(
            object.metadata()["compiled_reduction_producers"]
                .as_array()
                .unwrap()
                .len(),
            1
        );
    }
}
#[test]
fn numeric_reduction_producers_decline_escaping_or_nonpointwise_closures() {
    for source in [
        "+> stats\np := a<[f64]> * a\n(p,stats/sum/row(p))\n",
        "+> stats\np := a<[f64]> * a\n(stats/sum/row(p),stats/sum/column(p))\n",
        "+> stats\nstats/sum/row(a<[f64]> ** b<[f64]>)\n",
        "+> stats\nstats/sum/row((a<[f64]> * a)')\n",
        "a<[f64]> * a\n",
    ] {
        for artifact in artifacts(source) {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(recipes(&jit).is_empty(), "{source}");
        }
    }
}
#[test]
fn numeric_reduction_producers_ordered_axes_broadcasts_bits_restored_and_final_only_banks() {
    let words = [
        0.0f64.to_bits(),
        (-0.0f64).to_bits(),
        1.25f64.to_bits(),
        (-0.5f64).to_bits(),
        1,
        0x0010000000000000,
        1e8f64.to_bits(),
        (-1e8f64).to_bits(),
    ];
    for axis in 0..2 {
        let source = if axis == 0 {
            EXPR.to_string()
        } else {
            EXPR.replace("sum/row", "sum/column")
        };
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ReductionProducers,
            )
            .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::NumericMatrixArithmetic,
            )
            .unwrap();
            let recipe = recipes(&native).pop().unwrap();
            for shape in [[3, 5], [1, 5], [3, 1], [1, 1], [0, 5], [3, 0], [0, 0]] {
                for extra in [shape, [1, shape[1]], [shape[0], 1], [1, 1]] {
                    let mode = NativeProgramLimitMode::NoLimits;
                    let memory = mode.memory_budget();
                    let args = arguments(&artifact, &memory, shape, extra, &words);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let get = |shape: [u64; 2], r: u64, c: u64| {
                        f64::from_bits(
                            words[((if shape[0] == 1 { 0 } else { r }) * shape[1]
                                + if shape[1] == 1 { 0 } else { c })
                                as usize
                                % words.len()],
                        )
                    };
                    let count = if axis == 0 { shape[1] } else { shape[0] };
                    let inner = if axis == 0 { shape[0] } else { shape[1] };
                    let expected = Data::Matrix(
                        (0..count)
                            .map(|out| {
                                let mut acc = 0.0;
                                for k in 0..inner {
                                    let (r, c) = if axis == 0 { (k, out) } else { (out, k) };
                                    let a = get(shape, r, c);
                                    let b = get(extra, r, c);
                                    let square = std::hint::black_box(a * a);
                                    let scaled = std::hint::black_box(-0.75 * b);
                                    let added = std::hint::black_box(square + scaled);
                                    let term = std::hint::black_box(added / 1.25);
                                    acc = std::hint::black_box(acc + term);
                                }
                                Data::F64(F64Bits::from_f64(acc))
                            })
                            .collect(),
                    );
                    let mut host = vec![None; artifact.outputs().len()];
                    ordinary
                        .evaluate_into(&args, &mut host, mode.execution(), &memory)
                        .unwrap();
                    assert_eq!(
                        data(&host),
                        vec![expected.clone()],
                        "oracle {shape:?}/{extra:?},axis{axis}"
                    );
                    drop(host);
                    for restored in [false, true] {
                        let mut reference = vec![None; artifact.outputs().len()];
                        let reference_report = old
                            .evaluate_into(&args, &mut reference, mode.execution(), &memory)
                            .unwrap();
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
                        assert_eq!(data(&output), vec![expected.clone()]);
                        assert_eq!(data(&output), data(&reference));
                        assert_eq!(
                            counts.product_epilogues,
                            vec![(
                                recipe.entry as usize,
                                recipe.nodes.len(),
                                count as usize,
                                false
                            )],
                            "{shape:?}/{extra:?},axis{axis}: {:?}",
                            counts.helpers
                        );
                        for step in recipe.steps.iter().skip(1) {
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
                        assert!(report.work >= reference_report.work);
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
                    drop((args, aliases));
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
fn numeric_reduction_producers_nonfinite_replay_and_ordered_accumulator_overflow() {
    for axis in ["row", "column"] {
        let source = format!("+> stats\nstats/sum/{axis}(a<[f64]> * scale<f64>)\n");
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ReductionProducers,
            )
            .unwrap();
            for (words, scale) in [
                (vec![0, (-0.0f64).to_bits(), 1, 0x0010000000000000], 1.0),
                (
                    vec![
                        0x7ff8000000000042,
                        0x7ff0000000000042,
                        0xfff8000000000055,
                        1.0f64.to_bits(),
                    ],
                    2.0,
                ),
                (
                    vec![
                        f64::INFINITY.to_bits(),
                        f64::NEG_INFINITY.to_bits(),
                        (-0.0f64).to_bits(),
                    ],
                    0.0,
                ),
                (vec![1e308f64.to_bits()], 1e308),
                (vec![1e308f64.to_bits()], 1.0),
                (
                    vec![1e16f64.to_bits(), 1.0f64.to_bits(), (-1e16f64).to_bits()],
                    1.0,
                ),
            ] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let shape = [3, 5];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("a", matrix(&words, shape), &shape),
                        ("scale", Data::F64(F64Bits::from_f64(scale)), &[]),
                    ],
                );
                let before = contents(&args);
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut reference = vec![None; artifact.outputs().len()];
                old.evaluate_into(&args, &mut reference, mode.execution(), &memory)
                    .unwrap();
                for restored in [false, true] {
                    let mut output = vec![None; artifact.outputs().len()];
                    let (r, counts) = if restored {
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
                    r.unwrap();
                    assert_eq!(
                        data(&output),
                        data(&reference),
                        "{axis}, words{words:x?},scale{scale}"
                    );
                    assert!(counts.product_epilogues.len() <= 1);
                    assert_eq!(contents(&args), before);
                    drop(output);
                }
                drop(reference);
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
#[test]
fn numeric_reduction_producers_caps_empty_source_obligations_physical_cleanup_and_retry() {
    for artifact in artifacts(EXPR) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let old = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ReductionProducers,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = arguments(&artifact, &memory, [3, 5], [1, 5], &[1.25f64.to_bits()]);
        let before = contents(&args);
        let mut prior = vec![None; artifact.outputs().len()];
        let full = native
            .evaluate_into(&args, &mut prior, mode.execution(), &memory)
            .unwrap();
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        for case in 0..20 {
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
                18 => policy.execution.max_work = 1,
                _ => policy.library.max_comparison_work = Some(full.comparison_work - 1),
            }
            for restored in [false, true] {
                let mut output = prior.clone();
                let mut reference = prior.clone();
                let (a, _) = if restored {
                    observe::run_restored(&native, &args, &mut output, policy, &memory)
                } else {
                    observe::run(&native, &args, &mut output, policy, &memory)
                };
                let b = old.evaluate_into(&args, &mut reference, policy, &memory);
                assert_eq!(a.is_ok(), b.is_ok(), "cap{case}: {a:?}/{b:?}");
                if a.is_err() {
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                } else {
                    assert_eq!(data(&output), data(&prior));
                }
                drop((output, reference));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
        }
        // The reduction's small output cannot erase its larger source matrix.
        let mut policy = mode.execution();
        policy.library.max_output_elements = Some(5);
        let mut output = prior.clone();
        let mut reference = prior.clone();
        let (a, counts) = observe::run(&native, &args, &mut output, policy, &memory);
        let b = old.evaluate_into(&args, &mut reference, policy, &memory);
        assert!(a.is_err() && b.is_err());
        assert!(counts.product_epilogues.is_empty());
        drop((output, reference));
        // A nonempty scalar-shaped producer remains a source obligation even
        // when broadcasting it into an empty domain leaves the final sum empty.
        let empty = arguments(&artifact, &memory, [0, 0], [1, 1], &[1.25f64.to_bits()]);
        let empty_baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut unlimited = vec![None; artifact.outputs().len()];
        let (result, counts) =
            observe::run_restored(&native, &empty, &mut unlimited, mode.execution(), &memory);
        result.unwrap();
        assert_eq!(counts.product_epilogues.len(), 1);
        drop(unlimited);
        let mut capped = mode.execution();
        capped.library.max_output_elements = Some(0);
        for restored in [false, true] {
            let mut output = prior.clone();
            let mut reference = prior.clone();
            let (a, counts) = if restored {
                observe::run_restored(&native, &empty, &mut output, capped, &memory)
            } else {
                observe::run(&native, &empty, &mut output, capped, &memory)
            };
            let b = old.evaluate_into(&empty, &mut reference, capped, &memory);
            assert!(
                a.is_err() && b.is_err(),
                "empty final domain erased a nonempty producer"
            );
            assert!(counts.product_epilogues.is_empty());
            assert!(output.iter().zip(&prior).all(|(a, b)| {
                a.as_ref()
                    .unwrap()
                    .shares_frozen_storage(b.as_ref().unwrap())
            }));
            drop((output, reference));
            assert_eq!(
                (memory.used_bytes(), memory.retained_nodes()),
                empty_baseline
            );
        }
        drop(empty);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        for dimension in ["bytes", "nodes"] {
            let mut denied = 0;
            let mut success = 0;
            for allowance in [0, 16, 256, 4096, 65536, 1_048_576, 1_000_000_000_000] {
                let bounded = if dimension == "bytes" {
                    memory.with_limits(baseline.0 + allowance, memory.limit_nodes())
                } else {
                    memory.with_limits(memory.limit_bytes(), baseline.1 + allowance)
                };
                let mut output = prior.clone();
                let (r, _) =
                    observe::run_restored(&native, &args, &mut output, mode.execution(), &bounded);
                if r.is_ok() {
                    success += 1;
                    assert_eq!(data(&output), data(&prior));
                } else {
                    denied += 1;
                    assert!(output.iter().zip(&prior).all(|(a, b)| {
                        a.as_ref()
                            .unwrap()
                            .shares_frozen_storage(b.as_ref().unwrap())
                    }));
                }
                drop(output);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                assert_eq!(contents(&args), before);
            }
            assert!(denied > 0 && success > 0, "{dimension}: {denied}/{success}");
        }
        let mut retry = prior.clone();
        let (r, counts) =
            observe::run_restored(&native, &args, &mut retry, mode.execution(), &memory);
        r.unwrap();
        assert_eq!(counts.product_epilogues.len(), 1);
        drop((retry, prior, args));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
#[test]
fn numeric_reduction_producers_shape_first_errors_atomic_aliases_and_retry() {
    for artifact in artifacts(EXPR) {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let old = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ReductionProducers,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let good = arguments(&artifact, &memory, [3, 5], [1, 5], &[1.25f64.to_bits()]);
        let mut prior = vec![None; artifact.outputs().len()];
        native
            .evaluate_into(&good, &mut prior, mode.execution(), &memory)
            .unwrap();
        for extra in [[2, 5], [3, 4], [4, 2]] {
            let args = arguments(&artifact, &memory, [3, 5], extra, &[1.25f64.to_bits()]);
            let aliases = args.clone();
            let before = contents(&args);
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            for restored in [false, true] {
                let mut output = prior.clone();
                let mut reference = prior.clone();
                let (a, counts) = if restored {
                    observe::run_restored(&native, &args, &mut output, mode.execution(), &memory)
                } else {
                    observe::run(&native, &args, &mut output, mode.execution(), &memory)
                };
                let b = old.evaluate_into(&args, &mut reference, mode.execution(), &memory);
                assert_eq!(
                    format!("{:?}", a.unwrap_err()),
                    format!("{:?}", b.unwrap_err())
                );
                assert!(counts.product_epilogues.is_empty());
                assert!(output.iter().zip(&prior).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                drop((output, reference));
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
                assert_eq!(contents(&aliases), before);
            }
            drop((args, aliases));
        }
        let mut retry = prior.clone();
        let (r, counts) =
            observe::run_restored(&native, &good, &mut retry, mode.execution(), &memory);
        r.unwrap();
        assert_eq!(counts.product_epilogues.len(), 1);
        drop((retry, prior, good));
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
#[test]
fn numeric_reduction_producers_images_reconstruct_source_recipes_and_reject_authority() {
    let source =
        "+> stats\nstats/sum/row(alpha<f64>*a<[f64]>+beta<f64>*b<[f64]>+gamma<f64>*c<[f64]>)\n";
    let catalog = mech_stdlib::source_catalog();
    for artifact in artifacts(source) {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let original = recipes(&jit);
        assert_eq!(original.len(), 1);
        assert_eq!(
            jit.descriptor()
                .metadata()
                .numeric_binary
                .epilogue_arguments(),
            3
        );
        let encoded = jit.descriptor().encode_build_owned_image().unwrap();
        for wire in [
            encoded.clone(),
            crate::native_artifact::program::image::packing::pack(&encoded).unwrap(),
        ] {
            // SAFETY: exact metadata emitted by this checked JIT; no fabricated code entry.
            let decoded =
                unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                    .unwrap();
            let m = decoded.metadata();
            let restored = (0..m.plan.blocks().len())
                .filter_map(|s| {
                    m.numeric_binary
                        .epilogue(s)
                        .filter(|r| r.reduction_axis().is_some())
                        .cloned()
                })
                .collect::<Vec<_>>();
            assert_eq!(restored, original);
            assert_eq!(decoded.encode_build_owned_image().unwrap(), encoded);
        }
        let image: serde_json::Value = serde_json::from_slice(&encoded).unwrap();
        for case in 0..3 {
            let mut changed = image.clone();
            match case {
                0 => {
                    changed["descriptor"]["numeric_reduction_producers"] =
                        serde_json::json!({"owns_source":true,"fees_paid":true})
                }
                1 => changed["descriptor"]["plan"] = serde_json::Value::Null,
                _ => changed["descriptor"]["max_library_inputs"] = 2.into(),
            };
            let bytes = serde_json::to_vec(&changed).unwrap();
            for wire in [
                bytes.clone(),
                crate::native_artifact::program::image::packing::pack(&bytes).unwrap(),
            ] {
                // SAFETY: metadata-only negative decode; forged metadata is never bound/executed.
                assert!(
                    unsafe { NativeProgramDescriptor::decode_build_owned_image(&wire, &catalog) }
                        .is_err(),
                    "forgery{case}"
                );
            }
        }
    }
}
mod loans {
    use super::*;
    use crate::native_artifact::program::protocol::abi::{Frame, Helper, NumericProductFrame};
    use std::{cell::Cell, ffi::c_void};
    thread_local! {static ORIGINAL:Cell<Option<Helper>>=const{Cell::new(None)};static CASE:Cell<u8>=const{Cell::new(0)};static FINISHES:Cell<u64>=const{Cell::new(0)};}
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
        if op == GlobalHelper::NumericProductPrepare.raw() && case == 15 {
            a ^= 1;
        }
        if op == GlobalHelper::NumericProductFinish.raw() {
            FINISHES.with(|n| n.set(n.get() + 1));
            match case {
                1 => a = 1,
                2 => b += 1,
                3 => site += 1,
                4 => c = 2,
                _ => {}
            }
            if (5..=10).contains(&case) {
                // Only finish sees the forgery; arithmetic has already completed.
                let frame = unsafe { &mut *(a as *mut NumericProductFrame) };
                match case {
                    5 => frame.product.output ^= 1,
                    6 => frame.product.inner += 1,
                    7 => frame.product.count += 1,
                    8 => frame.leaves[0].pointer ^= 1,
                    9 => frame.leaves[1].mode = 4,
                    10 => frame.leaves[7].scalar ^= 1,
                    _ => {}
                }
            }
            let invocation = unsafe { &mut *context.cast::<Invocation<'_, '_, '_>>() };
            match case {
                11 => invocation.state.control.visits += 1,
                12 => invocation.state.control.current = Some(u32::MAX),
                13 => invocation.state.consumed = true,
                14 => invocation.state.control.previous = None,
                16 => {
                    let visits = invocation.state.control.visits;
                    assert!(invocation.visit(site).is_err());
                    assert!(
                        invocation
                            .helper(site, GlobalHelper::Move.raw(), [0; 3])
                            .is_err()
                    );
                    assert_eq!(visits, invocation.state.control.visits);
                }
                _ => {}
            }
        }
        unsafe { ORIGINAL.with(Cell::get).unwrap()(context, site, op, a, b, c, out) }
    }
    #[test]
    fn numeric_reduction_producers_sealed_frames_epochs_context_interleaving_and_retry() {
        for axis in ["row", "column"] {
            let source = EXPR.replace("sum/row", &format!("sum/{axis}"));
            for artifact in artifacts(&source) {
                let jit =
                    JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let args = arguments(&artifact, &memory, [3, 5], [1, 5], &[1.25f64.to_bits()]);
                let before = contents(&args);
                let mut prior = vec![None; artifact.outputs().len()];
                jit.evaluate_into(&args, &mut prior, mode.execution(), &memory)
                    .unwrap();
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for case in 0..=16 {
                    CASE.with(|v| v.set(case));
                    FINISHES.with(|v| v.set(0));
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
                        case == 0 || case == 16,
                        "{axis}/case{case}: {result:?}"
                    );
                    assert_eq!(FINISHES.with(Cell::get), u64::from(case != 15));
                    if result.is_err() {
                        assert!(output.iter().zip(&prior).all(|(a, b)| {
                            a.as_ref()
                                .unwrap()
                                .shares_frozen_storage(b.as_ref().unwrap())
                        }));
                    } else {
                        assert_eq!(data(&output), data(&prior));
                    }
                    drop(output);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    let mut retry = prior.clone();
                    let (r, counts) =
                        observe::run_restored(&jit, &args, &mut retry, mode.execution(), &memory);
                    r.unwrap();
                    assert_eq!(counts.product_epilogues.len(), 1);
                    drop(retry);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                    assert_eq!(contents(&args), before);
                }
                drop((args, prior));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
