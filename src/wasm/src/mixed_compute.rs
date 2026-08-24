use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use js_sys::{Array, Float32Array, Object, Reflect};
use mech_compute::{
    BackendClass, BackendId, BackendRequest, CPU_SCALAR_BACKEND, ComputeBackendCapabilities,
    ComputeBackendDescriptor, ComputeBackendError, ComputeBackendFactory, ComputeBackendRegistry,
    ComputeBackendRejection, ComputeCompletionOutcome, ComputeCompletionTarget,
    ComputeDispatchReport, ComputeDispatchRequest, ComputeExecutable, ComputeExecutionError,
    ComputeFaultEvidence, ComputeInitializerSet, ComputeInputUpdate, ComputeKernel,
    ComputeOutputSelection, ComputeOutputSnapshot, ComputePlatform, ComputePortId, ComputeProgram,
    ComputeSession, ComputeValue, TensorLayout, WGPU_BACKEND,
};
use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Program};
use mech_engine::ProgramArtifact;
use mech_gpu::{
    ComputeHostFactory, ComputeHostStateSnapshotHandle, ComputeLowerer, CpuScalarBackendFactory,
    ElementwiseKernel, FixedShapeKernel, lower_elementwise_compute_program,
};
use mech_runtime::{
    ConfigProfileOptions, ConfigValue, HostContextManifest, HostManifestConfig, MechConfigDocument,
    MechRuntime, RuntimeBuilder, RuntimeHostFactory, RuntimeHostInput, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeHostInstallation,
    RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest, materialize_host_manifest,
    parse_config_document,
};
use wasm_bindgen::prelude::*;
use web_time::Instant;

use crate::gpu::{BrowserGpuProgram, CompileTimings, gpu_program_manifest};

const POINTER_PATHS: [&str; 4] = ["pulse", "position", "pressed", "delta-seconds"];

#[wasm_bindgen]
pub struct WasmMixedComputeProject {
    runtime: MechRuntime,
    pointer: PointerInputHandle,
    compute: ComputeCommandHandle,
    outputs: BrowserOutputHandle,
    backend: String,
    manifest: JsValue,
    started: bool,
    stopped: bool,
}

#[wasm_bindgen]
impl WasmMixedComputeProject {
    #[wasm_bindgen(js_name = fromSource)]
    pub fn from_source(
        config_source: &str,
        source: &str,
        backend_override: &str,
        gpu_available: bool,
    ) -> Result<WasmMixedComputeProject, JsValue> {
        let document = parse_config_document(
            "browser-project/mech.mcfg",
            config_source,
            ConfigProfileOptions::default(),
        )
        .map_err(js_error)?;
        let parse_started = Instant::now();
        let tree = mech_syntax::parse(source.trim()).map_err(js_error)?;
        let parsing = milliseconds(parse_started);
        let pointer_index = configured_host_index(&document, "pointer").map_err(js_error)?;
        let compute_index = configured_host_index(&document, "compute").map_err(js_error)?;
        let pointer = PointerInputHandle::new(document.hosts[pointer_index].name.as_str());
        let prepared = compile_named_compute_region(&document, &tree, parsing, pointer.clone())
            .map_err(js_error)?;
        let compute = ComputeCommandHandle::new(prepared.region.clone(), 1);
        let outputs = BrowserOutputHandle::default();
        let registry =
            browser_compute_backend_registry(compute.clone(), outputs.clone(), gpu_available)
                .map_err(js_error)?;
        let mut compute_factory = ComputeHostFactory::new(
            prepared.region.clone(),
            prepared.placement,
            prepared.program.clone(),
            prepared.initializers.clone(),
            registry,
            ComputePlatform::Browser,
        )
        .and_then(|factory| factory.with_retained_outputs(prepared.retained_outputs.clone()))
        .map_err(js_error)?;
        if !backend_override.is_empty() {
            compute_factory = compute_factory
                .with_backend_override(BackendRequest::parse(backend_override).map_err(js_error)?);
        }
        let backend = compute_factory
            .resolved_backend_id(&document.hosts[compute_index].settings)
            .map_err(js_error)?
            .to_string();
        let manifest = gpu_program_manifest(
            prepared.kernel.browser_program(),
            &initializer_values(&prepared.program, &prepared.initializers).map_err(js_error)?,
            &backend,
            &prepared.retained_outputs,
            prepared.timings,
        )?;
        let mut builder = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .config(
                mech_runtime::RuntimeConfig::default()
                    .apply_patch(&document.runtime)
                    .map_err(js_error)?,
            )
            .host_factory(Box::new(PointerHostFactory::new(pointer.clone())))
            .map_err(js_error)?
            .host_factory(Box::new(compute_factory))
            .map_err(js_error)?;
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        if let Some(run) = &document.run {
            for grant in &run.grants {
                builder = builder.run_resource_grant(grant.clone());
            }
        }
        let mut runtime = builder.build().map_err(js_error)?;
        let durability = runtime.config().resident_durability;
        runtime
            .load_compiled_program(prepared.coordinator, durability)
            .map_err(js_error)?;

        Ok(Self {
            runtime,
            pointer,
            compute,
            outputs,
            backend,
            manifest,
            started: false,
            stopped: false,
        })
    }

    #[wasm_bindgen(js_name = computeManifest)]
    pub fn compute_manifest(&self) -> JsValue {
        self.manifest.clone()
    }

    pub fn backend(&self) -> String {
        self.backend.clone()
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        if self.started && !self.stopped {
            return Ok(());
        }
        if self.stopped {
            return Err(error("a stopped mixed compute project cannot be restarted"));
        }
        self.runtime.start_input_drivers().map_err(js_error)?;
        self.started = true;
        self.stopped = false;
        Ok(())
    }

    pub fn frame(
        &mut self,
        x: f64,
        y: f64,
        pressed: bool,
        delta_seconds: f64,
        max_inputs: usize,
    ) -> Result<JsValue, JsValue> {
        if !self.started || self.stopped {
            return Err(error("mixed compute project is not running"));
        }
        if max_inputs == 0 {
            return Err(error("max_inputs must be greater than zero"));
        }
        self.pointer
            .submit(x, y, pressed, delta_seconds)
            .map_err(js_error)?;
        let pending = self.runtime.pending_host_input_count().map_err(js_error)?;
        if pending > 0 {
            self.runtime
                .drain_host_inputs(pending.min(max_inputs))
                .map_err(js_error)?;
        }
        self.compute.take_command_value()
    }

    #[wasm_bindgen(js_name = acknowledgeComputeCommand)]
    pub fn acknowledge_compute_command(&self, dispatch_token: &str) -> Result<(), JsValue> {
        let token = self.compute.validate_token(dispatch_token)?;
        self.compute.acknowledge(token)
    }

    #[wasm_bindgen(js_name = completeComputeCommand)]
    pub fn complete_compute_command(
        &self,
        dispatch_token: &str,
        outputs: Array,
    ) -> Result<(), JsValue> {
        let token = self.compute.validate_token(dispatch_token)?;
        self.compute
            .complete(token, completed_outputs_from_js(outputs)?)
    }

    #[wasm_bindgen(js_name = rejectComputeCommand)]
    pub fn reject_compute_command(
        &self,
        dispatch_token: &str,
        reason: &str,
    ) -> Result<(), JsValue> {
        let token = self.compute.validate_token(dispatch_token)?;
        self.compute.reject(token, reason)
    }

    #[wasm_bindgen(js_name = rejectIntegrityComputeCommand)]
    pub fn reject_integrity_compute_command(
        &self,
        dispatch_token: &str,
        constraint: &str,
        instance: u32,
    ) -> Result<(), JsValue> {
        let token = self.compute.validate_token(dispatch_token)?;
        self.compute.reject_integrity(token, constraint, instance)
    }

    #[wasm_bindgen(js_name = cpuOutput)]
    pub fn cpu_output(&self, name: &str) -> Result<Float32Array, JsValue> {
        let values = self.outputs.output(name).map_err(js_error)?;
        Ok(Float32Array::from(values.as_slice()))
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        if self.stopped {
            return Ok(());
        }
        self.runtime.shutdown().map_err(js_error)?;
        self.started = false;
        self.stopped = true;
        Ok(())
    }
}

pub(crate) fn completed_outputs_from_js(
    outputs: Array,
) -> Result<BTreeMap<String, Vec<f32>>, JsValue> {
    let mut completed = BTreeMap::new();
    for value in outputs.iter() {
        let name = Reflect::get(&value, &JsValue::from_str("name"))?
            .as_string()
            .ok_or_else(|| error("completed compute output is missing its name"))?;
        let values = Reflect::get(&value, &JsValue::from_str("values"))?;
        if !values.is_instance_of::<Float32Array>() {
            return Err(error(format!(
                "completed compute output `{name}` must contain Float32Array values"
            )));
        }
        if completed
            .insert(name.clone(), Float32Array::new(&values).to_vec())
            .is_some()
        {
            return Err(error(format!(
                "completed compute output `{name}` was supplied more than once"
            )));
        }
    }
    Ok(completed)
}

#[derive(Debug)]
pub(crate) struct PreparedComputeRegion {
    pub(crate) region: String,
    pub(crate) placement: mech_core::ComputePlacement,
    pub(crate) coordinator: ProgramArtifact,
    pub(crate) program: ComputeProgram,
    pub(crate) initializers: ComputeInitializerSet,
    pub(crate) retained_outputs: BTreeSet<String>,
    pub(crate) kernel: PreparedGpuKernel,
    pub(crate) timings: CompileTimings,
}

#[derive(Clone)]
pub(crate) struct BrowserComputeBridge {
    command: ComputeCommandHandle,
    manifest: JsValue,
    backend: String,
    physical_revision: String,
    host_state: ComputeHostStateSnapshotHandle,
}

impl BrowserComputeBridge {
    pub(crate) fn manifest(&self) -> JsValue {
        self.manifest.clone()
    }

    pub(crate) fn backend(&self) -> String {
        self.backend.clone()
    }

    pub(crate) fn generation(&self) -> u64 {
        self.command.generation()
    }

    pub(crate) fn physical_revision(&self) -> &str {
        &self.physical_revision
    }

    pub(crate) fn host_state_snapshot(&self) -> &ComputeHostStateSnapshotHandle {
        &self.host_state
    }

    pub(crate) fn ensure_source_replacement_ready(&self) -> MResult<()> {
        self.command.ensure_source_replacement_ready()
    }

    pub(crate) fn take_command(&self) -> Result<JsValue, JsValue> {
        self.command.take_command_value()
    }

    pub(crate) fn validate_token(&self, token: &str) -> Result<ComputeDispatchToken, JsValue> {
        self.command.validate_token(token)
    }

