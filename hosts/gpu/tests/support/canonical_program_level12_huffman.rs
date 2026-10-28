//! Real compressor alphabets, scratch allocation sites and reversal table.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const SOURCE: &str = concat!(
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\nreversed := huff-build-reversal()\nlens0 := [0u8 | i <- 0u64..=n<u64>, i < n]\nwords0 := [0u32 | i <- 0u64..=n, i < n]\nhuff-make-code(n, maximum<u64>, frequencies<[u32]>, lens0, words0, reversed)\n"
);

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-huffman-level12-reference.json"
    ))
    .unwrap()
}

fn case_inputs(
    artifact: &mech_engine::ProgramArtifact,
    case: &serde_json::Value,
    memory: &ManagedMemoryBudget,
) -> Vec<mech_core::Value> {
    inputs(
        artifact,
        &[
            ("n", Data::U64(case["n"].as_u64().unwrap())),
            ("maximum", Data::U64(case["maximum"].as_u64().unwrap())),
            (
                "frequencies",
                Data::Matrix(
                    case["frequencies"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| Data::U32(v.as_u64().unwrap() as u32))
                        .collect(),
                ),
            ),
        ],
        memory,
    )
}

fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    // Explicit finite component qualification; no default-policy claim.
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}

fn check_code(output: &mech_core::Value, case: &serde_json::Value) {
    let Data::Record(fields) = output.canonical_data_draft().unwrap() else {
        panic!()
    };
    for (name, key, width) in [("lens", "lens", 8), ("codewords", "words", 32)] {
        let actual = &fields.iter().find(|v| v.name == name).unwrap().value;
        let expected = Data::Matrix(
            case[key]
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
        );
        assert_eq!(actual, &expected, "{} {name}", case["name"]);
    }
}

#[test]
fn real_huffman_buffers_and_reversal_match_pinned_outputs() {
    let fixture = fixture();
    assert_eq!(fixture["cases"].as_array().unwrap().len(), 17);
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(SOURCE).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        for case in fixture["cases"].as_array().unwrap() {
            let n = case["n"].as_u64().unwrap();
            assert_eq!(
                case["frequencies"].as_array().unwrap().len(),
                match n {
                    288 => 513,
                    32 => 270,
                    19 => 19,
                    _ => panic!(),
                }
            );
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = case_inputs(&artifact, case, &memory);
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"real-huffman-integration","artifact":form,"case":case["name"],"alphabet":n,"input_frequencies":case["frequencies"].as_array().unwrap().len(),"maximum":case["maximum"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            check_code(output.last().unwrap().as_ref().unwrap(), case);
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
fn huffman_checked_failure_preserves_publication_and_recovers() {
    let fixture = fixture();
    let valid = &fixture["cases"][0];
    let mut invalid = valid.clone();
    invalid["n"] = 0.into(); // sort_symbols checks n - 1 before allocating counters.
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        for (case, reject) in [(valid, false), (&invalid, true), (valid, false)] {
            let args = case_inputs(&artifact, case, &memory);
            let aliases = args.clone();
            let old = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let result = jit.evaluate_into(&args, &mut output, policy(), &memory);
            if reject {
                let error = result.unwrap_err();
                assert!(format!("{error:?}").contains("Arithmetic"), "{error:?}");
                assert!(old.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
            } else {
                result.unwrap();
                check_code(output.last().unwrap().as_ref().unwrap(), valid);
            }
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
        }
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}

#[test]
fn shared_reversal_table_matches_every_pinned_byte_and_keeps_its_reservation() {
    use mech_core::ValueData;
    let fixture = fixture();
    let table = fixture["reversal_table"].as_array().unwrap();
    assert_eq!(table.len(), 256);
    for (i, byte) in table.iter().enumerate() {
        assert_eq!(byte.as_u64().unwrap() as u8, (i as u8).reverse_bits());
    }
    let source = concat!(
        include_str!("../fixtures/native/deflate-huffman-level12.mec"),
        "\nhuff-build-reversal()\n"
    );
    for artifact in artifacts(source).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&[], &mut output, policy(), &memory)
            .unwrap();
        let value = output.last().unwrap().as_ref().unwrap();
        assert_eq!(
            value.canonical_data_draft().unwrap(),
            Data::Matrix(
                table
                    .iter()
                    .map(|v| Data::U8(v.as_u64().unwrap() as u8))
                    .collect()
            )
        );
        let ValueData::Matrix(matrix) = value.data() else {
            panic!()
        };
        assert_eq!(matrix.unsigned_buffer_capacity(), Some(256));
        drop(output);
        assert_eq!(memory.used_bytes(), 0);
        assert_eq!(memory.retained_snapshot_nodes(), 0);
    }
}
