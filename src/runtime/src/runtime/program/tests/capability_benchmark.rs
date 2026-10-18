use super::*;

const SOURCE: &str =
    "@clock := snapshot://clock/tick{:read(value)}\nvalue := @clock/value\nvalue\n";

#[test]
fn benchmark_capability_baseline_uses_the_same_canonical_artifact_and_execution() {
    let catalog = mech_stdlib::source_catalog();
    let reads = Arc::new(AtomicUsize::new(0));
    let provider = || DriverlessObservationProvider {
        reads: Arc::clone(&reads),
    };
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .resource_provider(Box::new(provider()))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(SOURCE)
        .unwrap()
        .into_parts()
        .0;
    let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let revision = artifact.revision();
    let mut summaries = Vec::new();
    for trusted in [false, true] {
        let mut config = crate::RuntimeConfig::default();
        config.limits.max_memory_bytes = Some(128 * 1024 * 1024);
        let mut builder = RuntimeBuilder::new()
            .config(config)
            .function_catalog(Arc::clone(&catalog))
            .resource_provider(Box::new(provider()));
        if trusted {
            builder = builder.benchmark_trusted_resident_capabilities();
        }
        let mut runtime = builder.build().unwrap();
        assert_eq!(runtime.trust_resident_requirements_for_benchmark, trusted);
        assert!(runtime.resident_memory_budget.is_some());
        if !trusted {
            let before = reads.load(Ordering::SeqCst);
            let error = runtime
                .load_compiled_program(
                    decode_program_artifact_bytecode_v1(&bytes).unwrap(),
                    crate::ResidentDurabilityPolicy::Volatile,
                )
                .unwrap_err();
            assert!(format!("{error:?}").contains("AuthorizationDenied"));
            assert_eq!(reads.load(Ordering::SeqCst), before);
            let subject = runtime.runtime_context().unwrap().subject;
            runtime
                .grant_capability(Arc::new(BasicCapability::from_keys(
                    CapabilityId(0xb001),
                    subject,
                    "snapshot://clock/tick/value",
                    ["read"],
                )))
                .unwrap();
        }
        let before = reads.load(Ordering::SeqCst);
        let loaded = runtime
            .load_compiled_program(
                decode_program_artifact_bytecode_v1(&bytes).unwrap(),
                crate::ResidentDurabilityPolicy::Volatile,
            )
            .unwrap();
        assert_eq!(reads.load(Ordering::SeqCst), before + 1);
        assert_eq!(loaded.info.program_revision, Some(revision));
        assert_eq!(loaded.info.route, RuntimeProgramRoute::ResidentExternal);
        assert_eq!(loaded.info.resident_accepted_turns, 1);
        let (actual, instance) = resident_parts(&runtime);
        assert_eq!(actual.revision(), revision);
        let output = match instance.output_borrow(0).unwrap() {
            ResidentValueBorrow::Bool { values, .. } => values == [1],
            ResidentValueBorrow::Snapshot { values, .. } => {
                matches!(values[0].as_ref().unwrap().data(), ValueData::Bool(true))
            }
            other => panic!("unexpected canonical Bool transport: {other:?}"),
        };
        assert!(output);
        summaries.push((loaded.info.observation_count, output));
        runtime.shutdown().unwrap();
    }
    assert_eq!(summaries[0].0, summaries[1].0);
    assert_eq!(summaries[0].1, summaries[1].1);
}

#[test]
fn benchmark_capability_baseline_does_not_bypass_provider_or_memory_admission() {
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .resource_provider(Box::new(DriverlessObservationProvider {
            reads: Arc::new(AtomicUsize::new(0)),
        }))
        .build_compiler()
        .unwrap();
    let artifact = compiler
        .compile_canonical_source(SOURCE)
        .unwrap()
        .into_parts()
        .0;
    for missing_provider in [false, true] {
        let mut config = crate::RuntimeConfig::default();
        config.limits.max_memory_bytes = Some(1);
        let mut builder = RuntimeBuilder::new()
            .config(config)
            .function_catalog(mech_stdlib::source_catalog())
            .benchmark_trusted_resident_capabilities();
        let reads = Arc::new(AtomicUsize::new(0));
        if !missing_provider {
            builder = builder.resource_provider(Box::new(DriverlessObservationProvider {
                reads: Arc::clone(&reads),
            }));
        }
        let mut runtime = builder.build().unwrap();
        assert!(
            runtime
                .load_compiled_program(artifact.clone(), crate::ResidentDurabilityPolicy::Volatile)
                .is_err()
        );
        assert_eq!(reads.load(Ordering::SeqCst), 0);
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
        runtime.shutdown().unwrap();
    }
}
