use std::collections::BTreeMap;
use std::sync::{
    Arc, LazyLock, Mutex,
    atomic::{AtomicBool, Ordering},
};

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult, MechError,
    MechErrorKind, OperationContractDeclaration, Ref,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest,
};

use crate::{GpuBindingRole, GpuProgram, ResidentGpuSession, column_major_to_row_major};

static GPU_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
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

#[derive(Clone, Debug)]
pub struct GpuRegionProgram {
    program: Arc<GpuProgram>,
    inputs: Arc<BTreeMap<String, Vec<f32>>>,
}

impl GpuRegionProgram {
    pub fn new(program: GpuProgram, inputs: BTreeMap<String, Vec<f32>>) -> Self {
        Self {
            program: Arc::new(program),
            inputs: Arc::new(inputs),
        }
    }

    pub fn from_initializers(
        program: GpuProgram,
        initializers: &BTreeMap<String, RuntimeHostInputValue>,
    ) -> MResult<Self> {
        let input_elements = program
            .bindings()
            .iter()
            .filter(|binding| binding.role() == GpuBindingRole::Input)
            .map(|binding| (binding.name.clone(), binding.elements as usize))
            .collect::<BTreeMap<_, _>>();
        let mut inputs = BTreeMap::new();
        for name in input_elements.keys() {
            let initializer = initializers.get(name).ok_or_else(|| {
                gpu_host_error(
                    "GpuRegionHostInitialize",
                    format!("GPU input `{name}` has no declaration-time value"),
                )
            })?;
            let value = initializer.clone().into_mech_value()?;
            inputs.insert(
                name.clone(),
                validated_gpu_input_values(name, value, &input_elements)?,
            );
        }
        Ok(Self::new(program, inputs))
    }
}

#[derive(Debug)]
pub struct GpuRegionHostFactory {
    programs: BTreeMap<String, GpuRegionProgram>,
    manifest: HostManifestConfig,
}

impl GpuRegionHostFactory {
    pub fn new(programs: BTreeMap<String, GpuRegionProgram>) -> MResult<Self> {
        if programs.is_empty() {
            return Err(gpu_host_error(
                "GpuRegionHostFactory",
                "the source document contains no GPU compute regions",
            ));
        }
        Ok(Self {
            programs,
            manifest: gpu_host_manifest(),
        })
    }
}

impl RuntimeHostFactory for GpuRegionHostFactory {
    fn provider_name(&self) -> &str {
        "gpu"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        let region = configured_region(settings)?;
        if !self.programs.contains_key(&region) {
            return Err(gpu_host_error(
                "GpuRegionHostConfig",
                format!("named GPU region `{region}` does not exist in the source document"),
            ));
        }
        Ok(())
    }

    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance_name, settings)?;
        let region = configured_region(settings)?;
        let prepared = &self.programs[&region];
        let session = prepared
            .program
            .prepare_resident(prepared.inputs.as_ref())
            .map_err(|error| {
                gpu_host_error(
                    "GpuRegionHostInitialize",
                    format!("region `{region}` could not initialize: {error}"),
                )
            })?;
        let state = GpuRegionState {
            adapter: session.adapter().to_owned(),
            turns: Ref::new(0.0),
            dispatch_ms: Ref::new(0.0),
            session,
        };
        let input_elements = Arc::new(
            prepared
                .inputs
                .iter()
                .map(|(name, values)| (name.clone(), values.len()))
                .collect(),
        );
        let telemetry = Arc::new(Mutex::new(None));
        let live = Arc::new(AtomicBool::new(false));
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(GpuRegionResourceProvider {
                instance: instance_name.to_owned(),
                region,
                state: Arc::new(Mutex::new(state)),
                input_elements,
                telemetry: telemetry.clone(),
            })],
            input_drivers: vec![Box::new(GpuTelemetryDriver {
                base_uri: format!("gpu://{instance_name}/kernel"),
                ingress: telemetry,
                live,
            })],
        })
    }
}

#[derive(Debug)]
struct GpuRegionState {
    adapter: String,
    turns: Ref<f64>,
    dispatch_ms: Ref<f64>,
    session: ResidentGpuSession,
}

