use std::{
    num::NonZeroU32,
    sync::{
        Arc, LazyLock, Mutex,
        atomic::{AtomicBool, Ordering},
    },
};

use mech_compute::{
    BackendRequest, ComputeBackendRegistry, ComputeDispatchReport, ComputeInitializerSet,
    ComputeInputUpdate, ComputePlatform, ComputePort, ComputeProgram, ComputeSession, ComputeValue,
    TensorLayout,
};
use mech_core::{
    AccessMode, ComputePlacement, DeliveryMode, EffectContract, EffectDeliveryPolicy,
    ExternalInteraction, IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue,
    MResult, MechError, MechErrorKind, OperationContractDeclaration, Ref,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest,
};

static COMPUTE_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
    LazyLock::new(|| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::AtMostOnce,
            idempotency: IdempotencyRequirement::NotRequired,
        }),
    });

/// Installs one compiler-produced compute region behind the ordinary resident
/// runtime host boundary. Backend selection and compilation are intentionally
/// contained here; the resource adapter only translates runtime values into
/// the typed compute interface.
pub struct ComputeHostFactory {
    region: Box<str>,
    placement: ComputePlacement,
    program: Arc<ComputeProgram>,
    initializers: ComputeInitializerSet,
    registry: Arc<ComputeBackendRegistry>,
    platform: ComputePlatform,
    backend_override: Option<BackendRequest>,
    installed_instance: Mutex<Option<String>>,
    manifest: HostManifestConfig,
}

impl std::fmt::Debug for ComputeHostFactory {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeHostFactory")
            .field("region", &self.region)
            .field("placement", &self.placement)
            .field("platform", &self.platform)
            .field("backend_override", &self.backend_override)
            .field("registry", &self.registry)
            .finish_non_exhaustive()
    }
}

impl ComputeHostFactory {
    pub fn new(
        region: impl Into<Box<str>>,
        placement: ComputePlacement,
        program: ComputeProgram,
        initializers: ComputeInitializerSet,
        registry: Arc<ComputeBackendRegistry>,
        platform: ComputePlatform,
    ) -> MResult<Self> {
        let region = region.into();
        if region.trim().is_empty() {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                "the configured compute region name must be nonempty",
            ));
        }
        Ok(Self {
            region,
            placement,
            program: Arc::new(program),
            initializers,
            registry,
            platform,
            backend_override: None,
            installed_instance: Mutex::new(None),
            manifest: compute_host_manifest(),
        })
    }

    pub fn with_backend_override(mut self, request: BackendRequest) -> Self {
        self.backend_override = Some(request);
        self
    }

    fn configured_request(&self, settings: &ConfigValue) -> MResult<BackendRequest> {
        let configured = configured_compute_settings(settings)?;
        if configured.region != self.region.as_ref() {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                format!(
                    "configured region `{}` does not match compiled region `{}`",
                    configured.region, self.region
                ),
            ));
        }
        Ok(self.backend_override.clone().unwrap_or(configured.backend))
    }
}

