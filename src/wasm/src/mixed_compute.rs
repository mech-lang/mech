use std::collections::{BTreeMap, BTreeSet};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

use js_sys::{Array, Float32Array, Object, Reflect};
use mech_compute::{
    BackendClass, BackendId, BackendRequest, CPU_SCALAR_BACKEND, ComputeBackendCapabilities,
    ComputeBackendDescriptor, ComputeBackendError, ComputeBackendFactory, ComputeBackendRegistry,
    ComputeBackendRejection, ComputeCompletionTarget, ComputeDispatchReport, ComputeExecutable,
    ComputeExecutionError, ComputeFaultEvidence, ComputeInitializerSet, ComputeInputUpdate,
    ComputeKernel, ComputeOutputSelection, ComputeOutputSnapshot, ComputePlatform, ComputeProgram,
    ComputeSession, ComputeValue, TensorLayout, WGPU_BACKEND,
};
use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Program};
use mech_engine::ProgramArtifact;
use mech_gpu::{
    ComputeHostFactory, ComputeLowerer, CpuScalarBackendFactory, ElementwiseKernel,
    FixedShapeKernel, lower_elementwise_compute_program,
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
        let compute = ComputeCommandHandle::new(prepared.region.clone());
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
        let Some(command) = self.compute.take_command_data()? else {
            return Ok(JsValue::NULL);
        };
        compute_command_value(&self.compute.region, command)
    }

    #[wasm_bindgen(js_name = acknowledgeComputeCommand)]
    pub fn acknowledge_compute_command(&self, dispatch_id: u32) -> Result<(), JsValue> {
        self.compute.acknowledge(dispatch_id)
    }

    #[wasm_bindgen(js_name = completeComputeCommand)]
    pub fn complete_compute_command(
        &self,
        dispatch_id: u32,
        outputs: Array,
    ) -> Result<(), JsValue> {
        self.compute
            .complete(dispatch_id, completed_outputs_from_js(outputs)?)
    }

    #[wasm_bindgen(js_name = rejectComputeCommand)]
    pub fn reject_compute_command(&self, dispatch_id: u32, reason: &str) -> Result<(), JsValue> {
        self.compute.reject(dispatch_id, reason)
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
    pub(crate) kernel: PreparedGpuKernel,
    pub(crate) timings: CompileTimings,
}

#[derive(Clone)]
pub(crate) struct BrowserComputeBridge {
    command: ComputeCommandHandle,
    manifest: JsValue,
    backend: String,
}

impl BrowserComputeBridge {
    pub(crate) fn manifest(&self) -> JsValue {
        self.manifest.clone()
    }

    pub(crate) fn backend(&self) -> String {
        self.backend.clone()
    }

    pub(crate) fn take_command(&self) -> Result<JsValue, JsValue> {
        self.command
            .take_command_data()?
            .map(|command| compute_command_value(&self.command.region, command))
            .transpose()
            .map(|command| command.unwrap_or(JsValue::NULL))
    }

    pub(crate) fn acknowledge(&self, dispatch_id: u32) -> Result<(), JsValue> {
        self.command.acknowledge(dispatch_id)
    }

    pub(crate) fn complete(
        &self,
        dispatch_id: u32,
        outputs: BTreeMap<String, Vec<f32>>,
    ) -> Result<(), JsValue> {
        self.command.complete(dispatch_id, outputs)
    }

    pub(crate) fn reject(&self, dispatch_id: u32, reason: &str) -> Result<(), JsValue> {
        self.command.reject(dispatch_id, reason)
    }

