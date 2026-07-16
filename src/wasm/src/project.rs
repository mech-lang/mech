use std::collections::{BTreeMap, HashMap};

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

use mech_core::{MechError, MechErrorKind};
#[cfg(feature = "served_project_authority")]
use mech_host_browser::BrowserRuntimeInjectionConfig;
#[cfg(feature = "served_project_authority")]
use mech_host_browser::{verify_browser_host_delegation, BrowserHostDelegationEnvelope};
#[cfg(feature = "served_project_authority")]
use mech_runtime::{HostDelegationKeyStore, HostDelegationPublicKey, HostDelegationVerificationRequest};
#[cfg(feature = "browser_host_dom")]
use mech_host_browser::BrowserHostFactory;
#[cfg(feature = "browser_host_console")]
use mech_host_console::BrowserConsoleHostFactory;
#[cfg(feature = "browser_host_scene")]
use mech_host_scene::{BrowserSceneHostFactory, BrowserSceneRegistry};
#[cfg(feature = "browser_host_time")]
use mech_host_time::BrowserTimeHostFactory;
#[cfg(feature = "browser_host_timer")]
use mech_host_timer::BrowserTimerHostFactory;
use mech_runtime::{
    ConfigProfileOptions, MechConfigDocument, MechRuntime, RuntimeBuilder, parse_config_document,
};

#[cfg(feature = "browser_host_dom")]
use crate::host::WasmBrowserDomBackend;

#[wasm_bindgen]
pub struct WasmProject {
    runtime: MechRuntime,
    #[cfg(feature = "browser_host_scene")]
    scenes: BrowserSceneRegistry,
    started: bool,
    stopped: bool,
}

#[wasm_bindgen]
impl WasmProject {
    #[wasm_bindgen(js_name = requiredPaths)]
    pub fn required_paths(config_source: &str) -> Result<Array, JsValue> {
        let paths = required_path_strings(config_source).map_err(to_js_error)?;
        let out = Array::new();
        for path in paths {
            out.push(&JsValue::from_str(&path));
        }
        Ok(out)
    }

