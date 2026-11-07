//! A source window justifies only its own current immutable view and index.
use super::*;
use crate::native_artifact::codegen::options::OptimizationPass;

const SOURCE: &str = include_str!("../../../../../tests/fixtures/native/numeric-array-windows.mec");

fn forward_sources() -> [(String, usize); 2] {
    let forward = SOURCE.replace(
        "reverse := bits/wrapping-sub(sequence/length(nodes), at)",
        "reverse := at",
    );
    assert_ne!(forward, SOURCE);
    let single = forward.replace(
        "node := ((reverse < sequence/length(nodes))? | true => nodes[bits/wrapping-add(reverse, 1u64)] | false => 0u64)",
        "node := 0u64",
    );
    assert_ne!(single, forward);
    [(forward, 2), (single, 1)]
}

#[test]
fn numeric_forward_windows_require_new_current_proofs_and_refuse_changing_bounds() {
    for (source, views) in forward_sources() {
        for artifact in artifacts(&source) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            if views == 1 {
                assert!(
                    metadata["readonly_numeric_regions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|region| {
                            region["proof"]["arrays"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .any(|array| {
                                    array["witness"].as_u64() == Some(usize::MAX as u64)
                                        && array["width"].as_u64() == Some(0)
                                        && array["writable"].as_bool() == Some(false)
                                        && array["appends"].as_bool() == Some(false)
                                })
                        }),
                    "the unused retained owner must remain opaque without content permissions"
                );
            }
            let windows = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|key| metadata[key].as_array().unwrap())
            .flat_map(|region| region["guarded_loop_windows"].as_array().unwrap())
            .collect::<Vec<_>>();
            assert!(
                windows.iter().any(|window| {
                    let distinct = window["views"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| v.as_u64().unwrap())
                        .collect::<std::collections::BTreeSet<_>>();
                    distinct.len() == views
                        && window["source_successors"].as_array().unwrap().len() >= 2
                        && window["read_sites"].as_array().unwrap().len() >= views
                }),
                "forward source needs a real activation-current window: {windows:?}"
            );
        }
        let changing = source
            .replace(
                ":Try(k<u64>, best<u32>, item<u64>)",
                ":Try(k<u64>, best<u32>, item<u64>, remaining<u64>)",
            )
            .replace(":Try(start, best, item)", ":Try(start, best, item, stop)")
            .replace(":Try(k, best, item)", ":Try(k, best, item, remaining)")
            .replace(
                "| k < stop ->",
                "| k >= 8u64 -> :Done(k, best, item)\n    | k < remaining ->",
            )
            .replace(
                ":Try(k + 1u64, next-best, next-item)",
                ":Try(k + 1u64, next-best, next-item, remaining + 2u64)",
            );
        assert_ne!(changing, source);
        for artifact in artifacts(&changing) {
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
                assert!(
                    region["guarded_loop_windows"]
                        .as_array()
                        .unwrap()
                        .is_empty(),
                    "a changing bound cannot inherit an entry window: {region}"
                );
            }
        }
    }
    let opaque = SOURCE
        .replace("value := ((at < sequence/length(costs))? | true => costs[bits/wrapping-add(at, 1u64)] | false => 0u32)", "value := 0u32")
        .replace("node := ((reverse < sequence/length(nodes))? | true => nodes[bits/wrapping-add(reverse, 1u64)] | false => 0u64)", "node := 0u64");
    assert_ne!(opaque, SOURCE);
    for artifact in artifacts(&opaque) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        assert!(
            metadata["compiled_readonly_numeric_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            metadata["compiled_numeric_write_loops"]
                .as_array()
                .unwrap()
                .is_empty()
        );
        assert!(
            !metadata["compiled_scalar_loops"]
                .as_array()
                .unwrap()
                .is_empty(),
            "a purely opaque flat carrier keeps its direct scalar loop"
        );
    }
}