impl RuntimeHostFactory for ComputeHostFactory {
    fn provider_name(&self) -> &str {
        "compute"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        let request = self.configured_request(settings)?;
        self.registry
            .resolve(&request, self.platform, self.placement, &self.program)
            .map(|_| ())
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendSelection",
                    format!(
                        "region `{}` has no compatible backend: {error}",
                        self.region
                    ),
                )
            })
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let mut installed = self.installed_instance.lock().map_err(|_| {
            compute_host_error(
                "ComputeHostConfiguration",
                "compute host installation lock is poisoned",
            )
        })?;
        if let Some(existing) = installed.as_ref() {
            return Err(compute_host_error(
                "MultipleComputeHostsUnsupported",
                format!(
                    "v0.4 supports one configured compute host; instance `{existing}` is already installed"
                ),
            ));
        }

        let request = self.configured_request(settings)?;
        let backend = self
            .registry
            .resolve(&request, self.platform, self.placement, &self.program)
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendSelection",
                    format!(
                        "region `{}` has no compatible backend: {error}",
                        self.region
                    ),
                )
            })?;
        let backend_id = backend.descriptor().id.clone();
        let executable = backend.compile(&self.program).map_err(|error| {
            compute_host_error(
                "ComputeBackendCompile",
                format!("region `{}` could not compile: {error}", self.region),
            )
        })?;
        let session = executable
            .create_session(&self.initializers)
            .map_err(|error| {
                compute_host_error(
                    "ComputeBackendInitialize",
                    format!("region `{}` could not initialize: {error}", self.region),
                )
            })?;

        let telemetry = Arc::new(Mutex::new(None));
        let live = Arc::new(AtomicBool::new(false));
        let base_uri = format!("compute://{instance_name}/kernel");
        let state = Arc::new(Mutex::new(ComputeHostState {
            backend: backend_id.to_string(),
            turns: Ref::new(0.0),
            dispatch_ms: Ref::new(0.0),
            fault_count: Ref::new(0.0),
            last_fault: Ref::new(String::new()),
            session,
        }));
        let installation = RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(ComputeResourceProvider {
                instance: instance_name.to_owned(),
                region: self.region.clone(),
                program: Arc::clone(&self.program),
                state,
                telemetry: Arc::clone(&telemetry),
            })],
            input_drivers: vec![Box::new(ComputeTelemetryDriver {
                base_uri,
                ingress: telemetry,
                live,
            })],
        };
        *installed = Some(instance_name.to_owned());
        Ok(installation)
    }
}

struct ComputeHostState {
    backend: String,
    turns: Ref<f64>,
    dispatch_ms: Ref<f64>,
    fault_count: Ref<f64>,
    last_fault: Ref<String>,
    session: Box<dyn ComputeSession>,
}

impl std::fmt::Debug for ComputeHostState {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeHostState")
            .field("backend", &self.backend)
            .field("turns", &self.turns)
            .field("dispatch_ms", &self.dispatch_ms)
            .field("fault_count", &self.fault_count)
            .field("last_fault", &self.last_fault)
            .field("session", &"<dyn ComputeSession>")
            .finish()
    }
}

#[derive(Debug)]
struct ComputeResourceProvider {
    instance: String,
    region: Box<str>,
    program: Arc<ComputeProgram>,
    state: Arc<Mutex<ComputeHostState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl ComputeResourceProvider {
    fn base_uri(&self) -> String {
        format!("compute://{}/kernel", self.instance)
    }

    fn declared_input(&self, path: &str) -> Option<&ComputePort> {
        let name = path.strip_prefix("input/")?;
        (!name.is_empty())
            .then(|| self.program.interface().input_named(name))
            .flatten()
    }

    fn telemetry_value(&self, path: &str, planning: bool) -> MResult<LegacyValue> {
        if planning {
            return match path {
                "backend" | "last-fault" => Ok(LegacyValue::String(Ref::new(String::new()))),
                "turns" | "dispatch-ms" | "fault-count" => Ok(LegacyValue::F64(Ref::new(0.0))),
                other => Err(compute_host_error(
                    "ComputeHostRead",
                    format!("unknown compute telemetry path `{other}`"),
                )),
            };
        }
        let state = self.state.lock().map_err(|_| {
            compute_host_error("ComputeHostRead", "compute host state lock is poisoned")
        })?;
        match path {
            "backend" => Ok(LegacyValue::String(Ref::new(state.backend.clone()))),
            "turns" => Ok(LegacyValue::F64(state.turns.clone())),
            "dispatch-ms" => Ok(LegacyValue::F64(state.dispatch_ms.clone())),
            "fault-count" => Ok(LegacyValue::F64(state.fault_count.clone())),
            "last-fault" => Ok(LegacyValue::String(state.last_fault.clone())),
            other => Err(compute_host_error(
                "ComputeHostRead",
                format!("unknown compute telemetry path `{other}`"),
            )),
        }
    }
}

impl RuntimeResourceProvider for ComputeResourceProvider {
    fn scheme(&self) -> &str {
        "compute"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&COMPUTE_EFFECT_CONTRACT)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.validate_base_uri(&request.base_uri, "ComputeHostRead")?;
        self.telemetry_value(&request.path, true)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.validate_base_uri(&request.base_uri, "ComputeHostRead")?;
        self.telemetry_value(&request.path, false)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.validate_base_uri(&request.base_uri, "ComputeHostWrite")?;
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(compute_host_error(
                "ComputeHostWrite",
                "compute dispatch is an effect; use <-",
            ));
        }
        if request.path != "turn" && self.declared_input(&request.path).is_none() {
            return Err(compute_host_error(
                "ComputeHostWrite",
                format!("unknown compute input path `{}`", request.path),
            ));
        }
        Ok(())
    }

    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if let Some(port) = self.declared_input(&request.path) {
            compute_input_update(port, request.value.try_deep_snapshot()?)?;
        }
        Ok(())
    }

    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.preflight_write(RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if let Some(port) = self.declared_input(&request.path) {
            let update = compute_input_update(port, request.value.try_deep_snapshot()?)?;
            return Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
                ComputeInputEffect {
                    resource: request.base_uri,
                    region: self.region.clone(),
                    update,
                    state: Arc::clone(&self.state),
                },
            )));
        }
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ComputeDispatchEffect {
                resource: request.base_uri,
                region: self.region.clone(),
                state: Arc::clone(&self.state),
                telemetry: Arc::clone(&self.telemetry),
            },
        )))
    }
}

