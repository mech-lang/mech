//! The generic source witness must select the actual vector-prefix recipe.
use super::*;

#[test]
fn numeric_ordered_reduction_selects_current_total_source_recipe() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&child_prepayment::source()) {
        let native = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let metadata = native.metadata();
        let windows = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|name| metadata[name].as_array().into_iter().flatten())
        .flat_map(|loop_| {
            loop_["guarded_loop_windows"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
        assert!(
            windows.iter().any(|w| !w["reduction"].is_null()),
            "{windows:?}"
        );
    }
    super::ordered_reduction_geometry::check_current_windows();
}

#[test]
fn numeric_ordered_reduction_preserves_prefix_remainders_wrapping_and_global_ties() {
    let mode = NativeProgramLimitMode::NoLimits;
    for artifact in artifacts(&child_prepayment::source()) {
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
                for pattern in 0..7 {
                    let mut prices = vec![u32::MAX; 300];
                    let mut nodes = vec![u64::MAX; 300];
                    if pattern == 1 {
                        prices.fill(99);
                        nodes.fill(0);
                        prices[4] = 2;
                        prices[7] = 2;
                    } else if pattern == 2 {
                        for (at, p) in prices.iter_mut().enumerate() {
                            *p = (at as u32).wrapping_mul(0x5fff_ffff);
                        }
                        for (at, p) in nodes.iter_mut().enumerate() {
                            *p = u64::MAX.wrapping_sub((at as u64).wrapping_mul(0x1_ffff_fffd));
                        }
                    } else if pattern >= 3 {
                        // Each possible vector lane wins, with ties across
                        // groups and distinct ignored high halves in every
                        // node. The independent scalar oracle below retains
                        // the earliest current source index, including tails.
                        prices.fill(0);
                        for k in 0..=297 {
                            let low = if k % 4 == pattern - 3 {
                                1
                            } else {
                                u32::MAX - k as u32
                            };
                            let high = (k as u64).wrapping_mul(0x9e37_79b9) ^ 0xa5a5_5a5a;
                            nodes[297 - k] = (high << 32) | u64::from(low);
                        }
                    }
                    for (start, stop) in [
                        (0, 0),
                        (0, 1),
                        (0, 2),
                        (0, 3),
                        (0, 4),
                        (0, 5),
                        (0, 6),
                        (0, 7),
                        (0, 8),
                        (0, 9),
                        (0, 10),
                        (0, 11),
                        (0, 12),
                        (0, 13),
                        (0, 14),
                        (0, 15),
                        (0, 16),
                        (0, 17),
                        (0, 18),
                        (0, 19),
                        (0, 20),
                        (0, 31),
                        (0, 32),
                        (0, 33),
                        (0, 255),
                        (0, 256),
                        (0, 258),
                        (1, 16),
                        (1, 17),
                        (1, 18),
                        (16, 31),
                        (16, 32),
                        (16, 33),
                        (17, 258),
                        (31, 32),
                        (31, 33),
                        (32, 32),
                        (33, 32),
                    ] {
                        for seed in [u32::MAX, u32::MAX - 1, 2] {
                            let args = inputs(
                                &artifact,
                                &[
                                    (
                                        "prices",
                                        Data::Matrix(
                                            prices
                                                .iter()
                                                .copied()
                                                .map(Data::U32)
                                                .collect::<Vec<_>>()
                                                .into(),
                                        ),
                                    ),
                                    (
                                        "nodes",
                                        Data::Matrix(
                                            nodes
                                                .iter()
                                                .copied()
                                                .map(Data::U64)
                                                .collect::<Vec<_>>()
                                                .into(),
                                        ),
                                    ),
                                    ("start", Data::U64(start)),
                                    ("stop", Data::U64(stop)),
                                    ("seed", Data::U32(seed)),
                                    ("nonce", Data::U64(119)),
                                    ("fault", Data::U64(u64::MAX)),
                                ],
                                &memory,
                            );
                            let aliases = args.clone();
                            let before = contents(&args);
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
                                observe::run(jit, &args, &mut outputs, mode.execution(), &memory).0
                            };
                            report.unwrap();
                            let (mut best, mut item) = (seed, 119u64);
                            for k in start..stop {
                                let at = (k + 3) as usize;
                                let cost =
                                    (u64::from(prices[at]).wrapping_add(nodes[300 - at])) as u32;
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
                                ),
                                "pattern={pattern},start={start},stop={stop},seed={seed},restored={restored}"
                            );
                            assert_eq!(contents(&args), before);
                            assert_eq!(contents(&aliases), before);
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

#[test]
fn numeric_ordered_reduction_refuses_changed_ties_predicates_and_carried_dependencies() {
    let base = child_prepayment::source();
    let unequal_item = {
        let at = base.rfind("cost < best").unwrap();
        let mut source = base.clone();
        source.replace_range(at..at + "cost < best".len(), "cost == best");
        source
    };
    for source in [
        base.replace("cost < best", "cost <= best"),
        unequal_item,
        base.replace(
            "bits/wrapping-add(wide, node)",
            "bits/wrapping-add(bits/wrapping-add(wide, node), item)",
        ),
    ] {
        assert_ne!(source, base);
        for artifact in artifacts(&source) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let count = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().into_iter().flatten())
            .flat_map(|region| {
                region["guarded_loop_windows"]
                    .as_array()
                    .into_iter()
                    .flatten()
            })
            .filter(|w| !w["reduction"].is_null())
            .count();
            assert_eq!(count, 0, "{source}");
        }
    }
}
