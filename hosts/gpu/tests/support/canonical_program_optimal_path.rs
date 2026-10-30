//! The complete backward cost search, against executed pinned reference cases.
use super::{allocation_probe, artifacts, inputs, limits};
use mech_core::{ManagedMemoryBudget, ValueData, ValueDataDraft as Data};
use mech_gpu::native_artifact::JitProgramArtifact;

const SOURCE: &str = concat!(
    include_str!("../fixtures/native/deflate-optimal-path.mec"),
    "\npath := optimal-find-min-cost-path(block-length<u64>, cache<[u32]>, cache-end<u64>, sequence/with-capacity(0u64, capacity<u64>), literal-costs<[u32]>, length-costs<[u32]>, offset-costs<[u32]>, slots<[u8]>)\npath.nodes\n"
);

fn case_inputs(
    artifact: &mech_engine::ProgramArtifact,
    fixture: &serde_json::Value,
    case: &serde_json::Value,
    memory: &ManagedMemoryBudget,
) -> Vec<mech_core::Value> {
    let slots = fixture["offset_slots"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| Data::U8(v.as_u64().unwrap() as u8))
        .collect::<Vec<_>>();
    assert_eq!(slots.len(), 256);
    let mut cache = vec![Data::U32(0); case["cache_size"].as_u64().unwrap() as usize];
    for entry in case["cache"].as_array().unwrap() {
        cache[entry[0].as_u64().unwrap() as usize] = Data::U32(entry[1].as_u64().unwrap() as u32);
    }
    let words = |name: &str| {
        Data::Matrix(
            case[name]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| Data::U32(v.as_u64().unwrap() as u32))
                .collect(),
        )
    };
    inputs(
        artifact,
        &[
            (
                "block-length",
                Data::U64(case["block_length"].as_u64().unwrap()),
            ),
            ("cache", Data::Matrix(cache.into())),
            ("cache-end", Data::U64(case["cache_end"].as_u64().unwrap())),
            ("capacity", Data::U64(305257)),
            ("literal-costs", words("literal_costs")),
            ("length-costs", words("length_costs")),
            ("offset-costs", words("offset_costs")),
            ("slots", Data::Matrix(slots.into())),
        ],
        memory,
    )
}

fn policy() -> mech_gpu::native_artifact::NativeProgramExecutionLimits {
    let mut policy = limits();
    // Explicit finite component diagnostic; no default-policy claim.
    policy.execution.max_work = 1_000_000_000_000;
    policy.library.max_compute_work = Some(1_000_000_000_000);
    policy.library.max_comparison_work = Some(1_000_000_000_000);
    policy
}