    pub(crate) fn reject_integrity(
        &self,
        dispatch_id: u32,
        constraint: &str,
        instance: u32,
    ) -> Result<(), JsValue> {
        self.command
            .reject_integrity(dispatch_id, constraint, instance)
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
) -> MResult<PreparedBrowserComputeHost> {
    let compute_index = configured_host_index(document, "compute")?;
    let command = ComputeCommandHandle::new(prepared.region.clone());
    let outputs = BrowserOutputHandle::default();
    let registry = browser_compute_backend_registry(command.clone(), outputs, gpu_available)?;
    let factory = ComputeHostFactory::new(
        prepared.region.clone(),
        prepared.placement,
        prepared.program.clone(),
        prepared.initializers.clone(),
        registry,
        ComputePlatform::Browser,
    )?;
    let backend = factory
        .resolved_backend_id(&document.hosts[compute_index].settings)?
        .to_string();
    let manifest = gpu_program_manifest(
        prepared.kernel.browser_program(),
        &initializer_values(&prepared.program, &prepared.initializers)?,
        prepared.timings,
    )
    .map_err(|failure| mixed_error(format!("browser compute manifest failed: {failure:?}")))?;
    Ok(PreparedBrowserComputeHost {
        factory,
        coordinator: prepared.coordinator,
        bridge: BrowserComputeBridge {
            command,
            manifest,
            backend,
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

#[derive(Default)]
struct ComputeCommandState {
    changed_inputs: BTreeMap<String, Vec<f32>>,
    completed_outputs: BTreeMap<String, Vec<f32>>,
    queued: Option<QueuedComputeCommand>,
    next_dispatch_id: u32,
    in_flight: BTreeSet<u32>,
    cpu_reserved: bool,
    completion_program: Option<ComputeProgram>,
    completion_target: Option<Arc<dyn ComputeCompletionTarget>>,
    fault_count: u64,
}

impl std::fmt::Debug for ComputeCommandHandle {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("ComputeCommandHandle")
            .field("region", &self.region)
            .finish_non_exhaustive()
    }
}

#[derive(Clone, Copy, Debug)]
struct QueuedComputeCommand {
    dispatch_id: Option<u32>,
    acknowledgement_required: bool,
}

#[derive(Debug)]
pub(crate) struct ComputeCommandData {
    pub(crate) changed_inputs: BTreeMap<String, Vec<f32>>,
    pub(crate) completed_outputs: BTreeMap<String, Vec<f32>>,
    pub(crate) dispatch_id: Option<u32>,
    pub(crate) acknowledgement_required: bool,
}

impl ComputeCommandHandle {
    pub(crate) fn new(region: String) -> Self {
        Self {
            region,
            state: Arc::new(Mutex::new(ComputeCommandState::default())),
        }
    }

    pub(crate) fn take_command_data(&self) -> Result<Option<ComputeCommandData>, JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        let Some(command) = state.queued.take() else {
            return Ok(None);
        };
        if command.acknowledgement_required {
            let dispatch_id = command
                .dispatch_id
                .expect("acknowledged commands have a dispatch ID");
            state.in_flight.insert(dispatch_id);
        }
        Ok(Some(ComputeCommandData {
            changed_inputs: std::mem::take(&mut state.changed_inputs),
            completed_outputs: std::mem::take(&mut state.completed_outputs),
            dispatch_id: command.dispatch_id,
            acknowledgement_required: command.acknowledgement_required,
        }))
    }

    fn reserve_cpu(&self) -> Result<(), ComputeExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            browser_execution_error(CPU_SCALAR_BACKEND, "dispatch", "command lock is poisoned")
        })?;
        ensure_command_slot_available(&state, CPU_SCALAR_BACKEND)?;
        state.cpu_reserved = true;
        Ok(())
    }

