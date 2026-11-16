//! Exact stored-block bytes, independent decompression and internal capacity reuse.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-path-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-stored-writer.mec"),
    "\n"
);
fn matrix(bytes: &[u8]) -> Data {
    Data::Matrix(bytes.iter().copied().map(Data::U8).collect())
}
fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-stored-writer-reference.json"
    ))
    .unwrap()
}
fn golden(case: &serde_json::Value) -> Vec<u8> {
    let hex = case["expected"]["zlib_hex"].as_str().unwrap();
    let compressed = (0..hex.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
        .collect::<Vec<_>>();
    miniz_oxide::inflate::decompress_to_vec_zlib(&compressed).unwrap()
}
fn source(reuse: bool) -> String {
    let tail = if reuse {
        "sequence/append(sequence/clear(written.bytes), 17u8)"
    } else {
        "written"
    };
    format!(
        "{COMPONENTS}written := block-write-stored(sequence/reserve(bytes<[u8]>, reserve<u64>), bitbuf<u64>, bitcount<u64>, input<[u8]>, begin<u64>, length<u64>, final<u64>)\n{tail}\n"
    )
}
#[test]
fn pinned_stored_writer_matches_reference_and_decompresses() {
    for reuse in [false, true] {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source(reuse)).0)
        {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            for case in fixture()["cases"].as_array().unwrap() {
                let begin = case["begin"].as_u64().unwrap();
                let length = case["length"].as_u64().unwrap();
                let input = (0..begin + length + 2).map(|n| n as u8).collect::<Vec<_>>();
                let prefix = case["out"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|n| n.as_u64().unwrap() as u8)
                    .collect::<Vec<_>>();
                let expected = golden(case);
                assert_eq!(
                    expected.len() as u64,
                    case["expected"]["length"].as_u64().unwrap()
                );
                let capacities = if reuse {
                    vec![131072u64, 6633832]
                } else {
                    vec![6633832]
                };
                let mut allocations = Vec::new();
                for capacity in capacities {
                    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                    let args = inputs(
                        &artifact,
                        &[
                            ("bytes", matrix(&prefix)),
                            ("reserve", Data::U64(capacity - prefix.len() as u64)),
                            ("input", matrix(&input)),
                            ("bitbuf", Data::U64(case["bitbuf"].as_u64().unwrap())),
                            ("bitcount", Data::U64(case["bitcount"].as_u64().unwrap())),
                            ("begin", Data::U64(begin)),
                            ("length", Data::U64(length)),
                            ("final", Data::U64(case["final"].as_u64().unwrap())),
                        ],
                        &memory,
                    );
                    let mut policy = limits();
                    // Use the existing full-block qualification bound: stored headers
                    // make a 65,535-byte payload larger than the generic 65,536 cap.
                    policy.library.max_output_elements = Some(1_500_513);
                    policy.execution.max_work = 1_000_000_000_000;
                    policy.library.max_compute_work = Some(1_000_000_000_000);
                    policy.library.max_comparison_work = Some(1_000_000_000_000);
                    let mut output = vec![None; artifact.outputs().len()];
                    if !reuse && length == 65535 {
                        let retained = memory.used_bytes();
                        let mut narrow = policy;
                        narrow.library.max_output_elements = Some(65536);
                        let error = jit
                            .evaluate_into(&args, &mut output, narrow, &memory)
                            .unwrap_err();
                        let message = format!("{error:?}");
                        assert!(
                            message.contains("OutputElements") && message.contains("65540"),
                            "{message}"
                        );
                        assert!(output.iter().all(Option::is_none));
                        assert_eq!(memory.used_bytes(), retained);
                    }

                    jit.evaluate_into(&args, &mut output, policy, &memory)
                        .unwrap();
                    let before = std::time::Instant::now();
                    let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                        jit.evaluate_into(&args, &mut output, policy, &memory)
                    });
                    result.unwrap();
                    allocations.push(bytes);
                    println!(
                        "{}",
                        serde_json::json!({"case":case["name"],"form":form,"reuse":reuse,"capacity":capacity,"seconds":before.elapsed().as_secs_f64(),"allocation_requests":requests,"allocated_bytes":bytes})
                    );
                    let value = output.last().unwrap().as_ref().unwrap();
                    if reuse {
                        assert_eq!(value.canonical_data_draft().unwrap(), matrix(&[17]));
                        let ValueData::Matrix(words) = value.data() else {
                            panic!()
                        };
                        assert_eq!(words.unsigned_buffer_capacity(), Some(capacity as usize));
                    } else {
                        let Data::Record(fields) = value.canonical_data_draft().unwrap() else {
                            panic!()
                        };
                        let actual = &fields.iter().find(|f| f.name == "bytes").unwrap().value;
                        assert_eq!(actual, &matrix(&expected), "{} {form}", case["name"]);
                        for name in ["bitbuf", "bitcount"] {
                            assert_eq!(
                                fields.iter().find(|f| f.name == name).unwrap().value,
                                Data::U64(case["expected"][name].as_u64().unwrap())
                            );
                        }
                        if case["independent_decompression"].as_bool() == Some(true) {
                            let Data::Matrix(words) = actual else {
                                panic!()
                            };
                            let raw = words
                                .iter()
                                .map(|v| if let Data::U8(n) = v { *n } else { panic!() })
                                .collect::<Vec<_>>();
                            assert_eq!(
                                miniz_oxide::inflate::decompress_to_vec(&raw).unwrap(),
                                input[begin as usize..(begin + length) as usize]
                            );
                        }
                    }
                    drop((args, output));
                    assert_eq!(memory.used_bytes(), 0);
                    assert_eq!(memory.retained_snapshot_nodes(), 0);
                }
                if reuse {
                    assert_eq!(
                        allocations[1] - allocations[0],
                        6633832 - 131072,
                        "{} {form}",
                        case["name"]
                    );
                }
            }
        }
    }
}
