//! Forward path tally and real Huffman allocation sizes, in one Mech program.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};

const COMPONENTS: &str = concat!(
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-tally.mec"),
    "\n"
);
const TALLY: &str = "reversed := huff-build-reversal()\noptimal-tally-and-build-codes(nodes<[u64]>, block-length<u64>, length-slots<[u8]>, slots<[u8]>, reversed)\n";
const PATH_TALLY: &str = "reversed := huff-build-reversal()\npath := optimal-find-min-cost-path(block-length<u64>, cache<[u32]>, cache-end<u64>, sequence/with-capacity(0u64, 305257u64), literal-costs<[u32]>, length-costs<[u32]>, offset-costs<[u32]>, slots<[u8]>)\noptimal-tally-and-build-codes(path.nodes, block-length, length-slots<[u8]>, slots, reversed)\n";

fn fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fixtures/native/deflate-tally-reference.json"
    ))
    .unwrap()
}
fn bytes(value: &serde_json::Value) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| Data::U8(v.as_u64().unwrap() as u8))
            .collect(),
    )
}
fn words(value: &serde_json::Value) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| Data::U32(v.as_u64().unwrap() as u32))
            .collect(),
    )
}
fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}
fn compile(artifact: &mech_engine::ProgramArtifact) -> JitProgramArtifact {
    // Same explicit finite diagnostic as collector/path qualification.
    let jit = JitProgramArtifact::compile_with_limits(
        artifact,
        &mech_stdlib::source_catalog(),
        NativeCompileLimits {
            max_nodes: 16_384,
            ..Default::default()
        },
    )
    .unwrap();
    println!(
        "tally native blocks={}",
        jit.descriptor().plan().blocks().len()
    );
    jit
}
fn check_tally(value: &mech_core::Value, case: &serde_json::Value) {
    let Data::Record(fields) = value.canonical_data_draft().unwrap() else {
        panic!()
    };
    for (name, key, size, width) in [
        ("freqs-litlen", "freqs_litlen", 513, 32),
        ("freqs-offset", "freqs_offset", 270, 32),
        ("litlen-lens", "litlen_lens", 288, 8),
        ("litlen-codewords", "litlen_codewords", 288, 32),
        ("offset-lens", "offset_lens", 32, 8),
        ("offset-codewords", "offset_codewords", 32, 32),
    ] {
        assert_eq!(case[key].as_array().unwrap().len(), size);
        let expected = if width == 8 {
            bytes(&case[key])
        } else {
            words(&case[key])
        };
        assert_eq!(
            &fields.iter().find(|v| v.name == name).unwrap().value,
            &expected,
            "{} {name}",
            case["name"]
        );
    }
}

