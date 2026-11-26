//! Exact scalar loop cells reuse current loans, source ordering and publication.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::prepare::readonly_numeric::Operation;
use mech_core::snapshot::F64Bits;

pub(super) fn cases() -> Vec<(&'static str, Vec<Data>, Data)> {
    vec![
        (
            "bool",
            [false, true, false, true, true, false]
                .map(Data::Bool)
                .into(),
            Data::Bool(true),
        ),
        (
            "u8",
            [3, 255, 1, 0, 17, 9].map(Data::U8).into(),
            Data::U8(255),
        ),
        (
            "u16",
            [3, 65535, 1, 0, 17, 9].map(Data::U16).into(),
            Data::U16(65535),
        ),
        (
            "u32",
            [3, u32::MAX, 1, 0, 17, 9].map(Data::U32).into(),
            Data::U32(u32::MAX),
        ),
        (
            "u64",
            [3, u64::MAX, 1, 0, 17, 9].map(Data::U64).into(),
            Data::U64(u64::MAX),
        ),
        (
            "i64",
            [i64::MIN, i64::MAX, -1, 0, 17, -9].map(Data::I64).into(),
            Data::I64(i64::MAX),
        ),
        (
            "f64",
            [
                1 << 63,
                0x7ff8_0000_0000_0037,
                0x7ff0_0000_0000_0000,
                0xfff0_0000_0000_0000,
                0,
                0x3ff8_0000_0000_0000,
            ]
            .map(|b| Data::F64(F64Bits::from_bits(b)))
            .into(),
            Data::F64(F64Bits::from_bits(0x7ff8_0000_0000_0091)),
        ),
    ]
}
fn source(kind: &str, nested: bool) -> String {
    let parent = if nested {
        "outer(data<[K]:2,3>, round<u64>, rounds<u64>, start<u64>, end<u64>, marker<K>) => (out<[K]:2,3>) :=\n  more := round < rounds\n  next := (more? | true => scan(data,start,end,marker) | false => data)\n  out := (more? | true => outer(next,round + 1u64,rounds,start,end,marker) | false => next).\nanswer := outer(data,0u64,2u64,start<u64>,end<u64>,marker<K>)\n"
    } else {
        "answer := scan(data,start<u64>,end<u64>,marker<K>)\n"
    };
    format!(
        r#"put(data<[K]:2,3>, at<u64>, value<K>) => (out<[K]:2,3>) :=
  ~next := data
  next[at] = value
  out := next.
scan(data<[K]:2,3>, at<u64>, end<u64>, marker<K>) => (out<[K]:2,3>) :=
  more := at < end
  old := (more? | true => data[at] | false => marker)
  next := (more? | true => put(data,at,marker) | false => data)
  out := (more? | true => scan(next,at + 1u64,end,old) | false => next).
data := input<[K]:2,3>
{parent}(answer,data)
"#
    )
    .replace('K', kind)
}
fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    cells: &[Data],
    marker: Data,
    start: u64,
    end: u64,
) -> Vec<Value> {
    super::numeric_maps::typed_arguments(
        artifact,
        memory,
        &[
            ("input", Data::Matrix(cells.to_vec().into()), &[2, 3]),
            ("marker", marker, &[]),
            ("start", Data::U64(start), &[]),
            ("end", Data::U64(end), &[]),
        ],
    )
}
fn last(values: &[Option<Value>]) -> Data {
    values
        .last()
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
fn typed_numeric_phases_select_real_loops_keep_all_exact_words_aliases_bounds_and_retry() {
    for (kind, cells, marker) in cases() {
        for nested in [false, true] {
            for artifact in artifacts(&source(kind, nested)) {
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
                            .any(|(_, a)| matches!(a, Operation::Read { .. }))
                            && p.writes())),
                    "admit actual {kind} numeric read/write loop"
                );
                crate::native_artifact::emit_program_object(&artifact, &catalog).unwrap();
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    for (start, end, no_op) in [
                        (1, 7, false),
                        (1, 1, false),
                        (2, 4, false),
                        (1, 2, true),
                        (0, 1, false),
                        (1, 8, false),
                        (1, 7, false),
                    ] {
                        let value = if no_op {
                            cells[0].clone()
                        } else {
                            marker.clone()
                        };
                        let args = arguments(&artifact, &memory, &cells, value.clone(), start, end);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let prior_fast = fast.clone();
                        let prior_cold = cold.clone();
                        let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                        let (a, counts) =
                            run(&jit, &args, &mut fast, &memory, mode.execution(), restored);
                        let (b, _) = run(
                            &ordinary,
                            &args,
                            &mut cold,
                            &memory,
                            mode.execution(),
                            restored,
                        );
                        if start == 0 || end > 7 {
                            assert_eq!(
                                format!("{:?}", a.unwrap_err()),
                                format!("{:?}", b.unwrap_err()),
                                "{kind} {start}..{end}"
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
                            let mut changed = cells.clone();
                            let mut changed_ever = false;
                            for _ in 0..if nested { 2 } else { 1 } {
                                let mut incoming = value.clone();
                                for at in start..end {
                                    let physical = ((at - 1) % 2 * 3 + (at - 1) / 2) as usize;
                                    changed_ever |= incoming != changed[physical];
                                    std::mem::swap(&mut incoming, &mut changed[physical]);
                                }
                            }
                            assert_eq!(
                                last(&fast),
                                Data::Tuple(
                                    vec![
                                        Data::Matrix(changed.into()),
                                        Data::Matrix(cells.clone().into())
                                    ]
                                    .into()
                                )
                            );
                            for (_, id, _) in &counts.tail_entries {
                                if let Some(proof) =
                                    &jit.descriptor().metadata().closed_tails.regions[*id].numeric
                                {
                                    for (site, action) in &proof.actions {
                                        if matches!(
                                            action,
                                            Operation::Read { .. } | Operation::Write { .. }
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
                                                0,
                                                "native cells must not dispatch one helper per iteration"
                                            );
                                        }
                                    }
                                }
                            }
                            assert!(
                                !counts.tail_entries.is_empty(),
                                "actual native entry for {kind}"
                            );
                            if !changed_ever {
                                assert!(
                                    counts.first_change_sites.is_empty(),
                                    "identity stores keep their immutable backing"
                                );
                            } else {
                                assert!(
                                    !counts.first_change_sites.is_empty(),
                                    "changed {kind} nested={nested} {start}..{end}: {counts:?}"
                                );
                                assert!(
                                    counts
                                        .first_change_backings
                                        .iter()
                                        .all(|(old, new)| old != new),
                                    "live input aliases force one first-change copy"
                                );
                            }
                        }
                        assert_eq!(contents(&args), before);
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
fn typed_numeric_phases_keep_finite_admission_atomic_outputs_and_recovery() {
    let (_, cells, marker) = cases().pop().unwrap();
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
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let args = arguments(&artifact, &memory, &cells, marker.clone(), 1, 7);
            let before = contents(&args);
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
                        7 => policy.execution.max_work = cap,
                        _ => unreachable!(),
                    }
                    let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut fast = vec![None; artifact.outputs().len()];
                    let mut cold = fast.clone();
                    let (a, _) = run(&jit, &args, &mut fast, &memory, policy, restored);
                    let (b, _) = run(&ordinary, &args, &mut cold, &memory, policy, restored);
                    assert_eq!(
                        a.is_ok(),
                        b.is_ok(),
                        "finite {dimension} cap{cap}: {a:?} / {b:?}"
                    );
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
                &args,
                &mut output,
                &memory,
                mode.execution(),
                restored,
            )
            .0
            .unwrap();
            assert_eq!(contents(&args), before);
            drop((args, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
