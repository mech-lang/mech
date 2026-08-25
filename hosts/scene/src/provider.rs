use std::sync::{Arc, LazyLock, Mutex};

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult,
    OperationContractDeclaration,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
    RuntimeResourceWriteRequest, materialize_host_manifest, resource_observation_contract,
};

use crate::{
    CircleElement, SceneHostSettings, SceneRendererKind, SceneSnapshot, scene_error,
    scene_host_manifest, scene_settings_from_config,
};

const BODY_PALETTE: [&str; 10] = [
    "#ffd166", "#b8b8b8", "#f4a261", "#4cc9f0", "#e76f51", "#f9c74f", "#90be6d", "#577590",
    "#4361ee", "#adb5bd",
];

static SCENE_EFFECT_CONTRACT: LazyLock<OperationContractDeclaration> =
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

const POINTER_PATHS: [&str; 4] = [
    "pointer-pulse",
    "pointer-position",
    "pointer-pressed",
    "pointer-delta-seconds",
];

#[derive(Clone, Debug)]
pub struct ScenePointerHandle {
    base_uri: Arc<str>,
    state: Arc<Mutex<ScenePointerDriverState>>,
}

#[derive(Debug, Default)]
struct ScenePointerDriverState {
    ingress: Option<RuntimeIngress>,
    pulse: u64,
    live: bool,
}

impl ScenePointerHandle {
    pub fn new(instance: impl AsRef<str>) -> Self {
        Self {
            base_uri: format!("scene://{}/frame", instance.as_ref()).into(),
            state: Arc::new(Mutex::new(ScenePointerDriverState::default())),
        }
    }

    pub fn submit(&self, x: f64, y: f64, pressed: bool, delta_seconds: f64) -> MResult<()> {
        let mut state = self
            .state
            .lock()
            .map_err(|_| scene_error("ScenePointer", "pointer state lock is poisoned"))?;
        if !state.live {
            return Err(scene_error(
                "ScenePointer",
                "scene pointer input is not running",
            ));
        }
        // Resident input draining may coalesce a pointer-down and pointer-up
        // packet into one turn. Count activations, not raw state packets, so
        // the final released state cannot erase the click and a separately
        // drained release cannot activate the target twice.
        let pulse = if pressed {
            state.pulse.checked_add(1).ok_or_else(|| {
                scene_error(
                    "ScenePointerSequenceExhausted",
                    "scene pointer activation sequence is exhausted",
                )
            })?
        } else {
            state.pulse
        };
        let ingress = state
            .ingress
            .clone()
            .ok_or_else(|| scene_error("ScenePointer", "scene pointer input is not attached"))?;
        let packet = RuntimeHostInput::new(vec![
            scene_pointer_update(
                &self.base_uri,
                "pointer-pulse",
                RuntimeHostInputValue::F64(pulse as f64),
            )?,
            scene_pointer_update(
                &self.base_uri,
                "pointer-position",
                RuntimeHostInputValue::F64Matrix {
                    rows: 2,
                    columns: 1,
                    values: vec![x, y],
                },
            )?,
            scene_pointer_update(
                &self.base_uri,
                "pointer-pressed",
                RuntimeHostInputValue::F64(f64::from(pressed)),
            )?,
            scene_pointer_update(
                &self.base_uri,
                "pointer-delta-seconds",
                RuntimeHostInputValue::F64(delta_seconds),
            )?,
        ])?
        .with_coalescing_group(pulse);
        // The pulse counter advances only after the complete gesture packet is
        // accepted. Down and up share a group; the next down starts a new one.
        ingress.submit(packet)?;
        state.pulse = pulse;
        Ok(())
    }

    pub fn input_driver(&self, instance: impl Into<String>) -> Box<dyn RuntimeHostInputDriver> {
        Box::new(ScenePointerInputDriver {
            instance: instance.into(),
            state: self.state.clone(),
        })
    }
}

fn scene_pointer_update(
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
struct ScenePointerInputDriver {
    instance: String,
    state: Arc<Mutex<ScenePointerDriverState>>,
}

impl RuntimeHostInputDriver for ScenePointerInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == format!("scene://{}/frame", self.instance)
            && POINTER_PATHS.contains(&source.path())
    }

    fn attach(&mut self, ingress: RuntimeIngress) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| scene_error("ScenePointer", "pointer state lock is poisoned"))?
            .ingress = Some(ingress);
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| scene_error("ScenePointer", "pointer state lock is poisoned"))?
            .live = true;
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        self.state
            .lock()
            .map_err(|_| scene_error("ScenePointer", "pointer state lock is poisoned"))?
            .live = false;
        Ok(())
    }

    fn is_live(&self) -> bool {
        self.state.lock().map(|state| state.live).unwrap_or(false)
    }
}

