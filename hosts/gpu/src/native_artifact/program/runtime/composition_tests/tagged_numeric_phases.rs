//! Guarded numeric FSM iterations use actual native phases and checked exits.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source(kind: &str) -> String {
    r#"#Scan(initial<[K]:2,3>, start<u64>, end<u64>, row<u64>, marker<K>) => <[K]:2,3>
  | :Walk(data<[K]:2,3>, at<u64>, carried<K>)
  | :Done(data<[K]:2,3>).
#Scan(initial, start, end, row, marker) -> :Walk(initial, start, marker)
  :Walk(data, at, carried)
    | at < end -> {
        old := data[row,at]
        ~next := data
        next[row,at] = carried
      } -> :Walk(next, at + 1u64, old)
    | * -> :Done(data)
  :Done(data) => data.
data := input<[K]:2,3>
answer := #Scan(data,start<u64>,end<u64>,row<u64>,marker<K>)
(answer,data)
"#
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
#[test]
fn tagged_numeric_phases_keep_exact_payloads_aliases_cold_transitions_and_bounds_retry() {
    for (kind, cells, marker) in super::typed_numeric_phases::cases() {
        for artifact in artifacts(&source(kind)) {
            let catalog = mech_stdlib::source_catalog();
            let mode = NativeProgramLimitMode::NoLimits;
            let jit =
                JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                    .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &catalog,
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let metadata = jit.descriptor().metadata();
            if !metadata
                .closed_tails
                .regions
                .iter()
                .any(|r| r.numeric.as_ref().is_some_and(|p| p.tagged.is_some()))
            {
                let facts = metadata.plan.facts();
                for control in metadata.plan.iterations() {
                    if let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = control {
                        let reason=crate::native_artifact::program::prepare::readonly_numeric::tagged::candidate(
                            &facts,tail,&metadata.buffer_frames,&metadata.scalar_aggregates,&metadata.calls,&metadata.visits,0).err();
                        eprintln!("{kind} guarded candidate: {reason:?}");
                    }
                }
                panic!("actual {kind} native guarded phase");
            }
            crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
            let memory = mode.memory_budget();
            let mut fast = vec![None; artifact.outputs().len()];
            let mut cold = fast.clone();
            for restored in [false, true] {
                for (start, end, row, error) in [
                    (1, 4, 1, false),
                    (1, 4, 2, false),
                    (1, 1, 0, false),
                    (0, 2, 1, true),
                    (1, 5, 1, true),
                    (1, 3, 3, true),
                    (1, 3, 1, false),
                ] {
                    let arguments =
                        args(&artifact, &memory, &cells, marker.clone(), start, end, row);
                    let aliases = arguments.clone();
                    let before = contents(&arguments);
                    let old = contents(&fast.iter().flatten().cloned().collect::<Vec<_>>());
                    let (a, observed) = if restored {
                        observe::run_restored(
                            &jit,
                            &arguments,
                            &mut fast,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&jit, &arguments, &mut fast, mode.execution(), &memory)
                    };
                    let (b, _) =
                        observe::run(&ordinary, &arguments, &mut cold, mode.execution(), &memory);
                    assert_eq!(a.is_err(), error, "{kind} {start}..{end} row {row}: {a:?}");
                    assert_eq!(a.is_err(), b.is_err(), "{kind} ordinary error parity");
                    if error {
                        assert_eq!(
                            format!("{:?}", a.as_ref().err()),
                            format!("{:?}", b.as_ref().err()),
                            "{kind} exact source error {start}..{end} row {row}"
                        );
                    }
                    assert_eq!(contents(&arguments), before);
                    assert_eq!(contents(&aliases), before);
                    assert_eq!(
                        contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                        contents(&cold.iter().flatten().cloned().collect::<Vec<_>>())
                    );
                    if error {
                        assert_eq!(
                            contents(&fast.iter().flatten().cloned().collect::<Vec<_>>()),
                            old
                        );
                    } else if start < end {
                        for (_, id, _) in &observed.tail_entries {
                            let proof =
                                metadata.closed_tails.regions[*id].numeric.as_ref().unwrap();
                            for (site, op) in &proof.actions {
                                if matches!(op, crate::native_artifact::program::prepare::readonly_numeric::Operation::Frame
                                    | crate::native_artifact::program::prepare::readonly_numeric::Operation::CellRead { .. }
                                    | crate::native_artifact::program::prepare::readonly_numeric::Operation::CellWrite { .. }) {
                                    assert_eq!(observed.helpers.get(&(*site as u64,GlobalHelper::Library.raw())).copied().unwrap_or(0),0,
                                        "no per-iteration library dispatch {kind} at {site}");
                                }
                            }
                        }
                        assert!(
                            !observed.tail_entries.is_empty(),
                            "actual native entry {kind}: {observed:?}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn tagged_numeric_phases_preserve_finite_admission_publication_and_retry() {
    let (_, cells, marker) = super::typed_numeric_phases::cases().pop().unwrap();
    for artifact in artifacts(&source("f64")) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
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
                    let a = if restored {
                        observe::run_restored(&jit, &arguments, &mut fast, policy, &memory).0
                    } else {
                        observe::run(&jit, &arguments, &mut fast, policy, &memory).0
                    };
                    let b = if restored {
                        observe::run_restored(&ordinary, &arguments, &mut cold, policy, &memory).0
                    } else {
                        observe::run(&ordinary, &arguments, &mut cold, policy, &memory).0
                    };
                    // Control and library work may differ under a native
                    // schedule, but tight output/storage caps must not admit
                    // an output that its maintained constructor rejects.
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
            jit.evaluate_into(&arguments, &mut out, mode.execution(), &memory)
                .unwrap();
        }
        drop(arguments);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn tagged_numeric_phases_decline_observable_older_versions() {
    let source = source("u64")
        .replace(
            "next[row,at] = carried",
            "next[row,at] = carried\n        previous := data[row,at]",
        )
        .replace(
            ":Walk(next, at + 1u64, old)",
            ":Walk(next, at + 1u64, previous)",
        );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
            .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .all(|r| r.numeric.as_ref().is_none_or(|p| p.tagged.is_none()))
        );
        let cells = [3, 7, 11, 13, 17, 19].map(Data::U64);
        let arguments = args(&artifact, &memory, &cells, Data::U64(23), 1, 4, 1);
        let before = contents(&arguments);
        let mut out = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&arguments, &mut out, mode.execution(), &memory)
            .unwrap();
        assert_eq!(contents(&arguments), before);
        assert_eq!(
            out.last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            Data::Tuple(
                vec![
                    Data::Matrix([23, 3, 7, 13, 17, 19].map(Data::U64).into()),
                    Data::Matrix(cells.into())
                ]
                .into()
            )
        );
    }
}

#[test]
fn tagged_numeric_phases_enter_after_an_ordinary_different_variant() {
    let source = source("u64")
        .replace("| :Walk(data<[u64]:2,3>, at<u64>, carried<u64>)",
            "| :Start(data<[u64]:2,3>, at<u64>, carried<u64>)\n  | :Walk(data<[u64]:2,3>, at<u64>, carried<u64>)")
        .replace("-> :Walk(initial, start, marker)",
            "-> :Start(initial, start, marker)\n  :Start(data, at, carried) -> :Walk(data, at, carried)");
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let variant = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find_map(|r| r.numeric.as_ref().and_then(|p| p.tagged.as_ref()))
            .unwrap();
        assert_eq!(
            variant.ordinal, 1,
            "the initial different tag must decline the numeric loan"
        );
        let cells = [3, 7, 11, 13, 17, 19].map(Data::U64);
        let arguments = args(&artifact, &memory, &cells, Data::U64(23), 1, 4, 1);
        let before = contents(&arguments);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let (result, observed) = if restored {
                observe::run_restored(&jit, &arguments, &mut out, mode.execution(), &memory)
            } else {
                observe::run(&jit, &arguments, &mut out, mode.execution(), &memory)
            };
            result.unwrap();
            assert!(
                !observed.tail_entries.is_empty(),
                "ordinary transition must permit native reentry"
            );
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix([23, 3, 7, 13, 17, 19].map(Data::U64).into()),
                        Data::Matrix(cells.clone().into())
                    ]
                    .into()
                )
            );
            assert_eq!(contents(&arguments), before);
        }
    }
}

