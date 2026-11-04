//! Exact source continuation after native child fuel runs short.
use super::*;

fn source() -> String {
    super::mutable_children::SOURCE.replacen(
        "  more := at < end\n",
        "  more := values[at + 1u64] != 0u32\n",
        1,
    )
}

#[test]
fn child_entry_and_header_fuel_misses_preserve_publication_current_state_and_retry() {
    for artifact in artifacts(&source()) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        for (prime, header, restored) in [
            (false, true, false),
            (false, true, true),
            (true, false, false),
            (true, false, true),
        ] {
            let memory = mode.memory_budget();
            let mut values = (1..16).map(Data::U32).collect::<Vec<_>>();
            values.push(Data::U32(0));
            let args = inputs(
                &artifact,
                &[
                    ("values", Data::Matrix(values.into())),
                    ("rounds", Data::U64(64)),
                    ("amount", Data::U32(1)),
                    ("prime", Data::Bool(prime)),
                    ("reject", Data::U64(u64::MAX)),
                ],
                &memory,
            );
            let aliases = args.clone();
            let before = contents(&args);
            let mut published = vec![None; artifact.outputs().len()];
            let run = |published: &mut [Option<Value>], policy| {
                if restored {
                    observe::run_restored(&jit, &args, published, policy, &memory)
                } else {
                    observe::run(&jit, &args, published, policy, &memory)
                }
            };
            let (result, counts) = run(&mut published, mode.execution());
            result.unwrap();
            assert_eq!(counts.phases, 1);
            let expected = published
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap();
            let mut words = (65..80).map(Data::U32).collect::<Vec<_>>();
            if prime {
                words[0] = Data::U32(129);
            }
            words.push(Data::U32(0));
            assert_eq!(expected, Data::Matrix(words.into()));
            let first_change_spent = u64::MAX - counts.phase_write_fuel[0];
            let region = jit
                .descriptor()
                .metadata()
                .closed_tails
                .regions
                .iter()
                .find(|region| region.tail.parent().is_none() && region.numeric.is_some())
                .unwrap();
            let child = region
                .numeric
                .as_ref()
                .unwrap()
                .phase
                .as_ref()
                .unwrap()
                .loops
                .iter()
                .find(|loop_| loop_.first_visit.is_some())
                .unwrap();
            let mut policy = mode.execution();
            policy.execution.max_work = first_change_spent
                + if header {
                    3 * child.compute
                } else {
                    child.compute / 2
                };
            let prior = published.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let (result, counts) = run(&mut published, policy);
            assert!(
                !counts.phase_write_fuel.is_empty(),
                "must fail after an actual private write: {counts:?}"
            );
            assert!(
                counts
                    .child_fuel_exits
                    .iter()
                    .any(|(_, actual)| *actual == header),
                "must take the required child-entry/header fuel exit: {counts:?}"
            );
            let error = format!("{:?}", result.unwrap_err());
            assert!(!error.contains("InvalidProgram"), "{error}");
            assert!(
                error.contains("Work"),
                "expected the finite work denial: {error}"
            );
            for (actual, old) in published.iter().zip(&prior) {
                assert!(
                    actual
                        .as_ref()
                        .unwrap()
                        .shares_frozen_storage(old.as_ref().unwrap())
                );
            }
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                retained
            );
            assert_eq!(contents(&args), before);
            let (result, _) = run(&mut published, mode.execution());
            result.unwrap();
            assert_eq!(
                published
                    .last()
                    .unwrap()
                    .as_ref()
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected
            );
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(value, alias)| value.shares_frozen_storage(alias))
            );
            assert_eq!(contents(&args), before);
            drop((published, prior, args, aliases));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}

#[test]
fn child_fuel_images_reconstruct_funding_homes_and_exact_exit_scope() {
    for artifact in artifacts(&source()) {
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            NativeProgramLimitMode::NoLimits.compilation(),
        )
        .unwrap();
        let image: serde_json::Value =
            serde_json::from_slice(&jit.descriptor().encode_build_owned_image().unwrap()).unwrap();
        let regions = image
            .pointer("/descriptor/closed_tails/regions")
            .unwrap()
            .as_array()
            .unwrap();
        let index = regions
            .iter()
            .position(|region| {
                region.pointer("/numeric/phase/checked_children") == Some(&serde_json::json!(true))
            })
            .unwrap();
        let sites = regions[index]
            .pointer("/numeric/sites")
            .unwrap()
            .as_array()
            .unwrap();
        assert!(sites.len() > 2);
        assert!(
            sites
                .windows(2)
                .all(|pair| pair[0].as_u64() < pair[1].as_u64())
        );
        assert_eq!(
            regions[index].pointer("/numeric/prepaid_sites"),
            Some(&serde_json::json!([]))
        );
        let mut reordered = sites.clone();
        reordered.swap(0, 1);
        let mut duplicate = sites.clone();
        duplicate.insert(1, sites[0].clone());
        for (suffix, replacement) in [
            ("sites", serde_json::json!(reordered)),
            ("sites", serde_json::json!(duplicate)),
            ("sites", serde_json::json!(&sites[1..])),
            ("phase/checked_children", serde_json::json!(false)),
            ("phase/loops/1/first_visit", serde_json::Value::Null),
            ("phase/loops/1/compute", serde_json::json!(0)),
            ("exits/0/kind", serde_json::json!("Unsupported")),
            ("exits/0/to", serde_json::json!(0)),
        ] {
            let path = format!("/descriptor/closed_tails/regions/{index}/numeric/{suffix}");
            let mut bad = image.clone();
            let current = bad.pointer_mut(&path).unwrap();
            assert_ne!(*current, replacement);
            *current = replacement;
            assert!(
                super::super::tests::rejects_malformed_helper_image(
                    &serde_json::to_vec(&bad).unwrap()
                ),
                "{path}"
            );
        }
    }
}