pub trait SceneBackend: Clone + std::fmt::Debug + 'static {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()>;
}

#[derive(Clone, Debug, Default)]
pub struct RecordingSceneBackend {
    latest: Arc<Mutex<Option<SceneSnapshot>>>,
    generations: Arc<Mutex<u64>>,
}
impl RecordingSceneBackend {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn latest(&self) -> Option<SceneSnapshot> {
        self.latest.lock().unwrap().clone()
    }
    pub fn generation(&self) -> u64 {
        *self.generations.lock().unwrap()
    }
}
impl SceneBackend for RecordingSceneBackend {
    fn replace_scene(&mut self, scene: SceneSnapshot) -> MResult<()> {
        *self
            .latest
            .lock()
            .map_err(|_| scene_error("SceneBackend", "scene backend lock is poisoned"))? =
            Some(scene);
        *self
            .generations
            .lock()
            .map_err(|_| scene_error("SceneBackend", "scene generation lock is poisoned"))? += 1;
        Ok(())
    }
}

#[derive(Debug)]
pub struct SceneResourceProvider<B: SceneBackend> {
    instance: String,
    settings: SceneHostSettings,
    backend: Arc<Mutex<B>>,
    last_accepted: Arc<Mutex<Option<SceneSnapshot>>>,
}
impl<B: SceneBackend> SceneResourceProvider<B> {
    pub fn new(instance: impl Into<String>, backend: B) -> Self {
        Self::new_with_settings(
            instance,
            backend,
            SceneHostSettings::new("#scene", SceneRendererKind::Svg),
        )
    }

    pub fn new_with_settings(
        instance: impl Into<String>,
        backend: B,
        settings: SceneHostSettings,
    ) -> Self {
        Self {
            instance: instance.into(),
            settings,
            backend: Arc::new(Mutex::new(backend)),
            last_accepted: Arc::new(Mutex::new(None)),
        }
    }
    fn base(&self) -> String {
        format!("scene://{}/frame", self.instance)
    }
}
impl<B: SceneBackend> RuntimeResourceProvider for SceneResourceProvider<B> {
    fn scheme(&self) -> &str {
        "scene"
    }
    fn base_uris(&self) -> Vec<String> {
        vec![self.base()]
    }
    fn semantic_write_contract(
        &self,
        intent: RuntimeResourceWriteIntent,
    ) -> Option<&'static OperationContractDeclaration> {
        (intent == RuntimeResourceWriteIntent::Send).then_some(&SCENE_EFFECT_CONTRACT)
    }
    fn semantic_read_contract(&self) -> Option<&'static OperationContractDeclaration> {
        Some(resource_observation_contract())
    }
    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.pointer_value(request)
    }
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        self.pointer_value(request)
    }
    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        if request.base_uri != self.base() {
            return Err(scene_error(
                "SceneResourceProvider",
                format!("unsupported scene resource `{}`", request.base_uri),
            ));
        }
        if request.intent != RuntimeResourceWriteIntent::Send {
            return Err(scene_error(
                "SceneResourceProvider",
                "scene accepts send writes only; use <-",
            ));
        }
        if request.path != "replace" && request.path != "points" {
            return Err(scene_error(
                "SceneResourceProvider",
                "scene frame supports only the `replace` and `points` paths",
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
        match request.path.as_str() {
            "replace" => SceneSnapshot::from_value(&request.value).map(|_| ()),
            "points" => scene_snapshot_from_points(&request.value, &self.settings).map(|_| ()),
            _ => unreachable!("scene path was preflighted"),
        }
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
        let scene = match request.path.as_str() {
            "replace" => SceneSnapshot::from_value(&request.value)?,
            "points" => scene_snapshot_from_points(&request.value, &self.settings)?,
            _ => unreachable!("scene path was preflighted"),
        };
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            SceneReplaceEffect {
                backend: self.backend.clone(),
                last_accepted: self.last_accepted.clone(),
                scene,
                resource: request.base_uri,
                operation: request.path,
            },
        )))
    }
}

