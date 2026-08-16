use std::collections::{BTreeMap, BTreeSet};
use std::sync::{Arc, LazyLock, Mutex};

use js_sys::{Array, Float32Array, Object, Reflect};
use mech_core::{
    AccessMode, Body, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult, MechCode,
    MechError, MechErrorKind, OperationContractDeclaration, Program, Ref, Section, SectionElement,
};
use mech_gpu::{GpuBindingRole, GpuHost, OwnedResidentCpuSession, column_major_to_row_major};
use mech_runtime::{
    ConfigProfileOptions, ConfigValue, HostContextManifest, HostManifestConfig, MechConfigDocument,
    MechRuntime, PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeBuilder,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest, parse_config_document,
};
use wasm_bindgen::prelude::*;
use web_time::Instant;

use crate::gpu::{CompileTimings, gpu_program_manifest};

const POINTER_PATHS: [&str; 5] = ["pulse", "x", "y", "pressed", "delta-seconds"];

static COMPUTE_COMMAND_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
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

#[wasm_bindgen]
pub struct WasmMixedComputeProject {
    runtime: MechRuntime,
    pointer: PointerInputHandle,
    compute: ComputeCommandHandle,
    cpu: Option<OwnedResidentCpuSession>,
    backend: ComputeBackend,
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
        let prepared = compile_named_compute_region(
            &document,
            &tree,
            parsing,
            backend_override,
            gpu_available,
        )
        .map_err(js_error)?;
        let manifest = gpu_program_manifest(&prepared.program, &prepared.inputs, prepared.timings)?;

        let pointer = PointerInputHandle::new(
            configured_host_instance(&document, "pointer").map_err(js_error)?,
        );
        let compute = ComputeCommandHandle::new(
            prepared.region.clone(),
            prepared
                .program
                .bindings()
                .iter()
                .filter(|binding| binding.role() == GpuBindingRole::Input)
                .map(|binding| (binding.name.clone(), binding.elements as usize)),
        );
        let cpu = if prepared.backend == ComputeBackend::Cpu {
            Some(
                prepared
                    .program
                    .into_cpu(&prepared.inputs)
                    .map_err(|failure| {
                        error(format!("CPU executor preparation failed: {failure}"))
                    })?,
            )
        } else {
            None
        };
        let mut builder = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .config(
                mech_runtime::RuntimeConfig::default()
                    .apply_patch(&document.runtime)
                    .map_err(js_error)?,
            )
            .host_factory(Box::new(PointerHostFactory::new(pointer.clone())))
            .map_err(js_error)?
            .host_factory(Box::new(ComputeCommandHostFactory::new(compute.clone())))
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
            .load_tree_program(&cpu_projection_tree(&tree), durability)
            .map_err(js_error)?;