#[derive(Debug)]
pub struct GpuRegionResourceProvider {
    instance: String,
    region: String,
    state: Arc<Mutex<GpuRegionState>>,
    input_elements: Arc<BTreeMap<String, usize>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl GpuRegionResourceProvider {
    fn base_uri(&self) -> String {
        format!("gpu://{}/kernel", self.instance)
    }

    fn value_for(&self, path: &str, planning: bool) -> MResult<LegacyValue> {
        if planning {
            return match path {
                "adapter" => Ok(LegacyValue::String(Ref::new(String::new()))),
                "turns" | "dispatch-ms" => Ok(LegacyValue::F64(Ref::new(0.0))),
                other => Err(gpu_host_error(
                    "GpuRegionHostRead",
                    format!("unknown GPU telemetry path `{other}`"),
                )),
            };
        }
        let state = self.state.lock().map_err(|_| {
            gpu_host_error("GpuRegionHostRead", "GPU region state lock is poisoned")
        })?;
        match path {
            "adapter" => Ok(LegacyValue::String(Ref::new(state.adapter.clone()))),
            "turns" => Ok(LegacyValue::F64(state.turns.clone())),
            "dispatch-ms" => Ok(LegacyValue::F64(state.dispatch_ms.clone())),
            other => Err(gpu_host_error(
                "GpuRegionHostRead",
                format!("unknown GPU telemetry path `{other}`"),
            )),
        }
    }
}

impl RuntimeResourceProvider for GpuRegionResourceProvider {
    fn scheme(&self) -> &str {
        "gpu"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![self.base_uri()]
    }

    fn semantic_read_contract(&self) -> Option<&'static mech_core::OperationContractDeclaration> {
        Some(mech_runtime::resource_observation_contract())
    }

    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static mech_core::OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&GPU_EFFECT_CONTRACT)
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != self.base_uri() {
            return Err(gpu_host_error(
                "GpuRegionHostRead",
                format!("unknown GPU resource `{}`", request.base_uri),
            ));
        }
        self.value_for(&request.path, true)
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != self.base_uri() {
            return Err(gpu_host_error(
                "GpuRegionHostRead",
                format!("unknown GPU resource `{}`", request.base_uri),
            ));
        }
        self.value_for(&request.path, false)
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.base_uri() {
            return Err(gpu_host_error(
                "GpuRegionHostWrite",
                format!("unknown GPU resource `{}`", request.base_uri),
            ));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(gpu_host_error(
                "GpuRegionHostWrite",
                "GPU dispatch is an effect; use <-",
            ));
        }
        if request.path != "turn" && self.declared_input(&request.path).is_none() {
            return Err(gpu_host_error(
                "GpuRegionHostWrite",
                format!("unknown GPU input path `{}`", request.path),
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
        if let Some(name) = self.declared_input(&request.path) {
            validated_gpu_input_values(
                name,
                request.value.try_deep_snapshot()?,
                self.input_elements.as_ref(),
            )?;
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
        if let Some(name) = self.declared_input(&request.path).map(str::to_owned) {
            let values = validated_gpu_input_values(
                &name,
                request.value.try_deep_snapshot()?,
                self.input_elements.as_ref(),
            )?;
            return Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
                GpuRegionInputEffect {
                    resource: request.base_uri,
                    region: self.region.clone(),
                    name,
                    values,
                    state: self.state.clone(),
                },
            )));
        }
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            GpuRegionDispatchEffect {
                resource: request.base_uri,
                region: self.region.clone(),
                state: self.state.clone(),
                telemetry: self.telemetry.clone(),
            },
        )))
    }
}

impl GpuRegionResourceProvider {
    fn declared_input<'a>(&self, path: &'a str) -> Option<&'a str> {
        declared_gpu_input(self.input_elements.as_ref(), path)
    }
}

fn declared_gpu_input<'a>(
    input_elements: &BTreeMap<String, usize>,
    path: &'a str,
) -> Option<&'a str> {
    let name = path.strip_prefix("input/")?;
    (!name.is_empty() && input_elements.contains_key(name)).then_some(name)
}

#[derive(Debug)]
struct GpuRegionInputEffect {
    resource: String,
    region: String,
    name: String,
    values: Vec<f32>,
    state: Arc<Mutex<GpuRegionState>>,
}

impl RuntimeAfterCommitEffect for GpuRegionInputEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        let items = u64::try_from(self.values.len()).unwrap_or(u64::MAX);
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gpu".to_owned(),
            },
            format!("input:{}:{}", self.region, self.name),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost {
            bytes: items.saturating_mul(std::mem::size_of::<f32>() as u64),
            items,
        })
    }

    fn deliver(&mut self) -> MResult<()> {
        let state = self.state.lock().map_err(|_| {
            gpu_host_error("GpuRegionHostWrite", "GPU region state lock is poisoned")
        })?;
        state
            .session
            .update_input(&self.name, &self.values)
            .map_err(|error| {
                gpu_host_error(
                    "GpuRegionHostWrite",
                    format!(
                        "input `{}` for region `{}` failed: {error}",
                        self.name, self.region
                    ),
                )
            })
    }
}

