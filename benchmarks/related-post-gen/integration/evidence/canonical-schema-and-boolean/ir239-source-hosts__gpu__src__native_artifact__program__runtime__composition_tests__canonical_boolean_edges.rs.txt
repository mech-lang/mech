//! Same-compiler branch-emission ablation preserves source work and cold exits.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    at: u64,
    gate: bool,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "bytes",
                Data::Matrix(
                    (0u8..32)
                        .map(|i| Data::U8(i.wrapping_mul(37).wrapping_add(29)))
                        .collect(),
                ),
            ),
            ("at", Data::U64(at)),
            ("steps", Data::U64(2)),
            ("seed", Data::U64(29)),
            ("gate", Data::Bool(gate)),
        ],
        memory,
    )
}

#[test]
fn canonical_boolean_branches_preserve_exact_work_errors_aliases_and_retry() {
    let mode = NativeProgramLimitMode::NoLimits;
    for true_first in [false, true] {
        for joined in [false, true] {
            for artifact in artifacts(&boolean_edges::alternate_source(true_first, joined)) {
                let pair = [
                    JitProgramArtifact::compile_with_limits(
                        &artifact,
                        &mech_stdlib::source_catalog(),
                        mode.compilation(),
                    )
                    .unwrap(),
                    JitProgramArtifact::compile_for_test(
                        &artifact,
                        &mech_stdlib::source_catalog(),
                        OptimizationPass::CanonicalBooleanBranches,
                    )
                    .unwrap(),
                ];
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut published = [
                        vec![None; artifact.outputs().len()],
                        vec![None; artifact.outputs().len()],
                    ];
                    let good = arguments(&artifact, &memory, 0, true);
                    let run = |jit: &JitProgramArtifact,
                               args: &[Value],
                               output: &mut [Option<Value>],
                               limits| {
                        if restored {
                            observe::run_restored(jit, args, output, limits, &memory).0
                        } else {
                            observe::run(jit, args, output, limits, &memory).0
                        }
                    };
                    let mut work = 0;
                    for side in 0..2 {
                        let report =
                            run(&pair[side], &good, &mut published[side], mode.execution())
                                .unwrap();
                        if side == 0 {
                            work = report.work;
                        } else {
                            assert_eq!(report.work, work);
                        }
                    }
                    for (at, gate, denial) in [
                        (0, true, "none"),
                        (0, false, "none"),
                        (24, true, "none"),
                        (24, false, "none"),
                        (25, true, "none"),
                        (u64::MAX, true, "none"),
                        (0, true, "work"),
                        (0, true, "late-work"),
                        (0, true, "compute"),
                        (0, true, "comparison"),
                    ] {
                        let args = arguments(&artifact, &memory, at, gate);
                        let aliases = args.clone();
                        let before = contents(&args);
                        let mut limits = mode.execution();
                        match denial {
                            "work" => limits.execution.max_work = 0,
                            "late-work" => limits.execution.max_work = work - 1,
                            "compute" => limits.library.max_compute_work = Some(0),
                            "comparison" => limits.library.max_comparison_work = Some(0),
                            _ => {}
                        }
                        let mut results = Vec::new();
                        for side in 0..2 {
                            let previous = published[side].clone();
                            let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                            let result = run(&pair[side], &args, &mut published[side], limits);
                            match result {
                                Ok(report) => {
                                    results.push(Ok((report.work, report.comparison_work)))
                                }
                                Err(error) => {
                                    read_packs::unchanged_publication(&published[side], &previous);
                                    assert_eq!(
                                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                                        held
                                    );
                                    results.push(Err(format!("{error:?}")));
                                }
                            }
                        }
                        assert_eq!(
                            results[0], results[1],
                            "at={at}, gate={gate}, denial={denial}, restored={restored}"
                        );
                        assert_eq!(contents(&args), before);
                        assert_eq!(contents(&aliases), before);
                        let output = |side: usize| {
                            published[side]
                                .iter()
                                .map(|v| v.as_ref().unwrap().canonical_data_draft().unwrap())
                                .collect::<Vec<_>>()
                        };
                        assert_eq!(output(0), output(1));
                        let mut retries = Vec::new();
                        for side in 0..2 {
                            let report =
                                run(&pair[side], &good, &mut published[side], mode.execution())
                                    .unwrap();
                            retries.push((report.work, report.comparison_work));
                        }
                        assert_eq!(retries[0], retries[1]);
                    }
                    drop((good, published));
                    assert_eq!(
                        (memory.used_bytes(), memory.retained_snapshot_nodes()),
                        (0, 0)
                    );
                }
            }
        }
    }
}
