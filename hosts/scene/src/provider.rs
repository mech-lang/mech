use std::sync::{Arc, LazyLock, Mutex};

use mech_core::{
    AccessMode, DeliveryMode, EffectContract, EffectDeliveryPolicy, ExternalInteraction,
    IdempotencyRequirement, InputPortLayout, InputPortPolicy, LegacyValue, MResult,
    OperationContractDeclaration,
};
use mech_runtime::{
    ConfigValue, HostManifestConfig, PreparedRuntimeEffect, RuntimeAfterCommitEffect,
    RuntimeEffectCost, RuntimeEffectMetadata, RuntimeEffectSource, RuntimeHostFactory,
    RuntimeHostInstallation, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    materialize_host_manifest,
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
    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        Err(scene_error(
            "SceneResourceProvider",
            format!("scene resource `{}` is write-only", request.base_uri),
        ))
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
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance_name, &self.manifest)?,
            resource_providers: vec![Box::new(SceneResourceProvider::new_with_settings(
                instance_name,
                self.backend.clone(),
                settings,
            ))],
            input_drivers: Vec::new(),
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
    if columns != 3 {
        return Err(scene_error(
            "ScenePoints",
            format!("scene points must have exactly 3 columns; got {columns}"),
        ));
    }
    let values = matrix.as_vec();
    let mut circles = Vec::with_capacity(rows);
    for row in 0..rows {
        let x = values[row];
        let y = values[rows + row];
        let z = values[2 * rows + row];
        if !x.is_finite() || !y.is_finite() || !z.is_finite() {
            return Err(scene_error(
                "ScenePoints",
                format!("scene point row {row} contains a nonfinite coordinate"),
            ));
        }
        let screen_x = settings.width as f64 / 2.0 + x * settings.pixels_per_unit as f64;
        let screen_y = settings.height as f64 / 2.0 - y * settings.pixels_per_unit as f64;
        if !screen_x.is_finite() || !screen_y.is_finite() {
            return Err(scene_error(
                "ScenePoints",
                format!("scene point row {row} overflows screen coordinates"),
            ));
        }
        circles.push(CircleElement {
            id: format!("body-{row}"),
            x: screen_x,
            y: screen_y,
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
    })
}
