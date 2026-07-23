use std::collections::{BTreeMap, HashMap};

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

use mech_core::{MechError, MechErrorKind};
#[cfg(feature = "served_project_authority")]
use base64::Engine as _;
#[cfg(feature = "served_project_authority")]
use serde::Deserialize;
#[cfg(feature = "served_project_authority")]
use mech_host_browser::BrowserRuntimeInjectionConfig;
#[cfg(feature = "served_project_authority")]
use mech_host_browser::{verify_browser_host_delegation, BrowserHostDelegationEnvelope};
#[cfg(feature = "served_project_authority")]
use mech_runtime::{HostDelegationKeyStore, HostDelegationPublicKey, HostDelegationVerificationRequest, HOST_DELEGATION_ALGORITHM_ED25519};
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


    #[wasm_bindgen(js_name = supportsServedAuthority)]
    pub fn supports_served_authority() -> bool {
        cfg!(feature = "served_project_authority")
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

    #[cfg(feature = "served_project_authority")]
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
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct InjectedHostDelegationPublicKey {
    issuer: String,
    key_id: String,
    algorithm: String,
    public_key: String,
}

#[cfg(feature = "served_project_authority")]
fn decode_injected_host_delegation_keys(
    keys: Vec<InjectedHostDelegationPublicKey>,
) -> Result<HostDelegationKeyStore, JsValue> {
    let mut decoded_keys = Vec::with_capacity(keys.len());
    for key in keys {
        if key.algorithm != HOST_DELEGATION_ALGORITHM_ED25519 {
            return Err(js_error(format!("unsupported trusted host key algorithm `{}`", key.algorithm)));
        }
        let public_key = base64::engine::general_purpose::STANDARD
            .decode(key.public_key.as_bytes())
            .map_err(|error| js_error(format!("invalid trusted host key publicKey: {error}")))?;
        if public_key.len() != 32 {
            return Err(js_error(format!("trusted host key publicKey must decode to 32 bytes, got {}", public_key.len())));
        }
        decoded_keys.push(HostDelegationPublicKey {
            issuer: key.issuer,
            key_id: key.key_id,
            algorithm: HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
            public_key,
        });
    }
    Ok(HostDelegationKeyStore::new(decoded_keys))
}

#[cfg(feature = "served_project_authority")]
fn trusted_host_keys_from_js_value(value: JsValue) -> Result<HostDelegationKeyStore, JsValue> {
    let keys: Vec<InjectedHostDelegationPublicKey> = serde_wasm_bindgen::from_value(value)
        .map_err(|error| js_error(format!("invalid trusted host keys: {error}")))?;
    decode_injected_host_delegation_keys(keys)
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
            let trusted_keys = trusted_host_keys_from_js_value(trusted)?;
            let audience = audience.as_string().ok_or_else(|| js_error("served host delegation audience must be a string"))?;
            let now_ms = js_sys::Date::now().max(0.0) as u64;
            let verified = verify_browser_host_delegation(
                &envelope,
                HostDelegationVerificationRequest {
                    now_ms,
                    expected_audience: audience,
                    trusted_keys,
                    max_clock_skew_ms: 60_000,
                },
            ).map_err(to_js_error)?;
            return Ok(verified.authority.runtime_injection);
        }
    }
    serde_wasm_bindgen::from_value(host_config)
        .map_err(|error| js_error(format!("invalid served host config: {error}")))
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
    #[cfg(not(target_arch = "wasm32"))]
    {
        let _ = message.into();
        return JsValue::NULL;
    }
    #[cfg(target_arch = "wasm32")]
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

    #[derive(Debug)]
    struct TestManualTimerHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        snapshot: mech_host_timer::SharedTimerSnapshot,
    }

    impl TestManualTimerHostFactory {
        fn new() -> Self {
            Self {
                manifest: mech_host_timer::timer_host_manifest().unwrap(),
                snapshot: mech_host_timer::new_shared_snapshot(mech_host_timer::TimerSnapshot::new(0, 60, 0)),
            }
        }
    }

    impl mech_runtime::RuntimeHostFactory for TestManualTimerHostFactory {
        fn provider_name(&self) -> &str { "timer" }
        fn manifest(&self) -> &mech_runtime::HostManifestConfig { &self.manifest }
        fn validate_settings(&self, _instance_name: &str, settings: &mech_runtime::ConfigValue) -> mech_core::MResult<()> {
            mech_host_timer::timer_settings_from_config(settings).map(|_| ())
        }
        fn instantiate(&self, instance_name: &str, settings: &mech_runtime::ConfigValue) -> mech_core::MResult<mech_runtime::RuntimeHostInstallation> {
            let settings = mech_host_timer::timer_settings_from_config(settings)?;
            Ok(mech_runtime::RuntimeHostInstallation {
                interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
                resource_providers: vec![Box::new(mech_host_timer::TimerResourceProvider::new(instance_name, self.snapshot.clone()))],
                input_drivers: vec![Box::new(mech_host_timer::ManualTimerInputDriver::new(
                    instance_name,
                    settings.frequency_hz,
                    settings.max_catch_up_steps,
                ))],
            })
        }
    }

    fn generic_fixture_document() -> MechConfigDocument {
        parse_config_document(
            "generic-timer-table-scene/mech.mcfg",
            include_str!("../tests/fixtures/generic-timer-table-scene/mech.mcfg"),
            ConfigProfileOptions::default(),
        ).unwrap()
    }

    fn generic_fixture_sources() -> HashMap<String, String> {
        let mut sources = HashMap::new();
        sources.insert(
            "table-scene.mec".to_string(),
            include_str!("../tests/fixtures/generic-timer-table-scene/table-scene.mec").to_string(),
        );
        sources
    }


    fn fixture_timer_packet(tick: u64, delta_seconds: f64) -> mech_runtime::RuntimeHostInput {
        mech_runtime::RuntimeHostInput::new(vec![
            mech_runtime::RuntimeHostInputUpdate {
                source: mech_runtime::RuntimeHostInputSource::new("timer://tick/tick", "tick").unwrap(),
                value: mech_runtime::RuntimeHostInputValue::F64(tick as f64),
            },
            mech_runtime::RuntimeHostInputUpdate {
                source: mech_runtime::RuntimeHostInputSource::new("timer://tick/tick", "delta-seconds").unwrap(),
                value: mech_runtime::RuntimeHostInputValue::F64(delta_seconds),
            },
        ]).unwrap()
    }

    #[test]
    fn generic_timer_table_scene_fixture_executes_with_timer_and_scene_hosts() {
        let document = generic_fixture_document();
        let source_paths = required_path_strings(include_str!("../tests/fixtures/generic-timer-table-scene/mech.mcfg")).unwrap();
        assert_eq!(source_paths, vec!["table-scene.mec".to_string()]);

        let scene_backend = mech_host_scene::RecordingSceneBackend::new();
        let mut builder = RuntimeBuilder::new()
            .host_input_capacity(16)
            .host_factory(Box::new(TestManualTimerHostFactory::new())).unwrap()
            .host_factory(Box::new(mech_host_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap())).unwrap();
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        let mut runtime = builder.build().unwrap();
        run_project_sources(&mut runtime, &document, &generic_fixture_sources()).unwrap();

        let initial_scene = scene_backend.latest().unwrap();
        assert_eq!(initial_scene.circles.len(), 2);
        assert_eq!(initial_scene.lines.len(), 3);
        let initial_x = initial_scene.circles[0].x;

        runtime.start_input_drivers().unwrap();
        runtime.ingress().submit(fixture_timer_packet(1, 0.25)).unwrap();
        assert_eq!(runtime.pending_host_input_count().unwrap(), 1);
        let outcomes = runtime.drain_host_inputs(1).unwrap();
        assert_eq!(outcomes.len(), 1);
        assert_eq!(runtime.pending_host_input_count().unwrap(), 0);

        let updated_scene = scene_backend.latest().unwrap();
        assert!(updated_scene.circles[0].x > initial_x);
        assert!((updated_scene.circles[0].x - 20.25).abs() < 0.000001);
        assert_eq!(updated_scene.circles.len(), 2);
        assert_eq!(updated_scene.lines.len(), 3);
        assert!(scene_backend.generation() >= 2);

        for tick in 2..12 {
            runtime.ingress().submit(fixture_timer_packet(tick, 0.25)).unwrap();
        }
        assert_eq!(runtime.pending_host_input_count().unwrap(), 10);
        let drained = runtime.drain_host_inputs(3).unwrap();
        assert_eq!(drained.len(), 3);
        assert_eq!(runtime.pending_host_input_count().unwrap(), 7);
        runtime.stop_input_drivers().unwrap();
        runtime.shutdown().unwrap();
        runtime.shutdown().unwrap();
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
    #[cfg(feature = "served_project_authority")]
    #[test]
    fn injected_ed25519_key_decodes() {
        use base64::Engine as _;
        let public_key = (0u8..32).collect::<Vec<_>>();
        let store = decode_injected_host_delegation_keys(vec![InjectedHostDelegationPublicKey {
            issuer: "issuer".to_string(),
            key_id: "key-1".to_string(),
            algorithm: mech_runtime::HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
            public_key: base64::engine::general_purpose::STANDARD.encode(&public_key),
        }]).unwrap();
        let key = store.key("issuer", "key-1").unwrap();
        assert_eq!(key.issuer, "issuer");
        assert_eq!(key.key_id, "key-1");
        assert_eq!(key.algorithm, "ed25519");
        assert_eq!(key.public_key, public_key);
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn injected_key_rejects_mixed_case_algorithm() {
        use base64::Engine as _;
        let result = decode_injected_host_delegation_keys(vec![InjectedHostDelegationPublicKey {
            issuer: "issuer".to_string(),
            key_id: "key-1".to_string(),
            algorithm: "ED25519".to_string(),
            public_key: base64::engine::general_purpose::STANDARD.encode([0u8; 32]),
        }]);
        assert!(result.is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn injected_key_rejects_invalid_base64() {
        let result = decode_injected_host_delegation_keys(vec![InjectedHostDelegationPublicKey {
            issuer: "issuer".to_string(),
            key_id: "key-1".to_string(),
            algorithm: mech_runtime::HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
            public_key: "not base64!".to_string(),
        }]);
        assert!(result.is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn injected_key_rejects_wrong_length() {
        use base64::Engine as _;
        for bytes in [vec![0u8; 31], vec![0u8; 33]] {
            let result = decode_injected_host_delegation_keys(vec![InjectedHostDelegationPublicKey {
                issuer: "issuer".to_string(),
                key_id: "key-1".to_string(),
                algorithm: mech_runtime::HOST_DELEGATION_ALGORITHM_ED25519.to_string(),
                public_key: base64::engine::general_purpose::STANDARD.encode(bytes),
            }]);
            assert!(result.is_err());
        }
    }

}

#[cfg(all(test, target_arch = "wasm32"))]
mod browser_tests {
    use super::*;
    use js_sys::Object;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);


    #[wasm_bindgen_test]
    fn wasm_project_reports_served_authority_capability() {
        assert_eq!(WasmProject::supports_served_authority(), cfg!(feature = "served_project_authority"));
    }

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
    fn generic_project_with_timer_table_and_scene_renders_fixture() {
        let window = web_sys::window().unwrap();
        let document = window.document().unwrap();
        let canvas = document.create_element("canvas").unwrap();
        canvas.set_attribute("id", "generic-scene").unwrap();
        document.body().unwrap().append_child(&canvas).unwrap();

        let config = include_str!("../tests/fixtures/generic-timer-table-scene/mech.mcfg");
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("table-scene.mec"),
            &JsValue::from_str(include_str!("../tests/fixtures/generic-timer-table-scene/table-scene.mec")),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        project.start().unwrap();
        let result = project.frame(1).unwrap();
        assert_eq!(Reflect::get(&result, &JsValue::from_str("rendered")).unwrap().as_f64(), Some(1.0));
        project.stop().unwrap();
        canvas.remove();
    }


}