#[test]
fn forward_tally_and_codes_match_every_pinned_buffer() {
    let fixture = fixture();
    let source = format!("{COMPONENTS}{TALLY}");
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = compile(&artifact);
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    (
                        "nodes",
                        Data::Matrix(
                            case["nodes"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| Data::U64(v.as_u64().unwrap()))
                                .collect(),
                        ),
                    ),
                    (
                        "block-length",
                        Data::U64(case["block_length"].as_u64().unwrap()),
                    ),
                    ("length-slots", bytes(&fixture["length_slots"])),
                    ("slots", bytes(&fixture["offset_slots"])),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, requested_bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"forward-tally-huffman","artifact":form,"case":case["name"],"block_length":case["block_length"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":requested_bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            check_tally(output.last().unwrap().as_ref().unwrap(), case);
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
fn one_program_finds_the_path_and_builds_the_pinned_codes() {
    let fixture = fixture();
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    let source = format!("{COMPONENTS}{PATH_TALLY}");
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = compile(&artifact);
        for case in fixture["cases"].as_array().unwrap() {
            let path = paths["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|p| p["name"] == case["name"])
                .unwrap();
            let mut cache = vec![Data::U32(0); 1500513];
            for entry in path["cache"].as_array().unwrap() {
                cache[entry[0].as_u64().unwrap() as usize] =
                    Data::U32(entry[1].as_u64().unwrap() as u32);
            }
            let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    (
                        "block-length",
                        Data::U64(case["block_length"].as_u64().unwrap()),
                    ),
                    ("cache", Data::Matrix(cache.into())),
                    ("cache-end", Data::U64(path["cache_end"].as_u64().unwrap())),
                    ("literal-costs", words(&path["literal_costs"])),
                    ("length-costs", words(&path["length_costs"])),
                    ("offset-costs", words(&path["offset_costs"])),
                    ("length-slots", bytes(&fixture["length_slots"])),
                    ("slots", bytes(&fixture["offset_slots"])),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, requested_bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"path-tally-huffman","artifact":form,"case":case["name"],"block_length":case["block_length"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":requested_bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            check_tally(output.last().unwrap().as_ref().unwrap(), case);
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
fn one_program_collects_matches_selects_path_and_builds_codes() {
    let collector = concat!(
        include_str!("../fixtures/native/deflate-matchfinder.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-binary-tree.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-collector.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-block-collector.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-block-heuristics.mec"),
        "\n",
        include_str!("../fixtures/native/deflate-block-loop.mec"),
        "\n"
    );
    let entry = "reversed := huff-build-reversal()\ncollected := optimal-first-block(input<[u8]>)\nblock-length := collected.block-end - collected.block-begin\npath := optimal-find-min-cost-path(block-length, collected.cache, collected.cache-end, sequence/with-capacity(0u64, 305257u64), literal-costs<[u32]>, length-costs<[u32]>, offset-costs<[u32]>, slots<[u8]>)\noptimal-tally-and-build-codes(path.nodes, block-length, length-slots<[u8]>, slots, reversed)\n";
    let source = format!("{collector}{COMPONENTS}{entry}");
    let fixture = fixture();
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    let blocks: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-block-reference.json"
    ))
    .unwrap();
    for (form, artifact) in ["original", "decoded"]
        .into_iter()
        .zip(artifacts(&source).0)
    {
        let jit = compile(&artifact);
        for name in ["eight-literals", "eight-overlap"] {
            let case = fixture["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap();
            let path = paths["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap();
            let block = blocks["cases"]
                .as_array()
                .unwrap()
                .iter()
                .find(|c| c["name"] == name)
                .unwrap();
            let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
            let args = inputs(
                &artifact,
                &[
                    ("input", bytes(&block["input"])),
                    ("literal-costs", words(&path["literal_costs"])),
                    ("length-costs", words(&path["length_costs"])),
                    ("offset-costs", words(&path["offset_costs"])),
                    ("length-slots", bytes(&fixture["length_slots"])),
                    ("slots", bytes(&fixture["offset_slots"])),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let mut policy = policy();
            // The measured combined frame is 1,103,136 bytes. This explicit
            // finite diagnostic allowance does not alter compiler defaults.
            policy.execution.max_transport_bytes = 2 * 1024 * 1024;
            policy.library.max_output_elements = Some(1500513);
            let start = std::time::Instant::now();
            let (result, requests, requested_bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"collector-path-tally-huffman","artifact":form,"case":name,"completed_positions":if result.is_ok() {8} else {0},"tree_entries":262144,"cache_words":1500513,"nodes_reservation":305257,"depth":300,"nice":258,"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":requested_bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            check_tally(output.last().unwrap().as_ref().unwrap(), case);
            if name == "eight-literals" {
                let old = output.clone();
                let retained = memory.used_bytes();
                let nodes = memory.retained_snapshot_nodes();
                let mut denied = policy;
                denied.execution.max_transport_bytes = 1024 * 1024;
                let (rejection, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, denied, &memory)
                });
                let error = rejection.unwrap_err();
                assert!(format!("{error:?}").contains("TransportLimit"), "{error:?}");
                assert_eq!((requests, bytes), (0, 0));
                assert!(old.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
                jit.evaluate_into(&args, &mut output, policy, &memory)
                    .unwrap();
                check_tally(output.last().unwrap().as_ref().unwrap(), case);
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
