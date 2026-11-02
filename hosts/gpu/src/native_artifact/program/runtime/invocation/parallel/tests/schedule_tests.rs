//! Real emitted pure bodies with unequal text elements and source-ordered errors.
use crate::native_artifact::program::runtime::invocation::parallel::tests::*;

#[test]
fn size_ordered_string_jobs_keep_unicode_outputs_in_source_order() {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source(
        "number/format(h{\"a\"}, 0u64)",
        r#"
items := ["x" "elephant" "é" "longer"]
text/join([count-text(text, width, sequence/length(item)) | item <- items], "|")
"#,
    ));
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(jit.descriptor().metadata().parallel.regions.len(), 1);
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: metadata and generated entry come from the same checked build.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = [input(&artifact, &memory)];
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let mut snapshots = Vec::new();
            for workers in [1, 8] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let report = observed_with_nodes(
                    descriptor,
                    jit.test_checked_entry(),
                    &args,
                    &mut outputs,
                    &memory,
                    workers,
                    false,
                    1_048_576,
                )
                .unwrap();
                if workers == 8 {
                    assert_eq!(report.parallel_tasks, 4);
                    assert_eq!(
                        crate::native_artifact::program::runtime::invocation::parallel::TEST_DISPATCH_ORDER
                            .with(|order| order.borrow().clone()),
                        [(0, 1), (0, 3), (0, 2), (0, 0)]
                    );
                }
                let output = outputs.last().unwrap().as_ref().unwrap();
                assert!(matches!(output.data(), ValueData::String(text)
                    if text.as_ref() == "1|3|1|2"));
                snapshots.push(output.canonical_snapshot_bytes(artifact.schemas()).unwrap());
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
            assert_eq!(snapshots[0], snapshots[1]);
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}

#[test]
fn size_ordered_later_failure_never_precedes_an_earlier_source_push() {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source(
        "number/format(h{\"a\"} + 8u64 / (8u64 - n), 0u64)",
        r#"
items := ["z" "enormous"]
text/join([count-text(text, width, sequence/length(item)) | item <- items], "|")
"#,
    ));
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let jit = crate::native_artifact::JitProgramArtifact::compile(
            &artifact,
            &mech_stdlib::source_catalog(),
        )
        .unwrap();
        for fail_push in [false, true] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let args = [input(&artifact, &memory)];
            let baseline = (memory.used_bytes(), memory.retained_nodes());
            let mut errors = Vec::new();
            for workers in [1, 8] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let error = observed_with_nodes(
                    jit.descriptor(),
                    jit.test_checked_entry(),
                    &args,
                    &mut outputs,
                    &memory,
                    workers,
                    fail_push,
                    1_048_576,
                )
                .unwrap_err();
                if workers == 8 {
                    assert_eq!(BATCHES.with(Cell::get), 1);
                    assert_eq!(
                        crate::native_artifact::program::runtime::invocation::parallel::TEST_DISPATCH_ORDER
                            .with(|order| order.borrow().clone()),
                        [(0, 1), (0, 0)]
                    );
                }
                assert!(outputs.iter().all(Option::is_none));
                errors.push(format!("{error:?}"));
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
            }
            assert_eq!(errors[0], errors[1]);
            assert!(errors[0].contains(if fail_push {
                "OutputBytes"
            } else {
                "Arithmetic"
            }));
            drop(args);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
        }
    }
}
