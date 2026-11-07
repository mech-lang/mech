//! A generic early-exit recurrence, with the same admitted loop but funding off.
use super::captured_windows::arguments;
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;
use crate::native_artifact::program::codegen::numeric_child_funding::{
    begin_test_observation, take_test_entries,
};

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/read-only-chunk-loop.mec");

fn gates(metadata: &serde_json::Value) -> usize {
    [
        "compiled_readonly_numeric_loops",
        "compiled_numeric_write_loops",
    ]
    .into_iter()
    .flat_map(|k| metadata[k].as_array().unwrap())
    .flat_map(|r| r["guarded_loop_windows"].as_array().unwrap())
    .filter(|v| v["funding"].is_object())
    .inspect(|v| {
        assert_eq!(v["funding"]["step"], 8);
        assert!(
            v["prepaid"].is_null(),
            "funding is not whole-loop prepayment"
        );
        assert_eq!(v["captured_windows"].as_array().unwrap().len(), 2);
    })
    .count()
}

#[test]
fn bounded_child_funding_uses_current_generic_recurrence_and_refuses_changed_captures() {
    let _observation = begin_test_observation();
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        assert!(gates(&object.metadata()) > 0);
    }
    for changed in [
        SOURCE.replace("at + 8u64 <= limit", "bits/wrapping-add(at, 8u64) <= limit"),
        SOURCE.replace(
            "bits/wrapping-add(at, 8u64), limit)",
            "bits/wrapping-add(at, 8u64), limit - 1u64)",
        ),
    ] {
        assert_ne!(SOURCE, changed);
        for artifact in artifacts(&changed) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            assert_eq!(gates(&object.metadata()), 0);
        }
    }
}