fn gpu_input_values(value: LegacyValue) -> MResult<Vec<f32>> {
    match value {
        LegacyValue::Typed(value, _) => gpu_input_values(*value),
        LegacyValue::MutableReference(value) => gpu_input_values(value.borrow().clone()),
        LegacyValue::F32(value) => Ok(vec![*value.borrow()]),
        LegacyValue::F64(value) => Ok(vec![*value.borrow() as f32]),
        LegacyValue::MatrixF32(matrix) => {
            column_major_to_row_major(matrix.rows(), matrix.cols(), &matrix.as_vec())
                .map_err(|reason| gpu_host_error("GpuRegionHostWrite", reason))
        }
        LegacyValue::MatrixF64(matrix) => {
            column_major_to_row_major(matrix.rows(), matrix.cols(), &matrix.as_vec())
                .map(|values| values.into_iter().map(|value| value as f32).collect())
                .map_err(|reason| gpu_host_error("GpuRegionHostWrite", reason))
        }
        other => Err(gpu_host_error(
            "GpuRegionHostWrite",
            format!(
                "GPU inputs must be f32 scalars or matrices, found `{}`",
                other.kind()
            ),
        )),
    }
}

fn validated_gpu_input_values(
    name: &str,
    value: LegacyValue,
    input_elements: &BTreeMap<String, usize>,
) -> MResult<Vec<f32>> {
    let values = gpu_input_values(value)?;
    let expected = input_elements.get(name).copied().ok_or_else(|| {
        gpu_host_error(
            "GpuRegionHostWrite",
            format!("GPU region has no input named `{name}`"),
        )
    })?;
    if values.len() != expected {
        return Err(gpu_host_error(
            "GpuRegionHostWrite",
            format!(
                "GPU input `{name}` expects {expected} f32 values, found {}",
                values.len()
            ),
        ));
    }
    Ok(values)
}

#[derive(Debug)]
struct GpuRegionDispatchEffect {
    resource: String,
    region: String,
    state: Arc<Mutex<GpuRegionState>>,
    telemetry: Arc<Mutex<Option<RuntimeIngress>>>,
}

impl RuntimeAfterCommitEffect for GpuRegionDispatchEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "gpu".to_owned(),
            },
            format!("dispatch:{}", self.region),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut state = self.state.lock().map_err(|_| {
            gpu_host_error("GpuRegionHostDispatch", "GPU region state lock is poisoned")
        })?;
        let elapsed = state.session.dispatch_turns(1).map_err(|error| {
            gpu_host_error(
                "GpuRegionHostDispatch",
                format!("region `{}` failed: {error}", self.region),
            )
        })?;
        *state.turns.borrow_mut() += 1.0;
        *state.dispatch_ms.borrow_mut() = elapsed.as_secs_f64() * 1_000.0;
        let turns = *state.turns.borrow();
        let dispatch_ms = *state.dispatch_ms.borrow();
        drop(state);
        if let Some(ingress) = self
            .telemetry
            .lock()
            .map_err(|_| {
                gpu_host_error(
                    "GpuRegionHostDispatch",
                    "GPU telemetry ingress lock is poisoned",
                )
            })?
            .as_ref()
        {
            ingress.submit(RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new(&self.resource, "turns")?,
                    value: RuntimeHostInputValue::F64(turns),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new(&self.resource, "dispatch-ms")?,
                    value: RuntimeHostInputValue::F64(dispatch_ms),
                },
            ])?)?;
        }
        Ok(())
    }
}

#[derive(Debug)]
struct GpuTelemetryDriver {
    base_uri: String,
    ingress: Arc<Mutex<Option<RuntimeIngress>>>,
    live: Arc<AtomicBool>,
}

