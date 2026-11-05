//! Confined byte construction keeps source errors, current views and recovery.
use super::*;

fn source(width: usize, first: u64) -> String {
    let mut body = String::from("+> bits\npack(xs<[u8]>, at<u64>) => (out<u64>) :=\n");
    let mut words = Vec::new();
    for byte in 0..width {
        let offset = first - 1 + byte as u64;
        let at = if offset == 0 {
            "at".to_owned()
        } else {
            format!("(at + {offset}u64)")
        };
        body.push_str(&format!("  b{byte}<u64> := xs[{at} + 1u64]\n"));
        words.push(format!("b{byte}"));
    }
    let mut shift = 8;
    while words.len() > 1 {
        words = words
            .chunks_exact(2)
            .map(|pair| {
                format!(
                    "bits/or({}, bits/shift-left-wrap({}, {shift}u64))",
                    pair[0], pair[1]
                )
            })
            .collect();
        shift *= 2;
    }
    body.push_str(&format!("  out := {}.\n", words[0]));
    body.push_str("walk(xs<[u8]>, at<u64>, left<u64>, total<u64>) => (out<u64>) :=\n  more := left > 0u64\n  word := (more? | true => pack(xs, at) | false => 0u64)\n  out := (more? | true => walk(xs, at + 1u64, left - 1u64, bits/xor(total, word)) | false => total).\nwalk(bytes<[u8]>, at<u64>, steps<u64>, seed<u64>)\n");
    body
}

