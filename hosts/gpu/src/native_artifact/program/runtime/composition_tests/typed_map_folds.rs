//! Exact old-word/default comparisons for ordered F64 accumulator transport.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;

fn matrix(bits: &[u64]) -> Data {
    Data::Matrix(
        bits.iter()
            .map(|&b| Data::F64(F64Bits::from_bits(b)))
            .collect(),
    )
}
fn output(values: &[Option<Value>]) -> Vec<Data> {
    contents(&values.iter().flatten().cloned().collect::<Vec<_>>())
}
fn pair(artifact: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    let typed = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let word =
        JitProgramArtifact::compile_for_test(artifact, &catalog, OptimizationPass::TypedMapFolds)
            .unwrap();
    crate::native_artifact::emit_program_object(artifact, &catalog).unwrap();
    (typed, word)
}
fn compare(
    typed: &JitProgramArtifact,
    word: &JitProgramArtifact,
    args: &[Value],
    memory: &ManagedMemoryBudget,
    policy: NativeProgramExecutionLimits,
    restored: bool,
    fast: &mut [Option<Value>],
    old: &mut [Option<Value>],
) -> observe::Counts {
    let before = contents(args);
    let previous = output(fast);
    let run = if restored {
        observe::run_restored
    } else {
        observe::run
    };
    let (a, counts) = run(typed, args, fast, policy, memory);
    let (b, prior_counts) = run(word, args, old, policy, memory);
    assert_eq!(
        a.as_ref().map_err(|e| format!("{e:?}")),
        b.as_ref().map_err(|e| format!("{e:?}"))
    );
    assert_eq!(
        output(fast),
        output(old),
        "exact F64 words and original first failure"
    );
    if a.is_err() {
        assert_eq!(output(fast), previous, "atomic error");
    }
    assert_eq!(counts.helpers, prior_counts.helpers);
    assert_eq!(counts.admits, prior_counts.admits);
    assert_eq!(contents(args), before, "unchanged source/aliases");
    counts
}
#[test]
fn typed_map_folds_standalone_order_payloads_vector_orientations_limits_cleanup_and_retry() {
    let source =
        "+> stats\nstats/sum/column([value * factor<f64> | value <- values<[f64]>])[1u64]\n";
    for artifact in artifacts(source) {
        let (typed, word) = pair(&artifact);
        assert!(
            typed
                .descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .any(|r| r.reduction.is_some())
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let cases = [
            vec![],
            vec![1u64 << 63, 1u64 << 63],
            vec![1, 2, 3, 0x8000_0000_0000_0001],
            vec![1e16f64.to_bits(), 1f64.to_bits(), (-1e16f64).to_bits()],
            vec![0x7ff0_0000_0000_0000, 0xfff0_0000_0000_0000],
            vec![0x7ff8_0000_0000_0037, 0xfff8_0000_0000_0041],
            vec![0x7ff0_0000_0000_0037, 1f64.to_bits()],
            vec![
                1f64.to_bits(),
                2f64.to_bits(),
                (-3f64).to_bits(),
                4f64.to_bits(),
            ],
        ];
        for restored in [false, true] {
            for cells in &cases {
                for column in [false, true] {
                    let axes = if column {
                        [cells.len() as u64, 1]
                    } else {
                        [1, cells.len() as u64]
                    };
                    let args = numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("values", matrix(cells), &axes),
                            ("factor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut old = fast.clone();
                    let counts = compare(
                        &typed,
                        &word,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut fast,
                        &mut old,
                    );
                    assert!(counts.helpers.iter().any(|((_, op), &n)| *op
                        == GlobalHelper::NumericReductionFinish.raw()
                        && n > 0));
                    for dimension in 0..5 {
                        for cap in [0, 1, 1_000_000_000_000] {
                            let mut policy = mode.execution();
                            match dimension {
                                0 => policy.library.max_output_elements = Some(cap),
                                1 => policy.library.max_temporary_bytes = Some(cap),
                                2 => policy.library.max_compute_work = Some(cap),
                                3 => policy.execution.max_work = cap,
                                _ => policy.execution.max_transport_bytes = cap,
                            }
                            compare(
                                &typed, &word, &args, &memory, policy, restored, &mut fast,
                                &mut old,
                            );
                        }
                    }
                    compare(
                        &typed,
                        &word,
                        &args,
                        &memory,
                        mode.execution(),
                        restored,
                        &mut fast,
                        &mut old,
                    );
                    drop((fast, old));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                    assert_eq!(contents(&aliases), contents(&args));
                }
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}

const PHASE: &str = r#"+> stats
#Fold(initial<[f64]:2,4>, first<u64>, last<u64>, row<u64>, divisor<f64>, rounds<u64>) => <[f64]:2,4>
  | :Step(data<[f64]:2,4>, pass<u64>)
  | :Done(data<[f64]:2,4>).
#Fold(initial, first, last, row, divisor, rounds) -> :Step(initial, 0u64)
  :Step(data, pass)
    | pass < rounds -> {
        total := stats/sum/column([(k <= 4u64? | true => data[row,k] / divisor | false => 0.0) | k <- first..last])[1u64]
        ~next := data
        next[1u64,1u64] = total
      } -> :Step(next, pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[f64]:2,4>
answer := #Fold(data,first<u64>,last<u64>,row<u64>,divisor<f64>,rounds<u64>)
(answer,data)
"#;

#[test]
fn typed_map_folds_phase_current_words_bounds_first_errors_restored_and_retry() {
    for artifact in artifacts(PHASE) {
        let (typed, word) = pair(&artifact);
        assert!(typed.descriptor().metadata().closed_tails.regions.iter().any(|r|r.numeric.iter().any(|p|p.actions.iter().any(|(_,a)|matches!(a,crate::native_artifact::program::prepare::readonly_numeric::Operation::Reduce{..})))));
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut fast = vec![None; artifact.outputs().len()];
            let mut old = fast.clone();
            for (first, last, row, rounds) in [
                (1, 5, 1, 3),
                (1, 5, 2, 2),
                (0, 5, 1, 1),
                (1, 5, 3, 1),
                (3, 1, 1, 1),
                (1, 1, 1, 1),
                (1, 7, 1, 2),
                (1, 5, 1, 3),
            ] {
                let bits = [
                    1e16f64.to_bits(),
                    1f64.to_bits(),
                    (-1e16f64).to_bits(),
                    3f64.to_bits(),
                    1,
                    0x8000_0000_0000_0000,
                    0x7ff0_0000_0000_0037,
                    0x7ff8_0000_0000_0041,
                ];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("input", matrix(&bits), &[2, 4]),
                        ("first", Data::U64(first), &[]),
                        ("last", Data::U64(last), &[]),
                        ("row", Data::U64(row), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                        ("divisor", Data::F64(F64Bits::from_f64(1.0)), &[]),
                    ],
                );
                let counts = compare(
                    &typed,
                    &word,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut fast,
                    &mut old,
                );
                if row == 1 && first == 1 && last == 5 {
                    assert!(counts.phases > 0);
                }
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
#[test]
fn typed_map_folds_nested_loans_exact_words_unchanged_helpers_and_cleanup() {
    let source = "+> stats\nvalues := data<[f64]:3,4>\nweights := coefficients<[f64]:3,1>\n[stats/sum/column([values[i,j] * weights[i] | i <- 1u64..=3u64])[1u64] | j <- 1u64..=4u64]\n";
    for artifact in artifacts(source) {
        let (typed, word) = pair(&artifact);
        assert!(
            typed
                .descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .any(|r| !r.nested.is_empty())
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            for final_word in [
                (-1e16f64).to_bits(),
                0x7ff8_0000_0000_0037,
                0x7ff0_0000_0000_0041,
            ] {
                let bits = [
                    1e16f64.to_bits(),
                    0,
                    1,
                    0x8000_0000_0000_0000,
                    1f64.to_bits(),
                    0,
                    2,
                    0x8000_0000_0000_0001,
                    final_word,
                    0,
                    3,
                    0,
                ];
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        ("data", matrix(&bits), &[3, 4]),
                        ("coefficients", matrix(&[1f64.to_bits(); 3]), &[3, 1]),
                    ],
                );
                let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut fast = vec![None; artifact.outputs().len()];
                let mut old = fast.clone();
                compare(
                    &typed,
                    &word,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut fast,
                    &mut old,
                );
                drop((fast, old));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
