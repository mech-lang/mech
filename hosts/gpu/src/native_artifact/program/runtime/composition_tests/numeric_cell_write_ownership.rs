//! Combined two-axis bounds preserve repeated writes, aliases and first change.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(kind: &str) -> String {
    r#"#Repeat(initial<[K]:2,3>, steps<u64>, row<u64>, column<u64>, marker<K>) => <([K]:2,3,K)>
  | :Write(data<[K]:2,3>, remaining<u64>, seen<K>)
  | :Done(data<[K]:2,3>, seen<K>).
#Repeat(initial, steps, row, column, marker) -> :Write(initial, steps, marker)
  :Write(data, remaining, seen)
    | remaining > 0u64 -> {
        old := data[row,column]
        ~next := data
        next[row,column] = marker
      } -> :Write(next, remaining - 1u64, old)
    | * -> :Done(data, seen)
  :Done(data, seen) => (data, seen).
data := input<[K]:2,3>
answer := #Repeat(data,steps<u64>,row<u64>,column<u64>,marker<K>)
(answer,data)
"#
    .replace('K', kind)
}

#[test]
fn numeric_cell_bounds_keep_repeated_equal_writes_aliases_exact_errors_and_noop_owners() {
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for artifact in artifacts(&source(kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let old = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let prior = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::PrivateWritesFirst,
            )
            .unwrap();
            let object = crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let old_object =
                crate::native_artifact::program::compiler::emit_program_object_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::ScalarLoops,
                )
                .unwrap();
            let sites = |object: &crate::native_artifact::NativeProgramObject| {
                let metadata = object.metadata();
                [
                    "compiled_readonly_numeric_loops",
                    "compiled_numeric_write_loops",
                ]
                .into_iter()
                .flat_map(|key| metadata[key].as_array().unwrap())
                .map(|r| r["typed_stores"].as_array().unwrap().len())
                .sum::<usize>()
            };
            assert!(
                sites(&object) > 0,
                "actual native cell store emission: {kind}"
            );
            assert_eq!(
                sites(&old_object),
                0,
                "ablation retains ordinary selected source operations"
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                let mut checked = fast.clone();
                for (row, column, steps, incoming, changed, error) in [
                    (1u64, 1u64, 8u64, cells[0].clone(), false, false),
                    (1, 1, 8, marker.clone(), true, false),
                    (0, 1, 8, marker.clone(), false, true),
                    (1, 0, 8, marker.clone(), false, true),
                    (3, 1, 8, marker.clone(), false, true),
                    (1, 4, 8, marker.clone(), false, true),
                    (0, 0, 8, marker.clone(), false, true),
                    (u64::MAX, u64::MAX, 0, marker.clone(), false, false),
                    (2, 3, 8, marker.clone(), true, false),
                ] {
                    let args = super::numeric_maps::typed_arguments(
                        &artifact,
                        &memory,
                        &[
                            ("input", Data::Matrix(cells.clone().into()), &[2, 3]),
                            ("marker", incoming.clone(), &[]),
                            ("steps", Data::U64(steps), &[]),
                            ("row", Data::U64(row), &[]),
                            ("column", Data::U64(column), &[]),
                        ],
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let (a, counts) = if restored {
                        observe::run_restored(&jit, &args, &mut fast, mode.execution(), &memory)
                    } else {
                        observe::run(&jit, &args, &mut fast, mode.execution(), &memory)
                    };
                    let (b, _) = observe::run(&old, &args, &mut cold, mode.execution(), &memory);
                    let (previous, _) = if restored {
                        observe::run_restored(
                            &prior,
                            &args,
                            &mut checked,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&prior, &args, &mut checked, mode.execution(), &memory)
                    };
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{previous:?}"),
                        "private-first ablation keeps every source counter/error"
                    );
                    assert_eq!(snapshots(&fast), snapshots(&checked));
                    assert_eq!(
                        a.is_err(),
                        error,
                        "{kind} ({row},{column}) steps{steps}: {a:?}"
                    );
                    assert_eq!(
                        a.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        b.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        "exact source error parity"
                    );
                    assert_eq!(
                        fast.iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>(),
                        cold.iter()
                            .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                            .collect::<Vec<_>>()
                    );
                    if let Ok(report) = a {
                        assert!(report.work > 0);
                        assert_eq!(
                            !counts.first_change_sites.is_empty(),
                            changed,
                            "{kind} first-change vs shared no-op"
                        );
                        assert!(
                            counts
                                .first_change_backings
                                .iter()
                                .all(|(old, new)| old != new)
                        );
                        let mut expected = cells.clone();
                        if changed {
                            expected[((row - 1) * 3 + column - 1) as usize] = incoming.clone();
                        }
                        let actual = fast
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap();
                        assert_eq!(
                            actual,
                            Data::Tuple(
                                vec![
                                    Data::Tuple(
                                        vec![Data::Matrix(expected.into()), incoming].into()
                                    ),
                                    Data::Matrix(cells.clone().into())
                                ]
                                .into()
                            )
                        );
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((fast, cold, checked));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

fn snapshots(values: &[Option<Value>]) -> Vec<Option<(mech_core::SchemaId, Vec<u64>, Data)>> {
    values
        .iter()
        .map(|v| {
            v.as_ref().map(|v| {
                (
                    v.schema(),
                    v.shape().parameter_values().to_vec(),
                    v.canonical_data_draft().unwrap(),
                )
            })
        })
        .collect()
}
fn late_source(kind: &str) -> String {
    r#"#Pair(initial<[K]:2,3>, steps<u64>, first<K>, second<K>, later-row<u64>, later-column<u64>) => <[K]:2,3>
  | :Write(data<[K]:2,3>, remaining<u64>)
  | :Done(data<[K]:2,3>).
#Pair(initial,steps,first,second,later-row,later-column) -> :Write(initial,steps)
  :Write(data,remaining)
    | remaining > 0u64 -> {
        first-turn := remaining == steps
        row := (first-turn? | true => 1u64 | false => later-row)
        column := (first-turn? | true => 1u64 | false => later-column)
        incoming := (first-turn? | true => first | false => second)
        ~next := data
        next[row,column] = incoming
      } -> :Write(next,remaining - 1u64)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Pair(data,steps<u64>,first<K>,second<K>,row<u64>,column<u64>)
(answer,data)
"#.replace('K',kind)
}
fn late_arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    first: Data,
    second: Data,
    steps: u64,
    row: u64,
    column: u64,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("input", Data::Matrix(cells.to_vec().into()), &[2, 3]),
            ("first", first, &[]),
            ("second", second, &[]),
            ("steps", Data::U64(steps), &[]),
            ("row", Data::U64(row), &[]),
            ("column", Data::U64(column), &[]),
        ],
    )
}
fn run_current(
    jit: &JitProgramArtifact,
    args: &[Value],
    out: &mut [Option<Value>],
    memory: &ManagedMemoryBudget,
    policy: NativeProgramExecutionLimits,
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
fn numeric_cell_private_first_keeps_late_axis_errors_old_aliases_and_zero_trip_recovery() {
    use mech_core::snapshot::F64Bits;
    let mut cases = super::typed_numeric_phases::cases();
    cases.push((
        "f64",
        [
            0x8000_0000_0000_0000,
            0x7ff0_0000_0000_0031,
            0x7ff8_0000_0000_0042,
            0,
            0x7ff0_0000_0000_0000,
            0xfff0_0000_0000_0000,
        ]
        .map(|v| Data::F64(F64Bits::from_bits(v)))
        .into(),
        Data::F64(F64Bits::from_bits(0x7ff0_0000_0000_0064)),
    ));
    for (kind, cells, first) in cases {
        for artifact in artifacts(&late_source(kind)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let catalog = mech_stdlib::source_catalog();
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let prior = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::PrivateWritesFirst,
            )
            .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            assert!(jit.descriptor().metadata().closed_tails.regions.iter().any(|r|r.numeric.as_ref().is_some_and(|p|p.actions.iter().any(|(_,op)|matches!(op,crate::native_artifact::program::prepare::readonly_numeric::Operation::CellWrite{..})))));
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut fast = vec![None; artifact.outputs().len()];
                let mut checked = fast.clone();
                let mut cold = fast.clone();
                for (steps, row, column, error) in [
                    (2, 2, 3, false),
                    (2, 0, 1, true),
                    (2, 1, 0, true),
                    (2, 3, 1, true),
                    (2, 1, 4, true),
                    (2, u64::MAX, u64::MAX, true),
                    (0, u64::MAX, u64::MAX, false),
                    (3, 1, 1, false),
                ] {
                    let args = late_arguments(
                        &artifact,
                        &memory,
                        &cells,
                        first.clone(),
                        cells[1].clone(),
                        steps,
                        row,
                        column,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = snapshots(&fast);
                    let (a, counts) =
                        run_current(&jit, &args, &mut fast, &memory, mode.execution(), restored);
                    let (b, _) = run_current(
                        &prior,
                        &args,
                        &mut checked,
                        &memory,
                        mode.execution(),
                        restored,
                    );
                    let (c, _) = run_current(
                        &ordinary,
                        &args,
                        &mut cold,
                        &memory,
                        mode.execution(),
                        restored,
                    );
                    assert_eq!(a.is_err(), error, "{kind} {steps} ({row},{column}):{a:?}");
                    assert_eq!(
                        format!("{a:?}"),
                        format!("{b:?}"),
                        "all native counters/error words match ablation"
                    );
                    assert_eq!(
                        a.as_ref().map(|_| ()).map_err(|e| format!("{e:?}")),
                        c.as_ref().map(|_| ()).map_err(|e| format!("{e:?}"))
                    );
                    assert_eq!(snapshots(&fast), snapshots(&checked));
                    assert_eq!(snapshots(&fast), snapshots(&cold));
                    if error {
                        assert!(
                            !counts.first_change_sites.is_empty(),
                            "first valid write really reached its copy boundary"
                        );
                        assert_eq!(snapshots(&fast), previous);
                    }
                    if steps == 0 {
                        assert!(counts.first_change_sites.is_empty());
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((fast, checked, cold));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
#[test]
fn numeric_cell_private_first_finite_denials_keep_publication_and_recover_on_the_same_inputs() {
    let (_, cells, first) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&late_source("f64")) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let prior = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::PrivateWritesFirst,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = late_arguments(
                &artifact,
                &memory,
                &cells,
                first.clone(),
                cells[1].clone(),
                3,
                2,
                3,
            );
            let before = contents(&args);
            let mut seed = vec![None; artifact.outputs().len()];
            run_current(&jit, &args, &mut seed, &memory, mode.execution(), restored)
                .0
                .unwrap();
            let initial = snapshots(&seed);
            for dimension in 0..8 {
                for cap in [0, 1, 16, 255, 4095, 65535, 1_000_000] {
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
                    }
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = seed.clone();
                    let mut checked = seed.clone();
                    let (a, _) = run_current(&jit, &args, &mut fast, &memory, policy, restored);
                    let (b, _) =
                        run_current(&prior, &args, &mut checked, &memory, policy, restored);
                    assert_eq!(
                        a.is_ok(),
                        b.is_ok(),
                        "dimension{dimension} cap{cap}: {a:?}/{b:?}"
                    );
                    assert_eq!(snapshots(&fast), snapshots(&checked));
                    if a.is_err() {
                        assert_eq!(snapshots(&fast), initial);
                    }
                    assert_eq!(contents(&args), before);
                    drop((fast, checked));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            let mut recovered = vec![None; artifact.outputs().len()];
            run_current(
                &jit,
                &args,
                &mut recovered,
                &memory,
                mode.execution(),
                restored,
            )
            .0
            .unwrap();
            assert_eq!(snapshots(&recovered), initial);
            drop((seed, recovered, args));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
