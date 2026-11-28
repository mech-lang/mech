//! Reuse only an already successful immutable read on the same source path.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;
fn data(xs: &[f64]) -> Data {
    Data::Matrix(
        xs.iter()
            .map(|&v| Data::F64(F64Bits::from_f64(v)))
            .collect(),
    )
}
fn outputs(xs: &[Option<Value>]) -> Vec<Data> {
    xs.iter()
        .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
        .collect()
}
fn reuses(value: &serde_json::Value) -> Vec<[u32; 3]> {
    match value {
        serde_json::Value::Object(fields) => fields
            .iter()
            .flat_map(|(k, v)| {
                if k == "map_read_reuses" {
                    v.as_array()
                        .unwrap()
                        .iter()
                        .map(|row| std::array::from_fn(|i| row[i].as_u64().unwrap() as u32))
                        .collect()
                } else {
                    reuses(v)
                }
            })
            .collect(),
        serde_json::Value::Array(values) => values.iter().flat_map(reuses).collect(),
        _ => Vec::new(),
    }
}
const PHASE: &str = r#"#Square(initial<[f64]:2,3>, rounds<u64>) => <[f64]:2,3>
  | :Step(a<[f64]:2,3>,k<u64>)
  | :Done(a<[f64]:2,3>).
#Square(initial,rounds) -> :Step(initial,0u64)
  :Step(a,k)
    | k < rounds -> {
        next := [[a[i,j] * a[i,j] | i <- 1u64..=2u64]' | j <- 1u64..=3u64]
      } -> :Step(next,k + 1u64)
    | * -> :Done(a)
  :Done(a) => a.
#Square(input<[f64]:2,3>,rounds<u64>)
"#;
#[test]
fn numeric_map_read_reuse_actual_phase_and_square_reduction_objects_show_successful_source_pairs() {
    let cases = [
        PHASE,
        include_str!("../../../../../tests/fixtures/polybench/gramschmidt.mec"),
        include_str!("../../../../../tests/fixtures/polybench/cholesky.mec"),
    ];
    for source in cases {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let old = crate::native_artifact::program::compiler::emit_program_object_for_test(
                &artifact,
                &catalog,
                OptimizationPass::RepeatedMatrixReads,
            )
            .unwrap();
            let records = reuses(&object.metadata());
            assert!(
                !records.is_empty(),
                "expected actual repeated checked reads: {}",
                object.metadata()
            );
            assert!(records.iter().all(|[_, first, next]| first != next));
            assert!(reuses(&old.metadata()).is_empty());
        }
    }
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
#[test]
fn numeric_map_read_reuse_packed_kinds_bits_range_rectangle_phase_and_nested_folds_match_ablation()
{
    let fp = [
        0,
        1u64 << 63,
        1,
        0x7ff8_0000_0000_0042,
        0x7ff0_0000_0000_0031,
        f64::INFINITY.to_bits(),
        f64::NEG_INFINITY.to_bits(),
    ];
    let kinds = vec![
        (
            "bool",
            Data::Matrix(
                vec![
                    Data::Bool(true),
                    Data::Bool(false),
                    Data::Bool(true),
                    Data::Bool(false),
                    Data::Bool(false),
                    Data::Bool(true),
                ]
                .into(),
            ),
            "==",
        ),
        ("u8", Data::Matrix((1..=6).map(Data::U8).collect()), "+"),
        ("u16", Data::Matrix((1..=6).map(Data::U16).collect()), "+"),
        ("u32", Data::Matrix((1..=6).map(Data::U32).collect()), "+"),
        ("u64", Data::Matrix((1..=6).map(Data::U64).collect()), "+"),
        ("i64", Data::Matrix((-3..3).map(Data::I64).collect()), "+"),
        (
            "f64",
            Data::Matrix(
                fp.iter()
                    .take(6)
                    .map(|&b| Data::F64(F64Bits::from_bits(b)))
                    .collect(),
            ),
            "*",
        ),
    ];
    for (kind, input, op) in kinds {
        let source = format!("a := input<[{kind}]>\n[a[i] {op} a[i] | i <- 1u64..=6u64]\n");
        for artifact in artifacts(&source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::RepeatedMatrixReads,
            )
            .unwrap();
            for shape in [[1, 6], [6, 1], [2, 3]] {
                let mode = NativeProgramLimitMode::NoLimits;
                let memory = mode.memory_budget();
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[("input", input.clone(), &shape)],
                );
                let aliases = args.clone();
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
                    assert_eq!(counts.helpers, old_counts.helpers);
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
    for source in [
        PHASE,
        "+> stats\na := input<[f64]:2,3>\n[stats/sum/column([a[i,j] * a[i,j] | i <- 1u64..=2u64])[1u64] | j <- 1u64..=3u64]\n",
    ] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::RepeatedMatrixReads,
            )
            .unwrap();
            let mode = NativeProgramLimitMode::NoLimits;
            let memory = mode.memory_budget();
            let input = data(&[0.0, -0.0, 1.5, -2.0, 3.0, 0.25]);
            let args = numeric_maps::typed_arguments(
                &artifact,
                &memory,
                &[
                    ("input", input.clone(), &[2, 3]),
                    ("rounds", Data::U64(3), &[]),
                ],
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
                assert_eq!(a.unwrap(), b.unwrap());
                assert_eq!(counts.helpers, old_counts.helpers);
                assert_eq!(outputs(&actual), outputs(&previous));
                assert_eq!(contents(&args).first(), Some(&input));
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
#[test]
fn numeric_map_read_reuse_branch_joins_distinct_selectors_invalid_bounds_keep_first_error_and_atomicity()
 {
    for source in [
        "a := input<[f64]>\n[((choose<bool>)? | true => a[i] | false => 0.0) + a[i] | i <- 1u64..=6u64]\n",
        "a := input<[f64]>\n[a[i] + a[i + 1u64] | i <- 1u64..=6u64]\n",
        "a := input<[f64]>\n[a[index<u64>] + a[index] | i <- 1u64..=6u64]\n",
    ] {
        for artifact in artifacts(source) {
            let catalog = mech_stdlib::source_catalog();
            let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::RepeatedMatrixReads,
            )
            .unwrap();
            for choose in [false, true] {
                for index in [0, 1, 6, u64::MAX] {
                    for size in [5, 7] {
                        let mode = NativeProgramLimitMode::NoLimits;
                        let memory = mode.memory_budget();
                        let input = data(&vec![1.25; size]);
                        let args = numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("input", input, &[1, size as u64]),
                                ("choose", Data::Bool(choose), &[]),
                                ("index", Data::U64(index), &[]),
                            ],
                        );
                        let baseline = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        for restored in [false, true] {
                            let mut actual = vec![None; artifact.outputs().len()];
                            let mut previous = actual.clone();
                            let (a, _) = run(
                                &native,
                                &args,
                                &mut actual,
                                mode.execution(),
                                &memory,
                                restored,
                            );
                            let (b, _) = run(
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
                                "{source},choose{choose},index{index},size{size}"
                            );
                            assert_eq!(
                                actual.iter().filter(|v| v.is_some()).count(),
                                previous.iter().filter(|v| v.is_some()).count()
                            );
                            if a.is_ok() {
                                assert_eq!(outputs(&actual), outputs(&previous));
                            } else {
                                assert!(actual.iter().all(Option::is_none));
                            }
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
    }
}
#[test]
fn numeric_map_read_reuse_finite_caps_atomic_prior_aliases_cleanup_and_retry_match_ablation() {
    for artifact in artifacts("a := input<[f64]>\n[a[i] * a[i] | i <- 1u64..=6u64]\n") {
        let catalog = mech_stdlib::source_catalog();
        let native = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let old = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::RepeatedMatrixReads,
        )
        .unwrap();
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[("input", data(&[1.5; 6]), &[1, 6])],
        );
        let previous_args = numeric_maps::typed_arguments(
            &artifact,
            &memory,
            &[("input", data(&[2.5; 6]), &[1, 6])],
        );
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
