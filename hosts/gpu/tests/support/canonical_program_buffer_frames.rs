//! Ordinary recursive frames retain immutable buffers rather than copying them.
use super::{
    JitProgramArtifact, ManagedMemoryBudget, ValueDataDraft as Data, allocation_probe, artifacts,
    inputs, limits, reference_result,
};
use mech_core::ValueData;
use mech_engine::native_artifact::lower_native_program_artifact;

const GROW: &str = "+> sequence\ngrow(words<[u64]>, left<u64>) => <[u64]>\n  | (words, left) => ((left > 0u64)?\n      | true => grow(sequence/append(words, left), left - 1u64)\n      | false => words).\ngrow(sequence/with-capacity(0u64, capacity<u64>), steps<u64>)\n";

#[test]
fn recursive_buffers_retain_real_reservations_without_payload_copies() {
    for (width, real) in [(64, 305257), (8, 6633832)] {
        let source = if width == 64 {
            GROW.to_owned()
        } else {
            GROW.replace("[u64]", "[u8]")
                .replace("append(words, left)", "append(words, 129u8)")
                .replace("0u64, capacity", "0u8, capacity")
        };
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert!(plan.facts().buffer_frames().elements() > 0);
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            let mut measurements = Vec::new();
            for capacity in [16, real] {
                for steps in [0, 1, 3] {
                    let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
                    let args = inputs(
                        &artifact,
                        &[
                            ("capacity", Data::U64(capacity)),
                            ("steps", Data::U64(steps)),
                        ],
                        &memory,
                    );
                    let mut output = vec![None; artifact.outputs().len()];
                    let mut policy = limits();
                    // Same finite diagnostic policy as the saved red acceptance.
                    policy.execution.max_work = 1_000_000_000_000;
                    policy.library.max_compute_work = Some(1_000_000_000_000);
                    // Warm immutable metadata and the allocator probe equally.
                    allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, policy, &memory)
                    })
                    .0
                    .unwrap();
                    let start = std::time::Instant::now();
                    let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, policy, &memory)
                    });
                    result.unwrap();
                    let value = output.last().unwrap().as_ref().unwrap();
                    let expected = Data::Matrix(
                        (1..=steps)
                            .rev()
                            .map(|n| {
                                if width == 64 {
                                    Data::U64(n)
                                } else {
                                    Data::U8(129)
                                }
                            })
                            .collect(),
                    );
                    assert_eq!(value.canonical_data_draft().unwrap(), expected);
                    let ValueData::Matrix(matrix) = value.data() else {
                        panic!()
                    };
                    assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                    println!(
                        "{}",
                        serde_json::json!({"witness":"recursive-reserved-buffer","artifact":form,"width":width,"capacity":capacity,"steps":steps,"requests":requests,"requested_bytes":bytes,"elapsed_seconds":start.elapsed().as_secs_f64()})
                    );
                    measurements.push((capacity, steps, bytes));
                    drop((args, output));
                    assert_eq!(memory.used_bytes(), 0);
                    assert_eq!(memory.retained_snapshot_nodes(), 0);
                }
            }
            for steps in [0, 1, 3] {
                let bytes = |capacity| {
                    measurements
                        .iter()
                        .find(|(c, n, _)| *c == capacity && *n == steps)
                        .unwrap()
                        .2
                };
                // The extra reservation is allocated exactly once, not once per
                // tuple pack, projection, recursive visit or return transport.
                assert_eq!(
                    bytes(real) - bytes(16),
                    (real - 16) as usize * (width / 8) as usize
                );
            }
        }
    }
}

#[test]
fn checked_frames_use_fresh_booleans_preserve_aliases_and_recover() {
    let source = "+> sequence\nwalk(words<[u64]>, left<u64>, allowed<bool>, next<bool>, divisor<u64>) => (out<u64>) :=\n  enabled! := allowed\n  out := ((left > 0u64)? | true => walk(words, left - 1u64, next, next, divisor) | false => sequence/length(words) + 12u64 / divisor).\nwalk(values<[u64]>, 3u64, true, allowed<bool>, divisor<u64>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (allowed, divisor, expected) in [
            (true, 3, Some(7)),
            (false, 0, None),
            (true, 0, None),
            (true, 2, Some(9)),
        ] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "values",
                        Data::Matrix(vec![Data::U64(41), Data::U64(43), Data::U64(47)].into()),
                    ),
                    ("allowed", Data::Bool(allowed)),
                    ("divisor", Data::U64(divisor)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let prior = output.clone();
            let before = memory.used_bytes();
            let result = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            if let Some(expected) = expected {
                result.unwrap();
                resident.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(expected)
                );
            } else {
                let error = format!("{:?}", result.unwrap_err());
                let resident = resident.unwrap_err();
                if !allowed {
                    assert!(error.contains("Integrity"), "{error}");
                    assert!(resident.contains("LexicalIntegrity"), "{resident}");
                } else {
                    assert!(!error.contains("Integrity"), "{error}");
                    assert!(!resident.contains("LexicalIntegrity"), "{resident}");
                }
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(v, a)| v.shares_frozen_storage(a))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn escaping_tuples_keep_canonical_storage_and_scalar_aggregates_stay_separate() {
    for source in [
        "(values<[u16]>, n<u64>)\n",
        "count(n<u64>) => <u64> | n => ((n > 0u64)? | true => count(n - 1u64) | false => n).\ncount(input<u64>)\n",
    ] {
        for artifact in artifacts(source).0 {
            let plan =
                lower_native_program_artifact(&artifact, &mech_stdlib::source_catalog()).unwrap();
            assert_eq!(plan.facts().buffer_frames().elements(), 0);
        }
    }
}

