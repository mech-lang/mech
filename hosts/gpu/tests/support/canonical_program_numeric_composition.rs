//! Native parent segments must compose with current child results and recovery.
use super::{
    JitProgramArtifact, ManagedMemoryBudget, artifacts, emit_program_object, equal, inputs, limits,
    reference_result,
};
use mech_core::ValueDataDraft;

#[path = "canonical_program_composition_cases.rs"]
mod cases;

#[test]
fn child_results_keep_current_arrays_indices_publication_and_recovery() {
    use mech_core::ValueDataDraft as Data;
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output: Vec<Option<mech_core::Value>> = vec![None; artifact.outputs().len()];
        // Multiple nonempty child calls expose stale returned arrays or indices;
        // zero widths/counts expose accidentally executed child paths. The short
        // output rejects after one valid write, then the same JIT/account retry.
        for (name, symbols, lens, counts, width, start, expected) in [
            (
                "three-widths",
                vec![2, 0, 3, 1],
                vec![9, 9, 9, 9],
                vec![0, 1, 1, 2],
                3,
                0,
                Some(vec![3, 1, 3, 2]),
            ),
            (
                "changed-start-and-counts",
                vec![2, 0, 3, 1],
                vec![9, 9, 9, 9],
                vec![0, 1, 2],
                2,
                1,
                Some(vec![2, 1, 9, 2]),
            ),
            (
                "zero-width",
                vec![],
                vec![9, 8, 7],
                vec![],
                0,
                0,
                Some(vec![9, 8, 7]),
            ),
            (
                "zero-child-counts",
                vec![],
                vec![9, 8, 7],
                vec![0, 0, 0, 0],
                3,
                0,
                Some(vec![9, 8, 7]),
            ),
            (
                "reject-after-first-write",
                vec![0, 3, 2, 1],
                vec![9, 9],
                vec![0, 1, 1, 2],
                3,
                0,
                None,
            ),
            (
                "reject-in-later-child",
                vec![0, 3, 2, 1],
                vec![9, 9],
                vec![0, 1, 1, 1],
                3,
                0,
                None,
            ),
            (
                "retry",
                vec![2, 0, 3, 1],
                vec![9, 9, 9, 9],
                vec![0, 1, 1, 2],
                3,
                0,
                Some(vec![3, 1, 3, 2]),
            ),
        ] {
            let named = [
                (
                    "a",
                    Data::Matrix(symbols.into_iter().map(Data::U32).collect()),
                ),
                (
                    "lens",
                    Data::Matrix(lens.into_iter().map(Data::U8).collect()),
                ),
                (
                    "counts",
                    Data::Matrix(counts.into_iter().map(Data::U64).collect()),
                ),
                ("width", Data::U64(width)),
                ("start", Data::U64(start)),
            ];
            let args = inputs(&artifact, &named, &memory);
            let aliases = args.clone();
            let original = args
                .iter()
                .map(|v| v.canonical_data_draft().unwrap())
                .collect::<Vec<_>>();
            let prior = output.clone();
            let published = prior
                .iter()
                .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                .collect::<Vec<_>>();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            let error = if let Some(expected) = expected {
                let report = native.unwrap();
                println!(
                    "{}",
                    serde_json::json!({"witness":"parent-runtime-visits","form":form,"case":name,"visits":report.visits,"work":report.work})
                );
                equal(&artifact, &output, &resident.unwrap());
                assert_eq!(
                    output
                        .last()
                        .unwrap()
                        .as_ref()
                        .unwrap()
                        .canonical_data_draft()
                        .unwrap(),
                    Data::Matrix(expected.into_iter().map(Data::U8).collect()),
                    "{name}"
                );
                None
            } else {
                let error = format!("{:?}", native.unwrap_err());
                assert!(error.contains("IndexOutOfRange"), "{error}");
                assert!(resident.unwrap_err().contains("IndexOutOfRange"));
                assert!(prior.iter().zip(&output).all(|(a, b)| {
                    a.as_ref()
                        .unwrap()
                        .shares_frozen_storage(b.as_ref().unwrap())
                }));
                assert_eq!(
                    (memory.used_bytes(), memory.retained_snapshot_nodes()),
                    retained
                );
                assert_eq!(
                    output
                        .iter()
                        .map(|v| v.as_ref().map(|v| v.canonical_data_draft().unwrap()))
                        .collect::<Vec<_>>(),
                    published
                );
                Some(error)
            };
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
            );
            assert_eq!(
                args.iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                original
            );
            assert_eq!(
                aliases
                    .iter()
                    .map(|v| v.canonical_data_draft().unwrap())
                    .collect::<Vec<_>>(),
                original
            );
            println!(
                "{}",
                serde_json::json!({"witness":"huffman-parent-child-semantics", "form":form, "case":name, "error":error})
            );
            drop((args, aliases, prior));
        }
        drop(output);
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn parent_segments_are_emitted_around_the_selected_native_child() {
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    for artifact in artifacts(source).0 {
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let plan = object.descriptor().plan();
        let frames = plan.facts().buffer_frames();
        println!(
            "{}",
            serde_json::json!({
                "witness":"parent-child-exact-cfg",
                "iterations":plan.iterations(),
                "blocks":plan.blocks(),
                "frames":format!("{frames:?}")
            })
        );
        let metadata = object.metadata();
        let segments = metadata["compiled_numeric_segments"].as_array();
        assert!(
            segments.is_some_and(|s| !s.is_empty()),
            "parent segments are absent; an ordinary parent plus a native child does not satisfy composition"
        );
        assert!(segments.unwrap().iter().any(|s| s["iteration"] == 0));
    }
}

