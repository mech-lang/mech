//! A complete reduction must still execute the exhausted source transition.
use super::*;

fn source(case: u8) -> String {
    let base = child_prepayment::source();
    let edge = "| * -> :Done(k, best, item)";
    assert_eq!(base.matches(edge).count(), 1);
    let replacement = match case {
        0 => "| * -> :Done(k, 0u32, item)",
        1 => "| * -> :Done(k, best, 0u64)",
        2 => "| * -> :Done(k, 0u32, bits/wrapping-add(item, 5u64))",
        _ => unreachable!(),
    };
    let once = "nodes<[u64]>, 2u64, stop";
    assert_eq!(base.matches(once).count(), 1);
    base.replace(edge, replacement)
        .replace(once, "nodes<[u64]>, 1u64, stop")
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    start: u64,
    stop: u64,
    seed: u32,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            ("prices", Data::Matrix(vec![Data::U32(9); 64].into())),
            ("nodes", Data::Matrix(vec![Data::U64(0); 64].into())),
            ("start", Data::U64(start)),
            ("stop", Data::U64(stop)),
            ("seed", Data::U32(seed)),
            ("nonce", Data::U64(119)),
            ("fault", Data::U64(u64::MAX)),
        ],
        memory,
    )
}

#[test]
fn numeric_ordered_reduction_preserves_exhausted_payload_updates() {
    let mode = NativeProgramLimitMode::NoLimits;
    for case in 0..3 {
        for artifact in artifacts(&source(case)) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            assert!(
                [
                    "compiled_readonly_numeric_loops",
                    "compiled_numeric_write_loops"
                ]
                .into_iter()
                .flat_map(|key| metadata[key].as_array().into_iter().flatten())
                .flat_map(|region| region["guarded_loop_windows"]
                    .as_array()
                    .into_iter()
                    .flatten())
                .any(|window| !window["reduction"].is_null()),
                "the optimized reduction must remain selected: case={case}"
            );
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &mech_stdlib::source_catalog(),
                crate::native_artifact::codegen::options::OptimizationPass::ScalarLoops,
            )
            .unwrap();
            for jit in [&ordinary, &native] {
                for restored in [false, true] {
                    let memory = mode.memory_budget();
                    let mut outputs = vec![None; artifact.outputs().len()];
                    for (start, stop) in [
                        (0, 0),
                        (0, 1),
                        (0, 15),
                        (0, 16),
                        (0, 17),
                        (0, 18),
                        (0, 19),
                        (0, 20),
                        (1, 18),
                        (17, 33),
                        (32, 32),
                        (33, 32),
                    ] {
                        for seed in [9, 100] {
                            let args = arguments(&artifact, &memory, start, stop, seed);
                            let aliases = args.clone();
                            let before = contents(&args);
                            let result = if restored {
                                observe::run_restored(
                                    jit,
                                    &args,
                                    &mut outputs,
                                    mode.execution(),
                                    &memory,
                                )
                                .0
                            } else {
                                observe::run(jit, &args, &mut outputs, mode.execution(), &memory).0
                            };
                            result.unwrap();
                            // All candidate costs are 9; strict ties retain the
                            // incoming item, and a first improvement picks start+1.
                            let (best, item) = if start < stop && seed > 9 {
                                (9, start + 1)
                            } else {
                                (seed, 119)
                            };
                            let (best, item) = match case {
                                0 => (0, item),
                                1 => (best, 0),
                                2 => (0, item + 5),
                                _ => unreachable!(),
                            };
                            assert_eq!(
                                outputs
                                    .last()
                                    .unwrap()
                                    .as_ref()
                                    .unwrap()
                                    .canonical_data_draft()
                                    .unwrap(),
                                Data::Tuple(
                                    vec![
                                        Data::U32(best),
                                        Data::U64(item),
                                        Data::U64(start.max(stop))
                                    ]
                                    .into()
                                ),
                                "case={case}, start={start}, stop={stop}, seed={seed}, restored={restored}"
                            );
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
            }
        }
    }
}

#[test]
fn numeric_ordered_reduction_exhausted_payload_keeps_finite_publication_and_retry() {
    resources::finite_boundaries(&source(2), |artifact, memory| {
        arguments(artifact, memory, 0, 18, 100)
    });
}
