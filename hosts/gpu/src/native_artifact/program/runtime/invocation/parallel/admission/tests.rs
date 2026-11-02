//! Admission boundaries use real owning invocation state and generated bodies.
use crate::native_artifact::program::runtime::*;

fn source() -> String {
    format!(
        r#"{}
+> number
+> text
+> sequence
render(s<string>, n<u64>) => (out<string>) :=
  h := #Count(s, 1u64, n, empty)
  out := text/join([number/format(h{{key}}, 0u64) | key <- ["a", "b"]], ",").
text := seed<string>
text/join([render(text, n) | n <- [4u64 8u64]], "|")
"#,
        crate::native_artifact::program::runtime::structural_projection_tests::COUNT
    )
}

#[test]
fn complete_invocation_metadata_is_shared_and_denied_without_leaking_prior_state() {
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source());
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let parent = crate::native_artifact::program::runtime::session::Prepared::new(
        descriptor,
        crate::native_artifact::program::runtime::tests::limits(),
        &memory,
    )
    .unwrap();
    let baseline = (
        memory.used_bytes(),
        memory.retained_nodes(),
        memory.retained_snapshot_nodes(),
    );
    let fixed_bytes =
        crate::native_artifact::program::runtime::session::Prepared::worker_setup_bytes(descriptor)
            .unwrap();
    eprintln!(
        "WORKER_SETUP_CENSUS held_bytes={} envelope_bytes={} constant_bounds={}",
        baseline.0,
        fixed_bytes,
        descriptor.metadata().constant_bytes.iter().sum::<u64>()
    );
    assert!(
        baseline.0 <= fixed_bytes,
        "constructor ownership must fit its fixed preflight envelope"
    );
    assert_eq!(
        baseline.1 - baseline.2,
        crate::native_artifact::program::runtime::session::Prepared::worker_setup_nodes(descriptor)
            .unwrap(),
        "fixed setup helper must equal the actual private constructor claims"
    );
    assert!(
        baseline.1
            > baseline.2
                + crate::native_artifact::program::runtime::invocation::caller_geometry::fixed_nodes().unwrap()
    );
    let capped = memory.with_limits(memory.limit_bytes(), baseline.1 * 2 - 1);
    let error = crate::native_artifact::program::runtime::session::Prepared::new(
        descriptor,
        crate::native_artifact::program::runtime::tests::limits(),
        &capped,
    )
    .err()
    .expect("two independent complete metadata inventories cannot fit one node short");
    assert!(format!("{error:?}").contains("retained node admission"));
    assert_eq!(
        (
            memory.used_bytes(),
            memory.retained_nodes(),
            memory.retained_snapshot_nodes()
        ),
        baseline
    );
    let exact = memory.with_limits(memory.limit_bytes(), baseline.1 * 2);
    let child = crate::native_artifact::program::runtime::session::Prepared::new(
        descriptor,
        crate::native_artifact::program::runtime::tests::limits(),
        &exact,
    )
    .unwrap();
    assert_eq!(memory.retained_nodes(), baseline.1 * 2);
    drop(child);
    assert_eq!(
        (
            memory.used_bytes(),
            memory.retained_nodes(),
            memory.retained_snapshot_nodes()
        ),
        baseline
    );
    drop(parent);
    assert_eq!(
        (
            memory.used_bytes(),
            memory.retained_nodes(),
            memory.retained_snapshot_nodes()
        ),
        (0, 0, 0)
    );
}