    pub(crate) fn acknowledge(&self, token: ComputeDispatchToken) -> Result<(), JsValue> {
        self.command.acknowledge(token)
    }

    pub(crate) fn complete(
        &self,
        token: ComputeDispatchToken,
        outputs: BTreeMap<String, Vec<f32>>,
    ) -> Result<(), JsValue> {
        self.command.complete(token, outputs)
    }

    pub(crate) fn reject(&self, token: ComputeDispatchToken, reason: &str) -> Result<(), JsValue> {
        self.command.reject(token, reason)
    }

    pub(crate) fn reject_integrity(
        &self,
        token: ComputeDispatchToken,
        constraint: &str,
        instance: u32,
    ) -> Result<(), JsValue> {
        self.command.reject_integrity(token, constraint, instance)
    }
}

pub(crate) struct PreparedBrowserComputeHost {
    pub(crate) factory: ComputeHostFactory,
    pub(crate) coordinator: ProgramArtifact,
    pub(crate) bridge: BrowserComputeBridge,
}

pub(crate) fn prepare_browser_compute_host(
    document: &MechConfigDocument,
    prepared: PreparedComputeRegion,
    gpu_available: bool,
    generation: u64,
    previous: Option<&BrowserComputeBridge>,
) -> MResult<PreparedBrowserComputeHost> {
    let compute_index = configured_host_index(document, "compute")?;
    let command = ComputeCommandHandle::new(prepared.region.clone(), generation);
    let registry = browser_resident_compute_backend_registry(command.clone(), gpu_available)?;
    let mut factory = ComputeHostFactory::new(
        prepared.region.clone(),
        prepared.placement,
        prepared.program.clone(),
        prepared.initializers.clone(),
        registry,
        ComputePlatform::Browser,
    )?
    .with_retained_outputs(prepared.retained_outputs.clone())?;
    let backend = factory
        .resolved_backend_id(&document.hosts[compute_index].settings)?
        .to_string();
    let manifest = gpu_program_manifest(
        prepared.kernel.browser_program(),
        &initializer_values(&prepared.program, &prepared.initializers)?,
        &backend,
        &prepared.retained_outputs,
        prepared.timings,
    )
    .map_err(|failure| mixed_error(format!("browser compute manifest failed: {failure:?}")))?;
    let physical_revision = Reflect::get(&manifest, &JsValue::from_str("physicalRevision"))
        .map_err(|failure| {
            mixed_error(format!(
                "browser compute manifest revision could not be read: {failure:?}"
            ))
        })?
        .as_string()
        .ok_or_else(|| mixed_error("browser compute manifest has no physical revision"))?;
    if let Some(previous) = previous.filter(|previous| {
        previous.backend == backend && previous.physical_revision() == physical_revision
    }) {
        let resume = previous
            .host_state_snapshot()
            .snapshot_retained(&prepared.retained_outputs)?
            .ok_or_else(|| {
                mixed_error(
                    "compatible browser compute state was retired before source replacement",
                )
            })?;
        factory = factory.with_resume_state(resume);
    }
    let host_state = factory.state_snapshot_handle();
    Ok(PreparedBrowserComputeHost {
        factory,
        coordinator: prepared.coordinator,
        bridge: BrowserComputeBridge {
            command,
            manifest,
            backend,
            physical_revision,
            host_state,
        },
    })
}

#[derive(Debug)]
pub(crate) enum PreparedGpuKernel {
    Elementwise(ElementwiseKernel),
    FixedShape(FixedShapeKernel),
}

impl PreparedGpuKernel {
    fn compute_program(&self) -> &ComputeProgram {
        match self {
            Self::Elementwise(program) => program.compute_program(),
            Self::FixedShape(program) => program.compute_program(),
        }
    }

    pub(crate) fn browser_program(&self) -> BrowserGpuProgram<'_> {
        match self {
            Self::Elementwise(program) => BrowserGpuProgram::Elementwise(program),
            Self::FixedShape(program) => BrowserGpuProgram::FixedShape(program),
        }
    }
}

fn compile_named_compute_region(
    document: &MechConfigDocument,
    tree: &Program,
    parsing: f64,
    pointer: PointerInputHandle,
) -> MResult<PreparedComputeRegion> {
    let compiler_started = Instant::now();
    let mut builder = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .host_factory(Box::new(PointerHostFactory::new(pointer)))?;
    for host in document
        .hosts
        .iter()
        .filter(|host| host.provider != "compute")
    {
        builder = builder.host_instance(host.clone());
    }
    if let Some(run) = &document.run {
        for grant in &run.grants {
            builder = builder.run_resource_grant(grant.clone());
        }
    }
    let mut compiler = builder.build_compiler()?;
    let catalog_setup = milliseconds(compiler_started);
    prepare_compute_region(&mut compiler, tree, parsing, catalog_setup)
}

pub(crate) fn prepare_compute_region(
    compiler: &mut mech_runtime::ProgramCompiler,
    tree: &Program,
    parsing: f64,
    catalog_setup: f64,
) -> MResult<PreparedComputeRegion> {
    let artifact_started = Instant::now();
    let mixed = compiler.compile_mixed_tree(tree)?;
    let artifact_compilation = milliseconds(artifact_started);
    let lowering_started = Instant::now();
    let initial_values =
        initializer_values_from_interface(&mixed.compute.interface, &mixed.compute.initializers)?;
    let mut activation_values = initial_values.clone();
    activation_values.extend(
        mixed
            .activation_inputs
            .iter()
            .map(|(name, value)| (name.clone(), value_elements(value))),
    );
    let kernel = match lower_elementwise_compute_program(&mixed.compute.artifact) {
        Ok(program) => PreparedGpuKernel::Elementwise(
            ElementwiseKernel::from_compute_program(&program)
                .map_err(|failure| mixed_error(format!("elementwise lowering failed: {failure}")))?,
        ),
        Err(elementwise_failure) => PreparedGpuKernel::FixedShape(
            ComputeLowerer
                .compile_broadcast(&mixed.compute.artifact, &activation_values)
                .map_err(|fixed_failure| {
                    mixed_error(format!(
                        "compute lowering failed for both portable kernels; elementwise: {elementwise_failure}; fixed-shape: {fixed_failure}",
                    ))
                })?,
        ),
    };
    let program = kernel.compute_program().clone();
    let gpu_lowering = milliseconds(lowering_started);
    let input_started = Instant::now();
    initializer_values(&program, &mixed.compute.initializers)?;
    let input_capture = milliseconds(input_started);
    Ok(PreparedComputeRegion {
        region: mixed.compute.declaration.name.to_string(),
        placement: mixed.compute.declaration.placement,
        coordinator: mixed.coordinator.into_artifact(),
        program,
        initializers: mixed.compute.initializers,
        retained_outputs: mixed.retained_outputs,
        kernel,
        timings: CompileTimings {
            catalog_setup,
            parsing,
            artifact_compilation,
            gpu_lowering,
            input_capture,
        },
    })
}

fn initializer_values_from_interface(
    interface: &mech_compute::ComputeRegionInterface,
    initializers: &ComputeInitializerSet,
) -> MResult<BTreeMap<String, Vec<f32>>> {
    interface
        .inputs
        .iter()
        .map(|port| {
            let value = initializers.get(port.id).ok_or_else(|| {
                mixed_error(format!("compute input `{}` has no initializer", port.name))
            })?;
            let value = port
                .normalize_value(value.clone())
                .map_err(|failure| mixed_error(failure.to_string()))?;
            Ok((port.name.to_string(), value_elements(&value)))
        })
        .collect()
}

fn configured_host_index(document: &MechConfigDocument, provider: &str) -> MResult<usize> {
    let hosts = document
        .hosts
        .iter()
        .enumerate()
        .filter(|(_, host)| host.provider == provider)
        .collect::<Vec<_>>();
    if hosts.len() != 1 {
        return Err(mixed_error(format!(
            "mixed browser projects require exactly one `{provider}` host, found {}",
            hosts.len()
        )));
    }
    Ok(hosts[0].0)
}

pub(crate) fn initializer_values(
    program: &ComputeProgram,
    initializers: &ComputeInitializerSet,
) -> MResult<BTreeMap<String, Vec<f32>>> {
    program
        .interface()
        .inputs
        .iter()
        .map(|port| {
            let value = initializers.get(port.id).ok_or_else(|| {
                mixed_error(format!("compute input `{}` has no initializer", port.name))
            })?;
            let value = port
                .normalize_value(value.clone())
                .map_err(|failure| mixed_error(failure.to_string()))?;
            Ok((port.name.to_string(), value_elements(&value)))
        })
        .collect()
}

#[derive(Clone, Debug)]
struct PointerInputHandle {
    base_uri: Arc<str>,
    state: Arc<Mutex<PointerDriverState>>,
}

#[derive(Debug, Default)]
struct PointerDriverState {
    ingress: Option<RuntimeIngress>,
    pulse: u64,
    live: bool,
}

impl PointerInputHandle {
    fn new(instance: impl AsRef<str>) -> Self {
        Self {
            base_uri: format!("pointer://{}/frame", instance.as_ref()).into(),
            state: Arc::new(Mutex::new(PointerDriverState::default())),
        }
    }

    fn submit(&self, x: f64, y: f64, pressed: bool, delta_seconds: f64) -> MResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| mixed_error("pointer input state lock is poisoned"))?;
        if !state.live {
            return Err(mixed_error("pointer input host is not running"));
        }
        state.pulse = state.pulse.saturating_add(1);
        let pulse = state.pulse;
        let ingress = state
            .ingress
            .clone()
            .ok_or_else(|| mixed_error("pointer input host is not attached"))?;
        drop(state);
        ingress.submit(RuntimeHostInput::new(vec![
            pointer_update(
                &self.base_uri,
                "pulse",
                RuntimeHostInputValue::F64(pulse as f64),
            )?,
            pointer_update(
                &self.base_uri,
                "position",
                RuntimeHostInputValue::F32Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![x as f32, y as f32],
                },
            )?,
            pointer_update(
                &self.base_uri,
                "pressed",
                RuntimeHostInputValue::F32(f32::from(pressed)),
            )?,
            pointer_update(
                &self.base_uri,
                "delta-seconds",
                RuntimeHostInputValue::F32(delta_seconds as f32),
            )?,
        ])?)
    }
}

fn pointer_update(
    base_uri: &str,
    path: &str,
    value: RuntimeHostInputValue,
) -> MResult<RuntimeHostInputUpdate> {
    Ok(RuntimeHostInputUpdate {
        source: RuntimeHostInputSource::new(base_uri, path)?,
        value,
    })
}

