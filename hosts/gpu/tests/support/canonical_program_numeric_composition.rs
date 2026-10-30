//! Native parent segments must compose with current child results and recovery.
use super::{
    JitProgramArtifact, ManagedMemoryBudget, artifacts, emit_program_object, equal, inputs, limits,
    reference_result,
};

#[test]
fn child_results_keep_current_arrays_indices_publication_and_recovery() {
    use mech_core::ValueDataDraft as Data;
    let source = include_str!("../fixtures/native/numeric-parent-child.mec");
    for (form, artifact) in ["original", "decoded"].into_iter().zip(artifacts(source).0) {
        let jit = JitProgramArtifact::compile(&artifact, &mech_stdlib::source_catalog()).unwrap();
        let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
        let mut output = vec![None; artifact.outputs().len()];
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
            let prior = output.clone();
            let retained = (memory.used_bytes(), memory.retained_snapshot_nodes());
            let native = jit.evaluate_into(&args, &mut output, limits(), &memory);
            let resident = reference_result(&artifact, &args);
            let error = if let Some(expected) = expected {
                native.unwrap();
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
                Some(error)
            };
            assert!(
                args.iter()
                    .zip(&aliases)
                    .all(|(a, b)| a.shares_frozen_storage(b))
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