    fn cancel_cpu_reservation(&self) {
        if let Ok(mut state) = self.state.lock() {
            state.cpu_reserved = false;
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
        if !state.cpu_reserved {
            return Err(browser_execution_error(
                CPU_SCALAR_BACKEND,
                "dispatch",
                "the browser CPU command slot was not reserved",
            ));
        }
        state.changed_inputs.append(changed_inputs);
        state.completed_outputs.append(completed_outputs);
        state.queued = Some(QueuedComputeCommand {
            dispatch_id: None,
            acknowledgement_required: false,
        });
        state.cpu_reserved = false;
        Ok(())
    }

    fn queue_wgpu(
        &self,
        changed_inputs: &mut BTreeMap<String, Vec<f32>>,
    ) -> Result<(), ComputeExecutionError> {
        let mut state = self.state.lock().map_err(|_| {
            browser_execution_error(WGPU_BACKEND, "dispatch", "command lock is poisoned")
        })?;
        ensure_command_slot_available(&state, WGPU_BACKEND)?;
        let dispatch_id = state.next_dispatch_id.checked_add(1).ok_or_else(|| {
            browser_execution_error(WGPU_BACKEND, "dispatch", "dispatch ID space exhausted")
        })?;
        state.next_dispatch_id = dispatch_id;
        state.changed_inputs.append(changed_inputs);
        state.queued = Some(QueuedComputeCommand {
            dispatch_id: Some(dispatch_id),
            acknowledgement_required: true,
        });
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

    pub(crate) fn acknowledge(&self, dispatch_id: u32) -> Result<(), JsValue> {
        self.complete(dispatch_id, BTreeMap::new())
    }

    pub(crate) fn complete(
        &self,
        dispatch_id: u32,
        outputs: BTreeMap<String, Vec<f32>>,
    ) -> Result<(), JsValue> {
        let (program, target, report) = {
            let state = self
                .state
                .lock()
                .map_err(|_| error("compute command state lock is poisoned"))?;
            if !state.in_flight.contains(&dispatch_id) {
                return Err(error(format!(
                    "compute dispatch {dispatch_id} is not awaiting acknowledgement"
                )));
            }
            let program = state
                .completion_program
                .clone()
                .ok_or_else(|| error("compute completion has no compiled output contract"))?;
            let target = state
                .completion_target
                .clone()
                .ok_or_else(|| error("compute completion has no resident publication target"))?;
            let report = ComputeDispatchReport {
                completed_turns: 1,
                fault_count: state.fault_count,
                ..Default::default()
            };
            (program, target, report)
        };
        let snapshot = browser_output_snapshot(&program, outputs)
            .map_err(|failure| error(format!("{}: {}", failure.operation, failure.detail)))?;
        self.state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?
            .in_flight
            .remove(&dispatch_id);
        target
            .complete(report, snapshot)
            .map_err(|failure| error(format!("{}: {}", failure.operation, failure.detail)))?;
        Ok(())
    }

    pub(crate) fn reject_integrity(
        &self,
        dispatch_id: u32,
        constraint: &str,
        instance: u32,
    ) -> Result<(), JsValue> {
        let (target, report) = {
            let mut state = self
                .state
                .lock()
                .map_err(|_| error("compute command state lock is poisoned"))?;
            if !state.in_flight.remove(&dispatch_id) {
                return Err(error(format!(
                    "compute dispatch {dispatch_id} is not awaiting acknowledgement"
                )));
            }
            state.fault_count = state.fault_count.saturating_add(1);
            let target = state
                .completion_target
                .clone()
                .ok_or_else(|| error("compute completion has no resident publication target"))?;
            let report = ComputeDispatchReport {
                completed_turns: 0,
                fault_count: state.fault_count,
                last_fault: Some(ComputeFaultEvidence {
                    attempted_turn: u64::from(dispatch_id),
                    constraint: constraint.to_owned().into_boxed_str(),
                    detail: format!("candidate rejected at batch instance {instance}")
                        .into_boxed_str(),
                }),
                ..Default::default()
            };
            (target, report)
        };
        target
            .complete(report, ComputeOutputSnapshot::default())
            .map_err(|failure| error(format!("{}: {}", failure.operation, failure.detail)))
    }

    pub(crate) fn reject(&self, dispatch_id: u32, reason: &str) -> Result<(), JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        if !state.in_flight.remove(&dispatch_id) {
            return Err(error(format!(
                "compute dispatch {dispatch_id} is not awaiting acknowledgement"
            )));
        }
        let _ = reason;
        Ok(())
    }
}

fn ensure_command_slot_available(
    state: &ComputeCommandState,
    backend: &str,
) -> Result<(), ComputeExecutionError> {
    if state.queued.is_some() || !state.in_flight.is_empty() || state.cpu_reserved {
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
    command: ComputeCommandData,
) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(&value, "region", region)?;
    set(&value, "dispatch", true)?;
    set(
        &value,
        "acknowledgementRequired",
        command.acknowledgement_required,
    )?;
    if let Some(dispatch_id) = command.dispatch_id {
        set(&value, "dispatchId", dispatch_id)?;
    }
    let inputs = Array::new();
    for (name, values) in command.changed_inputs {
        let input = Object::new();
        set(&input, "name", name)?;
        set(&input, "values", Float32Array::from(values.as_slice()))?;
        inputs.push(&input);
    }
    set(&value, "inputs", inputs)?;
    let outputs = Array::new();
    for (name, values) in command.completed_outputs {
        let output = Object::new();
        set(&output, "name", name)?;
        set(&output, "values", Float32Array::from(values.as_slice()))?;
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
            let snapshot = self.inner.read_outputs(&ComputeOutputSelection::All)?;
            let mut completed_outputs = self.outputs.publish(&self.program, &snapshot)?;
            self.command
                .commit_cpu(&mut self.changed_inputs, &mut completed_outputs)?;
            Ok(report)
        })();
        if result.is_err() {
            self.command.cancel_cpu_reservation();
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
        self.command.configure_completion(program)?;
        Ok(Box::new(BrowserWgpuExecutable {
            backend: self.descriptor.id.clone(),
            program: program.clone(),
            command: self.command.clone(),
        }))
    }

