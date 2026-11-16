//! Actual visit settlement is independent of chunk width and start alignment.
use super::*;
use crate::native_artifact::program::codegen::numeric_child_funding::{
    begin_test_observation, take_test_entries,
};

#[test]
fn funded_visit_displacements_preserve_strides_prefixes_and_retry() {
    let _observation = begin_test_observation();
    let base = include_str!("../../../../../tests/fixtures/native/read-only-chunk-loop.mec");
    for stride in [8, 12, 16, 24] {
        let source = base
            .replace("at + 8u64 <= limit", &format!("at + {stride}u64 <= limit"))
            .replace(
                "bits/wrapping-add(at, 8u64), limit)",
                &format!("bits/wrapping-add(at, {stride}u64), limit)"),
            );
        for artifact in artifacts(&source) {
            let pair = child_funding::pair(&artifact);
            let mode = NativeProgramLimitMode::NoLimits;
            let mut selected = false;
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut outputs = [
                    vec![None; artifact.outputs().len()],
                    vec![None; artifact.outputs().len()],
                ];
                let run = |jit, args: &[Value], outputs: &mut [Option<Value>], limits| {
                    take_test_entries();
                    let report = if restored {
                        observe::run_restored(jit, args, outputs, limits, &memory).0
                    } else {
                        observe::run(jit, args, outputs, limits, &memory).0
                    };
                    (report, take_test_entries())
                };
                for (start, stop) in [(0, 64), (3, 67), (17, 63)] {
                    for mismatch in [0, 7, 8, 11, 12, 16, 23, 31, 63, usize::MAX] {
                        let mut bytes = vec![0; 256];
                        if mismatch != usize::MAX {
                            bytes[48 + mismatch] = 1;
                        }
                        let args = captured_windows::arguments(
                            &artifact,
                            &memory,
                            [1, 256],
                            &bytes,
                            // Keep the independent resident oracle within its
                            // unchanged default work cap. Existing funding
                            // tests separately exercise three-round handoffs.
                            [128, 48, start, stop, 1],
                        );
                        let aliases = args.clone();
                        let before = contents(&args);
                        let expected = ordered_reduction_exhaustion::resident(&artifact, &args);
                        for side in 0..2 {
                            run(&pair[side], &args, &mut outputs[side], mode.execution())
                                .0
                                .unwrap();
                        }
                        let mut finite = mode.execution();
                        finite.execution.max_work = 1 << 60;
                        finite.library.max_compute_work = Some(1 << 60);
                        finite.library.max_comparison_work = Some(1 << 60);
                        let mut denied = mode.execution();
                        denied.execution.max_work = 0;
                        for policy in [mode.execution(), denied, finite] {
                            let mut reports = Vec::new();
                            let mut retries = Vec::new();
                            for side in 0..2 {
                                let prior = outputs[side].clone();
                                let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                                let (result, entered) =
                                    run(&pair[side], &args, &mut outputs[side], policy);
                                selected |= side == 1 && entered > 0;
                                if side == 0 {
                                    assert_eq!(entered, 0);
                                }
                                match result {
                                    Ok(report) => {
                                        assert_eq!(
                                            outputs[side]
                                                .iter()
                                                .map(|v| v
                                                    .as_ref()
                                                    .unwrap()
                                                    .canonical_data_draft()
                                                    .unwrap())
                                                .collect::<Vec<_>>(),
                                            expected
                                        );
                                        reports.push(Ok((report.work, report.comparison_work)));
                                    }
                                    Err(error) => {
                                        read_packs::unchanged_publication(&outputs[side], &prior);
                                        assert_eq!(
                                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                            held
                                        );
                                        reports.push(Err(format!("{error:?}")));
                                    }
                                }
                                let (retry, _) =
                                    run(&pair[side], &args, &mut outputs[side], mode.execution());
                                let retry = retry.unwrap();
                                retries.push((retry.work, retry.comparison_work));
                                assert_eq!(
                                    outputs[side]
                                        .iter()
                                        .map(|v| v
                                            .as_ref()
                                            .unwrap()
                                            .canonical_data_draft()
                                            .unwrap())
                                        .collect::<Vec<_>>(),
                                    expected
                                );
                            }
                            assert_eq!(
                                reports[0], reports[1],
                                "stride={stride}, start={start}, stop={stop}, mismatch={mismatch}"
                            );
                            assert_eq!(retries[0], retries[1]);
                        }
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                    }
                }
                drop(outputs);
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
            assert!(selected, "stride={stride} must execute the funded version");
        }
    }
}