impl ComputeResourceProvider {
    fn validate_base_uri(&self, actual: &str, operation: &'static str) -> MResult<()> {
        if actual == self.base_uri() {
            Ok(())
        } else {
            Err(compute_host_error(
                operation,
                format!("unknown compute resource `{actual}`"),
            ))
        }
    }
}

#[derive(Debug)]
struct ComputeInputEffect {
    resource: String,
    region: Box<str>,
    update: ComputeInputUpdate,
    state: Arc<Mutex<ComputeHostState>>,
}

impl RuntimeAfterCommitEffect for ComputeInputEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "compute".to_owned(),
            },
            format!("input:{}:{}", self.region, self.update.port.get()),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost {
            bytes: compute_value_elements(&self.update.value)
                .saturating_mul(std::mem::size_of::<f32>() as u64),
            items: compute_value_elements(&self.update.value),
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| {
                compute_host_error("ComputeHostWrite", "compute host state lock is poisoned")
            })?
            .session
            .update_inputs(std::slice::from_ref(&self.update))
            .map_err(|error| compute_host_error("ComputeHostWrite", error.to_string()))
    }
}

#[derive(Debug)]
struct ComputeDispatchEffect {
    resource: String,
    region: Box<str>,
    state: Arc<Mutex<ComputeHostState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl RuntimeAfterCommitEffect for ComputeDispatchEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "compute".to_owned(),
            },
            format!("dispatch:{}", self.region),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            compute_host_error("ComputeHostDispatch", "compute host state lock is poisoned")
        })?;
        let report = state
            .session
            .dispatch(NonZeroU32::new(1).expect("one is nonzero"))
            .map_err(|error| compute_host_error("ComputeHostDispatch", error.to_string()))?;
        apply_report(&mut state, &report);
        let updates = telemetry_updates(&self.resource, &state)?;
        drop(state);
        if let Some(ingress) = self
            .telemetry
            .lock()
            .map_err(|_| {
                compute_host_error(
                    "ComputeHostDispatch",
                    "compute telemetry ingress lock is poisoned",
                )
            })?
            .as_ref()
        {
            ingress.submit(RuntimeHostInput::new(updates)?)?;
        }
        Ok(())
    }
}