        Ok(Self {
            runtime,
            pointer,
            compute,
            cpu,
            backend: prepared.backend,
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
        self.backend.as_str().to_owned()
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
        if let Some(cpu) = self.cpu.as_mut() {
            cpu.update_inputs(&command.changed_inputs)
                .map_err(|failure| error(format!("CPU input update failed: {failure}")))?;
            cpu.dispatch_turns(1)
                .map_err(|failure| error(format!("CPU dispatch failed: {failure}")))?;
        }
        compute_command_value(&self.compute.region, command)
    }

    #[wasm_bindgen(js_name = cpuOutput)]
    pub fn cpu_output(&self, name: &str) -> Result<Float32Array, JsValue> {
        let cpu = self
            .cpu
            .as_ref()
            .ok_or_else(|| error("cpuOutput is available only under the CPU compute backend"))?;
        let values = cpu
            .output(name)
            .map_err(|failure| error(format!("CPU output read failed: {failure}")))?;
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComputeBackend {
    Cpu,
    Gpu,
}

impl ComputeBackend {
    const fn as_str(self) -> &'static str {
        match self {
            Self::Cpu => "cpu",
            Self::Gpu => "gpu",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ComputeBackendRequest {
    Auto,
    Cpu,
    Gpu,
}

impl ComputeBackendRequest {
    fn parse(value: &str, source: &str) -> MResult<Self> {
        match value {
            "auto" | "" => Ok(Self::Auto),
            "cpu" => Ok(Self::Cpu),
            "gpu" => Ok(Self::Gpu),
            _ => Err(mixed_error(format!(
                "{source} backend must be `auto`, `cpu`, or `gpu`, found `{value}`"
            ))),
        }
    }
}

struct ConfiguredComputeHost {
    instance: String,
    region: String,
    backend: ComputeBackendRequest,
}

#[derive(Debug)]
struct PreparedComputeRegion {
    region: String,
    backend: ComputeBackend,
    program: mech_gpu::GpuProgram,
    inputs: BTreeMap<String, Vec<f32>>,
    timings: CompileTimings,
}

fn compile_named_compute_region(
    document: &MechConfigDocument,
    tree: &Program,
    parsing: f64,
    backend_override: &str,
    gpu_available: bool,
) -> MResult<PreparedComputeRegion> {
    let configured = configured_compute_host(document)?;
    let region = configured.region.clone();
    let requested = if backend_override.is_empty() {
        configured.backend
    } else {
        ComputeBackendRequest::parse(backend_override, "command-line")?
    };
    let accelerated_sections = tree
        .body
        .sections
        .iter()
        .filter(|section| is_compute_owned(section))
        .collect::<Vec<_>>();
    if accelerated_sections.len() != 1 {
        return Err(mixed_error(format!(
            "mixed browser projects require exactly one accelerated section, found {}",
            accelerated_sections.len()
        )));
    }
    let section = accelerated_sections[0];
    let section_name = section
        .subtitle
        .as_ref()
        .map(|subtitle| subtitle.to_string().trim().to_owned())
        .unwrap_or_default();
    if section_name != region {
        return Err(mixed_error(format!(
            "compute host selects region `{region}`, but the compiled section is `{section_name}`"
        )));
    }
    let placement = section.compute.expect("filtered to a compute section");
    let backend = match requested {
        ComputeBackendRequest::Cpu => ComputeBackend::Cpu,
        ComputeBackendRequest::Gpu => ComputeBackend::Gpu,
        ComputeBackendRequest::Auto => match placement {
            mech_core::ComputePlacement::Cpu => ComputeBackend::Cpu,
            mech_core::ComputePlacement::Gpu => ComputeBackend::Gpu,
            mech_core::ComputePlacement::Compute if gpu_available => ComputeBackend::Gpu,
            mech_core::ComputePlacement::Compute => ComputeBackend::Cpu,
        },
    };
    let imports = import_prelude(tree);
    let isolated = isolated_region_tree(&imports, section);

    let compiler_started = Instant::now();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_native_plan_catalog())
        .build_compiler()?;
    let catalog_setup = milliseconds(compiler_started);
    let external_input_names = configured_compute_inputs(document, tree, &configured.instance)?;
    let artifact_started = Instant::now();
    let (product, initial_inputs) = compiler.compile_tree_artifact_with_input_initializers(
        &isolated,
        &BTreeMap::new(),
        &external_input_names,
    )?;
    let artifact_compilation = milliseconds(artifact_started);
    let artifact = product.artifact();
    let compute_regions = product.compute_regions();
    if compute_regions.len() != 1 || compute_regions[0].name != region {
        return Err(mixed_error(format!(
            "isolated section `{region}` did not produce exactly that compute region"
        )));
    }
    let lowering_started = Instant::now();
    let program = match backend {
        ComputeBackend::Cpu => GpuHost.compile_cpu_with_regions(artifact, compute_regions),
        ComputeBackend::Gpu => GpuHost.compile_with_regions(artifact, compute_regions),
    }
    .map_err(|failure| {
        mixed_error(format!(
            "{} compute backend rejected region `{region}`: {failure}",
            backend.as_str()
        ))
    })?;
    let gpu_lowering = milliseconds(lowering_started);
    let input_started = Instant::now();
    let mut inputs = BTreeMap::new();
    for binding in program
        .bindings()
        .iter()
        .filter(|binding| binding.role() == GpuBindingRole::Input)
    {
        let initial = initial_inputs.get(&binding.name).ok_or_else(|| {
            mixed_error(format!(
                "compute input `{}` has no declaration-time value",
                binding.name
            ))
        })?;
        let values = match initial {
            RuntimeHostInputValue::F32(value) => vec![*value],
            RuntimeHostInputValue::F32Matrix {
                rows,
                columns,
                values,
            } => column_major_to_row_major(*rows, *columns, values).map_err(mixed_error)?,
            other => {
                return Err(mixed_error(format!(
                    "compute input `{}` must be f32 data, found {other:?}",
                    binding.name
                )));
            }
        };
        if values.len() != binding.elements as usize {
            return Err(mixed_error(format!(
                "compute input `{}` initializer has {} elements, expected {}",
                binding.name,
                values.len(),
                binding.elements,
            )));
        }
        inputs.insert(binding.name.clone(), values);
    }
    let input_capture = milliseconds(input_started);
    Ok(PreparedComputeRegion {
        region,
        backend,
        program,
        inputs,
        timings: CompileTimings {
            catalog_setup,
            parsing,
            artifact_compilation,
            gpu_lowering,
            input_capture,
        },
    })
}

fn configured_compute_host(document: &MechConfigDocument) -> MResult<ConfiguredComputeHost> {
    let hosts = document
        .hosts
        .iter()
        .filter(|host| host.provider == "compute")
        .collect::<Vec<_>>();
    if hosts.len() != 1 {
        return Err(mixed_error(format!(
            "mixed browser projects require exactly one `compute` host, found {}",
            hosts.len()
        )));
    }
    let ConfigValue::Map(settings) = &hosts[0].settings else {
        return Err(mixed_error("compute host settings must be a map"));
    };
    let Some(ConfigValue::String(region)) = settings.get("region") else {
        return Err(mixed_error(
            "compute host setting `region` must be a string",
        ));
    };
    let backend = match settings.get("backend") {
        None => ComputeBackendRequest::Auto,
        Some(ConfigValue::String(backend)) => {
            ComputeBackendRequest::parse(backend, "compute host")?
        }
        Some(_) => {
            return Err(mixed_error(
                "compute host setting `backend` must be a string",
            ));
        }
    };
    Ok(ConfiguredComputeHost {
        instance: hosts[0].name.clone(),
        region: region.clone(),
        backend,
    })
}

fn configured_compute_inputs(
    document: &MechConfigDocument,
    tree: &Program,
    instance: &str,
) -> MResult<BTreeSet<String>> {
    let target = format!("{instance}/kernel");
    let grants = document
        .run
        .as_ref()
        .map(|run| run.grants.as_slice())
        .unwrap_or_default();
    Ok(mech_runtime::granted_resource_paths_from_program(
        tree,
        &format!("compute://{target}"),
        &target,
        "write",
        grants,
    )?
    .into_iter()
    .filter_map(|path| path.strip_prefix("input/").map(str::to_owned))
    .collect())
}

fn configured_host_instance(document: &MechConfigDocument, provider: &str) -> MResult<String> {
    let hosts = document
        .hosts
        .iter()
        .filter(|host| host.provider == provider)
        .collect::<Vec<_>>();
    if hosts.len() != 1 {
        return Err(mixed_error(format!(
            "mixed browser projects require exactly one `{provider}` host, found {}",
            hosts.len()
        )));
    }
    Ok(hosts[0].name.clone())
}

fn is_compute_owned(section: &Section) -> bool {
    section.compute.is_some()
}

fn cpu_projection_tree(tree: &Program) -> Program {
    let excluded_imports = tree
        .body
        .sections
        .iter()
        .filter(|section| is_compute_owned(section))
        .flat_map(|section| &section.elements)
        .filter_map(import_element)
        .collect::<Vec<_>>();
    let mut sections = Vec::with_capacity(tree.body.sections.len() + 1);
    if !excluded_imports.is_empty() {
        sections.push(Section {
            subtitle: None,
            compute: None,
            elements: excluded_imports,
        });
    }
    sections.extend(
        tree.body
            .sections
            .iter()
            .filter(|section| !is_compute_owned(section))
            .cloned(),
    );
    Program {
        title: tree.title.clone(),
        body: Body { sections },
    }
}

fn import_prelude(tree: &Program) -> Vec<SectionElement> {
    tree.body
        .sections
        .iter()
        .flat_map(|section| &section.elements)
        .filter_map(import_element)
        .collect()
}

fn import_element(element: &SectionElement) -> Option<SectionElement> {
    let SectionElement::MechCode(code) = element else {
        return None;
    };
    let imports = code
        .iter()
        .filter(|(code, _)| matches!(code, MechCode::Import(_)))
        .cloned()
        .collect::<Vec<_>>();
    (!imports.is_empty()).then_some(SectionElement::MechCode(imports))
}

fn isolated_region_tree(imports: &[SectionElement], section: &Section) -> Program {
    let mut sections = Vec::with_capacity(2);
    if !imports.is_empty() {
        sections.push(Section {
            subtitle: None,
            compute: None,
            elements: imports.to_vec(),
        });
    }
    sections.push(section.clone());
    Program {
        title: None,
        body: Body { sections },
    }
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
            pointer_update(&self.base_uri, "x", RuntimeHostInputValue::F64(x))?,
            pointer_update(&self.base_uri, "y", RuntimeHostInputValue::F64(y))?,
            pointer_update(
                &self.base_uri,
                "pressed",
                RuntimeHostInputValue::F64(f64::from(pressed)),
            )?,
            pointer_update(
                &self.base_uri,
                "delta-seconds",
                RuntimeHostInputValue::F64(delta_seconds),
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
        Ok(LegacyValue::F64(Ref::new(0.0)))
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

#[derive(Clone, Debug)]
struct ComputeCommandHandle {
    region: String,
    allowed_inputs: Arc<BTreeMap<String, usize>>,
    state: Arc<Mutex<ComputeCommandState>>,
}

#[derive(Debug, Default)]
struct ComputeCommandState {
    changed_inputs: BTreeMap<String, Vec<f32>>,
    dispatch: bool,
}

impl ComputeCommandHandle {
    fn new(region: String, inputs: impl IntoIterator<Item = (String, usize)>) -> Self {
        Self {
            region,
            allowed_inputs: Arc::new(inputs.into_iter().collect()),
            state: Arc::new(Mutex::new(ComputeCommandState::default())),
        }
    }

    fn take_command_data(&self) -> Result<Option<ComputeCommandState>, JsValue> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| error("compute command state lock is poisoned"))?;
        if !state.dispatch {
            return Ok(None);
        }
        state.dispatch = false;
        Ok(Some(ComputeCommandState {
            changed_inputs: std::mem::take(&mut state.changed_inputs),
            dispatch: true,
        }))
    }
}

fn compute_command_value(region: &str, command: ComputeCommandState) -> Result<JsValue, JsValue> {
    let value = Object::new();
    set(&value, "region", region)?;
    set(&value, "dispatch", command.dispatch)?;
    let inputs = Array::new();
    for (name, values) in command.changed_inputs {
        let input = Object::new();
        set(&input, "name", name)?;
        set(&input, "values", Float32Array::from(values.as_slice()))?;
        inputs.push(&input);
    }
    set(&value, "inputs", inputs)?;
    Ok(value.into())
}

#[derive(Debug)]
struct ComputeCommandHostFactory {
    handle: ComputeCommandHandle,
    manifest: HostManifestConfig,
}

impl ComputeCommandHostFactory {
    fn new(handle: ComputeCommandHandle) -> Self {
        Self {
            handle,
            manifest: HostManifestConfig {
                provider: "compute".to_owned(),
                contexts: vec![HostContextManifest {
                    name: "kernel".to_owned(),
                    base_uri_template: "compute://{instance}/kernel".to_owned(),
                    operations: vec!["write".to_owned()],
                }],
            },
        }
    }
}

impl RuntimeHostFactory for ComputeCommandHostFactory {
    fn provider_name(&self) -> &str {
        "compute"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        let ConfigValue::Map(map) = settings else {
            return Err(mixed_error("compute host settings must be a map"));
        };
        let keys = map.keys().cloned().collect::<BTreeSet<_>>();
        if !keys.is_subset(&BTreeSet::from(["backend".to_owned(), "region".to_owned()])) {
            return Err(mixed_error(
                "compute host settings support only `backend` and `region`",
            ));
        }
        if !matches!(map.get("region"), Some(ConfigValue::String(region)) if region == &self.handle.region)
        {
            return Err(mixed_error(format!(
                "compute host must select region `{}`",
                self.handle.region,
            )));
        }
        match map.get("backend") {
            None => Ok(()),
            Some(ConfigValue::String(backend))
                if backend == "auto" || backend == "cpu" || backend == "gpu" =>
            {
                Ok(())
            }
            _ => Err(mixed_error(
                "compute host backend must be `auto`, `cpu`, or `gpu`",
            )),
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
            resource_providers: vec![Box::new(ComputeCommandResourceProvider {
                instance: instance_name.to_owned(),
                handle: self.handle.clone(),
            })],
            input_drivers: Vec::new(),
        })
    }
}

#[derive(Debug)]
struct ComputeCommandResourceProvider {
    instance: String,
    handle: ComputeCommandHandle,
}

impl ComputeCommandResourceProvider {
    fn base(&self) -> String {
        format!("compute://{}/kernel", self.instance)
    }

    fn validate(&self, request: &RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.base() {
            return Err(mixed_error(format!(
                "unknown compute resource `{}`",
                request.base_uri
            )));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(mixed_error("compute commands are effects; use <-"));
        }
        if request.path == "turn" {
            return Ok(());
        }
        let Some(name) = request.path.strip_prefix("input/") else {
            return Err(mixed_error(format!(
                "unknown compute command `{}`",
                request.path
            )));
        };
        if !self.handle.allowed_inputs.contains_key(name) {
            return Err(mixed_error(format!(
                "compute region has no input named `{name}`"
            )));
        }
        Ok(())
    }
}

impl RuntimeResourceProvider for ComputeCommandResourceProvider {
    fn scheme(&self) -> &str {
        "compute"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }
    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&COMPUTE_COMMAND_EFFECT_CONTRACT)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(mixed_error(format!(
            "compute command resource `{}` is write-only",
            request.base_uri
        )))
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.validate(&request)
    }
    fn plan_write(&self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        self.validate(&RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri,
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        if let Some(name) = request.path.strip_prefix("input/") {
            validated_compute_input_values(
                name,
                request.value.try_deep_snapshot()?,
                &self.handle.allowed_inputs,
            )?;
        }
        Ok(())
    }
    fn prepare_write(
        &self,
        request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.validate(&RuntimeResourceWritePreflightRequest {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name,
            operation: request.operation,
            intent: request.intent,
        })?;
        let update = if request.path == "turn" {
            ComputeCommandUpdate::Dispatch
        } else {
            let name = request.path.trim_start_matches("input/").to_owned();
            let values = validated_compute_input_values(
                &name,
                request.value.try_deep_snapshot()?,
                &self.handle.allowed_inputs,
            )?;
            ComputeCommandUpdate::Input(name, values)
        };
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            ComputeCommandEffect {
                resource: request.base_uri,
                state: self.handle.state.clone(),
                update,
            },
        )))
    }
}