#[test]
fn actual_header_runs_keep_real_fixed_size_buffers_native() {
    let source = concat!(
        include_str!("../../../../../tests/fixtures/native/deflate-huffman-level12.mec"),
        "\n",
        include_str!("../../../../../tests/fixtures/native/deflate-header.mec"),
        "\ncomputed := block-compute-precode-items(lens<[u8]>, number<u64>, frequencies<[u32]>, items<[u32]>)\n(computed.freqs, computed.items, computed.num-items)\n"
    );
    let fixture: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../tests/fixtures/native/deflate-header-reference.json"
    ))
    .unwrap();
    let matrix = |values: &serde_json::Value, width| {
        Data::Matrix(
            values
                .as_array()
                .unwrap()
                .iter()
                .map(|value| {
                    if width == 8 {
                        Data::U8(value.as_u64().unwrap() as u8)
                    } else {
                        Data::U32(value.as_u64().unwrap() as u32)
                    }
                })
                .collect(),
        )
    };
    for artifact in artifacts(source) {
        let mode = NativeProgramLimitMode::NoLimits;
        let jit = JitProgramArtifact::compile_with_limits(
            &artifact,
            &mech_stdlib::source_catalog(),
            mode.compilation(),
        )
        .unwrap();
        let root = jit
            .descriptor()
            .metadata()
            .closed_tails
            .regions
            .iter()
            .find(|region| {
                region.tail.parent().is_none()
                    && region.numeric.as_ref().is_some_and(|proof| {
                        proof
                            .phase
                            .as_ref()
                            .is_some_and(|phase| phase.checked_children)
                    })
            })
            .unwrap_or_else(|| {
                let metadata=jit.descriptor().metadata();
                let reasons=metadata.plan.iterations().iter().filter_map(|loop_| {
                    let mech_engine::native_artifact::NativeControlLoop::Tail(tail)=loop_ else { return None; };
                    tail.parent().is_none().then(|| crate::native_artifact::program::prepare::readonly_numeric::derive_fueled(
                        &metadata.plan,tail,&metadata.buffer_frames,&metadata.scalar_aggregates,&metadata.calls,&metadata.visits
                    ).map(|_|()).err())
                }).collect::<Vec<_>>();
                panic!("actual header root must use checked native children: {reasons:?}");
            });
        assert!(root.numeric.as_ref().unwrap().arrays.len() >= 3);
        for case in fixture["precode_cases"].as_array().unwrap() {
            let memory = mode.memory_budget();
            let args = inputs(
                &artifact,
                &[
                    ("lens", matrix(&case["lens"], 8)),
                    ("number", Data::U64(case["num_lens"].as_u64().unwrap())),
                    ("frequencies", matrix(&case["freqs"], 32)),
                    ("items", matrix(&case["items"], 32)),
                ],
                &memory,
            );
            let before = contents(&args);
            let mut out = vec![None; artifact.outputs().len()];
            let (result, counts) =
                observe::run_restored(&jit, &args, &mut out, mode.execution(), &memory);
            result.unwrap();
            assert_eq!(counts.phases, 1, "{}", case["name"]);
            let Data::Tuple(fields) = out
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap()
            else {
                panic!("header fields")
            };
            for (index, name) in ["freqs", "items"].into_iter().enumerate() {
                assert_eq!(
                    fields[index],
                    matrix(&case["expected"][name], 32),
                    "{} {name}",
                    case["name"]
                );
            }
            assert_eq!(
                fields[2],
                Data::U64(case["expected"]["num_items"].as_u64().unwrap())
            );
            assert_eq!(contents(&args), before);
            drop((args, out));
            assert_eq!(
                (memory.used_bytes(), memory.retained_snapshot_nodes()),
                (0, 0)
            );
        }
    }
}