#[derive(Debug)]
struct PointerHostFactory {
    handle: PointerInputHandle,
    manifest: HostManifestConfig,
}

impl PointerHostFactory {
    fn new(handle: PointerInputHandle) -> Self {
        Self {
            handle,
            manifest: HostManifestConfig {
                provider: "pointer".to_owned(),
                contexts: vec![HostContextManifest {
                    name: "frame".to_owned(),
                    base_uri_template: "pointer://{instance}/frame".to_owned(),
                    operations: vec!["read".to_owned()],
                }],
            },
        }
    }
}

impl RuntimeHostFactory for PointerHostFactory {
    fn provider_name(&self) -> &str {
        "pointer"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        match settings {
            ConfigValue::Map(map) if map.is_empty() => Ok(()),
            _ => Err(mixed_error("pointer host settings must be an empty map")),
        }
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(PointerResourceProvider {
                instance: instance_name.to_owned(),
            })],
            input_drivers: vec![Box::new(PointerInputDriver {
                instance: instance_name.to_owned(),
                state: self.handle.state.clone(),
            })],
        })
    }
}

#[derive(Debug)]
struct PointerResourceProvider {
    instance: String,
}

impl PointerResourceProvider {
    fn base(&self) -> String {
        format!("pointer://{}/frame", self.instance)
    }
    fn value(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != self.base() || !POINTER_PATHS.contains(&request.path.as_str()) {
            return Err(mixed_error(format!(
                "unknown pointer input `{}/{}`",
                request.base_uri, request.path
            )));
        }
        if request.path == "position" {
            RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 1,
                values: vec![0.0, 0.0],
            }
            .into_mech_value()
        } else if request.path == "pulse" {
            RuntimeHostInputValue::F64(0.0).into_mech_value()
        } else {
            RuntimeHostInputValue::F32(0.0).into_mech_value()
        }
    }
}

impl RuntimeResourceProvider for PointerResourceProvider {
    fn scheme(&self) -> &str {
        "pointer"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }
    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.value(request)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.value(request)
    }
}

#[derive(Debug)]
struct PointerInputDriver {
    instance: String,
    state: Arc<Mutex<PointerDriverState>>,
}

impl RuntimeHostInputDriver for PointerInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == format!("pointer://{}/frame", self.instance)
            && POINTER_PATHS.contains(&source.path())
    }
    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| mixed_error("pointer input state lock is poisoned"))?
            .ingress = Some(ingress);
        Ok(())
    }
    fn start(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| mixed_error("pointer input state lock is poisoned"))?
            .live = true;
        Ok(())
    }
    fn stop(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| mixed_error("pointer input state lock is poisoned"))?
            .live = false;
        Ok(())
    }
    fn is_live(&self) -> bool {
        self.state.lock().map(|state| state.live).unwrap_or(false)
    }
}

#[derive(Clone)]
pub(crate) struct ComputeCommandHandle {
    pub(crate) region: String,
    state: Arc<Mutex<ComputeCommandState>>,
}

struct ComputeCommandState {
    generation: u64,
    next_dispatch_id: u64,
    phase: ComputeCommandPhase,
    completion_program: Option<ComputeProgram>,
    completion_target: Option<Arc<dyn ComputeCompletionTarget>>,
    fault_count: u64,
}

impl ComputeCommandState {
    fn new(generation: u64) -> Self {
        Self {
            generation,
            next_dispatch_id: 0,
            phase: ComputeCommandPhase::Idle,
            completion_program: None,
            completion_target: None,
            fault_count: 0,
        }
    }
}

impl std::fmt::Debug for ComputeCommandHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeCommandHandle")
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) struct ComputeDispatchToken {
    generation: u64,
    dispatch: u64,
}

impl std::fmt::Display for ComputeDispatchToken {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{}:{}", self.generation, self.dispatch)
    }
}

impl std::str::FromStr for ComputeDispatchToken {
    type Err = &'static str;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        let (generation, dispatch) = value.split_once(':').ok_or("missing token separator")?;
        if generation.is_empty() || dispatch.is_empty() || dispatch.contains(':') {
            return Err("invalid token fields");
        }
        let generation = generation.parse().map_err(|_| "invalid generation")?;
        let dispatch = dispatch.parse().map_err(|_| "invalid dispatch")?;
        if generation == 0 || dispatch == 0 || value != format!("{generation}:{dispatch}") {
            return Err("token is not canonical");
        }
        Ok(Self {
            generation,
            dispatch,
        })
    }
}

#[derive(Clone, Debug)]
struct ComputeCompletionRequest {
    token: ComputeDispatchToken,
    outputs: BTreeSet<ComputePortId>,
    logical_turn: u128,
}

#[derive(Clone, Debug)]
enum ComputeCommandPhase {
    Idle,
    CpuReserved,
    Queued(Arc<ComputeCommandData>),
    Serializing(Arc<ComputeCommandData>),
    Claimed(ComputeCompletionRequest),
    Completing(ComputeCompletionRequest),
    Terminal(Box<str>),
}

#[derive(Clone, Debug)]
pub(crate) struct ComputeCommandData {
    pub(crate) changed_inputs: BTreeMap<String, Arc<[f32]>>,
    pub(crate) completed_outputs: BTreeMap<String, Arc<[f32]>>,
    pub(crate) dispatch_token: Option<ComputeDispatchToken>,
    pub(crate) requested_outputs: BTreeSet<String>,
    pub(crate) acknowledgement_required: bool,
    completion_request: Option<ComputeCompletionRequest>,
}

impl ComputeCommandHandle {
    pub(crate) fn new(region: String, generation: u64) -> Self {
        debug_assert!(generation > 0);
        Self {
            region,
            state: Arc::new(Mutex::new(ComputeCommandState::new(generation))),
        }
    }

    pub(crate) fn generation(&self) -> u64 {
        self.state.lock().map(|state| state.generation).unwrap_or(0)
    }

    fn ensure_source_replacement_ready(&self) -> MResult<()> {
        let state = self
            .state
            .lock()
            .map_err(|_| mixed_error("compute command state lock is poisoned"))?;
        let phase = match &state.phase {
            ComputeCommandPhase::CpuReserved => Some("cpu-reserved"),
            ComputeCommandPhase::Queued(_) => Some("queued"),
            ComputeCommandPhase::Serializing(_) => Some("serializing"),
            ComputeCommandPhase::Claimed(_) => Some("claimed"),
            ComputeCommandPhase::Completing(_) => Some("completing"),
            ComputeCommandPhase::Idle | ComputeCommandPhase::Terminal(_) => None,
        };
        match phase {
            Some(phase) => Err(MechError::new(
                ComputeSourceReplacementBusy {
                    region: self.region.clone(),
                    phase,
                },
                None,
            )),
            None => Ok(()),
        }
    }

