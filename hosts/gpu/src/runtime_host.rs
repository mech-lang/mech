use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use mech_core::{LegacyValue, MResult, MechError, MechErrorKind, Ref};
use mech_engine::{MechProgram, MechProgramConfig};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostManifestConfig, PreparedRuntimeEffect,
    RuntimeAfterCommitEffect, RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeHostFactory, RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource,
    RuntimeHostInputUpdate, RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, materialize_host_manifest,
    provider_defined_effect_contract,
};

use crate::{GpuHost, ResidentCpuSession, ResidentGpuSession};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GpuRuntimeBackend {
    Wgpu,
    Cpu,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GpuRuntimeHostSettings {
    pub source: PathBuf,
    pub backend: GpuRuntimeBackend,
    pub turns_per_dispatch: u32,
    pub inputs: BTreeMap<String, f32>,
}

#[derive(Clone, Debug)]
struct GpuRuntimeHostError {
    operation: &'static str,
    reason: String,
}

impl MechErrorKind for GpuRuntimeHostError {
    fn name(&self) -> &str {
        "GpuRuntimeHostError"
    }

    fn message(&self) -> String {
        format!("{} failed: {}", self.operation, self.reason)
    }
}

fn host_error<T>(operation: &'static str, reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        GpuRuntimeHostError {
            operation,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

pub fn gpu_runtime_host_manifest() -> HostManifestConfig {
    HostManifestConfig {
        provider: "gpu".to_owned(),
        contexts: vec![HostContextManifest {
            name: "kernel".to_owned(),
            base_uri_template: "gpu://{instance}/kernel".to_owned(),
            operations: vec!["read".to_owned(), "write".to_owned()],
        }],
    }
}

pub fn gpu_runtime_host_settings(settings: &ConfigValue) -> MResult<GpuRuntimeHostSettings> {
    let ConfigValue::Map(settings) = settings else {
        return host_error("validate-settings", "GPU host settings must be a map");
    };
    for key in settings.keys() {
        if !matches!(
            key.as_str(),
            "source" | "backend" | "backends" | "turns-per-dispatch" | "inputs"
        ) {
            return host_error(
                "validate-settings",
                format!("unknown GPU host setting `{key}`"),
            );
        }
    }
    let source = match settings.get("source") {
        Some(ConfigValue::String(source)) if !source.trim().is_empty() => PathBuf::from(source),
        Some(_) => return host_error("validate-settings", "GPU host `source` must be a string"),
        None => return host_error("validate-settings", "GPU host `source` is required"),
    };
    let backend = match settings.get("backend") {
        None => GpuRuntimeBackend::Wgpu,
        Some(ConfigValue::String(backend)) if matches!(backend.as_str(), "auto" | "wgpu") => {
            GpuRuntimeBackend::Wgpu
        }
        Some(ConfigValue::String(backend)) if backend == "cpu" => GpuRuntimeBackend::Cpu,
        Some(ConfigValue::String(backend)) => {
            return host_error(
                "validate-settings",
                format!("GPU backend `{backend}` is unsupported; use `auto`, `wgpu`, or `cpu`"),
            );
        }
        Some(_) => return host_error("validate-settings", "GPU host `backend` must be a string"),
    };
    if let Some(value) = settings.get("backends") {
        let ConfigValue::List(values) = value else {
            return host_error("validate-settings", "GPU host `backends` must be a list");
        };
        let mut available = BTreeSet::new();
        for value in values {
            let ConfigValue::String(value) = value else {
                return host_error(
                    "validate-settings",
                    "GPU host `backends` entries must be strings",
                );
            };
            let parsed = match value.as_str() {
                "wgpu" => GpuRuntimeBackend::Wgpu,
                "cpu" => GpuRuntimeBackend::Cpu,
                _ => {
                    return host_error(
                        "validate-settings",
                        format!("GPU backend list contains unsupported backend `{value}`"),
                    );
                }
            };
            available.insert(parsed);
        }
        if !available.contains(&backend) {
            return host_error(
                "validate-settings",
                "selected GPU backend is not declared in `backends`",
            );
        }
    }
    let turns_per_dispatch = match settings.get("turns-per-dispatch") {
        None => 1,
        Some(ConfigValue::Integer(turns)) if *turns > 0 => u32::try_from(*turns).map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "validate-settings",
                    reason: "GPU `turns-per-dispatch` exceeds u32".to_owned(),
                },
                None,
            )
        })?,
        Some(ConfigValue::Integer(_)) => {
            return host_error(
                "validate-settings",
                "GPU `turns-per-dispatch` must be positive",
            );
        }
        Some(_) => {
            return host_error(
                "validate-settings",
                "GPU `turns-per-dispatch` must be an integer",
            );
        }
    };
    let inputs = match settings.get("inputs") {
        None => BTreeMap::new(),
        Some(ConfigValue::Map(inputs)) => inputs
            .iter()
            .map(|(name, value)| {
                let value = match value {
                    ConfigValue::Float(value) => *value as f32,
                    ConfigValue::Integer(value) => *value as f32,
                    _ => {
                        return host_error(
                            "validate-settings",
                            format!("GPU input `{name}` must be a scalar number"),
                        );
                    }
                };
                if name.trim().is_empty() || !value.is_finite() {
                    return host_error(
                        "validate-settings",
                        format!("GPU input `{name}` must have a name and finite value"),
                    );
                }
                Ok((name.clone(), value))
            })
            .collect::<MResult<BTreeMap<_, _>>>()?,
        Some(_) => return host_error("validate-settings", "GPU host `inputs` must be a map"),
    };
    Ok(GpuRuntimeHostSettings {
        source,
        backend,
        turns_per_dispatch,
        inputs,
    })
}

