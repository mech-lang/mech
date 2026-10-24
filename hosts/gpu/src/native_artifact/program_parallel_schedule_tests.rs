//! Real emitted pure bodies with unequal text elements and source-ordered errors.
use super::*;

#[test]
fn size_ordered_string_jobs_keep_unicode_outputs_in_source_order() {
    let original = tests::artifact_source(
        r#"
+> text
items := ["x" "elephant" "é" "longer"]
text/join([item + item | item <- items], "|")
"#,
    );
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(
        &mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    for artifact in [original, decoded] {
        let catalog = mech_stdlib::source_catalog();
        let jit = crate::native_artifact::JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(jit.descriptor().parallel.regions.len(), 1);
        let image = jit.descriptor().encode_build_owned_image().unwrap();
        // SAFETY: metadata and generated entry come from the same checked build.
        let restored =
            unsafe { NativeProgramDescriptor::decode_build_owned_image(&image, &catalog) }.unwrap();
        for descriptor in [jit.descriptor(), &restored] {
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let mut snapshots = Vec::new();
            for workers in [1, 8] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let report = observed_with_nodes(
                    descriptor,
                    jit.test_checked_entry(),
                    &[],
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
                        parallel::TEST_DISPATCH_ORDER.with(|order| order.borrow().clone()),
                        [(0, 1), (0, 3), (0, 2), (0, 0)]
                    );
                }
                let output = outputs.last().unwrap().as_ref().unwrap();
                assert!(matches!(output.data(), ValueData::String(text)
                    if text.as_ref() == "xx|elephantelephant|éé|longerlonger"));
                snapshots.push(output.canonical_snapshot_bytes(artifact.schemas()).unwrap());
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            assert_eq!(snapshots[0], snapshots[1]);
        }
    }
}

#[test]
fn size_ordered_later_failure_never_precedes_an_earlier_source_push() {
    let original = tests::artifact_source(
        r#"
+> text
+> number
+> sequence
render(item<string>) => (out<string>) :=
  out := number/format(8u64 / (8u64 - sequence/length(item)), 0u64).
items := ["z" "enormous"]
text/join([render(item) | item <- items], "|")
"#,
    );
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
            let mut errors = Vec::new();
            for workers in [1, 8] {
                let mut outputs = vec![None; artifact.outputs().len()];
                let error = observed_with_nodes(
                    jit.descriptor(),
                    jit.test_checked_entry(),
                    &[],
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
                        parallel::TEST_DISPATCH_ORDER.with(|order| order.borrow().clone()),
                        [(0, 1), (0, 0)]
                    );
                }
                assert!(outputs.iter().all(Option::is_none));
                errors.push(format!("{error:?}"));
                drop(outputs);
                assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
            }
            assert_eq!(errors[0], errors[1]);
            assert!(errors[0].contains(if fail_push {
                "OutputBytes"
            } else {
                "Arithmetic"
            }));
        }
    }
}