    #[cfg(test)]
    pub(crate) fn take_command_data(&self) -> Result<Option<ComputeCommandData>, JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        let ComputeCommandPhase::Queued(command) = &state.phase else {
            return Ok(None);
        };
        let command = Arc::clone(command);
        state.phase = claimed_phase(&command);
        Ok(Some((*command).clone()))
    }

    pub(crate) fn take_command_value(&self) -> Result<JsValue, JsValue> {
        let Some(command) = self.lease_command()? else {
            return Ok(JsValue::NULL);
        };
        match compute_command_value(&self.region, &command) {
            Ok(value) => {
                self.commit_delivery(&command)?;
                Ok(value)
            }
            Err(failure) => {
                self.rollback_delivery(command);
                Err(failure)
            }
        }
    }

    fn lease_command(&self) -> Result<Option<Arc<ComputeCommandData>>, JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        let ComputeCommandPhase::Queued(command) = &state.phase else {
            return Ok(None);
        };
        let command = Arc::clone(command);
        state.phase = ComputeCommandPhase::Serializing(Arc::clone(&command));
        Ok(Some(command))
    }

    fn commit_delivery(&self, command: &Arc<ComputeCommandData>) -> Result<(), JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        if !matches!(&state.phase, ComputeCommandPhase::Serializing(current) if Arc::ptr_eq(current, command))
        {
            return Err(error(
                "compute command changed while it was being serialized",
            ));
        }
        state.phase = claimed_phase(command);
        Ok(())
    }

    fn rollback_delivery(&self, command: Arc<ComputeCommandData>) {
        if let Ok(mut state) = self.state.lock() {
            if matches!(&state.phase, ComputeCommandPhase::Serializing(current) if Arc::ptr_eq(current, &command))
            {
                state.phase = ComputeCommandPhase::Queued(command);
            }
        }
    }

    fn reserve_cpu(&self) -> Result<(), ComputeExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            browser_execution_error(CPU_SCALAR_BACKEND, "dispatch", "command lock is poisoned")
        })?;
        ensure_command_slot_available(&state, CPU_SCALAR_BACKEND)?;
        state.phase = ComputeCommandPhase::CpuReserved;
        Ok(())
    }

    fn cancel_cpu_reservation(&self) {
        if let Ok(mut state) = self.state.lock() {
            if matches!(state.phase, ComputeCommandPhase::CpuReserved) {
                state.phase = ComputeCommandPhase::Idle;
            }
        }
    }

    fn fail_cpu(&self, failure: &ComputeExecutionError) {
        if let Ok(mut state) = self.state.lock() {
            if matches!(state.phase, ComputeCommandPhase::CpuReserved) {
                state.phase = if failure.state_advanced {
                    ComputeCommandPhase::Terminal(failure.to_string().into_boxed_str())
                } else {
                    ComputeCommandPhase::Idle
                };
            }
        }
    }

    fn commit_cpu(
        &self,
        changed_inputs: &mut BTreeMap<String, Vec<f32>>,
        completed_outputs: &mut BTreeMap<String, Vec<f32>>,
    ) -> Result<(), ComputeExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            browser_execution_error(CPU_SCALAR_BACKEND, "dispatch", "command lock is poisoned")
        })?;
        if !matches!(state.phase, ComputeCommandPhase::CpuReserved) {
            return Err(browser_execution_error(
                CPU_SCALAR_BACKEND,
                "dispatch",
                "the browser CPU command slot was not reserved",
            ));
        }
        state.phase = ComputeCommandPhase::Queued(Arc::new(ComputeCommandData {
            changed_inputs: take_shared_values(changed_inputs),
            completed_outputs: take_shared_values(completed_outputs),
            dispatch_token: None,
            requested_outputs: BTreeSet::new(),
            acknowledgement_required: false,
            completion_request: None,
        }));
        Ok(())
    }

    fn queue_wgpu(
        &self,
        changed_inputs: &mut BTreeMap<String, Vec<f32>>,
        request: &ComputeDispatchRequest,
    ) -> Result<(), ComputeExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            browser_execution_error(WGPU_BACKEND, "dispatch", "command lock is poisoned")
        })?;
        ensure_command_slot_available(&state, WGPU_BACKEND)?;
        let dispatch_id = state.next_dispatch_id.checked_add(1).ok_or_else(|| {
            browser_execution_error(WGPU_BACKEND, "dispatch", "dispatch ID space exhausted")
        })?;
        state.next_dispatch_id = dispatch_id;
        let token = ComputeDispatchToken {
            generation: state.generation,
            dispatch: dispatch_id,
        };
        let requested_outputs = state
            .completion_program
            .as_ref()
            .ok_or_else(|| {
                browser_execution_error(
                    WGPU_BACKEND,
                    "dispatch",
                    "compute completion has no compiled output contract",
                )
            })?
            .interface()
            .outputs
            .iter()
            .filter(|port| request.outputs.contains(&port.id))
            .map(|port| port.name.to_string())
            .collect();
        let completion_request = ComputeCompletionRequest {
            token,
            outputs: request.outputs.clone(),
            logical_turn: request.logical_turn,
        };
        state.phase = ComputeCommandPhase::Queued(Arc::new(ComputeCommandData {
            changed_inputs: take_shared_values(changed_inputs),
            completed_outputs: BTreeMap::new(),
            dispatch_token: Some(token),
            requested_outputs,
            acknowledgement_required: true,
            completion_request: Some(completion_request),
        }));
        Ok(())
    }

    fn configure_completion(&self, program: &ComputeProgram) -> Result<(), ComputeBackendError> {
        let mut state = self.state.lock().map_err(|_| ComputeBackendError {
            backend: BackendId::new(WGPU_BACKEND).expect("static backend ID is valid"),
            operation: "compile",
            detail: "compute command lock is poisoned".into(),
        })?;
        state.completion_program = Some(program.clone());
        Ok(())
    }

    fn bind_completion_target(
        &self,
        target: Arc<dyn ComputeCompletionTarget>,
    ) -> Result<(), ComputeBackendError> {
        let mut state = self.state.lock().map_err(|_| ComputeBackendError {
            backend: BackendId::new(WGPU_BACKEND).expect("static backend ID is valid"),
            operation: "bind completion",
            detail: "compute command lock is poisoned".into(),
        })?;
        state.completion_target = Some(target);
        Ok(())
    }

    pub(crate) fn validate_token(&self, token: &str) -> Result<ComputeDispatchToken, JsValue> {
        let token: ComputeDispatchToken = token.parse().map_err(|detail| {
            error(format!(
                "invalid compute dispatch token `{token}`: {detail}"
            ))
        })?;
        self.validate_token_value(token).map_err(js_execution_error)
    }

    fn validate_token_value(
        &self,
        token: ComputeDispatchToken,
    ) -> Result<ComputeDispatchToken, ComputeExecutionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
        match &state.phase {
            ComputeCommandPhase::Claimed(request) if request.token == token => Ok(token),
            ComputeCommandPhase::Completing(request) if request.token == token => Err(
                command_completion_error(format!("compute dispatch {token} is already completing")),
            ),
            _ => Err(command_completion_error(format!(
                "compute dispatch {token} does not belong to the active runtime command"
            ))),
        }
    }

    pub(crate) fn acknowledge(&self, token: ComputeDispatchToken) -> Result<(), JsValue> {
        self.acknowledge_native(token).map_err(js_execution_error)
    }

    fn acknowledge_native(&self, token: ComputeDispatchToken) -> Result<(), ComputeExecutionError> {
        let request = self.begin_completion(token)?;
        if !request.outputs.is_empty() {
            let required = request.outputs.len();
            self.restore_claimed(request);
            return Err(command_completion_error(format!(
                "compute dispatch {token} requires {} output value(s)",
                required
            )));
        }
        let attempted_turn = request.logical_turn;
        self.publish_completion(
            request,
            ComputeCompletionOutcome::Completed {
                attempted_turn,
                report: self.success_report()?,
                snapshot: ComputeOutputSnapshot::default(),
            },
            false,
        )
    }

    pub(crate) fn complete(
        &self,
        token: ComputeDispatchToken,
        outputs: BTreeMap<String, Vec<f32>>,
    ) -> Result<(), JsValue> {
        self.complete_native(token, outputs)
            .map_err(js_execution_error)
    }

    fn complete_native(
        &self,
        token: ComputeDispatchToken,
        outputs: BTreeMap<String, Vec<f32>>,
    ) -> Result<(), ComputeExecutionError> {
        let program = self.completion_program()?;
        let request = self.begin_completion(token)?;
        let snapshot = match browser_output_snapshot(&program, &request.outputs, outputs) {
            Ok(snapshot) => snapshot,
            Err(failure) => {
                self.restore_claimed(request);
                return Err(failure);
            }
        };
        let attempted_turn = request.logical_turn;
        self.publish_completion(
            request,
            ComputeCompletionOutcome::Completed {
                attempted_turn,
                report: self.success_report()?,
                snapshot,
            },
            false,
        )
    }

    pub(crate) fn reject_integrity(
        &self,
        token: ComputeDispatchToken,
        constraint: &str,
        instance: u32,
    ) -> Result<(), JsValue> {
        self.reject_integrity_native(token, constraint, instance)
            .map_err(js_execution_error)
    }

    fn reject_integrity_native(
        &self,
        token: ComputeDispatchToken,
        constraint: &str,
        instance: u32,
    ) -> Result<(), ComputeExecutionError> {
        let request = self.begin_completion(token)?;
        let report = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
            state.fault_count = state.fault_count.saturating_add(1);
            ComputeDispatchReport {
                completed_turns: 0,
                fault_count: state.fault_count,
                last_fault: Some(ComputeFaultEvidence {
                    attempted_turn: request.logical_turn,
                    constraint: constraint.to_owned().into_boxed_str(),
                    detail: format!("candidate rejected at batch instance {instance}")
                        .into_boxed_str(),
                }),
                ..Default::default()
            }
        };
        let attempted_turn = request.logical_turn;
        self.publish_completion(
            request,
            ComputeCompletionOutcome::IntegrityRejected {
                attempted_turn,
                report,
            },
            false,
        )
    }

    pub(crate) fn reject(&self, token: ComputeDispatchToken, reason: &str) -> Result<(), JsValue> {
        self.reject_native(token, reason)
            .map_err(js_execution_error)
    }

    fn reject_native(
        &self,
        token: ComputeDispatchToken,
        reason: &str,
    ) -> Result<(), ComputeExecutionError> {
        let request = self.begin_completion(token)?;
        self.publish_completion(
            request.clone(),
            ComputeCompletionOutcome::TransportFailed {
                attempted_turn: request.logical_turn,
                reason: reason.to_owned().into_boxed_str(),
            },
            true,
        )
    }

    fn begin_completion(
        &self,
        token: ComputeDispatchToken,
    ) -> Result<ComputeCompletionRequest, ComputeExecutionError> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
        let ComputeCommandPhase::Claimed(request) = &state.phase else {
            return Err(command_completion_error(format!(
                "compute dispatch {token} is not awaiting acknowledgement"
            )));
        };
        if request.token != token {
            return Err(command_completion_error(format!(
                "compute dispatch {token} does not own the active command"
            )));
        }
        let request = request.clone();
        state.phase = ComputeCommandPhase::Completing(request.clone());
        Ok(request)
    }

    fn restore_claimed(&self, request: ComputeCompletionRequest) {
        if let Ok(mut state) = self.state.lock() {
            if matches!(&state.phase, ComputeCommandPhase::Completing(current) if current.token == request.token)
            {
                state.phase = ComputeCommandPhase::Claimed(request);
            }
        }
    }

    fn publish_completion(
        &self,
        request: ComputeCompletionRequest,
        outcome: ComputeCompletionOutcome,
        terminal: bool,
    ) -> Result<(), ComputeExecutionError> {
        let target = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
            let Some(target) = state.completion_target.clone() else {
                state.phase = ComputeCommandPhase::Terminal(
                    "compute completion has no resident publication target".into(),
                );
                return Err(command_completion_error(
                    "compute completion has no resident publication target",
                ));
            };
            target
        };
        let result = target.complete(outcome);
        let mut state = self
            .state
            .lock()
            .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
        if !matches!(&state.phase, ComputeCommandPhase::Completing(current) if current.token == request.token)
        {
            return Err(command_completion_error(
                "compute completion ownership changed during publication",
            ));
        }
        match result {
            Ok(()) if !terminal => {
                state.phase = ComputeCommandPhase::Idle;
                Ok(())
            }
            Ok(()) => {
                state.phase =
                    ComputeCommandPhase::Terminal("browser compute transport failed".into());
                Ok(())
            }
            Err(failure) => {
                state.phase = ComputeCommandPhase::Terminal(failure.to_string().into_boxed_str());
                Err(failure)
            }
        }
    }

    fn completion_program(&self) -> Result<ComputeProgram, ComputeExecutionError> {
        self.state
            .lock()
            .map_err(|_| command_completion_error("compute command state lock is poisoned"))?
            .completion_program
            .clone()
            .ok_or_else(|| {
                command_completion_error("compute completion has no compiled output contract")
            })
    }

    fn success_report(&self) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        let state = self
            .state
            .lock()
            .map_err(|_| command_completion_error("compute command state lock is poisoned"))?;
        Ok(ComputeDispatchReport {
            completed_turns: 1,
            fault_count: state.fault_count,
            ..Default::default()
        })
    }
}

fn take_shared_values(source: &mut BTreeMap<String, Vec<f32>>) -> BTreeMap<String, Arc<[f32]>> {
    std::mem::take(source)
        .into_iter()
        .map(|(name, values)| (name, Arc::from(values)))
        .collect()
}

fn claimed_phase(command: &ComputeCommandData) -> ComputeCommandPhase {
    match &command.completion_request {
        Some(request) => ComputeCommandPhase::Claimed(request.clone()),
        None => ComputeCommandPhase::Idle,
    }
}

fn ensure_command_slot_available(
    state: &ComputeCommandState,
    backend: &str,
) -> Result<(), ComputeExecutionError> {
    if let ComputeCommandPhase::Terminal(reason) = &state.phase {
        return Err(browser_execution_error(
            backend,
            "dispatch",
            format!("browser compute session is terminal: {reason}"),
        ));
    }
    if !matches!(state.phase, ComputeCommandPhase::Idle) {
        return Err(browser_execution_error(
            backend,
            "dispatch",
            "the previous browser command has not been consumed",
        ));
    }
    Ok(())
}

