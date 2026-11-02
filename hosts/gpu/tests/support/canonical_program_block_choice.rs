//! Exact block-cost calculation and stored/static/dynamic tie precedence.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-choice.mec"),
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
fn pinned_block_costs_and_choice_match_reference() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-block-choice-reference.json"
    ))
    .unwrap();
    let source = format!(
        "{COMPONENTS}precode := {{freqs: pre-freqs<[u32]>, lens: pre-lens<[u8]>, codewords: pre-words<[u32]>, items: pre-items<[u32]>, num-items: pre-count<u64>, num-litlen-syms: pre-litlen<u64>, num-offset-syms: pre-offset<u64>, num-explicit-lens: pre-explicit<u64>}}\nblock-compute-costs(precode, frequencies<[u32]>, offsets<[u32]>, lens<[u8]>, offset-lens<[u8]>, fixed-lens<[u8]>, extra-precode<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>, length<u64>, bitcount<u64>)\n"
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
                    ("frequencies", matrix(&case["freqs_litlen"], 32)),
                    ("offsets", matrix(&case["freqs_offset"], 32)),
                    ("lens", matrix(&case["litlen_lens"], 8)),
                    ("offset-lens", matrix(&case["offset_lens"], 8)),
                    (
                        "fixed-lens",
                        matrix(&fixture["tables"]["fixed_litlen_lens"], 8),
                    ),
                    (
                        "extra-precode",
                        matrix(&fixture["tables"]["extra_precode_bits"], 8),
                    ),
                    (
                        "extra-lengths",
                        matrix(&fixture["tables"]["extra_length_bits"], 8),
                    ),
                    (
                        "extra-offsets",
                        matrix(&fixture["tables"]["extra_offset_bits"], 8),
                    ),
                    ("length", Data::U64(case["block_length"].as_u64().unwrap())),
                    ("bitcount", Data::U64(case["bitcount"].as_u64().unwrap())),
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
            for name in ["dynamic", "fixed", "stored", "kind"] {
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
#[test]
fn block_choice_ties_prefer_stored_then_fixed_then_dynamic() {
    let source = format!("{COMPONENTS}block-choose-kind(dynamic<u64>, fixed<u64>, stored<u64>)\n");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for (dynamic, fixed, stored, expected) in [
            (7, 7, 7, 0),
            (7, 7, 8, 1),
            (7, 8, 7, 0),
            (8, 7, 7, 0),
            (6, 7, 8, 2),
            (8, 6, 7, 1),
            (8, 7, 6, 0),
        ] {
            let memory = ManagedMemoryBudget::new(1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("dynamic", Data::U64(dynamic)),
                    ("fixed", Data::U64(fixed)),
                    ("stored", Data::U64(stored)),
                ],
                &memory,
            );
            let mut output = vec![None; artifact.outputs().len()];
            jit.evaluate_into(&args, &mut output, limits(), &memory)
                .unwrap();
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
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
        }
    }
}