fn apply_report(state: &mut ComputeHostState, report: &ComputeDispatchReport) {
    *state.turns.borrow_mut() += f64::from(report.completed_turns);
    *state.dispatch_ms.borrow_mut() = report.dispatch_milliseconds;
    *state.fault_count.borrow_mut() = report.fault_count as f64;
    if let Some(fault) = &report.last_fault {
        *state.last_fault.borrow_mut() = format!(
            "turn {}: {}: {}",
            fault.attempted_turn, fault.constraint, fault.detail
        );
    }
}

fn telemetry_updates(
    resource: &str,
    state: &ComputeHostState,
) -> MResult<Vec<RuntimeHostInputUpdate>> {
    Ok(vec![
        telemetry_update(
            resource,
            "backend",
            RuntimeHostInputValue::String(state.backend.clone()),
        )?,
        telemetry_update(
            resource,
            "turns",
            RuntimeHostInputValue::F64(*state.turns.borrow()),
        )?,
        telemetry_update(
            resource,
            "dispatch-ms",
            RuntimeHostInputValue::F64(*state.dispatch_ms.borrow()),
        )?,
        telemetry_update(
            resource,
            "fault-count",
            RuntimeHostInputValue::F64(*state.fault_count.borrow()),
        )?,
        telemetry_update(
            resource,
            "last-fault",
            RuntimeHostInputValue::String(state.last_fault.borrow().clone()),
        )?,
    ])
}

fn telemetry_update(
    resource: &str,
    path: &str,
    value: RuntimeHostInputValue,
) -> MResult<RuntimeHostInputUpdate> {
    Ok(RuntimeHostInputUpdate {
        source: RuntimeHostInputSource::new(resource, path)?,
        value,
    })
}

#[derive(Debug)]
struct ComputeTelemetryDriver {
    base_uri: String,
    ingress: Arc<Mutex<Option<RuntimeIngress>>>,
    live: Arc<AtomicBool>,
}

impl RuntimeHostInputDriver for ComputeTelemetryDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == self.base_uri
            && matches!(
                source.path(),
                "backend" | "turns" | "dispatch-ms" | "fault-count" | "last-fault"
            )
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut attached = self.ingress.lock().map_err(|_| {
            compute_host_error(
                "ComputeTelemetryDriver",
                "compute telemetry ingress lock is poisoned",
            )
        })?;
        if attached.is_some() {
            return Err(compute_host_error(
                "ComputeTelemetryDriver",
                "compute telemetry driver is already attached",
            ));
        }
        *attached = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        if self
            .ingress
            .lock()
            .map_err(|_| {
                compute_host_error(
                    "ComputeTelemetryDriver",
                    "compute telemetry ingress lock is poisoned",
                )
            })?
            .is_none()
        {
            return Err(compute_host_error(
                "ComputeTelemetryDriver",
                "compute telemetry driver must be attached before start",
            ));
        }
        self.live.store(true, Ordering::SeqCst);
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.live.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.live.load(Ordering::SeqCst)
    }
}

pub fn compute_host_manifest() -> HostManifestConfig {
    HostManifestConfig {
        provider: "compute".to_owned(),
        contexts: vec![mech_runtime::HostContextManifest {
            name: "kernel".to_owned(),
            base_uri_template: "compute://{instance}/kernel".to_owned(),
            operations: vec!["read".to_owned(), "write".to_owned()],
        }],
    }
}

struct ConfiguredComputeSettings {
    region: String,
    backend: BackendRequest,
}

