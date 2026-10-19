//! Original canonical artifacts through checked native bodies and real CLI services.
use mech_core::{MResult, ManagedMemoryBudget, MechError, Schema, Value};
use mech_engine::{ProgramArtifact, decode_program_artifact_bytecode_v1};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionError,
    NativeProgramExecutionLimits, NativeProgramExecutionReport,
};
use mech_runtime::*;
use mech_terminal::{CliBackend, CliHostFactory};
use std::sync::{Arc, Mutex};

const FULL: &str = include_str!("fixtures/native/knucleotide-idiomatic.mec");
const ECHO: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\n@out/text <- @in/text\n";
const TWO: &str = "+> @out := cli/stdout\n@out/text <- \"first\"\n@out/text <- \"second\"\n";
const CLOSED_FAIL: &str =
    "+> @in := cli/stdin\n+> @out := cli/stdout\nclosed := 1u64 / 0u64\n@out/text <- @in/text\n";
const CLOSED_LIVE: &str = "+> @in := cli/stdin\n+> @out := cli/stdout\nprefix := \"closed\" + \"-\"\n@out/text <- prefix + @in/text\n";
const FAIL: &str = "+> sequence\n+> number\n+> @in := cli/stdin\n+> @out := cli/stdout\nn := sequence/length(@in/text)\nbad := 1u64 / (n - n)\n@out/text <- number/format(bad, 0u64)\n";
const GGTA_INPUT: &[u8] = b">ONE\nAAAA\n>THREE\nGGTA\n>FOUR\nTTTT\n";
const GGTA_OUTPUT: &str = "G 50.000\nA 25.000\nT 25.000\n\nGG 33.333\nGT 33.333\nTA 33.333\n\n1\tGGT\n1\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n";
const EMPTY_OUTPUT: &str =
    "\n\n\n\n0\tGGT\n0\tGGTA\n0\tGGTATT\n0\tGGTATTTTAATT\n0\tGGTATTTTAATTTATAGT\n";

#[derive(Clone, Debug, Default)]
struct Trace {
    input: Vec<u8>,
    reads: usize,
    acquisitions: usize,
    prepares: usize,
    deliveries: usize,
    flushes: usize,
    output: String,
    fail_flush: bool,
    fail_read: bool,
}
#[derive(Clone, Debug)]
struct Backend(Arc<Mutex<Trace>>);
fn failure(message: &str) -> MechError {
    MechError::new(
        mech_core::GenericError {
            msg: message.into(),
        },
        None,
    )
}
impl CliBackend for Backend {
    fn env_var(&self, _: &str) -> MResult<Option<String>> {
        Ok(None)
    }
    fn stdin_bytes(&self) -> Option<MResult<Vec<u8>>> {
        let mut trace = self.0.lock().unwrap();
        trace.acquisitions += 1;
        Some(if trace.fail_read {
            Err(failure("controlled stdin acquisition failure"))
        } else {
            Ok(trace.input.clone())
        })
    }
    fn stdin_text(&self) -> MResult<String> {
        panic!("real CLI provider must use its cached raw-byte snapshot");
    }
    fn write_stdout(&mut self, text: &str) -> MResult<()> {
        let mut trace = self.0.lock().unwrap();
        trace.deliveries += 1;
        trace.output.push_str(text);
        trace.flushes += 1;
        if trace.fail_flush {
            return Err(failure("controlled stdout flush failure"));
        }
        Ok(())
    }
    fn write_stderr(&mut self, _: &str) -> MResult<()> {
        Err(failure("unexpected application stderr send"))
    }
}