fn pair(artifact: &ProgramArtifact) -> [JitProgramArtifact; 2] {
    let catalog = mech_stdlib::source_catalog();
    [
        JitProgramArtifact::compile_for_test(artifact, &catalog, OptimizationPass::ChildFunding)
            .unwrap(),
        JitProgramArtifact::compile_with_limits(
            artifact,
            &catalog,
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap(),
    ]
}

#[test]
fn bounded_child_funding_preserves_actual_work_finite_prefixes_aliases_cleanup_and_retry() {
    let _observation = begin_test_observation();
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(SOURCE) {
        let pair = pair(&artifact);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            let run = |jit, args: &[Value], published: &mut [Option<Value>], limits| {
                take_test_entries();
                let (result, counts) = if restored {
                    observe::run_restored(jit, args, published, limits, &memory)
                } else {
                    observe::run(jit, args, published, limits, &memory)
                };
                (result, counts, take_test_entries())
            };
            let good = arguments(&artifact, &memory, [1, 256], &[0; 256], [128, 48, 0, 64, 3]);
            let mut full_work = 0;
            let mut entry_spent = 0;
            for side in 0..2 {
                let (result, counts, entered) =
                    run(&pair[side], &good, &mut output[side], mode.execution());
                let result = result.unwrap();
                assert_eq!(entered, if side == 0 { 0 } else { 3 });
                assert_eq!(
                    output[side]
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(192)
                );
                if side == 0 {
                    full_work = result.work;
                    entry_spent = u64::MAX - counts.phase_initial_fuel[0];
                } else {
                    assert_eq!(result.work, full_work);
                }
            }
            let phase = pair[1]
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .filter_map(|r| r.numeric.as_ref())
                .find_map(|p| p.phase.as_ref().filter(|p| p.checked_children))
                .unwrap();
            let child = phase
                .loops
                .iter()
                .filter(|d| d.first_visit.is_some())
                .max_by_key(|d| d.compute)
                .unwrap();
            let inside = entry_spent + phase.loops[0].compute + 3 * child.compute;
            let mut limits = vec![mode.execution(); 10];
            for (i, work) in [
                0,
                1000,
                inside,
                full_work / 2,
                full_work - 1,
                full_work,
                full_work + 1,
            ]
            .into_iter()
            .enumerate()
            {
                limits[i].execution.max_work = work;
            }
            limits[7].library.max_compute_work = Some(0);
            limits[8].library.max_temporary_bytes = Some(0);
            limits[9].execution.max_work = 1 << 60;
            limits[9].library.max_compute_work = Some(1 << 60);
            limits[9].library.max_comparison_work = Some(1 << 60);
            let mut cases = Vec::new();
            for dims in [[1, 256], [256, 1], [16, 16], [1, 7], [0, 1]] {
                for mismatch in (0..=17).chain([31, 32, 63, usize::MAX]) {
                    let mut bytes = vec![0; (dims[0] * dims[1]) as usize];
                    if 48 + mismatch.min(256) < bytes.len() {
                        bytes[48 + mismatch] = 1;
                    }
                    cases.push((dims, bytes, [128, 48, 0, 64, 3]));
                }
            }
            for coords in [
                [128, 48, 3, 63, 3],
                [128, 48, 0, 7, 3],
                [128, 48, 17, 16, 2],
                [u64::MAX, 48, 0, 16, 2],
                [128, 48, u64::MAX, 0, 1],
                [128, 48, u64::MAX, 0, 0],
            ] {
                cases.push(([1, 256], vec![0; 256], coords));
            }
            for (dims, bytes, coords) in cases {
                let args = arguments(&artifact, &memory, dims, &bytes, coords);
                let before = contents(&args);
                let aliases = args.clone();
                let mut prices = Vec::new();
                for side in 0..2 {
                    let prior = output[side].clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _, entered) =
                        run(&pair[side], &args, &mut output[side], mode.execution());
                    if side == 0 || coords[4] == 0 {
                        assert_eq!(entered, 0);
                    }
                    match result {
                        Ok(v) => prices.push(Ok((v.work, v.comparison_work))),
                        Err(e) => {
                            read_packs::unchanged_publication(&output[side], &prior);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                            prices.push(Err(format!("{e:?}")));
                        }
                    }
                }
                assert_eq!(prices[0], prices[1], "{dims:?} {coords:?}");
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                if prices[0].is_ok() {
                    assert_eq!(
                        contents(
                            &output[0]
                                .iter()
                                .map(|x| x.as_ref().unwrap().clone())
                                .collect::<Vec<_>>()
                        ),
                        contents(
                            &output[1]
                                .iter()
                                .map(|x| x.as_ref().unwrap().clone())
                                .collect::<Vec<_>>()
                        )
                    );
                }
            }
            let mut saw_header = false;
            for policy in limits {
                let mut outcomes = Vec::new();
                let mut settlements = Vec::new();
                let mut retries = Vec::new();
                for side in 0..2 {
                    let prior = output[side].clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts, entered) =
                        run(&pair[side], &good, &mut output[side], policy);
                    if policy.execution.max_work == 1 << 60 {
                        assert_eq!(entered, if side == 0 { 0 } else { 3 });
                    }
                    saw_header |= counts.child_fuel_exits.iter().any(|(_, header)| *header);
                    settlements.push(counts.phase_settlements);
                    match result {
                        Ok(v) => outcomes.push(Ok((v.work, v.comparison_work))),
                        Err(e) => {
                            read_packs::unchanged_publication(&output[side], &prior);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                            outcomes.push(Err(format!("{e:?}")));
                        }
                    }
                    let (result, _, entered) =
                        run(&pair[side], &good, &mut output[side], mode.execution());
                    let result = result.unwrap();
                    retries.push((result.work, result.comparison_work));
                    assert_eq!(
                        output[side]
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::U64(192)
                    );
                    assert_eq!(entered, if side == 0 { 0 } else { 3 });
                }
                if policy.execution.max_work == 1 << 60 {
                    assert!(outcomes[0].is_ok());
                }
                assert_eq!(outcomes[0], outcomes[1]);
                assert_eq!(settlements[0], settlements[1]);
                assert_eq!(retries[0], retries[1]);
            }
            assert!(
                saw_header,
                "finite cases must exercise checked header fallback"
            );
            drop((good, output));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn bounded_child_funding_keeps_a_fallible_operation_inside_the_funded_visit() {
    let _observation = begin_test_observation();
    let source = SOURCE
        .replace(
            "  out := bits/xor(right-word, left-word).",
            "  difference := bits/xor(right-word, left-word)\n  out := difference + 1u64.",
        )
        .replace("| 0u64 => compare-chunks", "| 1u64 => compare-chunks");
    assert_ne!(source, SOURCE);
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source) {
        let pair = pair(&artifact);
        for restored in [false, true] {
            let memory = mode.memory_budget();
            let mut output = [
                vec![None; artifact.outputs().len()],
                vec![None; artifact.outputs().len()],
            ];
            for bad in [false, true, false] {
                let mut bytes = vec![0; 256];
                if bad {
                    bytes[56..64].fill(255);
                }
                let args = arguments(&artifact, &memory, [1, 256], &bytes, [128, 48, 0, 64, 3]);
                let aliases = args.clone();
                let before = contents(&args);
                let mut outcomes = Vec::new();
                for side in 0..2 {
                    let prior = output[side].clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    take_test_entries();
                    let (result, _) = if restored {
                        observe::run_restored(
                            &pair[side],
                            &args,
                            &mut output[side],
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(
                            &pair[side],
                            &args,
                            &mut output[side],
                            mode.execution(),
                            &memory,
                        )
                    };
                    let entered = take_test_entries();
                    if side == 1 {
                        assert!(entered > 0, "error must occur inside funded code");
                    } else {
                        assert_eq!(entered, 0);
                    }
                    match result {
                        Ok(v) => {
                            assert!(!bad);
                            outcomes.push(Ok(v.work));
                        }
                        Err(e) => {
                            assert!(bad);
                            let e = format!("{e:?}");
                            assert!(e.contains("Arithmetic"));
                            read_packs::unchanged_publication(&output[side], &prior);
                            assert_eq!(
                                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                held
                            );
                            outcomes.push(Err(e));
                        }
                    }
                }
                assert_eq!(outcomes[0], outcomes[1]);
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
            }
            drop(output);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