#[derive(Debug)]
enum RuntimeKernelSession {
    Wgpu(ResidentGpuSession),
    Cpu(ResidentCpuSession),
}

impl RuntimeKernelSession {
    fn adapter_name(&self) -> &str {
        match self {
            Self::Wgpu(session) => session.adapter_name(),
            Self::Cpu(_) => "Mech fused CPU executor",
        }
    }

    fn write_input(&mut self, name: &str, value: f32) -> Result<(), String> {
        match self {
            Self::Wgpu(session) => session
                .write_input(name, &[value])
                .map_err(|error| error.to_string()),
            Self::Cpu(session) => session
                .write_input(name, &[value])
                .map_err(|error| error.to_string()),
        }
    }

    fn submit_turns(&mut self, turns: u32) -> Result<Duration, String> {
        match self {
            Self::Wgpu(session) => session
                .submit_turns(turns)
                .map_err(|error| error.to_string()),
            Self::Cpu(session) => {
                let started = Instant::now();
                session
                    .dispatch_turns(turns)
                    .map_err(|error| error.to_string())?;
                Ok(started.elapsed())
            }
        }
    }
}

#[derive(Debug)]
struct RuntimeGpuState {
    instance: String,
    settings: GpuRuntimeHostSettings,
    session: Option<RuntimeKernelSession>,
    adapter: String,
    dispatched_turns: u64,
    dispatch_ms: f64,
    ingress: Option<RuntimeIngress>,
    live: bool,
}

impl RuntimeGpuState {
    fn telemetry_input(&self) -> MResult<RuntimeHostInput> {
        let base_uri = format!("gpu://{}/kernel", self.instance);
        RuntimeHostInput::new(vec![
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(base_uri.clone(), "adapter")?,
                value: RuntimeHostInputValue::String(self.adapter.clone()),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(base_uri.clone(), "turns")?,
                value: RuntimeHostInputValue::F64(self.dispatched_turns as f64),
            },
            RuntimeHostInputUpdate {
                source: RuntimeHostInputSource::new(base_uri, "dispatch-ms")?,
                value: RuntimeHostInputValue::F64(self.dispatch_ms),
            },
        ])
    }
}