impl<B: SceneBackend> SceneResourceProvider<B> {
    fn pointer_value(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if request.base_uri != self.base() || !POINTER_PATHS.contains(&request.path.as_str()) {
            return Err(scene_error(
                "SceneResourceProvider",
                format!(
                    "unknown scene pointer input `{}/{}`",
                    request.base_uri, request.path
                ),
            ));
        }
        if request.path == "pointer-position" {
            RuntimeHostInputValue::F64Matrix {
                rows: 2,
                columns: 1,
                values: vec![0.0, 0.0],
            }
            .into_mech_value()
        } else {
            RuntimeHostInputValue::F64(0.0).into_mech_value()
        }
    }
}

#[derive(Debug)]
struct SceneReplaceEffect<B: SceneBackend> {
    backend: Arc<Mutex<B>>,
    last_accepted: Arc<Mutex<Option<SceneSnapshot>>>,
    scene: SceneSnapshot,
    resource: String,
    operation: String,
}

impl<B: SceneBackend> RuntimeAfterCommitEffect for SceneReplaceEffect<B> {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "scene".to_string(),
            },
            self.operation.clone(),
        )
        .with_resource(self.resource.clone())
        .with_cost(RuntimeEffectCost { bytes: 0, items: 1 })
    }

    fn deliver(&mut self) -> MResult<()> {
        let mut last_accepted = self.last_accepted.lock().map_err(|_| {
            scene_error("SceneResourceProvider", "scene acceptance lock is poisoned")
        })?;
        if last_accepted.as_ref() == Some(&self.scene) {
            return Ok(());
        }
        self.backend
            .lock()
            .map_err(|_| scene_error("SceneResourceProvider", "scene backend lock is poisoned"))?
            .replace_scene(self.scene.clone())?;
        *last_accepted = Some(self.scene.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub struct SceneHostFactory<B: SceneBackend> {
    backend: B,
    manifest: HostManifestConfig,
}
impl<B: SceneBackend> SceneHostFactory<B> {
    pub fn with_backend(backend: B) -> MResult<Self> {
        Ok(Self {
            backend,
            manifest: scene_host_manifest()?,
        })
    }
}
impl<B: SceneBackend> RuntimeHostFactory for SceneHostFactory<B> {
    fn provider_name(&self) -> &str {
        "scene"
    }
    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }
    fn validate_settings(&self, _instance_name: &str, settings: &ConfigValue) -> MResult<()> {
        scene_settings_from_config(settings).map(|_| ())
    }
    fn instantiate(
        &self,
        instance_name: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        let settings: SceneHostSettings = scene_settings_from_config(settings)?;
        let pointer = ScenePointerHandle::new(instance_name);
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(SceneResourceProvider::new_with_settings(
                instance_name,
                self.backend.clone(),
                settings,
            ))],
            input_drivers: vec![pointer.input_driver(instance_name)],
        })
    }
}

pub fn scene_snapshot_from_points(
    value: &LegacyValue,
    settings: &SceneHostSettings,
) -> MResult<SceneSnapshot> {
    let LegacyValue::MatrixF64(matrix) = value else {
        return Err(scene_error(
            "ScenePoints",
            "scene points must be a dense f64 matrix",
        ));
    };
    let rows = matrix.rows();
    let columns = matrix.cols();
    if rows == 0 {
        return Err(scene_error("ScenePoints", "scene points must not be empty"));
    }
    if columns != 2 {
        return Err(scene_error(
            "ScenePoints",
            format!("scene points must have exactly 2 columns; got {columns}"),
        ));
    }
    let values = matrix.as_vec();
    let mut circles = Vec::with_capacity(rows);
    for row in 0..rows {
        let x = values[row];
        let y = values[rows + row];
        if !x.is_finite() || !y.is_finite() {
            return Err(scene_error(
                "ScenePoints",
                format!("scene point row {row} contains a nonfinite coordinate"),
            ));
        }
        circles.push(CircleElement {
            id: format!("body-{row}"),
            x,
            y,
            radius: if row == 0 {
                settings.first_point_radius as f64
            } else {
                settings.point_radius as f64
            },
            fill: BODY_PALETTE[row % BODY_PALETTE.len()].to_owned(),
            stroke: "none".to_owned(),
            stroke_width: 0.0,
            opacity: 1.0,
        });
    }
    Ok(SceneSnapshot {
        width: settings.width as f64,
        height: settings.height as f64,
        background: settings.background.clone(),
        circles,
        lines: Vec::new(),
        line_strips: Vec::new(),
        texts: Vec::new(),
    })
}