fn configured_compute_settings(settings: &ConfigValue) -> MResult<ConfiguredComputeSettings> {
    let ConfigValue::Map(map) = settings else {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host settings must be a map",
        ));
    };
    for key in map.keys() {
        if !matches!(key.as_str(), "region" | "backend") {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                format!("unknown compute host setting `{key}`"),
            ));
        }
    }
    let Some(ConfigValue::String(region)) = map.get("region") else {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host setting `region` must be a string",
        ));
    };
    if region.trim().is_empty() {
        return Err(compute_host_error(
            "ComputeHostConfiguration",
            "compute host setting `region` must be nonempty",
        ));
    }
    let backend = match map.get("backend") {
        None => BackendRequest::Auto,
        Some(ConfigValue::String(value)) => BackendRequest::parse(value).map_err(|error| {
            compute_host_error(
                "ComputeHostConfiguration",
                format!("invalid compute backend `{value}`: {error}"),
            )
        })?,
        Some(_) => {
            return Err(compute_host_error(
                "ComputeHostConfiguration",
                "compute host setting `backend` must be a string",
            ));
        }
    };
    Ok(ConfiguredComputeSettings {
        region: region.clone(),
        backend,
    })
}

fn compute_input_update(port: &ComputePort, value: LegacyValue) -> MResult<ComputeInputUpdate> {
    let value = match value {
        LegacyValue::Typed(value, _) => return compute_input_update(port, *value),
        LegacyValue::MutableReference(value) => {
            return compute_input_update(port, value.borrow().clone());
        }
        LegacyValue::F32(value) => ComputeValue::ScalarF32(*value.borrow()),
        LegacyValue::MatrixF32(matrix) => ComputeValue::TensorF32 {
            dimensions: vec![matrix.rows() as u64, matrix.cols() as u64].into_boxed_slice(),
            layout: TensorLayout::ColumnMajor,
            values: Arc::from(matrix.as_vec()),
        },
        other => {
            return Err(compute_host_error(
                "ComputeHostWrite",
                format!(
                    "compute input `{}` requires fixed-shape f32 data, found `{}`",
                    port.name,
                    other.kind()
                ),
            ));
        }
    };
    let value = port.normalize_value(value).map_err(|error| {
        compute_host_error(
            "ComputeHostWrite",
            format!("compute input `{}` is invalid: {error}", port.name),
        )
    })?;
    Ok(ComputeInputUpdate {
        port: port.id,
        value,
    })
}

fn compute_value_elements(value: &ComputeValue) -> u64 {
    match value {
        ComputeValue::ScalarF32(_) => 1,
        ComputeValue::TensorF32 { values, .. } => u64::try_from(values.len()).unwrap_or(u64::MAX),
    }
}

#[derive(Clone, Debug)]
struct ComputeHostError {
    name: &'static str,
    message: String,
}

