//! Executed pinned writer cases; component measurements are not full compression.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft};
use mech_gpu::native_artifact::JitProgramArtifact;
#[test]
fn pinned_sequence_writer_matches_reference_with_real_output_reservation() {
    let source = format!(
        "{}\n{}\n{}\nblock-write-sequences(sequence/reserve(bytes<[u8]>, reserve<u64>), bitbuf<u64>, bitcount<u64>, input<[u8]>, 0u64, seqs<[block-sequence]>, packed<[u64]>)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        include_str!("../fixtures/native/deflate-matchfinder.mec"),
        include_str!("../fixtures/native/deflate-sequence-writer.mec")
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-sequence-writer-reference.json"
    ))
    .unwrap();
    let matrix = |v: &serde_json::Value, width| {
        ValueDataDraft::Matrix(
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| {
                    if width == 8 {
                        ValueDataDraft::U8(n.as_u64().unwrap() as u8)
                    } else {
                        ValueDataDraft::U64(n.as_u64().unwrap())
                    }
                })
                .collect(),
        )
    };
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mech_gpu::native_artifact::NativeCompileLimits {
                max_nodes: 16384,
                ..Default::default()
            },
        )
        .unwrap();
        println!(
            "writer blocks={} form={form}",
            jit.descriptor().plan().blocks().len()
        );
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
            let seqs = ValueDataDraft::Matrix(
                case["seqs"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|r| {
                        ValueDataDraft::Record(
                            [
                                (
                                    "litrunlen-and-length",
                                    ValueDataDraft::U32(
                                        r["litrunlen_and_length"].as_u64().unwrap() as u32,
                                    ),
                                ),
                                (
                                    "offset",
                                    ValueDataDraft::U16(r["offset"].as_u64().unwrap() as u16),
                                ),
                                (
                                    "offset-slot",
                                    ValueDataDraft::U16(r["offset_slot"].as_u64().unwrap() as u16),
                                ),
                            ]
                            .into_iter()
                            .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                                name: name.into(),
                                value,
                            })
                            .collect(),
                        )
                    })
                    .collect(),
            );
            let args = inputs(
                &artifact,
                &[
                    ("bytes", matrix(&case["out"], 8)),
                    (
                        "reserve",
                        ValueDataDraft::U64(6633832 - case["out"].as_array().unwrap().len() as u64),
                    ),
                    (
                        "bitbuf",
                        ValueDataDraft::U64(case["bitbuf"].as_u64().unwrap()),
                    ),
                    (
                        "bitcount",
                        ValueDataDraft::U64(case["bitcount"].as_u64().unwrap()),
                    ),
                    ("input", matrix(&case["input"], 8)),
                    ("seqs", seqs),
                    ("packed", matrix(&fixture["packed"], 64)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.execution.max_transport_bytes = 2 * 1024 * 1024;
            policy.execution.max_work = 1_000_000_000_000;
            policy.library.max_compute_work = Some(1_000_000_000_000);
            policy.library.max_comparison_work = Some(1_000_000_000_000);
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"case":case["name"],"form":form,"seconds":start.elapsed().as_secs_f64(),"allocations":requests,"allocated_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            // Publishing a whole record canonicalizes its nested byte storage.
            // Private return/clear/append capacity is checked in the separate
            // internal-reuse test below, before that publication boundary.
            let ValueDataDraft::Record(fields) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            let get = |name: &str| {
                &fields
                    .iter()
                    .find(|f| f.name.as_str() == name)
                    .unwrap()
                    .value
            };
            assert_eq!(get("bytes"), &matrix(&case["expected"]["out"], 8));
            assert_eq!(
                get("bitbuf"),
                &ValueDataDraft::U64(case["expected"]["bitbuf"].as_u64().unwrap())
            );
            assert_eq!(
                get("bitcount"),
                &ValueDataDraft::U64(case["expected"]["bitcount"].as_u64().unwrap())
            );
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}

#[test]
fn pinned_sequence_writer_retains_internal_capacity_without_payload_copies() {
    let source = format!(
        "{}\n{}\n{}\nblock-write-sequences(sequence/reserve(bytes<[u8]>, reserve<u64>), bitbuf<u64>, bitcount<u64>, input<[u8]>, 0u64, seqs<[block-sequence]>, packed<[u64]>)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        include_str!("../fixtures/native/deflate-matchfinder.mec"),
        include_str!("../fixtures/native/deflate-sequence-writer.mec")
    );
    let (prefix, call) = source.rsplit_once("\nblock-write-sequences(").unwrap();
    let source = format!(
        "{prefix}\nwritten := block-write-sequences({}\nsequence/append(sequence/clear(written.bytes), 17u8)\n",
        call.trim()
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-sequence-writer-reference.json"
    ))
    .unwrap();
    let matrix = |v: &serde_json::Value, width| {
        ValueDataDraft::Matrix(
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| {
                    if width == 8 {
                        ValueDataDraft::U8(n.as_u64().unwrap() as u8)
                    } else {
                        ValueDataDraft::U64(n.as_u64().unwrap())
                    }
                })
                .collect(),
        )
    };
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mech_gpu::native_artifact::NativeCompileLimits {
                max_nodes: 16384,
                ..Default::default()
            },
        )
        .unwrap();
        println!(
            "writer blocks={} form={form}",
            jit.descriptor().plan().blocks().len()
        );
        for case in fixture["cases"].as_array().unwrap() {
            let mut allocated = Vec::new();
            for capacity in [512, 6633832] {
                let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
                let seqs = ValueDataDraft::Matrix(
                    case["seqs"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|r| {
                            ValueDataDraft::Record(
                                [
                                    (
                                        "litrunlen-and-length",
                                        ValueDataDraft::U32(
                                            r["litrunlen_and_length"].as_u64().unwrap() as u32,
                                        ),
                                    ),
                                    (
                                        "offset",
                                        ValueDataDraft::U16(r["offset"].as_u64().unwrap() as u16),
                                    ),
                                    (
                                        "offset-slot",
                                        ValueDataDraft::U16(
                                            r["offset_slot"].as_u64().unwrap() as u16
                                        ),
                                    ),
                                ]
                                .into_iter()
                                .map(|(name, value)| mech_core::snapshot::NamedValueDraft {
                                    name: name.into(),
                                    value,
                                })
                                .collect(),
                            )
                        })
                        .collect(),
                );
                let args = inputs(
                    &artifact,
                    &[
                        ("bytes", matrix(&case["out"], 8)),
                        (
                            "reserve",
                            ValueDataDraft::U64(
                                capacity - case["out"].as_array().unwrap().len() as u64,
                            ),
                        ),
                        (
                            "bitbuf",
                            ValueDataDraft::U64(case["bitbuf"].as_u64().unwrap()),
                        ),
                        (
                            "bitcount",
                            ValueDataDraft::U64(case["bitcount"].as_u64().unwrap()),
                        ),
                        ("input", matrix(&case["input"], 8)),
                        ("seqs", seqs),
                        ("packed", matrix(&fixture["packed"], 64)),
                    ],
                    &memory,
                );
                let mut output = vec![None; artifact.outputs().len()];
                let mut policy = limits();
                policy.execution.max_transport_bytes = 2 * 1024 * 1024;
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                policy.library.max_comparison_work = Some(1_000_000_000_000);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                let start = std::time::Instant::now();
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                });
                println!(
                    "{}",
                    serde_json::json!({"case":case["name"],"form":form,"seconds":start.elapsed().as_secs_f64(),"allocations":requests,"allocated_bytes":bytes,"result":format!("{result:?}")})
                );
                result.unwrap();
                let value = output.last().unwrap().as_ref().unwrap();
                let mech_core::ValueData::Matrix(matrix) = value.data() else {
                    panic!()
                };
                assert_eq!(matrix.unsigned_buffer_capacity(), Some(capacity as usize));
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    ValueDataDraft::Matrix(vec![ValueDataDraft::U8(17)].into())
                );
                allocated.push(bytes);
                drop((args, output));
                assert_eq!(memory.used_bytes(), 0);
            }
            assert_eq!(allocated[1] - allocated[0], 6633832 - 512);
        }
    }
}