#[test]
fn numeric_forward_windows_keep_zero_trip_errors_aliases_publication_and_recovery() {
    let mode = NativeProgramLimitMode::NoLimits;
    for (source, _) in forward_sources() {
        for artifact in artifacts(&source) {
            let ordinary = JitProgramArtifact::compile_for_test(
                &artifact,
                &mech_stdlib::source_catalog(),
                OptimizationPass::ScalarLoops,
            )
            .unwrap();
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut actual = vec![None; artifact.outputs().len()];
                let mut expected = actual.clone();
                for (size, start, stop, seed, fault) in [
                    (300, 0, 256, u32::MAX, u64::MAX),
                    (300, 0, 3, u32::MAX, 1),
                    (0, 0, 3, 0, u64::MAX),
                    (2, 0, 4, u32::MAX, u64::MAX),
                    (300, 1, 5, 0, u64::MAX),
                    (0, u64::MAX, u64::MAX, 23, u64::MAX),
                    (300, u64::MAX - 1, u64::MAX, 0, 0),
                    (300, 0, 3, u32::MAX, 1),
                    (300, 0, 8, u32::MAX, u64::MAX),
                ] {
                    let args = arguments(&artifact, &memory, size, start, stop, seed, fault);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let prior = actual.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let run = |jit: &JitProgramArtifact, output: &mut [Option<Value>]| {
                        if restored {
                            observe::run_restored(jit, &args, output, mode.execution(), &memory)
                        } else {
                            observe::run(jit, &args, output, mode.execution(), &memory)
                        }
                    };
                    let (reference, _) = run(&ordinary, &mut expected);
                    let (result, _) = run(&native, &mut actual);
                    if fault == 1 {
                        assert_eq!(
                            format!("{:?}", result.unwrap_err()),
                            format!("{:?}", reference.unwrap_err())
                        );
                        read_packs::unchanged_publication(&actual, &prior);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    } else {
                        reference.unwrap();
                        result.unwrap();
                        let values = |output: &[Option<Value>]| {
                            contents(
                                &output
                                    .iter()
                                    .map(|v| v.as_ref().unwrap().clone())
                                    .collect::<Vec<_>>(),
                            )
                        };
                        assert_eq!(values(&actual), values(&expected));
                    }
                    assert_eq!(contents(&args), before);
                    assert_eq!(contents(&aliases), before);
                }
                drop((actual, expected));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    (0, 0)
                );
            }
        }
    }
}

#[test]
fn numeric_forward_windows_keep_every_finite_boundary_and_retry() {
    for (source, _) in forward_sources() {
        resources::finite_boundaries(&source, |artifact, memory| {
            arguments(artifact, memory, 300, 0, 3, u32::MAX, u64::MAX)
        });
    }
}

pub(super) fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    size: usize,
    start: u64,
    stop: u64,
    seed: u32,
    fault: u64,
) -> Vec<Value> {
    inputs(
        artifact,
        &[
            (
                "prices",
                Data::Matrix(
                    (0..size)
                        .map(|i| Data::U32((i as u32).wrapping_mul(17).wrapping_add(11)))
                        .collect(),
                ),
            ),
            (
                "nodes",
                Data::Matrix(
                    (0..size)
                        .map(|i| Data::U64((i as u64).wrapping_mul(0x1_0000_0029)))
                        .collect(),
                ),
            ),
            ("start", Data::U64(start)),
            ("stop", Data::U64(stop)),
            ("seed", Data::U32(seed)),
            ("nonce", Data::U64(119)),
            ("fault", Data::U64(fault)),
        ],
        memory,
    )
}

#[test]
fn numeric_current_windows_emit_current_source_read_proofs() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let guards = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|key| metadata[key].as_array().unwrap())
        .flat_map(|region| {
            region
                .get("proven_scalar_read_guards")
                .and_then(|v| v.as_array())
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
        assert!(
            guards.len() >= 2,
            "both forward/reverse reads require current proofs: {guards:?}"
        );
    }
}

#[test]
fn numeric_current_windows_prove_complete_child_windows_for_both_views() {
    for artifact in artifacts(SOURCE) {
        let object = crate::native_artifact::emit_program_object_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let metadata = object.metadata();
        let windows = [
            "compiled_readonly_numeric_loops",
            "compiled_numeric_write_loops",
        ]
        .into_iter()
        .flat_map(|key| metadata[key].as_array().unwrap())
        .flat_map(|region| {
            region
                .get("guarded_loop_windows")
                .and_then(|value| value.as_array())
                .into_iter()
                .flatten()
        })
        .collect::<Vec<_>>();
        assert!(
            windows.iter().any(|window| {
                let views = window["views"].as_array().unwrap();
                let distinct = views
                    .iter()
                    .map(|v| v.as_u64().unwrap())
                    .collect::<std::collections::BTreeSet<_>>();
                distinct.len() >= 2
                    && window["source_successors"].as_array().unwrap().len() >= 2
                    && window["read_sites"].as_array().unwrap().len() >= 2
            }),
            "one complete child needs current windows for both views: {windows:?}"
        );
    }
}