impl RuntimeHostInputDriver for GpuTelemetryDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == self.base_uri && matches!(source.path(), "turns" | "dispatch-ms")
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        let mut attached = self.ingress.lock().map_err(|_| {
            gpu_host_error(
                "GpuTelemetryDriver",
                "GPU telemetry ingress lock is poisoned",
            )
        })?;
        if attached.is_some() {
            return Err(gpu_host_error(
                "GpuTelemetryDriver",
                "GPU telemetry driver is already attached",
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
                gpu_host_error(
                    "GpuTelemetryDriver",
                    "GPU telemetry ingress lock is poisoned",
                )
            })?
            .is_none()
        {
            return Err(gpu_host_error(
                "GpuTelemetryDriver",
                "GPU telemetry driver must be attached before start",
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

pub fn gpu_host_manifest() -> HostManifestConfig {
    HostManifestConfig {
        provider: "gpu".to_owned(),
        contexts: vec![mech_runtime::HostContextManifest {
            name: "kernel".to_owned(),
            base_uri_template: "gpu://{instance}/kernel".to_owned(),
            operations: vec!["read".to_owned(), "write".to_owned()],
        }],
    }
}

fn configured_region(settings: &ConfigValue) -> MResult<String> {
    let ConfigValue::Map(map) = settings else {
        return Err(gpu_host_error(
            "GpuRegionHostConfig",
            "GPU host settings must be a map",
        ));
    };
    for key in map.keys() {
        if key != "region" {
            return Err(gpu_host_error(
                "GpuRegionHostConfig",
                format!("unknown GPU host setting `{key}`"),
            ));
        }
    }
    let Some(ConfigValue::String(region)) = map.get("region") else {
        return Err(gpu_host_error(
            "GpuRegionHostConfig",
            "GPU host setting `region` must be a string",
        ));
    };
    if region.trim().is_empty() {
        return Err(gpu_host_error(
            "GpuRegionHostConfig",
            "GPU host setting `region` must be non-empty",
        ));
    }
    Ok(region.clone())
}

#[derive(Clone, Debug)]
struct GpuRegionHostError {
    name: &'static str,
    message: String,
}

impl MechErrorKind for GpuRegionHostError {
    fn name(&self) -> &str {
        self.name
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

fn gpu_host_error(name: &'static str, message: impl Into<String>) -> MechError {
    MechError::new(
        GpuRegionHostError {
            name,
            message: message.into(),
        },
        None,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{GpuBinding, GpuBindingAccess, GpuBindingKind};
    use mech_core::CellSlotId;

    #[test]
    fn manifest_exposes_one_read_write_kernel_context() {
        let manifest = gpu_host_manifest();
        assert_eq!(manifest.provider, "gpu");
        assert_eq!(manifest.contexts.len(), 1);
        assert_eq!(manifest.contexts[0].name, "kernel");
        assert_eq!(
            manifest.contexts[0].base_uri_template,
            "gpu://{instance}/kernel"
        );
        assert_eq!(manifest.contexts[0].operations, ["read", "write"]);
    }

    #[test]
    fn region_setting_is_explicit_and_rejects_unknown_keys() {
        let settings = ConfigValue::Map(BTreeMap::from([(
            "region".to_owned(),
            ConfigValue::String("particle-field".to_owned()),
        )]));
        assert_eq!(configured_region(&settings).unwrap(), "particle-field");

        let bad = ConfigValue::Map(BTreeMap::from([(
            "source".to_owned(),
            ConfigValue::String("magic.mec".to_owned()),
        )]));
        assert!(configured_region(&bad).is_err());
    }

    #[test]
    fn only_declared_input_paths_are_writable() {
        let inputs = BTreeMap::from([("matrix".to_owned(), 6)]);
        assert_eq!(declared_gpu_input(&inputs, "input/matrix"), Some("matrix"));
        assert_eq!(declared_gpu_input(&inputs, "input/missing"), None);
        assert_eq!(declared_gpu_input(&inputs, "input/"), None);
        assert_eq!(declared_gpu_input(&inputs, "matrix"), None);
    }

    #[test]
    fn mech_column_major_matrix_inputs_cross_to_gpu_row_major_order() {
        let value = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 3,
            values: vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
        }
        .into_mech_value()
        .unwrap();

        assert_eq!(
            gpu_input_values(value).unwrap(),
            vec![1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn region_program_preserves_source_initializers_in_kernel_layout() {
        let program = GpuProgram {
            wgsl: String::new(),
            bindings: vec![GpuBinding {
                binding: 0,
                name: "matrix".to_owned(),
                access: GpuBindingAccess::Read,
                elements: 6,
                kind: GpuBindingKind::Input(CellSlotId(0)),
            }],
            operations: Vec::new(),
            outputs: Vec::new(),
            states: Vec::new(),
            input_slots: BTreeMap::new(),
            constants: BTreeMap::new(),
            dispatch_elements: 6,
        };
        let initializers = BTreeMap::from([(
            "matrix".to_owned(),
            RuntimeHostInputValue::F32Matrix {
                rows: 2,
                columns: 3,
                values: vec![1.0, 4.0, 2.0, 5.0, 3.0, 6.0],
            },
        )]);

        let prepared = GpuRegionProgram::from_initializers(program, &initializers).unwrap();
        assert_eq!(
            prepared.inputs["matrix"].as_slice(),
            [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
        );
    }

    #[test]
    fn gpu_input_planning_rejects_wrong_shapes_before_delivery() {
        let inputs = BTreeMap::from([("matrix".to_owned(), 6)]);
        let value = RuntimeHostInputValue::F32Matrix {
            rows: 2,
            columns: 2,
            values: vec![1.0, 3.0, 2.0, 4.0],
        }
        .into_mech_value()
        .unwrap();

        let error = validated_gpu_input_values("matrix", value, &inputs).unwrap_err();
        assert!(format!("{error:?}").contains("expects 6 f32 values, found 4"));
    }
}
