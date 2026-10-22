//! Prepared metadata retains neither a program graph nor runtime authority.
use super::*;

fn restored_metadata(artifact: &mech_engine::ProgramArtifact) -> CanonicalNativeResourceMetadata {
    let prepared = CanonicalNativeResourceMetadata::prepare(artifact, 16).unwrap();
    let bytes = prepared.encode_build_owned_image().unwrap();
    // SAFETY: unchanged metadata from this checked artifact, with its SAME
    // revision and schemas. The tests pair it only with that artifact's JIT.
    let restored = unsafe {
        CanonicalNativeResourceMetadata::decode_build_owned_image(
            &bytes,
            artifact.revision(),
            artifact.schemas().clone(),
        )
    }
    .unwrap();
    assert_eq!(bytes, restored.encode_build_owned_image().unwrap());
    assert_eq!(
        restored.plan().requirements().len(),
        artifact.requirements().len()
    );
    restored
}

#[test]
fn prepared_resource_metadata_executes_after_original_artifact_is_dropped() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let metadata = restored_metadata(&artifact);
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(metadata.plan().program_revision(), artifact.revision());
        drop(artifact);
        let (mut runtime, trace) = runtime(b"prepared payload", true);
        let memory = mech_core::ManagedMemoryBudget::new(256 * 1024 * 1024);
        let plan = jit.descriptor().plan();
        let mut outputs = vec![None; plan.transport_output_count()];
        runtime
            .with_prepared_canonical_native_resources_phased_text(
                &metadata,
                bridge_limits(),
                |admission| {
                    jit.prepare_hosted(admission, limits(), &memory)
                        .map(CanonicalNativeResourcePrepared::snapshot)
                        .map_err(|error| MechError::new(error, None))
                },
                |_, _| panic!("snapshot session must not receive chunks"),
                |session, turn| {
                    let report = session
                        .resume_hosted_into(turn, &[], &mut outputs)
                        .map_err(|error| MechError::new(error, None))?;
                    let effects = outputs[plan.outputs().len()..]
                        .iter_mut()
                        .map(|value| value.take().unwrap())
                        .collect::<Vec<_>>()
                        .into_boxed_slice();
                    Ok(CanonicalNativeResourceCompletion {
                        result: report,
                        effects,
                    })
                },
            )
            .unwrap();
        assert_eq!(trace.lock().unwrap().output, "prepared payload");
        // The caller-owned output transport still retains ordinary candidates.
        drop(outputs);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&runtime);
        runtime.shutdown().unwrap();

        // Reuse of successfully executed metadata carries no grant to a new runtime.
        let (mut denied, trace) = self::runtime(b"must not read", false);
        assert!(matches!(
            denied.with_prepared_canonical_native_resources_phased_text::<(), ()>(
                &metadata,
                bridge_limits(),
                |_| panic!("missing authority must precede native activation"),
                |_, _| panic!("must not capture"),
                |_, _| panic!("must not execute"),
            ),
            Err(CanonicalNativeResourceRunError::BeforeCommit {
                phase: "admission",
                ..
            })
        ));
        quiet(&trace);
        denied.shutdown().unwrap();
    }
}

#[test]
fn prepared_resource_metadata_rechecks_current_port_limit_before_activation() {
    for artifact in artifacts(ECHO).0 {
        let metadata = restored_metadata(&artifact);
        drop(artifact);
        let (mut runtime, trace) = runtime(b"must not read", true);
        let mut caps = bridge_limits();
        caps.max_ports = 0;
        assert!(matches!(
            runtime.with_prepared_canonical_native_resources_phased_text::<(), ()>(
                &metadata,
                caps,
                |_| panic!("port cap must precede activation"),
                |_, _| panic!("must not capture"),
                |_, _| panic!("must not execute"),
            ),
            Err(CanonicalNativeResourceRunError::BeforeCommit {
                phase: "admission",
                ..
            })
        ));
        quiet(&trace);
        runtime.shutdown().unwrap();
    }
}
