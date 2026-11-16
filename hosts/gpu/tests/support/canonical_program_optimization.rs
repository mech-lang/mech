use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueDataDraft as Data};
use mech_gpu::native_artifact::{JitProgramArtifact, NativeCompileLimits};

const COMPONENTS: &str = concat!(
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
    "\n",
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-huffman-level12.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-tally.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimal-costs.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-header.mec"),
    "\n",
    include_str!("../fixtures/native/deflate-optimize.mec"),
    "\n"
);
const SETUP: &str = "reversed := huff-build-reversal()\nnodes0 := sequence/with-capacity(0u64, 305257u64)\nliteral0 := [0u32 | i <- 0u64..=256u64, i < 256u64]\nlength0 := [0u32 | i <- 0u64..=259u64, i < 259u64]\noffset0 := [0u32 | i <- 0u64..=32u64, i < 32u64]\n";
fn matrix(value: &serde_json::Value, width: u32) -> Data {
    Data::Matrix(
        value
            .as_array()
            .unwrap()
            .iter()
            .map(|v| match width {
                8 => Data::U8(v.as_u64().unwrap() as u8),
                32 => Data::U32(v.as_u64().unwrap() as u32),
                64 => Data::U64(v.as_u64().unwrap()),
                _ => panic!(),
            })
            .collect(),
    )
}
fn field<'a>(value: &'a Data, name: &str) -> &'a Data {
    let Data::Record(fields) = value else {
        panic!("expected record for {name}")
    };
    &fields.iter().find(|f| f.name == name).unwrap().value
}
fn check(value: &mech_core::Value, case: &serde_json::Value) {
    let data = value.canonical_data_draft().unwrap();
    for (name, key) in [
        ("best-true-cost", "best_true_cost"),
        ("actual-cost", "true_cost"),
        ("passes-remaining", "passes_remaining"),
    ] {
        assert_eq!(
            field(&data, name),
            &Data::U64(case["expected"][key].as_u64().unwrap())
        );
    }
    for (name, key) in [
        ("literal", "saved_literal"),
        ("length", "saved_length"),
        ("offset-slot", "saved_offset_slot"),
    ] {
        assert_eq!(
            field(field(&data, "saved"), name),
            &matrix(&case["expected"][key], 32)
        );
    }
    assert_eq!(
        field(&data, "nodes"),
        &matrix(&case["expected"]["nodes"], 64)
    );
    for (name, key) in [
        ("literal", "literal"),
        ("length", "length"),
        ("offset-slot", "offset_slot"),
    ] {
        assert_eq!(
            field(field(&data, "costs"), name),
            &matrix(&case["expected"][key], 32),
            "{} {name}",
            case["name"]
        );
    }
    for (name, key, width) in [
        ("freqs-litlen", "freqs_litlen", 32),
        ("freqs-offset", "freqs_offset", 32),
        ("litlen-lens", "litlen_lens", 8),
        ("litlen-codewords", "litlen_codewords", 32),
        ("offset-lens", "offset_lens", 8),
        ("offset-codewords", "offset_codewords", 32),
    ] {
        assert_eq!(
            field(field(&data, "tallied"), name),
            &matrix(&case["expected"][key], width),
            "{} {name}",
            case["name"]
        );
    }
}
#[test]
fn optimization_loop_matches_pinned_passes() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimization-reference.json"
    ))
    .unwrap();
    let tables: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-costs-reference.json"
    ))
    .unwrap();
    let defaults: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-default-costs-reference.json"
    ))
    .unwrap();
    let header: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-header-reference.json"
    ))
    .unwrap();
    let paths: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    for collected in [false, true] {
        let entry = if collected {
            "collected := optimal-first-block(input<[u8]>)\nbegin := collected.block-begin\nblock-length := collected.block-end - collected.block-begin\nfrequencies := collected.frequencies\ncache := collected.cache\ncache-end := collected.cache-end\n"
        } else {
            "block-length := length<u64>\n"
        };
        let models = "defaults := optimal-choose-default-costs(input<[u8]>, begin<u64>, block-length, frequencies<[u32]>, 300u64, table0<[u8]>, table1<[u8]>, table2<[u8]>, symbol-costs<[u8]>)\ninitial := optimal-set-default-costs(defaults.lit-cost, defaults.len-sym-cost, literal0, length0, offset0, length-slots<[u8]>, extra-lengths<[u8]>, extra-offsets<[u8]>)\nsaved-literal := [0u32 | i <- 0u64..=256u64, i < 256u64]\nsaved-length := [0u32 | i <- 0u64..=259u64, i < 259u64]\nsaved-offset := [0u32 | i <- 0u64..=32u64, i < 32u64]\noptimal-optimize-loop(block-length, cache<[u32]>, cache-end<u64>, nodes0, initial.literal, initial.length, initial.offset-slot, saved-literal, saved-length, saved-offset, 18446744073709551615u64, passes<u64>, length-slots, extra-lengths, extra-offsets, slots<[u8]>, permutation<[u8]>, extra-precode<[u8]>, reversed)\n";
        let source = format!("{COMPONENTS}{SETUP}{entry}{models}");
        for (form, artifact) in ["original", "decoded"]
            .into_iter()
            .zip(artifacts(&source).0)
        {
            let jit = JitProgramArtifact::compile_with_limits(
                &artifact,
                &mech_stdlib::source_catalog(),
                NativeCompileLimits {
                    max_nodes: 16_384,
                    ..Default::default()
                },
            )
            .unwrap();
            println!(
                "optimization native blocks={} collected={collected}",
                jit.descriptor().plan().blocks().len()
            );
            for case in fixture["cases"].as_array().unwrap().iter().filter(|c| {
                !collected || (c["length"].as_u64() == Some(8) && c["passes"].as_u64() == Some(10))
            }) {
                let memory = ManagedMemoryBudget::new(1024 * 1024 * 1024);
                let mut named = vec![
                    ("input", matrix(&case["input"], 8)),
                    ("passes", Data::U64(case["passes"].as_u64().unwrap())),
                    (
                        "permutation",
                        matrix(&header["tables"]["precode_lens_permutation"], 8),
                    ),
                    (
                        "extra-precode",
                        matrix(&header["tables"]["extra_precode_bits"], 8),
                    ),
                    (
                        "length-slots",
                        matrix(&tables["tables"]["length_slot_tab"], 8),
                    ),
                    (
                        "extra-lengths",
                        matrix(&tables["tables"]["extra_length_bits"], 8),
                    ),
                    (
                        "extra-offsets",
                        matrix(&tables["tables"]["extra_offset_bits"], 8),
                    ),
                    ("slots", matrix(&paths["offset_slots"], 8)),
                    (
                        "table0",
                        matrix(&defaults["tables"]["default_lit_cost_0"], 8),
                    ),
                    (
                        "table1",
                        matrix(&defaults["tables"]["default_lit_cost_1"], 8),
                    ),
                    (
                        "table2",
                        matrix(&defaults["tables"]["default_lit_cost_2"], 8),
                    ),
                    (
                        "symbol-costs",
                        matrix(&defaults["tables"]["default_len_sym_costs"], 8),
                    ),
                ];
                if !collected {
                    let mut cache = vec![Data::U32(0); 1500513];
                    for pair in case["cache"].as_array().unwrap() {
                        cache[pair[0].as_u64().unwrap() as usize] =
                            Data::U32(pair[1].as_u64().unwrap() as u32);
                    }
                    named.extend([
                        ("begin", Data::U64(case["begin"].as_u64().unwrap())),
                        ("length", Data::U64(case["length"].as_u64().unwrap())),
                        ("frequencies", matrix(&case["frequencies"], 32)),
                        ("cache", Data::Matrix(cache.into())),
                        ("cache-end", Data::U64(case["cache_end"].as_u64().unwrap())),
                    ]);
                }
                let args = inputs(&artifact, &named, &memory);
                drop(named);
                let aliases = args.clone();
                let mut output = vec![None; artifact.outputs().len()];
                let mut policy = limits();
                policy.execution.max_transport_bytes = 2 * 1024 * 1024;
                policy.execution.max_work = 1_000_000_000_000;
                policy.library.max_compute_work = Some(1_000_000_000_000);
                policy.library.max_comparison_work = Some(1_000_000_000_000);
                policy.library.max_output_elements = Some(1500513);
                let start = std::time::Instant::now();
                let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                    jit.evaluate_into(&args, &mut output, policy, &memory)
                });
                println!(
                    "{}",
                    serde_json::json!({"witness":"optimization-loop", "artifact":form,"collected":collected,"case":case["name"],
                    "completed_positions":if result.is_ok() {case["length"].as_u64().unwrap()} else {0}, "elapsed_seconds":start.elapsed().as_secs_f64(),
                    "allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
                );
                result.unwrap();
                check(output.last().unwrap().as_ref().unwrap(), case);
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