pub(crate) fn compute_command_value(
    region: &str,
    command: &ComputeCommandData,
) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(&value, "region", region)?;
    set(&value, "dispatch", true)?;
    set(
        &value,
        "acknowledgementRequired",
        command.acknowledgement_required,
    )?;
    if let Some(dispatch_token) = command.dispatch_token {
        set(&value, "dispatchToken", dispatch_token.to_string())?;
    }
    set(
        &value,
        "requestedOutputs",
        Array::from_iter(command.requested_outputs.iter().map(JsValue::from)),
    )?;
    let inputs = Array::new();
    for (name, values) in &command.changed_inputs {
        let input = Object::new();
        set(&input, "name", name)?;
        set(&input, "values", Float32Array::from(values.as_ref()))?;
        inputs.push(&input);
    }
    set(&value, "inputs", inputs)?;
    let outputs = Array::new();
    for (name, values) in &command.completed_outputs {
        let output = Object::new();
        set(&output, "name", name)?;
        set(&output, "values", Float32Array::from(values.as_ref()))?;
        outputs.push(&output);
    }
    set(&value, "outputs", outputs)?;
    Ok(value.into())
}

#[derive(Clone, Debug, Default)]
pub(crate) struct BrowserOutputHandle {
    values: Arc<Mutex<BTreeMap<String, Vec<f32>>>>,
}

impl BrowserOutputHandle {
    fn publish(
        &self,
        program: &ComputeProgram,
        snapshot: &ComputeOutputSnapshot,
    ) -> Result<BTreeMap<String, Vec<f32>>, ComputeExecutionError> {
        let mut candidate = BTreeMap::new();
        let mut completed = BTreeMap::new();
        for port in &program.interface().outputs {
            let value = snapshot.values.get(&port.id).ok_or_else(|| {
                browser_execution_error(
                    CPU_SCALAR_BACKEND,
                    "publish outputs",
                    format!("backend omitted output `{}`", port.name),
                )
            })?;
            let values = value_elements(value);
            let completion_values = if let Some(storage) = program.fixed_shape_storage() {
                let elements_per_instance = port.elements().map_err(|failure| {
                    browser_execution_error(
                        CPU_SCALAR_BACKEND,
                        "publish outputs",
                        format!("output `{}` has an invalid shape: {failure}", port.name),
                    )
                })?;
                let expected_elements = elements_per_instance
                    .checked_mul(storage.instances as usize)
                    .ok_or_else(|| {
                        browser_execution_error(
                            CPU_SCALAR_BACKEND,
                            "publish outputs",
                            format!("output `{}` batch size overflowed", port.name),
                        )
                    })?;
                if values.len() != expected_elements {
                    return Err(browser_execution_error(
                        CPU_SCALAR_BACKEND,
                        "publish outputs",
                        format!(
                            "output `{}` contained {} values; expected {expected_elements}",
                            port.name,
                            values.len(),
                        ),
                    ));
                }
                values[..elements_per_instance].to_vec()
            } else {
                values.clone()
            };
            candidate.insert(port.name.to_string(), values);
            completed.insert(port.name.to_string(), completion_values);
        }
        *self.values.lock().map_err(|_| {
            browser_execution_error(
                CPU_SCALAR_BACKEND,
                "publish outputs",
                "output lock is poisoned",
            )
        })? = candidate;
        // Browser WebGPU only reads back lane zero for fixed-shape batches.
        // Queue the same sampled output contract for the synchronous scalar
        // backend so completion observers never pay for or depend on a full
        // batch transfer.
        Ok(completed)
    }

    fn output(&self, name: &str) -> MResult<Vec<f32>> {
        self.values
            .lock()
            .map_err(|_| mixed_error("browser compute output lock is poisoned"))?
            .get(name)
            .cloned()
            .ok_or_else(|| mixed_error(format!("compute output `{name}` is not available")))
    }
}

pub(crate) fn browser_compute_backend_registry(
    command: ComputeCommandHandle,
    outputs: BrowserOutputHandle,
    gpu_available: bool,
) -> MResult<Arc<ComputeBackendRegistry>> {
    let mut registry = ComputeBackendRegistry::default();
    registry
        .register(Arc::new(BrowserCpuBackendFactory::new(
            command.clone(),
            outputs,
        )))
        .map_err(|failure| mixed_error(failure.to_string()))?;
    registry
        .register(Arc::new(BrowserWgpuBackendFactory::new(
            command,
            gpu_available,
        )))
        .map_err(|failure| mixed_error(failure.to_string()))?;
    Ok(Arc::new(registry))
}

/// Backend registry for a resident browser document.
///
/// Scalar compute is executed and sampled directly by the resident compute
/// host. Only WebGPU needs the browser command transport. Keeping that split
/// here prevents the standalone presentation adapter from turning every CPU
/// dispatch into an all-output readback and JavaScript command.
pub(crate) fn browser_resident_compute_backend_registry(
    command: ComputeCommandHandle,
    gpu_available: bool,
) -> MResult<Arc<ComputeBackendRegistry>> {
    let mut registry = ComputeBackendRegistry::default();
    registry
        .register(Arc::new(CpuScalarBackendFactory::new()))
        .map_err(|failure| mixed_error(failure.to_string()))?;
    registry
        .register(Arc::new(BrowserWgpuBackendFactory::new(
            command,
            gpu_available,
        )))
        .map_err(|failure| mixed_error(failure.to_string()))?;
    Ok(Arc::new(registry))
}

struct BrowserCpuBackendFactory {
    inner: CpuScalarBackendFactory,
    command: ComputeCommandHandle,
    outputs: BrowserOutputHandle,
}

impl BrowserCpuBackendFactory {
    fn new(command: ComputeCommandHandle, outputs: BrowserOutputHandle) -> Self {
        Self {
            inner: CpuScalarBackendFactory::new(),
            command,
            outputs,
        }
    }
}

impl ComputeBackendFactory for BrowserCpuBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        self.inner.descriptor()
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        self.inner.supports(program)
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        Ok(Box::new(BrowserCpuExecutable {
            inner: self.inner.compile(program)?,
            program: program.clone(),
            command: self.command.clone(),
            outputs: self.outputs.clone(),
        }))
    }
}

struct BrowserCpuExecutable {
    inner: Box<dyn ComputeExecutable>,
    program: ComputeProgram,
    command: ComputeCommandHandle,
    outputs: BrowserOutputHandle,
}

impl ComputeExecutable for BrowserCpuExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        Ok(Box::new(BrowserCpuSession {
            inner: self.inner.create_session(initializers)?,
            program: self.program.clone(),
            command: self.command.clone(),
            outputs: self.outputs.clone(),
            changed_inputs: BTreeMap::new(),
        }))
    }
}

struct BrowserCpuSession {
    inner: Box<dyn ComputeSession>,
    program: ComputeProgram,
    command: ComputeCommandHandle,
    outputs: BrowserOutputHandle,
    changed_inputs: BTreeMap<String, Vec<f32>>,
}

impl ComputeSession for BrowserCpuSession {
    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        let mut normalized = Vec::with_capacity(updates.len());
        for update in updates {
            let update = self
                .program
                .normalize_input_update(update.clone())
                .map_err(|failure| {
                    browser_execution_error(
                        CPU_SCALAR_BACKEND,
                        "update inputs",
                        failure.to_string(),
                    )
                })?;
            let port = self
                .program
                .interface()
                .input(update.port)
                .expect("normalized update identifies an input");
            self.changed_inputs
                .insert(port.name.to_string(), value_elements(&update.value));
            normalized.push(update);
        }
        self.inner.update_inputs(&normalized)
    }

    fn dispatch(
        &mut self,
        turns: NonZeroU32,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        self.command.reserve_cpu()?;
        let result = (|| {
            let report = self.inner.dispatch(turns)?;
            if report.completed_turns == 0 {
                // Integrity rejection preserves the backend's previously
                // accepted state. There is no new snapshot or browser
                // completion to publish for this attempted turn.
                self.changed_inputs.clear();
                self.command.cancel_cpu_reservation();
                return Ok(report);
            }
            let snapshot = self
                .inner
                .read_outputs(&ComputeOutputSelection::All)
                .map_err(ComputeExecutionError::after_state_advance)?;
            let mut completed_outputs = self
                .outputs
                .publish(&self.program, &snapshot)
                .map_err(ComputeExecutionError::after_state_advance)?;
            self.command
                .commit_cpu(&mut self.changed_inputs, &mut completed_outputs)
                .map_err(ComputeExecutionError::after_state_advance)?;
            Ok(report)
        })();
        if let Err(failure) = &result {
            self.command.fail_cpu(failure);
        }
        result
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        self.inner.read_outputs(selection)
    }
}

struct BrowserWgpuBackendFactory {
    descriptor: ComputeBackendDescriptor,
    command: ComputeCommandHandle,
    available: bool,
}

impl BrowserWgpuBackendFactory {
    fn new(command: ComputeCommandHandle, available: bool) -> Self {
        Self {
            descriptor: ComputeBackendDescriptor {
                id: BackendId::new(WGPU_BACKEND).expect("static backend ID is valid"),
                class: BackendClass::Gpu,
                priority: 400,
                capabilities: ComputeBackendCapabilities {
                    elementwise: true,
                    fixed_shape: true,
                    integrity_rejection: true,
                    browser: true,
                    ..Default::default()
                },
            },
            command,
            available,
        }
    }
}

impl ComputeBackendFactory for BrowserWgpuBackendFactory {
    fn descriptor(&self) -> &ComputeBackendDescriptor {
        &self.descriptor
    }

    fn supports(&self, program: &ComputeProgram) -> Result<(), ComputeBackendRejection> {
        if !self.available {
            return Err(ComputeBackendRejection {
                backend: self.descriptor.id.clone(),
                reason: "WebGPU is unavailable in this browser".into(),
            });
        }
        let planned = match program.kernel() {
            ComputeKernel::Elementwise(_) => program.elementwise_storage().is_some(),
            ComputeKernel::FixedShape(_) => program.fixed_shape_storage().is_some(),
        };
        if !planned {
            return Err(ComputeBackendRejection {
                backend: self.descriptor.id.clone(),
                reason: "browser wgpu requires a complete physical storage plan".into(),
            });
        }
        Ok(())
    }

    fn compile(
        &self,
        program: &ComputeProgram,
    ) -> Result<Box<dyn ComputeExecutable>, ComputeBackendError> {
        match program.kernel() {
            ComputeKernel::Elementwise(_) => {
                ElementwiseKernel::from_compute_program(program).map(|_| ())
            }
            ComputeKernel::FixedShape(_) => {
                FixedShapeKernel::from_compute_program(program).map(|_| ())
            }
        }
        .map_err(|failure| ComputeBackendError {
            backend: self.descriptor.id.clone(),
            operation: "compile",
            detail: format!("{failure:?}").into(),
        })?;
        Ok(Box::new(BrowserWgpuExecutable {
            backend: self.descriptor.id.clone(),
            program: program.clone(),
            command: self.command.clone(),
        }))
    }
}

