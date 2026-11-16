//! Static/dynamic block header bytes and bit carry from executed reference code.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-path-writer.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-header-writer.mec"),
    "\n"
);
fn matrix(v: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        v.as_array()
            .unwrap()
            .iter()
            .map(|n| match width {
                8 => Data::U8(n.as_u64().unwrap().try_into().unwrap()),
                32 => Data::U32(n.as_u64().unwrap().try_into().unwrap()),
                _ => panic!(),
            })
            .collect(),
    )
}
#[test]
fn pinned_block_headers_match_reference_with_real_reservation() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-block-header-writer-reference.json"
    ))
    .unwrap();
    let source = format!(
        "{COMPONENTS}precode := {{freqs: pre-freqs<[u32]>, lens: pre-lens<[u8]>, codewords: pre-words<[u32]>, items: pre-items<[u32]>, num-items: pre-count<u64>, num-litlen-syms: pre-litlen<u64>, num-offset-syms: pre-offset<u64>, num-explicit-lens: pre-explicit<u64>}}\nbytes := sequence/reserve(bytes0<[u8]>, reserve<u64>)\n(static<bool>? | true => block-write-static-header(bytes, bitbuf<u64>, bitcount<u64>, final<u64>) | false => block-write-dynamic-header(bytes, bitbuf, bitcount, final, precode, permutation<[u8]>, extra-precode<[u8]>))\n"
    );
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(128 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("pre-freqs", matrix(&case["precode"]["freqs"], 32)),
                    ("pre-lens", matrix(&case["precode"]["lens"], 8)),
                    ("pre-words", matrix(&case["precode"]["codewords"], 32)),
                    ("pre-items", matrix(&case["precode"]["items"], 32)),
                    (
                        "pre-count",
                        Data::U64(case["precode"]["num_items"].as_u64().unwrap()),
                    ),
                    (
                        "pre-litlen",
                        Data::U64(case["precode"]["num_litlen_syms"].as_u64().unwrap()),
                    ),
                    (
                        "pre-offset",
                        Data::U64(case["precode"]["num_offset_syms"].as_u64().unwrap()),
                    ),
                    (
                        "pre-explicit",
                        Data::U64(case["precode"]["num_explicit_lens"].as_u64().unwrap()),
                    ),
                    ("bytes0", matrix(&case["out"], 8)),
                    (
                        "reserve",
                        Data::U64(6633832 - case["out"].as_array().unwrap().len() as u64),
                    ),
                    ("bitbuf", Data::U64(case["bitbuf"].as_u64().unwrap())),
                    ("bitcount", Data::U64(case["bitcount"].as_u64().unwrap())),
                    ("final", Data::U64(case["final"].as_u64().unwrap())),
                    ("static", Data::Bool(case["static"].as_bool().unwrap())),
                    (
                        "permutation",
                        matrix(&fixture["tables"]["precode_lens_permutation"], 8),
                    ),
                    (
                        "extra-precode",
                        matrix(&fixture["tables"]["extra_precode_bits"], 8),
                    ),
                ],
                &memory,
            );
            let mut policy = limits();
            policy.execution.max_work = 1_000_000_000_000;
            policy.library.max_compute_work = Some(1_000_000_000_000);
            policy.library.max_comparison_work = Some(1_000_000_000_000);
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"case":case["name"],"form":form,"seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"allocated_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            let Data::Record(fields) = output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!()
            };
            assert_eq!(
                fields.iter().find(|f| f.name == "bytes").unwrap().value,
                matrix(&case["expected"]["out"], 8),
                "{} bytes",
                case["name"]
            );
            for name in ["bitbuf", "bitcount"] {
                assert_eq!(
                    fields.iter().find(|f| f.name == name).unwrap().value,
                    Data::U64(case["expected"][name].as_u64().unwrap()),
                    "{} {name}",
                    case["name"]
                );
            }
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
