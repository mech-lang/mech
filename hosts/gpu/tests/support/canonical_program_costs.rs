use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
fn data(value: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| {
                if width == 8 {
                    Data::U8(v.as_u64().unwrap() as u8)
                } else {
                    Data::U32(v.as_u64().unwrap() as u32)
                }
            })
            .collect(),
    )
}
fn qualify_cost_models(header: &str, source_variant: &str) {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-costs-reference.json"
    ))
    .unwrap();
    for (mode, entry) in [
        (
            0,
            "optimal-set-default-costs(lit-cost<u64>, len-cost<u64>, literal<[u32]>, length<[u32]>, offset<[u32]>, slots<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>)",
        ),
        (
            1,
            "optimal-set-costs-from-codes(litlen-lens<[u8]>, offset-lens<[u8]>, literal<[u32]>, length<[u32]>, offset<[u32]>, slots<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>)",
        ),
        (
            2,
            "optimal-adjust-costs-impl(lit-cost<u64>, len-cost<u64>, change<u64>, literal<[u32]>, length<[u32]>, offset<[u32]>, slots<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>)",
        ),
    ] {
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&format!("{header}{entry}\n")).0)
        {
            let jit =
                JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
            for case in fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|c| c["mode"].as_u64() == Some(mode))
            {
                let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
                let mut named = vec![
                    ("lit-cost", Data::U64(case["lit_cost"].as_u64().unwrap())),
                    ("len-cost", Data::U64(case["len_cost"].as_u64().unwrap())),
                    ("change", Data::U64(case["change"].as_u64().unwrap())),
                    ("literal", data(&case["literal"], 32)),
                    ("length", data(&case["length"], 32)),
                    ("offset", data(&case["offset_slot"], 32)),
                    ("slots", data(&fixture["tables"]["length_slot_tab"], 8)),
                    (
                        "extra-lengths",
                        data(&fixture["tables"]["extra_length_bits"], 8),
                    ),
                    (
                        "extra-offsets",
                        data(&fixture["tables"]["extra_offset_bits"], 8),
                    ),
                ];
                if mode == 1 {
                    named.push(("litlen-lens", data(&case["litlen_lens"], 8)));
                    named.push(("offset-lens", data(&case["offset_lens"], 8)));
                }
                let args = inputs(&artifact, &named, &memory);
                let aliases = args.clone();
                let mut output = vec![None; artifact.outputs().len()];
                let mut policy = limits();
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                policy.library.max_comparison_work = Some(1_000_000_000_000);
                let start = std::time::Instant::now();
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                });
                println!(
                    "{}",
                    serde_json::json!({"witness":"real-cost-model-updates","source_variant":source_variant,"artifact":form,"case":case["name"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
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
                for (name, key) in [
                    ("literal", "literal"),
                    ("length", "length"),
                    ("offset-slot", "offset_slot"),
                ] {
                    assert_eq!(
                        &fields.iter().find(|f| f.name == name).unwrap().value,
                        &data(&case["expected"][key], 32),
                        "{} {name}",
                        case["name"]
                    );
                }
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

#[path = "../../../../benchmarks/mech-deflate/source-variants/independent_tables.rs"]
mod independent_tables;

#[test]
fn real_cost_model_updates_match_pinned_functions() {
    let header = concat!(
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-optimal-costs.mec"),
        "\n"
    );
    qualify_cost_models(header, "pinned");
}

#[test]
fn independently_constructed_cost_tables_match_unchanged_reference_cases() {
    let header = concat!(
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-optimal-costs.mec"),
        "\n"
    );
    qualify_cost_models(&independent_tables::source(header), "independent-tables");
}

#[path = "canonical_program_cost_variant_bounds.rs"]
mod variant_bounds;