impl RuntimeGpuState {
    fn dispatch(&mut self) -> MResult<()> {
        if self.session.is_none() {
            let source = std::fs::read_to_string(&self.settings.source).map_err(|error| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "read-kernel-source",
                        reason: format!("{}: {error}", self.settings.source.display()),
                    },
                    None,
                )
            })?;
            let mut source_program = MechProgram::with_function_catalog(
                MechProgramConfig::default(),
                mech_stdlib::source_catalog(),
            );
            source_program.run_string(&source)?;
            let external_inputs = self
                .settings
                .inputs
                .keys()
                .cloned()
                .collect::<BTreeSet<_>>();
            let artifact = source_program
                .compile_program_product_with_external_inputs(&external_inputs)?
                .into_parts()
                .0;
            let program = GpuHost.compile(&artifact).map_err(|error| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "compile-kernel",
                        reason: error.to_string(),
                    },
                    None,
                )
            })?;
            let inputs = self
                .settings
                .inputs
                .iter()
                .map(|(name, value)| (name.clone(), vec![*value]))
                .collect::<BTreeMap<_, _>>();
            let session = match self.settings.backend {
                GpuRuntimeBackend::Wgpu => program
                    .prepare_resident(&inputs)
                    .map(RuntimeKernelSession::Wgpu)
                    .map_err(|error| {
                        MechError::new(
                            GpuRuntimeHostError {
                                operation: "prepare-kernel",
                                reason: error.to_string(),
                            },
                            None,
                        )
                    })?,
                GpuRuntimeBackend::Cpu => program
                    .prepare_cpu(&inputs)
                    .map(RuntimeKernelSession::Cpu)
                    .map_err(|error| {
                        MechError::new(
                            GpuRuntimeHostError {
                                operation: "prepare-kernel",
                                reason: error.to_string(),
                            },
                            None,
                        )
                    })?,
            };
            self.adapter = session.adapter_name().to_owned();
            self.session = Some(session);
        }
        for (name, value) in &self.settings.inputs {
            self.session
                .as_mut()
                .expect("session initialized above")
                .write_input(name, *value)
                .map_err(|error| {
                    MechError::new(
                        GpuRuntimeHostError {
                            operation: "upload-kernel-input",
                            reason: error.to_string(),
                        },
                        None,
                    )
                })?;
        }
        let elapsed = self
            .session
            .as_mut()
            .expect("session initialized above")
            .submit_turns(self.settings.turns_per_dispatch)
            .map_err(|error| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "dispatch-kernel",
                        reason: error.to_string(),
                    },
                    None,
                )
            })?;
        self.dispatched_turns = self
            .dispatched_turns
            .saturating_add(u64::from(self.settings.turns_per_dispatch));
        self.dispatch_ms = elapsed.as_secs_f64() * 1_000.0;
        Ok(())
    }
}

#[derive(Debug)]
pub struct GpuRuntimeResourceProvider {
    instance: String,
    state: Arc<Mutex<RuntimeGpuState>>,
}

impl GpuRuntimeResourceProvider {
    pub fn new(instance: impl Into<String>, settings: GpuRuntimeHostSettings) -> Self {
        let instance = instance.into();
        Self {
            instance: instance.clone(),
            state: Arc::new(Mutex::new(RuntimeGpuState {
                instance,
                settings,
                session: None,
                adapter: "not-initialized".to_owned(),
                dispatched_turns: 0,
                dispatch_ms: 0.0,
                ingress: None,
                live: false,
            })),
        }
    }

    fn from_shared(instance: impl Into<String>, state: Arc<Mutex<RuntimeGpuState>>) -> Self {
        Self {
            instance: instance.into(),
            state,
        }
    }

    fn base_uri(&self) -> String {
        format!("gpu://{}/kernel", self.instance)
    }

    fn validate_base(&self, base_uri: &str) -> MResult<()> {
        if base_uri == self.base_uri() {
            Ok(())
        } else {
            host_error(
                "address-resource",
                format!("unknown GPU resource `{base_uri}`"),
            )
        }
    }
}

impl RuntimeResourceProvider for GpuRuntimeResourceProvider {
    fn scheme(&self) -> &str {
        "gpu"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then(provider_defined_effect_contract)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.validate_base(&request.base_uri)?;
        match request.path.as_str() {
            "adapter" => Ok(LegacyValue::String(Ref::new("planning".to_owned()))),
            "turns" | "dispatch-ms" => Ok(LegacyValue::F64(Ref::new(0.0))),
            path => host_error("plan-read", format!("unknown GPU telemetry path `{path}`")),
        }
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.validate_base(&request.base_uri)?;
        let state = self.state.lock().map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "read-telemetry",
                    reason: "GPU state lock is poisoned".to_owned(),
                },
                None,
            )
        })?;
        match request.path.as_str() {
            "adapter" => Ok(LegacyValue::String(Ref::new(state.adapter.clone()))),
            "turns" => Ok(LegacyValue::F64(Ref::new(state.dispatched_turns as f64))),
            "dispatch-ms" => Ok(LegacyValue::F64(Ref::new(state.dispatch_ms))),
            path => host_error(
                "read-telemetry",
                format!("unknown GPU telemetry path `{path}`"),
            ),
        }
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.validate_base(&request.base_uri)?;
        let state = self.state.lock().map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "preflight-dispatch",
                    reason: "GPU state lock is poisoned".to_owned(),
                },
                None,
            )
        })?;
        let input = request.path.strip_prefix("input/");
        if request.intent != RuntimeResourceWriteIntent::Send
            || (request.path != "turn"
                && !input.is_some_and(|name| state.settings.inputs.contains_key(name)))
        {
            return host_error(
                "preflight-dispatch",
                "GPU kernel accepts `turn` or a configured `input/<name>` send",
            );
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
            context_name: request.context_name.clone(),
            operation: request.operation.clone(),
            intent: request.intent,
        })?;
        if request.path == "turn" {
            return Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
                GpuDispatchEffect {
                    state: Arc::clone(&self.state),
                    resource: request.base_uri,
                },
            )));
        }
        let name = request
            .path
            .strip_prefix("input/")
            .expect("preflight accepted a configured input")
            .to_owned();
        let value = match request.value {
            LegacyValue::F32(value) => *value.borrow(),
            LegacyValue::F64(value) => *value.borrow() as f32,
            value => {
                return host_error(
                    "prepare-input",
                    format!(
                        "GPU input `{name}` needs a scalar float, got {:?}",
                        value.kind()
                    ),
                );
            }
        };
        if !value.is_finite() {
            return host_error(
                "prepare-input",
                format!("GPU input `{name}` must be finite"),
            );
        }
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            GpuInputEffect {
                state: Arc::clone(&self.state),
                resource: request.base_uri,
                name,
                value,
            },
        )))
    }
}