// Transparent counters around the ACTUAL CLI provider/factory. Every semantic
// contract, read/schema/cache rule, effect preparation and delivery is delegated.
#[derive(Debug)]
struct CountProvider {
    inner: Box<dyn RuntimeResourceProvider>,
    trace: Arc<Mutex<Trace>>,
}
impl RuntimeResourceProvider for CountProvider {
    fn scheme(&self) -> &str {
        self.inner.scheme()
    }
    fn base_uris(&self) -> Vec<String> {
        self.inner.base_uris()
    }
    fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
        self.inner.equivalent_base_uri_groups()
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        self.inner.semantic_read_contract()
    }
    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        self.inner.semantic_write_contract(intent)
    }
    fn observation_requires_input_driver(&self, request: &RuntimeResourceReadRequest) -> bool {
        self.inner.observation_requires_input_driver(request)
    }
    fn supports_resident_idempotency(&self, intent: RuntimeResourceWriteIntent) -> bool {
        self.inner.supports_resident_idempotency(intent)
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.inner.plan_read(request)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        self.trace.lock().unwrap().reads += 1;
        self.inner.read(request)
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.inner.preflight_write(request)
    }
    fn preflight_write_schema(
        &self,
        request: RuntimeResourceWritePreflightRequest,
        schema: &Schema,
    ) -> MResult<ResourceSchemaPreflight> {
        self.inner.preflight_write_schema(request, schema)
    }
    fn plan_write(&self, request: RuntimeResourceWriteCommand) -> MResult<()> {
        self.inner.plan_write(request)
    }
    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.trace.lock().unwrap().prepares += 1;
        self.inner.prepare_write(request)
    }
}
#[derive(Debug)]
struct Factory {
    inner: CliHostFactory<Backend>,
    trace: Arc<Mutex<Trace>>,
}
impl RuntimeHostFactory for Factory {
    fn provider_name(&self) -> &str {
        self.inner.provider_name()
    }
    fn manifest(&self) -> &HostManifestConfig {
        self.inner.manifest()
    }
    fn validate_settings(&self, name: &str, settings: &ConfigValue) -> MResult<()> {
        self.inner.validate_settings(name, settings)
    }
    fn instantiate(&self, name: &str, settings: &ConfigValue) -> MResult<RuntimeHostInstallation> {
        let mut installation = self.inner.instantiate(name, settings)?;
        installation.resource_providers = installation
            .resource_providers
            .into_iter()
            .map(|inner| {
                Box::new(CountProvider {
                    inner,
                    trace: self.trace.clone(),
                }) as Box<dyn RuntimeResourceProvider>
            })
            .collect();
        Ok(installation)
    }
}
fn builder(trace: Arc<Mutex<Trace>>, grants: bool) -> RuntimeBuilder {
    let factory = Factory {
        inner: CliHostFactory::with_backend(Backend(trace.clone())).unwrap(),
        trace,
    };
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .host_factory(Box::new(factory))
        .unwrap()
        .host_instance(HostInstanceConfig {
            name: "cli".into(),
            provider: "cli".into(),
            settings: ConfigValue::Map(Default::default()),
        });
    if grants {
        for (target, operation) in [("cli/stdin", "read"), ("cli/stdout", "write")] {
            builder = builder.run_resource_grant(RunResourceGrantConfig {
                target: target.into(),
                operations: vec![operation.into()],
                paths: vec!["text".into()],
            });
        }
    }
    builder
}
fn artifacts(source: &str) -> (Vec<ProgramArtifact>, Arc<mech_core::FunctionCatalog>) {
    let trace = Arc::new(Mutex::new(Trace::default()));
    let mut compiler = builder(trace.clone(), false).build_compiler().unwrap();
    let product = compiler.compile_canonical_source(source).unwrap();
    let original = product.artifact().clone();
    let decoded = decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(original.requirements(), decoded.requirements());
    quiet(&trace);
    (vec![original, decoded], mech_stdlib::source_catalog())
}
fn runtime(input: &[u8], grants: bool) -> (MechRuntime, Arc<Mutex<Trace>>) {
    let trace = Arc::new(Mutex::new(Trace {
        input: input.into(),
        ..Trace::default()
    }));
    (builder(trace.clone(), grants).build().unwrap(), trace)
}
fn quiet(trace: &Arc<Mutex<Trace>>) {
    let trace = trace.lock().unwrap();
    assert_eq!(
        (
            trace.reads,
            trace.acquisitions,
            trace.prepares,
            trace.deliveries,
            trace.flushes
        ),
        (0, 0, 0, 0, 0)
    );
    assert!(trace.output.is_empty());
}
fn no_resident(runtime: &MechRuntime) {
    let info = runtime.program_execution_info();
    assert_eq!(info.route, RuntimeProgramRoute::None);
    assert!(info.program_revision.is_none());
    assert_eq!(
        (info.resident_accepted_turns, info.resident_rejected_turns),
        (0, 0)
    );
}
fn limits() -> NativeProgramExecutionLimits {
    let mut library = mech_core::TargetMemoryProfile::current_resident_cpu()
        .unwrap()
        .limits;
    library.max_output_elements = Some(1_000_000);
    library.max_output_bytes = Some(128 * 1024 * 1024);
    library.max_temporary_bytes = Some(128 * 1024 * 1024);
    library.max_cloned_bytes = Some(128 * 1024 * 1024);
    library.max_retained_nodes = Some(1_000_000);
    library.max_compute_work = Some(134_217_728);
    library.max_comparison_work = Some(1_000_000);
    NativeProgramExecutionLimits {
        execution: NativeExecutionLimits {
            max_work: 1_000_000_000,
            max_transport_bytes: 16 * 1024 * 1024,
            max_native_stack_bytes: 1024 * 1024,
        },
        library,
    }
}
fn bridge_limits() -> CanonicalNativeResourceLimits {
    CanonicalNativeResourceLimits {
        max_ports: 16,
        max_input_text_bytes: 1024 * 1024,
        max_output_text_bytes: 1024 * 1024,
    }
}
fn execute(
    runtime: &mut MechRuntime,
    artifact: &ProgramArtifact,
    jit: &JitProgramArtifact,
    policy: NativeProgramExecutionLimits,
    memory: &ManagedMemoryBudget,
) -> Result<NativeProgramExecutionReport, CanonicalNativeResourceRunError> {
    let plan = jit.descriptor().plan();
    let mut outputs = vec![None; plan.transport_output_count()];
    runtime.with_canonical_native_resources_phased(
        artifact,
        bridge_limits(),
        |admission| {
            jit.prepare_hosted(admission, policy, memory)
                .map_err(|error| MechError::new(error, None))
        },
        |session, turn| {
            let report = session
                .resume_hosted_into(turn, &[], &mut outputs)
                .map_err(|error| MechError::new(error, None))?;
            let effects = outputs[plan.outputs().len()..]
                .iter_mut()
                .map(|value| value.take().expect("checked native completion"))
                .collect::<Vec<_>>()
                .into_boxed_slice();
            Ok(CanonicalNativeResourceCompletion {
                result: report,
                effects,
            })
        },
    )
}

