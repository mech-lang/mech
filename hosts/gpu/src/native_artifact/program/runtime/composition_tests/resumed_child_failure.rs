//! Fail only in a complete child reached after the real collection handoff.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn source() -> String {
    let source = SOURCE
        .replace(
            "poke(nodes<[u8]>)",
            "poke(nodes<[u8]>, read-at<u64>, write-at<u64>, divisor<u64>, amount<u64>)",
        )
        .replace("old<u64> := nodes[1u64]", "old<u64> := nodes[read-at]")
        .replace("bits/and(old + 1u64, 255u64)", "bits/and((old + amount) / divisor, 255u64)")
        .replace("next[1u64] = value", "next[write-at] = value")
        .replace(
            "leaf(nodes<[u8]>, at<u64>, end<u64>)",
            "leaf(nodes<[u8]>, at<u64>, end<u64>, read-at<u64>, write-at<u64>, divisor<u64>, amount<u64>)",
        )
        .replace("poke(nodes)", "poke(nodes, read-at, write-at, divisor, amount)")
        .replace(
            "leaf(next, at + 1u64, end)",
            "leaf(next, at + 1u64, end, read-at, write-at, divisor, amount)",
        )
        .replace(
            "  next := (more? | true => leaf(nodes, 0u64, 1u64) | false => nodes)",
            "  fresh := at == 0u64\n  read-at := ((fresh || ((probe != 1u64) && (probe != 5u64)))? | true => 1u64 | false => 0u64)\n  write-at := ((fresh || ((probe != 2u64) && (probe != 6u64)))? | true => 1u64 | false => 0u64)\n  divisor := ((fresh || ((probe != 3u64) && (probe != 5u64) && (probe != 6u64)))? | true => 1u64 | false => 0u64)\n  amount := ((fresh || (probe != 4u64))? | true => 1u64 | false => 18446744073709551615u64)\n  next := (more? | true => leaf(nodes, 0u64, 1u64, read-at, write-at, divisor, amount) | false => nodes)",
        )
        .replace("current[probe + 1u64]", "current[1u64]")
        .replace("current[probe + 2u64]", "current[2u64]");
    assert!(!source.contains("leaf(nodes, 0u64, 1u64)"));
    source
}

fn arguments(artifact: &ProgramArtifact, memory: &ManagedMemoryBudget, fault: u64) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("nodes", Data::Matrix([129, 129, 7].map(Data::U8).into())),
            ("cache", Data::Matrix([0, 0].map(Data::U32).into())),
            ("steps", Data::U64(2)),
            ("checkpoint", Data::U64(u64::MAX)),
            ("child-checkpoint", Data::U64(0)),
            ("reject", Data::U64(u64::MAX)),
            ("child-reject", Data::U64(u64::MAX)),
            ("probe", Data::U64(fault)),
        ],
        memory,
    )
}

#[test]
fn resumed_child_rejections_keep_ordinary_errors_publication_aliases_and_retry() {
    let mut errors = [None, None, None, None, None, None];
    let mut expected = None;
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let catalog = mech_stdlib::source_catalog();
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &catalog,
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native =
            JitProgramArtifact::compile_with_limits(&artifact, &catalog, mode.compilation())
                .unwrap();
        for (reference, jit) in [(true, &ordinary), (false, &native)] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                // Faults 5/6 establish read-before-arithmetic and
                // arithmetic-before-write precedence in the secondary entry.
                for fault in [0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6, 0] {
                    let args = arguments(&artifact, &memory, fault);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut published, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut published, mode.execution(), &memory)
                    };
                    if fault == 0 {
                        result.unwrap();
                        let actual = published
                            .iter()
                            .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                            .collect::<Vec<_>>();
                        assert_eq!(actual, *expected.get_or_insert(actual.clone()));
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(
                            error.contains(if matches!(fault, 3 | 4 | 6) {
                                "Arithmetic"
                            } else {
                                "Library"
                            }),
                            "fault={fault}: {error}"
                        );
                        assert_eq!(
                            error,
                            *errors[fault as usize - 1].get_or_insert(error.clone())
                        );
                        assert!(published.iter().zip(&prior).all(|(actual, old)| {
                            actual
                                .as_ref()
                                .unwrap()
                                .shares_frozen_storage(old.as_ref().unwrap())
                        }));
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                        if !reference {
                            assert!(
                                counts.tail_boundaries.iter().any(|row| row.3.is_some()
                                    && row.2 == if matches!(fault, 3 | 4 | 6) { 3 } else { 2 }),
                                "a selected secondary must settle the error: {counts:?}"
                            );
                        }
                    }
                    if !reference {
                        assert!(
                            counts
                                .tail_entries
                                .iter()
                                .any(|(_, _, route)| route.is_some()),
                            "the secondary must actually execute: {counts:?}"
                        );
                        for row in counts.tail_boundaries.iter().filter(|row| row.3.is_some()) {
                            assert_eq!(row.3, row.4, "preserve the outer prepaid cursor: {row:?}");
                        }
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(published);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn resumed_child_finite_admission_keeps_prior_publication_and_recovery() {
    resources::finite_boundaries(&source(), |artifact, memory| arguments(artifact, memory, 0));
}