#[test]
fn real_recursive_arguments_keep_allocation_below_either_payload() {
    let source = "+> sequence\nwalk(table<[u16]>, cache<[u32]>, n<u64>, allowed<bool>) => (out<u64>) :=\n  enabled! := allowed\n  out := ((n == 0u64)? | true => sequence/length(table) + sequence/length(cache) | false => walk(table, cache, n - 1u64, allowed)).\nwalk(table<[u16]>, cache<[u32]>, steps<u64>, allowed<bool>)\n";
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut policy = limits();
        policy.execution.max_work = 10_000_000_000;
        policy.library.max_compute_work = Some(10_000_000_000);
        policy.library.max_comparison_work = Some(10_000_000_000);
        // Argument transport creates no cloned application payload.
        policy.library.max_cloned_bytes = Some(0);
        for allowed in [true, false, true] {
            let args = inputs(
                &artifact,
                &[
                    (
                        "table",
                        Data::Matrix(vec![Data::U16(0xf123); 262144].into()),
                    ),
                    (
                        "cache",
                        Data::Matrix(vec![Data::U32(0xf0123456); 1500513].into()),
                    ),
                    ("steps", Data::U64(1)),
                    ("allowed", Data::Bool(allowed)),
                ],
                &memory,
            );
            let prior = output.clone();
            let before = memory.used_bytes();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            assert!(
                bytes < 262144 * 2,
                "{requests} allocations / {bytes} bytes: {result:?}"
            );
            if allowed {
                result.unwrap();
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(262144 + 1500513)
                );
            } else {
                assert!(format!("{:?}", result.unwrap_err()).contains("Integrity"));
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            }
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn nested_repeated_calls_keep_published_buffers_immutable_and_recover_from_denial() {
    let grow = GROW.split("grow(sequence/with-capacity").next().unwrap();
    let source = format!(
        "{grow}original := sequence/with-capacity(0u64, 32u64)\nfirst := grow(original, 2u64)\nsecond := grow(first, 1u64)\ncleared := sequence/clear(second)\nthird := grow(cleared, 3u64)\nsequence/length(original) + sequence/length(first) + sequence/length(second) + sequence/length(third)\n"
    );
    let (forms, last) = super::artifacts_mode(&source, true);
    for artifact in forms {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
        let args = inputs(&artifact, &[], &memory);
        let mut output = vec![None; artifact.outputs().len()];
        for denied in [false, true, false] {
            let before = memory.used_bytes();
            let prior = output.clone();
            let mut policy = limits();
            if denied {
                policy.execution.max_transport_bytes = 1;
            }
            let result = jit.evaluate_into(&args, &mut output, policy, &memory);
            if denied {
                assert!(result.is_err());
                assert_eq!(memory.used_bytes(), before);
                assert!(output.iter().zip(&prior).all(|(v, p)| {
                    v.as_ref()
                        .unwrap()
                        .shares_frozen_storage(p.as_ref().unwrap())
                }));
            } else {
                result.unwrap();
                let resident = reference_result(&artifact, &args).unwrap();
                super::equal(&artifact, &output, &resident);
                let matrices = output
                    .iter()
                    .filter_map(|v| match v.as_ref().unwrap().data() {
                        ValueData::Matrix(matrix) => Some(matrix),
                        _ => None,
                    })
                    .collect::<Vec<_>>();
                assert_eq!(matrices.len(), 5);
                assert!(
                    matrices
                        .iter()
                        .all(|m| m.unsigned_buffer_capacity() == Some(32))
                );
                assert_eq!(
                    output
                        .get(last)
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::U64(8)
                );
            }
        }
        drop((args, output));
        assert_eq!(memory.used_bytes(), 0);
    }
}

#[test]
fn build_owned_image_rederives_private_frame_metadata() {
    for artifact in artifacts(GROW).0 {
        let catalog = mech_stdlib::source_catalog();
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let bytes = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: this checked build produced these bytes. The restored
        // metadata remains confined to comparison and cannot execute code.
        let restored = unsafe {
            mech_gpu::native_artifact::NativeProgramDescriptor::decode_build_owned_image(
                &bytes, &catalog,
            )
        }
        .unwrap();
        assert_eq!(restored.encode_build_owned_image().unwrap(), bytes);
    }
}