    #[wasm_bindgen(js_name = fromSources)]
    pub fn from_sources(config_source: &str, sources: JsValue) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        validate_compiled_host_providers(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let mut runtime = build_runtime(&document, #[cfg(feature = "browser_host_scene")] scenes.clone())?;
        run_project_sources(&mut runtime, &document, &source_map).map_err(to_js_error)?;
        Ok(Self {
            runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
            started: false,
            stopped: false,
        })
    }

    #[wasm_bindgen(js_name = fromServedSources)]
    pub fn from_served_sources(config_source: &str, sources: JsValue) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let authority = served_browser_authority()?;
        validate_served_authority(&document, &authority).map_err(to_js_error)?;
        validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let mut runtime = build_runtime_from_authority(&document, &authority, #[cfg(feature = "browser_host_scene")] scenes.clone())?;
        run_project_sources(&mut runtime, &document, &source_map).map_err(to_js_error)?;
        Ok(Self {
            runtime,
            #[cfg(feature = "browser_host_scene")]
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
        #[cfg(feature = "browser_host_scene")]
        let rendered = self.scenes.render_frame().map_err(to_js_error)?;
        #[cfg(not(feature = "browser_host_scene"))]
        let rendered = 0;
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
fn required_path_strings(source: &str) -> mech_core::MResult<Vec<String>> {
    let document = parse_config_document(
        "browser-project/mech.mcfg",
        source,
        ConfigProfileOptions::default(),
    )?;
    let run = require_run(&document)?;
    Ok(run
        .paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect())
}
fn runtime_builder_with_factories(
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
) -> Result<RuntimeBuilder, JsValue> {
    let mut builder = RuntimeBuilder::new();
    #[cfg(feature = "browser_host_dom")]
    {
        builder = builder
            .host_factory(Box::new(
                BrowserHostFactory::new(WasmBrowserDomBackend::new()).map_err(to_js_error)?,
            ))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_time")]
    {
        builder = builder
            .host_factory(Box::new(BrowserTimeHostFactory::new().map_err(to_js_error)?))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_timer")]
    {
        builder = builder
            .host_factory(Box::new(BrowserTimerHostFactory::new().map_err(to_js_error)?))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_console")]
    {
        builder = builder
            .host_factory(Box::new(BrowserConsoleHostFactory::new().map_err(to_js_error)?))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_scene")]
    {
        let scene_factory = BrowserSceneHostFactory::with_registry(scenes).map_err(to_js_error)?;
        builder = builder.host_factory(Box::new(scene_factory)).map_err(to_js_error)?;
    }
    Ok(builder)
}

fn build_runtime(
    document: &MechConfigDocument,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    let mut builder = runtime_builder_with_factories(#[cfg(feature = "browser_host_scene")] scenes)?;
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
#[cfg(feature = "served_project_authority")]
fn build_runtime_from_authority(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    let mut builder = runtime_builder_with_factories(#[cfg(feature = "browser_host_scene")] scenes)?
        .config(authority.into_runtime_config().map_err(to_js_error)?);
    for required in &document.hosts {
        if let Some(host) = authority.hosts.iter().find(|host| host.name == required.name && host.provider == required.provider) {
            builder = builder.host_instance(host.clone());
        }
    }
    for grant in required_issued_grants(document, authority) {
        builder = builder.run_resource_grant(grant);
    }
    builder.build().map_err(to_js_error)
}

#[cfg(not(feature = "served_project_authority"))]
fn build_runtime_from_authority(
    _document: &MechConfigDocument,
    _authority: &(),
    #[cfg(feature = "browser_host_scene")] _scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    Err(js_error("served project authority support was not compiled into this WASM artifact"))
}

fn compiled_browser_providers() -> BTreeMap<&'static str, &'static str> {
    let mut providers = BTreeMap::new();
    #[cfg(feature = "browser_host_dom")]
    providers.insert("browser", "browser_host_dom");
    #[cfg(feature = "browser_host_time")]
    providers.insert("time", "browser_host_time");
    #[cfg(feature = "browser_host_timer")]
    providers.insert("timer", "browser_host_timer");
    #[cfg(feature = "browser_host_console")]
    providers.insert("console", "browser_host_console");
    #[cfg(feature = "browser_host_scene")]
    providers.insert("scene", "browser_host_scene");
    providers
}

fn standard_browser_provider_feature(provider: &str) -> Option<&'static str> {
    match provider {
        "browser" => Some("browser_host_dom"),
        "time" => Some("browser_host_time"),
        "timer" => Some("browser_host_timer"),
        "console" => Some("browser_host_console"),
        "scene" => Some("browser_host_scene"),
        _ => None,
    }
}

fn validate_compiled_host_providers(document: &MechConfigDocument) -> mech_core::MResult<()> {
    validate_compiled_host_providers_for_hosts(&document.hosts)
}

fn validate_compiled_host_providers_for_hosts(hosts: &[mech_runtime::HostInstanceConfig]) -> mech_core::MResult<()> {
    let compiled = compiled_browser_providers();
    for host in hosts {
        if let Some(feature) = standard_browser_provider_feature(&host.provider) {
            if !compiled.contains_key(host.provider.as_str()) {
                return Err(MechError::new(
                    ProjectError { message: format!("project requires host provider `{}`, but this WASM artifact was built without `{}`", host.provider, feature) },
                    None,
                ));
            }
        }
    }
    Ok(())
}
#[cfg(feature = "served_project_authority")]
fn served_browser_authority() -> Result<BrowserRuntimeInjectionConfig, JsValue> {
    let window = web_sys::window().ok_or_else(|| js_error("served project authority requires a browser window"))?;
    let host_config = Reflect::get(&window, &JsValue::from_str("__MECH_HOST_CONFIG"))?;
    if host_config.is_undefined() || host_config.is_null() {
        return Err(js_error("served project authority is missing __MECH_HOST_CONFIG"));
    }
    #[cfg(feature = "served_project_authority")]
    {
        let trusted = Reflect::get(&window, &JsValue::from_str("__MECH_TRUSTED_HOST_KEYS"))?;
        let audience = Reflect::get(&window, &JsValue::from_str("__MECH_HOST_DELEGATION_AUDIENCE"))?;
        if !trusted.is_undefined() && !trusted.is_null() {
            let envelope: BrowserHostDelegationEnvelope = serde_wasm_bindgen::from_value(host_config.clone())
                .map_err(|error| js_error(format!("invalid served host delegation envelope: {error}")))?;
            let keys: Vec<HostDelegationPublicKey> = serde_wasm_bindgen::from_value(trusted)
                .map_err(|error| js_error(format!("invalid trusted host keys: {error}")))?;
            let audience = audience.as_string().ok_or_else(|| js_error("served host delegation audience must be a string"))?;
            let now_ms = js_sys::Date::now().max(0.0) as u64;
            let verified = verify_browser_host_delegation(
                &envelope,
                HostDelegationVerificationRequest {
                    now_ms,
                    expected_audience: audience,
                    trusted_keys: HostDelegationKeyStore::new(keys),
                    max_clock_skew_ms: 60_000,
                },
            ).map_err(to_js_error)?;
            return Ok(verified.authority.runtime_injection);
        }
    }
    serde_wasm_bindgen::from_value(host_config)
        .map_err(|error| js_error(format!("invalid served host config: {error}")))
}

#[cfg(not(feature = "served_project_authority"))]
fn served_browser_authority() -> Result<(), JsValue> {
    Err(js_error("served project authority support was not compiled into this WASM artifact"))
}

fn validate_served_authority(
    document: &MechConfigDocument,
    #[cfg(feature = "served_project_authority")] authority: &BrowserRuntimeInjectionConfig,
    #[cfg(not(feature = "served_project_authority"))] _authority: &(),
) -> mech_core::MResult<()> {
    #[cfg(not(feature = "served_project_authority"))]
    {
        return Err(MechError::new(ProjectError { message: "served project authority support was not compiled into this WASM artifact".into() }, None));
    }
    #[cfg(feature = "served_project_authority")]
    {
        for required in &document.hosts {
            if !authority.hosts.iter().any(|host| host.name == required.name && host.provider == required.provider) {
                return Err(MechError::new(ProjectError { message: format!("served project requires host `{}` provider `{}`, but server authority did not grant it", required.name, required.provider) }, None));
            }
        }
        validate_required_grants(document, authority)?;
        Ok(())
    }
}

#[cfg(feature = "served_project_authority")]
fn required_issued_grants(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
) -> Vec<mech_runtime::RunResourceGrantConfig> {
    let mut out = Vec::new();
    if let Some(run) = &document.run {
        for required in &run.grants {
            let operations = required.operations.clone();
            let paths = required.paths.clone();
            if authority.run_grants.iter().any(|issued| issued.target == required.target) {
                out.push(mech_runtime::RunResourceGrantConfig {
                    target: required.target.clone(),
                    operations,
                    paths,
                });
            }
        }
    }
    out
}

#[cfg(feature = "served_project_authority")]
fn validate_required_grants(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
) -> mech_core::MResult<()> {
    if let Some(run) = &document.run {
        for required in &run.grants {
            let issued = authority
                .run_grants
                .iter()
                .filter(|issued| issued.target == required.target)
                .collect::<Vec<_>>();
            if issued.is_empty() {
                return Err(MechError::new(ProjectError { message: format!("served project requires grant `{}`, but server authority did not issue it", required.target) }, None));
            }
            for operation in &required.operations {
                for path in &required.paths {
                    let authorized = issued.iter().any(|grant| {
                        grant.operations.iter().any(|issued| issued == operation)
                            && grant.paths.iter().any(|issued| grant_path_allows(issued, path))
                    });
                    if !authorized {
                        return Err(MechError::new(ProjectError { message: format!("served project grant `{}` requires operation `{}` on path `{}` outside server authority", required.target, operation, path) }, None));
                    }
                }
            }
        }
    }
    Ok(())
}

#[cfg(feature = "served_project_authority")]
fn grant_path_allows(grant_path: &str, requested_path: &str) -> bool {
    if grant_path == "*" || grant_path == requested_path {
        return true;
    }
    if let Some(prefix) = grant_path.strip_suffix("/*") {
        return requested_path.starts_with(&format!("{}/", prefix));
    }
    false
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

#[cfg(test)]
mod tests {
    use super::*;

    const CONFIG: &str = r#"config := {
  hosts: []
  run: {
    paths: ["a.mec" "b.mec"]
    grants: []
  }
}"#;

    #[test]
    fn required_paths_returns_configured_paths() {
        assert_eq!(
            required_path_strings(CONFIG).unwrap(),
            vec!["a.mec".to_string(), "b.mec".to_string()]
        );
    }

    #[test]
    fn required_paths_rejects_missing_run() {
        assert!(required_path_strings("config := { hosts: [] }").is_err());
    }

    #[test]
    fn required_paths_rejects_empty_paths() {
        let config = r#"config := { hosts: [] run: { paths: [] grants: [] } }"#;
        assert!(required_path_strings(config).is_err());
    }

    #[test]
    fn from_sources_executes_paths_in_order() {
        let mut runtime = RuntimeBuilder::new().build().unwrap();
        let document = parse_config_document(
            "test.mcfg",
            CONFIG,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let mut sources = HashMap::new();
        sources.insert("a.mec".to_string(), "x := 1".to_string());
        sources.insert("b.mec".to_string(), "y := 2".to_string());
        run_project_sources(&mut runtime, &document, &sources).unwrap();
    }


    #[cfg(feature = "served_project_authority")]
    fn authority_config(hosts: Vec<mech_runtime::HostInstanceConfig>, grants: Vec<mech_runtime::RunResourceGrantConfig>) -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_host_browser::BrowserHostRuntimeConfig::from(&mech_runtime::RuntimeConfig::default()),
            hosts,
            run_grants: grants,
        }
    }

    #[cfg(feature = "served_project_authority")]
    fn host(name: &str, provider: &str) -> mech_runtime::HostInstanceConfig {
        mech_runtime::HostInstanceConfig { name: name.to_string(), provider: provider.to_string(), settings: mech_runtime::ConfigValue::Map(Default::default()) }
    }

    #[cfg(feature = "served_project_authority")]
    fn grant(target: &str, operations: &[&str], paths: &[&str]) -> mech_runtime::RunResourceGrantConfig {
        mech_runtime::RunResourceGrantConfig {
            target: target.to_string(),
            operations: operations.iter().map(|op| op.to_string()).collect(),
            paths: paths.iter().map(|path| path.to_string()).collect(),
        }
    }

    #[cfg(feature = "served_project_authority")]
    fn document_with_grant(path: &str, operation: &str) -> MechConfigDocument {
        parse_config_document(
            "served-test.mcfg",
            &format!(r#"config := {{
  hosts: [{{ name: "view" provider: "scene" settings: {{}} }}]
  run: {{
    paths: ["main.mec"]
    grants: [{{ target: "view/frame" operations: ["{operation}"] paths: ["{path}"] }}]
  }}
}}"#),
            ConfigProfileOptions::default(),
        ).unwrap()
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn split_grants_for_one_target_authorize_project_request() {
        let doc = document_with_grant("replace", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![grant("view/frame", &["read"], &["replace"]), grant("view/frame", &["write"], &["replace"])],
        );
        validate_served_authority(&doc, &authority).unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn broader_path_grant_authorizes_narrower_project_request() {
        let doc = document_with_grant("hands/second", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![grant("view/frame", &["write"], &["hands/*"])],
        );
        validate_served_authority(&doc, &authority).unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn extra_operation_is_rejected() {
        let doc = document_with_grant("replace", "write");
        let authority = authority_config(vec![host("view", "scene")], vec![grant("view/frame", &["read"], &["replace"])]);
        assert!(validate_served_authority(&doc, &authority).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn broader_path_request_is_rejected() {
        let doc = document_with_grant("hands/*", "write");
        let authority = authority_config(vec![host("view", "scene")], vec![grant("view/frame", &["write"], &["hands/second"])]);
        assert!(validate_served_authority(&doc, &authority).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn crossed_operation_and_path_grants_are_rejected() {
        let doc = document_with_grant("secret/file", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![grant("view/frame", &["write"], &["public/*"]), grant("view/frame", &["read"], &["secret/*"])],
        );
        assert!(validate_served_authority(&doc, &authority).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn unrelated_issued_host_does_not_require_compiled_provider() {
        let doc = document_with_grant("replace", "write");
        let authority = authority_config(
            vec![host("view", "scene"), host("unused", "browser")],
            vec![grant("view/frame", &["write"], &["replace"])],
        );
        validate_served_authority(&doc, &authority).unwrap();
        validate_compiled_host_providers_for_hosts(&doc.hosts).unwrap();
    }



    #[test]
    fn generic_table_project_source_runs_through_runtime_loader() {
        let document = parse_config_document(
            "generic-table.mcfg",
            r#"config := { hosts: [] run: { paths: ["generic-table.mec"] grants: [] } }"#,
            ConfigProfileOptions::default(),
        ).unwrap();
        let mut sources = HashMap::new();
        sources.insert(
            "generic-table.mec".to_string(),
            r#"delta := 0.25
rows := |id<string> x<f64>|
  | "row-a" 1 + delta |
  | "row-b" 2 + delta |"#.to_string(),
        );
        let mut runtime = RuntimeBuilder::new().build().unwrap();
        run_project_sources(&mut runtime, &document, &sources).unwrap();
    }

    #[test]
    fn generic_timer_table_scene_fixture_declares_reusable_project_pipeline() {
        let paths = required_path_strings(include_str!("../tests/fixtures/generic-timer-table-scene/mech.mcfg")).unwrap();
        assert_eq!(paths, vec!["table-scene.mec".to_string()]);
        let source = include_str!("../tests/fixtures/generic-timer-table-scene/table-scene.mec");
        assert!(source.contains("@tick/delta-seconds"));
        assert!(source.contains("rows := |id<string> x<f64> y<f64> radius<f64> fill<string>|"));
        assert!(source.contains("@view/replace <- scene"));
    }

    #[test]
    fn from_sources_rejects_missing_source() {
        let mut runtime = RuntimeBuilder::new().build().unwrap();
        let document = parse_config_document(
            "test.mcfg",
            CONFIG,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let sources = HashMap::new();
        assert!(run_project_sources(&mut runtime, &document, &sources).is_err());
    }
}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use js_sys::Object;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn generic_project_starts_and_stops_idempotently() {
        let config = r#"config := { hosts: [] run: { paths: ["main.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str("x := 1"),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        project.start().unwrap();
        project.start().unwrap();
        project.stop().unwrap();
        project.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn generic_project_frame_respects_input_bound() {
        let config = r#"config := { hosts: [] run: { paths: ["generic-table.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("generic-table.mec"),
            &JsValue::from_str(r#"delta := 0.25
rows := |id<string> x<f64>|
  | "row-a" 1 + delta |
  | "row-b" 2 + delta |"#),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        assert!(project.frame(1).is_ok());
    }

    #[wasm_bindgen_test]
    fn generic_project_frame_reports_pending_inputs() {
        let config = r#"config := { hosts: [] run: { paths: ["main.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str("x := 1"),
        )
        .unwrap();
        let project = WasmProject::from_sources(config, sources.into()).unwrap();
        assert_eq!(project.pending_inputs().unwrap(), 0);
    }

    #[wasm_bindgen_test]
    fn generic_project_frame_renders_latest_scene() {
        let config = r#"config := { hosts: [] run: { paths: ["main.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str("x := 1"),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        let result = project.frame(1).unwrap();
        assert_eq!(Reflect::get(&result, &JsValue::from_str("rendered")).unwrap().as_f64(), Some(0.0));
    }

    #[wasm_bindgen_test]
    fn generic_project_with_time_console_and_scene_runs_clock_source() {
        assert!(required_path_strings(include_str!("../../../examples/analog-clock/mech.mcfg")).is_ok());
    }

    #[wasm_bindgen_test]
    fn generic_project_with_timer_and_scene_runs_fixed_step_fixture() {
        let config = r#"config := {
  hosts: []
  run: { paths: ["generic-table-scene.mec"] grants: [] }
}"#;
        assert_eq!(required_path_strings(config).unwrap(), vec!["generic-table-scene.mec".to_string()]);
    }
}