fn validated_compute_input_values(
    name: &str,
    value: LegacyValue,
    allowed: &BTreeMap<String, usize>,
) -> MResult<Vec<f32>> {
    let values = compute_input_values(value)?;
    let expected = allowed
        .get(name)
        .copied()
        .ok_or_else(|| mixed_error(format!("compute region has no input named `{name}`")))?;
    if values.len() != expected {
        return Err(mixed_error(format!(
            "compute input `{name}` received {} values, expected {expected}",
            values.len()
        )));
    }
    Ok(values)
}

fn compute_input_values(value: LegacyValue) -> MResult<Vec<f32>> {
    match value {
        LegacyValue::Typed(value, _) => compute_input_values(*value),
        LegacyValue::MutableReference(value) => compute_input_values(value.borrow().clone()),
        LegacyValue::F32(value) => Ok(vec![*value.borrow()]),
        LegacyValue::F64(value) => Ok(vec![*value.borrow() as f32]),
        LegacyValue::MatrixF32(matrix) => {
            column_major_to_row_major(matrix.rows(), matrix.cols(), &matrix.as_vec())
                .map_err(mixed_error)
        }
        LegacyValue::MatrixF64(matrix) => {
            column_major_to_row_major(matrix.rows(), matrix.cols(), &matrix.as_vec())
                .map(|values| values.into_iter().map(|value| value as f32).collect())
                .map_err(mixed_error)
        }
        other => Err(mixed_error(format!(
            "compute inputs must be f32/f64 scalars or matrices, found `{}`",
            other.kind()
        ))),
    }
}