struct BrowserWgpuExecutable {
    backend: BackendId,
    program: ComputeProgram,
    command: ComputeCommandHandle,
}

impl ComputeExecutable for BrowserWgpuExecutable {
    fn create_session(
        &self,
        initializers: &ComputeInitializerSet,
    ) -> Result<Box<dyn ComputeSession>, ComputeBackendError> {
        initializer_values(&self.program, initializers).map_err(|failure| ComputeBackendError {
            backend: self.backend.clone(),
            operation: "create session",
            detail: format!("{failure:?}").into(),
        })?;
        self.command.configure_completion(&self.program)?;
        Ok(Box::new(BrowserWgpuSession {
            backend: self.backend.clone(),
            program: self.program.clone(),
            command: self.command.clone(),
            changed_inputs: BTreeMap::new(),
        }))
    }
}

struct BrowserWgpuSession {
    backend: BackendId,
    program: ComputeProgram,
    command: ComputeCommandHandle,
    changed_inputs: BTreeMap<String, Vec<f32>>,
}

impl ComputeSession for BrowserWgpuSession {
    fn bind_completion_target(
        &mut self,
        target: Arc<dyn ComputeCompletionTarget>,
    ) -> Result<(), ComputeBackendError> {
        self.command.bind_completion_target(target)
    }

    fn update_inputs(
        &mut self,
        updates: &[ComputeInputUpdate],
    ) -> Result<(), ComputeExecutionError> {
        for update in updates {
            let update = self
                .program
                .normalize_input_update(update.clone())
                .map_err(|failure| {
                    browser_execution_error(
                        &self.backend.to_string(),
                        "update inputs",
                        failure.to_string(),
                    )
                })?;
            let port = self
                .program
                .interface()
                .input(update.port)
                .expect("normalized update identifies an input");
            self.changed_inputs
                .insert(port.name.to_string(), value_elements(&update.value));
        }
        Ok(())
    }

    fn dispatch(
        &mut self,
        turns: NonZeroU32,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        self.dispatch_requested(turns, &ComputeDispatchRequest::default())
    }

    fn dispatch_requested(
        &mut self,
        turns: NonZeroU32,
        request: &ComputeDispatchRequest,
    ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
        if turns.get() != 1 {
            return Err(browser_execution_error(
                WGPU_BACKEND,
                "dispatch",
                "the browser render bridge accepts one resident turn per frame",
            ));
        }
        self.command.queue_wgpu(&mut self.changed_inputs, request)?;
        Ok(ComputeDispatchReport {
            completed_turns: 0,
            ..Default::default()
        })
    }

    fn read_outputs(
        &mut self,
        selection: &ComputeOutputSelection,
    ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
        let _ = selection;
        Ok(ComputeOutputSnapshot::default())
    }
}

fn browser_output_snapshot(
    program: &ComputeProgram,
    requested: &BTreeSet<ComputePortId>,
    completed: BTreeMap<String, Vec<f32>>,
) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
    if completed.len() != requested.len() {
        let missing = program
            .interface()
            .outputs
            .iter()
            .filter(|port| requested.contains(&port.id))
            .filter(|port| !completed.contains_key(port.name.as_ref()))
            .map(|port| port.name.as_ref())
            .collect::<Vec<_>>();
        return Err(browser_execution_error(
            WGPU_BACKEND,
            "read outputs",
            format!(
                "browser bridge returned {} outputs; expected {}{}",
                completed.len(),
                requested.len(),
                if missing.is_empty() {
                    String::new()
                } else {
                    format!("; missing {}", missing.join(", "))
                },
            ),
        ));
    }
    let mut values = BTreeMap::new();
    for (name, logical) in completed {
        let port = program
            .interface()
            .outputs
            .iter()
            .find(|port| port.name.as_ref() == name)
            .ok_or_else(|| {
                browser_execution_error(
                    WGPU_BACKEND,
                    "read outputs",
                    format!("browser bridge returned undeclared output `{name}`"),
                )
            })?;
        if !requested.contains(&port.id) {
            return Err(browser_execution_error(
                WGPU_BACKEND,
                "read outputs",
                format!("browser bridge returned unrequested output `{name}`"),
            ));
        }
        let expected = port.elements().map_err(|failure| {
            browser_execution_error(WGPU_BACKEND, "read outputs", failure.to_string())
        })?;
        if logical.len() != expected {
            return Err(browser_execution_error(
                WGPU_BACKEND,
                "read outputs",
                format!(
                    "browser bridge returned {} values for `{name}`; expected {expected}",
                    logical.len()
                ),
            ));
        }
        let value = if port.dimensions.is_empty() {
            ComputeValue::ScalarF32(logical[0])
        } else {
            ComputeValue::TensorF32 {
                dimensions: port.dimensions.clone(),
                // The browser bridge canonicalizes every physical backend
                // buffer before it crosses back into the portable runtime.
                layout: TensorLayout::RowMajor,
                values: logical.into(),
            }
        };
        values.insert(port.id, value);
    }
    Ok(ComputeOutputSnapshot { values })
}

fn value_elements(value: &ComputeValue) -> Vec<f32> {
    match value {
        ComputeValue::ScalarF32(value) => vec![*value],
        ComputeValue::TensorF32 { values, .. } => values.to_vec(),
    }
}

fn browser_execution_error(
    backend: &str,
    operation: &'static str,
    detail: impl Into<Box<str>>,
) -> ComputeExecutionError {
    ComputeExecutionError {
        backend: BackendId::new(backend).expect("browser backend ID is valid"),
        operation,
        detail: detail.into(),
        state_advanced: false,
    }
}

fn command_completion_error(detail: impl Into<Box<str>>) -> ComputeExecutionError {
    browser_execution_error(WGPU_BACKEND, "complete command", detail)
}

fn js_execution_error(failure: ComputeExecutionError) -> JsValue {
    error(format!("{}: {}", failure.operation, failure.detail))
}

#[derive(Clone, Debug)]
struct MixedComputeProjectError(String);

impl MechErrorKind for MixedComputeProjectError {
    fn name(&self) -> &str {
        "MixedComputeProjectError"
    }
    fn message(&self) -> String {
        self.0.clone()
    }
}

fn mixed_error(message: impl Into<String>) -> MechError {
    MechError::new(MixedComputeProjectError(message.into()), None).with_compiler_loc()
}

#[derive(Clone, Debug)]
struct ComputeSourceReplacementBusy {
    region: String,
    phase: &'static str,
}

impl MechErrorKind for ComputeSourceReplacementBusy {
    fn name(&self) -> &str {
        "ComputeSourceReplacementBusy"
    }

    fn message(&self) -> String {
        format!(
            "compute region `{}` is {phase}; wait for its current turn to complete before replacing source",
            self.region,
            phase = self.phase,
        )
    }
}

fn milliseconds(started: Instant) -> f64 {
    started.elapsed().as_secs_f64() * 1_000.0
}

fn set(target: &Object, name: &str, value: impl Into<JsValue>) -> Result<(), JsValue> {
    Reflect::set(target, &JsValue::from_str(name), &value.into()).map(|_| ())
}

fn js_error(failure: impl std::fmt::Debug) -> JsValue {
    error(format!("{failure:?}"))
}

fn error(message: impl AsRef<str>) -> JsValue {
    js_sys::Error::new(message.as_ref()).into()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[derive(Default)]
    struct RecordingCompletion {
        outcomes: Mutex<Vec<ComputeCompletionOutcome>>,
    }

    impl ComputeCompletionTarget for RecordingCompletion {
        fn complete(&self, outcome: ComputeCompletionOutcome) -> Result<(), ComputeExecutionError> {
            self.outcomes.lock().unwrap().push(outcome);
            Ok(())
        }
    }

    struct FailingCompletion;

    impl ComputeCompletionTarget for FailingCompletion {
        fn complete(
            &self,
            _outcome: ComputeCompletionOutcome,
        ) -> Result<(), ComputeExecutionError> {
            Err(browser_execution_error(
                WGPU_BACKEND,
                "publish completion",
                "injected completion target failure",
            )
            .after_state_advance())
        }
    }

    struct RejectingCpuSession;

    impl ComputeSession for RejectingCpuSession {
        fn update_inputs(
            &mut self,
            _updates: &[ComputeInputUpdate],
        ) -> Result<(), ComputeExecutionError> {
            Ok(())
        }

        fn dispatch(
            &mut self,
            _turns: NonZeroU32,
        ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
            Ok(ComputeDispatchReport {
                completed_turns: 0,
                fault_count: 1,
                last_fault: Some(ComputeFaultEvidence {
                    attempted_turn: 1,
                    constraint: "finite".into(),
                    detail: "candidate rejected".into(),
                }),
                ..Default::default()
            })
        }

        fn read_outputs(
            &mut self,
            _selection: &ComputeOutputSelection,
        ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
            panic!("a rejected CPU turn must not read or publish outputs")
        }
    }

    struct AdvancedCpuSession;

    impl ComputeSession for AdvancedCpuSession {
        fn update_inputs(
            &mut self,
            _updates: &[ComputeInputUpdate],
        ) -> Result<(), ComputeExecutionError> {
            Ok(())
        }

        fn dispatch(
            &mut self,
            _turns: NonZeroU32,
        ) -> Result<ComputeDispatchReport, ComputeExecutionError> {
            Ok(ComputeDispatchReport {
                completed_turns: 1,
                ..Default::default()
            })
        }

        fn read_outputs(
            &mut self,
            _selection: &ComputeOutputSelection,
        ) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
            // The missing snapshot simulates any fallible readback or output
            // contract failure after the scalar backend accepted the turn.
            Ok(ComputeOutputSnapshot::default())
        }
    }

    const CONFIG: &str = r#"
config := {
  runtime: {
    resident-durability: "volatile"
  }
  hosts: [
    { name: "cursor" provider: "pointer" settings: {} }
    { name: "particles" provider: "compute" settings: { region: "particle-field" backend: "auto" } }
  ]
  run: {
    paths: ["particles.mec"]
    grants: [
      { target: "cursor/frame" operations: ["read"] paths: ["pulse", "position", "pressed", "delta-seconds"] }
      { target: "particles/kernel" operations: ["write"] paths: ["input/force-point", "input/force-strength", "input/dt", "turn"] }
    ]
  }
}
"#;

    const SOURCE: &str = r#"
