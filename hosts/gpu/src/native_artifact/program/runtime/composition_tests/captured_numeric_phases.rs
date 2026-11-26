//! Immutable captures and mutable FSM payloads share the maintained native loan.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::{ArrayStorage, Operation};

fn source(kind: &str, cell: bool) -> String {
    let selector = if cell { "row,at" } else { "at" };
    r#"#Scan(initial<[K]:2,3>, source<[K]:2,3>, start<u64>, end<u64>, row<u64>, marker<K>) => <[K]:2,3>
  | :Walk(data<[K]:2,3>, at<u64>, carried<K>)
  | :Done(data<[K]:2,3>).
#Scan(initial, source, start, end, row, marker) -> :Walk(initial, start, marker)
  :Walk(data, at, carried)
    | at < end -> {
        ~next := data
        next[SELECTOR] = carried
        original := source[SELECTOR]
      } -> :Walk(next, at + 1u64, original)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Scan(data,data,start<u64>,end<u64>,row<u64>,marker<K>)
(answer,data)
"#
    .replace("SELECTOR", selector)
    .replace('K', kind)
}
fn args(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    marker: Data,
    start: u64,
    end: u64,
    row: u64,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("input", Data::Matrix(cells.to_vec().into()), &[2, 3]),
            ("marker", marker, &[]),
            ("start", Data::U64(start), &[]),
            ("end", Data::U64(end), &[]),
            ("row", Data::U64(row), &[]),
        ],
    )
}
fn run(
    jit: &JitProgramArtifact,
    arguments: &[Value],
    out: &mut [Option<Value>],
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
    restored: bool,
) -> (
    Result<NativeProgramExecutionReport, NativeProgramExecutionError>,
    observe::Counts,
) {
    if restored {
        observe::run_restored(jit, arguments, out, policy, memory)
    } else {
        observe::run(jit, arguments, out, policy, memory)
    }
}

#[test]
fn captured_numeric_phases_use_real_guarded_loans_and_keep_immutable_aliases_for_all_kinds() {
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for cell in [false, true] {
            for artifact in artifacts(&source(kind, cell)) {
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
                let metadata = jit.descriptor().metadata();
                let captured = metadata
                    .closed_tails
                    .regions
                    .iter()
                    .enumerate()
                    .filter_map(|(id, r)| {
                        r.numeric
                            .as_ref()
                            .filter(|p| {
                                p.tagged.is_some()
                                    && p.arrays
                                        .iter()
                                        .any(|a| matches!(a.storage, ArrayStorage::Capture { .. }))
                            })
                            .map(|p| (id, p))
                    })
                    .collect::<Vec<_>>();
                if captured.is_empty() {
                    let facts = metadata.plan.facts();
                    for control in metadata.plan.iterations() {
                        if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control
                        {
                            eprintln!("{kind} cell={cell} capture candidate: {:?}", crate::native_artifact::program::prepare::readonly_numeric::tagged::candidate(
                                &facts,tail,&metadata.buffer_frames,&metadata.scalar_aggregates,&metadata.calls,&metadata.visits,0).err());
                        }
                    }
                    panic!("actual guarded capture phase {kind} cell={cell}");
                }
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                let memory = mode.memory_budget();
                let mut fast = vec![None; artifact.outputs().len()];
                let mut cold = fast.clone();
                for restored in [false, true] {
                    let mut current = cells.clone();
                    if restored {
                        current.reverse();
                    }
                    let good_end = if cell { 4 } else { 7 };
                    for (start, end, row, error) in [
                        (1, good_end, 1, false),
                        (1, 1, 0, false),
                        (0, 2, 1, true),
                        (1, good_end + 1, 1, true),
                        (1, good_end, if cell { 3 } else { 2 }, cell),
                        (1, good_end, 2, false),
                    ] {
                        let arguments = args(
                            &artifact,
                            &memory,
                            &current,
                            marker.clone(),
                            start,
                            end,
                            row,
                        );
                        let aliases = arguments.clone();
                        let before = contents(&arguments);
                        let previous =
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                        let (a, observed) = run(
                            &jit,
                            &arguments,
                            &mut fast,
                            mode.execution(),
                            &memory,
                            restored,
                        );
                        let (b, _) = run(
                            &ordinary,
                            &arguments,
                            &mut cold,
                            mode.execution(),
                            &memory,
                            restored,
                        );
                        assert_eq!(
                            a.is_err(),
                            error,
                            "{kind} cell={cell} {start}..{end} row {row}: {a:?}"
                        );
                        assert_eq!(a.is_err(), b.is_err(), "{kind} ordinary parity");
                        if error {
                            assert_eq!(
                                format!("{:?}", a.as_ref().err()),
                                format!("{:?}", b.as_ref().err()),
                                "exact source error"
                            );
                            assert_eq!(
                                contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                                previous
                            );
                        } else if start < end {
                            assert!(
                                observed.tail_entries.iter().any(|(_, id, _)| captured
                                    .iter()
                                    .any(|(expected, _)| id == expected)),
                                "actual captured native entry: {observed:?}"
                            );
                            for (id, proof) in &captured {
                                if !observed
                                    .tail_entries
                                    .iter()
                                    .any(|(_, entered, _)| entered == id)
                                {
                                    continue;
                                }
                                for (site, op) in &proof.actions {
                                    if matches!(
                                        op,
                                        Operation::Read { .. }
                                            | Operation::CellRead { .. }
                                            | Operation::Write { .. }
                                            | Operation::CellWrite { .. }
                                            | Operation::Frame
                                    ) {
                                        assert_eq!(
                                            observed
                                                .helpers
                                                .get(&(*site as u64, GlobalHelper::Library.raw()))
                                                .copied()
                                                .unwrap_or(0),
                                            0,
                                            "successful native iteration has no library callback"
                                        );
                                    }
                                }
                            }
                        }
                        assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                        );
                        assert_eq!(
                            contents(&arguments),
                            before,
                            "capture and carrier arguments stay immutable"
                        );
                        assert_eq!(
                            contents(&aliases),
                            before,
                            "external aliases stay immutable"
                        );
                    }
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

#[test]
fn captured_numeric_phases_keep_finite_admission_atomic_publication_and_retry() {
    let (_, cells, marker) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&source("f64", true)) {
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
        let memory = mode.memory_budget();
        let arguments = args(&artifact, &memory, &cells, marker.clone(), 1, 4, 1);
        let before = contents(&arguments);
        for restored in [false, true] {
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
                    let (a, _) = run(&jit, &arguments, &mut fast, policy, &memory, restored);
                    let (b, _) = run(&ordinary, &arguments, &mut cold, policy, &memory, restored);
                    if dimension < 5 {
                        assert_eq!(a.is_ok(), b.is_ok(), "{dimension} {cap}: {a:?} / {b:?}");
                    }
                    if a.is_ok() && b.is_ok() {
                        assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                        );
                    }
                    if a.is_err() {
                        assert!(fast.iter().all(Option::is_none));
                    }
                    if b.is_err() {
                        assert!(cold.iter().all(Option::is_none));
                    }
                    drop((fast, cold));
                    assert_eq!(contents(&arguments), before);
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        retained
                    );
                }
            }
            let mut out = vec![None; artifact.outputs().len()];
            run(
                &jit,
                &arguments,
                &mut out,
                mode.execution(),
                &memory,
                restored,
            )
            .0
            .unwrap();
        }
        drop(arguments);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}