#[test]
fn parent_partition_keeps_exact_child_result_mapping_and_single_entry_boundaries() {
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    let mut prior = None;
    for artifact in artifacts(source).0 {
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let metadata = object.metadata();
        let parents = metadata["numeric_parent_partitions"]["parents"]
            .as_array()
            .unwrap();
        assert_eq!(parents.len(), 1);
        let parent = &parents[0];
        assert_eq!(parent["iteration"], 0);
        assert_eq!(parent["child"], 1);
        assert_eq!(parent["returned_arrays"], serde_json::json!([[0, 1]]));
        println!(
            "{}",
            serde_json::json!({"witness":"parent-current-frame-recipes", "parent":parent})
        );
        let plan = object.descriptor().plan();
        let mech_engine::native_artifact::NativeControlLoop::Tail(tail) = &plan.iterations()[0]
        else {
            panic!()
        };
        let facts = plan.facts();
        let frames = facts.buffer_frames();
        let mech_engine::native_artifact::NativeControlLoop::Tail(child) = &plan.iterations()[1]
        else {
            panic!()
        };
        let returned_schema = plan.values()[child.result().get() as usize].schema();
        let current_lens = plan
            .blocks()
            .iter()
            .enumerate()
            .find_map(|(site, block)| {
                if block.iteration() != Some(0) {
                    return None;
                }
                match frames.action(site)?.operation() {
                    mech_engine::native_artifact::NativeBufferFrameOperation::Project {
                        source,
                        ordinal: 0,
                        output,
                    } if plan.values()[source.get() as usize].schema() == returned_schema => {
                        Some(output.get())
                    }
                    _ => None,
                }
            })
            .unwrap();
        let mut checked_current_result = false;
        let mut all = std::collections::BTreeSet::new();
        for segment in parent["segments"].as_array().unwrap() {
            let entry = segment["entry"].as_u64().unwrap() as u32;
            let sites = segment["sites"]
                .as_array()
                .unwrap()
                .iter()
                .map(|v| v.as_u64().unwrap() as u32)
                .collect::<std::collections::BTreeSet<_>>();
            assert!(sites.contains(&entry));
            if let Some(recipe) = segment["frame_plan"].as_object() {
                let expected = segment["sites"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .filter_map(|site| {
                        let at = site.as_u64().unwrap() as usize;
                        let action = frames.action(at)?;
                        (!action.retire().is_empty()).then(|| {
                            serde_json::json!([
                                at,
                                action.retire().iter().map(|v| v.get()).collect::<Vec<_>>()
                            ])
                        })
                    })
                    .collect::<Vec<_>>();
                assert_eq!(recipe["retirements"], serde_json::json!(expected));
                if let Some(next) = recipe["frames"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|f| f["value"] == tail.next().get())
                {
                    let root = next["arrays"][1].as_u64().unwrap() as usize;
                    assert_eq!(
                        recipe["sources"][root],
                        serde_json::json!({"Value":current_lens})
                    );
                    checked_current_result = true;
                }
            }
            for &site in &sites {
                assert!(all.insert(site), "overlapping native pieces");
                assert_eq!(plan.blocks()[site as usize].iteration(), Some(0));
                assert!(
                    ![
                        tail.header(),
                        tail.latch(),
                        tail.finish(),
                        parent["child_initialize"].as_u64().unwrap() as u32,
                        parent["child_exit"].as_u64().unwrap() as u32
                    ]
                    .contains(&site)
                );
                if site != entry {
                    assert!(
                        facts
                            .predecessors(site)
                            .unwrap()
                            .iter()
                            .all(|p| sites.contains(p))
                    );
                }
                for next in facts.successors(site).unwrap().into_iter().flatten() {
                    assert_eq!(
                        segment["exits"]
                            .as_array()
                            .unwrap()
                            .contains(&serde_json::json!([site, next])),
                        !sites.contains(&next),
                        "unrecorded or invented boundary"
                    );
                }
            }
        }
        assert!(all.len() > 30);
        assert!(
            checked_current_result,
            "parent next frame has no proved current child buffer"
        );
        if let Some(prior) = prior {
            assert_eq!(prior, *parent);
        }
        prior = Some(parent.clone());
    }
}

#[test]
fn parent_partition_follows_reordered_child_results() {
    let source = include_str!("../fixtures/native/numeric-parent-child.mec")
        .replace("=> <([u8],u64)>", "=> <(u64,[u8])>")
        .replace("(lens, i)", "(i, lens)")
        .replace("(next, next-i) := step", "(next-i, next) := step");
    for artifact in artifacts(&source).0 {
        let object = emit_program_object(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let metadata = object.metadata();
        let parents = metadata["numeric_parent_partitions"]["parents"]
            .as_array()
            .unwrap();
        assert_eq!(parents.len(), 1);
        assert_eq!(parents[0]["returned_arrays"], serde_json::json!([[1, 1]]));
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let args = inputs(
            &artifact,
            &[
                (
                    "a",
                    ValueDataDraft::Matrix(
                        [2, 0, 3, 1].into_iter().map(ValueDataDraft::U32).collect(),
                    ),
                ),
                (
                    "lens",
                    ValueDataDraft::Matrix(
                        [9, 9, 9, 9].into_iter().map(ValueDataDraft::U8).collect(),
                    ),
                ),
                (
                    "counts",
                    ValueDataDraft::Matrix(
                        [0, 1, 1, 2].into_iter().map(ValueDataDraft::U64).collect(),
                    ),
                ),
                ("width", ValueDataDraft::U64(3)),
                ("start", ValueDataDraft::U64(0)),
            ],
            &memory,
        );
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let mut output = vec![None; artifact.outputs().len()];
        jit.evaluate_into(&args, &mut output, limits(), &memory)
            .unwrap();
        equal(
            &artifact,
            &output,
            &reference_result(&artifact, &args).unwrap(),
        );
        assert_eq!(
            output
                .last()
                .unwrap()
                .as_ref()
                .unwrap()
                .canonical_data_draft()
                .unwrap(),
            ValueDataDraft::Matrix([3, 1, 3, 2].into_iter().map(ValueDataDraft::U8).collect())
        );
        drop((output, args));
        assert_eq!(
            (memory.used_bytes(), memory.retained_snapshot_nodes()),
            (0, 0)
        );
    }
}

#[test]
fn parent_partition_declines_multiple_children_and_append_owners() {
    let original = include_str!("../fixtures/native/numeric-parent-child.mec");
    let siblings = original.replace("  (next, next-i) := step\n",
        "  (first, first-i) := step\n  (next, next-i) := second-symbols(a, lens, width, first-i, 0u64)\n");
    let start = original.find("\nhuff-assign-symbols(").unwrap() + 1;
    let end = original.find("\nhuff-assign-lengths(").unwrap() + 1;
    let second = original[start..end].replace("huff-assign-symbols", "second-symbols");
    let siblings = siblings.replacen(
        "huff-assign-lengths(",
        &format!("{second}huff-assign-lengths("),
        1,
    );
    assert_ne!(siblings, original);
    // The child is the already qualified record-buffer append recurrence.
    let append = "+> sequence\ngrow(words<[u64]>, left<u64>) => (out<{nodes<[u64]>}>) :=\n  out := ((left > 0u64)? | true => grow(sequence/append(words, left), left - 1u64) | false => {nodes: words}).\nouter(words<[u64]>, left<u64>) => (out<[u64]>) :=\n  result := grow(words, left)\n  out := ((left > 0u64)? | true => outer(result.nodes, left - 1u64) | false => result.nodes).\nouter(sequence/with-capacity(0u64, capacity<u64>), steps<u64>)\n";
    for (source, expect_append) in [(siblings.as_str(), false), (append, true)] {
        println!("negative parent append={expect_append}");
        for artifact in artifacts(source).0 {
            let metadata = emit_program_object(&artifact, &mech_stdlib::source_catalog())
                .unwrap()
                .metadata();
            let parents = metadata["numeric_parent_partitions"]["parents"]
                .as_array()
                .unwrap();
            assert!(parents.is_empty());
            if expect_append {
                assert!(
                    metadata["numeric_write_regions"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|r| r["proof"]["arrays"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .any(|a| a["appends"] == true))
                );
            } else {
                assert_eq!(
                    metadata["readonly_numeric_candidates"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .filter(|c| c["parent"] == 0 && c["selected"] == true)
                        .count(),
                    2
                );
            }
        }
    }
}