#[derive(Debug)]
struct GpuInputEffect {
    state: Arc<Mutex<RuntimeGpuState>>,
    resource: String,
    name: String,
    value: f32,
}

impl RuntimeAfterCommitEffect for GpuInputEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gpu".to_owned(),
            },
            "write-input",
        )
        .with_resource(format!("{}/input/{}", self.resource, self.name))
        .with_cost(RuntimeEffectCost { bytes: 4, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "write-input",
                    reason: "GPU state lock is poisoned".to_owned(),
                },
                None,
            )
        })?;
        state.settings.inputs.insert(self.name.clone(), self.value);
        Ok(())
    }
}

#[derive(Debug)]
struct GpuDispatchEffect {
    state: Arc<Mutex<RuntimeGpuState>>,
    resource: String,
}

impl RuntimeAfterCommitEffect for GpuDispatchEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gpu".to_owned(),
            },
            "dispatch-turn",
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let (ingress, input) = {
            let mut state = self.state.lock().map_err(|_| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "dispatch-kernel",
                        reason: "GPU state lock is poisoned".to_owned(),
                    },
                    None,
                )
            })?;
            state.dispatch()?;
            (state.ingress.clone(), state.telemetry_input()?)
        };
        ingress
            .ok_or_else(|| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "publish-telemetry",
                        reason: "GPU input driver is not attached".to_owned(),
                    },
                    None,
                )
            })?
            .submit(input)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn telemetry_driver_claims_only_its_gpu_read_paths() {
        let state = Arc::new(Mutex::new(RuntimeGpuState {
            instance: "particles".to_owned(),
            settings: GpuRuntimeHostSettings {
                source: PathBuf::from("kernel.mec"),
                backend: GpuRuntimeBackend::Wgpu,
                turns_per_dispatch: 1,
                inputs: BTreeMap::new(),
            },
            session: None,
            adapter: "not-initialized".to_owned(),
            dispatched_turns: 0,
            dispatch_ms: 0.0,
            ingress: None,
            live: false,
        }));
        let driver = GpuTelemetryInputDriver {
            instance: "particles".to_owned(),
            state,
        };

        assert!(
            driver
                .drives(&RuntimeHostInputSource::new("gpu://particles/kernel", "adapter").unwrap())
        );
        assert!(
            driver.drives(&RuntimeHostInputSource::new("gpu://particles/kernel", "turns").unwrap())
        );
        assert!(
            !driver.drives(&RuntimeHostInputSource::new("gpu://particles/kernel", "turn").unwrap())
        );
        assert!(
            !driver.drives(&RuntimeHostInputSource::new("gpu://other/kernel", "turns").unwrap())
        );
    }

    #[test]
    fn host_inputs_and_recurrent_state_compile_to_distinct_artifact_roles() {
        let mut program = MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            mech_stdlib::source_catalog(),
        );
        program
            .run_string("drive := 0.0\n~x := [1.0 2.0]\nnext-x := x + drive\nx = next-x\nx")
            .unwrap();

        let product = program
            .compile_program_product_with_external_inputs(
                &["drive".to_owned()].into_iter().collect(),
            )
            .unwrap();
        let artifact = product.artifact();
        assert_eq!(artifact.inputs().len(), 1);
        assert_eq!(artifact.inputs()[0].name, "drive");
        assert_eq!(
            artifact
                .slots()
                .iter()
                .filter(|slot| slot.role == mech_engine::SlotRole::State)
                .count(),
            1,
        );
        let gpu_program = GpuHost.compile(artifact).unwrap();
        assert!(gpu_program.bindings().iter().any(|binding| {
            binding.role() == crate::GpuBindingRole::Input && binding.name == "drive"
        }));

        let decoded = mech_engine::decode_program_artifact_bytecode_v1(product.bytecode()).unwrap();
        assert_eq!(decoded.inputs(), artifact.inputs());
        assert_eq!(decoded.inputs()[0].name, "drive");
    }

    #[test]
    fn configured_cpu_backend_drives_the_same_resident_kernel_inputs() {
        fn run_with_force(force_enabled: f32) -> Vec<f32> {
            let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../examples/mixed-cpu-gpu-particles/kernel.mec");
            let mut state = RuntimeGpuState {
                instance: "particles".to_owned(),
                settings: GpuRuntimeHostSettings {
                    source,
                    backend: GpuRuntimeBackend::Cpu,
                    turns_per_dispatch: 1,
                    inputs: BTreeMap::from([
                        ("force-x".to_owned(), 0.8),
                        ("force-y".to_owned(), -0.3),
                        ("force-enabled".to_owned(), force_enabled),
                        ("dt".to_owned(), 0.016),
                    ]),
                },
                session: None,
                adapter: "not-initialized".to_owned(),
                dispatched_turns: 0,
                dispatch_ms: 0.0,
                ingress: None,
                live: false,
            };

            state.dispatch().unwrap();
            assert_eq!(state.dispatched_turns, 1);
            assert_eq!(state.adapter, "Mech fused CPU executor");
            let Some(RuntimeKernelSession::Cpu(session)) = state.session.as_ref() else {
                panic!("configured CPU backend must create a CPU session");
            };
            session.outputs().unwrap()["x"].clone()
        }

        let without_force = run_with_force(0.0);
        let with_force = run_with_force(1_000.0);
        assert_ne!(with_force, without_force);
    }
}

