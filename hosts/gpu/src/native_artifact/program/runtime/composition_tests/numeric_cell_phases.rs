//! Two-axis scalar loops retain independent bounds and functional versions.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;

fn source(kind: &str, nested: bool, old_peer: bool) -> String {
    let parent = if nested {
        "outer(data<[K]>, pass<u64>, passes<u64>, start<u64>, end<u64>, row<u64>, peer<u64>, marker<K>, use-old<bool>) => (out<[K]>) :=\n  more := pass < passes\n  next := (more? | true => walk(data,start,end,row,peer,marker,use-old) | false => data)\n  out := (more? | true => outer(next,pass + 1u64,passes,start,end,row,peer,marker,use-old) | false => next).\nanswer := outer(data,0u64,2u64,start<u64>,end<u64>,row<u64>,peer<u64>,marker<K>,use-old<bool>)\n"
    } else {
        "answer := walk(data,start<u64>,end<u64>,row<u64>,peer<u64>,marker<K>,use-old<bool>)\n"
    };
    let peer = if old_peer { "data" } else { "next" };
    format!(r#"put(data<[K]>, row<u64>, column<u64>, marker<K>) => (out<[K]>) :=
  ~next := data
  next[row,column] = marker
  out := next.
walk(data<[K]>, column<u64>, end<u64>, row<u64>, peer<u64>, marker<K>, use-old<bool>) => (out<[K]>) :=
  more := column < end
  old := (more? | true => data[row,column] | false => marker)
  next := (more? | true => put(data,row,column,marker) | false => data)
  other := (more? | true => {peer}[row,peer] | false => marker)
  selected := (use-old? | true => old | false => other)
  out := (more? | true => walk(next,column + 1u64,end,row,peer,selected,use-old) | false => next).
data := input<[K]>
{parent}(answer,data)
"#).replace('K',kind)
}
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    shape: [u64; 2],
    marker: Data,
    coordinates: [u64; 4],
    use_old: bool,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("input", Data::Matrix(cells.to_vec().into()), &shape),
            ("marker", marker, &[]),
            ("start", Data::U64(coordinates[0]), &[]),
            ("end", Data::U64(coordinates[1]), &[]),
            ("row", Data::U64(coordinates[2]), &[]),
            ("peer", Data::U64(coordinates[3]), &[]),
            ("use-old", Data::Bool(use_old), &[]),
        ],
    )
}
fn last(out: &[Option<Value>]) -> Data {
    out.last()
        .unwrap()
        .as_ref()
        .unwrap()
        .canonical_data_draft()
        .unwrap()
}
fn run(
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
fn numeric_cell_phases_execute_all_exact_kinds_with_distinct_columns_and_partial_failure_recovery()
{
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for nested in [false, true] {
            for artifact in artifacts(&source(kind, nested, false)) {
                let mode = NativeProgramLimitMode::NoLimits;
                let catalog = mech_stdlib::source_catalog();
                let jit = JitProgramArtifact::compile_with_limits(
                    &artifact,
                    &catalog,
                    mode.compilation(),
                )
                .unwrap();
                let ordinary = JitProgramArtifact::compile_for_test(
                    &artifact,
                    &catalog,
                    OptimizationPass::ScalarLoops,
                )
                .unwrap();
                assert!(
                    jit.descriptor()
                        .metadata()
                        .closed_tails
                        .regions
                        .iter()
                        .any(|r| r.numeric.as_ref().is_some_and(|p| p
                            .actions
                            .iter()
                            .any(|(_, a)| matches!(a, Operation::CellRead { .. }))
                            && p.actions
                                .iter()
                                .any(|(_, a)| matches!(a, Operation::CellWrite { .. })))),
                    "actual {kind} cell proof"
                );
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    for (coordinates, use_old, error) in [
                        ([1, 4, 1, 2], true, false),
                        ([1, 4, 2, 1], false, false),
                        ([1, 2, 1, 1], false, false),
                        ([1, 1, 0, 0], false, false),
                        ([0, 1, 1, 2], true, true),
                        ([1, 2, 0, 3], true, true),
                        ([1, 2, 3, 1], false, true),
                        ([1, 5, 1, 2], true, true),
                        ([1, 2, 1, 0], false, true),
                        ([1, 2, 1, 4], false, true),
                        ([1, 4, 2, 3], false, false),
                    ] {
                        let arguments = args(
                            &artifact,
                            &memory,
                            &cells,
                            [2, 3],
                            marker.clone(),
                            coordinates,
                            use_old,
                        );
                        let aliases = arguments.clone();
                        let before = contents(&arguments);
                        let prior_fast = fast.clone();
                        let prior_cold = cold.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (a, counts) = run(
                            &jit,
                            &arguments,
                            &mut fast,
                            &memory,
                            mode.execution(),
                            restored,
                        );
                        let (b, _) = run(
                            &ordinary,
                            &arguments,
                            &mut cold,
                            &memory,
                            mode.execution(),
                            restored,
                        );
                        if error {
                            assert_eq!(
                                format!("{:?}", a.unwrap_err()),
                                format!("{:?}", b.unwrap_err()),
                                "{kind} {coordinates:?}"
                            );
                            for (now, old) in fast
                                .iter()
                                .zip(&prior_fast)
                                .chain(cold.iter().zip(&prior_cold))
                            {
                                assert!(match (now, old) {
                                    (Some(n), Some(o)) => n.shares_frozen_storage(o),
                                    (None, None) => true,
                                    _ => false,
                                });
                            }
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                retained
                            );
                        } else {
                            a.unwrap();
                            b.unwrap();
                            assert_eq!(last(&fast), last(&cold));
                            let mut expected = cells.clone();
                            let mut changed = false;
                            for _ in 0..if nested { 2 } else { 1 } {
                                let mut incoming = marker.clone();
                                for column in coordinates[0]..coordinates[1] {
                                    let physical = ((coordinates[2] - 1) * 3 + column - 1) as usize;
                                    let old = expected[physical].clone();
                                    changed |= expected[physical] != incoming;
                                    expected[physical] = incoming;
                                    incoming = if use_old {
                                        old
                                    } else {
                                        expected[((coordinates[2] - 1) * 3 + coordinates[3] - 1)
                                            as usize]
                                            .clone()
                                    };
                                }
                            }
                            assert_eq!(
                                last(&fast),
                                Data::Tuple(
                                    vec![
                                        Data::Matrix(expected.into()),
                                        Data::Matrix(cells.clone().into())
                                    ]
                                    .into()
                                )
                            );
                            assert!(
                                !counts.tail_entries.is_empty(),
                                "execute native {kind} cells"
                            );
                            for (_, id, _) in &counts.tail_entries {
                                if let Some(proof) =
                                    &jit.descriptor().metadata().closed_tails.regions[*id].numeric
                                {
                                    for (site, a) in &proof.actions {
                                        if matches!(
                                            a,
                                            Operation::CellRead { .. }
                                                | Operation::CellWrite { .. }
                                        ) {
                                            assert_eq!(
                                                counts
                                                    .helpers
                                                    .get(&(
                                                        *site as u64,
                                                        GlobalHelper::Library.raw()
                                                    ))
                                                    .copied()
                                                    .unwrap_or(0),
                                                0
                                            );
                                        }
                                    }
                                }
                            }
                            assert_eq!(
                                !counts.first_change_sites.is_empty(),
                                changed,
                                "no-op/first-change {kind} {coordinates:?}"
                            );
                            assert!(
                                counts
                                    .first_change_backings
                                    .iter()
                                    .all(|(old, new)| old != new),
                                "live aliases retain their backing"
                            );
                        }
                        assert_eq!(contents(&arguments), before);
                        assert_eq!(contents(&aliases), before);
                    }
                    drop((fast, cold));
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
fn numeric_cell_phases_keep_empty_axes_inactive_reads_and_finite_admission() {
    let (_, cells, marker) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&source("f64", true, false)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for (shape, coordinates, error) in [
                ([0, 3], [1, 1, 0, 0], false),
                ([0, 3], [1, 2, 1, 1], true),
                ([2, 0], [1, 1, 0, 0], false),
                ([2, 0], [1, 2, 1, 1], true),
            ] {
                let arguments = args(
                    &artifact,
                    &memory,
                    &[],
                    shape,
                    marker.clone(),
                    coordinates,
                    false,
                );
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                let (a, _) = run(
                    &jit,
                    &arguments,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                );
                let (b, _) = run(
                    &ordinary,
                    &arguments,
                    &mut cold,
                    &memory,
                    mode.execution(),
                    restored,
                );
                assert_eq!(a.is_err(), error);
                assert_eq!(b.is_err(), error);
                if error {
                    assert_eq!(format!("{a:?}"), format!("{b:?}"));
                } else {
                    assert_eq!(last(&fast), last(&cold));
                }
            }
            let arguments = args(
                &artifact,
                &memory,
                &cells,
                [2, 3],
                marker.clone(),
                [1, 4, 1, 2],
                false,
            );
            let before = contents(&arguments);
            for dimension in 0..8 {
                for cap in [0, 1, 16, 255, 4095, 65535] {
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
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, _) = run(&jit, &arguments, &mut fast, &memory, policy, restored);
                    let (b, _) = run(&ordinary, &arguments, &mut cold, &memory, policy, restored);
                    assert_eq!(a.is_ok(), b.is_ok(), "{dimension} {cap}: {a:?} / {b:?}");
                    if a.is_ok() {
                        assert_eq!(last(&fast), last(&cold));
                    } else {
                        assert!(fast.iter().all(Option::is_none));
                        assert!(cold.iter().all(Option::is_none));
                    }
                    drop((fast, cold));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            let mut output = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &arguments,
                &mut output,
                &memory,
                mode.execution(),
                restored,
            )
            .0
            .unwrap();
            assert_eq!(contents(&arguments), before);
            drop((arguments, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn numeric_cell_phases_never_rebind_an_older_matrix_version_to_private_current_cells() {
    let (_, cells, marker) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&source("f64", true, true)) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for peer in [1, 2, 3] {
                let arguments = args(
                    &artifact,
                    &memory,
                    &cells,
                    [2, 3],
                    marker.clone(),
                    [1, 4, 1, peer],
                    false,
                );
                let before = contents(&arguments);
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                run(
                    &jit,
                    &arguments,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                )
                .0
                .unwrap();
                run(
                    &ordinary,
                    &arguments,
                    &mut cold,
                    &memory,
                    mode.execution(),
                    restored,
                )
                .0
                .unwrap();
                assert_eq!(last(&fast), last(&cold));
                assert_eq!(contents(&arguments), before);
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
#[test]
fn numeric_cell_phases_preserve_unsigned_append_authority_after_first_cell_write() {
    let source = r#"+> sequence
put(data<[u8]>, marker<u8>) => (out<[u8]>) :=
  ~next := data
  next[1u64,1u64] = marker
  out := next.
walk(data<[u8]>, at<u64>, end<u64>) => (out<[u8]>) :=
  more := at < end
  changed := (more? | true => put(data,9u8) | false => data)
  next := (more? | true => sequence/append(changed,9u8) | false => data)
  out := (more? | true => walk(next,at + 1u64,end) | false => next).
empty := sequence/with-capacity(0u8,8u64)
data := sequence/append(sequence/append(empty,0u8),0u8)
answer := walk(data,0u64,steps<u64>)
(answer,data)
"#;
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r
                    .numeric
                    .as_ref()
                    .is_some_and(|p| p.arrays.iter().any(|a| a.appends)
                        && p.actions
                            .iter()
                            .any(|(_, a)| matches!(a, Operation::CellWrite { .. }))))
        );
        for restored in [false, true] {
            let memory = mode.memory_budget();
            for steps in [0, 3, 9, 2] {
                let arguments = inputs(&artifact, &[("steps", Data::U64(steps))], &memory);
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                let (a, counts) = run(
                    &jit,
                    &arguments,
                    &mut fast,
                    &memory,
                    mode.execution(),
                    restored,
                );
                let (b, _) = run(
                    &ordinary,
                    &arguments,
                    &mut cold,
                    &memory,
                    mode.execution(),
                    restored,
                );
                a.unwrap();
                b.unwrap();
                assert_eq!(last(&fast), last(&cold));
                if steps > 0 {
                    assert!(!counts.first_change_sites.is_empty());
                }
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
