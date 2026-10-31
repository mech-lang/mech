//! Writer packed tables preserve the pinned words and reserved append buffers.
use super::{artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-writer-tables.mec"),
    "\n"
);
fn matrix(value: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| match width {
                8 => Data::U8(v.as_u64().unwrap().try_into().unwrap()),
                32 => Data::U32(v.as_u64().unwrap().try_into().unwrap()),
                64 => Data::U64(v.as_u64().unwrap()),
                _ => panic!(),
            })
            .collect(),
    )
}
#[test]
fn writer_packed_tables_match_reference_and_retain_reserved_capacity() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-writer-tables-reference.json"
    ))
    .unwrap();
    for kind in ["codes", "offsets"] {
        let entry = if kind == "codes" {
            "block-pack-codes(words<[u32]>, lens<[u8]>)\n"
        } else {
            "block-pack-offsets(words<[u32]>, lens<[u8]>, extra<[u8]>, bases<[u32]>)\n"
        };
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&format!("{COMPONENTS}{entry}")).0)
        {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            for case in fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|case| case["kind"] == kind)
            {
                let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
                let mut named = vec![
                    ("words", matrix(&case["codewords"], 32)),
                    ("lens", matrix(&case["lens"], 8)),
                ];
                if kind == "offsets" {
                    named.push(("extra", matrix(&fixture["extra_offset_bits"], 8)));
                    named.push(("bases", matrix(&fixture["offset_slot_base"], 32)));
                }
                let args = inputs(&artifact, &named, &memory);
                let aliases = args.clone();
                let mut output = vec![None; artifact.outputs().len()];
                let start = std::time::Instant::now();
                // Use the existing compressor component diagnostic policy. The first
                // default-policy attempt is retained as a resource-denial receipt.
                let mut policy = limits();
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                policy.library.max_comparison_work = Some(1_000_000_000_000);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                let value = output.last().unwrap().as_ref().unwrap();
                assert_eq!(
                    value.canonical_data_draft().unwrap(),
                    matrix(&case["packed"], 64)
                );
                let ValueData::Matrix(buffer) = value.data() else {
                    panic!()
                };
                assert_eq!(
                    buffer.unsigned_buffer_capacity(),
                    Some(case["codewords"].as_array().unwrap().len())
                );
                println!(
                    "writer-tables {form} {} seconds={}",
                    case["name"],
                    start.elapsed().as_secs_f64()
                );
                assert!(
                    args.iter()
                        .zip(&aliases)
                        .all(|(a, b)| a.shares_frozen_storage(b))
                );
                drop((args, aliases, output));
                assert_eq!(memory.used_bytes(), 0);
                assert_eq!(memory.retained_snapshot_nodes(), 0);
            }
        }
    }
}

#[test]
fn writer_full_length_tables_match_reference() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-writer-tables-reference.json"
    ))
    .unwrap();
    let source = format!(
        "{COMPONENTS}block-compute-full-length-codewords(lens<[u8]>, words<[u32]>, slots<[u8]>, bases<[u32]>, extra<[u8]>)\n"
    );
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for case in fixture["full_cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(32 * 1024 * 1024);
            let named = vec![
                ("lens", matrix(&case["lens"], 8)),
                ("words", matrix(&case["codewords"], 32)),
                ("slots", matrix(&fixture["length_slot_tab"], 8)),
                ("bases", matrix(&fixture["length_slot_base"], 32)),
                ("extra", matrix(&fixture["extra_length_bits"], 8)),
            ];
            let args = inputs(&artifact, &named, &memory);
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = limits();
            policy.execution.max_work = 1_000_000_000_000;
            policy.library.max_compute_work = Some(1_000_000_000_000);
            policy.library.max_comparison_work = Some(1_000_000_000_000);
            let start = std::time::Instant::now();
            jit.evaluate_into(&args, &mut output, policy, &memory)
                .unwrap();
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
            for (name, width) in [("codewords", 32), ("lens", 8)] {
                assert_eq!(
                    fields
                        .iter()
                        .find(|field| field.name == name)
                        .unwrap()
                        .value,
                    matrix(&case["expected"][name], width)
                );
            }
            println!(
                "full-length {form} {} seconds={}",
                case["name"],
                start.elapsed().as_secs_f64()
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            drop((args, aliases, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}
