//! Shared real CLI hosted test setup; no replacement provider semantics.
use super::{
    JitProgramArtifact, NativeExecutionLimits, NativeProgramExecutionLimits,
    NativeProgramExecutionReport,
};
use mech_core::{MResult, ManagedMemoryBudget, MechError, Schema, Value};
use mech_engine::{ProgramArtifact, decode_program_artifact_bytecode_v1};
use mech_runtime::{
    CanonicalNativeResourceCompletion, CanonicalNativeResourceLimits,
    CanonicalNativeResourceRunError, ConfigValue, HostInstanceConfig, HostManifestConfig,
    MechRuntime, PreparedRuntimeEffect, ResourceSchemaPreflight, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeHostFactory, RuntimeHostInstallation, RuntimeProgramRoute,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteCommand,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};
use mech_terminal::{CliBackend, CliHostFactory};
use std::sync::{Arc, Mutex};

#[derive(Clone, Debug, Default)]
pub(super) struct Trace {
    pub(super) input: Vec<u8>,
    pub(super) reads: usize,
    pub(super) acquisitions: usize,
    pub(super) prepares: usize,
    pub(super) deliveries: usize,
    pub(super) flushes: usize,
    pub(super) output: String,
    pub(super) fail_flush: bool,
    pub(super) fail_read: bool,
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
pub(super) fn artifacts(source: &str) -> (Vec<ProgramArtifact>, Arc<mech_core::FunctionCatalog>) {
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
pub(super) fn runtime(input: &[u8], grants: bool) -> (MechRuntime, Arc<Mutex<Trace>>) {
    let trace = Arc::new(Mutex::new(Trace {
        input: input.into(),
        ..Trace::default()
    }));
    (builder(trace.clone(), grants).build().unwrap(), trace)
}
pub(super) fn quiet(trace: &Arc<Mutex<Trace>>) {
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
pub(super) fn no_resident(runtime: &MechRuntime) {
    let info = runtime.program_execution_info();
    assert_eq!(info.route, RuntimeProgramRoute::None);
    assert!(info.program_revision.is_none());
    assert_eq!(
        (info.resident_accepted_turns, info.resident_rejected_turns),
        (0, 0)
    );
}
pub(super) fn limits() -> NativeProgramExecutionLimits {
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
pub(super) fn bridge_limits() -> CanonicalNativeResourceLimits {
    CanonicalNativeResourceLimits {
        max_ports: 16,
        max_input_text_bytes: 1024 * 1024,
        max_output_text_bytes: 1024 * 1024,
    }
}
pub(super) fn execute(
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
