//! Independently check current affine reads at both supported storage widths.
use super::*;

fn source(forward_wide: bool, reverse_wide: bool, delta: usize) -> String {
    let mut source = child_prepayment::source().replace("3u64", &format!("{delta}u64"));
    if forward_wide {
        source = source
            .replace("costs<[u32]>", "costs<[u64]>")
            .replace("prices<[u32]>", "prices<[u64]>")
            .replace(
                "costs[bits/wrapping-add(at, 1u64)] | false => 0u32)",
                "costs[bits/wrapping-add(at, 1u64)] | false => 0u64)",
            );
    }
    if !reverse_wide {
        source = source.replace("nodes<[u64]>", "nodes<[u32]>").replace(
            "nodes[bits/wrapping-add(reverse, 1u64)] | false => 0u64)",
            "nodes[bits/wrapping-add(reverse, 1u64)] | false => 0u32)",
        );
    }
    source.replace("node := ((", "raw-node := ((").replace(
        "wide<u64> := value",
        "node<u64> := raw-node\n        wide<u64> := value",
    )
}

fn matrix(values: &[u64], wide: bool) -> Data {
    Data::Matrix(
        values
            .iter()
            .map(|&v| {
                if wide {
                    Data::U64(v)
                } else {
                    Data::U32(v as u32)
                }
            })
            .collect::<Vec<_>>()
            .into(),
    )
}

pub(super) fn check_current_windows() {
    let mode = NativeProgramLimitMode::NoLimits;
    for (forward_wide, reverse_wide, delta) in [
        (false, false, 1),
        (true, false, 7),
        (false, true, 7),
        (true, true, 1),
    ] {
        for artifact in artifacts(&source(forward_wide, reverse_wide, delta)) {
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
                "widths={forward_wide}/{reverse_wide}, delta={delta}: {metadata}"
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
                    // Different current lengths and values on the same code;
                    // no address or captured length may survive an invocation.
                    for size in [40, 65, 300] {
                        let prices = (0..size + 5)
                            .map(|i| ((i as u64 + size as u64) << 32) | ((i * 17 + size) as u64))
                            .collect::<Vec<_>>();
                        let nodes = (0..size)
                            .map(|i| {
                                ((i as u64 * 29 + 1) << 32) | u64::from(u32::MAX - (i * 41) as u32)
                            })
                            .collect::<Vec<_>>();
                        for (start, stop) in [
                            (0, 0),
                            (0, 1),
                            (1, 4),
                            (1, 5),
                            (1, 6),
                            (1, 7),
                            (1, 8),
                            (1, 9),
                            (1, 16),
                            (1, 17),
                            (1, 18),
                            (9, 28),
                            (33, 32),
                            (
                                size as u64 - delta as u64 - 3,
                                size as u64 - delta as u64 + 1,
                            ),
                        ] {
                            for seed in [2, u32::MAX] {
                                let args = inputs(
                                    &artifact,
                                    &[
                                        ("prices", matrix(&prices, forward_wide)),
                                        ("nodes", matrix(&nodes, reverse_wide)),
                                        ("start", Data::U64(start)),
                                        ("stop", Data::U64(stop)),
                                        ("seed", Data::U32(seed)),
                                        ("nonce", Data::U64(119)),
                                        ("fault", Data::U64(u64::MAX)),
                                    ],
                                    &memory,
                                );
                                let aliases = args.clone();
                                let prior = contents(&args);
                                let report = if restored {
                                    observe::run_restored(
                                        jit,
                                        &args,
                                        &mut outputs,
                                        mode.execution(),
                                        &memory,
                                    )
                                    .0
                                } else {
                                    observe::run(
                                        jit,
                                        &args,
                                        &mut outputs,
                                        mode.execution(),
                                        &memory,
                                    )
                                    .0
                                };
                                report.unwrap();
                                let (mut best, mut item) = (seed, 119);
                                for k in start..stop {
                                    let at = k as usize + delta;
                                    let cost =
                                        (prices[at] as u32).wrapping_add(nodes[size - at] as u32);
                                    if cost < best {
                                        best = cost;
                                        item = k + 1;
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
                                        vec![
                                            Data::U32(best),
                                            Data::U64(item),
                                            Data::U64(start.max(stop))
                                        ]
                                        .into()
                                    )
                                );
                                assert_eq!(contents(&args), prior);
                                assert_eq!(contents(&aliases), prior);
                            }
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