fn arguments(
    artifact: &ProgramArtifact,
    memory: &ManagedMemoryBudget,
    dimensions: [u64; 2],
    at: u64,
    steps: u64,
    nonce: u8,
) -> Vec<Value> {
    artifact
        .inputs()
        .iter()
        .map(|port| {
            let name = mech_engine::decode_source_input_name(&port.name).unwrap();
            let data = match name.as_str() {
                "bytes" => Data::Matrix(
                    (0..dimensions[0] * dimensions[1])
                        .map(|i| Data::U8((i as u8).wrapping_mul(37).wrapping_add(nonce)))
                        .collect(),
                ),
                "at" => Data::U64(at),
                "steps" => Data::U64(steps),
                "seed" => Data::U64(u64::from(nonce)),
                _ => panic!("unexpected input {name}"),
            };
            let shape_values = if matches!(data, Data::Matrix(_)) {
                mech_core::shape_for_resolved_extents(
                    artifact.schemas().get(port.schema).unwrap(),
                    &dimensions,
                )
                .unwrap()
                .parameter_values()
                .into()
            } else {
                Box::new([])
            };
            let value = ValueDraft {
                schema: port.schema,
                shape_values,
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let mut reservation = memory
                .reserve_capacity(value.memory_budget_admission_bytes(memory).unwrap())
                .unwrap();
            value.into_memory_budget(&mut reservation).unwrap()
        })
        .collect()
}

// Enumerate source column order independently over the row-major cells.
fn expected(width: usize, dimensions: [u64; 2], at: u64, steps: u64, nonce: u8) -> u64 {
    let [rows, columns] = dimensions;
    let mut result = u64::from(nonce);
    for visit in 0..steps {
        let mut word = 0;
        for byte in 0..width {
            let ordinal = at + visit + byte as u64;
            let physical = (ordinal % rows) * columns + ordinal / rows;
            let value = (physical as u8).wrapping_mul(37).wrapping_add(nonce);
            word |= u64::from(value) << (byte * 8);
        }
        result ^= word;
    }
    result
}

fn unchanged_publication(published: &[Option<Value>], previous: &[Option<Value>]) {
    for (current, old) in published.iter().zip(previous) {
        match (current, old) {
            (Some(current), Some(old)) => assert!(current.shares_frozen_storage(old)),
            (None, None) => {}
            _ => panic!("failed call changed publication"),
        }
    }
}

#[test]
fn byte_packs_keep_current_geometry_windows_and_recovery() {
    for width in [2, 4, 8] {
        for artifact in artifacts(&source(width, 1)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            assert!(
                native
                    .descriptor()
                    .metadata()
                    .closed_tails
                    .regions
                    .iter()
                    .any(|r| r.numeric.is_some())
            );
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (case, (dimensions, at, steps, succeeds)) in [
                    ([1, 32], 0, 4, true),
                    ([32, 1], 7, 3, true),
                    ([4, 8], 3, 4, true),
                    ([8, 4], 5, 3, true),
                    ([1, 32], 32 - width as u64, 1, true),
                    ([1, width as u64 - 1], 0, 1, false),
                    ([1, 1], 0, 1, false),
                    ([0, 1], 0, 1, false),
                    ([1, 0], 0, 1, false),
                    ([0, 6], u64::MAX, 0, true),
                    ([6, 0], u64::MAX, 0, true),
                    ([1, 32], u64::MAX, 1, false),
                    ([1, 32], u64::MAX - 1, 1, false),
                    ([1, 32], 32, 1, false),
                    ([2, 16], 2, 3, true),
                    ([32, 1], 1, 4, true),
                ]
                .into_iter()
                .enumerate()
                {
                    let nonce = 17 + case as u8;
                    let args = arguments(&artifact, &memory, dimensions, at, steps, nonce);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let (result, _) = if restored {
                        observe::run_restored(
                            &native,
                            &args,
                            &mut published,
                            mode.execution(),
                            &memory,
                        )
                    } else {
                        observe::run(&native, &args, &mut published, mode.execution(), &memory)
                    };
                    if succeeds {
                        let report = result.unwrap();
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected(width, dimensions, at, steps, nonce))
                        );
                        println!(
                            "read-pack-case width={width} restored={restored} case={case} work={}",
                            report.work
                        );
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        assert_eq!(error.contains("Arithmetic"), at == u64::MAX, "{error}");
                        unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                        println!(
                            "read-pack-case width={width} restored={restored} case={case} error={error}"
                        );
                    }
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
fn byte_packs_keep_overflow_before_load_and_finite_failure_retry() {
    for first in [1, 2] {
        for artifact in artifacts(&source(8, first)) {
            let mode = NativeProgramLimitMode::NoLimits;
            let native = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                mode.compilation(),
            )
            .unwrap();
            for restored in [false, true] {
                let memory = mode.memory_budget();
                let mut published = vec![None; artifact.outputs().len()];
                for (case, (at, work)) in [
                    (0, None),
                    (u64::MAX - 1, None),
                    (u64::MAX, None),
                    (0, Some(0)),
                    (0, Some(1_000)),
                    (0, None),
                ]
                .into_iter()
                .enumerate()
                {
                    let args = arguments(&artifact, &memory, [1, 32], at, 3, 41);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut limits = mode.execution();
                    if let Some(work) = work {
                        limits.execution.max_work = work;
                    }
                    let (result, _) = if restored {
                        observe::run_restored(&native, &args, &mut published, limits, &memory)
                    } else {
                        observe::run(&native, &args, &mut published, limits, &memory)
                    };
                    if at == 0 && work.is_none() {
                        let report = result.unwrap();
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected(8, [1, 32], first - 1, 3, 41))
                        );
                        println!(
                            "read-pack-refusal first={first} restored={restored} case={case} work={}",
                            report.work
                        );
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        if work.is_none() {
                            assert_eq!(
                                error.contains("Arithmetic"),
                                first == 2 || at == u64::MAX,
                                "{error}"
                            );
                        }
                        unchanged_publication(&published, &previous);
                        assert_eq!(
                            (memory.used_bytes(), memory.retained_snapshot_nodes()),
                            held
                        );
                        println!(
                            "read-pack-refusal first={first} restored={restored} case={case} error={error}"
                        );
                    }
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