+> math
@pointer := pointer://cursor/frame{:read(pulse), :read(position), :read(pressed), :read(delta-seconds)}
@particles := compute://particles/kernel{:write(input/force-point), :write(input/force-strength), :write(input/dt), :write(turn)}
pulse := @pointer/pulse
force-point := @pointer/position
force-strength := @pointer/pressed
dt := @pointer/delta-seconds
@particles/input/force-point <- force-point
@particles/input/force-strength <- force-strength
@particles/input/dt <- dt
@particles/turn <- pulse

particle-field @compute
-------------------------------------------------------------------------------
particle-index := 1f32..=4f32
particle-x := math/cos(particle-index)
particle-y := math/sin(particle-index)
~positions := [particle-x; particle-y]
~velocities := [(0f32 - particle-y); particle-x]
force-point := [0f32; 0f32]
force-strength := 0f32
dt := 0.016666667<f32>
offset := force-point - positions
distance-square := offset * offset + 0.018<f32>
pointer-pull := force-strength * (1f32 - distance-square * 0.18<f32>)
acceleration := (0f32 - positions) * 0.435<f32> + offset * pointer-pull
next-velocities := velocities + acceleration * dt
next-positions := positions + next-velocities * dt
velocities = next-velocities
positions = next-positions
(positions, velocities)
"#;

    const SERVED_CONFIG: &str = include_str!("../../../examples/gpu-particles/mech.mcfg");
    const SERVED_SOURCE: &str = include_str!("../../../examples/gpu-particles/particles.mec");

    const FIXED_SHAPE_SOURCE: &str = r#"
+> math
@particles := compute://particles/kernel{:write(input/control), :write(turn)}
lane := 1f32..=1000f32
@particles/input/control <- lane * 0.001<f32>
@particles/turn <- 1

