use std::collections::HashMap;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

use mech_core::{MechError, MechErrorKind};
use mech_host_browser::BrowserHostFactory;
use mech_host_console::BrowserConsoleHostFactory;
use mech_host_scene::{BrowserSceneHostFactory, BrowserSceneRegistry};
use mech_host_time::BrowserTimeHostFactory;
use mech_host_timer::BrowserTimerHostFactory;
use mech_runtime::{
    ConfigProfileOptions, MechConfigDocument, MechRuntime, RuntimeBuilder, parse_config_document,
};

use crate::host::WasmBrowserDomBackend;

#[wasm_bindgen]
pub struct WasmProject {
    runtime: MechRuntime,
    scenes: BrowserSceneRegistry,
    started: bool,
    stopped: bool,
}

#[wasm_bindgen]
impl WasmProject {
    #[wasm_bindgen(js_name = requiredPaths)]
    pub fn required_paths(config_source: &str) -> Result<Array, JsValue> {
        let document = parse_project_config(config_source)?;
        let run = require_run(&document).map_err(to_js_error)?;
        let out = Array::new();
        for path in &run.paths {
            out.push(&JsValue::from_str(&path.to_string_lossy()));
        }
        Ok(out)
    }

    #[wasm_bindgen(js_name = fromSources)]
    pub fn from_sources(config_source: &str, sources: JsValue) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let scenes = BrowserSceneRegistry::new();
        let mut runtime = build_runtime(&document, scenes.clone())?;
        run_project_sources(&mut runtime, &document, &source_map).map_err(to_js_error)?;
        Ok(Self {
            runtime,
            scenes,
            started: false,
            stopped: false,
        })
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        if self.started {
            return Ok(());
        }
        self.runtime.start_input_drivers().map_err(to_js_error)?;
        self.started = true;
        self.stopped = false;
        Ok(())
    }

    pub fn frame(&mut self, max_inputs: usize) -> Result<JsValue, JsValue> {
        if max_inputs == 0 {
            return Err(js_error("max_inputs must be greater than zero"));
        }
        let pending_before = self
            .runtime
            .pending_host_input_count()
            .map_err(to_js_error)?;
        let to_drain = pending_before.min(max_inputs);
        let processed = if to_drain == 0 {
            0
        } else {
            self.runtime
                .drain_host_inputs(to_drain)
                .map_err(to_js_error)?
                .len()
        };
        let pending = self
            .runtime
            .pending_host_input_count()
            .map_err(to_js_error)?;
        let rendered = self.scenes.render_frame().map_err(to_js_error)?;
        let out = Object::new();
        Reflect::set(
            &out,
            &JsValue::from_str("processed"),
            &JsValue::from_f64(processed as f64),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("pending"),
            &JsValue::from_f64(pending as f64),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("rendered"),
            &JsValue::from_f64(rendered as f64),
        )?;
        Ok(out.into())
    }

    #[wasm_bindgen(js_name = pendingInputs)]
    pub fn pending_inputs(&self) -> Result<usize, JsValue> {
        self.runtime.pending_host_input_count().map_err(to_js_error)
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        if self.stopped {
            return Ok(());
        }
        self.runtime.stop_input_drivers().map_err(to_js_error)?;
        self.runtime.shutdown().map_err(to_js_error)?;
        self.started = false;
        self.stopped = true;
        Ok(())
    }
}

fn parse_project_config(source: &str) -> Result<MechConfigDocument, JsValue> {
    parse_config_document(
        "browser-project/mech.mcfg",
        source,
        ConfigProfileOptions::default(),
    )
    .map_err(to_js_error)
}
fn build_runtime(
    document: &MechConfigDocument,
    scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    let scene_factory = BrowserSceneHostFactory::with_registry(scenes).map_err(to_js_error)?;
    let mut builder = RuntimeBuilder::new()
        .host_factory(Box::new(
            BrowserHostFactory::new(WasmBrowserDomBackend::new()).map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(
            BrowserTimeHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(
            BrowserTimerHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(
            BrowserConsoleHostFactory::new().map_err(to_js_error)?,
        ))
        .map_err(to_js_error)?
        .host_factory(Box::new(scene_factory))
        .map_err(to_js_error)?;
    for host in &document.hosts {
        builder = builder.host_instance(host.clone());
    }
    if let Some(run) = &document.run {
        for grant in &run.grants {
            builder = builder.run_resource_grant(grant.clone());
        }
    }
    builder.build().map_err(to_js_error)
}
fn source_map_from_js(value: JsValue) -> Result<HashMap<String, String>, JsValue> {
    if !value.is_object() || value.is_null() {
        return Err(js_error("sources must be an object"));
    }
    let object = Object::from(value);
    let keys = Object::keys(&object);
    let mut out = HashMap::new();
    for key in keys.iter() {
        let Some(path) = key.as_string() else {
            return Err(js_error("source map keys must be strings"));
        };
        let text = Reflect::get(&object, &key)?
            .as_string()
            .ok_or_else(|| js_error(format!("source `{path}` must be a string")))?;
        out.insert(path, text);
    }
    Ok(out)
}
fn run_project_sources(
    runtime: &mut MechRuntime,
    document: &MechConfigDocument,
    sources: &HashMap<String, String>,
) -> mech_core::MResult<()> {
    let run = require_run(document)?;
    for path in &run.paths {
        let key = path.to_string_lossy().to_string();
        let source = sources.get(&key).ok_or_else(|| {
            MechError::new(
                ProjectError {
                    message: format!("missing source `{key}`"),
                },
                None,
            )
        })?;
        runtime.run_string(source)?;
    }
    Ok(())
}
fn require_run(document: &MechConfigDocument) -> mech_core::MResult<&mech_runtime::RunHostConfig> {
    let run = document.run.as_ref().ok_or_else(|| {
        MechError::new(
            ProjectError {
                message: "project config must contain run settings".into(),
            },
            None,
        )
    })?;
    if run.paths.is_empty() {
        return Err(MechError::new(
            ProjectError {
                message: "project config must contain at least one run path".into(),
            },
            None,
        ));
    }
    Ok(run)
}

#[derive(Debug, Clone)]
struct ProjectError {
    message: String,
}
impl MechErrorKind for ProjectError {
    fn name(&self) -> &str {
        "BrowserProjectError"
    }
    fn message(&self) -> String {
        self.message.clone()
    }
}
fn js_error(message: impl Into<String>) -> JsValue {
    JsValue::from_str(&message.into())
}
fn to_js_error(error: MechError) -> JsValue {
    js_error(format!("{error:?}"))
}
