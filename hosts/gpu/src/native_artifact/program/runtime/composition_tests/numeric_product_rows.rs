//! General ordered product row pairs preserve whole-source semantics.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
fn data(values: &[f64]) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|&v| Data::F64(F64Bits::from_f64(v)))
            .collect(),
    )
}
fn outputs(values: &[Option<Value>]) -> Vec<Data> {
    values
        .iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn run(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    observe::Counts,
) {
    if restored {
        observe::run_restored(jit, args, out, policy, memory)
    } else {
        observe::run(jit, args, out, policy, memory)
    }
}
fn oracle(a: &[f64], b: &[f64], rows: u64, inner: u64, columns: u64) -> Data {
    let mut cells = Vec::new();
    if rows * columns != 0 {
        for row in 0..rows {
            for column in 0..columns {
                let mut sum = 0.0f64;
                for k in 0..inner {
                    let term = std::hint::black_box(
                        a[(row * inner + k) as usize] * b[(k * columns + column) as usize],
                    );
                    sum = std::hint::black_box(sum + term);
                }
                cells.push(Data::F64(F64Bits::from_f64(sum)));
            }
        }
    }
    Data::Matrix(cells.into())
}
#[test]
fn numeric_product_rows_dynamic_fixed_rectangles_vectors_empty_axes_bits_oracle_and_aliases() {
    let huge = u32::MAX as u64;
    for (rows, inner, columns) in [
        (0, 0, 0),
        (0, 7, 5),
        (3, 0, 5),
        (3, 7, 0),
        (1, 1, 1),
        (1, 5, 7),
        (7, 5, 1),
        (2, 3, 2),
        (3, 5, 7),
        (5, 7, 3),
        (8, 9, 6),
        (0, huge, 0),
        (huge, 0, 0),
        (0, 0, huge),
    ] {
        for fixed in [false, true] {
            let axes = |r, c| {
                if fixed {
                    format!(":{r},{c}")
                } else {
                    String::new()
                }
            };
            let source = format!(
                "a := left<[f64]{}>\nb := right<[f64]{}>\n(a ** b,a,b)\n",
                axes(rows, inner),
                axes(inner, columns)
            );
            for artifact in artifacts(&source) {
                let catalog = mech_stdlib::source_catalog();
                let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
                let old = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::ProductRowPairs,
                )
                .unwrap();
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let words = [1e16, 1.0, -1e16, -0.0, 0.0, 2.5, -3.25];
                let a = (0..rows * inner)
                    .map(|i| words[i as usize % words.len()])
                    .collect::<Vec<_>>();
                let b = (0..inner * columns)
                    .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
                    .collect::<Vec<_>>();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", data(&a), &[rows, inner]),
                        ("right", data(&b), &[inner, columns]),
                    ],
                );
                let aliases = args.clone();
                let before = contents(&args);
                let expected = Data::Tuple(
                    vec![oracle(&a, &b, rows, inner, columns), data(&a), data(&b)].into(),
                );
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut previous = actual.clone();
                    let (a, counts) = run(
                        &native,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    let (b, old_counts) = run(
                        &old,
                        &args,
                        &mut previous,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    assert_eq!(
                        a.as_ref().map_err(|e| format!("{e:?}")),
                        b.as_ref().map_err(|e| format!("{e:?}")),
                        "{rows}x{inner}x{columns},fixed{fixed}"
                    );
                    a.unwrap();
                    assert_eq!(counts.helpers, old_counts.helpers);
                    assert!(counts.helpers.iter().any(|((_, op), &n)| *op
                        == GlobalHelper::NumericBinaryFinish.raw()
                        && n > 0));
                    assert_eq!(
                        outputs(&actual),
                        vec![expected.clone()],
                        "{rows}x{inner}x{columns},fixed{fixed}"
                    );
                    assert_eq!(outputs(&actual), outputs(&previous));
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                    drop((actual, previous));
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
#[test]
fn numeric_product_rows_epilogues_broadcasts_and_nonfinite_cancellation_match_old_emitter() {
    for shape in [None, Some([1, 1]), Some([1, 7]), Some([5, 1]), Some([5, 7])] {
        let kind = if shape.is_some() { "[f64]" } else { "f64" };
        let source = format!("alpha<f64> * (left<[f64]> ** right<[f64]>) + bias<{kind}>\n");
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ProductRowPairs,
            )
            .unwrap();
            for overflow in [false, true] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let a = vec![if overflow { f64::MAX } else { 0.75 }; 15];
                let b = vec![2.0; 21];
                let bias = match shape {
                    Some([r, c]) => data(&vec![1.25; (r * c) as usize]),
                    None => Data::F64(F64Bits::from_f64(1.25)),
                };
                let extents = shape.map(|s| s.to_vec()).unwrap_or_default();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("left", data(&a), &[5, 3]),
                        ("right", data(&b), &[3, 7]),
                        (
                            "alpha",
                            Data::F64(F64Bits::from_f64(if overflow { 0.0 } else { 0.25 })),
                            &[],
                        ),
                        ("bias", bias, &extents),
                    ],
                );
                let before = contents(&args);
                let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                for restored in [false, true] {
                    let mut actual = vec![None; artifact.outputs().len()];
                    let mut previous = actual.clone();
                    let (a, counts) = run(
                        &native,
                        &args,
                        &mut actual,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    let (b, old_counts) = run(
                        &old,
                        &args,
                        &mut previous,
                        mode.execution(),
                        &memory,
                        restored,
                    );
                    assert_eq!(a.unwrap(), b.unwrap());
                    assert!(
                        !counts.product_epilogues.is_empty(),
                        "real fused product: {counts:?}"
                    );
                    assert_eq!(counts.helpers, old_counts.helpers);
                    assert_eq!(outputs(&actual), outputs(&previous));
                    assert_eq!(contents(&args), before);
                    drop((actual, previous));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
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
fn numeric_product_rows_finite_missing_caps_atomic_failure_and_retry_match_old_emitter() {
    for artifact in artifacts("left<[f64]> ** right<[f64]>\n") {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let old = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ProductRowPairs,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let make = |v| {
            numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("left", data(&vec![v; 15]), &[3, 5]),
                    ("right", data(&vec![1.0; 15]), &[5, 3]),
                ],
            )
        };
        let args = make(1.5);
        let previous_args = make(2.5);
        let mut prior = vec![None; artifact.outputs().len()];
        run(
            &old,
            &previous_args,
            &mut prior,
            mode.execution(),
            &memory,
            false,
        )
        .0
        .unwrap();
        drop(previous_args);
        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
        for restored in [false, true] {
            for dimension in 0..12 {
                for cap in [None, Some(0), Some(1), Some(4096), Some(u64::MAX)] {
                    let mut policy = mode.execution();
                    match dimension {
                        0 => policy.library.max_output_elements = cap,
                        1 => policy.library.max_output_bytes = cap,
                        2 => policy.library.max_temporary_bytes = cap,
                        3 => policy.library.max_cloned_bytes = cap,
                        4 => policy.library.max_retained_nodes = cap,
                        5 => policy.library.max_compute_work = cap,
                        6 => policy.library.max_comparison_work = cap,
                        7 => policy.library.max_scalar_instructions = cap,
                        8 => policy.library.max_transfer_bytes = cap,
                        9 => policy.library.max_storage_buffer_bytes = cap,
                        10 => {
                            policy.library.max_storage_bindings =
                                cap.map(|n| n.min(u32::MAX as u64) as u32)
                        }
                        _ => policy.execution.max_work = cap.unwrap_or(u64::MAX),
                    }
                    let mut actual = prior.clone();
                    let mut previous = prior.clone();
                    let (a, _) = run(&native, &args, &mut actual, policy, &memory, restored);
                    let (b, _) = run(&old, &args, &mut previous, policy, &memory, restored);
                    assert_eq!(a.is_ok(), b.is_ok(), "{dimension} {cap:?}: {a:?}/{b:?}");
                    if let (Ok(a), Ok(b)) = (&a, &b) {
                        assert_eq!(a, b);
                    }
                    assert_eq!(outputs(&actual), outputs(&previous));
                    if a.is_err() {
                        for (a, p) in actual.iter().zip(&prior) {
                            assert!(
                                a.as_ref()
                                    .unwrap()
                                    .shares_frozen_storage(p.as_ref().unwrap())
                            );
                        }
                    }
                    drop((actual, previous));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        baseline
                    );
                }
            }
            let mut retry = prior.clone();
            run(
                &native,
                &args,
                &mut retry,
                mode.execution(),
                &memory,
                restored,
            )
            .0
            .unwrap();
            assert_ne!(outputs(&retry), outputs(&prior));
            drop(retry);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                baseline
            );
        }
        drop((args, prior));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