particle-field @compute
-------------------------------------------------------------------------------
transform := [1f32 0f32; 0f32 1f32]
~state := [1f32; 2f32]
control := 0f32
state = transform ** state + [control; 0.25<f32>]
state
"#;

    fn compile_fixture(config: &str, source: &str) -> (MechConfigDocument, PreparedComputeRegion) {
        let document =
            parse_config_document("test.mcfg", config, ConfigProfileOptions::default()).unwrap();
        let tree = mech_syntax::parse(source).unwrap();
        let pointer_index = configured_host_index(&document, "pointer").unwrap();
        let pointer = PointerInputHandle::new(document.hosts[pointer_index].name.as_str());
        let prepared = compile_named_compute_region(&document, &tree, 0.0, pointer).unwrap();
        (document, prepared)
    }

    fn start_runtime_fixture(
        document: &MechConfigDocument,
        prepared: PreparedComputeRegion,
        request: BackendRequest,
        gpu_available: bool,
    ) -> (
        MechRuntime,
        PointerInputHandle,
        ComputeCommandHandle,
        BrowserOutputHandle,
        String,
    ) {
        let pointer_index = configured_host_index(document, "pointer").unwrap();
        let compute_index = configured_host_index(document, "compute").unwrap();
        let pointer = PointerInputHandle::new(document.hosts[pointer_index].name.as_str());
        let compute = ComputeCommandHandle::new(prepared.region.clone(), 1);
        let outputs = BrowserOutputHandle::default();
        let registry =
            browser_compute_backend_registry(compute.clone(), outputs.clone(), gpu_available)
                .unwrap();
        let factory = ComputeHostFactory::new(
            prepared.region,
            prepared.placement,
            prepared.program,
            prepared.initializers,
            registry,
            ComputePlatform::Browser,
        )
        .unwrap()
        .with_retained_outputs(prepared.retained_outputs)
        .unwrap()
        .with_backend_override(request);
        let backend = factory
            .resolved_backend_id(&document.hosts[compute_index].settings)
            .unwrap()
            .to_string();
        let mut builder = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_native_plan_catalog())
            .config(
                mech_runtime::RuntimeConfig::default()
                    .apply_patch(&document.runtime)
                    .unwrap(),
            )
            .host_factory(Box::new(PointerHostFactory::new(pointer.clone())))
            .unwrap()
            .host_factory(Box::new(factory))
            .unwrap();
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        let mut runtime = builder.build().unwrap();
        let durability = runtime.config().resident_durability;
        runtime
            .load_compiled_program(prepared.coordinator, durability)
            .unwrap();
        runtime.start_input_drivers().unwrap();
        (runtime, pointer, compute, outputs, backend)
    }

    #[test]
    #[ignore = "million-particle browser compiler acceptance test"]
    fn served_million_particle_source_compiles_without_bytecode_serialization() {
        let document = parse_config_document(
            "examples/gpu-particles/mech.mcfg",
            SERVED_CONFIG,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        assert_eq!(
            document
                .serve
                .as_ref()
                .and_then(|serve| serve.wasm.as_deref()),
            Some(std::path::Path::new("../../src/wasm/pkg")),
        );
        let pointer_index = configured_host_index(&document, "pointer").unwrap();
        let pointer = PointerInputHandle::new(document.hosts[pointer_index].name.as_str());
        let tree = mech_syntax::parse(SERVED_SOURCE).unwrap();
        let prepared = compile_named_compute_region(&document, &tree, 0.0, pointer).unwrap();
        let inputs = initializer_values(&prepared.program, &prepared.initializers).unwrap();

        assert_eq!(prepared.region, "particle-field");
        let PreparedGpuKernel::Elementwise(kernel) = prepared.kernel else {
            panic!("particle source must select the elementwise kernel")
        };
        assert_eq!(kernel.dispatch_elements(), 2_000_000);
        assert_eq!(inputs["force-point"], vec![0.0, 0.0]);
        assert_eq!(inputs["force-strength"], vec![0.0]);
        assert_eq!(inputs["dt"], vec![0.016666667]);
    }

    #[test]
    fn browser_compiler_selects_the_generic_fixed_shape_kernel_for_matrix_batches() {
        let (_document, prepared) = compile_fixture(CONFIG, FIXED_SHAPE_SOURCE);
        let inputs = initializer_values(&prepared.program, &prepared.initializers).unwrap();
        let PreparedGpuKernel::FixedShape(kernel) = prepared.kernel else {
            panic!("matrix recurrence must select the fixed-shape kernel")
        };

        assert_eq!(kernel.instances(), 1_000);
        assert_eq!(kernel.physical_inputs(&inputs).unwrap()[0].elements, 1_000);
        assert_eq!(kernel.physical_states()[0].elements, 2_000);
        assert!(kernel.wgsl().contains("state_write_"));
    }

    #[test]
    fn browser_compute_lowers_ceil_to_wgsl() {
        let source = SOURCE.replacen(
            "acceleration := (0f32 - positions) * 0.435<f32> + offset * pointer-pull",
            "acceleration := math/ceil((0f32 - positions) * 0.435<f32> + offset * pointer-pull)",
            1,
        );
        let (_document, prepared) = compile_fixture(CONFIG, &source);
        let PreparedGpuKernel::Elementwise(kernel) = prepared.kernel else {
            panic!("elementwise ceil source must select the elementwise kernel")
        };

        assert!(
            kernel.wgsl().contains("ceil("),
            "generated WGSL did not contain ceil:\n{}",
            kernel.wgsl()
        );
    }

    #[test]
    fn one_document_runs_the_coordinator_and_wgpu_bridge_through_the_common_host() {
        let (document, prepared) = compile_fixture(CONFIG, SOURCE);
        let input_names = prepared
            .program
            .interface()
            .inputs
            .iter()
            .map(|port| port.name.as_ref())
            .collect::<BTreeSet<_>>();
        assert!(input_names.contains("force-point"));
        assert!(input_names.contains("force-strength"));
        assert!(input_names.contains("dt"));

        let (mut runtime, pointer, compute, _, backend) =
            start_runtime_fixture(&document, prepared, BackendRequest::Gpu, true);
        assert_eq!(backend, WGPU_BACKEND);
        pointer.submit(0.25, -0.5, true, 1.0 / 60.0).unwrap();
        runtime.drain_host_inputs(1).unwrap();
        let command = compute.take_command_data().unwrap().unwrap();
        assert!(command.acknowledgement_required);
        assert_eq!(command.changed_inputs["force-point"].as_ref(), [0.25, -0.5]);
        assert_eq!(command.changed_inputs["force-strength"].as_ref(), [1.0]);
        let dispatch_token = command.dispatch_token.unwrap();
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Claimed(_)
        ));
        let completed = command
            .requested_outputs
            .iter()
            .map(|name| {
                let elements = compute
                    .state
                    .lock()
                    .unwrap()
                    .completion_program
                    .as_ref()
                    .unwrap()
                    .interface()
                    .outputs
                    .iter()
                    .find(|port| port.name.as_ref() == name)
                    .unwrap()
                    .elements()
                    .unwrap();
                (name.clone(), vec![0.0; elements])
            })
            .collect();
        compute.complete(dispatch_token, completed).unwrap();
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Idle
        ));
    }

    #[test]
    fn auto_selects_cpu_when_compute_gpu_is_unavailable() {
        let (document, prepared) = compile_fixture(CONFIG, SOURCE);
        assert_eq!(
            prepared
                .program
                .interface()
                .outputs
                .iter()
                .map(|output| output.name.as_ref())
                .collect::<Vec<_>>(),
            ["result.0", "result.1"]
        );
        let (mut runtime, pointer, compute, outputs, backend) =
            start_runtime_fixture(&document, prepared, BackendRequest::Auto, false);
        assert_eq!(backend, CPU_SCALAR_BACKEND);

        pointer.submit(0.25, -0.5, true, 1.0 / 60.0).unwrap();
        runtime.drain_host_inputs(1).unwrap();

        let state = compute.take_command_data().unwrap().unwrap();
        assert!(!state.acknowledgement_required);
        assert_eq!(state.dispatch_token, None);
        assert_eq!(state.changed_inputs["force-point"].as_ref(), [0.25, -0.5]);
        assert_eq!(state.changed_inputs["force-strength"].as_ref(), [1.0]);
        let positions = outputs.output("result.0").unwrap();
        assert_eq!(positions.len(), 8);
        assert!(positions.iter().all(|value| value.is_finite()));
        assert_eq!(state.completed_outputs["result.0"].as_ref(), positions);
        assert_eq!(state.completed_outputs["result.1"].len(), 8);
    }

    #[test]
    fn cpu_integrity_rejection_does_not_publish_a_completion_command() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let command = ComputeCommandHandle::new(prepared.region.clone(), 1);
        let outputs = BrowserOutputHandle::default();
        let mut session = BrowserCpuSession {
            inner: Box::new(RejectingCpuSession),
            program: prepared.program,
            command: command.clone(),
            outputs: outputs.clone(),
            changed_inputs: BTreeMap::from([("force-strength".to_owned(), vec![1.0])]),
        };

        let report = session.dispatch(NonZeroU32::new(1).unwrap()).unwrap();

        assert_eq!(report.completed_turns, 0);
        assert_eq!(report.fault_count, 1);
        assert!(session.changed_inputs.is_empty());
        assert!(command.take_command_data().unwrap().is_none());
        assert!(matches!(
            command.state.lock().unwrap().phase,
            ComputeCommandPhase::Idle
        ));
        assert!(outputs.values.lock().unwrap().is_empty());
    }

    #[test]
    fn cpu_publication_failure_reports_that_resident_state_advanced() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let command = ComputeCommandHandle::new(prepared.region.clone(), 1);
        let mut session = BrowserCpuSession {
            inner: Box::new(AdvancedCpuSession),
            program: prepared.program,
            command: command.clone(),
            outputs: BrowserOutputHandle::default(),
            changed_inputs: BTreeMap::new(),
        };

        let error = session.dispatch(NonZeroU32::new(1).unwrap()).unwrap_err();

        assert!(error.state_advanced);
        assert!(command.take_command_data().unwrap().is_none());
        assert!(matches!(
            command.state.lock().unwrap().phase,
            ComputeCommandPhase::Terminal(_)
        ));
    }

    #[test]
    fn wgpu_holds_the_command_slot_until_the_bridge_completes_it() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 1);
        compute.configure_completion(&prepared.program).unwrap();
        let completion = Arc::new(RecordingCompletion::default());
        compute.bind_completion_target(completion).unwrap();
        let mut first_inputs = BTreeMap::from([("x".to_owned(), vec![1.0])]);
        compute
            .queue_wgpu(
                &mut first_inputs,
                &ComputeDispatchRequest {
                    outputs: BTreeSet::new(),
                    logical_turn: 9,
                },
            )
            .unwrap();

        let first = compute.take_command_data().unwrap().unwrap();
        assert!(first.acknowledgement_required);
        let dispatch_token = first.dispatch_token.unwrap();
        let mut second_inputs = BTreeMap::new();
        assert!(
            compute
                .queue_wgpu(&mut second_inputs, &ComputeDispatchRequest::default())
                .is_err()
        );
        compute.acknowledge(dispatch_token).unwrap();
        compute
            .queue_wgpu(&mut second_inputs, &ComputeDispatchRequest::default())
            .unwrap();
    }

    #[test]
    fn wgpu_bridge_transport_rejection_makes_the_command_terminal() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 1);
        compute.configure_completion(&prepared.program).unwrap();
        compute
            .bind_completion_target(Arc::new(RecordingCompletion::default()))
            .unwrap();
        let mut inputs = BTreeMap::new();
        compute
            .queue_wgpu(
                &mut inputs,
                &ComputeDispatchRequest {
                    outputs: BTreeSet::new(),
                    logical_turn: 41,
                },
            )
            .unwrap();
        let command = compute.take_command_data().unwrap().unwrap();
        compute
            .reject(command.dispatch_token.unwrap(), "device lost")
            .unwrap();

        assert!(
            compute
                .queue_wgpu(&mut inputs, &ComputeDispatchRequest::default())
                .is_err()
        );
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Terminal(_)
        ));
    }

    #[test]
    fn wgpu_completion_validates_the_whole_snapshot_before_publishing() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 1);
        compute.configure_completion(&prepared.program).unwrap();
        let completion = Arc::new(RecordingCompletion::default());
        compute.bind_completion_target(completion.clone()).unwrap();
        let mut inputs = BTreeMap::new();
        let requested = prepared
            .program
            .interface()
            .outputs
            .iter()
            .map(|port| port.id)
            .collect();
        compute
            .queue_wgpu(
                &mut inputs,
                &ComputeDispatchRequest {
                    outputs: requested,
                    logical_turn: 17,
                },
            )
            .unwrap();
        let dispatch_token = compute
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_token
            .unwrap();
        let requested = match &compute.state.lock().unwrap().phase {
            ComputeCommandPhase::Claimed(request) => request.outputs.clone(),
            phase => panic!("expected claimed command, got {phase:?}"),
        };

        let error = browser_output_snapshot(
            &prepared.program,
            &requested,
            BTreeMap::from([("result.0".to_owned(), vec![0.0; 8])]),
        )
        .unwrap_err();
        assert!(error.detail.contains("missing result.1"));
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Claimed(_)
        ));
        assert!(completion.outcomes.lock().unwrap().is_empty());

        compute
            .complete(
                dispatch_token,
                BTreeMap::from([
                    ("result.0".to_owned(), vec![0.0; 8]),
                    ("result.1".to_owned(), vec![0.0; 8]),
                ]),
            )
            .unwrap();
        assert!(matches!(
            &completion.outcomes.lock().unwrap()[0],
            ComputeCompletionOutcome::Completed { report, .. } if report.completed_turns == 1
        ));
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Idle
        ));
    }

    #[test]
    fn browser_completion_enters_the_runtime_in_portable_row_major_order() {
        let (_document, prepared) = compile_fixture(CONFIG, FIXED_SHAPE_SOURCE);
        let output = &prepared.program.interface().outputs[0];
        let snapshot = browser_output_snapshot(
            &prepared.program,
            &BTreeSet::from([output.id]),
            BTreeMap::from([(output.name.to_string(), vec![1.0, 2.0])]),
        )
        .unwrap();

        let ComputeValue::TensorF32 { layout, values, .. } = &snapshot.values[&output.id] else {
            panic!("the fixed-shape matrix output must remain a tensor");
        };
        assert_eq!(*layout, TensorLayout::RowMajor);
        assert_eq!(values.as_ref(), [1.0, 2.0]);
    }

    #[test]
    fn failed_js_delivery_rolls_the_exact_command_back_to_queued() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 7);
        compute.configure_completion(&prepared.program).unwrap();
        let mut inputs = BTreeMap::new();
        compute
            .queue_wgpu(&mut inputs, &ComputeDispatchRequest::default())
            .unwrap();

        let lease = compute.lease_command().unwrap().unwrap();
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Serializing(_)
        ));
        compute.rollback_delivery(lease);
        let delivered = compute.take_command_data().unwrap().unwrap();
        assert_eq!(delivered.dispatch_token.unwrap().to_string(), "7:1");
    }

    #[test]
    fn stale_generation_cannot_complete_replacement_dispatch_one() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let request = ComputeDispatchRequest::default();
        let completion = Arc::new(RecordingCompletion::default());
        let old = ComputeCommandHandle::new("test".to_owned(), 17);
        old.configure_completion(&prepared.program).unwrap();
        old.bind_completion_target(completion.clone()).unwrap();
        let replacement = ComputeCommandHandle::new("test".to_owned(), 18);
        replacement.configure_completion(&prepared.program).unwrap();
        replacement.bind_completion_target(completion).unwrap();
        let mut inputs = BTreeMap::new();
        old.queue_wgpu(&mut inputs, &request).unwrap();
        let old_token = old
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_token
            .unwrap();
        replacement.queue_wgpu(&mut inputs, &request).unwrap();
        let replacement_token = replacement
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_token
            .unwrap();

        assert_eq!(old_token.to_string(), "17:1");
        assert_eq!(replacement_token.to_string(), "18:1");
        assert!(replacement.validate_token_value(old_token).is_err());
        assert_eq!(
            replacement.validate_token_value(replacement_token).unwrap(),
            replacement_token
        );
    }

    #[test]
    fn source_replacement_is_recoverably_busy_until_the_command_completes() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 19);
        compute.configure_completion(&prepared.program).unwrap();
        compute
            .bind_completion_target(Arc::new(RecordingCompletion::default()))
            .unwrap();
        assert!(compute.ensure_source_replacement_ready().is_ok());

        let mut inputs = BTreeMap::new();
        compute
            .queue_wgpu(&mut inputs, &ComputeDispatchRequest::default())
            .unwrap();
        let queued = compute.ensure_source_replacement_ready().unwrap_err();
        assert_eq!(queued.kind_name(), "ComputeSourceReplacementBusy");
        assert!(queued.display_message().contains("queued"));

        let token = compute
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_token
            .unwrap();
        let claimed = compute.ensure_source_replacement_ready().unwrap_err();
        assert!(claimed.display_message().contains("claimed"));

        compute.acknowledge(token).unwrap();
        assert!(compute.ensure_source_replacement_ready().is_ok());
    }

    #[test]
    fn completion_target_failure_retires_advanced_command_as_terminal() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned(), 1);
        compute.configure_completion(&prepared.program).unwrap();
        compute
            .bind_completion_target(Arc::new(FailingCompletion))
            .unwrap();
        let mut inputs = BTreeMap::new();
        compute
            .queue_wgpu(&mut inputs, &ComputeDispatchRequest::default())
            .unwrap();
        let token = compute
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_token
            .unwrap();

        assert!(compute.acknowledge_native(token).is_err());
        assert!(matches!(
            compute.state.lock().unwrap().phase,
            ComputeCommandPhase::Terminal(_)
        ));
    }

    #[test]
    fn hard_region_placement_conflicts_are_reported_by_browser_compiler() {
        let hard_gpu = SOURCE.replacen("@compute", "@gpu", 1);
        let (_, prepared) = compile_fixture(CONFIG, &hard_gpu);
        let registry = browser_compute_backend_registry(
            ComputeCommandHandle::new(prepared.region, 1),
            BrowserOutputHandle::default(),
            true,
        )
        .unwrap();
        let error = match registry.resolve(
            &BackendRequest::Cpu,
            ComputePlatform::Browser,
            prepared.placement,
            &prepared.program,
        ) {
            Ok(_) => panic!("CPU request must not satisfy @gpu placement"),
            Err(error) => error,
        };
        assert!(format!("{error}").contains("incompatible"));

        let hard_cpu = SOURCE.replacen("@compute", "@cpu", 1);
        let (_, prepared) = compile_fixture(CONFIG, &hard_cpu);
        let registry = browser_compute_backend_registry(
            ComputeCommandHandle::new(prepared.region, 1),
            BrowserOutputHandle::default(),
            true,
        )
        .unwrap();
        let error = match registry.resolve(
            &BackendRequest::Gpu,
            ComputePlatform::Browser,
            prepared.placement,
            &prepared.program,
        ) {
            Ok(_) => panic!("GPU request must not satisfy @cpu placement"),
            Err(error) => error,
        };
        assert!(format!("{error}").contains("incompatible"));
    }
}
