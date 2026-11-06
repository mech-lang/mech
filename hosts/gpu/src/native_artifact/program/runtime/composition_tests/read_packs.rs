//! Confined byte construction keeps source errors, current views and recovery.
use super::*;

pub(super) fn source(width: usize, first: u64) -> String {
    let mut body = String::from("+> bits\npack(xs<[u8]>, at<u64>) => (out<u64>) :=\n");
    let mut words = Vec::new();
    for byte in 0..width {
        let offset = first + byte as u64;
        let at = if offset == 0 {
            "at".to_owned()
        } else if offset == 1 {
            "at + 1u64".to_owned()
        } else {
            format!("(at + {}u64) + 1u64", offset - 1)
        };
        body.push_str(&format!("  b{byte}<u64> := xs[{at}]\n"));
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

pub(super) fn arguments(
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
            let shape_values: Box<[u64]> = if matches!(data, Data::Matrix(_)) {
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

pub(super) fn unchanged_publication(published: &[Option<Value>], previous: &[Option<Value>]) {
    for (current, old) in published.iter().zip(previous) {
        match (current, old) {
            (Some(current), Some(old)) => assert!(current.shares_frozen_storage(old)),
            (None, None) => {}
            _ => panic!("failed call changed publication"),
        }
    }
}

#[test]
fn byte_pack_emission_requires_a_complete_confined_expression() {
    for width in [2, 4, 8] {
        for artifact in [0, 1, 2]
            .into_iter()
            .flat_map(|first| artifacts(&source(width, first)))
        {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let packs = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|name| metadata[name].as_array().unwrap())
            .flat_map(|region| region["read_pack_sites"].as_array().unwrap());
            let widths = packs
                .map(|pack| pack[2].as_u64().unwrap())
                .collect::<Vec<_>>();
            assert_eq!(widths, [width as u64], "{metadata}");
        }
    }
    let ordinary = source(8, 1);
    let packed = "bits/or(bits/or(bits/or(b0, bits/shift-left-wrap(b1, 8u64)), bits/shift-left-wrap(bits/or(b2, bits/shift-left-wrap(b3, 8u64)), 16u64)), bits/shift-left-wrap(bits/or(bits/or(b4, bits/shift-left-wrap(b5, 8u64)), bits/shift-left-wrap(bits/or(b6, bits/shift-left-wrap(b7, 8u64)), 16u64)), 32u64))";
    assert!(ordinary.contains(packed));
    let escape = ordinary.replace(
        &format!("out := {packed}."),
        &format!("packed := {packed}\n  out := bits/xor(packed, b0)."),
    );
    let permuted = ordinary
        .replace("(at + 6u64)", "(at + 99u64)")
        .replace("(at + 7u64)", "(at + 6u64)")
        .replace("(at + 99u64)", "(at + 7u64)");
    let mut mutation = ordinary.replace(
        "  b4<u64>",
        "  ~changed := xs\n  changed[at + 5u64] = 99u8\n  b4<u64>",
    );
    for byte in 4..8 {
        mutation = mutation.replace(
            &format!("b{byte}<u64> := xs["),
            &format!("b{byte}<u64> := changed["),
        );
    }
    for (case, changed) in [
        ("external-byte-consumer", escape),
        ("permuted-bytes", permuted),
        ("interior-mutation", mutation),
        (
            "noncontiguous-shifts",
            ordinary.replace("32u64))", "31u64))"),
        ),
        ("wrapping-shift", ordinary.replace("32u64))", "64u64))")),
        (
            "overlapping-byte",
            ordinary.replace("xs[(at + 1u64) + 1u64]", "xs[at + 1u64]"),
        ),
        (
            "unrelated-checked-add",
            ordinary.replace("  b4<u64>", "  unused := at + 900u64\n  b4<u64>"),
        ),
        (
            "interior-assertion",
            ordinary.replace("  b4<u64>", "  inside! := at < 3u64\n  b4<u64>"),
        ),
        (
            "interior-branch",
            ordinary.replace(
                "xs[(at + 4u64) + 1u64]",
                "((at < 3u64)? | true => xs[(at + 4u64) + 1u64] | false => 0u8)",
            ),
        ),
    ] {
        assert_ne!(changed, ordinary, "{case} must change the witness");
        for artifact in artifacts(&changed) {
            let object = crate::native_artifact::emit_program_object_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeProgramLimitMode::NoLimits.compilation(),
            )
            .unwrap();
            let metadata = object.metadata();
            let full = [
                "compiled_readonly_numeric_loops",
                "compiled_numeric_write_loops",
            ]
            .into_iter()
            .flat_map(|name| metadata[name].as_array().unwrap())
            .flat_map(|region| region["read_pack_sites"].as_array().unwrap())
            .any(|pack| pack[2].as_u64() == Some(8));
            assert!(
                !full,
                "{case} must retain the full source expression: {metadata}"
            );
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
    for first in [0, 1, 2] {
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
                let mut successful_work = None;
                let good_at = u64::from(first == 0);
                let mut cases = vec![
                    (good_at, "none"),
                    (u64::MAX - 1, "none"),
                    (u64::MAX, "none"),
                    (good_at, "setup-work"),
                    (good_at, "prefix-work"),
                    (good_at, "late-work"),
                    (good_at, "compute"),
                    (good_at, "temporary"),
                ];
                if first == 0 {
                    cases.push((0, "zero-index"));
                }
                cases.push((good_at, "none"));
                for (case, (at, denial)) in cases.into_iter().enumerate() {
                    let args = arguments(&artifact, &memory, [1, 32], at, 3, 41);
                    let aliases = args.clone();
                    let before = contents(&args);
                    let previous = published.clone();
                    let held = (memory.used_bytes(), memory.retained_snapshot_nodes());
                    let mut limits = mode.execution();
                    match denial {
                        "setup-work" => limits.execution.max_work = 0,
                        "prefix-work" => limits.execution.max_work = 1_000,
                        "late-work" => limits.execution.max_work = successful_work.unwrap() - 1,
                        "compute" => limits.library.max_compute_work = Some(0),
                        "temporary" => limits.library.max_temporary_bytes = Some(0),
                        "none" | "zero-index" => {}
                        _ => unreachable!(),
                    }
                    let (result, _) = if restored {
                        observe::run_restored(&native, &args, &mut published, limits, &memory)
                    } else {
                        observe::run(&native, &args, &mut published, limits, &memory)
                    };
                    if at == good_at && denial == "none" {
                        let report = result.unwrap();
                        successful_work = Some(report.work);
                        assert_eq!(
                            published
                                .last()
                                .unwrap()
                                .as_ref()
                                .unwrap()
                                .canonical_data_draft()
                                .unwrap(),
                            Data::U64(expected(8, [1, 32], good_at + first - 1, 3, 41))
                        );
                        println!(
                            "read-pack-refusal first={first} restored={restored} case={case} work={}",
                            report.work
                        );
                    } else {
                        let error = format!("{:?}", result.unwrap_err());
                        if denial == "none" {
                            assert_eq!(
                                error.contains("Arithmetic"),
                                at.checked_add(first).is_none(),
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