impl MechErrorKind for ComputeHostError {
    fn name(&self) -> &str {
        self.name
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

fn compute_host_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        ComputeHostError {
            name,
            message: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    use mech_compute::{
        BackendClass, ComputeBackendCapabilities, ComputeBackendDescriptor, ComputeBackendError,
        ComputeBackendFactory, ComputeBackendRejection, ComputeExecutable, ComputeKernel,
        ComputePhysicalPlan, ComputeRegionInterface, ElementwiseIr,
    };
    use mech_core::{CellSlotId, SchemaId};

    struct FakeBackend {
        descriptor: ComputeBackendDescriptor,
    }

    impl ComputeBackendFactory for FakeBackend {
        fn descriptor(&self) -> &ComputeBackendDescriptor {
            &self.descriptor
        }

        fn supports(&self, _program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
            Ok(())
        }

        fn compile(
            &self,
            _program: &ComputeProgram,
        ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
            Ok(Box::new(FakeExecutable {
                backend: self.descriptor.id.clone(),
            }))
        }
    }

    struct FakeExecutable {
        backend: mech_compute::BackendId,
    }

    impl ComputeExecutable for FakeExecutable {
        fn create_session(
            &self,
            _initializers: &ComputeInitializerSet,
        ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
            Ok(Box::new(FakeSession {
                backend: self.backend.clone(),
            }))
        }
    }

    struct FakeSession {
        backend: mech_compute::BackendId,
    }

    impl ComputeSession for FakeSession {
        fn update_inputs(
            &mut self,
            _updates: &[ComputeInputUpdate],
        ) -> Result<(), mech_compute::ComputeExecutionError> {
            Ok(())
        }

        fn dispatch(
            &mut self,
            turns: NonZeroU32,
        ) -> Result<ComputeDispatchReport, mech_compute::ComputeExecutionError> {
            Ok(ComputeDispatchReport {
                completed_turns: turns.get(),
                ..Default::default()
            })
        }

        fn read_outputs(
            &mut self,
            _selection: &mech_compute::ComputeOutputSelection,
        ) -> Result<mech_compute::ComputeOutputSnapshot, mech_compute::ComputeExecutionError>
        {
            let _ = &self.backend;
            Ok(Default::default())
        }
    }

    fn program() -> ComputeProgram {
        ComputeProgram::new(
            ComputeRegionInterface {
                inputs: vec![ComputePort {
                    id: mech_compute::ComputePortId::new(0),
                    name: "matrix".into(),
                    slot: CellSlotId::new(0),
                    schema: SchemaId::new(0),
                    element: mech_compute::ComputeElementType::F32,
                    dimensions: vec![2, 3].into_boxed_slice(),
                }]
                .into_boxed_slice(),
                ..Default::default()
            },
            ComputePhysicalPlan::default(),
            ComputeKernel::Elementwise(ElementwiseIr::default()),
        )
    }

    fn registry() -> Arc<ComputeBackendRegistry> {
        let mut registry = ComputeBackendRegistry::default();
        registry
            .register(Arc::new(FakeBackend {
                descriptor: ComputeBackendDescriptor {
                    id: mech_compute::BackendId::new("cpu-scalar").unwrap(),
                    class: BackendClass::Cpu,
                    priority: 1,
                    capabilities: ComputeBackendCapabilities {
                        elementwise: true,
                        native: true,
                        ..Default::default()
                    },
                },
            }))
            .unwrap();
        Arc::new(registry)
    }

    fn settings() -> ConfigValue {
        ConfigValue::Map(BTreeMap::from([
            (
                "region".to_owned(),
                ConfigValue::String("particle-field".to_owned()),
            ),
            ("backend".to_owned(), ConfigValue::String("cpu".to_owned())),
        ]))
    }

    #[test]
    fn manifest_uses_backend_neutral_provider_and_scheme() {
        let manifest = compute_host_manifest();
        assert_eq!(manifest.provider, "compute");
        assert_eq!(
            manifest.contexts[0].base_uri_template,
            "compute://{instance}/kernel"
        );
    }

    #[test]
    fn exact_matrix_shape_and_layout_are_checked_at_the_host_boundary() {
        let program = program();
        let port = &program.interface().inputs[0];
        let valid = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 3,
            values: vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
        }
        .into_mech_value()
        .unwrap();
        let update = compute_input_update(port, valid).unwrap();
        let ComputeValue::TensorF32 { layout, values, .. } = update.value else {
            panic!("matrix became a scalar")
        };
        assert_eq!(layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]);

        let wrong_shape = RuntimeHostInputValue::F32Matrix {
            rows: 3,
            columns: 2,
            values: vec![0.0; 6],
        }
        .into_mech_value()
        .unwrap();
        assert!(compute_input_update(port, wrong_shape).is_err());
    }

    #[test]
    fn factory_rejects_a_second_compute_host_instance() {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap();
        factory.instantiate("particles", &settings()).unwrap();
        let error = factory.instantiate("second", &settings()).unwrap_err();
        assert_eq!(error.kind_name(), "MultipleComputeHostsUnsupported");
    }

    #[test]
    fn backend_override_is_resolved_by_registry_without_changing_source() {
        let factory = ComputeHostFactory::new(
            "particle-field",
            ComputePlacement::Compute,
            program(),
            ComputeInitializerSet::default(),
            registry(),
            ComputePlatform::Native,
        )
        .unwrap()
        .with_backend_override(BackendRequest::Exact(
            mech_compute::BackendId::new("wgpu").unwrap(),
        ));
        assert!(factory.validate_settings("particles", &settings()).is_err());
    }
}