#[test]
fn simultaneous_worker_constructor_preflight_checks_bytes_and_nodes_without_leaking() {
    let artifact = crate::native_artifact::program::runtime::tests::artifact_source(&source());
    let jit = crate::native_artifact::JitProgramArtifact::compile(
        &artifact,
        &mech_stdlib::source_catalog(),
    )
    .unwrap();
    let descriptor = jit.descriptor();
    assert_eq!(
        descriptor.metadata().constant_admission_bytes,
        descriptor.metadata().constant_bytes.iter().sum::<u64>()
    );
    let image = descriptor.encode_build_owned_image().unwrap();
    // SAFETY: this is the same still-owned generated build and exact image.
    let restored = unsafe {
        NativeProgramDescriptor::decode_build_owned_image(&image, &mech_stdlib::source_catalog())
    }
    .unwrap();
    assert_eq!(
        restored.metadata().constant_admission_bytes,
        descriptor.metadata().constant_admission_bytes
    );
    assert_eq!(
        restored.encode_build_owned_image().unwrap(),
        image,
        "derived constructor sums must not add an injectable serialized field"
    );
    let mut malformed: serde_json::Value = serde_json::from_slice(&image).unwrap();
    assert!(descriptor.metadata().constant_bytes.len() > 1);
    malformed["descriptor"]["constant_bytes"][0] = u64::MAX.into();
    let malformed = serde_json::to_vec(&malformed).unwrap();
    // SAFETY: decoder-only negative test; no result can execute generated code.
    assert!(
        unsafe {
            NativeProgramDescriptor::decode_build_owned_image(
                &malformed,
                &mech_stdlib::source_catalog(),
            )
        }
        .is_err()
    );
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let parent = crate::native_artifact::program::runtime::session::Prepared::new(
        descriptor,
        crate::native_artifact::program::runtime::tests::limits(),
        &memory,
    )
    .unwrap();
    let baseline = (memory.used_bytes(), memory.retained_nodes());
    let bytes = (crate::native_artifact::program::runtime::session::Prepared::worker_setup_bytes(
        descriptor,
    )
    .unwrap()
        + NativeLibraryWorkBudget::worker_account_metadata_bytes())
        * 2
        + NativeLibraryWorkBudget::worker_group_metadata_bytes();
    let nodes =
        crate::native_artifact::program::runtime::session::Prepared::worker_setup_nodes(descriptor)
            .unwrap()
            * 2;
    assert!(bytes > 0 && nodes > 0);
    for (byte_limit, node_limit, fits) in [
        (baseline.0 + bytes - 1, baseline.1 + nodes, false),
        (baseline.0 + bytes, baseline.1 + nodes - 1, false),
        (baseline.0 + bytes, baseline.1 + nodes, true),
    ] {
        let capped = memory.with_limits(byte_limit, node_limit);
        assert_eq!(
            crate::native_artifact::program::runtime::invocation::parallel::worker_setup_fits(
                descriptor, &capped, 2
            )
            .unwrap(),
            fits
        );
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
    }
    drop(parent);
    assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
}

#[test]
fn ordinary_nested_join_body_retains_parallel_node_admission() {
    let original = crate::native_artifact::program::runtime::tests::artifact_source(&source());
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
        assert!(!jit.descriptor().metadata().parallel.regions.is_empty());
        assert!(
            jit.descriptor()
                .metadata()
                .parallel
                .regions
                .iter()
                .enumerate()
                .all(|(id, _)| {
                    crate::native_artifact::program::runtime::invocation::parallel::admission::supported(
                        jit.descriptor(),
                        id,
                    )
                })
        );
        let memory =
            ManagedMemoryBudget::new(256 * 1024 * 1024).with_limits(256 * 1024 * 1024, 1 << 24);
        let value = ValueDraft {
            schema: artifact.inputs()[0].schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::String("abababab".into()),
        }
        .finalize(&SnapshotValidationContext::new(artifact.schemas()))
        .unwrap();
        let mut bytes = memory
            .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
            .unwrap();
        let args = [value.into_memory_budget(&mut bytes).unwrap()];
        drop(bytes);
        let baseline = (memory.used_bytes(), memory.retained_nodes());
        let mut snapshots = Vec::new();
        for workers in [1, 8] {
            struct Restore(Option<usize>);
            impl Drop for Restore {
                fn drop(&mut self) {
                    crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
                        .with(|value| value.set(self.0));
                }
            }
            let _restore = Restore(
                crate::native_artifact::program::runtime::invocation::parallel::TEST_WORKERS
                    .with(|value| value.replace(Some(workers))),
            );
            let mut outputs = vec![None; artifact.outputs().len()];
            let report = jit
                .evaluate_into(
                    &args,
                    &mut outputs,
                    crate::native_artifact::program::runtime::tests::limits(),
                    &memory,
                )
                .unwrap_or_else(|error| panic!("workers={workers}: {error:?}"));
            assert_eq!(report.parallel_tasks, if workers == 1 { 0 } else { 2 });
            let output = outputs.last().unwrap().as_ref().unwrap();
            assert!(matches!(output.data(), ValueData::String(text) if text.as_ref() == "2,2|4,4"));
            snapshots.push(output.canonical_snapshot_bytes(artifact.schemas()).unwrap());
            drop(outputs);
            assert_eq!((memory.used_bytes(), memory.retained_nodes()), baseline);
        }
        assert_eq!(snapshots[0], snapshots[1]);
        drop(args);
        assert_eq!((memory.used_bytes(), memory.retained_nodes()), (0, 0));
    }
}