#[test]
fn path_scan_emits_native_pieces_around_its_nested_scalar_child() {
    for artifact in artifacts(SOURCE).0 {
        let object = super::emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let metadata = object.metadata();
        let parents = metadata["numeric_parent_partitions"]["parents"]
            .as_array()
            .unwrap();
        let segments = metadata["compiled_numeric_segments"].as_array().unwrap();
        let selected = parents
            .iter()
            .filter(|parent| parent["scalar_child"] == true)
            .filter(|parent| {
                segments
                    .iter()
                    .any(|segment| segment["iteration"] == parent["iteration"])
            })
            .collect::<Vec<_>>();
        assert_eq!(
            selected.len(),
            1,
            "the unchanged path scan must emit native pieces"
        );
        let parent = selected[0];
        assert!(parent["returned_arrays"].as_array().unwrap().is_empty());
        let child = parent["child"].as_u64().unwrap() as u32;
        assert!(
            object
                .descriptor()
                .plan()
                .iterations()
                .iter()
                .any(|iteration| iteration.as_ref().parent() == Some(child)),
            "the actual match-choice child retains its nested length search"
        );
        for segment in segments
            .iter()
            .filter(|segment| segment["iteration"] == parent["iteration"])
        {
            assert_eq!(segment["successful_interior_helpers"], 0);
            assert!(
                segment["source_sites"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .all(|site| {
                        site != &parent["child_initialize"] && site != &parent["child_exit"]
                    })
            );
        }
    }
}

#[test]
fn path_match_search_groups_existing_private_scalar_aggregate_steps() {
    for artifact in artifacts(SOURCE).0 {
        let object = super::emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&object.descriptor().encode_build_owned_image().unwrap())
                .unwrap();
        let aggregates = object.descriptor().plan().scalar_aggregates();
        let metadata = object.metadata();
        let parent = metadata["numeric_parent_partitions"]["parents"]
            .as_array()
            .unwrap()
            .iter()
            .find(|parent| parent["scalar_child"] == true)
            .unwrap();
        let child = parent["child"].as_u64().unwrap() as u32;
        assert!(
            image["descriptor"]["map_admission"]["groups"]
                .as_array()
                .unwrap()
                .iter()
                .any(|group| group["owner"].is_null()
                    && group["sites"].as_array().unwrap().iter().any(|site| {
                        let site = site.as_u64().unwrap() as usize;
                        aggregates.action(site).is_some()
                            && object.descriptor().plan().blocks()[site].iteration() == Some(child)
                    })),
            "the unchanged match-choice search must group its existing scalar aggregate steps"
        );
        let groups = image["descriptor"]["map_admission"]["groups"]
            .as_array()
            .unwrap();
        for segment in metadata["compiled_numeric_segments"].as_array().unwrap() {
            let sites = segment["source_sites"].as_array().unwrap();
            for group in groups {
                let members = group["sites"].as_array().unwrap();
                assert!(
                    !members.iter().any(|site| sites.contains(site)),
                    "a numeric piece and ordinary admission group must have separate entry authority"
                );
            }
        }
    }
}

#[test]
fn pinned_min_cost_paths_keep_exact_nodes_and_real_reservation() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(SOURCE).0) {
        let compile_start = std::time::Instant::now();
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        println!(
            "path compile form={form} seconds={}",
            compile_start.elapsed().as_secs_f64()
        );
        for case in fixture["cases"].as_array().unwrap() {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = case_inputs(&artifact, &fixture, case, &memory);
            let mut output = vec![None; artifact.outputs().len()];
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, policy(), &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"pinned-min-cost-path","artifact":form,"case":case["name"],"block_length":case["block_length"],"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            let value = output.last().unwrap().as_ref().unwrap();
            let expected = Data::Matrix(
                case["nodes"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|v| Data::U64(v.as_u64().unwrap()))
                    .collect(),
            );
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                expected,
                "{}",
                case["name"]
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(305257));
            drop((args, output));
            assert_eq!(memory.used_bytes(), 0);
            assert_eq!(memory.retained_snapshot_nodes(), 0);
        }
    }
}