#[derive(Debug)]
struct GpuTelemetryInputDriver {
    instance: String,
    state: Arc<Mutex<RuntimeGpuState>>,
}

impl RuntimeHostInputDriver for GpuTelemetryInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == format!("gpu://{}/kernel", self.instance)
            && matches!(source.path(), "adapter" | "turns" | "dispatch-ms")
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "attach-telemetry",
                    reason: "GPU state lock is poisoned".to_owned(),
                },
                None,
            )
        })?;
        if state.ingress.is_some() {
            return host_error(
                "attach-telemetry",
                "GPU telemetry driver is already attached",
            );
        }
        state.ingress = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            MechError::new(
                GpuRuntimeHostError {
                    operation: "start-telemetry",
                    reason: "GPU state lock is poisoned".to_owned(),
                },
                None,
            )
        })?;
        if state.ingress.is_none() {
            return host_error("start-telemetry", "GPU telemetry driver is not attached");
        }
        state.live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| {
                MechError::new(
                    GpuRuntimeHostError {
                        operation: "stop-telemetry",
                        reason: "GPU state lock is poisoned".to_owned(),
                    },
                    None,
                )
            })?
            .live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.state.lock().map(|state| state.live).unwrap_or(false)
    }
}

#[derive(Debug)]
pub struct NativeGpuHostFactory {
    manifest: HostManifestConfig,
}

impl NativeGpuHostFactory {
    pub fn new() -> Self {
        Self {
            manifest: gpu_runtime_host_manifest(),
        }
    }
}

impl Default for NativeGpuHostFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl RuntimeHostFactory for NativeGpuHostFactory {
    fn provider_name(&self) -> &str {
        "gpu"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        gpu_runtime_host_settings(settings).map(|_| ())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let settings = gpu_runtime_host_settings(settings)?;
        let state = Arc::new(Mutex::new(RuntimeGpuState {
            instance: instance_name.to_owned(),
            settings,
            session: None,
            adapter: "not-initialized".to_owned(),
            dispatched_turns: 0,
            dispatch_ms: 0.0,
            ingress: None,
            live: false,
        }));
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(GpuRuntimeResourceProvider::from_shared(
                instance_name,
                Arc::clone(&state),
            ))],
            input_drivers: vec![Box::new(GpuTelemetryInputDriver {
                instance: instance_name.to_owned(),
                state,
            })],
        })
    }
}
