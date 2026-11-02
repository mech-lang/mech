use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;
fn matrix(value: &serde_json::Value, width: u32) -> Data {
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
const HEADER: &str = concat!(
    include_str!("../fixtures/native/deflate-matchfinder.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-block-heuristics.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-costs.mec"),
    "\n"
);
#[test]
fn default_cost_selection_matches_pinned_input_statistics() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-default-costs-reference.json"
    ))
    .unwrap();
    let source = format!(
        "{HEADER}optimal-choose-default-costs(input<[u8]>, begin<u64>, length<u64>, frequencies<[u32]>, depth<u64>, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbols<[u8]>)\n"
    );
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("input", matrix(&case["input"], 8)),
                    ("begin", Data::U64(case["begin"].as_u64().unwrap())),
                    ("length", Data::U64(case["length"].as_u64().unwrap())),
                    ("frequencies", matrix(&case["frequencies"], 32)),
                    ("depth", Data::U64(case["depth"].as_u64().unwrap())),
                    (
                        "table0",
                        matrix(&fixture["tables"]["default_lit_cost_0"], 8),
                    ),
                    (
                        "table1",
                        matrix(&fixture["tables"]["default_lit_cost_1"], 8),
                    ),
                    (
                        "table2",
                        matrix(&fixture["tables"]["default_lit_cost_2"], 8),
                    ),
                    (
                        "symbols",
                        matrix(&fixture["tables"]["default_len_sym_costs"], 8),
                    ),
                ],
                &memory,
            );
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
                serde_json::json!({"witness":"default-cost-choice","artifact":form,"case":case["name"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
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
            for (name, at) in [("lit-cost", 0), ("len-sym-cost", 1)] {
                assert_eq!(
                    fields.iter().find(|f| f.name == name).unwrap().value,
                    Data::U64(case["expected"][at].as_u64().unwrap()),
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
#[test]
fn signed_wrap_for_default_cost_statistics_preserves_all_u64_bits() {
    let source = format!("{HEADER}optimal-to-i64-wrap(value<u64>)\n");
    for artifact in artifacts(&source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for value in [
            0u64,
            1,
            9223372036854775807,
            9223372036854775808,
            18446744073709551614,
            u64::MAX,
        ] {
            let args = inputs(&artifact, &[("value", Data::U64(value))], &memory);
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
                Data::I64(value as i64)
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