#[test]
fn path_checks_preserve_entry_order_publication_aliases_and_recovery() {
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    let valid = fixture["cases"][1].clone();
    let mut short = valid.clone();
    short["length_costs"].as_array_mut().unwrap().pop();
    let invalid = |length: u32| {
        let mut case = valid.clone();
        case["block_length"] = 1.into();
        case["cache_end"] = 2.into();
        case["cache"] = serde_json::json!([[0, 65536u32 | length], [1, 6356993]]);
        case
    };
    let too_short = invalid(2);
    let too_long = invalid(259);
    let mut both = too_short.clone();
    both["length_costs"] = short["length_costs"].clone();
    for artifact in artifacts(SOURCE).0 {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
        let mut entry_error = None;
        for (case, reject) in [
            (&valid, false),
            (&short, true),
            (&too_short, true),
            (&too_long, true),
            (&both, true),
            (&valid, false),
        ] {
            let args = case_inputs(&artifact, &fixture, case, &memory);
            let aliases = args.clone();
            let old = output.clone();
            let retained = memory.used_bytes();
            let nodes = memory.retained_snapshot_nodes();
            let result = jit.evaluate_into(&args, &mut output, policy(), &memory);
            if reject {
                let error = format!("{:?}", result.unwrap_err());
                assert!(error.contains("Integrity"), "{error}");
                if std::ptr::eq(case, &short) {
                    entry_error = Some(error.clone());
                }
                if std::ptr::eq(case, &both) {
                    assert_eq!(Some(error.clone()), entry_error);
                }
                if std::ptr::eq(case, &too_short) || std::ptr::eq(case, &too_long) {
                    assert_ne!(Some(error), entry_error);
                }
                assert!(old.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(memory.used_bytes(), retained);
                assert_eq!(memory.retained_snapshot_nodes(), nodes);
            } else {
                result.unwrap();
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
fn one_compiled_program_collects_its_own_matches_and_selects_the_reference_path() {
    use mech_gpu::native_artifact::NativeCompileLimits;
    let source = concat!(
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
        "collected := optimal-first-block(input<[u8]>)\npath := optimal-find-min-cost-path(collected.block-end - collected.block-begin, collected.cache, collected.cache-end, sequence/with-capacity(0u64, 305257u64), literal-costs<[u32]>, length-costs<[u32]>, offset-costs<[u32]>, slots<[u8]>)\npath.nodes\n"
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-optimal-path-reference.json"
    ))
    .unwrap();
    let blocks: serde_json::Value = serde_json::from_str(include_str!(
        "../fixtures/native/deflate-block-reference.json"
    ))
    .unwrap();
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let compile_start = std::time::Instant::now();
        // The same explicit finite compilation profile as the existing full
        // block completion diagnostic. No compiler default is changed.
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
            "collector + path compile form={form} blocks={} seconds={}",
            jit.descriptor().plan().blocks().len(),
            compile_start.elapsed().as_secs_f64()
        );
        for name in ["eight-literals", "eight-overlap"] {
            let case = fixture["cases"]
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
            let words = |name: &str| {
                Data::Matrix(
                    case[name]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| Data::U32(v.as_u64().unwrap() as u32))
                        .collect(),
                )
            };
            let args = inputs(
                &artifact,
                &[
                    (
                        "input",
                        Data::Matrix(
                            block["input"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| Data::U8(v.as_u64().unwrap() as u8))
                                .collect(),
                        ),
                    ),
                    ("literal-costs", words("literal_costs")),
                    ("length-costs", words("length_costs")),
                    ("offset-costs", words("offset_costs")),
                    (
                        "slots",
                        Data::Matrix(
                            fixture["offset_slots"]
                                .as_array()
                                .unwrap()
                                .iter()
                                .map(|v| Data::U8(v.as_u64().unwrap() as u8))
                                .collect(),
                        ),
                    ),
                ],
                &memory,
            );
            let aliases = args.clone();
            let mut output = vec![None; artifact.outputs().len()];
            let mut limits = policy();
            limits.library.max_output_elements = Some(1_500_513);
            let start = std::time::Instant::now();
            let (result, requests, bytes) = allocation_probe::measured_with_bytes(|| {
                jit.evaluate_into(&args, &mut output, limits, &memory)
            });
            println!(
                "{}",
                serde_json::json!({"witness":"connected-collector-and-path","artifact":form,"case":name,"input_bytes":8,"tree_entries":262144,"cache_words":1500513,"nodes_reservation":305257,"depth":300,"nice":258,"elapsed_seconds":start.elapsed().as_secs_f64(),"allocation_requests":requests,"requested_bytes":bytes,"result":format!("{result:?}")})
            );
            result.unwrap();
            let value = output.last().unwrap().as_ref().unwrap();
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                Data::Matrix(
                    case["nodes"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .map(|v| Data::U64(v.as_u64().unwrap()))
                        .collect()
                )
            );
            let ValueData::Matrix(matrix) = value.data() else {
                panic!()
            };
            assert_eq!(matrix.unsigned_buffer_capacity(), Some(305257));
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
