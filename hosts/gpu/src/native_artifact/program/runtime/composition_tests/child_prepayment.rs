//! Complete child funding must retain source arithmetic and finite fallback.
use super::*;

fn source() -> String {
    let source = include_str!("../../../../../tests/fixtures/native/numeric-array-windows.mec");
    let fallible = "divisor := ((k == fault)? | true => 0u64 | false => 1u64)\n        checked := 10u64 / divisor";
    assert_eq!(source.matches(fallible).count(), 1);
    source.replace(fallible, "checked := 1u64")
}

#[test]
fn numeric_child_prepayment_preserves_current_views_and_zero_partial_visits() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source()) {
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for jit in [&ordinary, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for (size, start, stop) in [
                    (300, 0, 258),
                    (300, 17, 258),
                    (300, 258, 258),
                    (300, 257, 258),
                    (8, 0, 8),
                    (0, 0, 0),
                    (0, u64::MAX, u64::MAX),
                    (300, u64::MAX - 1, u64::MAX),
                    (300, 0, 7),
                ] {
                    let args = current_windows::arguments(
                        &artifact,
                        &memory,
                        size,
                        start,
                        stop,
                        u32::MAX,
                        u64::MAX,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    if restored {
                        observe::run_restored(jit, &args, &mut outputs, mode.execution(), &memory)
                            .0
                            .unwrap();
                    } else {
                        observe::run(jit, &args, &mut outputs, mode.execution(), &memory)
                            .0
                            .unwrap();
                    }
                    let (mut best, mut item) = (u32::MAX, 119);
                    for k in start..stop {
                        let at = k.wrapping_add(3);
                        let price = if at < size as u64 {
                            (at as u32).wrapping_mul(17).wrapping_add(11)
                        } else {
                            0
                        };
                        let reverse = (size as u64).wrapping_sub(at);
                        let node = if reverse < size as u64 {
                            reverse.wrapping_mul(0x1_0000_0029)
                        } else {
                            0
                        };
                        let cost = u64::from(price).wrapping_add(node) as u32;
                        if cost < best {
                            best = cost;
                            item = k.checked_add(1).unwrap();
                        }
                    }
                    assert_eq!(
                        outputs
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(
                            vec![Data::U32(best), Data::U64(item), Data::U64(start.max(stop))]
                                .into()
                        )
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
    resources::finite_boundaries(&source(), |artifact, memory| {
        current_windows::arguments(artifact, memory, 300, 0, 7, u32::MAX, u64::MAX)
    });
}

#[test]
fn numeric_child_prepayment_reports_complete_funded_visits() {
    for artifact in artifacts(&source()) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let prepaid = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|key| metadata[key].as_array().unwrap())
        .flat_map(|region| {
            region["guarded_loop_windows"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .filter(|window| window["prepaid"].is_object())
        .count();
        assert!(
            prepaid > 0,
            "complete proved children require a funded visit count; found {prepaid}"
        );
    }
}

#[test]
fn numeric_child_prepayment_executes_with_finite_funding_and_recovers() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source()) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for restored in [false, true] {
            let memory = mode.memory_budget().with_limits(1 << 40, 1 << 40);
            let mut outputs = vec![None; artifact.outputs().len()];
            for (start, stop) in [(0, 258), (17, 258), (258, 258), (0, 0)] {
                let args = current_windows::arguments(
                    &artifact,
                    &memory,
                    300,
                    start,
                    stop,
                    u32::MAX,
                    u64::MAX,
                );
                let aliases = args.clone();
                let before = contents(&args);
                let run = |outputs: &mut [Option<Value>], limits| {
                    if restored {
                        observe::run_restored(&jit, &args, outputs, limits, &memory)
                    } else {
                        observe::run(&jit, &args, outputs, limits, &memory)
                    }
                };
                let unlimited = run(&mut outputs, mode.execution()).0.unwrap();
                let mut limits = mode.execution();
                limits.execution.max_work = 1 << 60;
                limits.execution.max_transport_bytes = 1 << 40;
                limits.library.max_compute_work = Some(1 << 60);
                limits.library.max_comparison_work = Some(1 << 60);
                let (result, counts) = run(&mut outputs, limits);
                let result = result.unwrap();
                // These two runs already report different work in the clean
                // parent. Compare each configuration across revisions.
                assert_eq!(result.comparison_work, unlimited.comparison_work);
                assert!(
                    counts.phases > 0,
                    "finite funding must select the native phase: {counts:?}"
                );
                let visits = counts
                    .phase_settlements
                    .iter()
                    .filter(|row| row.0 == 0)
                    .map(|row| row.2)
                    .sum::<u64>();
                assert_eq!(visits, stop.saturating_sub(start) + 4, "{counts:?}");
                let prior = outputs.clone();
                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                let mut denied = limits;
                denied.execution.max_work = 0;
                assert!(run(&mut outputs, denied).0.is_err());
                for (current, old) in outputs.iter().zip(&prior) {
                    assert!(
                        current
                            .as_ref()
                            .unwrap()
                            .shares_frozen_storage(old.as_ref().unwrap())
                    );
                }
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    held
                );
                run(&mut outputs, limits).0.unwrap();
                assert_eq!(contents(&args), before);
                assert_eq!(contents(&aliases), before);
                println!(
                    "{}",
                    serde_json::json!({"witness":"complete-child-finite-funded", "restored":restored, "start":start,"stop":stop,"selected_phases":counts.phases,"child_visits":visits,"work":result.work,"unlimited_work":unlimited.work,"comparison_work":result.comparison_work})
                );
            }
            drop(outputs);
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_child_prepayment_refuses_fallible_and_nonunit_children() {
    let fallible = include_str!("../../../../../tests/fixtures/native/numeric-array-windows.mec");
    let stride = source().replace(
        ":Try(k + 1u64, next-best, next-item)",
        ":Try(k + 2u64, next-best, next-item)",
    );
    let early = source().replace(
        "| k < stop ->",
        "| k == 3u64 -> :Done(k, best, item)\n    | k < stop ->",
    );
    for source in [fallible, &stride, &early] {
        for artifact in artifacts(source) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            for region in [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().unwrap())
            {
                for window in region["guarded_loop_windows"].as_array().unwrap() {
                    assert!(
                        window["prepaid"].is_null(),
                        "an incomplete or fallible child must retain its per-visit charges: {window}"
                    );
                }
            }
        }
    }
}

#[test]
fn numeric_child_prepayment_preserves_wrapping_sums_and_first_ties() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&source()) {
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for jit in [&ordinary, &native] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = vec![None; artifact.outputs().len()];
                for seed in [u32::MAX, u32::MAX - 1] {
                    let args = inputs(
                        &artifact,
                        &[
                            ("prices", Data::Matrix(vec![Data::U32(u32::MAX); 8].into())),
                            ("nodes", Data::Matrix(vec![Data::U64(u64::MAX); 8].into())),
                            ("start", Data::U64(0)),
                            ("stop", Data::U64(3)),
                            ("seed", Data::U32(seed)),
                            ("nonce", Data::U64(119)),
                            ("fault", Data::U64(u64::MAX)),
                        ],
                        &memory,
                    );
                    let aliases = args.clone();
                    let before = contents(&args);
                    if restored {
                        observe::run_restored(jit, &args, &mut outputs, mode.execution(), &memory)
                            .0
                            .unwrap();
                    } else {
                        observe::run(jit, &args, &mut outputs, mode.execution(), &memory)
                            .0
                            .unwrap();
                    }
                    let cost = u64::from(u32::MAX).wrapping_add(u64::MAX) as u32;
                    assert_eq!(cost, u32::MAX - 1);
                    let item = if cost < seed { 1 } else { 119 };
                    assert_eq!(
                        outputs
                            .last()
                            .unwrap()
                            .as_ref()
                            .unwrap()
                            .canonical_data_draft()
                            .unwrap(),
                        Data::Tuple(vec![Data::U32(cost), Data::U64(item), Data::U64(3)].into())
                    );
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}