#[test]
fn numeric_current_windows_refuse_a_bound_changed_on_each_visit() {
    let source = SOURCE
        .replace(
            ":Try(k<u64>, best<u32>, item<u64>)",
            ":Try(k<u64>, best<u32>, item<u64>, remaining<u64>)",
        )
        .replace(":Try(start, best, item)", ":Try(start, best, item, stop)")
        .replace(":Try(k, best, item)", ":Try(k, best, item, remaining)")
        .replace(
            "| k < stop ->",
            "| k >= 8u64 -> :Done(k, best, item)\n    | k < remaining ->",
        )
        .replace(
            ":Try(k + 1u64, next-best, next-item)",
            ":Try(k + 1u64, next-best, next-item, remaining + 2u64)",
        );
    for artifact in artifacts(&source) {
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
            assert!(
                region
                    .get("guarded_loop_windows")
                    .and_then(|v| v.as_array())
                    .is_none_or(Vec::is_empty),
                "a changing bound must not inherit an entry gate: {region}"
            );
        }
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        for jit in [&ordinary, &native] {
            let memory = NativeProgramLimitMode::NoLimits.memory_budget();
            let args = arguments(&artifact, &memory, 6, 0, 2, u32::MAX, u64::MAX);
            let aliases = args.clone();
            let before = contents(&args);
            let mut output = vec![None; artifact.outputs().len()];
            observe::run_restored(
                jit,
                &args,
                &mut output,
                NativeProgramLimitMode::NoLimits.execution(),
                &memory,
            )
            .0
            .unwrap();
            assert_eq!(
                output
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                Data::Tuple(vec![Data::U32(0), Data::U64(13), Data::U64(8)].into())
            );
            assert_eq!(contents(&aliases), before);
            drop((output, args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn numeric_current_windows_preserve_default_zero_errors_and_fresh_recovery() {
    let mode = NativeProgramLimitMode::NoLimits;
    let mut expected_error = None;
    for artifact in artifacts(SOURCE) {
        let ordinary = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::ScalarLoops,
        )
        .unwrap();
        let native = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let without_cleanup = JitProgramArtifact::compile_for_test(
            &artifact,
            &mech_stdlib::source_catalog(),
            OptimizationPass::CfgCleanup,
        )
        .unwrap();
        for (reference, jit) in [
            (true, &ordinary),
            (false, &without_cleanup),
            (false, &native),
        ] {
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut output = vec![None; artifact.outputs().len()];
                for (size, start, stop, seed, fault) in [
                    (300, 0, 256, u32::MAX, u64::MAX),
                    (300, 0, 3, u32::MAX, 1),
                    (0, 0, 3, 0, u64::MAX),
                    (2, 0, 4, u32::MAX, u64::MAX),
                    (300, 1, 5, 0, u64::MAX),
                    (0, u64::MAX, u64::MAX, 23, u64::MAX),
                    (300, u64::MAX - 1, u64::MAX, 0, 0),
                    (300, 0, 3, u32::MAX, 1),
                    (300, 0, 8, u32::MAX, u64::MAX),
                ] {
                    let args = arguments(&artifact, &memory, size, start, stop, seed, fault);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = output.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, counts) = if restored {
                        observe::run_restored(jit, &args, &mut output, mode.execution(), &memory)
                    } else {
                        observe::run(jit, &args, &mut output, mode.execution(), &memory)
                    };
                    if fault == 1 {
                        let error = format!("{:?}", result.unwrap_err());
                        assert!(error.contains("Arithmetic"), "{error}");
                        assert_eq!(error, *expected_error.get_or_insert(error.clone()));
                        super::read_packs::unchanged_publication(&output, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                    } else {
                        result.unwrap();
                        let (mut best, mut item) = (seed, 119);
                        for k in start..stop {
                            let at = k.wrapping_add(3);
                            let value = if at < size as u64 {
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
                            let cost = u64::from(value).wrapping_add(node) as u32;
                            if cost < best {
                                best = cost;
                                item = k.checked_add(10).unwrap();
                            }
                        }
                        assert_eq!(
                            output
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
                        if !reference {
                            assert!(!counts.tail_entries.is_empty());
                        }
                    }
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
}

#[test]
fn numeric_current_windows_keep_finite_prefixes_aliases_and_retry() {
    resources::finite_boundaries(SOURCE, |artifact, memory| {
        arguments(artifact, memory, 300, 0, 3, u32::MAX, u64::MAX)
    });
}