#[derive(Debug)]
enum ComputeCommandUpdate {
    Input(String, Vec<f32>),
    Dispatch,
}

#[derive(Debug)]
struct ComputeCommandEffect {
    resource: String,
    state: Arc<Mutex<ComputeCommandState>>,
    update: ComputeCommandUpdate,
}

impl RuntimeAfterCommitEffect for ComputeCommandEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        let bytes = match &self.update {
            ComputeCommandUpdate::Input(_, values) => {
                (values.len() as u64).saturating_mul(std::mem::size_of::<f32>() as u64)
            }
            ComputeCommandUpdate::Dispatch => 0,
        };
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "compute".to_owned(),
            },
            "browser-compute-command",
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes, items: 1 })
    }
    fn deliver(&mut self) -> MResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| mixed_error("compute command state lock is poisoned"))?;
        match &self.update {
            ComputeCommandUpdate::Input(name, values) => {
                state.changed_inputs.insert(name.clone(), values.clone());
            }
            ComputeCommandUpdate::Dispatch => state.dispatch = true,
        }
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct MixedGpuProjectError(String);

impl MechErrorKind for MixedGpuProjectError {
    fn name(&self) -> &str {
        "MixedGpuProjectError"
    }
    fn message(&self) -> String {
        self.0.clone()
    }
}