#[test]
fn complete_knucleotide_original_and_decoded_run_through_actual_secured_cli() {
    let (variants, catalog) = artifacts(FULL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert_eq!(jit.descriptor().plan().resource_requirements().len(), 2);
        assert_eq!(
            jit.descriptor().plan().observations()[0].request().delivery,
            mech_core::ResourceDelivery::Live
        );
        for (input, expected) in [
            (GGTA_INPUT, GGTA_OUTPUT),
            (b">THREE\n>FOUR\nA\n".as_slice(), EMPTY_OUTPUT),
            (b">ONE\nGGTA\n".as_slice(), EMPTY_OUTPUT),
        ] {
            let (mut runtime, trace) = runtime(input, true);
            let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
            let report = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
            assert!(report.visits > 0 && report.work > 0);
            no_resident(&runtime);
            let snapshot = trace.lock().unwrap().clone();
            assert_eq!(snapshot.output, expected);
            assert_eq!(
                (
                    snapshot.reads,
                    snapshot.acquisitions,
                    snapshot.prepares,
                    snapshot.deliveries,
                    snapshot.flushes
                ),
                (1, 1, 1, 1, 1)
            );
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn all_grants_are_required_before_any_observation_and_raw_inputs_cannot_bypass_them() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut fake = vec![None; jit.descriptor().plan().transport_output_count()];
        assert!(matches!(
            jit.evaluate_into(&[], &mut fake, limits(), &memory),
            Err(NativeProgramExecutionError::Base(
                mech_gpu::native_artifact::NativeExecutionError::Descriptor { .. }
            ))
        ));
        for read_only in [false, true] {
            let (mut runtime, trace) = runtime(b"payload", false);
            if read_only {
                runtime
                    .install_run_resource_grant(&RunResourceGrantConfig {
                        target: "cli/stdin".into(),
                        operations: vec!["read".into()],
                        paths: vec!["text".into()],
                    })
                    .unwrap();
            }
            assert!(matches!(
                execute(&mut runtime, &artifact, &jit, limits(), &memory),
                Err(CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "admission",
                    ..
                })
            ));
            quiet(&trace);
            no_resident(&runtime);
            assert_eq!(memory.used_bytes(), 0);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn finite_use_grant_admission_matches_the_existing_canonical_resident_policy() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", false);
        runtime
            .install_run_resource_grant(&RunResourceGrantConfig {
                target: "cli/stdout".into(),
                operations: vec!["write".into()],
                paths: vec!["text".into()],
            })
            .unwrap();
        let selected = CanonicalNativeResourcePlan::select(&artifact, 16).unwrap();
        let read = selected.observations()[0].request();
        let key = RuntimeResourceKey::new(&read.base_uri, &read.path).unwrap();
        let subject = runtime.runtime_context().unwrap().subject().to_owned();
        let capability = BasicCapability::from_keys(
            runtime.next_capability_id(),
            subject,
            key.capability_resource(),
            ["read"],
        )
        .with_constraints(BasicConstraints {
            max_uses: Some(1),
            ..BasicConstraints::default()
        });
        runtime.grant_capability(Arc::new(capability)).unwrap();
        let native = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error: native, .. } = native else {
            panic!("finite-use capability must be rejected during admission")
        };
        let resident = runtime
            .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
            .unwrap_err();
        assert_eq!(native.kind_name(), resident.kind_name());
        assert!(native.display_message().contains("finite-use"));
        assert!(resident.display_message().contains("finite-use"));
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn failed_native_body_prepares_no_effect_and_recovers_without_retaining_native_memory() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let mut denied = limits();
        denied.execution.max_work = 1;
        let error = execute(&mut runtime, &artifact, &jit, denied, &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error, cleanup, .. } = error else {
            panic!("work denial must not commit")
        };
        assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
        assert!(cleanup.is_none());
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (
                snapshot.reads,
                snapshot.acquisitions,
                snapshot.prepares,
                snapshot.deliveries
            ),
            (0, 0, 0, 0)
        );
        assert_eq!(memory.used_bytes(), 0);
        execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap();
        assert_eq!(trace.lock().unwrap().output, "payload");
        assert_eq!(
            trace.lock().unwrap().acquisitions,
            1,
            "same real provider snapshot"
        );
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
    let (variants, catalog) = artifacts(FAIL);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let error = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit { error, .. } = error else {
            panic!("arithmetic failure must not commit")
        };
        assert!(matches!(
            error.kind_as::<NativeProgramExecutionError>(),
            Some(NativeProgramExecutionError::Control(
                mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { .. }
            ))
        ));
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (1, 0, 0)
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn token_for_another_original_artifact_cannot_authorize_the_native_entry() {
    let (variants, catalog) = artifacts(ECHO);
    let other = artifacts(&format!("{ECHO}different := 7u64\n")).0.remove(0);
    let wrong = JitProgramArtifact::compile(&other, &catalog).unwrap();
    for artifact in variants {
        let (mut runtime, trace) = runtime(b"payload", true);
        let error = execute(
            &mut runtime,
            &artifact,
            &wrong,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit { .. }
        ));
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (0, 0, 0)
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn late_bad_effect_payload_is_rejected_before_the_first_real_provider_prepare() {
    for artifact in artifacts(TWO).0 {
        let (mut runtime, trace) = runtime(b"", true);
        let error = runtime
            // Privileged malformed-payload fixture, NOT native execution.
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |_| Ok(()),
                |(), turn| {
                    assert_eq!(turn.plan().effects().len(), 2);
                    let site = &turn.plan().effects()[0];
                    let value = RuntimeHostInputValue::String("first".into())
                        .into_value()
                        .unwrap();
                    let first = value
                        .rebind(site.schema(), value.shape(), artifact.schemas())
                        .unwrap();
                    let bad = RuntimeHostInputValue::U64(7).into_value().unwrap();
                    Ok(CanonicalNativeResourceCompletion {
                        result: (),
                        effects: vec![first, bad].into(),
                    })
                },
            )
            .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit {
                phase: "effect materialization",
                ..
            }
        ));
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn postcommit_flush_failure_is_not_reported_as_an_atomic_denial() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_flush = true;
        let error = execute(
            &mut runtime,
            &artifact,
            &jit,
            limits(),
            &ManagedMemoryBudget::new(64 * 1024 * 1024),
        )
        .unwrap_err();
        let CanonicalNativeResourceRunError::CommittedWithFailures(outcome) = error else {
            panic!("delivery has been attempted after commit; not a retry-safe precommit failure")
        };
        assert_eq!(outcome.delivery_failures.len(), 1);
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.prepares, snapshot.deliveries, snapshot.flushes),
            (1, 1, 1)
        );
        assert_eq!(snapshot.output, "payload");
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn missing_authority_prevents_even_the_pre_capture_native_activation_callback() {
    for artifact in artifacts(CLOSED_FAIL).0 {
        let (mut runtime, trace) = runtime(b"payload", false);
        let called = std::cell::Cell::new(false);
        let error = runtime
            .with_canonical_native_resources_phased::<(), ()>(
                &artifact,
                bridge_limits(),
                |_| {
                    called.set(true);
                    Ok(())
                },
                |(), _| panic!("missing grants must not reach captured/native turn"),
            )
            .unwrap_err();
        assert!(matches!(
            error,
            CanonicalNativeResourceRunError::BeforeCommit {
                phase: "admission",
                ..
            }
        ));
        assert!(!called.get());
        quiet(&trace);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn closed_native_arithmetic_failure_precedes_both_successful_and_failing_provider_reads() {
    let (variants, catalog) = artifacts(CLOSED_FAIL);
    for artifact in variants {
        assert!(mech_engine::resident::canonical_resource_activation_required(&artifact).unwrap());
        let divide = artifact
            .nodes()
            .iter()
            .find(|node| {
                node.as_operation()
                    .is_some_and(|operation| operation.operation.canonical_name() == "math/div")
            })
            .unwrap()
            .node;
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        for fail_read in [false, true] {
            // Establish actual canonical resident precedence, not an invented
            // native-only order. All grant/provider metadata checks succeed.
            let (mut resident, resident_trace) = runtime(b"payload", true);
            resident_trace.lock().unwrap().fail_read = fail_read;
            let reference = resident
                .load_compiled_program(artifact.clone(), ResidentDurabilityPolicy::Volatile)
                .unwrap_err();
            assert!(
                reference.display_message().contains("ActivationKernel"),
                "{reference:?}"
            );
            quiet(&resident_trace);
            no_resident(&resident);
            resident.shutdown().unwrap();

            let (mut runtime, trace) = runtime(b"payload", true);
            trace.lock().unwrap().fail_read = fail_read;
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native activation",
                error,
                cleanup,
            } = failure
            else {
                panic!("closed failure was masked by capture: {failure:?}")
            };
            assert!(cleanup.is_none());
            assert!(
                matches!(
                    error.kind_as::<NativeProgramExecutionError>(),
                    Some(NativeProgramExecutionError::Control(
                        mech_gpu::native_artifact::NativeIterationExecutionError::Arithmetic { origin }
                    )) if origin.node() == divide
                ),
                "{error:?}"
            );
            quiet(&trace);
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn native_memory_admission_precedes_even_a_failing_provider_acquisition() {
    let (variants, catalog) = artifacts(ECHO);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_read = true;
        let memory = ManagedMemoryBudget::new(0);
        let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "native activation",
            error,
            cleanup,
        } = failure
        else {
            panic!("provider failure masked memory admission: {failure:?}")
        };
        assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
        assert!(cleanup.is_none());
        quiet(&trace);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn provider_capture_failure_drops_the_already_prepared_native_session_without_effects() {
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        assert!(mech_engine::resident::canonical_resource_activation_required(&artifact).unwrap());
        let (mut runtime, trace) = runtime(b"payload", true);
        trace.lock().unwrap().fail_read = true;
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let failure = execute(&mut runtime, &artifact, &jit, limits(), &memory).unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "capture",
            error,
            cleanup,
        } = failure
        else {
            panic!("{failure:?}")
        };
        assert!(
            error
                .display_message()
                .contains("controlled stdin acquisition failure")
        );
        assert!(cleanup.is_none());
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (
                snapshot.reads,
                snapshot.acquisitions,
                snapshot.prepares,
                snapshot.deliveries,
                snapshot.flushes
            ),
            (1, 1, 0, 0, 0)
        );
        assert!(snapshot.output.is_empty());
        assert_eq!(
            memory.used_bytes(),
            0,
            "failed capture must drop activation storage/ledger"
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn activation_node_work_denial_observes_nothing_and_releases_all_native_storage() {
    use mech_gpu::native_artifact::{
        NativeExecutionError as E, NativeExecutionPhase as Phase,
        NativeIterationExecutionError as I, NativeValueError as V,
    };
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let (mut runtime, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        // Move only to the next typed cumulative admission frontier, rather
        // than hard-coding setup/library costs that legitimately evolve.
        let mut cap = 1;
        let mut found_activation_visit = false;
        for _ in 0..128 {
            let mut policy = limits();
            policy.execution.max_work = cap;
            let failure = execute(&mut runtime, &artifact, &jit, policy, &memory).unwrap_err();
            let CanonicalNativeResourceRunError::BeforeCommit {
                phase: "native activation",
                error,
                ..
            } = failure
            else {
                panic!("denial escaped activation/captured a provider: {failure:?}")
            };
            let native = error.kind_as::<NativeProgramExecutionError>().unwrap();
            let (node, required) = match native {
                NativeProgramExecutionError::Base(E::WorkLimit {
                    phase, required, ..
                })
                | NativeProgramExecutionError::Value(V::Base(E::WorkLimit {
                    phase,
                    required,
                    ..
                }))
                | NativeProgramExecutionError::Control(I::Base(E::WorkLimit {
                    phase,
                    required,
                    ..
                })) => (
                    match phase {
                        Phase::Node(node) => Some(*node),
                        _ => None,
                    },
                    *required,
                ),
                NativeProgramExecutionError::Control(I::Work {
                    origin, required, ..
                }) => (origin.as_ref().map(|origin| origin.node()), *required),
                NativeProgramExecutionError::Library {
                    origin,
                    error:
                        mech_engine::native_artifact::NativeCollectionInvocationError::Kernel(
                            mech_core::ResidentKernelError::ResourceLimit {
                                dimension: mech_core::MemoryBudgetDimension::ComputeWork,
                                required,
                                ..
                            },
                        ),
                } => (Some(origin.node()), *required),
                _ => panic!("expected typed cumulative work denial: {native:?}"),
            };
            quiet(&trace);
            assert_eq!(memory.used_bytes(), 0);
            assert!(required > cap);
            if node.is_some() {
                found_activation_visit = true;
                break;
            }
            cap = required;
        }
        assert!(
            found_activation_visit,
            "fixture never reached an activation-node denial"
        );
        no_resident(&runtime);
        runtime.shutdown().unwrap();
    }
}

#[test]
fn phased_linear_native_activation_is_not_replayed_and_shares_the_final_work_limit() {
    let (variants, catalog) = artifacts(CLOSED_LIVE);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let expected_visits = jit.descriptor().plan().blocks().len() as u64;
        let (mut baseline, trace) = runtime(b"payload", true);
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let report = execute(&mut baseline, &artifact, &jit, limits(), &memory).unwrap();
        assert_eq!(trace.lock().unwrap().output, "closed-payload");
        // This fixture has no loops, branches or dormant rejects. The phase
        // checkpoint returns BEFORE the turn boundary's visit; resume visits
        // it once, without replaying any closed block or setup.
        assert_eq!(report.visits, expected_visits);
        assert_eq!(memory.used_bytes(), 0);
        no_resident(&baseline);
        baseline.shutdown().unwrap();

        for accepted in [true, false] {
            let (mut runtime, trace) = runtime(b"payload", true);
            let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
            let mut policy = limits();
            policy.execution.max_work = report.work - u64::from(!accepted);
            let result = execute(&mut runtime, &artifact, &jit, policy, &memory);
            if accepted {
                let exact = result.unwrap();
                assert_eq!(exact.work, report.work);
                assert_eq!(exact.visits, expected_visits);
                assert_eq!(trace.lock().unwrap().output, "closed-payload");
            } else {
                let failure = result.unwrap_err();
                let CanonicalNativeResourceRunError::BeforeCommit {
                    phase: "native execution",
                    error,
                    ..
                } = failure
                else {
                    panic!("expected cumulative completion denial: {failure:?}")
                };
                assert!(error.kind_as::<NativeProgramExecutionError>().is_some());
                let snapshot = trace.lock().unwrap().clone();
                assert_eq!(
                    (snapshot.reads, snapshot.prepares, snapshot.deliveries),
                    (1, 0, 0)
                );
                assert!(snapshot.output.is_empty());
            }
            assert_eq!(memory.used_bytes(), 0);
            no_resident(&runtime);
            runtime.shutdown().unwrap();
        }
    }
}

#[test]
fn same_artifact_session_cannot_resume_with_a_different_admitted_transaction_token() {
    let source = "+> @in := cli/stdin\ntext := @in/text\n";
    let (variants, catalog) = artifacts(source);
    for artifact in variants {
        let jit = JitProgramArtifact::compile(&artifact, &catalog).unwrap();
        let memory = ManagedMemoryBudget::new(64 * 1024 * 1024);
        let (mut first, _) = runtime(b"first", true);
        // Privileged test host intentionally retains an unresumed session.
        // This is NOT successful native evaluation or an application path.
        let prepared = first
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |admission| {
                    jit.prepare_hosted(admission, limits(), &memory)
                        .map_err(|error| MechError::new(error, None))
                },
                |session, _| {
                    Ok(CanonicalNativeResourceCompletion {
                        result: session,
                        effects: Box::new([]),
                    })
                },
            )
            .unwrap();
        first.shutdown().unwrap();
        assert!(memory.used_bytes() > 0);
        let (mut second, trace) = runtime(b"second", true);
        let mut outputs = vec![None; jit.descriptor().plan().transport_output_count()];
        let failure = second
            .with_canonical_native_resources_phased(
                &artifact,
                bridge_limits(),
                |_| Ok(prepared),
                |session, turn| {
                    let result = session
                        .resume_hosted_into(turn, &[], &mut outputs)
                        .map_err(|error| MechError::new(error, None))?;
                    Ok(CanonicalNativeResourceCompletion {
                        result,
                        effects: Box::new([]),
                    })
                },
            )
            .unwrap_err();
        let CanonicalNativeResourceRunError::BeforeCommit {
            phase: "native execution",
            error,
            ..
        } = failure
        else {
            panic!("{failure:?}")
        };
        assert!(matches!(
            error.kind_as::<NativeProgramExecutionError>(),
            Some(NativeProgramExecutionError::Base(
                mech_gpu::native_artifact::NativeExecutionError::Descriptor { .. }
            ))
        ));
        assert!(outputs.iter().all(Option::is_none));
        assert_eq!(memory.used_bytes(), 0);
        let snapshot = trace.lock().unwrap().clone();
        assert_eq!(
            (snapshot.reads, snapshot.prepares, snapshot.deliveries),
            (1, 0, 0)
        );
        no_resident(&second);
        second.shutdown().unwrap();
    }
}

#[path = "support/canonical_hosted_linked.rs"]
mod linked;