#[test]
fn tagged_numeric_phases_refused_hot_run_does_not_retry_every_iteration() {
    let source = source("u64").replace(
        "~next := data\n        next[row,at] = carried",
        "next := data",
    );
    for artifact in artifacts(&source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let memory = mode.memory_budget();
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        assert!(
            jit.descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .any(|r| r.numeric.as_ref().is_some_and(|p| p.tagged.is_some()))
        );
        let cells = [3, 7, 11, 13, 17, 19].map(Data::U64);
        let arguments = args(&artifact, &memory, &cells, Data::U64(23), 1, 4, 1);
        let mut policy = mode.execution();
        // Scalar reads and three-field payload packs fit. The native array
        // output envelope conservatively declines its six-element entry.
        policy.library.max_output_elements = Some(3);
        for restored in [false, true] {
            let mut out = vec![None; artifact.outputs().len()];
            let (result, observed) = if restored {
                observe::run_restored(&jit, &arguments, &mut out, policy, &memory)
            } else {
                observe::run(&jit, &arguments, &mut out, policy, &memory)
            };
            result.unwrap();
            assert!(observed.tail_entries.is_empty());
            let attempts = observed
                .helpers
                .iter()
                .filter(|((_, op), _)| *op == GlobalHelper::ClosedTailBegin.raw())
                .map(|(_, n)| *n)
                .sum::<u64>();
            assert_eq!(
                attempts, 2,
                "one initial preview and one fresh hot-run preview: {observed:?}"
            );
            assert_eq!(
                out.last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(
                    vec![
                        Data::Matrix(cells.clone().into()),
                        Data::Matrix(cells.clone().into())
                    ]
                    .into()
                )
            );
        }
    }
}
