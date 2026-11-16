//! Executed pinned writer cases; component measurements are not full compression.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft};
use mech_gpu::native_artifact::JitProgramArtifact;
#[test]
fn pinned_path_writer_matches_reference_with_real_output_reservation() {
    let source = format!(
        "{}\n{}\n{}\npath-write-items(sequence/reserve(bytes<[u8]>, reserve<u64>), bitbuf<u64>, bitcount<u64>, items<[u64]>, block-length<u64>, litlen-words<[u32]>, litlen-lens<[u8]>, full-words<[u32]>, full-lens<[u8]>, offset-words<[u32]>, offset-lens<[u8]>, offset-slots<[u8]>, offset-bases<[u32]>, offset-extra<[u8]>)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        include_str!("../fixtures/native/deflate-optimal-path.mec"),
        include_str!("../fixtures/native/deflate-path-writer.mec")
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-path-writer-reference.json"
    ))
    .unwrap();
    let matrix = |v: &serde_json::Value, width| {
        ValueDataDraft::Matrix(
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| match width {
                    8 => ValueDataDraft::U8(n.as_u64().unwrap() as u8),
                    32 => ValueDataDraft::U32(n.as_u64().unwrap() as u32),
                    _ => ValueDataDraft::U64(n.as_u64().unwrap()),
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
            "path-writer blocks={} form={form}",
            jit.descriptor().plan().blocks().len()
        );
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
            let mut data = vec![
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
                ("items", matrix(&case["items"], 64)),
                (
                    "block-length",
                    ValueDataDraft::U64(case["block_length"].as_u64().unwrap()),
                ),
            ];
            for (name, width) in [
                ("litlen-words", 32),
                ("litlen-lens", 8),
                ("full-words", 32),
                ("full-lens", 8),
                ("offset-words", 32),
                ("offset-lens", 8),
                ("offset-slots", 8),
                ("offset-bases", 32),
                ("offset-extra", 8),
            ] {
                data.push((name, matrix(&fixture["tables"][name], width)));
            }
            let args = inputs(&artifact, &data, &memory);
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
fn pinned_path_writer_retains_internal_capacity_without_payload_copies() {
    let source = format!(
        "{}\n{}\n{}\npath-write-items(sequence/reserve(bytes<[u8]>, reserve<u64>), bitbuf<u64>, bitcount<u64>, items<[u64]>, block-length<u64>, litlen-words<[u32]>, litlen-lens<[u8]>, full-words<[u32]>, full-lens<[u8]>, offset-words<[u32]>, offset-lens<[u8]>, offset-slots<[u8]>, offset-bases<[u32]>, offset-extra<[u8]>)\n",
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        include_str!("../fixtures/native/deflate-optimal-path.mec"),
        include_str!("../fixtures/native/deflate-path-writer.mec")
    );
    let (prefix, call) = source.rsplit_once("\npath-write-items(").unwrap();
    let source = format!(
        "{prefix}\nwritten := path-write-items({}\nsequence/append(sequence/clear(written.bytes), 17u8)\n",
        call.trim()
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-path-writer-reference.json"
    ))
    .unwrap();
    let matrix = |v: &serde_json::Value, width| {
        ValueDataDraft::Matrix(
            v.as_array()
                .unwrap()
                .iter()
                .map(|n| match width {
                    8 => ValueDataDraft::U8(n.as_u64().unwrap() as u8),
                    32 => ValueDataDraft::U32(n.as_u64().unwrap() as u32),
                    _ => ValueDataDraft::U64(n.as_u64().unwrap()),
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
            "path-writer blocks={} form={form}",
            jit.descriptor().plan().blocks().len()
        );
        for case in fixture["cases"].as_array().unwrap() {
            let mut allocated = Vec::new();
            for capacity in [512, 6633832] {
                let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
                let mut data = vec![
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
                    ("items", matrix(&case["items"], 64)),
                    (
                        "block-length",
                        ValueDataDraft::U64(case["block_length"].as_u64().unwrap()),
                    ),
                ];
                for (name, width) in [
                    ("litlen-words", 32),
                    ("litlen-lens", 8),
                    ("full-words", 32),
                    ("full-lens", 8),
                    ("offset-words", 32),
                    ("offset-lens", 8),
                    ("offset-slots", 8),
                    ("offset-bases", 32),
                    ("offset-extra", 8),
                ] {
                    data.push((name, matrix(&fixture["tables"][name], width)));
                }
                let args = inputs(&artifact, &data, &memory);
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