fn mixed_error(message: impl Into<String>) -> MechError {
    MechError::new(MixedGpuProjectError(message.into()), None).with_compiler_loc()
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
      { target: "cursor/frame" operations: ["read"] paths: ["pulse", "x", "y", "pressed", "delta-seconds"] }
      { target: "particles/kernel" operations: ["write"] paths: ["input/force-x", "input/force-y", "input/force-strength", "input/dt", "turn"] }
    ]
  }
}
"#;

    const SOURCE: &str = r#"
+> math
@pointer := pointer://cursor/frame{:read(pulse), :read(x), :read(y), :read(pressed), :read(delta-seconds)}
@particles := compute://particles/kernel{:write(input/force-x), :write(input/force-y), :write(input/force-strength), :write(input/dt), :write(turn)}
pulse := @pointer/pulse
force-x := @pointer/x
force-y := @pointer/y
force-strength := @pointer/pressed * 1.25
dt := @pointer/delta-seconds
@particles/input/force-x <- force-x
@particles/input/force-y <- force-y
@particles/input/force-strength <- force-strength
@particles/input/dt <- dt
@particles/turn <- pulse

particle-field @ compute
-------------------------------------------------------------------------------
particle-index := 1f32..=4f32
particle-x := math/cos(particle-index)
particle-y := math/sin(particle-index)
~positions := [particle-x; particle-y]
~velocities := [(0f32 - particle-y); particle-x]
force-x := 0f32
force-y := 0f32
force-strength := 0f32
dt := 0.016666667<f32>
force-point := [force-x; force-y]
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

    #[test]
    fn compute_input_initializers_cross_the_matrix_layout_boundary_once() {
        assert_eq!(
            column_major_to_row_major(2, 3, &[1.0, 4.0, 2.0, 5.0, 3.0, 6.0]).unwrap(),
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn live_rectangular_compute_inputs_use_kernel_row_major_order() {
        let value = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 3,
            values: vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
        }
        .into_mech_value()
        .unwrap();

        assert_eq!(
            compute_input_values(value).unwrap(),
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn wildcard_config_grants_expand_to_source_declared_compute_inputs() {
        let tree = mech_syntax::parse(SOURCE).unwrap();
        for wildcard in ["*", "input/*"] {
            let config = CONFIG.replace(
                "[\"input/force-x\", \"input/force-y\", \"input/force-strength\", \"input/dt\", \"turn\"]",
                &format!("[\"{wildcard}\"]"),
            );
            let document =
                parse_config_document("test.mcfg", &config, ConfigProfileOptions::default())
                    .unwrap();
            assert_eq!(
                configured_compute_inputs(&document, &tree, "particles").unwrap(),
                BTreeSet::from([
                    "dt".to_owned(),
                    "force-strength".to_owned(),
                    "force-x".to_owned(),
                    "force-y".to_owned(),
                ])
            );
        }
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
        let tree = mech_syntax::parse(SERVED_SOURCE).unwrap();
        let prepared = compile_named_compute_region(&document, &tree, 0.0, "gpu", true).unwrap();

        assert_eq!(prepared.region, "particle-field");
        assert_eq!(prepared.program.dispatch_elements(), 2_000_000);
        assert_eq!(prepared.inputs["force-x"], vec![0.0]);
        assert_eq!(prepared.inputs["force-y"], vec![0.0]);
        assert_eq!(prepared.inputs["force-strength"], vec![0.0]);
        assert_eq!(prepared.inputs["dt"], vec![0.016666667]);
    }

    #[test]
    fn one_document_builds_cpu_graph_and_generic_gpu_region() {
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();
        let tree = mech_syntax::parse(SOURCE).unwrap();
        let prepared = compile_named_compute_region(&document, &tree, 0.0, "gpu", true).unwrap();
        let input_names = prepared
            .program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::Input)
            .map(|binding| binding.name.as_str())
            .collect::<BTreeSet<_>>();
        assert!(input_names.contains("force-x"));
        assert!(input_names.contains("force-y"));
        assert!(input_names.contains("force-strength"));
        assert!(input_names.contains("dt"));

        let pointer =
            PointerInputHandle::new(configured_host_instance(&document, "pointer").unwrap());
        let compute = ComputeCommandHandle::new(
            prepared.region,
            prepared
                .program
                .bindings()
                .iter()
                .filter(|binding| binding.role() == GpuBindingRole::Input)
                .map(|binding| (binding.name.clone(), binding.elements as usize)),
        );
        let mut builder = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .config(
                mech_runtime::RuntimeConfig::default()
                    .apply_patch(&document.runtime)
                    .unwrap(),
            )
            .host_factory(Box::new(PointerHostFactory::new(pointer.clone())))
            .unwrap()
            .host_factory(Box::new(ComputeCommandHostFactory::new(compute.clone())))
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
            .load_tree_program(&cpu_projection_tree(&tree), durability)
            .unwrap();
        runtime.start_input_drivers().unwrap();
        pointer.submit(0.25, -0.5, true, 1.0 / 60.0).unwrap();
        runtime.drain_host_inputs(1).unwrap();
        let state = compute.state.lock().unwrap();
        assert!(state.dispatch);
        assert_eq!(state.changed_inputs["force-x"], vec![0.25]);
        assert_eq!(state.changed_inputs["force-y"], vec![-0.5]);
        assert_eq!(state.changed_inputs["force-strength"], vec![1.25]);
    }

    #[test]
    fn auto_selects_cpu_when_compute_gpu_is_unavailable() {
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();
        let tree = mech_syntax::parse(SOURCE).unwrap();
        let prepared = compile_named_compute_region(&document, &tree, 0.0, "auto", false).unwrap();

        assert_eq!(prepared.backend, ComputeBackend::Cpu);
        let mut cpu = prepared.program.into_cpu(&prepared.inputs).unwrap();
        cpu.dispatch_turns(2).unwrap();
        assert_eq!(cpu.output("result.0").unwrap().len(), 8);
    }

    #[test]
    fn hard_region_placement_conflicts_are_reported_by_browser_compiler() {
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();

        let hard_gpu = SOURCE.replacen("@ compute", "@ gpu", 1);
        let tree = mech_syntax::parse(&hard_gpu).unwrap();
        let error = format!(
            "{:?}",
            compile_named_compute_region(&document, &tree, 0.0, "cpu", true).unwrap_err()
        );
        assert!(error.contains("requires GPU execution"), "{error}");

        let hard_cpu = SOURCE.replacen("@ compute", "@ cpu", 1);
        let tree = mech_syntax::parse(&hard_cpu).unwrap();
        let error = format!(
            "{:?}",
            compile_named_compute_region(&document, &tree, 0.0, "gpu", true).unwrap_err()
        );
        assert!(error.contains("requires CPU execution"), "{error}");
    }
}