    fn bind_completion_target(
        &self,
        target: Arc<dyn ComputeCompletionTarget>,
    ) -> Result<(), ComputeBackendError> {
        self.command.bind_completion_target(target)
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
        if turns.get() != 1 {
            return Err(browser_execution_error(
                WGPU_BACKEND,
                "dispatch",
                "the browser render bridge accepts one resident turn per frame",
            ));
        }
        self.command.queue_wgpu(&mut self.changed_inputs)?;
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
    completed: BTreeMap<String, Vec<f32>>,
) -> Result<ComputeOutputSnapshot, ComputeExecutionError> {
    if completed.len() != program.interface().outputs.len() {
        let missing = program
            .interface()
            .outputs
            .iter()
            .filter(|port| !completed.contains_key(port.name.as_ref()))
            .map(|port| port.name.as_ref())
            .collect::<Vec<_>>();
        return Err(browser_execution_error(
            WGPU_BACKEND,
            "read outputs",
            format!(
                "browser bridge returned {} outputs; expected {}{}",
                completed.len(),
                program.interface().outputs.len(),
                if missing.is_empty() {
                    String::new()
                } else {
                    format!("; missing {}", missing.join(", "))
                },
            ),
        ));
    }
    let mut values = BTreeMap::new();
    for (name, physical) in completed {
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
        let expected = port.elements().map_err(|failure| {
            browser_execution_error(WGPU_BACKEND, "read outputs", failure.to_string())
        })?;
        if physical.len() != expected {
            return Err(browser_execution_error(
                WGPU_BACKEND,
                "read outputs",
                format!(
                    "browser bridge returned {} values for `{name}`; expected {expected}",
                    physical.len()
                ),
            ));
        }
        let value = if port.dimensions.is_empty() {
            ComputeValue::ScalarF32(physical[0])
        } else {
            ComputeValue::TensorF32 {
                dimensions: port.dimensions.clone(),
                layout: match program.kernel() {
                    ComputeKernel::FixedShape(_) => TensorLayout::ColumnMajor,
                    ComputeKernel::Elementwise(_) => TensorLayout::RowMajor,
                },
                values: physical.into(),
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
    }
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
        reports: Mutex<Vec<ComputeDispatchReport>>,
    }

    impl ComputeCompletionTarget for RecordingCompletion {
        fn complete(
            &self,
            report: ComputeDispatchReport,
            _snapshot: ComputeOutputSnapshot,
        ) -> Result<(), ComputeExecutionError> {
            self.reports.lock().unwrap().push(report);
            Ok(())
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
        let compute = ComputeCommandHandle::new(prepared.region.clone());
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
        assert_eq!(command.changed_inputs["force-point"], vec![0.25, -0.5]);
        assert_eq!(command.changed_inputs["force-strength"], vec![1.0]);
        let dispatch_id = command.dispatch_id.unwrap();
        assert!(
            compute
                .state
                .lock()
                .unwrap()
                .in_flight
                .contains(&dispatch_id)
        );
        compute
            .complete(
                dispatch_id,
                BTreeMap::from([
                    ("result.0".to_owned(), vec![0.0; 8]),
                    ("result.1".to_owned(), vec![0.0; 8]),
                ]),
            )
            .unwrap();
        assert!(
            !compute
                .state
                .lock()
                .unwrap()
                .in_flight
                .contains(&dispatch_id)
        );
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
        assert_eq!(state.dispatch_id, None);
        assert_eq!(state.changed_inputs["force-point"], vec![0.25, -0.5]);
        assert_eq!(state.changed_inputs["force-strength"], vec![1.0]);
        let positions = outputs.output("result.0").unwrap();
        assert_eq!(positions.len(), 8);
        assert!(positions.iter().all(|value| value.is_finite()));
        assert_eq!(state.completed_outputs["result.0"], positions);
        assert_eq!(state.completed_outputs["result.1"].len(), 8);
    }

    #[test]
    fn wgpu_holds_the_command_slot_until_the_bridge_completes_it() {
        let compute = ComputeCommandHandle::new("test".to_owned());
        let mut first_inputs = BTreeMap::from([("x".to_owned(), vec![1.0])]);
        compute.queue_wgpu(&mut first_inputs).unwrap();

        let first = compute.take_command_data().unwrap().unwrap();
        assert!(first.acknowledgement_required);
        let dispatch_id = first.dispatch_id.unwrap();
        let mut second_inputs = BTreeMap::new();
        assert!(compute.queue_wgpu(&mut second_inputs).is_err());
        compute.reject(dispatch_id, "test cleanup").unwrap();
        compute.queue_wgpu(&mut second_inputs).unwrap();
    }

    #[test]
    fn wgpu_bridge_rejection_releases_the_command_slot() {
        let compute = ComputeCommandHandle::new("test".to_owned());
        let mut inputs = BTreeMap::new();
        compute.queue_wgpu(&mut inputs).unwrap();
        let command = compute.take_command_data().unwrap().unwrap();
        compute
            .reject(command.dispatch_id.unwrap(), "device lost")
            .unwrap();

        compute.queue_wgpu(&mut inputs).unwrap();
    }

    #[test]
    fn wgpu_completion_validates_the_whole_snapshot_before_publishing() {
        let (_document, prepared) = compile_fixture(CONFIG, SOURCE);
        let compute = ComputeCommandHandle::new("test".to_owned());
        compute.configure_completion(&prepared.program).unwrap();
        let completion = Arc::new(RecordingCompletion::default());
        compute.bind_completion_target(completion.clone()).unwrap();
        let mut inputs = BTreeMap::new();
        compute.queue_wgpu(&mut inputs).unwrap();
        let dispatch_id = compute
            .take_command_data()
            .unwrap()
            .unwrap()
            .dispatch_id
            .unwrap();

        let error = browser_output_snapshot(
            &prepared.program,
            BTreeMap::from([("result.0".to_owned(), vec![0.0; 8])]),
        )
        .unwrap_err();
        assert!(error.detail.contains("missing result.1"));
        assert!(
            compute
                .state
                .lock()
                .unwrap()
                .in_flight
                .contains(&dispatch_id)
        );
        assert!(completion.reports.lock().unwrap().is_empty());

        compute
            .complete(
                dispatch_id,
                BTreeMap::from([
                    ("result.0".to_owned(), vec![0.0; 8]),
                    ("result.1".to_owned(), vec![0.0; 8]),
                ]),
            )
            .unwrap();
        assert_eq!(completion.reports.lock().unwrap()[0].completed_turns, 1);
        assert!(
            !compute
                .state
                .lock()
                .unwrap()
                .in_flight
                .contains(&dispatch_id)
        );
    }

    #[test]
    fn hard_region_placement_conflicts_are_reported_by_browser_compiler() {
        let hard_gpu = SOURCE.replacen("@compute", "@gpu", 1);
        let (_, prepared) = compile_fixture(CONFIG, &hard_gpu);
        let registry = browser_compute_backend_registry(
            ComputeCommandHandle::new(prepared.region),
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
            ComputeCommandHandle::new(prepared.region),
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
