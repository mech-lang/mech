//! Public original/decoded/restored comparisons against the old checked DAG.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use mech_core::snapshot::F64Bits;

pub(super) fn matrix(bits: &[u64]) -> Data {
    Data::Matrix(
        bits.iter()
            .map(|&b| Data::F64(F64Bits::from_bits(b)))
            .collect(),
    )
}
pub(super) fn output(values: &[Option<Value>]) -> Vec<Data> {
    contents(&values.iter().flatten().cloned().collect::<Vec<_>>())
}
pub(super) fn pair(artifact: &ProgramArtifact) -> (JitProgramArtifact, JitProgramArtifact) {
    let catalog = mech_stdlib::source_catalog();
    let fast = JitProgramArtifact::compile(artifact, &catalog).unwrap();
    let old =
        JitProgramArtifact::compile_for_test(artifact, &catalog, OptimizationPass::OrderedMapDots)
            .unwrap();
    crate::native_artifact::emit_program_object(artifact, &catalog).unwrap();
    (fast, old)
}
pub(super) fn compare(
    fast: &JitProgramArtifact,
    old: &JitProgramArtifact,
    args: &[Value],
    memory: &ManagedMemoryBudget,
    limits: NativeProgramExecutionLimits,
    restored: bool,
    outputs: &mut [Option<Value>],
    previous: &mut [Option<Value>],
) -> (bool, observe::Counts) {
    let before = contents(args);
    let retained = output(outputs);
    let run = if restored {
        observe::run_restored
    } else {
        observe::run
    };
    let (a, counts) = run(fast, args, outputs, limits, memory);
    let (b, other) = run(old, args, previous, limits, memory);
    assert_eq!(
        a.as_ref().map_err(|e| format!("{e:?}")),
        b.as_ref().map_err(|e| format!("{e:?}")),
        "same original first error"
    );
    assert_eq!(
        output(outputs),
        output(previous),
        "exact ordered F64 words and NaN payloads"
    );
    if a.is_err() {
        assert_eq!(output(outputs), retained, "atomic error");
    }
    assert_eq!(counts.helpers, other.helpers);
    assert_eq!(counts.admits, other.admits);
    assert_eq!(contents(args), before, "immutable aliases/source");
    (a.is_ok(), counts)
}
const SOURCE: &str = "+> stats\na := left<[f64]>\nb := right<[f64]>\nstats/sum/column([a[i * scale<u64> + shift<u64>] * b[bound<u64> - i] | i <- first<u64>..last<u64>])[1u64]\n";
#[test]
fn ordered_map_dots_scalar_affine_reverse_linear_layouts_first_errors_payloads_and_retry() {
    for artifact in artifacts(SOURCE) {
        let (fast, old) = pair(&artifact);
        assert!(
            fast.descriptor()
                .metadata()
                .numeric_maps
                .regions
                .iter()
                .any(|r| {
                    crate::native_artifact::program::prepare::numeric_maps::dots::Dot::derive(
                        &fast.descriptor().metadata().plan,
                        r,
                        None,
                    )
                    .is_some()
                })
        );
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            for axes in [[1, 8], [8, 1], [2, 4]] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let mut previous = outputs.clone();
                for special in [
                    (-1e16f64).to_bits(),
                    1,
                    0x8000_0000_0000_0000,
                    0x7ff8_0000_0000_0037,
                    0xfff0_0000_0000_0041,
                    0x7ff0_0000_0000_0000,
                ] {
                    let left = [
                        1e16f64.to_bits(),
                        1f64.to_bits(),
                        special,
                        3f64.to_bits(),
                        0,
                        2,
                        0x8000_0000_0000_0001,
                        (-2f64).to_bits(),
                    ];
                    for (first, last, scale, shift, bound) in [
                        (1, 4, 1, 0, 8),
                        (1, 4, 2, 0, 8),
                        (1, 2, 1, 0, 8),
                        (1, 1, 1, 0, 8),
                        (3, 1, 1, 0, 8),
                        (0, 4, 1, 0, 8),
                        (1, 4, 1, 0, 2),
                        (1, 4, u64::MAX, 1, 8),
                        (1, 4, 1, 7, 8),
                        (1, 4, 1, 0, 8),
                    ] {
                        let args = numeric_maps::typed_arguments(
                            &artifact,
                            &memory,
                            &[
                                ("left", matrix(&left), &axes),
                                ("right", matrix(&[1f64.to_bits(); 8]), &axes),
                                ("first", Data::U64(first), &[]),
                                ("last", Data::U64(last), &[]),
                                ("scale", Data::U64(scale), &[]),
                                ("shift", Data::U64(shift), &[]),
                                ("bound", Data::U64(bound), &[]),
                            ],
                        );
                        let aliases = args.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        compare(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            mode.execution(),
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                        assert_eq!(contents(&aliases), contents(&args));
                        // Outputs can retain public source bindings; releasing both
                        // result sets must release every temporary/private bank.
                        outputs.fill(None);
                        previous.fill(None);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            retained
                        );
                    }
                }
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
const PHASE: &str = r#"+> stats
#Fold(initial<[f64]:2,4>, first<u64>, last<u64>, row<u64>, rounds<u64>) => <[f64]:2,4>
  | :Step(data<[f64]:2,4>, pass<u64>)
  | :Done(data<[f64]:2,4>).
#Fold(initial, first, last, row, rounds) -> :Step(initial, 0u64)
  :Step(data, pass)
    | pass < rounds -> {
        total := stats/sum/column([data[row,k] * data[2u64,k] | k <- first..last])[1u64]
        ~next := data
        next[1u64,1u64] = total
      } -> :Step(next, pass + 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[f64]:2,4>
answer := #Fold(data,first<u64>,last<u64>,row<u64>,rounds<u64>)
(answer,data)
"#;
#[test]
fn ordered_map_dots_phase_current_views_order_finite_limits_atomicity_and_restored() {
    for artifact in artifacts(PHASE) {
        let (fast, old) = pair(&artifact);
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        for restored in [false, true] {
            let mut outputs = vec![None; artifact.outputs().len()];
            let mut previous = outputs.clone();
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
                let args = numeric_maps::typed_arguments(
                    &artifact,
                    &memory,
                    &[
                        (
                            "input",
                            matrix(&[
                                1e16f64.to_bits(),
                                1f64.to_bits(),
                                (-1e16f64).to_bits(),
                                1,
                                1f64.to_bits(),
                                1f64.to_bits(),
                                1f64.to_bits(),
                                0x8000_0000_0000_0000,
                            ]),
                            &[2, 4],
                        ),
                        ("first", Data::U64(first), &[]),
                        ("last", Data::U64(last), &[]),
                        ("row", Data::U64(row), &[]),
                        ("rounds", Data::U64(rounds), &[]),
                    ],
                );
                let (ok, counts) = compare(
                    &fast,
                    &old,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                if first == 1 && last == 5 && row == 1 {
                    assert!(ok);
                    assert!(counts.phases > 0);
                }
                for dimension in 0..8 {
                    for cap in [0, 1, 64, 65536, 1 << 36] {
                        let mut policy = mode.execution();
                        match dimension {
                            0 => policy.library.max_output_elements = Some(cap),
                            1 => policy.library.max_output_bytes = Some(cap),
                            2 => policy.library.max_temporary_bytes = Some(cap),
                            3 => policy.library.max_cloned_bytes = Some(cap),
                            4 => policy.library.max_retained_nodes = Some(cap),
                            5 => policy.library.max_compute_work = Some(cap),
                            6 => policy.library.max_comparison_work = Some(cap),
                            _ => policy.execution.max_work = cap,
                        };
                        compare(
                            &fast,
                            &old,
                            &args,
                            &memory,
                            policy,
                            restored,
                            &mut outputs,
                            &mut previous,
                        );
                    }
                }
                compare(
                    &fast,
                    &old,
                    &args,
                    &memory,
                    mode.execution(),
                    restored,
                    &mut outputs,
                    &mut previous,
                );
                outputs.fill(None);
                previous.fill(None);
            }
        }
        assert_eq!(memory.used_bytes(), 0);
    }
}
