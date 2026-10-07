use std::cell::Cell;
#[cfg(any(
    feature = "browser_project_core",
    feature = "browser_compute",
    feature = "browser_host_scene"
))]
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
#[cfg(feature = "served_project_authority")]
use std::path::Path;
use std::rc::Rc;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

#[cfg(feature = "served_project_authority")]
use base64::Engine as _;
#[cfg(feature = "browser_host_dom")]
use mech_browser::BrowserHostFactory;
#[cfg(feature = "served_project_authority")]
use mech_browser::BrowserRuntimeInjectionConfig;
#[cfg(feature = "served_project_authority")]
use mech_browser::{BrowserHostDelegationEnvelope, verify_browser_host_delegation};
#[cfg(feature = "browser_compute")]
use mech_browser::{PointerHostFactory, PointerInputHandle};
#[cfg(feature = "browser_host_console")]
use mech_console::{BrowserConsoleHostFactory, ConsoleHostFactory};
use mech_core::{GenericError, MResult, MechError, MechErrorKind, MechSourceCode, OutputId};
use mech_engine::{
    CanonicalSourceFrontend, SourceDocumentOutputKind, root_document_program_output_id,
};
#[cfg(feature = "browser_host_scene")]
use mech_runtime::MechEvent;
use mech_runtime::{
    BrowserDocumentPayload, CanonicalDependencySource, CanonicalProgramBundle,
    ConfigProfileOptions, ConfigValue, HostInstanceConfig, InMemorySourceResolver,
    MechConfigDocument, MechEventBuffer, MechEventBus, MechRuntime, ModuleBuildOptions,
    ResidentRouteFailure, ResidentRouteFailureClass, ResolvedSource, RunResourceGrantConfig,
    RuntimeBuilder, RuntimeProgramExecutionInfo, RuntimeProgramLoadOutcome, RuntimeProgramRoute,
    SourceDocument, SourceKind, SourceRequest, SourceResolutionEntry,
    import_may_resolve_source_dependency, parse_config_document, source_request_for_import,
    validate_source_resolution_entries,
};
#[cfg(feature = "served_project_authority")]
use mech_runtime::{
    HOST_DELEGATION_ALGORITHM_ED25519, HostDelegationKeyStore, HostDelegationPublicKey,
    HostDelegationVerificationRequest,
};
#[cfg(feature = "browser_host_scene")]
use mech_scene::{BrowserSceneHostFactory, BrowserSceneRegistry};
#[cfg(feature = "browser_host_time")]
use mech_time::BrowserTimeHostFactory;
#[cfg(feature = "browser_host_timer")]
use mech_timer::BrowserTimerHostFactory;
use serde::Deserialize;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ServedSourceProvenance {
    pub(crate) nominal_origin: mech_core::CanonicalNominalPath,
    pub(crate) nominal_package_id: Option<String>,
}

fn served_provenance_from_js(
    value: JsValue,
    sources: &HashMap<String, String>,
) -> Result<HashMap<String, ServedSourceProvenance>, JsValue> {
    let provenance = if value.is_undefined() || value.is_null() {
        HashMap::new()
    } else {
        serde_wasm_bindgen::from_value(value)
            .map_err(|error| js_error(format!("invalid served nominal provenance: {error}")))?
    };
    if provenance
        .keys()
        .any(|specifier| !sources.contains_key(specifier))
    {
        return Err(js_error("served nominal provenance has an unknown source"));
    }
    Ok(provenance)
}

use crate::canonical_document::CanonicalWasmDocument;
#[cfg(feature = "browser_host_dom")]
use crate::host::WasmBrowserDomBackend;
#[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
use crate::mixed_compute::prepare_browser_compute_admission_factory;
#[cfg(feature = "browser_compute")]
use crate::mixed_compute::{
    BrowserComputeBridge, BrowserComputePurpose, PreparedComputeRegion,
    prepare_browser_compute_runtime, prepare_compute_document_region, prepare_compute_root_region,
};

#[wasm_bindgen]
pub struct WasmProject {
    runtime: MechRuntime,
    events: MechEventBus,
    #[cfg(feature = "browser_compute")]
    pointer: Option<PointerInputHandle>,
    #[cfg(feature = "browser_compute")]
    compute: Option<BrowserComputeBridge>,
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

    #[wasm_bindgen(js_name = supportsServedDocumentResolutions)]
    pub fn supports_served_document_resolutions() -> bool {
        cfg!(feature = "served_project_authority")
    }

    #[wasm_bindgen(js_name = supportsServedDocumentProvenance)]
    pub fn supports_served_document_provenance() -> bool {
        cfg!(feature = "served_project_authority")
    }

    #[wasm_bindgen(js_name = supportsCompute)]
    pub fn supports_compute() -> bool {
        cfg!(feature = "browser_compute")
    }

    /// Compile with this package's actual language and host closure. This is
    /// planning only: no program activation or input-driver startup occurs.
    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = validateStaticSources)]
    pub fn validate_static_sources(
        config_source: &str,
        sources: JsValue,
        roots: JsValue,
        resolutions: JsValue,
        provenance: JsValue,
    ) -> Result<bool, JsValue> {
        let mut document = parse_project_config(config_source)?;
        let sources = source_map_from_js(sources)?;
        let roots = bundle_roots_from_js(roots)?;
        let resolutions = document_resolutions_from_js(resolutions, &sources)?;
        let provenance = served_provenance_from_js(provenance, &sources)?;
        replace_bundle_run_paths(&mut document, roots)?;
        validate_static_project_sources(&document, &sources, &resolutions, &provenance)
            .map_err(to_js_error)?;
        Ok(true)
    }

    #[wasm_bindgen(js_name = fromSources)]
    pub fn from_sources(config_source: &str, sources: JsValue) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        Self::from_project_sources(document, source_map, Vec::new(), HashMap::new())
    }

    #[wasm_bindgen(js_name = fromSourcesWithResolutions)]
    pub fn from_sources_with_resolutions(
        config_source: &str,
        sources: JsValue,
        resolutions: JsValue,
        provenance: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
        let provenance = served_provenance_from_js(provenance, &source_map)?;
        Self::from_project_sources(document, source_map, resolutions, provenance)
    }

    fn from_project_sources(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
        provenance: HashMap<String, ServedSourceProvenance>,
    ) -> Result<WasmProject, JsValue> {
        validate_compiled_host_providers(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_compute")]
        let pointer = configured_project_pointer(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = project_source_resolver_with_resolutions_and_provenance(
            &source_map,
            &resolutions,
            &provenance,
        )
        .map_err(to_js_error)?;
        let candidate = build_project_source_runtime(
            &document,
            source_resolver,
            #[cfg(feature = "served_project_authority")]
            None,
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
            #[cfg(feature = "browser_compute")]
            pointer.clone(),
        )?;
        let project = Self::from_runtime(
            candidate.runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
            #[cfg(feature = "browser_compute")]
            pointer,
        );
        #[cfg(feature = "browser_compute")]
        let project = project.with_compute_bridge(candidate.compute);
        Ok(project)
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedSources)]
    pub fn from_served_sources(
        config_source: &str,
        sources: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        Self::from_served_project(document, source_map, Vec::new(), HashMap::new())
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedSourcesWithResolutions)]
    pub fn from_served_sources_with_resolutions(
        config_source: &str,
        sources: JsValue,
        resolutions: JsValue,
        provenance: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
        let provenance = served_provenance_from_js(provenance, &source_map)?;
        Self::from_served_project(document, source_map, resolutions, provenance)
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedBundle)]
    pub fn from_served_bundle(
        config_source: &str,
        sources: JsValue,
        artifacts: JsValue,
        roots: JsValue,
        provenance: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let mut document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let artifact_map = source_map_from_js(artifacts)?;
        let roots = bundle_roots_from_js(roots)?;
        let provenance: HashMap<String, ServedSourceProvenance> =
            serde_wasm_bindgen::from_value(provenance)
                .map_err(|error| js_error(format!("invalid bundle nominal provenance: {error}")))?;
        if provenance
            .keys()
            .any(|specifier| !source_map.contains_key(specifier))
        {
            return Err(js_error("bundle nominal provenance has an unknown source"));
        }
        replace_bundle_run_paths(&mut document, roots.clone())?;
        Self::from_served_project_bundle(document, source_map, artifact_map, roots, provenance)
    }

    #[cfg(feature = "served_project_authority")]
    fn from_served_project_bundle(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        artifact_map: HashMap<String, String>,
        roots: Vec<String>,
        provenance: HashMap<String, ServedSourceProvenance>,
    ) -> Result<WasmProject, JsValue> {
        let authority = served_browser_authority()?;
        validate_served_authority(&document, &authority).map_err(to_js_error)?;
        validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
        #[cfg(feature = "browser_compute")]
        let pointer = configured_project_pointer(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = project_source_resolver_with_provenance(&source_map, &provenance)
            .map_err(to_js_error)?;
        let mut runtime = build_runtime_from_authority(
            &document,
            &authority,
            source_resolver,
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
            #[cfg(feature = "browser_compute")]
            pointer.clone(),
        )?;
        let [root] = roots.as_slice() else {
            return Err(to_js_error(MechError::new(
                GenericError {
                    msg: "canonical browser bundles require exactly one root artifact".to_owned(),
                },
                None,
            )));
        };
        let source = source_map.get(root).ok_or_else(|| {
            JsValue::from_str(&format!("canonical bundle root source is missing: {root}"))
        })?;
        let encoded = artifact_map.get(root).ok_or_else(|| {
            JsValue::from_str(&format!(
                "canonical bundle root artifact is missing: {root}"
            ))
        })?;
        let root_provenance = provenance.get(root);
        let bundle = CanonicalProgramBundle::decode_with_root_provenance(
            encoded,
            Some(source),
            root_provenance.map(|item| &item.nominal_origin),
            root_provenance.and_then(|item| item.nominal_package_id.as_deref()),
        )
        .map_err(to_js_error)?;
        bundle
            .validate_dependency_sources_with_provenance(|uri| {
                let specifier = uri.strip_prefix("bundle:///")?;
                let source = source_map.get(specifier)?.as_str();
                let retained = provenance.get(specifier);
                Some(CanonicalDependencySource {
                    source,
                    nominal_origin: retained.map(|item| &item.nominal_origin),
                    nominal_package_id: retained
                        .and_then(|item| item.nominal_package_id.as_deref()),
                })
            })
            .map_err(to_js_error)?;
        if bundle.canonical_uri != format!("bundle:///{root}") {
            return Err(JsValue::from_str(
                "canonical bundle root identity is stale; regenerate the bundle",
            ));
        }
        let durability = runtime.config().resident_durability;
        runtime
            .load_bytecode_program(&bundle.bytecode, durability)
            .map_err(to_js_error)?;
        Ok(Self::from_runtime(
            runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
            #[cfg(feature = "browser_compute")]
            pointer,
        ))
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedDocuments)]
    pub fn from_served_documents(
        config_source: &str,
        sources: JsValue,
        documents: JsValue,
        roots: JsValue,
        resolutions: JsValue,
        provenance: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let mut document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let document_map = source_map_from_js(documents)?;
        let roots = bundle_roots_from_js(roots)?;
        let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
        let provenance = served_provenance_from_js(provenance, &source_map)?;
        replace_bundle_run_paths(&mut document, roots.clone())?;
        Self::from_served_project_documents(
            document,
            source_map,
            document_map,
            roots,
            resolutions,
            provenance,
        )
    }

    #[cfg(feature = "served_project_authority")]
    fn from_served_project_documents(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        document_map: HashMap<String, String>,
        roots: Vec<String>,
        resolutions: Vec<SourceResolutionEntry>,
        provenance: HashMap<String, ServedSourceProvenance>,
    ) -> Result<WasmProject, JsValue> {
        let [root] = roots.as_slice() else {
            return Err(to_js_error(MechError::new(
                GenericError {
                    msg: "canonical browser projects require exactly one root document".to_owned(),
                },
                None,
            )));
        };
        let encoded = document_map.get(root).ok_or_else(|| {
            JsValue::from_str(&format!(
                "canonical browser root document is missing: {root}"
            ))
        })?;
        let payload = decode_document_payload(encoded)?;
        validate_document_payload(&payload, root, &source_map)?;
        validate_static_nominal_provenance(&source_map, &provenance).map_err(to_js_error)?;
        Self::from_served_project(document, source_map, resolutions, provenance)
    }

    #[cfg(feature = "served_project_authority")]
    fn from_served_project(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
        provenance: HashMap<String, ServedSourceProvenance>,
    ) -> Result<WasmProject, JsValue> {
        let authority = served_browser_authority()?;
        validate_served_authority(&document, &authority).map_err(to_js_error)?;
        validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
        #[cfg(feature = "browser_compute")]
        let pointer = configured_project_pointer(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = project_source_resolver_with_resolutions_and_provenance(
            &source_map,
            &resolutions,
            &provenance,
        )
        .map_err(to_js_error)?;
        let candidate = build_project_source_runtime(
            &document,
            source_resolver,
            Some(&authority),
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
            #[cfg(feature = "browser_compute")]
            pointer.clone(),
        )?;
        let project = Self::from_runtime(
            candidate.runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
            #[cfg(feature = "browser_compute")]
            pointer,
        );
        #[cfg(feature = "browser_compute")]
        let project = project.with_compute_bridge(candidate.compute);
        Ok(project)
    }

    #[cfg(feature = "browser_compute")]
    fn with_compute_bridge(mut self, compute: Option<BrowserComputeBridge>) -> Self {
        self.compute = compute;
        self
    }

    fn from_runtime(
        runtime: MechRuntime,
        #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
        #[cfg(feature = "browser_compute")] pointer: Option<PointerInputHandle>,
    ) -> Self {
        Self {
            runtime,
            events: MechEventBus::default(),
            #[cfg(feature = "browser_compute")]
            pointer,
            #[cfg(feature = "browser_compute")]
            compute: None,
            #[cfg(feature = "browser_host_scene")]
            scenes,
            started: false,
            stopped: false,
        }
    }

    #[wasm_bindgen(js_name = renderedOutput)]
    pub fn rendered_output(&self, output_id: u64) -> Result<JsValue, JsValue> {
        let Ok(output_id) = u32::try_from(output_id) else {
            return Ok(JsValue::NULL);
        };
        self.runtime
            .output_value(OutputId::new(output_id))
            .map_err(to_js_error)?
            .map(|value| rendered_value(value, mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT))
            .transpose()
            .map(|value| value.unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = renderedSymbol)]
    pub fn rendered_symbol(&self, name: &str) -> Result<JsValue, JsValue> {
        let names = vec![name.to_string()];
        let value = self
            .runtime
            .program_output_values(&names)
            .map_err(to_js_error)?
            .pop()
            .map(|(_, value)| value);
        value
            .map(|value| rendered_value(value, mech_runtime::DEFAULT_REPL_VALUE_ELEMENT_LIMIT))
            .transpose()
            .map(|value| value.unwrap_or(JsValue::NULL))
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        self.refresh_relevant_input_drivers()?;
        self.started = true;
        self.stopped = false;
        Ok(())
    }

    #[wasm_bindgen(js_name = hasPointerInput)]
    pub fn has_pointer_input(&self) -> bool {
        #[cfg(feature = "browser_compute")]
        {
            self.pointer
                .as_ref()
                .is_some_and(PointerInputHandle::is_running)
        }
        #[cfg(not(feature = "browser_compute"))]
        {
            false
        }
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = pointerInput)]
    pub fn pointer_input(
        &self,
        x: f64,
        y: f64,
        pressed: bool,
        delta_seconds: f64,
    ) -> Result<(), JsValue> {
        self.pointer
            .as_ref()
            .ok_or_else(|| js_error("project has no pointer host"))?
            .submit(x, y, pressed, delta_seconds)
            .map_err(to_js_error)
    }

    /// Reconciles drivers against the retained runtime's current live input
    /// bindings. The runtime operation is idempotent: active drivers remain
    /// active and newly relevant drivers are started exactly once.
    fn refresh_relevant_input_drivers(&mut self) -> Result<(), JsValue> {
        self.runtime.start_input_drivers().map_err(to_js_error)?;
        Ok(())
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = computeManifest)]
    pub fn compute_manifest(&self) -> JsValue {
        self.compute
            .as_ref()
            .map(BrowserComputeBridge::manifest)
            .unwrap_or(JsValue::NULL)
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = computeBackend)]
    pub fn compute_backend(&self) -> String {
        self.compute
            .as_ref()
            .map(BrowserComputeBridge::backend)
            .unwrap_or_default()
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = computeGeneration)]
    pub fn compute_generation(&self) -> String {
        self.compute
            .as_ref()
            .map(|compute| compute.generation().to_string())
            .unwrap_or_default()
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = isComputeCommandTokenCurrent)]
    pub fn is_compute_command_token_current(&self, token: &str) -> bool {
        self.compute
            .as_ref()
            .is_some_and(|compute| compute.validate_token(token).is_ok())
    }

    #[cfg(feature = "browser_compute")]
    #[wasm_bindgen(js_name = completeComputeCommand)]
    pub fn complete_compute_command(&self, completion: JsValue) -> Result<(), JsValue> {
        self.compute
            .as_ref()
            .ok_or_else(|| js_error("project has no compute region"))?
            .complete_command(&completion)
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
        #[cfg(feature = "browser_host_scene")]
        self.events.publish_all(
            self.scenes
                .drain_output_events()
                .map_err(to_js_error)?
                .into_iter()
                .map(MechEvent::Output),
        );
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
        Reflect::set(
            &out,
            &JsValue::from_str("events"),
            &serde_wasm_bindgen::to_value(&self.events.drain())?,
        )?;
        let info = self.runtime.program_execution_info();
        Reflect::set(
            &out,
            &JsValue::from_str("route"),
            &JsValue::from_str(runtime_route_name(info.route)),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("residentTurns"),
            &JsValue::from_f64(
                info.resident_accepted_turns
                    .saturating_add(info.resident_rejected_turns) as f64,
            ),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("accepted"),
            &JsValue::from_f64(info.resident_accepted_turns as f64),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("rejected"),
            &JsValue::from_f64(info.resident_rejected_turns as f64),
        )?;
        Reflect::set(
            &out,
            &JsValue::from_str("coalesced"),
            &JsValue::from_f64(info.coalesced_host_packets as f64),
        )?;
        #[cfg(feature = "browser_compute")]
        Reflect::set(
            &out,
            &JsValue::from_str("computeCommand"),
            &match self.compute.as_ref() {
                Some(compute) => compute.take_command()?,
                None => JsValue::NULL,
            },
        )?;
        Ok(out.into())
    }

    #[wasm_bindgen(js_name = runtimeInfo)]
    pub fn runtime_info(&self) -> Result<JsValue, JsValue> {
        runtime_info_value(&self.runtime.program_execution_info())
    }

    #[wasm_bindgen(js_name = pendingInputs)]
    pub fn pending_inputs(&self) -> Result<usize, JsValue> {
        self.runtime.pending_host_input_count().map_err(to_js_error)
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        if self.stopped {
            return Ok(());
        }
        self.runtime.shutdown().map_err(to_js_error)?;
        self.started = false;
        self.stopped = true;
        Ok(())
    }
}

/// Browser runtime adapter for one formatted Mech source document.
///
/// The document owns its bootstrap through its HTML shim. This adapter only
/// decodes and executes the shim's detached `{{CODE}}` payload, retains the
/// runtime, and exposes detached render queries.
#[derive(Clone)]
pub(crate) struct WasmDocumentBootstrap {
    root_specifier: String,
    source_map: HashMap<String, String>,
    resolutions: Vec<SourceResolutionEntry>,
    provenance: HashMap<String, ServedSourceProvenance>,
    document: CanonicalWasmDocument,
    document_base: Rc<RefCell<Staged<SourceDocument>>>,
    presentation_state: Rc<RefCell<Staged<document::DocumentOutputState>>>,
    presentation_output_ids: Vec<u64>,
    initial_bundle: Option<CanonicalProgramBundle>,
    console_instance: String,
    lifecycle: DocumentRuntimeLifecycle,
    #[cfg(feature = "served_project_authority")]
    served: Option<ServedDocumentBootstrap>,
}

#[derive(Clone, Default)]
struct Staged<T> {
    active: T,
    pending: Option<T>,
}

impl<T> Staged<T> {
    fn stage(&mut self, value: T) {
        self.pending = Some(value);
    }

    fn commit(&mut self) {
        if let Some(value) = self.pending.take() {
            self.active = value;
        }
    }

    fn abort(&mut self) {
        self.pending = None;
    }
}

#[derive(Clone, Default)]
struct DocumentRuntimeLifecycle {
    drivers_started: Rc<Cell<bool>>,
    #[cfg(feature = "browser_compute")]
    pointer: Rc<RefCell<Staged<Option<PointerInputHandle>>>>,
    #[cfg(feature = "browser_compute")]
    compute_generation: Rc<Cell<u64>>,
    #[cfg(feature = "browser_host_scene")]
    scenes: Rc<RefCell<Staged<BrowserSceneRegistry>>>,
    #[cfg(feature = "browser_compute")]
    compute: Rc<RefCell<Staged<Option<BrowserComputeBridge>>>>,
}

impl DocumentRuntimeLifecycle {
    fn drivers_started(&self) -> bool {
        self.drivers_started.get()
    }

    fn set_drivers_started(&self, started: bool) {
        self.drivers_started.set(started);
    }

    #[cfg(feature = "browser_compute")]
    fn pointer(&self) -> Option<PointerInputHandle> {
        self.pointer.borrow().active.clone()
    }

    #[cfg(feature = "browser_compute")]
    fn stage_pointer(&self, pointer: Option<PointerInputHandle>) {
        self.pointer.borrow_mut().stage(pointer);
    }

    #[cfg(feature = "browser_compute")]
    fn commit_pointer(&self) {
        self.pointer.borrow_mut().commit();
    }

    #[cfg(feature = "browser_compute")]
    fn abort_pointer(&self) {
        self.pointer.borrow_mut().abort();
    }

    #[cfg(feature = "browser_host_scene")]
    fn scenes(&self) -> BrowserSceneRegistry {
        self.scenes.borrow().active.clone()
    }

    #[cfg(feature = "browser_host_scene")]
    fn stage_scenes(&self, scenes: BrowserSceneRegistry) {
        self.scenes.borrow_mut().stage(scenes);
    }

    #[cfg(feature = "browser_host_scene")]
    fn commit_scenes(&self) {
        self.scenes.borrow_mut().commit();
    }

    #[cfg(feature = "browser_host_scene")]
    fn abort_scenes(&self) {
        self.scenes.borrow_mut().abort();
    }

    #[cfg(feature = "browser_compute")]
    fn compute(&self) -> Option<BrowserComputeBridge> {
        self.compute.borrow().active.clone()
    }

    #[cfg(feature = "browser_compute")]
    fn compute_generation(&self) -> u64 {
        self.compute_generation.get()
    }

    #[cfg(feature = "browser_compute")]
    fn next_compute_generation(&self) -> MResult<u64> {
        self.compute_generation
            .get()
            .checked_add(1)
            .ok_or_else(|| document_runtime_error("browser compute generation space exhausted"))
    }

    #[cfg(feature = "browser_compute")]
    fn stage_compute(&self, compute: Option<BrowserComputeBridge>) {
        self.compute.borrow_mut().stage(compute);
    }

    #[cfg(feature = "browser_compute")]
    fn commit_compute(&self) {
        if self.compute.borrow().pending.is_none() {
            return;
        }
        let next = self
            .compute_generation
            .get()
            .checked_add(1)
            .expect("compute generation exhaustion is rejected while preparing the runtime");
        debug_assert!(
            self.compute
                .borrow()
                .pending
                .as_ref()
                .is_some_and(|compute| {
                    compute
                        .as_ref()
                        .is_none_or(|bridge| bridge.generation() == next)
                })
        );
        self.compute.borrow_mut().commit();
        self.compute_generation.set(next);
    }

    #[cfg(feature = "browser_compute")]
    fn abort_compute(&self) {
        self.compute.borrow_mut().abort();
    }
}

#[cfg(feature = "served_project_authority")]
#[derive(Clone)]
pub(crate) struct ServedDocumentBootstrap {
    config_source: String,
    authority: BrowserRuntimeInjectionConfig,
}

impl WasmDocumentBootstrap {
    fn source(&self) -> &Self {
        self
    }

    pub(crate) fn initial_repl_source(&self) -> String {
        self.document_base().source().to_contiguous_string()
    }

    pub(crate) fn initial_document(&self) -> SourceDocument {
        self.preserve_document_provenance(self.document_base())
    }

    fn document_base(&self) -> SourceDocument {
        let base = self.document_base.borrow();
        base.pending.as_ref().unwrap_or(&base.active).clone()
    }

    fn stage_document_base(&self, document: SourceDocument) {
        let document = self.preserve_document_provenance(document);
        self.document_base.borrow_mut().stage(document);
    }

    fn preserve_document_provenance(&self, document: SourceDocument) -> SourceDocument {
        if let Some(provenance) = self.provenance.get(&self.root_specifier) {
            return document.with_nominal_provenance(
                provenance.nominal_origin.clone(),
                provenance.nominal_package_id.clone(),
            );
        }
        if document.nominal_origin().is_some() {
            return document;
        }
        let base = self.document_base();
        let owner = if base.nominal_origin().is_some() {
            &base
        } else {
            self.document.document()
        };
        match owner.nominal_origin() {
            Some(origin) => document.with_nominal_provenance(
                origin.clone(),
                owner.nominal_package_id().map(str::to_owned),
            ),
            None => document,
        }
    }

    fn stage_repl_document_boundary(&self, request: &str) -> MResult<()> {
        use mech_runtime::{ReplCommand, ReplRequest, parse_repl_request};
        if request.trim() == ":reset" {
            self.stage_document_base(self.document.document().clone());
            return Ok(());
        }
        let Ok(ReplRequest::InvokeCommand {
            command: ReplCommand::Clear(names),
            ..
        }) = parse_repl_request(request)
        else {
            return Ok(());
        };
        let base = self.document_base();
        let source = if names.is_empty() {
            String::new()
        } else {
            let names = names.into_iter().collect();
            mech_runtime::remove_canonical_definitions(&base, &names)?.0
        };
        let mut document = SourceDocument::parse_resolved(
            "runtime:interactive",
            base.source().revision(),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .map_err(|error| document_runtime_error(format!("invalid browser boundary: {error:?}")))?;
        if let Some(origin) = base.nominal_origin() {
            document = document.with_nominal_origin(origin.clone());
        }
        if let Some(package) = base.nominal_package_id() {
            document = document.with_nominal_package_id(package);
        }
        self.stage_document_base(document);
        Ok(())
    }

    #[cfg(test)]
    fn program_output_id(&self) -> MResult<Option<OutputId>> {
        runtime_document(self, &self.document_base()).map(|(_, output)| output)
    }

    pub(crate) fn console_output_context(&self) -> String {
        format!("console://{}/output", self.source().console_instance)
    }

    pub(crate) fn prepare_commit(&self, runtime: &mut MechRuntime) -> MResult<()> {
        let mut presentation = self.presentation_state.borrow_mut();
        if let Some(state) = presentation.pending.as_mut() {
            state.program_snapshot = match state.program_output {
                Some(output) => runtime
                    .output_value(OutputId::new(output as u32))?
                    .filter(|value| !value.is_empty()),
                None => None,
            };
        }
        drop(presentation);
        #[cfg(feature = "browser_compute")]
        if let Some(compute) = self.source().lifecycle.compute() {
            compute.ensure_source_replacement_ready()?;
        }
        if self.source().lifecycle.drivers_started() {
            runtime.start_input_drivers()?;
        }
        Ok(())
    }

    pub(crate) fn commit(&self) {
        self.document_base.borrow_mut().commit();
        self.presentation_state.borrow_mut().commit();
        #[cfg(feature = "browser_compute")]
        self.source().lifecycle.commit_pointer();
        #[cfg(feature = "browser_host_scene")]
        self.source().lifecycle.commit_scenes();
        #[cfg(feature = "browser_compute")]
        self.source().lifecycle.commit_compute();
    }

    pub(crate) fn abort(&self) {
        self.document_base.borrow_mut().abort();
        self.presentation_state.borrow_mut().abort();
        #[cfg(feature = "browser_compute")]
        self.source().lifecycle.abort_pointer();
        #[cfg(feature = "browser_host_scene")]
        self.source().lifecycle.abort_scenes();
        #[cfg(feature = "browser_compute")]
        self.source().lifecycle.abort_compute();
    }
}

pub(crate) fn build_document_repl_runtime(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
) -> MResult<MechRuntime> {
    build_document_repl_runtime_for_document(bootstrap, events, bootstrap.document.document())
        .map(|candidate| candidate.runtime)
}

pub(crate) fn activate_document_repl_runtime(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    source: &str,
) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
    let document = SourceDocument::parse_resolved(
        "runtime:interactive",
        mech_syntax::document::Revision(0),
        source,
        mech_syntax::document::ParseConfig::default(),
    )
    .map_err(|error| document_runtime_error(format!("invalid browser source: {error:?}")))?;
    activate_document_repl_runtime_document(bootstrap, events, &document)
}

pub(crate) fn activate_document_repl_runtime_document(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    document: &SourceDocument,
) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
    let initial_bundle = bootstrap.source().initial_bundle.as_ref().filter(|_| {
        document.source().revision() == bootstrap.source().document.document().source().revision()
    });
    let mut candidate = build_document_repl_runtime_for_document(bootstrap, events, document)?;
    let source = document.source().to_contiguous_string();
    if source.trim().is_empty() {
        bootstrap
            .presentation_state
            .borrow_mut()
            .stage(document::DocumentOutputState {
                source,
                ..document::DocumentOutputState::default()
            });
        #[cfg(feature = "browser_host_scene")]
        bootstrap.source().lifecycle.stage_scenes(candidate.scenes);
        #[cfg(feature = "browser_compute")]
        bootstrap
            .source()
            .lifecycle
            .stage_pointer(candidate.pointer);
        #[cfg(feature = "browser_compute")]
        bootstrap
            .source()
            .lifecycle
            .stage_compute(candidate.compute.clone());
        return Ok((
            candidate.runtime,
            RuntimeProgramLoadOutcome {
                route: mech_runtime::RuntimeProgramRoute::None,
                initial_value: mech_runtime::RuntimeValueSnapshot::empty(),
                info: mech_runtime::RuntimeProgramExecutionInfo::default(),
            },
        ));
    }
    let runtime = &mut candidate.runtime;
    let durability = runtime.config().resident_durability;
    let activation = if let Some(bundle) = initial_bundle {
        runtime.load_bytecode_program(&bundle.bytecode, durability)
    } else {
        #[cfg(feature = "browser_compute")]
        {
            match candidate.coordinator.take() {
                Some(coordinator) => runtime.load_compiled_program(coordinator, durability),
                None => runtime.load_interactive_root_program(
                    SourceRequest::new(&bootstrap.source().root_specifier),
                    browser_module_options(),
                    durability,
                ),
            }
        }
        #[cfg(not(feature = "browser_compute"))]
        {
            runtime.load_interactive_root_program(
                SourceRequest::new(&bootstrap.source().root_specifier),
                browser_module_options(),
                durability,
            )
        }
    };
    let outcome = match activation {
        Ok(outcome) => outcome,
        Err(error) => {
            if let Err(shutdown_error) = candidate.runtime.shutdown() {
                bootstrap.abort();
                return Err(shutdown_error.with_source(error));
            }
            bootstrap.abort();
            return Err(error);
        }
    };
    let state = match document::document_output_state_for_runtime(
        bootstrap,
        document,
        &candidate.runtime,
        candidate.has_document_result,
    ) {
        Ok(state) => state,
        Err(error) => {
            let shutdown = candidate.runtime.shutdown();
            bootstrap.abort();
            return Err(shutdown
                .err()
                .map_or(error.clone(), |shutdown| shutdown.with_source(error)));
        }
    };
    bootstrap.presentation_state.borrow_mut().stage(state);
    #[cfg(feature = "browser_host_scene")]
    bootstrap.source().lifecycle.stage_scenes(candidate.scenes);
    #[cfg(feature = "browser_compute")]
    bootstrap
        .source()
        .lifecycle
        .stage_pointer(candidate.pointer);
    #[cfg(feature = "browser_compute")]
    bootstrap
        .source()
        .lifecycle
        .stage_compute(candidate.compute);
    Ok((candidate.runtime, outcome))
}

struct DocumentRuntimeCandidate {
    runtime: MechRuntime,
    has_document_result: bool,
    #[cfg(feature = "browser_compute")]
    pointer: Option<PointerInputHandle>,
    #[cfg(feature = "browser_compute")]
    coordinator: Option<mech_engine::ProgramArtifact>,
    #[cfg(feature = "browser_compute")]
    compute: Option<BrowserComputeBridge>,
    #[cfg(feature = "browser_host_scene")]
    scenes: BrowserSceneRegistry,
}

fn compile_browser_interactive_document(
    bootstrap: &WasmDocumentBootstrap,
    document: &SourceDocument,
) -> MResult<mech_engine::CanonicalSourceProgram> {
    let resolver = document_source_resolver(document, bootstrap)?;
    let resolved_root = mech_runtime::SourceResolver::resolve(
        &resolver,
        &SourceRequest::new(&bootstrap.root_specifier),
    )?
    .ok_or_else(|| document_runtime_error("browser document root did not resolve"))?;
    let mut compiler = document_planning_compiler(bootstrap, document)?;
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    if let Some(served) = bootstrap.served.as_ref() {
        let config = parse_config_document(
            "mech.mcfg",
            &served.config_source,
            ConfigProfileOptions::default(),
        )?;
        if config.hosts.iter().any(|host| host.provider == "compute") {
            return compiler.plan_canonical_mixed_resolved_root(resolved_root);
        }
    }
    compiler.plan_canonical_interactive_resolved_root(resolved_root)
}

fn document_planning_compiler(
    bootstrap: &WasmDocumentBootstrap,
    document: &SourceDocument,
) -> MResult<mech_runtime::ProgramCompiler> {
    let source = bootstrap.source();
    #[cfg(feature = "browser_host_scene")]
    let planning_scenes = BrowserSceneRegistry::new();
    let mut builder = runtime_builder_with_factories(
        None,
        #[cfg(feature = "browser_host_scene")]
        planning_scenes,
        #[cfg(feature = "browser_compute")]
        None,
    )
    .map_err(js_value_to_mech_error)?;
    #[cfg(feature = "browser_compute")]
    {
        builder = builder.function_catalog(mech_stdlib::source_native_plan_catalog());
    }
    let resolver = document_source_resolver(document, source)?;

    #[cfg(feature = "served_project_authority")]
    match bootstrap.served.as_ref() {
        None => {
            builder = builder
                .config(mech_runtime::RuntimeConfig::new("wasm-document-planning"))
                .source_resolver(resolver);
        }
        Some(served) => {
            let config = parse_config_document(
                "mech.mcfg",
                &served.config_source,
                ConfigProfileOptions::default(),
            )?;
            builder = builder
                .config(served.authority.into_runtime_config()?)
                .source_resolver(resolver);
            for required in config
                .hosts
                .iter()
                .filter(|host| host.provider != "compute")
            {
                if let Some(host) =
                    served.authority.hosts.iter().find(|host| {
                        host.name == required.name && host.provider == required.provider
                    })
                {
                    builder = builder.host_instance(host.clone());
                }
            }
            for grant in required_issued_grants(&config, &served.authority) {
                builder = builder.run_resource_grant(grant);
            }
        }
    }
    #[cfg(not(feature = "served_project_authority"))]
    {
        builder = builder
            .config(mech_runtime::RuntimeConfig::new("wasm-document-planning"))
            .source_resolver(resolver);
    }

    builder
        .host_instance(HostInstanceConfig {
            name: source.console_instance.clone(),
            provider: "console".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: format!("{}/output", source.console_instance),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        })
        .build_compiler()
}

#[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
fn configured_document_pointer(
    bootstrap: &WasmDocumentBootstrap,
) -> MResult<Option<PointerInputHandle>> {
    let Some(served) = bootstrap.served.as_ref() else {
        return Ok(None);
    };
    let document = parse_config_document(
        "mech.mcfg",
        &served.config_source,
        ConfigProfileOptions::default(),
    )?;
    configured_project_pointer(&document)
}

#[cfg(feature = "browser_compute")]
fn configured_project_pointer(
    document: &MechConfigDocument,
) -> MResult<Option<PointerInputHandle>> {
    let mut pointers = document
        .hosts
        .iter()
        .filter(|host| host.provider == "pointer");
    let pointer = pointers.next();
    if pointers.next().is_some() {
        return Err(document_runtime_error(
            "a browser project supports one pointer host instance",
        ));
    }
    Ok(pointer.map(|host| PointerInputHandle::new(&host.name)))
}

fn build_document_repl_runtime_for_document(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    candidate_document: &SourceDocument,
) -> MResult<DocumentRuntimeCandidate> {
    let source = bootstrap.source();
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    let candidate_pointer = configured_document_pointer(bootstrap)?;
    #[cfg(all(feature = "browser_compute", not(feature = "served_project_authority")))]
    let candidate_pointer = None;
    #[cfg(feature = "browser_compute")]
    let previous_compute = source.lifecycle.compute();
    #[cfg(feature = "browser_compute")]
    if let Some(previous) = previous_compute.as_ref() {
        previous.ensure_source_replacement_ready()?;
    }
    let (candidate_document, document_result) = runtime_document(source, candidate_document)?;
    #[cfg(feature = "browser_host_scene")]
    let candidate_scenes = BrowserSceneRegistry::new();

    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    let prepared_compute = match bootstrap.served.as_ref() {
        Some(served) => {
            let document = parse_config_document(
                "mech.mcfg",
                &served.config_source,
                ConfigProfileOptions::default(),
            )?;
            let document = compute_document_with_issued_settings(&document, &served.authority)?;
            if document.hosts.iter().any(|host| host.provider == "compute") {
                #[cfg(feature = "browser_host_scene")]
                let planning_scenes = BrowserSceneRegistry::new();
                let mut planning = runtime_builder_with_factories(
                    Some(events.clone()),
                    #[cfg(feature = "browser_host_scene")]
                    planning_scenes,
                    #[cfg(feature = "browser_compute")]
                    None,
                )
                .map_err(js_value_to_mech_error)?
                .function_catalog(mech_stdlib::source_native_plan_catalog())
                .config(served.authority.into_runtime_config()?)
                .source_resolver(document_source_resolver(&candidate_document, source)?);
                for required in document
                    .hosts
                    .iter()
                    .filter(|host| host.provider != "compute")
                {
                    if let Some(host) = served.authority.hosts.iter().find(|host| {
                        host.name == required.name && host.provider == required.provider
                    }) {
                        planning = planning.host_instance(host.clone());
                    }
                }
                for grant in required_issued_grants(&document, &served.authority) {
                    planning = planning.run_resource_grant(grant);
                }
                planning = planning
                    .host_instance(HostInstanceConfig {
                        name: source.console_instance.clone(),
                        provider: "console".to_string(),
                        settings: ConfigValue::Map(Default::default()),
                    })
                    .run_resource_grant(RunResourceGrantConfig {
                        target: format!("{}/output", source.console_instance),
                        operations: vec!["write".to_string()],
                        paths: vec!["line".to_string()],
                    });
                let compiler_started = web_time::Instant::now();
                let mut compiler = planning.build_compiler()?;
                let catalog_setup = compiler_started.elapsed().as_secs_f64() * 1_000.0;
                let prepared = prepare_compute_document_region(
                    &mut compiler,
                    &candidate_document,
                    0.0,
                    catalog_setup,
                )?;
                Some(prepare_browser_compute_runtime(
                    &document,
                    prepared,
                    browser_gpu_available(),
                    BrowserComputePurpose::ResidentDocument {
                        generation: source.lifecycle.next_compute_generation()?,
                        previous: previous_compute.as_ref(),
                    },
                )?)
            } else {
                None
            }
        }
        None => None,
    };
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    let (compute_factory, compute_coordinator, compute_bridge) = match prepared_compute {
        Some(prepared) => (
            Some(prepared.factory),
            Some(prepared.coordinator),
            Some(prepared.bridge),
        ),
        None => (None, None, None),
    };

    let mut builder = runtime_builder_with_factories(
        Some(events),
        #[cfg(feature = "browser_host_scene")]
        candidate_scenes.clone(),
        #[cfg(feature = "browser_compute")]
        candidate_pointer.clone(),
    )
    .map_err(js_value_to_mech_error)?;
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    if let Some(factory) = compute_factory {
        builder = builder.host_factory(Box::new(factory))?;
    }

    let resolver = document_source_resolver(&candidate_document, source)?;

    #[cfg(feature = "served_project_authority")]
    match bootstrap.served.as_ref() {
        None => {
            builder = builder
                .config(mech_runtime::RuntimeConfig::new("wasm-document-repl"))
                .source_resolver(resolver);
        }
        #[cfg(feature = "served_project_authority")]
        Some(served) => {
            let document = parse_config_document(
                "mech.mcfg",
                &served.config_source,
                ConfigProfileOptions::default(),
            )?;
            builder = builder
                .config(served.authority.into_runtime_config()?)
                .source_resolver(resolver);
            for required in &document.hosts {
                if let Some(host) =
                    served.authority.hosts.iter().find(|host| {
                        host.name == required.name && host.provider == required.provider
                    })
                {
                    builder = builder.host_instance(host.clone());
                }
            }
            for grant in required_issued_grants(&document, &served.authority) {
                builder = builder.run_resource_grant(grant);
            }
        }
    }
    #[cfg(not(feature = "served_project_authority"))]
    {
        builder = builder
            .config(mech_runtime::RuntimeConfig::new("wasm-document-repl"))
            .source_resolver(resolver);
    }

    let runtime = builder
        .host_instance(HostInstanceConfig {
            name: source.console_instance.clone(),
            provider: "console".to_string(),
            settings: ConfigValue::Map(Default::default()),
        })
        .run_resource_grant(RunResourceGrantConfig {
            target: format!("{}/output", source.console_instance),
            operations: vec!["write".to_string()],
            paths: vec!["line".to_string()],
        })
        .build()?;
    Ok(DocumentRuntimeCandidate {
        runtime,
        has_document_result: document_result.is_some(),
        #[cfg(feature = "browser_compute")]
        pointer: candidate_pointer,
        #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
        coordinator: compute_coordinator,
        #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
        compute: compute_bridge,
        #[cfg(all(feature = "browser_compute", not(feature = "served_project_authority")))]
        coordinator: None,
        #[cfg(all(feature = "browser_compute", not(feature = "served_project_authority")))]
        compute: None,
        #[cfg(feature = "browser_host_scene")]
        scenes: candidate_scenes,
    })
}

/// Derive the executable browser revision from the retained candidate by
/// inserting one canonical runtime-only capture at the original document boundary.
/// Console overlays remain after that boundary and cannot replace the fixed
/// document Output pane.
fn runtime_document(
    source: &WasmDocumentBootstrap,
    candidate: &SourceDocument,
) -> MResult<(SourceDocument, Option<OutputId>)> {
    use mech_syntax::document::{
        AstNode, CodeBlockSyntax, ParseConfig, Revision, SyntaxNode, TextSize,
    };

    fn enclosing_fence_end(node: &SyntaxNode, point: TextSize) -> Option<usize> {
        if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
            let range = fence.syntax().range();
            if range.start <= point && point <= range.end {
                return Some(range.end.0 as usize);
            }
        }
        node.children()
            .find_map(|child| enclosing_fence_end(&child, point))
    }

    let candidate = source.preserve_document_provenance(candidate.clone());
    let original = source.initial_repl_source();
    let candidate_source = candidate.source().to_contiguous_string();
    let (base, suffix) = candidate_source
        .strip_prefix(&original)
        .map(|suffix| (original.as_str(), suffix))
        .unwrap_or((candidate_source.as_str(), ""));
    // Resolver provenance owns packaged roots. A standalone editor owner is
    // only a fallback, and its package discriminator must not leak into a
    // different defining origin.
    let (nominal_origin, nominal_package_id) =
        if let Some(provenance) = source.provenance.get(&source.root_specifier) {
            (
                Some(provenance.nominal_origin.clone()),
                provenance.nominal_package_id.clone(),
            )
        } else {
            (
                candidate.nominal_origin().cloned(),
                candidate.nominal_package_id().map(str::to_owned),
            )
        };
    let retain_provenance = |document: SourceDocument| {
        if let Some(origin) = &nominal_origin {
            document.with_nominal_provenance(origin.clone(), nominal_package_id.clone())
        } else {
            document
        }
    };
    let base_document = retain_provenance(
        SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(candidate.source().revision().0),
            base,
            ParseConfig::default(),
        )
        .map_err(|error| document_runtime_error(format!("invalid browser source: {error:?}")))?,
    );
    if CanonicalSourceFrontend
        .root_statement_nodes(&base_document.document())
        .map_err(|error| document_runtime_error(error.to_string()))?
        .is_empty()
    {
        return Ok((candidate.clone(), None));
    }
    let original_program = compile_browser_interactive_document(source, &base_document)?;
    let program_boundary = original_program
        .document_outputs()
        .iter()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .and_then(|output| {
            original_program
                .source_map()
                .outputs
                .get(output.output as usize)
        })
        .map(|anchor| {
            enclosing_fence_end(base_document.document().syntax(), anchor.range.end)
                .unwrap_or(anchor.range.end.0 as usize)
        });
    let Some(program_boundary) = program_boundary else {
        return Ok((candidate.clone(), None));
    };

    const CAPTURE: &str = "```mech\nans\n```\n";
    let mut executable = String::with_capacity(candidate_source.len() + CAPTURE.len() + 1);
    let (program_prefix, trailing_presentation) = base.split_at(program_boundary.min(base.len()));
    executable.push_str(program_prefix);
    if !executable.ends_with(['\r', '\n']) {
        executable.push('\n');
    }
    let capture_start = executable.len();
    executable.push_str(CAPTURE);
    let capture_end = executable.len();
    executable.push_str(trailing_presentation);
    executable.push_str(suffix);
    let document = retain_provenance(
        SourceDocument::parse_resolved(
            "runtime:interactive",
            Revision(candidate.source().revision().0),
            executable,
            ParseConfig::default(),
        )
        .map_err(|error| {
            document_runtime_error(format!("invalid browser runtime source: {error:?}"))
        })?,
    );
    let program = compile_browser_interactive_document(source, &document)?;
    let output = program.document_outputs().iter().find_map(|output| {
        let anchor = program.source_map().outputs.get(output.output as usize)?;
        let start = anchor.range.start.0 as usize;
        (output.kind == SourceDocumentOutputKind::Fence
            && start >= capture_start
            && start < capture_end)
            .then_some(OutputId::new(output.output))
    });
    let output = output.ok_or_else(|| {
        document_runtime_error("canonical browser program output capture was not published")
    })?;
    Ok((document, Some(output)))
}

fn retained_submission_fragment<'a>(
    retained_source: &'a str,
    accepted_before: usize,
    submitted: &str,
) -> MResult<(&'a str, usize)> {
    let mut normalized = submitted.to_owned();
    if let Some(terminal) = mech_syntax::document::submission_terminal(submitted)
        && terminal.suppresses_value
    {
        normalized.remove(terminal.byte_offset);
    }
    let suffix = retained_source.get(accepted_before..).ok_or_else(|| {
        document_runtime_error("accepted documentation range is outside the retained source")
    })?;
    let relative_start = suffix
        .rfind(&normalized)
        .ok_or_else(|| document_runtime_error("accepted documentation source was not retained"))?;
    let relative_end = relative_start + normalized.len();
    if !suffix[relative_end..]
        .chars()
        .all(|character| matches!(character, '\r' | '\n'))
    {
        return Err(document_runtime_error(
            "accepted documentation source is not the final retained entry",
        ));
    }
    let start = accepted_before + relative_start;
    let end = start + normalized.len();
    Ok((&retained_source[start..end], start))
}

fn retained_document_fragment_addresses(
    bindings: &[document::DocumentOutputBinding],
    fragment_start: usize,
    fragment_len: usize,
) -> MResult<Vec<(mech_syntax::document::TextRange, u64)>> {
    use mech_syntax::document::{TextRange, TextSize};

    let fragment_end = fragment_start.checked_add(fragment_len).ok_or_else(|| {
        document_runtime_error("accepted documentation fragment range overflowed")
    })?;
    bindings
        .iter()
        .filter_map(|binding| {
            let (start, end) = binding.source_span?;
            (start >= fragment_start && end <= fragment_end).then_some((binding, start, end))
        })
        .map(|(binding, start, end)| {
            let start = u32::try_from(start - fragment_start).map_err(|_| {
                document_runtime_error("documentation output start exceeds renderer limits")
            })?;
            let end = u32::try_from(end - fragment_start).map_err(|_| {
                document_runtime_error("documentation output end exceeds renderer limits")
            })?;
            Ok((
                TextRange::new(TextSize(start), TextSize(end)),
                binding.output_id,
            ))
        })
        .collect()
}

#[cfg(test)]
fn live_document_fragment_addresses(
    accepted: &SourceDocument,
    runtime: &MechRuntime,
    fragment: &str,
    accepted_before: usize,
) -> MResult<Vec<(mech_syntax::document::TextRange, u64)>> {
    let retained = accepted.source().to_contiguous_string();
    let fragment_start = retained
        .rfind(fragment)
        .filter(|start| *start >= accepted_before)
        .ok_or_else(|| {
            document_runtime_error("accepted documentation fragment was not retained")
        })?;
    document::document_fragment_output_addresses(runtime, accepted, fragment_start, fragment.len())
}

fn document_runtime_error(message: impl Into<String>) -> MechError {
    MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
}

fn js_value_to_mech_error(error: JsValue) -> MechError {
    document_runtime_error(
        error
            .as_string()
            .unwrap_or_else(|| format!("browser runtime construction failed: {error:?}")),
    )
}

#[cfg(any(test, feature = "served_project_authority"))]
fn internal_repl_console_instance(hosts: &[HostInstanceConfig]) -> String {
    for candidate in std::iter::once("repl".to_string()).chain(
        std::iter::once("repl-console".to_string())
            .chain((2_u64..).map(|index| format!("repl-console-{index}"))),
    ) {
        if hosts.iter().all(|host| host.name != candidate) {
            return candidate;
        }
    }
    unreachable!("the generated REPL console namespace is unbounded")
}

mod document {
    use super::*;
    use mech_core::hash_str;

    use std::collections::HashSet;

    #[derive(Clone, Copy, Debug)]
    struct SourceEditAnchors {
        old_start: usize,
        old_end: usize,
        new_start: usize,
        new_end: usize,
    }

    #[derive(Clone, Copy, Debug)]
    pub(super) struct PendingSourceEdit {
        old_start: usize,
        old_end: usize,
        new_start: usize,
        new_end: usize,
    }

    fn utf16_to_byte_offset(source: &str, offset: u32) -> Option<usize> {
        let mut units = 0_u32;
        for (byte, character) in source.char_indices() {
            if units == offset {
                return Some(byte);
            }
            units = units.checked_add(character.len_utf16() as u32)?;
            if units > offset {
                return None;
            }
        }
        (units == offset).then_some(source.len())
    }

    #[derive(Clone, Copy, Debug, Eq, PartialEq)]
    pub(super) struct DocumentOutputBinding {
        pub(super) output_id: u64,
        semantic_id: u64,
        kind: SourceDocumentOutputKind,
        ordinal: u64,
        pub(super) source_span: Option<(usize, usize)>,
        title_slot: Option<(&'static str, usize)>,
    }

    #[derive(Clone, Debug, Eq, PartialEq)]
    struct RetiredDocumentOutput {
        output_id: u64,
        semantic_id: u64,
        kind: SourceDocumentOutputKind,
        source: String,
        anchor: usize,
        // A suppressed occurrence still owns its original DOM address. None
        // denotes actual deletion, whose restoration requires exact identity.
        unpublished_span: Option<(usize, usize)>,
        title_slot: Option<(&'static str, usize)>,
    }

    #[derive(Clone, Default)]
    pub(super) struct DocumentOutputState {
        pub(super) bindings: Vec<DocumentOutputBinding>,
        pub(super) program_output: Option<u64>,
        pub(super) source: String,
        pub(super) program_snapshot: Option<mech_runtime::RuntimeValueSnapshot>,
    }

    impl DocumentOutputState {
        fn ordinals(&self) -> HashMap<u64, u64> {
            let mut ordinals = self
                .bindings
                .iter()
                .map(|binding| (binding.output_id, binding.ordinal))
                .collect::<HashMap<_, _>>();
            if let Some(output) = self.program_output {
                ordinals.insert(root_document_program_output_id(), output);
            }
            ordinals
        }
    }

    fn document_presentation_output_identity(
        name: &str,
    ) -> Option<(SourceDocumentOutputKind, u32)> {
        let decoded = mech_engine::decode_interactive_symbol_output_name(name);
        let suffix = decoded
            .as_deref()
            .unwrap_or(name)
            .strip_prefix("document:")?;
        let (owner, offset) = suffix.rsplit_once(':')?;
        let role = owner.rsplit(':').next().unwrap_or(owner);
        let kind = match role {
            "inline" => SourceDocumentOutputKind::Inline,
            "fence" => SourceDocumentOutputKind::Fence,
            _ => return None,
        };
        Some((kind, offset.parse::<u32>().ok()?))
    }

    fn runtime_presentation_outputs(
        runtime: &MechRuntime,
    ) -> Vec<(SourceDocumentOutputKind, u32, u64)> {
        let mut outputs = runtime
            .published_outputs()
            .into_iter()
            .filter_map(|(output, name)| {
                document_presentation_output_identity(&name)
                    .map(|(kind, offset)| (kind, offset, u64::from(output.0)))
            })
            .collect::<Vec<_>>();
        outputs.sort_by_key(|(_, offset, _)| *offset);
        outputs
    }

    struct ActiveDocumentOutputs {
        presentation: Vec<(mech_syntax::document::TextRange, u64)>,
        program: Option<u64>,
    }

    fn capture_output_index(
        source: &[mech_runtime::CanonicalDocumentPresentationOutput],
        runtime: &[(SourceDocumentOutputKind, u32, u64)],
    ) -> Option<usize> {
        (0..runtime.len()).find(|&index| {
            let prefix_matches =
                source[..index]
                    .iter()
                    .zip(&runtime[..index])
                    .all(|(source, (kind, offset, _))| {
                        source.kind == *kind && source.range.start.0 == *offset
                    });
            if !prefix_matches {
                return false;
            }
            let source_suffix = &source[index..];
            let runtime_suffix = &runtime[index + 1..];
            let Some((first_source, (first_kind, first_offset, _))) =
                source_suffix.first().zip(runtime_suffix.first())
            else {
                return source_suffix.is_empty() && runtime_suffix.is_empty();
            };
            let Some(shift) = first_offset.checked_sub(first_source.range.start.0) else {
                return false;
            };
            first_source.kind == *first_kind
                && shift > 0
                && source_suffix
                    .iter()
                    .zip(runtime_suffix)
                    .all(|(source, (kind, offset, _))| {
                        source.kind == *kind
                            && offset.checked_sub(source.range.start.0) == Some(shift)
                    })
        })
    }

    fn active_document_outputs(
        candidate: &SourceDocument,
        runtime: &MechRuntime,
    ) -> MResult<ActiveDocumentOutputs> {
        let source = mech_runtime::canonical_document_presentation_outputs(&candidate.document())
            .map_err(|error| document_runtime_error(error.to_string()))?;
        let mut active = runtime_presentation_outputs(runtime);
        let program = if active.len() == source.len() {
            if !source
                .iter()
                .zip(&active)
                .all(|(source, (kind, offset, _))| {
                    source.kind == *kind && source.range.start.0 == *offset
                })
            {
                return Err(document_runtime_error(
                    "active runtime presentation offsets do not match the accepted document",
                ));
            }
            runtime
                .program_output_id()
                .map(|output| u64::from(output.0))
        } else if active.len() == source.len() + 1 {
            let index = capture_output_index(&source, &active).ok_or_else(|| {
                document_runtime_error(
                    "active runtime program capture does not align with the accepted document",
                )
            })?;
            Some(active.remove(index).2)
        } else {
            return Err(document_runtime_error(format!(
                "accepted document has {} presentation outputs for {} active runtime outputs",
                source.len(),
                active.len()
            )));
        };
        Ok(ActiveDocumentOutputs {
            presentation: source
                .into_iter()
                .zip(active)
                .map(|(source, (_, _, output))| (source.range, output))
                .collect(),
            program,
        })
    }

    #[cfg(test)]
    pub(super) fn document_fragment_output_addresses(
        runtime: &MechRuntime,
        candidate: &SourceDocument,
        fragment_start: usize,
        fragment_len: usize,
    ) -> MResult<Vec<(mech_syntax::document::TextRange, u64)>> {
        use mech_syntax::document::{TextRange, TextSize};

        let fragment_end = fragment_start.checked_add(fragment_len).ok_or_else(|| {
            document_runtime_error("accepted documentation fragment range overflowed")
        })?;
        Ok(active_document_outputs(candidate, runtime)?
            .presentation
            .into_iter()
            .filter_map(|(range, output)| {
                let start = range.start.0 as usize;
                let end = range.end.0 as usize;
                (start >= fragment_start && end <= fragment_end).then(|| {
                    (
                        TextRange::new(
                            TextSize((start - fragment_start) as u32),
                            TextSize((end - fragment_start) as u32),
                        ),
                        output,
                    )
                })
            })
            .collect())
    }

    #[cfg(test)]
    pub(super) fn document_output_ordinals_for_runtime(
        bootstrap: &WasmDocumentBootstrap,
        candidate: &SourceDocument,
        runtime: &MechRuntime,
        require_all: bool,
    ) -> MResult<HashMap<u64, u64>> {
        let active = active_document_outputs(candidate, runtime)?;
        if require_all && active.presentation.len() != bootstrap.presentation_output_ids.len() {
            return Err(document_runtime_error(format!(
                "active runtime has {} presentation outputs for {} browser addresses",
                active.presentation.len(),
                bootstrap.presentation_output_ids.len()
            )));
        }
        let mut ordinals = bootstrap
            .presentation_output_ids
            .iter()
            .copied()
            .zip(active.presentation.into_iter().map(|(_, ordinal)| ordinal))
            .collect::<HashMap<_, _>>();
        if let Some(output) = active.program {
            ordinals.insert(root_document_program_output_id(), output);
        }
        Ok(ordinals)
    }

    fn source_spans_overlap(left: (usize, usize), right: (usize, usize)) -> bool {
        left.0 < right.1 && right.0 < left.1
    }

    // Bind the accepted source to outputs published by the loaded artifact.
    // This runs during candidate activation, before the resident handoff commits.
    pub(super) fn document_output_state_for_runtime(
        bootstrap: &WasmDocumentBootstrap,
        candidate: &SourceDocument,
        runtime: &MechRuntime,
        has_document_result: bool,
    ) -> MResult<DocumentOutputState> {
        use mech_syntax::document::{AstNode, SyntaxKind, TextRange};
        let syntax = candidate.document();
        let source = candidate.source().to_contiguous_string();
        let metadata = mech_runtime::canonical_document_presentation_outputs(&syntax)
            .map_err(|error| document_runtime_error(error.to_string()))?;
        let mut active = active_document_outputs(candidate, runtime)?;
        if !has_document_result {
            active.program = None;
        }
        let initial =
            candidate.source().revision() == bootstrap.document.document().source().revision();
        if initial && metadata.len() != bootstrap.presentation_output_ids.len() {
            return Err(document_runtime_error(format!(
                "accepted document has {} presentation outputs for {} browser addresses",
                metadata.len(),
                bootstrap.presentation_output_ids.len()
            )));
        }
        // A shim has one mount per recognized title field, owned by its final
        // authored value. Earlier duplicate values must not reclaim that mount.
        const TITLE_FIELDS: [&str; 8] = [
            "author", "date", "hero", "kicker", "section", "summary", "next", "previous",
        ];
        let mut fields = Vec::<(&'static str, TextRange)>::new();
        let mut winners = HashMap::<&'static str, TextRange>::new();
        if let Some(front) = syntax.title().and_then(|title| title.front_matter()) {
            let mut field = None;
            for child in front.syntax().children() {
                if child.kind() == SyntaxKind::Identifier {
                    let text = child.source().text(child.range()).map_err(|error| {
                        document_runtime_error(format!("invalid title field: {error:?}"))
                    })?;
                    let text = text.trim().to_uppercase();
                    field = TITLE_FIELDS
                        .iter()
                        .copied()
                        .find(|name| name.to_uppercase() == text);
                } else if matches!(
                    child.kind(),
                    SyntaxKind::InlineParagraph | SyntaxKind::Img | SyntaxKind::Figures
                ) {
                    if let Some(name) = field.take() {
                        fields.push((name, child.range()));
                        winners.insert(name, child.range());
                    }
                }
            }
        }
        let fences = root_fence_occurrences(&source);
        let mut title_occurrences = HashMap::<&'static str, usize>::new();
        let mut bindings = Vec::new();
        for (index, (output, (_, ordinal))) in
            metadata.into_iter().zip(active.presentation).enumerate()
        {
            let field = fields.iter().find(|(_, range)| {
                range.start <= output.range.start && output.range.end <= range.end
            });
            let title_slot = if let Some((name, range)) = field {
                if winners.get(name) != Some(range) {
                    continue;
                }
                let occurrence = title_occurrences.entry(name).or_default();
                let slot = Some((*name, *occurrence));
                *occurrence += 1;
                slot
            } else {
                None
            };
            let span = (output.range.start.0 as usize, output.range.end.0 as usize);
            let span = if output.kind == SourceDocumentOutputKind::Fence {
                fences
                    .iter()
                    .copied()
                    .find(|fence| source_spans_overlap(*fence, span))
                    .unwrap_or(span)
            } else {
                span
            };
            let text = source.get(span.0..span.1).ok_or_else(|| {
                document_runtime_error("presentation output is outside the accepted source")
            })?;
            bindings.push(DocumentOutputBinding {
                output_id: if initial {
                    bootstrap.presentation_output_ids[index]
                } else {
                    output.output_id
                },
                semantic_id: mech_core::hash_str(&format!("document:{:?}:{text}", output.kind)),
                kind: output.kind,
                ordinal,
                source_span: Some(span),
                title_slot,
            });
        }
        let program_snapshot = match active.program {
            Some(output) => runtime
                .output_value(OutputId::new(u32::try_from(output).map_err(|_| {
                    document_runtime_error("program output exceeds browser limits")
                })?))?
                .filter(|value| !value.is_empty()),
            None => None,
        };
        Ok(DocumentOutputState {
            bindings,
            program_output: active.program,
            source,
            program_snapshot,
        })
    }

    fn occurrence_output_id(
        kind: SourceDocumentOutputKind,
        semantic_id: u64,
        occurrence: u64,
    ) -> u64 {
        if occurrence == 0 {
            return semantic_id;
        }
        match kind {
            SourceDocumentOutputKind::Inline => hash_str(&format!(
                "mech/inline-document-output-occurrence/v1/{semantic_id}/{occurrence}"
            )),
            SourceDocumentOutputKind::Fence => hash_str(&format!(
                "mech/fenced-document-output/{semantic_id}/{occurrence}"
            )),
            SourceDocumentOutputKind::Program => root_document_program_output_id(),
        }
    }

    fn inferred_source_edit(previous: &str, next: &str) -> Option<SourceEditAnchors> {
        if previous == next {
            return None;
        }

        let mut prefix = previous
            .bytes()
            .zip(next.bytes())
            .take_while(|(old, new)| old == new)
            .count();
        while prefix > 0 && (!previous.is_char_boundary(prefix) || !next.is_char_boundary(prefix)) {
            prefix -= 1;
        }

        let suffix_limit = (previous.len() - prefix).min(next.len() - prefix);
        let mut suffix = previous
            .as_bytes()
            .iter()
            .rev()
            .zip(next.as_bytes().iter().rev())
            .take(suffix_limit)
            .take_while(|(old, new)| old == new)
            .count();
        while suffix > 0
            && (!previous.is_char_boundary(previous.len() - suffix)
                || !next.is_char_boundary(next.len() - suffix))
        {
            suffix -= 1;
        }

        Some(SourceEditAnchors {
            old_start: prefix,
            old_end: previous.len() - suffix,
            new_start: prefix,
            new_end: next.len() - suffix,
        })
    }

    fn binding_intersects_edit(binding: &DocumentOutputBinding, start: usize, end: usize) -> bool {
        binding
            .source_span
            .is_some_and(|(binding_start, binding_end)| {
                if start == end {
                    binding_start <= start && binding_end >= end
                } else {
                    binding_start < end && binding_end > start
                }
            })
    }

    #[derive(Clone, Copy)]
    enum OffsetAffinity {
        BeforeInsertion,
        AfterInsertion,
    }

    fn offset_after_edit(
        offset: usize,
        edit: SourceEditAnchors,
        affinity: OffsetAffinity,
    ) -> usize {
        if offset < edit.old_start
            || (offset == edit.old_start && matches!(affinity, OffsetAffinity::BeforeInsertion))
        {
            offset
        } else if offset >= edit.old_end {
            edit.new_end.saturating_add(offset - edit.old_end)
        } else {
            edit.new_start
        }
    }

    fn binding_source<'a>(binding: &DocumentOutputBinding, source: &'a str) -> Option<&'a str> {
        let (start, end) = binding.source_span?;
        source.get(start..end)
    }

    fn root_fence_occurrences(source: &str) -> Vec<(usize, usize)> {
        use mech_syntax::document::{AstNode, CodeBlockSyntax, CodeFenceScope};
        let Ok(document) = SourceDocument::parse_resolved(
            "browser:occurrence",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        ) else {
            return Vec::new();
        };
        let mut nodes = vec![document.document().syntax().clone()];
        let mut spans = Vec::new();
        while let Some(node) = nodes.pop() {
            if let Some(fence) = CodeBlockSyntax::cast(node.clone()) {
                if fence.mech_code().is_some()
                    && matches!(
                        fence.info().map(|info| info.scope),
                        Some(CodeFenceScope::Root)
                    )
                {
                    // Syntax-node ranges can include neighboring newline trivia.
                    // The occurrence owns its delimiters, independently of prose
                    // inserted immediately before or after the fence.
                    if let Some(range) = fence.delimiter_range() {
                        spans.push((range.start.0 as usize, range.end.0 as usize));
                    }
                }
            }
            nodes.extend(node.children());
        }
        spans.sort_unstable();
        spans
    }

    fn continuing_fence_occurrence(
        span: (usize, usize),
        edit: SourceEditAnchors,
        next: &[(usize, usize)],
    ) -> Option<(usize, usize)> {
        // Replacing/deleting a whole fence ends its occurrence even when an
        // unrelated fence is inserted at the same location in the same edit.
        if edit.old_start <= span.0 && edit.old_end >= span.1 && edit.old_start != edit.old_end {
            return None;
        }
        let mapped = (
            // Insertions at the opening boundary precede the occurrence;
            // insertions at its closing boundary belong after it.
            offset_after_edit(span.0, edit, OffsetAffinity::AfterInsertion),
            offset_after_edit(span.1, edit, OffsetAffinity::BeforeInsertion),
        );
        next.iter().copied().find(|candidate| *candidate == mapped)
    }

    fn retain_output_identities(
        previous: &[DocumentOutputBinding],
        next: &mut [DocumentOutputBinding],
        previous_source: &str,
        next_source: &str,
        edit: Option<SourceEditAnchors>,
        reserved: &mut HashSet<u64>,
        retired: &mut Vec<RetiredDocumentOutput>,
    ) {
        let edit = edit.or_else(|| inferred_source_edit(previous_source, next_source));
        let mut old_assigned = vec![false; previous.len()];
        let mut assigned = vec![false; next.len()];
        // Title markup keeps one slot per recognized field. Its public address
        // follows the last field occurrence, even when an earlier occurrence
        // survives unchanged or the winning expression changes completely.
        for new_index in 0..next.len() {
            if let Some(slot) = next[new_index].title_slot
                && let Some(old_index) =
                    previous.iter().position(|old| old.title_slot == Some(slot))
            {
                preserve_output_identity(
                    previous,
                    next,
                    old_index,
                    new_index,
                    &mut old_assigned,
                    &mut assigned,
                );
            }
        }
        let mut groups = HashMap::<
            (SourceDocumentOutputKind, u64, Option<(&'static str, usize)>),
            (Vec<usize>, Vec<usize>),
        >::new();
        for (index, binding) in previous.iter().enumerate() {
            if old_assigned[index] {
                continue;
            }
            groups
                .entry((binding.kind, binding.semantic_id, binding.title_slot))
                .or_default()
                .0
                .push(index);
        }
        for (index, binding) in next.iter().enumerate() {
            if assigned[index] {
                continue;
            }
            groups
                .entry((binding.kind, binding.semantic_id, binding.title_slot))
                .or_default()
                .1
                .push(index);
        }

        // Static document markup can outlive several accepted source edits.
        // Keep every address ever published by this document reserved so a
        // removed placeholder can never begin displaying a later output.
        let mut claimed = reserved.clone();
        claimed.extend(previous.iter().map(|binding| binding.output_id));
        for (old, new) in groups.values() {
            if let Some(edit) = edit {
                let old_before = old
                    .iter()
                    .copied()
                    .filter(|&index| {
                        previous[index]
                            .source_span
                            .is_some_and(|(_, end)| end <= edit.old_start)
                    })
                    .collect::<Vec<_>>();
                let new_before = new
                    .iter()
                    .copied()
                    .filter(|&index| {
                        next[index]
                            .source_span
                            .is_some_and(|(_, end)| end <= edit.new_start)
                    })
                    .collect::<Vec<_>>();
                for (old_index, new_index) in old_before.into_iter().zip(new_before) {
                    preserve_output_identity(
                        previous,
                        next,
                        old_index,
                        new_index,
                        &mut old_assigned,
                        &mut assigned,
                    );
                }
                let old_after = old
                    .iter()
                    .rev()
                    .copied()
                    .filter(|&index| {
                        previous[index]
                            .source_span
                            .is_some_and(|(start, _)| start >= edit.old_end)
                    })
                    .collect::<Vec<_>>();
                let new_after = new
                    .iter()
                    .rev()
                    .copied()
                    .filter(|&index| {
                        next[index]
                            .source_span
                            .is_some_and(|(start, _)| start >= edit.new_end)
                    })
                    .collect::<Vec<_>>();
                for (old_index, new_index) in old_after.into_iter().zip(new_after) {
                    if !old_assigned[old_index] && !assigned[new_index] {
                        preserve_output_identity(
                            previous,
                            next,
                            old_index,
                            new_index,
                            &mut old_assigned,
                            &mut assigned,
                        );
                    }
                }
            } else if previous_source == next_source || old.len() == new.len() {
                // Full-source replacement has no insertion boundary. Equal
                // cardinality makes authored order the stable correspondence
                // for otherwise indistinguishable duplicate outputs.
                for (&old_index, &new_index) in old.iter().zip(new) {
                    preserve_output_identity(
                        previous,
                        next,
                        old_index,
                        new_index,
                        &mut old_assigned,
                        &mut assigned,
                    );
                }
            }

            let remaining_old = old
                .iter()
                .copied()
                .filter(|&index| !old_assigned[index])
                .collect::<Vec<_>>();
            let remaining_new = new
                .iter()
                .copied()
                .filter(|&index| !assigned[index])
                .collect::<Vec<_>>();
            // An explicit or inferred edit boundary makes authored order the
            // stable correspondence for outputs changed inside that boundary.
            if edit.is_some() {
                for (old_index, new_index) in remaining_old.into_iter().zip(remaining_new) {
                    preserve_output_identity(
                        previous,
                        next,
                        old_index,
                        new_index,
                        &mut old_assigned,
                        &mut assigned,
                    );
                }
            } else if remaining_old.len() == 1 && remaining_new.len() == 1 {
                preserve_output_identity(
                    previous,
                    next,
                    remaining_old[0],
                    remaining_new[0],
                    &mut old_assigned,
                    &mut assigned,
                );
            }
        }

        // An edited output may retain its authored location while changing its
        // semantic hash. Pair the still-unmatched outputs inside the edit by
        // kind and order so live DOM placeholders keep their public address.
        if let Some(edit) = edit {
            let mut changed = HashMap::<
                (SourceDocumentOutputKind, Option<(&'static str, usize)>),
                (Vec<usize>, Vec<usize>),
            >::new();
            for (index, binding) in previous.iter().enumerate() {
                if !old_assigned[index]
                    && binding_intersects_edit(binding, edit.old_start, edit.old_end)
                {
                    changed
                        .entry((binding.kind, binding.title_slot))
                        .or_default()
                        .0
                        .push(index);
                }
            }
            for (index, binding) in next.iter().enumerate() {
                if !assigned[index]
                    && binding_intersects_edit(binding, edit.new_start, edit.new_end)
                {
                    changed
                        .entry((binding.kind, binding.title_slot))
                        .or_default()
                        .1
                        .push(index);
                }
            }
            for (old, new) in changed.values() {
                for (&old_index, &new_index) in old.iter().zip(new) {
                    preserve_output_identity(
                        previous,
                        next,
                        old_index,
                        new_index,
                        &mut old_assigned,
                        &mut assigned,
                    );
                }
            }
        }

        // A removed placeholder may still exist in the static DOM. Retain
        // enough source identity to reactivate that exact address if a later
        // edit restores the same output at the removal site. Other outputs
        // continue to skip every retired address through `reserved`.
        if let Some(edit) = edit {
            let previous_fences = root_fence_occurrences(previous_source);
            let next_fences = root_fence_occurrences(next_source);
            for tombstone in retired.iter_mut() {
                tombstone.anchor =
                    offset_after_edit(tombstone.anchor, edit, OffsetAffinity::BeforeInsertion);
                tombstone.unpublished_span = tombstone
                    .unpublished_span
                    .and_then(|span| continuing_fence_occurrence(span, edit, &next_fences));
            }
            let mut restored_tombstones = Vec::new();
            for (index, binding) in next.iter_mut().enumerate() {
                if assigned[index] {
                    continue;
                }
                let Some(source) = binding_source(binding, next_source) else {
                    continue;
                };
                let Some(tombstone_index) =
                    retired
                        .iter()
                        .enumerate()
                        .find_map(|(tombstone_index, tombstone)| {
                            (!restored_tombstones.contains(&tombstone_index)
                                && tombstone.kind == binding.kind
                                && tombstone.title_slot == binding.title_slot
                                && (binding.title_slot.is_some()
                                    || tombstone.unpublished_span.is_some_and(|span| {
                                        binding.source_span.is_some_and(|binding_span| {
                                            source_spans_overlap(span, binding_span)
                                        })
                                    })
                                    || (binding_intersects_edit(
                                        binding,
                                        edit.new_start,
                                        edit.new_end,
                                    ) && tombstone.anchor == edit.new_start
                                        && tombstone.semantic_id == binding.semantic_id
                                        && tombstone.source == source)))
                                .then_some(tombstone_index)
                        })
                else {
                    continue;
                };
                binding.output_id = retired[tombstone_index].output_id;
                assigned[index] = true;
                claimed.insert(binding.output_id);
                restored_tombstones.push(tombstone_index);
            }
            restored_tombstones.sort_unstable();
            restored_tombstones.dedup();
            for index in restored_tombstones.into_iter().rev() {
                retired.remove(index);
            }

            for (index, binding) in previous.iter().enumerate() {
                if old_assigned[index]
                    || retired
                        .iter()
                        .any(|tombstone| tombstone.output_id == binding.output_id)
                {
                    continue;
                }
                let Some(source) = binding_source(binding, previous_source) else {
                    continue;
                };
                let anchor = if binding_intersects_edit(binding, edit.old_start, edit.old_end) {
                    edit.new_start
                } else {
                    binding
                        .source_span
                        .map(|(start, _)| {
                            offset_after_edit(start, edit, OffsetAffinity::BeforeInsertion)
                        })
                        .unwrap_or(edit.new_start)
                };
                retired.push(RetiredDocumentOutput {
                    output_id: binding.output_id,
                    semantic_id: binding.semantic_id,
                    kind: binding.kind,
                    title_slot: binding.title_slot,
                    source: source.to_owned(),
                    anchor,
                    unpublished_span: (binding.kind == SourceDocumentOutputKind::Fence)
                        .then(|| {
                            let span = binding.source_span?;
                            let fence = previous_fences
                                .iter()
                                .copied()
                                // Canonical output ranges can include newline
                                // trivia outside the owned delimiter span.
                                .find(|fence| source_spans_overlap(*fence, span))?;
                            continuing_fence_occurrence(fence, edit, &next_fences)
                        })
                        .flatten(),
                });
            }
        }

        for ((kind, semantic_id, _), (_, new)) in groups {
            let mut occurrence = 0_u64;
            for index in new {
                if assigned[index] {
                    continue;
                }
                let output_id = loop {
                    let candidate = occurrence_output_id(kind, semantic_id, occurrence);
                    occurrence = occurrence
                        .checked_add(1)
                        .expect("document output occurrence space is exhausted");
                    if claimed.insert(candidate) {
                        break candidate;
                    }
                };
                next[index].output_id = output_id;
                assigned[index] = true;
            }
        }
        reserved.extend(next.iter().map(|binding| binding.output_id));
    }

    fn preserve_output_identity(
        previous: &[DocumentOutputBinding],
        next: &mut [DocumentOutputBinding],
        old_index: usize,
        new_index: usize,
        old_assigned: &mut [bool],
        assigned: &mut [bool],
    ) {
        next[new_index].output_id = previous[old_index].output_id;
        old_assigned[old_index] = true;
        assigned[new_index] = true;
    }

    #[cfg(test)]
    mod output_identity_tests {
        use super::*;

        fn retain_output_identities(
            previous: &[DocumentOutputBinding],
            next: &mut [DocumentOutputBinding],
            previous_source: &str,
            next_source: &str,
            edit: Option<SourceEditAnchors>,
            reserved: &mut HashSet<u64>,
        ) {
            super::retain_output_identities(
                previous,
                next,
                previous_source,
                next_source,
                edit,
                reserved,
                &mut Vec::new(),
            );
        }

        fn fence_with_semantic(
            output_id: u64,
            semantic_id: u64,
            ordinal: u64,
            start: usize,
        ) -> DocumentOutputBinding {
            DocumentOutputBinding {
                output_id,
                semantic_id,
                kind: SourceDocumentOutputKind::Fence,
                ordinal,
                source_span: Some((start, start + 1)),
                title_slot: None,
            }
        }

        fn fence(output_id: u64, ordinal: u64, start: usize) -> DocumentOutputBinding {
            fence_with_semantic(output_id, 17, ordinal, start)
        }

        #[test]
        fn fence_occurrence_affinity_excludes_inserted_neighbors() {
            for prefix in ["Prose before the fence.\n\n", "~~~mech\n22\n~~~\n\n"] {
                let source = "~~~mech{output: false}\n22\n~~~\n";
                let span = root_fence_occurrences(source)[0];
                let prefixed = format!("{prefix}{source}");
                let after_prefix = root_fence_occurrences(&prefixed);
                let edit = SourceEditAnchors {
                    old_start: 0,
                    old_end: 0,
                    new_start: 0,
                    new_end: prefix.len(),
                };
                let shifted = continuing_fence_occurrence(span, edit, &after_prefix)
                    .unwrap_or_else(|| {
                        panic!("prefix={prefix:?}, span={span:?}, next={after_prefix:?}")
                    });
                let appended = format!("{prefixed}\nProse after the fence.\n");
                let after_suffix = root_fence_occurrences(&appended);
                let edit = SourceEditAnchors {
                    old_start: prefixed.len(),
                    old_end: prefixed.len(),
                    new_start: prefixed.len(),
                    new_end: appended.len(),
                };
                assert_eq!(
                    continuing_fence_occurrence(shifted, edit, &after_suffix),
                    Some(shifted),
                    "prefix={prefix:?}, next={after_suffix:?}"
                );
            }
        }

        #[test]
        fn prepended_identical_fence_does_not_renumber_retained_outputs() {
            let base = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let third = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 2);
            let previous = vec![fence(base, 4, 0), fence(second, 5, 2)];
            let mut next = vec![fence(base, 4, 0), fence(second, 5, 2), fence(third, 6, 4)];
            let mut reserved = HashSet::from([base, second]);

            retain_output_identities(
                &previous,
                &mut next,
                "F F",
                "F F F",
                Some(SourceEditAnchors {
                    old_start: 0,
                    old_end: 0,
                    new_start: 0,
                    new_end: 2,
                }),
                &mut reserved,
            );

            assert_eq!(
                next.iter()
                    .map(|binding| binding.output_id)
                    .collect::<Vec<_>>(),
                vec![third, base, second]
            );
        }

        #[test]
        fn appended_identical_fence_keeps_leading_output_addresses() {
            let base = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let third = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 2);
            let previous = vec![fence(base, 4, 0), fence(second, 5, 2)];
            let mut next = vec![fence(base, 4, 0), fence(second, 5, 2), fence(third, 6, 4)];
            let mut reserved = HashSet::from([base, second]);

            retain_output_identities(
                &previous,
                &mut next,
                "F F",
                "F F F",
                Some(SourceEditAnchors {
                    old_start: 3,
                    old_end: 3,
                    new_start: 3,
                    new_end: 5,
                }),
                &mut reserved,
            );

            assert_eq!(
                next.iter()
                    .map(|binding| binding.output_id)
                    .collect::<Vec<_>>(),
                vec![base, second, third]
            );
        }

        #[test]
        fn appended_fragment_skips_an_id_retired_by_an_earlier_edit() {
            let base = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let third = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 2);
            // The first duplicate was removed in an earlier edit, so the
            // surviving fence deliberately retains the second occurrence ID.
            let previous = vec![fence(second, 4, 0)];
            let mut next = vec![fence(base, 4, 0), fence(second, 5, 2)];
            let mut reserved = HashSet::from([base, second]);

            retain_output_identities(
                &previous,
                &mut next,
                "F",
                "F F",
                Some(SourceEditAnchors {
                    old_start: 1,
                    old_end: 1,
                    new_start: 1,
                    new_end: 3,
                }),
                &mut reserved,
            );

            assert_eq!(next[0].output_id, second);
            assert_eq!(next[1].output_id, third);
            let addresses =
                super::super::retained_document_fragment_addresses(&next, 2, 1).unwrap();
            assert_eq!(addresses.len(), 1);
            assert_eq!(addresses[0].1, third);
        }

        #[test]
        fn restoring_a_deleted_output_reactivates_its_original_address() {
            let original = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let previous = vec![fence(original, 4, 0)];
            let mut removed = Vec::new();
            let mut reserved = HashSet::from([original]);
            let mut retired = Vec::new();

            super::retain_output_identities(
                &previous,
                &mut removed,
                "F",
                "",
                Some(SourceEditAnchors {
                    old_start: 0,
                    old_end: 1,
                    new_start: 0,
                    new_end: 0,
                }),
                &mut reserved,
                &mut retired,
            );
            assert_eq!(retired.len(), 1);

            let mut restored = vec![fence(original, 5, 0)];
            super::retain_output_identities(
                &removed,
                &mut restored,
                "",
                "F",
                Some(SourceEditAnchors {
                    old_start: 0,
                    old_end: 0,
                    new_start: 0,
                    new_end: 1,
                }),
                &mut reserved,
                &mut retired,
            );

            assert_eq!(restored[0].output_id, original);
            assert!(retired.is_empty());
        }

        #[test]
        fn an_identical_output_at_another_location_does_not_reuse_a_tombstone() {
            let original = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let previous = vec![fence(original, 4, 0)];
            let mut removed = Vec::new();
            let mut reserved = HashSet::from([original]);
            let mut retired = Vec::new();

            super::retain_output_identities(
                &previous,
                &mut removed,
                "F X",
                " X",
                Some(SourceEditAnchors {
                    old_start: 0,
                    old_end: 1,
                    new_start: 0,
                    new_end: 0,
                }),
                &mut reserved,
                &mut retired,
            );

            let mut appended = vec![fence(original, 5, 3)];
            super::retain_output_identities(
                &removed,
                &mut appended,
                " X",
                " X F",
                Some(SourceEditAnchors {
                    old_start: 2,
                    old_end: 2,
                    new_start: 2,
                    new_end: 4,
                }),
                &mut reserved,
                &mut retired,
            );

            assert_eq!(appended[0].output_id, second);
            assert_eq!(retired[0].output_id, original);
        }

        #[test]
        fn full_replacement_preserves_equal_duplicate_groups_by_order() {
            let base = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let previous = vec![fence(base, 4, 2), fence(second, 5, 4)];
            let mut next = vec![fence(base, 4, 8), fence(second, 5, 10)];
            let mut reserved = HashSet::from([base, second]);

            retain_output_identities(
                &previous,
                &mut next,
                "A F F",
                "Changed F F",
                None,
                &mut reserved,
            );

            assert_eq!(next[0].output_id, base);
            assert_eq!(next[1].output_id, second);
        }

        #[test]
        fn full_replacement_infers_duplicate_insertion_side() {
            let base = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let second = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 1);
            let third = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 2);
            let previous = vec![fence(base, 4, 0), fence(second, 5, 2)];
            let mut next = vec![fence(base, 4, 0), fence(second, 5, 2), fence(0, 6, 4)];
            let mut reserved = HashSet::from([base, second]);

            retain_output_identities(&previous, &mut next, "F F", "F F F", None, &mut reserved);

            assert_eq!(
                next.iter()
                    .map(|binding| binding.output_id)
                    .collect::<Vec<_>>(),
                vec![base, second, third]
            );
        }

        #[test]
        fn edited_output_keeps_its_public_id_when_semantics_change() {
            let old_id = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let generated_id = occurrence_output_id(SourceDocumentOutputKind::Fence, 29, 0);
            let previous = vec![fence_with_semantic(old_id, 17, 4, 0)];
            let mut next = vec![fence_with_semantic(generated_id, 29, 5, 0)];
            let mut reserved = HashSet::from([old_id]);

            retain_output_identities(
                &previous,
                &mut next,
                "A",
                "B",
                Some(SourceEditAnchors {
                    old_start: 0,
                    old_end: 1,
                    new_start: 0,
                    new_end: 1,
                }),
                &mut reserved,
            );

            assert_eq!(next[0].output_id, old_id);
            assert_eq!(next[0].semantic_id, 29);
            assert_eq!(next[0].ordinal, 5);
        }

        #[test]
        fn full_replacement_infers_semantic_change_and_keeps_public_id() {
            let old_id = occurrence_output_id(SourceDocumentOutputKind::Fence, 17, 0);
            let generated_id = occurrence_output_id(SourceDocumentOutputKind::Fence, 29, 0);
            let previous = vec![fence_with_semantic(old_id, 17, 4, 0)];
            let mut next = vec![fence_with_semantic(generated_id, 29, 5, 0)];
            let mut reserved = HashSet::from([old_id]);

            retain_output_identities(&previous, &mut next, "A", "B", None, &mut reserved);

            assert_eq!(next[0].output_id, old_id);
            assert_eq!(next[0].semantic_id, 29);
            assert_eq!(next[0].ordinal, 5);
        }

        #[test]
        fn utf16_edit_offsets_reject_half_a_surrogate_pair() {
            assert_eq!(utf16_to_byte_offset("a😀b", 1), Some(1));
            assert_eq!(utf16_to_byte_offset("a😀b", 2), None);
            assert_eq!(utf16_to_byte_offset("a😀b", 3), Some(5));
        }
    }

    fn selected_value_response(
        response: JsValue,
        presentation: Option<mech_runtime::ValueOutput>,
        block_html: Option<String>,
        name: &str,
        identity: Option<&str>,
    ) -> Result<JsValue, JsValue> {
        let result = Object::new();
        Reflect::set(&result, &JsValue::from_str("response"), &response)?;
        Reflect::set(
            &result,
            &JsValue::from_str("identity"),
            &identity.map(JsValue::from_str).unwrap_or(JsValue::NULL),
        )?;
        let rendered = match (presentation, block_html) {
            (Some(presentation), Some(block_html)) => {
                let rendered = Object::new();
                Reflect::set(
                    &rendered,
                    &JsValue::from_str("name"),
                    &JsValue::from_str(name),
                )?;
                Reflect::set(
                    &rendered,
                    &JsValue::from_str("kind"),
                    &JsValue::from_str(&presentation.kind),
                )?;
                Reflect::set(
                    &rendered,
                    &JsValue::from_str("blockHtml"),
                    &JsValue::from_str(&block_html),
                )?;
                Reflect::set(
                    &rendered,
                    &JsValue::from_str("inlineHtml"),
                    &JsValue::from_str(&mech_core::escape_html_text(&presentation.text)),
                )?;
                rendered.into()
            }
            _ => JsValue::NULL,
        };
        Reflect::set(&result, &JsValue::from_str("rendered"), &rendered)?;
        Ok(result.into())
    }

    struct DocumentProgramOutput {
        selection_token: String,
        output_id: OutputId,
    }

    fn capture_program_output(
        repl: &mut crate::repl::WasmRepl,
        bootstrap: &WasmDocumentBootstrap,
        previous: Option<&DocumentProgramOutput>,
        prepared_token: Option<&str>,
    ) -> MResult<Option<DocumentProgramOutput>> {
        let output_id = bootstrap
            .presentation_state
            .borrow()
            .active
            .program_output
            .and_then(|ordinal| u32::try_from(ordinal).ok())
            .map(OutputId::new);
        let captured = bootstrap
            .presentation_state
            .borrow()
            .active
            .program_snapshot
            .clone();
        let Some(snapshot) = captured.filter(|value| !value.is_empty()) else {
            if let Some(token) = previous
                .map(|previous| previous.selection_token.as_str())
                .or(prepared_token)
            {
                repl.session.release_retained_selection(token);
            }
            return Ok(None);
        };
        let selection_token = if let Some(previous) = previous {
            repl.session
                .refresh_retained_selection(&previous.selection_token, "ans", snapshot)?;
            previous.selection_token.clone()
        } else if let Some(token) = prepared_token {
            repl.session
                .refresh_retained_selection(token, "ans", snapshot)?;
            token.to_string()
        } else {
            // Construction has no previously accepted runtime to preserve.
            repl.session.retain_selection("ans", snapshot, None)?
        };
        Ok(Some(DocumentProgramOutput {
            selection_token,
            output_id: output_id.unwrap(),
        }))
    }

    #[wasm_bindgen]
    pub struct WasmDocument {
        pub(super) repl: crate::repl::WasmRepl,
        pub(super) bootstrap: WasmDocumentBootstrap,
        document_output_ordinals: HashMap<u64, u64>,
        document_output_bindings: Vec<DocumentOutputBinding>,
        reserved_document_output_ids: HashSet<u64>,
        retired_document_outputs: Vec<RetiredDocumentOutput>,
        document_output_source: String,
        program_output: Option<DocumentProgramOutput>,
        started: bool,
        stopped: bool,
    }

    #[wasm_bindgen]
    impl WasmDocument {
        #[wasm_bindgen(js_name = fromEncoded)]
        pub fn from_encoded(encoded: &str) -> Result<WasmDocument, JsValue> {
            let payload = decode_document_payload(encoded)?;
            let root_specifier = payload.root_specifier().to_owned();
            let source_map = HashMap::from([(root_specifier.clone(), payload.source().to_owned())]);
            Self::from_payload_with_sources(payload, &root_specifier, source_map, Vec::new())
        }

        /// Builds a formatted source document with a resolver rooted at its
        /// logical source specifier. This keeps relative imports available without
        /// requiring a configured project.
        #[wasm_bindgen(js_name = fromEncodedWithSources)]
        pub fn from_encoded_with_sources(
            encoded: &str,
            root_specifier: &str,
            sources: JsValue,
        ) -> Result<WasmDocument, JsValue> {
            let payload = decode_document_payload(encoded)?;
            let source_map = source_map_from_js(sources)?;
            Self::from_payload_with_sources(payload, root_specifier, source_map, Vec::new())
        }

        #[wasm_bindgen(js_name = fromEncodedWithBundle)]
        pub fn from_encoded_with_bundle(
            encoded: &str,
            root_specifier: &str,
            sources: JsValue,
            resolutions: JsValue,
            provenance: JsValue,
        ) -> Result<WasmDocument, JsValue> {
            let source_map = source_map_from_js(sources)?;
            let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
            let provenance = served_provenance_from_js(provenance, &source_map)?;
            if let Ok(payload) = BrowserDocumentPayload::decode(encoded) {
                return Self::from_payload_with_sources_and_provenance(
                    payload,
                    root_specifier,
                    source_map,
                    resolutions,
                    provenance,
                    None,
                );
            }
            let bundle = decode_document_bundle(encoded, root_specifier, &source_map, &provenance)?;
            let payload = document_payload_from_bundle(root_specifier, &bundle)?;
            Self::from_payload_with_sources_and_provenance(
                payload,
                root_specifier,
                source_map,
                resolutions,
                provenance,
                Some(bundle),
            )
        }

        pub(super) fn from_payload_with_sources(
            payload: BrowserDocumentPayload,
            root_specifier: &str,
            source_map: HashMap<String, String>,
            resolutions: Vec<SourceResolutionEntry>,
        ) -> Result<WasmDocument, JsValue> {
            Self::from_payload_with_sources_and_provenance(
                payload,
                root_specifier,
                source_map,
                resolutions,
                HashMap::new(),
                None,
            )
        }

        fn from_payload_with_sources_and_provenance(
            payload: BrowserDocumentPayload,
            root_specifier: &str,
            source_map: HashMap<String, String>,
            resolutions: Vec<SourceResolutionEntry>,
            provenance: HashMap<String, ServedSourceProvenance>,
            initial_bundle: Option<CanonicalProgramBundle>,
        ) -> Result<WasmDocument, JsValue> {
            validate_document_payload(&payload, root_specifier, &source_map)?;
            let document = CanonicalWasmDocument::retain(
                "runtime:interactive",
                mech_syntax::document::Revision(0),
                payload.source(),
            )
            .map_err(to_js_error)?;
            let document = retain_browser_root_provenance(
                document,
                provenance.get(root_specifier),
                initial_bundle.as_ref(),
            );
            let document_base = Rc::new(RefCell::new(Staged {
                active: document.document().clone(),
                pending: None,
            }));
            Self::from_bootstrap(WasmDocumentBootstrap {
                root_specifier: root_specifier.to_string(),
                source_map,
                resolutions,
                provenance,
                document,
                document_base,
                presentation_state: Rc::new(RefCell::new(Staged::default())),
                presentation_output_ids: payload.presentation_output_ids().to_vec(),
                initial_bundle,
                console_instance: "repl".to_string(),
                lifecycle: DocumentRuntimeLifecycle::default(),
                #[cfg(feature = "served_project_authority")]
                served: None,
            })
        }

        fn from_bootstrap(bootstrap: WasmDocumentBootstrap) -> Result<WasmDocument, JsValue> {
            Self::try_from_bootstrap(bootstrap).map_err(to_js_error)
        }

        pub(super) fn try_from_bootstrap(
            bootstrap: WasmDocumentBootstrap,
        ) -> MResult<WasmDocument> {
            let mut repl = crate::repl::WasmRepl::from_document(bootstrap.clone())?;
            let state = bootstrap.presentation_state.borrow().active.clone();
            let document_output_ordinals = state.ordinals();
            let reserved_document_output_ids = state
                .bindings
                .iter()
                .map(|binding| binding.output_id)
                .collect();
            let program_output = capture_program_output(&mut repl, &bootstrap, None, None)?;
            Ok(Self {
                repl,
                bootstrap,
                document_output_ordinals,
                document_output_bindings: state.bindings,
                reserved_document_output_ids,
                retired_document_outputs: Vec::new(),
                document_output_source: state.source,
                program_output,
                started: false,
                stopped: false,
            })
        }

        /// Builds a formatted source document with the configured project's
        /// server-projected host authority and complete source resolver.
        #[cfg(feature = "served_project_authority")]
        #[wasm_bindgen(js_name = fromServedEncoded)]
        pub fn from_served_encoded(
            encoded: &str,
            root_specifier: &str,
            config_source: &str,
            sources: JsValue,
        ) -> Result<WasmDocument, JsValue> {
            let payload = decode_document_payload(encoded)?;
            let document = parse_project_config(config_source)?;
            let source_map = source_map_from_js(sources)?;
            let authority = served_browser_authority()?;
            Self::from_served_payload(
                payload,
                root_specifier,
                document,
                config_source,
                source_map,
                Vec::new(),
                HashMap::new(),
                authority,
                None,
            )
        }

        /// Builds a served document with the resolver's authoritative dependency
        /// edges. This keeps browser resolution identical to the native workspace
        /// for extension, index, alias, and other resolver-specific matches.
        #[cfg(feature = "served_project_authority")]
        #[wasm_bindgen(js_name = fromServedEncodedWithBundle)]
        pub fn from_served_encoded_with_bundle(
            encoded: &str,
            root_specifier: &str,
            config_source: &str,
            sources: JsValue,
            resolutions: JsValue,
            provenance: JsValue,
        ) -> Result<WasmDocument, JsValue> {
            let document = parse_project_config(config_source)?;
            let source_map = source_map_from_js(sources)?;
            let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
            let provenance = served_provenance_from_js(provenance, &source_map)?;
            let (payload, initial_bundle) =
                if let Ok(payload) = BrowserDocumentPayload::decode(encoded) {
                    (payload, None)
                } else {
                    let bundle =
                        decode_document_bundle(encoded, root_specifier, &source_map, &provenance)?;
                    (
                        document_payload_from_bundle(root_specifier, &bundle)?,
                        Some(bundle),
                    )
                };
            let authority = served_browser_authority()?;
            Self::from_served_payload(
                payload,
                root_specifier,
                document,
                config_source,
                source_map,
                resolutions,
                provenance,
                authority,
                initial_bundle,
            )
        }

        #[cfg(feature = "served_project_authority")]
        pub(super) fn from_served_payload(
            payload: BrowserDocumentPayload,
            root_specifier: &str,
            document: MechConfigDocument,
            config_source: &str,
            source_map: HashMap<String, String>,
            resolutions: Vec<SourceResolutionEntry>,
            provenance: HashMap<String, ServedSourceProvenance>,
            authority: BrowserRuntimeInjectionConfig,
            initial_bundle: Option<CanonicalProgramBundle>,
        ) -> Result<WasmDocument, JsValue> {
            validate_served_authority(&document, &authority).map_err(to_js_error)?;
            validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
            validate_document_payload(&payload, root_specifier, &source_map)?;
            let retained = CanonicalWasmDocument::retain(
                "runtime:interactive",
                mech_syntax::document::Revision(0),
                payload.source(),
            )
            .map_err(to_js_error)?;
            let retained = retain_browser_root_provenance(
                retained,
                provenance.get(root_specifier),
                initial_bundle.as_ref(),
            );
            let document_base = Rc::new(RefCell::new(Staged {
                active: retained.document().clone(),
                pending: None,
            }));
            Self::from_bootstrap(WasmDocumentBootstrap {
                root_specifier: root_specifier.to_string(),
                source_map,
                resolutions,
                provenance,
                document: retained,
                document_base,
                presentation_state: Rc::new(RefCell::new(Staged::default())),
                presentation_output_ids: payload.presentation_output_ids().to_vec(),
                initial_bundle,
                console_instance: internal_repl_console_instance(&document.hosts),
                lifecycle: DocumentRuntimeLifecycle::default(),
                served: Some(ServedDocumentBootstrap {
                    config_source: config_source.to_string(),
                    authority,
                }),
            })
        }

        #[wasm_bindgen(js_name = runtimeInfo)]
        pub fn runtime_info(&self) -> Result<JsValue, JsValue> {
            runtime_info_value(&self.runtime()?.program_execution_info())
        }

        #[wasm_bindgen(js_name = renderedOutput)]
        pub fn rendered_output(&self, output_id: u64) -> Result<JsValue, JsValue> {
            let max_elements = self.repl.session.value_element_limit();
            let document_output_id = output_id;
            let Some(runtime_output_id) = self.runtime_output_id(document_output_id) else {
                return Ok(JsValue::NULL);
            };
            let runtime = self.runtime()?;
            let Some(snapshot) = runtime
                .output_value(runtime_output_id)
                .map_err(to_js_error)?
            else {
                return Ok(JsValue::NULL);
            };
            let rendered = rendered_named_value(snapshot, None, max_elements)?;
            set_rendered_output_identity(&rendered, document_output_id)?;
            Ok(rendered)
        }

        /// Render the program's implicit display projection. This is the final
        /// ordinary source result, independent of formatted code-fence output.
        /// Explicit output events can replace this default in the host UI.
        #[wasm_bindgen(js_name = renderedProgramOutput)]
        pub fn rendered_program_output(&mut self) -> Result<JsValue, JsValue> {
            let max_elements = self.repl.session.value_element_limit();
            let Some(output) = &self.program_output else {
                return Ok(JsValue::NULL);
            };
            let output_id = output.output_id;
            let selection_token = output.selection_token.clone();
            let Some(snapshot) = self
                .runtime()?
                .output_value(output_id)
                .map_err(to_js_error)?
            else {
                return Ok(JsValue::NULL);
            };
            self.repl
                .session
                .refresh_retained_selection(&selection_token, "ans", snapshot.clone())
                .map_err(to_js_error)?;
            let rendered = rendered_named_value(snapshot, None, max_elements)?;
            set_rendered_selection_identity(&rendered, &selection_token)?;
            Ok(rendered)
        }

        #[wasm_bindgen(js_name = renderedSymbol)]
        pub fn rendered_symbol(&self, name: &str) -> Result<JsValue, JsValue> {
            let max_elements = self.repl.session.value_element_limit();
            let snapshot = self
                .repl
                .session
                .symbol(name)
                .map_err(to_js_error)?
                .ok_or_else(|| js_error(format!("document symbol `{name}` is not resident")))?;
            rendered_value(snapshot, max_elements)
        }

        /// Resolve a formatter placeholder without merging integrity
        /// constraints into the interactive symbol namespace used by :whos.
        #[wasm_bindgen(js_name = renderedDocumentValue)]
        pub fn rendered_document_value(&self, name: &str) -> Result<JsValue, JsValue> {
            let max_elements = self.repl.session.value_element_limit();
            let mut constraints = self
                .repl
                .session
                .integrity_constraints(&[name.to_string()])
                .map_err(to_js_error)?;
            if let Some((_, snapshot)) = constraints.pop() {
                let rendered = rendered_value(snapshot, max_elements)?;
                Reflect::set(
                    &rendered,
                    &JsValue::from_str("interactive"),
                    &JsValue::FALSE,
                )?;
                return Ok(rendered);
            }
            if let Some(snapshot) = self.repl.session.symbol(name).map_err(to_js_error)? {
                let rendered = rendered_value(snapshot, max_elements)?;
                Reflect::set(&rendered, &JsValue::from_str("interactive"), &JsValue::TRUE)?;
                return Ok(rendered);
            }
            Ok(JsValue::NULL)
        }

        #[wasm_bindgen(js_name = reset)]
        pub fn reset(&mut self, encoded: &str) -> Result<(), JsValue> {
            // Construct before touching the live project. A malformed replacement
            // must leave the current document usable.
            let mut replacement_bootstrap = self.bootstrap.clone();
            let payload = decode_document_payload(encoded)?;
            if payload.root_specifier() != replacement_bootstrap.root_specifier {
                return Err(js_error(
                    "replacement document changes the retained root specifier",
                ));
            }
            replacement_bootstrap.source_map.insert(
                replacement_bootstrap.root_specifier.clone(),
                payload.source().to_owned(),
            );
            replacement_bootstrap.document = replacement_bootstrap
                .document
                .replace_source(payload.source())
                .map_err(to_js_error)?;
            replacement_bootstrap.initial_bundle = None;
            replacement_bootstrap.document_base = Rc::new(RefCell::new(Staged {
                active: replacement_bootstrap.document.document().clone(),
                pending: None,
            }));
            replacement_bootstrap.presentation_state = Rc::new(RefCell::new(Staged::default()));
            replacement_bootstrap.presentation_output_ids =
                payload.presentation_output_ids().to_vec();
            let mut replacement = Self::from_bootstrap(replacement_bootstrap)?;
            // Request generations belong to the stable WasmDocument wrapper,
            // not to one replaceable runtime. Carry the clock forward before
            // retirement so callbacks from the old runtime can never match a
            // request created by the replacement.
            replacement
                .repl
                .inherit_host_request_generation(self.repl.host_request_generation());
            replacement
                .repl
                .inherit_step_request_generation(self.repl.step_request_generation());
            let was_started = self.started && !self.stopped;

            // Constructing the candidate is the rollback-safe phase. Once
            // retirement starts, shutdown may already have closed ingress or
            // stopped some drivers before reporting an error, so the old
            // document can no longer be restored. Commit the viable candidate
            // and publish retirement failure on its event stream instead.
            let retirement_failure = self.retire_runtime().err();
            self.repl = replacement.repl;
            self.bootstrap = replacement.bootstrap;
            self.document_output_ordinals = replacement.document_output_ordinals;
            self.document_output_bindings = replacement.document_output_bindings;
            self.reserved_document_output_ids = replacement.reserved_document_output_ids;
            self.retired_document_outputs = replacement.retired_document_outputs;
            self.document_output_source = replacement.document_output_source;
            self.program_output = replacement.program_output;
            self.started = false;
            self.stopped = false;
            if let Some(error) = retirement_failure {
                self.repl.session.emit_message_diagnostic(
                    mech_runtime::Severity::Warning,
                    mech_runtime::DiagnosticPhase::Host,
                    "PreviousDocumentShutdown",
                    format!(
                        "The replacement document was accepted, but retired document cleanup reported: {}",
                        error.display_message(),
                    ),
                );
            }
            if was_started {
                self.start()?;
            }
            Ok(())
        }

        #[wasm_bindgen(js_name = step)]
        pub fn step(&mut self, count: u64) -> Result<(), JsValue> {
            self.repl.step_immediate(count).map_err(to_js_error)
        }

        #[wasm_bindgen(js_name = renderedSymbols)]
        pub fn rendered_symbols(&self, names: JsValue) -> Result<JsValue, JsValue> {
            let max_elements = self.repl.session.value_element_limit();
            let names = rendered_symbol_names_from_js(names)?;
            let values = match names {
                Some(names) => self.repl.session.symbols(&names).map_err(to_js_error)?,
                None => self.repl.session.symbols(&[]).map_err(to_js_error)?,
            };
            let rows = Array::new();
            for (name, value) in values {
                rows.push(&rendered_symbol_row(&name, value, max_elements)?);
            }
            Ok(rows.into())
        }

        pub fn start(&mut self) -> Result<(), JsValue> {
            self.repl
                .session
                .start_input_drivers()
                .map_err(to_js_error)?;
            self.bootstrap.source().lifecycle.set_drivers_started(true);
            self.started = true;
            self.stopped = false;
            Ok(())
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = computeManifest)]
        pub fn compute_manifest(&self) -> JsValue {
            self.bootstrap
                .source()
                .lifecycle
                .compute()
                .map(|compute| compute.manifest())
                .unwrap_or(JsValue::NULL)
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = computeBackend)]
        pub fn compute_backend(&self) -> String {
            self.bootstrap
                .source()
                .lifecycle
                .compute()
                .map(|compute| compute.backend())
                .unwrap_or_default()
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = computeGeneration)]
        pub fn compute_generation(&self) -> String {
            self.bootstrap
                .source()
                .lifecycle
                .compute_generation()
                .to_string()
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = isComputeCommandTokenCurrent)]
        pub fn is_compute_command_token_current(&self, dispatch_token: &str) -> bool {
            self.bootstrap
                .source()
                .lifecycle
                .compute()
                .is_some_and(|compute| compute.validate_token(dispatch_token).is_ok())
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = reportComputeStateReset)]
        pub fn report_compute_state_reset(
            &mut self,
            previous_revision: &str,
            next_revision: &str,
        ) -> Result<JsValue, JsValue> {
            self.repl
                .report_compute_state_reset(previous_revision, next_revision)
        }

        /// Rebuild the accepted source generation after the browser has
        /// withdrawn WebGPU availability. Compatible resident state is
        /// migrated through the ordinary runtime handoff, so auto fallback
        /// never reconstructs an obsolete initial document.
        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = fallbackComputeToCpu)]
        pub fn fallback_compute_to_cpu(&mut self) -> Result<(), JsValue> {
            if self.compute_backend() != mech_compute::WGPU_BACKEND {
                return Ok(());
            }
            self.repl
                .session
                .rebuild_runtime_preserving_state()
                .map_err(to_js_error)?;
            self.refresh_document_output_ordinals(None)
                .map_err(to_js_error)?;
            if self.compute_backend() == mech_compute::WGPU_BACKEND {
                return Err(js_error(
                    "browser compute fallback rebuilt the accepted generation with WebGPU still selected",
                ));
            }
            Ok(())
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = completeComputeCommand)]
        pub fn complete_compute_command(&self, completion: JsValue) -> Result<(), JsValue> {
            let bridge = self
                .bootstrap
                .source()
                .lifecycle
                .compute()
                .ok_or_else(|| js_error("document has no compute region"))?;
            bridge.complete_command(&completion)
        }

        #[cfg(feature = "browser_host_scene")]
        #[wasm_bindgen(js_name = scenePointerInput)]
        pub fn scene_pointer_input(
            &self,
            instance: &str,
            x: f64,
            y: f64,
            pressed: bool,
            delta_seconds: f64,
        ) -> Result<(), JsValue> {
            self.bootstrap
                .source()
                .lifecycle
                .scenes()
                .submit_pointer(instance, x, y, pressed, delta_seconds)
                .map_err(to_js_error)
        }

        #[wasm_bindgen(js_name = hasPointerInput)]
        pub fn has_pointer_input(&self) -> bool {
            #[cfg(feature = "browser_compute")]
            {
                self.bootstrap
                    .source()
                    .lifecycle
                    .pointer()
                    .is_some_and(|pointer| pointer.is_running())
            }
            #[cfg(not(feature = "browser_compute"))]
            {
                false
            }
        }

        #[cfg(feature = "browser_compute")]
        #[wasm_bindgen(js_name = pointerInput)]
        pub fn pointer_input(
            &self,
            x: f64,
            y: f64,
            pressed: bool,
            delta_seconds: f64,
        ) -> Result<(), JsValue> {
            self.bootstrap
                .source()
                .lifecycle
                .pointer()
                .ok_or_else(|| js_error("document has no pointer host"))?
                .submit(x, y, pressed, delta_seconds)
                .map_err(to_js_error)
        }

        pub fn frame(&mut self, max_inputs: usize) -> Result<JsValue, JsValue> {
            if max_inputs == 0 {
                return Err(js_error("max_inputs must be greater than zero"));
            }
            let pending_before = self
                .runtime()?
                .pending_host_input_count()
                .map_err(to_js_error)?;
            let processed = self
                .repl
                .session
                .drain_pending_inputs(pending_before.min(max_inputs))
                .map_err(to_js_error)?;
            let pending = self
                .runtime()?
                .pending_host_input_count()
                .map_err(to_js_error)?;
            #[cfg(feature = "browser_host_scene")]
            let rendered = self
                .bootstrap
                .source()
                .lifecycle
                .scenes()
                .render_frame()
                .map_err(to_js_error)?;
            #[cfg(not(feature = "browser_host_scene"))]
            let rendered = 0;
            #[cfg(feature = "browser_host_scene")]
            for output in self
                .bootstrap
                .source()
                .lifecycle
                .scenes()
                .drain_output_events()
                .map_err(to_js_error)?
            {
                self.repl.session.emit(MechEvent::Output(output));
            }

            let info = self.runtime()?.program_execution_info();
            let out = Object::new();
            for (name, value) in [
                ("processed", processed as f64),
                ("pending", pending as f64),
                ("rendered", rendered as f64),
                (
                    "residentTurns",
                    info.resident_accepted_turns
                        .saturating_add(info.resident_rejected_turns) as f64,
                ),
                ("accepted", info.resident_accepted_turns as f64),
                ("rejected", info.resident_rejected_turns as f64),
                ("coalesced", info.coalesced_host_packets as f64),
            ] {
                Reflect::set(&out, &JsValue::from_str(name), &JsValue::from_f64(value))?;
            }
            Reflect::set(
                &out,
                &JsValue::from_str("route"),
                &JsValue::from_str(runtime_route_name(info.route)),
            )?;
            Reflect::set(
                &out,
                &JsValue::from_str("events"),
                &serde_wasm_bindgen::to_value(
                    &self.repl.session.drain_events().map_err(to_js_error)?,
                )?,
            )?;
            #[cfg(feature = "browser_compute")]
            Reflect::set(
                &out,
                &JsValue::from_str("computeCommand"),
                &match self.bootstrap.source().lifecycle.compute() {
                    Some(compute) => compute.take_command()?,
                    None => JsValue::NULL,
                },
            )?;
            Ok(out.into())
        }

        pub fn stop(&mut self) -> Result<(), JsValue> {
            if self.stopped {
                return Ok(());
            }
            self.retire_runtime().map_err(to_js_error)
        }

        fn retire_runtime(&mut self) -> MResult<()> {
            self.bootstrap.source().lifecycle.set_drivers_started(false);
            self.started = false;
            self.stopped = true;
            self.repl.terminate_session()
        }

        #[wasm_bindgen(js_name = replInvoke)]
        pub fn repl_invoke(&mut self, source: &str) -> Result<JsValue, JsValue> {
            let accepted_before = self.repl.session.source().to_string();
            self.bootstrap
                .stage_repl_document_boundary(source)
                .map_err(to_js_error)?;
            let prepared_token = self.prepare_program_selection().map_err(|error| {
                self.bootstrap.abort();
                to_js_error(error)
            })?;
            // Reset is an existing browser resident operation; keep its alias
            // at the document boundary instead of inventing portable commands.
            let response = if source.trim() == ":reset" {
                self.repl.reset()
            } else {
                self.repl.invoke(source)
            };
            let response = match response {
                Ok(response) => response,
                Err(error) => {
                    self.bootstrap.abort();
                    self.discard_program_selection(prepared_token.as_deref());
                    return Err(error);
                }
            };
            if self.repl.session.source() != accepted_before {
                let edit = self
                    .repl
                    .session
                    .source()
                    .starts_with(&accepted_before)
                    .then(|| PendingSourceEdit {
                        old_start: accepted_before.len(),
                        old_end: accepted_before.len(),
                        new_start: accepted_before.len(),
                        new_end: self.repl.session.source().len(),
                    });
                self.refresh_document_output_ordinals(edit)
                    .map_err(to_js_error)?;
                self.program_output = capture_program_output(
                    &mut self.repl,
                    &self.bootstrap,
                    self.program_output.as_ref(),
                    prepared_token.as_deref(),
                )
                .map_err(to_js_error)?;
            } else {
                self.bootstrap.abort();
                self.discard_program_selection(prepared_token.as_deref());
            }
            Ok(response)
        }

        /// Return the complete source currently accepted by the resident
        /// document session. This is the read side of the editor handoff; REPL
        /// submissions continue to use `replInvoke`.
        #[wasm_bindgen(js_name = replSource)]
        pub fn repl_source(&self) -> String {
            self.repl.source()
        }

        /// Transactionally replace the complete accepted document source.
        /// Compatible hosts migrate resident state; incompatible hosts start
        /// from their new plan initializers and report that reset explicitly.
        #[wasm_bindgen(js_name = replReplaceSource)]
        pub fn repl_replace_source(&mut self, source: &str) -> Result<JsValue, JsValue> {
            self.replace_source_with_edit(source, None)
        }

        /// Apply one editor change using UTF-16 offsets in the accepted source.
        /// This preserves the insertion side of otherwise identical outputs.
        #[wasm_bindgen(js_name = replApplyEdit)]
        pub fn repl_apply_edit(
            &mut self,
            start: u32,
            end: u32,
            inserted: &str,
        ) -> Result<JsValue, JsValue> {
            let accepted_before = self.repl.session.source().to_string();
            let start = utf16_to_byte_offset(&accepted_before, start)
                .ok_or_else(|| js_error("edit start is not a UTF-16 character boundary"))?;
            let end = utf16_to_byte_offset(&accepted_before, end)
                .ok_or_else(|| js_error("edit end is not a UTF-16 character boundary"))?;
            if end < start {
                return Err(js_error("edit end precedes edit start"));
            }
            let mut source = accepted_before;
            source.replace_range(start..end, inserted);
            self.replace_source_with_edit(&source, Some((start, end, inserted.len())))
        }

        fn replace_source_with_edit(
            &mut self,
            source: &str,
            edit: Option<(usize, usize, usize)>,
        ) -> Result<JsValue, JsValue> {
            let accepted_before = self.repl.session.source().to_string();
            let pending_edit = edit
                .map(|(start, end, inserted_len)| {
                    let new_end = start
                        .checked_add(inserted_len)
                        .ok_or_else(|| js_error("edited source is too large"))?;
                    Ok::<_, JsValue>(PendingSourceEdit {
                        old_start: start,
                        old_end: end,
                        new_start: start,
                        new_end,
                    })
                })
                .transpose()?;
            let revision = self
                .repl
                .session
                .source_document()
                .map(|document| document.source().revision().0)
                .unwrap_or(0)
                .saturating_add(1);
            let candidate = SourceDocument::parse_resolved(
                "runtime:interactive",
                mech_syntax::document::Revision(revision),
                source,
                mech_syntax::document::ParseConfig::default(),
            )
            .map_err(|error| {
                to_js_error(document_runtime_error(format!(
                    "invalid browser source: {error:?}"
                )))
            })?;
            let prepared_token = self.prepare_program_selection().map_err(to_js_error)?;
            self.bootstrap.stage_document_base(candidate);
            let response = match self.repl.replace_source(source) {
                Ok(response) => response,
                Err(error) => {
                    self.bootstrap.abort();
                    self.discard_program_selection(prepared_token.as_deref());
                    return Err(error);
                }
            };
            if self.repl.session.source() != accepted_before {
                self.program_output = capture_program_output(
                    &mut self.repl,
                    &self.bootstrap,
                    self.program_output.as_ref(),
                    prepared_token.as_deref(),
                )
                .map_err(to_js_error)?;
                let accepted_edit = if self.repl.session.source() == source {
                    pending_edit
                } else {
                    None
                };
                self.refresh_document_output_ordinals(accepted_edit)
                    .map_err(to_js_error)?;
            } else {
                self.bootstrap.abort();
                self.discard_program_selection(prepared_token.as_deref());
            }
            Ok(response)
        }

        #[wasm_bindgen(js_name = replPublishProgramEvent)]
        pub fn repl_publish_program_event(&self, event: JsValue) -> Result<(), JsValue> {
            self.repl.publish_program_event(event)
        }

        #[wasm_bindgen(js_name = replContinueStep)]
        pub fn repl_continue_step(
            &mut self,
            max_steps: u32,
            request_id: &str,
        ) -> Result<JsValue, JsValue> {
            self.repl.continue_step(max_steps, request_id)
        }

        #[wasm_bindgen(js_name = replInterrupt)]
        pub fn repl_interrupt(&mut self) -> Result<JsValue, JsValue> {
            self.repl.interrupt()
        }

        #[wasm_bindgen(js_name = replSetQuiet)]
        pub fn repl_set_quiet(&mut self, quiet: bool) -> Result<JsValue, JsValue> {
            self.repl.set_quiet(quiet)
        }

        #[wasm_bindgen(js_name = replSetValueElementLimit)]
        pub fn repl_set_value_element_limit(
            &mut self,
            max_elements: usize,
        ) -> Result<JsValue, JsValue> {
            self.repl.set_value_element_limit(max_elements)
        }

        /// Return canonical syntax-highlighted markup only when a console
        /// entry is a complete, recoverable Mech source fragment.
        #[wasm_bindgen(js_name = replFormatSource)]
        pub fn repl_format_source(&self, source: &str) -> Option<String> {
            CanonicalWasmDocument::repl_format_source(source)
        }

        #[wasm_bindgen(js_name = replFinishHostRequest)]
        pub fn repl_finish_host_request(&mut self, request_id: &str) -> Result<JsValue, JsValue> {
            self.repl.finish_host_request(request_id)
        }

        #[wasm_bindgen(js_name = replDocumentationIndex)]
        pub fn repl_documentation_index(&mut self, request_id: &str) -> Result<JsValue, JsValue> {
            self.repl.finish_documentation_index(request_id)
        }

        #[wasm_bindgen(js_name = replSelectSymbol)]
        pub fn repl_select_symbol(
            &mut self,
            name: &str,
            render_popup: bool,
        ) -> Result<JsValue, JsValue> {
            if let Some(response) = self.repl.begin_selection()? {
                return selected_value_response(response, None, None, name, None);
            }
            let pending_identity = self
                .repl
                .session
                .symbol_selection_identity(name)
                .map(str::to_string);
            let reuse_identity = self
                .repl
                .session
                .symbol_output_id(name)
                .map(|output| format!("resident-output:{}", output.get()));
            let snapshot = self
                .repl
                .session
                .symbol(name)
                .map_err(to_js_error)?
                .ok_or_else(|| js_error(format!("document symbol `{name}` is not resident")))?;
            let block_html = (render_popup && !self.repl.session.is_quiet())
                .then(|| snapshot.format_repl_html(self.repl.session.value_element_limit()));
            let identity = match pending_identity {
                Some(identity) => identity,
                None => self
                    .repl
                    .session
                    .retain_selection(name, snapshot.clone(), reuse_identity.as_deref())
                    .map_err(to_js_error)?,
            };
            let (response, presentation) =
                self.repl
                    .publish_selection(name, snapshot, Some(identity.clone()))?;
            selected_value_response(response, presentation, block_html, name, Some(&identity))
        }

        #[wasm_bindgen(js_name = replSelectOutput)]
        pub fn repl_select_output(
            &mut self,
            output_id: u64,
            render_popup: bool,
        ) -> Result<JsValue, JsValue> {
            if let Some(response) = self.repl.begin_selection()? {
                return selected_value_response(response, None, None, "ans", None);
            }
            let Some(runtime_output_id) = self.runtime_output_id(output_id) else {
                return Err(js_error("document output is not resident"));
            };
            let (snapshot, source_echo) = {
                let runtime = self.runtime()?;
                let snapshot = runtime
                    .output_value(runtime_output_id)
                    .map_err(to_js_error)?
                    .ok_or_else(|| js_error("document output is not resident"))?;
                let source_echo = runtime
                    .output_name(runtime_output_id)
                    .unwrap_or_else(|| "ans".to_string());
                (snapshot, source_echo)
            };
            let block_html = (render_popup && !self.repl.session.is_quiet())
                .then(|| snapshot.format_repl_html(self.repl.session.value_element_limit()));
            let reuse_identity = format!("resident-output:{}", runtime_output_id.get());
            let identity = self
                .repl
                .session
                .retain_selection(&source_echo, snapshot.clone(), Some(&reuse_identity))
                .map_err(to_js_error)?;
            let (response, presentation) =
                self.repl
                    .publish_selection(&source_echo, snapshot, Some(identity.clone()))?;
            selected_value_response(
                response,
                presentation,
                block_html,
                &source_echo,
                Some(&identity),
            )
        }

        #[wasm_bindgen(js_name = replSelectRetained)]
        pub fn repl_select_retained(
            &mut self,
            selection_token: &str,
            render_popup: bool,
        ) -> Result<JsValue, JsValue> {
            if let Some(response) = self.repl.begin_selection()? {
                return selected_value_response(response, None, None, "ans", None);
            }
            let (source_echo, snapshot) = self
                .repl
                .session
                .retained_selection(selection_token)
                .ok_or_else(|| js_error("document selection is no longer retained"))?;
            let block_html = (render_popup && !self.repl.session.is_quiet())
                .then(|| snapshot.format_repl_html(self.repl.session.value_element_limit()));
            let (response, presentation) = self.repl.publish_selection(
                &source_echo,
                snapshot,
                Some(selection_token.to_string()),
            )?;
            selected_value_response(
                response,
                presentation,
                block_html,
                &source_echo,
                Some(selection_token),
            )
        }

        #[wasm_bindgen(js_name = replLoadDocumentation)]
        pub fn repl_load_documentation(
            &mut self,
            request_id: &str,
            topic: &str,
            source: &str,
        ) -> Result<JsValue, JsValue> {
            if !self.repl.host_request_pending(request_id) {
                return Err(js_error(
                    "documentation response does not match the active REPL host request",
                ));
            }
            let document = CanonicalWasmDocument::retain(
                &format!("browser:documentation:{topic}"),
                mech_syntax::document::Revision(0),
                source,
            )
            .map_err(to_js_error)?;
            document.document().index().map_err(|error| {
                to_js_error(document_runtime_error(format!(
                    "invalid documentation source: {error}"
                )))
            })?;
            mech_runtime::CanonicalDocumentRenderer
                .format_html_body(&document.document().document())
                .map_err(|error| js_error(error.to_string()))?;
            let accepted_source = self.repl.session.source().to_string();
            let accepted_before = accepted_source.len();
            let accepted = match self.repl.session.submit_host_source(source) {
                Ok(_) => {
                    let edit = self
                        .repl
                        .session
                        .source()
                        .starts_with(&accepted_source)
                        .then(|| PendingSourceEdit {
                            old_start: accepted_before,
                            old_end: accepted_before,
                            new_start: accepted_before,
                            new_end: self.repl.session.source().len(),
                        });
                    self.refresh_document_output_ordinals(edit)
                        .map_err(to_js_error)?;
                    true
                }
                Err(error) => {
                    self.repl.session.emit_error(
                        &error,
                        mech_runtime::DiagnosticPhase::Compile,
                        Some(topic),
                    );
                    false
                }
            };
            let html = if accepted {
                let current = self.repl.session.source_document().ok_or_else(|| {
                    js_error("documentation source was accepted without a retained document")
                })?;
                let retained_source = current.source().to_contiguous_string();
                let (accepted_fragment, fragment_start) =
                    retained_submission_fragment(&retained_source, accepted_before, source)
                        .map_err(to_js_error)?;
                let accepted_document = CanonicalWasmDocument::retain(
                    &format!("browser:documentation:{topic}:accepted"),
                    mech_syntax::document::Revision(0),
                    accepted_fragment.to_owned(),
                )
                .map_err(to_js_error)?;
                let addresses = retained_document_fragment_addresses(
                    &self.document_output_bindings,
                    fragment_start,
                    accepted_fragment.len(),
                )
                .map_err(to_js_error)?;
                Some(
                    mech_runtime::CanonicalDocumentRenderer
                        .format_html_body_live(&accepted_document.document().document(), &addresses)
                        .map_err(|error| js_error(error.to_string()))?,
                )
            } else {
                None
            };
            let result = Object::new();
            Reflect::set(
                &result,
                &JsValue::from_str("topic"),
                &JsValue::from_str(topic),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("html"),
                &html.map_or(JsValue::NULL, |html| JsValue::from_str(&html)),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("accepted"),
                &JsValue::from_bool(accepted),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("response"),
                &self.repl.response(None)?,
            )?;
            Ok(result.into())
        }
    }

    impl WasmDocument {
        fn prepare_program_selection(&mut self) -> MResult<Option<String>> {
            if self.program_output.is_some() {
                return Ok(None);
            }
            self.repl
                .session
                .retain_selection("ans", mech_runtime::RuntimeValueSnapshot::empty(), None)
                .map(Some)
        }

        fn discard_program_selection(&mut self, token: Option<&str>) {
            if let Some(token) = token {
                self.repl.session.release_retained_selection(token);
            }
        }

        pub(super) fn runtime(&self) -> Result<&MechRuntime, JsValue> {
            self.repl
                .session
                .runtime()
                .ok_or_else(|| js_error("document runtime is not active"))
        }

        pub(super) fn refresh_document_output_ordinals(
            &mut self,
            edit: Option<PendingSourceEdit>,
        ) -> MResult<()> {
            let mut state = self.bootstrap.presentation_state.borrow().active.clone();
            let current_source = state.source.clone();
            let edit = edit.map(|edit| SourceEditAnchors {
                old_start: edit.old_start,
                old_end: edit.old_end,
                new_start: edit.new_start,
                new_end: edit.new_end,
            });
            retain_output_identities(
                &self.document_output_bindings,
                &mut state.bindings,
                &self.document_output_source,
                &current_source,
                edit,
                &mut self.reserved_document_output_ids,
                &mut self.retired_document_outputs,
            );
            let ordinals = state.ordinals();
            let output_id = ordinals
                .get(&root_document_program_output_id())
                .and_then(|ordinal| u32::try_from(*ordinal).ok())
                .map(OutputId::new);
            self.document_output_ordinals = ordinals;
            self.document_output_bindings = state.bindings;
            self.document_output_source = current_source;
            if let (Some(program_output), Some(output_id)) =
                (self.program_output.as_mut(), output_id)
            {
                program_output.output_id = output_id;
            }
            Ok(())
        }

        #[cfg(test)]
        pub(super) fn document_output_ordinal(&self, output_id: u64) -> Option<u64> {
            self.document_output_ordinals.get(&output_id).copied()
        }

        fn runtime_output_id(&self, output_id: u64) -> Option<OutputId> {
            let output_id = self.document_output_ordinals.get(&output_id).copied()?;
            u32::try_from(output_id).ok().map(OutputId::new)
        }
    }
}

pub use document::WasmDocument;

#[cfg(feature = "served_project_authority")]
fn bundle_roots_from_js(value: JsValue) -> Result<Vec<String>, JsValue> {
    if !Array::is_array(&value) {
        return Err(js_error("bundle roots must be an array"));
    }
    let roots = Array::from(&value);
    if roots.length() == 0 {
        return Err(js_error("bundle roots must not be empty"));
    }
    roots
        .iter()
        .map(|root| {
            let root = root
                .as_string()
                .ok_or_else(|| js_error("bundle roots must contain only strings"))?;
            validate_bundle_root(&root)?;
            Ok(root)
        })
        .collect()
}

#[cfg(feature = "served_project_authority")]
fn validate_bundle_root(root: &str) -> Result<(), JsValue> {
    if root.is_empty()
        || Path::new(root).is_absolute()
        || root.contains('\\')
        || root.contains(':')
        || root
            .split('/')
            .any(|component| component.is_empty() || matches!(component, "." | ".."))
    {
        return Err(js_error(format!("invalid bundle root `{root}`")));
    }
    Ok(())
}

#[cfg(feature = "served_project_authority")]
fn replace_bundle_run_paths(
    document: &mut MechConfigDocument,
    roots: Vec<String>,
) -> Result<(), JsValue> {
    let run = document
        .run
        .as_mut()
        .ok_or_else(|| js_error("project config must contain run settings"))?;
    run.paths = roots.into_iter().map(Into::into).collect();
    Ok(())
}

fn parse_project_config(source: &str) -> Result<MechConfigDocument, JsValue> {
    parse_config_document(
        "browser-project/mech.mcfg",
        source,
        ConfigProfileOptions::default(),
    )
    .map_err(to_js_error)
}

fn decode_document_payload(encoded: &str) -> Result<BrowserDocumentPayload, JsValue> {
    BrowserDocumentPayload::decode(encoded).map_err(to_js_error)
}

fn retain_browser_root_provenance(
    document: CanonicalWasmDocument,
    provenance: Option<&ServedSourceProvenance>,
    bundle: Option<&CanonicalProgramBundle>,
) -> CanonicalWasmDocument {
    let retained = document.document().clone();
    let retained = if let Some(provenance) = provenance {
        retained.with_nominal_provenance(
            provenance.nominal_origin.clone(),
            provenance.nominal_package_id.clone(),
        )
    } else if let Some(bundle) = bundle.filter(|bundle| bundle.root_nominal_origin.is_some()) {
        retained.with_nominal_provenance(
            bundle.root_nominal_origin.clone().unwrap(),
            bundle.root_nominal_package_id.clone(),
        )
    } else {
        retained
    };
    CanonicalWasmDocument::from_document(retained)
}

fn decode_document_bundle(
    encoded: &str,
    root_specifier: &str,
    source_map: &HashMap<String, String>,
    provenance: &HashMap<String, ServedSourceProvenance>,
) -> Result<CanonicalProgramBundle, JsValue> {
    let source = source_map.get(root_specifier).ok_or_else(|| {
        js_error(format!(
            "document root `{root_specifier}` is missing from the source map"
        ))
    })?;
    let root_provenance = provenance.get(root_specifier);
    let bundle = match root_provenance {
        Some(root_provenance) => CanonicalProgramBundle::decode_with_root_provenance(
            encoded,
            Some(source),
            Some(&root_provenance.nominal_origin),
            root_provenance.nominal_package_id.as_deref(),
        ),
        None => CanonicalProgramBundle::decode_standalone(encoded, Some(source)),
    }
    .map_err(to_js_error)?;
    bundle
        .validate_dependency_sources_with_provenance(|uri| {
            let specifier = uri.strip_prefix("bundle:///")?;
            let source = source_map.get(specifier)?.as_str();
            let retained = provenance.get(specifier);
            Some(CanonicalDependencySource {
                source,
                nominal_origin: retained.map(|item| &item.nominal_origin),
                nominal_package_id: retained.and_then(|item| item.nominal_package_id.as_deref()),
            })
        })
        .map_err(to_js_error)?;
    if bundle.canonical_uri != format!("bundle:///{root_specifier}") {
        return Err(js_error(
            "canonical bundle root identity does not match the requested document root",
        ));
    }
    Ok(bundle)
}

fn document_payload_from_bundle(
    root_specifier: &str,
    bundle: &CanonicalProgramBundle,
) -> Result<BrowserDocumentPayload, JsValue> {
    let document = SourceDocument::parse_resolved(
        &bundle.canonical_uri,
        mech_syntax::document::Revision(bundle.source_revision),
        bundle.source.as_str(),
        mech_syntax::document::ParseConfig::default(),
    )
    .map_err(|error| js_error(format!("invalid canonical bundle source: {error:?}")))?;
    let presentation_output_ids =
        mech_runtime::canonical_document_presentation_output_ids(&document.document())
            .map_err(|error| js_error(error.to_string()))?;
    BrowserDocumentPayload::new(root_specifier, &bundle.source)
        .map(|payload| payload.with_presentation_output_ids(presentation_output_ids))
        .map_err(to_js_error)
}

fn validate_document_payload(
    payload: &BrowserDocumentPayload,
    root_specifier: &str,
    source_map: &HashMap<String, String>,
) -> Result<(), JsValue> {
    if payload.root_specifier() != root_specifier {
        return Err(js_error(format!(
            "browser document payload root `{}` does not match requested root `{root_specifier}`",
            payload.root_specifier(),
        )));
    }
    let source = source_map.get(root_specifier).ok_or_else(|| {
        js_error(format!(
            "document root `{root_specifier}` is missing from the source map"
        ))
    })?;
    if source != payload.source() {
        return Err(js_error(format!(
            "browser document payload is stale for source-map root `{root_specifier}`"
        )));
    }
    Ok(())
}

fn required_path_strings(source: &str) -> mech_core::MResult<Vec<String>> {
    let document = parse_config_document(
        "browser-project/mech.mcfg",
        source,
        ConfigProfileOptions::default(),
    )?;
    let run = require_run(&document)?;
    let mut paths = run
        .paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    if let Some(serve) = &document.serve {
        for path in &serve.paths {
            if SourceKind::from_path(path) != SourceKind::Mech {
                continue;
            }

            let path = path.to_string_lossy().to_string();
            if !paths.contains(&path) {
                paths.push(path);
            }
        }
    }
    Ok(paths)
}
pub(super) fn browser_runtime_builder() -> RuntimeBuilder {
    RuntimeBuilder::new().function_catalog(mech_stdlib::source_catalog())
}

fn runtime_builder_with_factories(
    repl_events: Option<MechEventBuffer>,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
    #[cfg(feature = "browser_compute")] pointer: Option<PointerInputHandle>,
) -> Result<RuntimeBuilder, JsValue> {
    let mut builder = browser_runtime_builder();
    #[cfg(feature = "browser_host_dom")]
    {
        builder = builder
            .host_factory(Box::new(
                BrowserHostFactory::new(WasmBrowserDomBackend::new()).map_err(to_js_error)?,
            ))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_compute")]
    {
        let pointer_factory =
            pointer.map_or_else(PointerHostFactory::planning, PointerHostFactory::new);
        builder = builder
            .host_factory(Box::new(pointer_factory))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_time")]
    {
        builder = builder
            .host_factory(Box::new(
                BrowserTimeHostFactory::new().map_err(to_js_error)?,
            ))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_timer")]
    {
        builder = builder
            .host_factory(Box::new(
                BrowserTimerHostFactory::new().map_err(to_js_error)?,
            ))
            .map_err(to_js_error)?;
    }
    #[cfg(feature = "browser_host_console")]
    {
        builder = if let Some(events) = repl_events {
            builder
                .host_factory(Box::new(
                    ConsoleHostFactory::with_backend(crate::repl::ReplConsoleBackend::new(events))
                        .map_err(to_js_error)?,
                ))
                .map_err(to_js_error)?
        } else {
            builder
                .host_factory(Box::new(
                    BrowserConsoleHostFactory::new().map_err(to_js_error)?,
                ))
                .map_err(to_js_error)?
        };
    }
    #[cfg(feature = "browser_host_scene")]
    {
        let scene_factory = BrowserSceneHostFactory::with_registry(scenes).map_err(to_js_error)?;
        builder = builder
            .host_factory(Box::new(scene_factory))
            .map_err(to_js_error)?;
    }
    Ok(builder)
}

#[cfg(feature = "served_project_authority")]
fn build_runtime_from_authority(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
    source_resolver: InMemorySourceResolver,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
    #[cfg(feature = "browser_compute")] pointer: Option<PointerInputHandle>,
) -> Result<MechRuntime, JsValue> {
    project_runtime_builder(
        document,
        source_resolver,
        Some(authority),
        false,
        #[cfg(feature = "browser_compute")]
        None,
        #[cfg(feature = "browser_host_scene")]
        scenes,
        #[cfg(feature = "browser_compute")]
        pointer,
    )?
    .build()
    .map_err(to_js_error)
}

/// Host settings, grants, and runtime limits stay under the same authority for
/// planning and loading. Only planning omits the not-yet-prepared compute host.
fn project_runtime_builder(
    document: &MechConfigDocument,
    source_resolver: InMemorySourceResolver,
    #[cfg(feature = "served_project_authority")] authority: Option<&BrowserRuntimeInjectionConfig>,
    omit_compute: bool,
    #[cfg(feature = "browser_compute")] compute_factory: Option<mech_gpu::ComputeHostFactory>,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
    #[cfg(feature = "browser_compute")] pointer: Option<PointerInputHandle>,
) -> Result<RuntimeBuilder, JsValue> {
    #[cfg(not(feature = "served_project_authority"))]
    let runtime_config = mech_runtime::RuntimeConfig::default()
        .apply_patch(&document.runtime)
        .map_err(to_js_error)?;
    #[cfg(feature = "served_project_authority")]
    let runtime_config = match authority {
        Some(authority) => authority.into_runtime_config().map_err(to_js_error)?,
        None => mech_runtime::RuntimeConfig::default()
            .apply_patch(&document.runtime)
            .map_err(to_js_error)?,
    };
    let mut builder = runtime_builder_with_factories(
        None,
        #[cfg(feature = "browser_host_scene")]
        scenes,
        #[cfg(feature = "browser_compute")]
        pointer,
    )?
    .config(runtime_config)
    .source_resolver(source_resolver);
    #[cfg(feature = "browser_compute")]
    if let Some(factory) = compute_factory {
        builder = builder
            .host_factory(Box::new(factory))
            .map_err(to_js_error)?;
    }
    for required in document
        .hosts
        .iter()
        .filter(|host| !omit_compute || host.provider != "compute")
    {
        #[cfg(feature = "served_project_authority")]
        if let Some(authority) = authority {
            if let Some(host) = authority
                .hosts
                .iter()
                .find(|host| host.name == required.name && host.provider == required.provider)
            {
                builder = builder.host_instance(host.clone());
            }
            continue;
        }
        builder = builder.host_instance(required.clone());
    }
    #[cfg(feature = "served_project_authority")]
    if let Some(authority) = authority {
        for grant in required_issued_grants(document, authority) {
            builder = builder.run_resource_grant(grant);
        }
        return Ok(builder);
    }
    if let Some(run) = &document.run {
        for grant in &run.grants {
            builder = builder.run_resource_grant(grant.clone());
        }
    }
    Ok(builder)
}

struct ProjectRuntimeCandidate {
    runtime: MechRuntime,
    #[cfg(feature = "browser_compute")]
    compute: Option<BrowserComputeBridge>,
}

#[cfg(feature = "browser_compute")]
fn prepare_project_compute_region(
    document: &MechConfigDocument,
    resolver: InMemorySourceResolver,
    #[cfg(feature = "served_project_authority")] authority: Option<&BrowserRuntimeInjectionConfig>,
) -> MResult<Option<PreparedComputeRegion>> {
    if !document.hosts.iter().any(|host| host.provider == "compute") {
        return Ok(None);
    }
    let run = require_run(document)?;
    let [root] = run.paths.as_slice() else {
        return Err(document_runtime_error(
            "static browser compute projects require exactly one root",
        ));
    };
    let mut compiler = project_runtime_builder(
        document,
        resolver,
        #[cfg(feature = "served_project_authority")]
        authority,
        true,
        None,
        #[cfg(feature = "browser_host_scene")]
        BrowserSceneRegistry::new(),
        None,
    )
    .map_err(js_value_to_mech_error)?
    .function_catalog(mech_stdlib::source_native_plan_catalog())
    .build_compiler()?;
    prepare_compute_root_region(
        &mut compiler,
        SourceRequest::new(root.to_string_lossy()),
        browser_module_options(),
    )
    .map(Some)
}

fn build_project_source_runtime(
    document: &MechConfigDocument,
    resolver: InMemorySourceResolver,
    #[cfg(feature = "served_project_authority")] authority: Option<&BrowserRuntimeInjectionConfig>,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
    #[cfg(feature = "browser_compute")] pointer: Option<PointerInputHandle>,
) -> Result<ProjectRuntimeCandidate, JsValue> {
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    let issued_document = authority
        .map(|authority| compute_document_with_issued_settings(document, authority))
        .transpose()
        .map_err(to_js_error)?;
    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    let compute_document = issued_document.as_ref().unwrap_or(document);
    #[cfg(all(feature = "browser_compute", not(feature = "served_project_authority")))]
    let compute_document = document;
    #[cfg(feature = "browser_compute")]
    let prepared = prepare_project_compute_region(
        compute_document,
        resolver.clone(),
        #[cfg(feature = "served_project_authority")]
        authority,
    )
    .map_err(to_js_error)?
    .map(|prepared| {
        prepare_browser_compute_runtime(
            compute_document,
            prepared,
            browser_gpu_available(),
            BrowserComputePurpose::ResidentDocument {
                generation: 1,
                previous: None,
            },
        )
    })
    .transpose()
    .map_err(to_js_error)?;
    #[cfg(feature = "browser_compute")]
    let (factory, coordinator, compute) = match prepared {
        Some(prepared) => (
            Some(prepared.factory),
            Some(prepared.coordinator),
            Some(prepared.bridge),
        ),
        None => (None, None, None),
    };
    let mut runtime = project_runtime_builder(
        document,
        resolver,
        #[cfg(feature = "served_project_authority")]
        authority,
        false,
        #[cfg(feature = "browser_compute")]
        factory,
        #[cfg(feature = "browser_host_scene")]
        scenes,
        #[cfg(feature = "browser_compute")]
        pointer,
    )?
    .build()
    .map_err(to_js_error)?;
    #[cfg(feature = "browser_compute")]
    if let Some(coordinator) = coordinator {
        let durability = runtime.config().resident_durability;
        runtime
            .load_compiled_program(coordinator, durability)
            .map_err(to_js_error)?;
        return Ok(ProjectRuntimeCandidate { runtime, compute });
    }
    run_project_sources(&mut runtime, document).map_err(to_js_error)?;
    Ok(ProjectRuntimeCandidate {
        runtime,
        #[cfg(feature = "browser_compute")]
        compute,
    })
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
    #[cfg(feature = "browser_compute")]
    providers.insert("compute", "browser_compute");
    #[cfg(feature = "browser_compute")]
    providers.insert("pointer", "browser_compute");
    providers
}

#[cfg(feature = "browser_compute")]
fn browser_gpu_available() -> bool {
    let Some(window) = web_sys::window() else {
        return false;
    };
    if let Ok(available) = Reflect::get(window.as_ref(), &JsValue::from_str("__MECH_GPU_AVAILABLE"))
        && let Some(available) = available.as_bool()
    {
        return available;
    }
    Reflect::get(window.as_ref(), &JsValue::from_str("navigator"))
        .ok()
        .and_then(|navigator| Reflect::get(&navigator, &JsValue::from_str("gpu")).ok())
        .is_some_and(|gpu| !gpu.is_null() && !gpu.is_undefined())
}

fn standard_browser_provider_feature(provider: &str) -> Option<&'static str> {
    match provider {
        "browser" => Some("browser_host_dom"),
        "time" => Some("browser_host_time"),
        "timer" => Some("browser_host_timer"),
        "console" => Some("browser_host_console"),
        "scene" => Some("browser_host_scene"),
        "compute" => Some("browser_compute"),
        "pointer" => Some("browser_compute"),
        _ => None,
    }
}

fn validate_compiled_host_providers(document: &MechConfigDocument) -> mech_core::MResult<()> {
    validate_compiled_host_providers_for_hosts(&document.hosts)
}

fn validate_compiled_host_providers_for_hosts(
    hosts: &[mech_runtime::HostInstanceConfig],
) -> mech_core::MResult<()> {
    let compiled = compiled_browser_providers();
    for host in hosts {
        if let Some(feature) = standard_browser_provider_feature(&host.provider) {
            if !compiled.contains_key(host.provider.as_str()) {
                return Err(MechError::new(
                    ProjectError {
                        message: format!(
                            "project requires host provider `{}`, but this WASM artifact was built without `{}`",
                            host.provider, feature
                        ),
                    },
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
            return Err(js_error(format!(
                "unsupported trusted host key algorithm `{}`",
                key.algorithm
            )));
        }
        let public_key = base64::engine::general_purpose::STANDARD
            .decode(key.public_key.as_bytes())
            .map_err(|error| js_error(format!("invalid trusted host key publicKey: {error}")))?;
        if public_key.len() != 32 {
            return Err(js_error(format!(
                "trusted host key publicKey must decode to 32 bytes, got {}",
                public_key.len()
            )));
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
    let window = web_sys::window()
        .ok_or_else(|| js_error("served project authority requires a browser window"))?;
    let host_config = Reflect::get(&window, &JsValue::from_str("__MECH_HOST_CONFIG"))?;
    if host_config.is_undefined() || host_config.is_null() {
        return Err(js_error(
            "served project authority is missing __MECH_HOST_CONFIG",
        ));
    }
    #[cfg(feature = "served_project_authority")]
    {
        let trusted = Reflect::get(&window, &JsValue::from_str("__MECH_TRUSTED_HOST_KEYS"))?;
        let audience = Reflect::get(
            &window,
            &JsValue::from_str("__MECH_HOST_DELEGATION_AUDIENCE"),
        )?;
        if !trusted.is_undefined() && !trusted.is_null() {
            let envelope: BrowserHostDelegationEnvelope =
                serde_wasm_bindgen::from_value(host_config.clone()).map_err(|error| {
                    js_error(format!("invalid served host delegation envelope: {error}"))
                })?;
            let trusted_keys = trusted_host_keys_from_js_value(trusted)?;
            let audience = audience
                .as_string()
                .ok_or_else(|| js_error("served host delegation audience must be a string"))?;
            let now_ms = js_sys::Date::now().max(0.0) as u64;
            let verified = verify_browser_host_delegation(
                &envelope,
                HostDelegationVerificationRequest {
                    now_ms,
                    expected_audience: audience,
                    trusted_keys,
                    max_clock_skew_ms: 60_000,
                },
            )
            .map_err(to_js_error)?;
            return Ok(verified.authority.runtime_injection);
        }
    }
    serde_wasm_bindgen::from_value(host_config)
        .map_err(|error| js_error(format!("invalid served host config: {error}")))
}

#[cfg(feature = "served_project_authority")]
fn validate_served_authority(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
) -> mech_core::MResult<()> {
    for required in &document.hosts {
        if !authority
            .hosts
            .iter()
            .any(|host| host.name == required.name && host.provider == required.provider)
        {
            return Err(MechError::new(
                ProjectError {
                    message: format!(
                        "served project requires host `{}` provider `{}`, but server authority did not grant it",
                        required.name, required.provider
                    ),
                },
                None,
            ));
        }
    }
    validate_required_grants(document, authority)?;
    Ok(())
}

/// Source declares required host identity and grants; issued authority owns
/// host settings. Prepare the compute backend and manifest from the same
/// settings that the runtime will install, including legitimate overrides.
#[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
fn compute_document_with_issued_settings(
    document: &MechConfigDocument,
    authority: &BrowserRuntimeInjectionConfig,
) -> MResult<MechConfigDocument> {
    let mut effective = document.clone();
    for required in effective
        .hosts
        .iter_mut()
        .filter(|host| host.provider == "compute")
    {
        let issued = authority
            .hosts
            .iter()
            .find(|host| host.name == required.name && host.provider == required.provider)
            .ok_or_else(|| {
                document_runtime_error("required compute host was not issued by served authority")
            })?;
        required.settings = issued.settings.clone();
    }
    Ok(effective)
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
            if authority
                .run_grants
                .iter()
                .any(|issued| issued.target == required.target)
            {
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
                return Err(MechError::new(
                    ProjectError {
                        message: format!(
                            "served project requires grant `{}`, but server authority did not issue it",
                            required.target
                        ),
                    },
                    None,
                ));
            }
            for operation in &required.operations {
                for path in &required.paths {
                    let authorized = issued.iter().any(|grant| {
                        grant.operations.iter().any(|issued| issued == operation)
                            && grant
                                .paths
                                .iter()
                                .any(|issued| grant_path_allows(issued, path))
                    });
                    if !authorized {
                        return Err(MechError::new(
                            ProjectError {
                                message: format!(
                                    "served project grant `{}` requires operation `{}` on path `{}` outside server authority",
                                    required.target, operation, path
                                ),
                            },
                            None,
                        ));
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

fn required_resolution_field(value: &JsValue, field: &'static str) -> Result<String, JsValue> {
    let value = Reflect::get(value, &JsValue::from_str(field))?
        .as_string()
        .ok_or_else(|| {
            js_error(format!(
                "document source resolution `{field}` must be a string",
            ))
        })?;
    if value.trim().is_empty() {
        return Err(js_error(format!(
            "document source resolution `{field}` must not be empty",
        )));
    }
    Ok(value)
}

fn document_resolutions_from_js(
    value: JsValue,
    sources: &HashMap<String, String>,
) -> Result<Vec<SourceResolutionEntry>, JsValue> {
    if !Array::is_array(&value) {
        return Err(js_error("document source resolutions must be an array"));
    }
    let mut resolutions = Vec::new();
    for entry in Array::from(&value).iter() {
        if !entry.is_object() || entry.is_null() {
            return Err(js_error(
                "document source resolution entries must be objects",
            ));
        }
        let referrer = required_resolution_field(&entry, "referrer")?;
        let specifier = required_resolution_field(&entry, "specifier")?;
        let target = required_resolution_field(&entry, "target")?;
        resolutions.push(SourceResolutionEntry::new(referrer, specifier, target));
    }
    validate_source_resolution_entries(sources.keys().map(String::as_str), &resolutions)
        .map_err(to_js_error)?;
    resolutions.sort();
    resolutions.dedup();
    Ok(resolutions)
}

fn project_source_resolver(
    sources: &HashMap<String, String>,
) -> mech_core::MResult<InMemorySourceResolver> {
    let mut resolver = InMemorySourceResolver::new();
    for (specifier, source) in sources {
        resolver.insert_string(specifier, source)?;
    }
    Ok(resolver)
}

fn project_source_resolver_with_provenance(
    sources: &HashMap<String, String>,
    provenance: &HashMap<String, ServedSourceProvenance>,
) -> mech_core::MResult<InMemorySourceResolver> {
    let mut resolver = InMemorySourceResolver::new();
    for (specifier, source) in sources {
        if let Some(retained) = provenance.get(specifier) {
            let mut resolved = ResolvedSource::new(
                specifier,
                format!("memory:{specifier}"),
                MechSourceCode::String(source.clone()),
            )
            .with_kind(SourceKind::Mech)
            .with_nominal_origin(retained.nominal_origin.clone());
            if let Some(package_id) = &retained.nominal_package_id {
                resolved = resolved.with_nominal_package_id(package_id.clone());
            }
            resolver.insert_source(
                specifier,
                resolved
                    .retain_source_document(
                        mech_syntax::document::Revision(0),
                        mech_syntax::document::ParseConfig::default(),
                    )?
                    .admit_canonical_document()?,
            )?;
        } else {
            resolver.insert_string(specifier, source)?;
        }
    }
    Ok(resolver)
}

fn project_source_resolver_with_resolutions(
    sources: &HashMap<String, String>,
    resolutions: &[SourceResolutionEntry],
) -> mech_core::MResult<InMemorySourceResolver> {
    validate_source_resolution_entries(sources.keys().map(String::as_str), resolutions)?;
    let mut resolver = project_source_resolver(sources)?;
    for resolution in resolutions {
        resolver.insert_resolution_entry(resolution)?;
    }
    Ok(resolver)
}

fn project_source_resolver_with_resolutions_and_provenance(
    sources: &HashMap<String, String>,
    resolutions: &[SourceResolutionEntry],
    provenance: &HashMap<String, ServedSourceProvenance>,
) -> mech_core::MResult<InMemorySourceResolver> {
    if provenance.is_empty() {
        return project_source_resolver_with_resolutions(sources, resolutions);
    }
    validate_source_resolution_entries(sources.keys().map(String::as_str), resolutions)?;
    let mut resolver = project_source_resolver_with_provenance(sources, provenance)?;
    for resolution in resolutions {
        resolver.insert_resolution_entry(resolution)?;
    }
    Ok(resolver)
}

fn document_source_resolver(
    document: &SourceDocument,
    source: &WasmDocumentBootstrap,
) -> MResult<InMemorySourceResolver> {
    let document = source.preserve_document_provenance(document.clone());
    if source.root_specifier.trim().is_empty() {
        return Err(document_runtime_error(
            "document root specifier must not be empty",
        ));
    }
    if !source.source_map.contains_key(&source.root_specifier) {
        return Err(document_runtime_error(format!(
            "document root `{}` is missing from the source map",
            source.root_specifier,
        )));
    }

    let mut resolver = project_source_resolver_with_resolutions_and_provenance(
        &source.source_map,
        &source.resolutions,
        &source.provenance,
    )?;
    let default_root_uri = format!("memory:{}", source.root_specifier);
    let mut derived_root_resolutions = Vec::new();
    let index = document
        .index()
        .map_err(|error| MechError::new(error, None))?;
    for declaration in index.root.program_imports() {
        if !import_may_resolve_source_dependency(&declaration) {
            continue;
        }
        let request = source_request_for_import(&declaration, Some(&default_root_uri));
        let Some(resolved) = mech_runtime::SourceResolver::resolve(&resolver, &request)? else {
            continue;
        };
        let Some(target) = source
            .source_map
            .keys()
            .find(|specifier| format!("memory:{specifier}") == resolved.canonical_uri)
        else {
            continue;
        };
        derived_root_resolutions.push(SourceResolutionEntry::new(
            source.root_specifier.clone(),
            request.specifier,
            target.clone(),
        ));
    }
    let candidate_source = document.source().to_contiguous_string();
    let resolved = ResolvedSource::new(
        &source.root_specifier,
        "runtime:interactive",
        MechSourceCode::String(candidate_source),
    )
    .with_source_document(document.clone())?
    .with_kind(SourceKind::Mech);
    resolver.insert_source(&source.root_specifier, resolved)?;
    for resolution in &derived_root_resolutions {
        resolver.insert_resolution_entry(resolution)?;
    }
    Ok(resolver)
}

fn browser_module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new(
        env!("CARGO_PKG_VERSION"),
        "v0.3",
        "wasm32-unknown-unknown",
        &[],
        &[],
    )
}

fn run_project_sources(
    runtime: &mut MechRuntime,
    document: &MechConfigDocument,
) -> mech_core::MResult<()> {
    let run = require_run(document)?;
    let roots = run
        .paths
        .iter()
        .map(|path| path.to_string_lossy().to_string())
        .collect::<Vec<_>>();
    run_source_roots(runtime, roots.iter().map(String::as_str))
}

#[cfg(feature = "served_project_authority")]
fn validate_static_project_sources(
    document: &MechConfigDocument,
    sources: &HashMap<String, String>,
    resolutions: &[SourceResolutionEntry],
    provenance: &HashMap<String, ServedSourceProvenance>,
) -> MResult<()> {
    validate_compiled_host_providers(document)?;
    validate_static_nominal_provenance(sources, provenance)?;
    let resolver =
        project_source_resolver_with_resolutions_and_provenance(sources, resolutions, provenance)?;
    #[cfg(feature = "browser_compute")]
    let prepared = prepare_project_compute_region(document, resolver.clone(), None)?;
    #[cfg(feature = "browser_compute")]
    let factory = prepared
        .as_ref()
        .map(|prepared| prepare_browser_compute_admission_factory(document, prepared))
        .transpose()?;
    let builder = project_runtime_builder(
        document,
        resolver,
        None,
        false,
        #[cfg(feature = "browser_compute")]
        factory,
        #[cfg(feature = "browser_host_scene")]
        BrowserSceneRegistry::new(),
        #[cfg(feature = "browser_compute")]
        None,
    )
    .map_err(js_value_to_mech_error)?;
    let run = require_run(document)?;
    if run.paths.len() != 1 {
        return Err(document_runtime_error(
            "static browser projects require exactly one root",
        ));
    }
    let mut compiler = builder.build_compiler()?;
    #[cfg(feature = "browser_compute")]
    if let Some(prepared) = prepared {
        compiler.preflight_resident_artifact(&prepared.coordinator)?;
        return Ok(());
    }
    // Non-compute projects keep the same interactive root compiler and catalog
    // as their source loader. Mixed projects preflight the prepared coordinator.
    let product = compiler.compile_canonical_interactive_root_with_options(
        SourceRequest::new(run.paths[0].to_string_lossy()),
        browser_module_options(),
    )?;
    compiler.preflight_resident_artifact(product.artifact())?;
    Ok(())
}

#[cfg(feature = "served_project_authority")]
fn validate_static_nominal_provenance(
    sources: &HashMap<String, String>,
    provenance: &HashMap<String, ServedSourceProvenance>,
) -> MResult<()> {
    for (specifier, source) in sources {
        let document = SourceDocument::parse_resolved(
            specifier,
            mech_syntax::document::Revision(0),
            source.as_str(),
            Default::default(),
        )
        .map_err(|error| document_runtime_error(format!("invalid static source: {error:?}")))?;
        if CanonicalSourceFrontend
            .has_origin_dependent_declarations(&document.document())
            .map_err(|error| document_runtime_error(error.to_string()))?
            && !provenance.contains_key(specifier)
        {
            return Err(document_runtime_error(format!(
                "static source {specifier} is missing nominal provenance; regenerate the bundle"
            )));
        }
    }
    Ok(())
}

fn run_source_roots<'a>(
    runtime: &mut MechRuntime,
    roots: impl IntoIterator<Item = &'a str>,
) -> mech_core::MResult<()> {
    let roots = roots
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<String>>();
    if roots.len() != 1 {
        return Err(MechError::new(
            ResidentRouteFailure {
                class: ResidentRouteFailureClass::MultipleRootsUnsupported,
                reason: "browser products require exactly one resident program root".to_string(),
            },
            None,
        ));
    }
    let durability = runtime.config().resident_durability;
    runtime.load_interactive_root_program(
        SourceRequest::new(roots[0].clone()),
        browser_module_options(),
        durability,
    )?;
    Ok(())
}

fn runtime_route_name(route: RuntimeProgramRoute) -> &'static str {
    match route {
        RuntimeProgramRoute::None => "none",
        RuntimeProgramRoute::ResidentPure => "resident-pure",
        RuntimeProgramRoute::ResidentExternal => "resident-external",
    }
}

fn runtime_info_value(info: &RuntimeProgramExecutionInfo) -> Result<JsValue, JsValue> {
    let out = Object::new();
    let revision = info.program_revision.map(|revision| {
        revision
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>()
    });
    for (key, value) in [
        ("route", JsValue::from_str(runtime_route_name(info.route))),
        ("routing_policy", JsValue::from_str("require-resident")),
        (
            "program_revision",
            revision.map_or(JsValue::NULL, |value| JsValue::from_str(&value)),
        ),
        (
            "plan_generation",
            info.plan_generation.map_or(JsValue::NULL, |value| {
                JsValue::from_f64(value.get().saturating_add(1) as f64)
            }),
        ),
        (
            "layout_generation",
            info.layout_generation.map_or(JsValue::NULL, |value| {
                JsValue::from_f64(value.get().saturating_add(1) as f64)
            }),
        ),
        (
            "requirements",
            JsValue::from_f64(info.requirement_count as f64),
        ),
        (
            "observations",
            JsValue::from_f64(info.observation_count as f64),
        ),
        ("effects", JsValue::from_f64(info.effect_count as f64)),
        (
            "resident_accepted_turns",
            JsValue::from_f64(info.resident_accepted_turns as f64),
        ),
        (
            "resident_rejected_turns",
            JsValue::from_f64(info.resident_rejected_turns as f64),
        ),
        (
            "coalesced_host_packets",
            JsValue::from_f64(info.coalesced_host_packets as f64),
        ),
        (
            "ignored_host_packets",
            JsValue::from_f64(info.ignored_host_packets as f64),
        ),
    ] {
        Reflect::set(&out, &JsValue::from_str(key), &value)?;
    }
    Ok(out.into())
}

fn rendered_value_kind(snapshot: &mech_runtime::RuntimeValueSnapshot) -> String {
    snapshot.format_repl_kind()
}

pub(super) fn rendered_value(
    snapshot: mech_runtime::RuntimeValueSnapshot,
    max_elements: usize,
) -> Result<JsValue, JsValue> {
    let rendered = Object::new();
    Reflect::set(
        &rendered,
        &JsValue::from_str("kind"),
        &JsValue::from_str(&rendered_value_kind(&snapshot)),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("blockHtml"),
        &JsValue::from_str(&snapshot.format_repl_html(max_elements)),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("inlineHtml"),
        &JsValue::from_str(&mech_core::escape_html_text(
            &snapshot.format_repl_inline(max_elements),
        )),
    )?;
    Ok(rendered.into())
}

fn rendered_named_value(
    snapshot: mech_runtime::RuntimeValueSnapshot,
    name: Option<&str>,
    max_elements: usize,
) -> Result<JsValue, JsValue> {
    let rendered = rendered_value(snapshot, max_elements)?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("name"),
        &name.map(JsValue::from_str).unwrap_or(JsValue::NULL),
    )?;
    Ok(rendered)
}

fn set_rendered_output_identity(rendered: &JsValue, output_id: u64) -> Result<(), JsValue> {
    Reflect::set(
        rendered,
        &JsValue::from_str("outputId"),
        &JsValue::from_str(&output_id.to_string()),
    )?;
    Reflect::set(
        rendered,
        &JsValue::from_str("identity"),
        &JsValue::from_str(&format!("resident-output:{output_id}")),
    )?;
    Ok(())
}

fn set_rendered_selection_identity(
    rendered: &JsValue,
    selection_token: &str,
) -> Result<(), JsValue> {
    Reflect::set(
        rendered,
        &JsValue::from_str("selectionToken"),
        &JsValue::from_str(selection_token),
    )?;
    Reflect::set(
        rendered,
        &JsValue::from_str("identity"),
        &JsValue::from_str(selection_token),
    )?;
    Ok(())
}

pub(super) fn rendered_symbol_names_from_js(
    names: JsValue,
) -> Result<Option<Vec<String>>, JsValue> {
    if names.is_null() || names.is_undefined() {
        return Ok(None);
    }
    if !Array::is_array(&names) {
        return Err(js_error(
            "renderedSymbols names must be null, undefined, or an array of strings",
        ));
    }
    Array::from(&names)
        .iter()
        .map(|name| {
            name.as_string()
                .ok_or_else(|| js_error("renderedSymbols names must contain only strings"))
        })
        .collect::<Result<Vec<_>, _>>()
        .map(Some)
}

pub(super) fn rendered_symbol_row(
    name: &str,
    snapshot: mech_runtime::RuntimeValueSnapshot,
    max_elements: usize,
) -> Result<JsValue, JsValue> {
    let rendered_value = rendered_value(snapshot, max_elements)?;
    let row = Object::new();
    Reflect::set(&row, &JsValue::from_str("name"), &JsValue::from_str(name))?;
    for property in ["kind", "inlineHtml", "blockHtml"] {
        Reflect::set(
            &row,
            &JsValue::from_str(property),
            &Reflect::get(&rendered_value, &JsValue::from_str(property))?,
        )?;
    }
    Ok(row.into())
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
fn js_error(
    #[cfg(target_arch = "wasm32")] message: impl Into<String>,
    #[cfg(not(target_arch = "wasm32"))] _: impl Into<String>,
) -> JsValue {
    #[cfg(not(target_arch = "wasm32"))]
    {
        return JsValue::NULL;
    }
    #[cfg(target_arch = "wasm32")]
    JsValue::from_str(&message.into())
}
fn to_js_error(
    #[cfg(target_arch = "wasm32")] error: MechError,
    #[cfg(not(target_arch = "wasm32"))] _: MechError,
) -> JsValue {
    #[cfg(not(target_arch = "wasm32"))]
    {
        JsValue::NULL
    }
    #[cfg(target_arch = "wasm32")]
    {
        let recoverable_resident_turn = error
            .kind_as::<mech_runtime::ResidentHostTurnFailed>()
            .is_some_and(mech_runtime::ResidentHostTurnFailed::is_recoverable);
        let kind = error.kind_name();
        let rendered = format!("{error:?}");
        let javascript_error = js_sys::Error::new(&rendered);
        let target = javascript_error.as_ref();
        drop(Reflect::set(
            target,
            &JsValue::from_str("mechKind"),
            &JsValue::from_str(&kind),
        ));
        drop(Reflect::set(
            target,
            &JsValue::from_str("mechRecoverableResidentTurn"),
            &JsValue::from_bool(recoverable_resident_turn),
        ));
        javascript_error.into()
    }
}

#[cfg(test)]
fn test_document_payload(root_specifier: &str, source: &str) -> BrowserDocumentPayload {
    let document = SourceDocument::parse_resolved(
        "runtime:test-presentation",
        mech_syntax::document::Revision(0),
        source,
        mech_syntax::document::ParseConfig::default(),
    )
    .unwrap();
    let presentation_output_ids = CanonicalSourceFrontend
        .compile_document(&document.document())
        .ok()
        .into_iter()
        .flat_map(|program| {
            program
                .document_outputs()
                .iter()
                .filter(|output| output.visible && output.kind != SourceDocumentOutputKind::Program)
                .map(|output| {
                    mech_core::hash_str(&format!("browser-test-output:{}", output.output))
                })
                .collect::<Vec<_>>()
        });
    BrowserDocumentPayload::new(root_specifier, source)
        .unwrap()
        .with_presentation_output_ids(presentation_output_ids)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_rendered_snapshot_unchanged(
        snapshot: &mech_runtime::RuntimeValueSnapshot,
        before: &mech_core::Value,
    ) {
        let after = snapshot.value();
        assert_eq!(after.schema(), before.schema());
        assert_eq!(after.schema_key(), before.schema_key());
        assert_eq!(after.shape(), before.shape());
        assert!(
            std::ptr::eq(after.data(), before.data()),
            "rendering retains the exact immutable payload owner"
        );
    }

    fn rendered_kind_snapshot(
        schema: mech_core::SchemaDraft,
        shape_values: Box<[u64]>,
        data: mech_core::ValueDataDraft,
    ) -> mech_runtime::RuntimeValueSnapshot {
        let mut builder = mech_core::SchemaTableBuilder::new();
        let handle = builder.insert(schema.finalize().unwrap()).unwrap();
        let build = builder.finish().unwrap();
        let schema = build.resolve(handle).unwrap();
        let (schemas, _) = build.into_parts();
        let value = mech_core::ValueDraft {
            schema,
            shape_values,
            data,
        }
        .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
            &schemas,
        ))
        .unwrap();
        mech_runtime::RuntimeValueSnapshot::from_value(value).unwrap()
    }

    fn static_rendered_kind_snapshot(
        body: mech_core::SchemaBody,
        data: mech_core::ValueDataDraft,
    ) -> mech_runtime::RuntimeValueSnapshot {
        rendered_kind_snapshot(
            mech_core::SchemaDraft {
                dimension_parameters: Box::new([]),
                body,
            },
            Box::new([]),
            data,
        )
    }

    #[test]
    fn browser_rendered_kind_preserves_nonsquare_oriented_and_empty_matrix_shapes() {
        use mech_core::{DimensionExpr, FloatWidth, SchemaBody, ValueDataDraft};

        for (rows, columns) in [(2, 3), (1, 3), (3, 1), (0, 3), (3, 0)] {
            let snapshot = static_rendered_kind_snapshot(
                SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: vec![
                        DimensionExpr::Constant(rows),
                        DimensionExpr::Constant(columns),
                    ]
                    .into_boxed_slice(),
                },
                ValueDataDraft::Matrix(
                    (0..rows * columns)
                        .map(|value| {
                            ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(
                                value as f64,
                            ))
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                ),
            );
            let before = snapshot.value().clone();
            assert_eq!(
                rendered_value_kind(&snapshot),
                format!("[f64]:{rows},{columns}")
            );
            let _ = snapshot.format_repl_inline(1);
            assert_eq!(
                rendered_value_kind(&snapshot),
                format!("[f64]:{rows},{columns}"),
                "payload elision cannot erase the complete browser kind"
            );
            assert_rendered_snapshot_unchanged(&snapshot, &before);
            assert!(snapshot.value().shape().parameter_values().is_empty());
        }
    }

    #[test]
    fn browser_rendered_kind_uses_the_resolved_shape_witness() {
        use mech_core::{
            DimensionExpr, DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
            DimensionParameterOrigin, FloatWidth, SchemaBody, SchemaDraft, ValueDataDraft,
        };

        let snapshot = rendered_kind_snapshot(
            SchemaDraft {
                dimension_parameters: (0..2)
                    .map(|id| DimensionParameterDeclaration {
                        id: DimensionParameterId::new(id),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(1),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
                    dimensions: vec![
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        DimensionExpr::Add(
                            vec![
                                DimensionExpr::Parameter(DimensionParameterId::new(1)),
                                DimensionExpr::Constant(1),
                            ]
                            .into_boxed_slice(),
                        ),
                    ]
                    .into_boxed_slice(),
                },
            },
            vec![2, 2].into_boxed_slice(),
            ValueDataDraft::Matrix(
                (0..6)
                    .map(|value| {
                        ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value as f64))
                    })
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            ),
        );
        let before = snapshot.value().clone();
        assert_eq!(rendered_value_kind(&snapshot), "[f64]:2,3");
        assert_eq!(snapshot.value().shape().parameter_values(), [2, 2]);
        assert_rendered_snapshot_unchanged(&snapshot, &before);
    }

    #[test]
    fn browser_rendered_kind_preserves_exact_integer_interval_identity() {
        use mech_core::{DimensionExpr, IntegerInterval, IntegerWidth, SchemaBody, ValueDataDraft};

        let lower = u128::MAX - 1;
        for upper_inclusive in [false, true] {
            let interval = SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower,
                upper: u128::MAX,
                upper_inclusive,
            });
            let expected = format!(
                "u128:{lower}..{}{}",
                if upper_inclusive { "=" } else { "" },
                u128::MAX
            );
            let scalar =
                static_rendered_kind_snapshot(interval.clone(), ValueDataDraft::U128(lower));
            let before = scalar.value().clone();
            assert_eq!(rendered_value_kind(&scalar), expected);
            assert_rendered_snapshot_unchanged(&scalar, &before);
            let matrix = static_rendered_kind_snapshot(
                SchemaBody::Matrix {
                    element: Box::new(interval),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                        .into_boxed_slice(),
                },
                ValueDataDraft::Matrix(vec![ValueDataDraft::U128(lower); 2].into_boxed_slice()),
            );
            let before = matrix.value().clone();
            assert_eq!(rendered_value_kind(&matrix), format!("[{expected}]:1,2"));
            assert_rendered_snapshot_unchanged(&matrix, &before);
        }
        let signed = static_rendered_kind_snapshot(
            SchemaBody::IntegerInterval(IntegerInterval::Signed {
                width: IntegerWidth::W128,
                lower: i128::MIN,
                upper: -1,
                upper_inclusive: false,
            }),
            ValueDataDraft::I128(i128::MIN),
        );
        let before = signed.value().clone();
        assert_eq!(
            rendered_value_kind(&signed),
            format!("i128:{}..-1", i128::MIN)
        );
        assert_rendered_snapshot_unchanged(&signed, &before);
        let base = static_rendered_kind_snapshot(
            SchemaBody::UnsignedInteger(IntegerWidth::W128),
            ValueDataDraft::U128(lower),
        );
        let interval = static_rendered_kind_snapshot(
            SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower,
                upper: u128::MAX,
                upper_inclusive: false,
            }),
            ValueDataDraft::U128(lower),
        );
        let before = base.value().clone();
        assert_eq!(rendered_value_kind(&base), "u128");
        assert_eq!(
            rendered_value_kind(&interval),
            format!("u128:{lower}..{}", u128::MAX)
        );
        assert_eq!(base.kind(), interval.kind());
        for snapshot in [&base, &interval] {
            assert!(
                matches!(snapshot.value().data(), mech_core::ValueData::U128(value) if *value == lower)
            );
        }
        assert_ne!(base.schema_key(), interval.schema_key());
        assert_rendered_snapshot_unchanged(&base, &before);
    }

    #[test]
    fn pretty_text_normalizes_set_commas() {
        let document = SourceDocument::parse_resolved(
            "set-format.mec",
            mech_syntax::document::Revision(0),
            "values:={1,2}\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let formatted = mech_runtime::CanonicalDocumentRenderer
            .format_pretty_text(&document.document())
            .unwrap();
        assert_eq!(formatted, "values := {1, 2}\n");
    }

    const CONFIG: &str = r#"config := {
  hosts: []
  run: {
    paths: ["a.mec" "b.mec"]
    grants: []
  }
}"#;

    fn document_payload(root_specifier: &str, source: &str) -> BrowserDocumentPayload {
        let document = SourceDocument::parse_resolved(
            "runtime:test-presentation",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let presentation_output_ids =
            mech_runtime::canonical_document_presentation_output_ids(&document.document()).unwrap();
        BrowserDocumentPayload::new(root_specifier, source)
            .unwrap()
            .with_presentation_output_ids(presentation_output_ids)
    }

    fn document_bootstrap(
        root_specifier: &str,
        source: &str,
        mut source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
    ) -> WasmDocumentBootstrap {
        source_map.insert(root_specifier.to_owned(), source.to_owned());
        let payload = document_payload(root_specifier, source);
        let document = CanonicalWasmDocument::retain(
            "runtime:interactive",
            mech_syntax::document::Revision(0),
            source,
        )
        .unwrap();
        let document_base = Rc::new(RefCell::new(Staged {
            active: document.document().clone(),
            pending: None,
        }));
        WasmDocumentBootstrap {
            root_specifier: root_specifier.to_owned(),
            source_map,
            resolutions,
            provenance: HashMap::new(),
            document,
            document_base,
            presentation_state: Rc::new(RefCell::new(Staged::default())),
            presentation_output_ids: payload.presentation_output_ids().to_vec(),
            initial_bundle: None,
            console_instance: "repl".to_owned(),
            lifecycle: DocumentRuntimeLifecycle::default(),
            #[cfg(feature = "served_project_authority")]
            served: None,
        }
    }

    #[test]
    fn document_output_hashes_map_to_resident_output_ordinals() {
        for source in [
            include_str!("../../../examples/working/fizzbuzz.mec"),
            include_str!("../../../tests/fixtures/shims/all-slots.mec"),
        ] {
            let bootstrap = document_bootstrap("document.mec", source, HashMap::new(), Vec::new());
            let candidate = bootstrap.initial_document();
            let (runtime, _) = activate_document_repl_runtime_document(
                &bootstrap,
                MechEventBuffer::default(),
                &candidate,
            )
            .unwrap();
            let outputs = document::document_output_ordinals_for_runtime(
                &bootstrap, &candidate, &runtime, true,
            )
            .unwrap();
            for output_id in &bootstrap.presentation_output_ids {
                assert!(outputs.contains_key(output_id));
            }
            assert_eq!(
                outputs.contains_key(&root_document_program_output_id()),
                runtime.program_output_id().is_some(),
            );
        }
    }

    #[test]
    fn bundled_import_outputs_use_the_server_artifact_ordinals() {
        let source = "+> ./dep.mec\nanswer := dep/value + 1.0\nanswer\n\nResult {answer}.\n";
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_canonical_string("dep.mec", "value := 41.0\n<+ value\n")
            .unwrap();
        resolver
            .insert_canonical_string("main.mec", source)
            .unwrap();
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        let product = compiler
            .compile_canonical_interactive_root(SourceRequest::new("main.mec"))
            .unwrap();
        let root = SourceDocument::parse_resolved(
            "main.mec",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let bundle = CanonicalProgramBundle::from_product("main.mec", &root, &product).unwrap();
        let mut bootstrap = document_bootstrap(
            "main.mec",
            source,
            HashMap::from([("dep.mec".to_owned(), "value := 41.0\n<+ value\n".to_owned())]),
            Vec::new(),
        );
        bootstrap.presentation_output_ids =
            mech_runtime::canonical_document_presentation_output_ids(&root.document()).unwrap();
        assert!(!bootstrap.presentation_output_ids.is_empty());
        bootstrap.initial_bundle = Some(bundle);

        let initial = bootstrap.initial_document();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &initial,
        )
        .unwrap();
        let ordinals =
            document::document_output_ordinals_for_runtime(&bootstrap, &initial, &runtime, true)
                .unwrap();
        assert!(ordinals.contains_key(&root_document_program_output_id()));
        for output_id in &bootstrap.presentation_output_ids {
            assert!(ordinals.contains_key(output_id));
        }

        let candidate_source = format!("{source}\nnext := answer + 1.0\nnext\n");
        let candidate = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            candidate_source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &candidate,
        )
        .unwrap();
        let refreshed =
            document::document_output_ordinals_for_runtime(&bootstrap, &candidate, &runtime, false)
                .unwrap();
        assert!(refreshed.contains_key(&root_document_program_output_id()));
        for output_id in &bootstrap.presentation_output_ids {
            assert!(refreshed.contains_key(output_id));
        }
        let fixed = refreshed[&root_document_program_output_id()];
        assert_eq!(
            runtime
                .output_value(OutputId::new(fixed as u32))
                .unwrap()
                .unwrap()
                .to_string(),
            "42",
            "the imported document result survives the appended value 43",
        );
    }

    #[test]
    fn accepted_export_edit_rebuilds_presentation_ordinals() {
        let original = "value := 42\n\nResult {value}.\n";
        let replacement = "value := 42\n<+ value\n\nResult {value}.\n";
        let bootstrap = document_bootstrap("document.mec", original, HashMap::new(), Vec::new());
        let initial = bootstrap.initial_document();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &initial,
        )
        .unwrap();
        let before =
            document::document_output_ordinals_for_runtime(&bootstrap, &initial, &runtime, true)
                .unwrap();
        let candidate = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            replacement,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        bootstrap.stage_document_base(candidate.clone());
        bootstrap.commit();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &candidate,
        )
        .unwrap();
        let after =
            document::document_output_ordinals_for_runtime(&bootstrap, &candidate, &runtime, false)
                .unwrap();
        let address = bootstrap.presentation_output_ids[0];
        assert_ne!(before[&address], after[&address]);
        let mapped_name = runtime.output_name(OutputId::new(after[&address] as u32));
        assert!(
            mapped_name
                .as_deref()
                .is_some_and(|name| name.starts_with("document:inline:")),
            "mapped presentation output: {mapped_name:?}; active outputs: {:?}",
            runtime.published_outputs(),
        );
    }

    #[test]
    fn interactive_presentation_skips_integrity_constraint_outputs() {
        let source = include_str!("../../../examples/working/fizzbuzz.mec");
        let bootstrap = document_bootstrap("document.mec", source, HashMap::new(), Vec::new());
        let initial = bootstrap.initial_document();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &initial,
        )
        .unwrap();
        let ordinals =
            document::document_output_ordinals_for_runtime(&bootstrap, &initial, &runtime, true)
                .unwrap();
        let address = bootstrap.presentation_output_ids[0];
        let value = runtime
            .output_value(OutputId::new(ordinals[&address] as u32))
            .unwrap()
            .unwrap();
        assert!(value.format_canonical_inline().contains("✨🐝"));
    }

    #[test]
    fn cleared_source_stages_the_document_capture_boundary() {
        let original = "first := 1\nsecond := 2\nsecond\n";
        let retained = "second := 2\nsecond\n";
        let bootstrap = document_bootstrap("document.mec", original, HashMap::new(), Vec::new());
        #[cfg(feature = "browser_compute")]
        let compute_generation = bootstrap.source().lifecycle.compute_generation();
        bootstrap
            .stage_repl_document_boundary(":clear first")
            .unwrap();
        assert_eq!(bootstrap.initial_repl_source(), retained);
        #[cfg(feature = "browser_compute")]
        assert_eq!(
            bootstrap.source().lifecycle.compute_generation(),
            compute_generation,
            "rebasing the document boundary must not commit the already accepted compute bridge",
        );
        let overlaid = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(2),
            format!("{retained}40 + 2\n"),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let (runtime_source, capture) = runtime_document(&bootstrap, &overlaid).unwrap();
        let capture = capture.unwrap();
        let program = CanonicalSourceFrontend
            .compile_interactive_document_with_catalog(
                &runtime_source.document(),
                mech_stdlib::source_catalog(),
            )
            .unwrap();
        let overlay_start = runtime_source
            .source()
            .to_contiguous_string()
            .find("40 + 2")
            .unwrap();
        let capture_start = program.source_map().outputs[capture.get() as usize]
            .range
            .start
            .0 as usize;
        assert!(capture_start < overlay_start);
    }

    #[test]
    fn browser_document_profile_runs_nbody_module_imports() {
        let source = "+> combinatorics\n\
             +> stats\n\
             pairs := combinatorics/n-choose-k(10.0, 2.0)\n\
             column-totals := stats/sum/column([1.0 2.0; 3.0 4.0])\n\
             row-totals := stats/sum/row([1.0 2.0; 3.0 4.0])\n\
             pairs";
        let bootstrap = document_bootstrap("document.mec", source, HashMap::new(), Vec::new());
        let document = bootstrap.initial_document();
        let (runtime, outcome) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &document,
        )
        .unwrap();

        assert_eq!(outcome.route, RuntimeProgramRoute::ResidentPure,);
        assert_eq!(
            runtime
                .root_symbol_values(&["pairs"])
                .unwrap()
                .pop()
                .unwrap()
                .1
                .format_canonical_inline(),
            "45",
        );
        assert_eq!(
            runtime
                .root_symbol_values(&["column-totals"])
                .unwrap()
                .pop()
                .unwrap()
                .1
                .format_canonical_inline(),
            "[3; 7]",
        );
        assert_eq!(
            runtime
                .root_symbol_values(&["row-totals"])
                .unwrap()
                .pop()
                .unwrap()
                .1
                .format_canonical_inline(),
            "[4 6]",
        );
    }

    #[test]
    fn document_program_output_identity_survives_console_overlays() {
        let source = "~answer := 0\nanswer += 7\nanswer\n";
        let encoded = document_payload("document.mec", source).encode().unwrap();
        let mut document = document::WasmDocument::from_encoded(&encoded).unwrap();
        let output_id = document
            .bootstrap
            .program_output_id()
            .unwrap()
            .expect("the fixture has an implicit program output");
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .output_value(output_id)
                .unwrap()
                .unwrap()
                .to_string(),
            "7",
            "the retained document result is captured",
        );

        document.repl.session.submit("40 + 2").unwrap();
        document.refresh_document_output_ordinals(None).unwrap();

        assert_eq!(
            document
                .runtime()
                .unwrap()
                .output_value(output_id)
                .unwrap()
                .unwrap()
                .to_string(),
            "7",
            "a console result must not replace the fixed document output",
        );
        assert_eq!(
            document.document_output_ordinal(root_document_program_output_id()),
            Some(u64::from(output_id.0)),
            "the canonical document mount must remain mapped to the captured result",
        );
    }

    #[test]
    fn accepted_document_edit_becomes_the_program_output_boundary() {
        let source = "answer := 1\nanswer\n";
        let encoded = document_payload("document.mec", source).encode().unwrap();
        let mut document = document::WasmDocument::from_encoded(&encoded).unwrap();
        let replacement = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            "answer := 40\nanswer\n",
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();

        document.bootstrap.stage_document_base(replacement.clone());
        document.repl.session.replace_document(replacement).unwrap();
        document.repl.session.submit("1 + 1").unwrap();

        let output_id = document
            .bootstrap
            .program_output_id()
            .unwrap()
            .expect("the edited document keeps a fixed program output");
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .output_value(output_id)
                .unwrap()
                .unwrap()
                .to_string(),
            "40",
            "a later REPL overlay must remain outside the accepted edit boundary",
        );
    }

    #[test]
    fn required_paths_returns_configured_paths() {
        assert_eq!(
            required_path_strings(CONFIG).unwrap(),
            vec!["a.mec".to_string(), "b.mec".to_string()]
        );
    }

    #[test]
    fn required_paths_omits_directory_serve_paths_after_run_roots() {
        let config = r#"config := {
  hosts: []
  run: {
    paths: ["app/main.mec" "other.mec"]
    grants: []
  }
  serve: {
    paths: ["app" "app/main.mec" "app/lib.mec" "shared" "other.mec"]
  }
}"#;

        assert_eq!(
            required_path_strings(config).unwrap(),
            vec!["app/main.mec", "other.mec", "app/lib.mec"]
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

    fn assert_production_route_failed_closed(
        runtime: &MechRuntime,
        error: &MechError,
        expected: ResidentRouteFailureClass,
    ) {
        let failure = error
            .kind_as::<ResidentRouteFailure>()
            .expect("production project failure must retain its resident route class");
        assert_eq!(failure.class, expected, "{}", failure.reason);
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::None);
    }

    #[test]
    fn multiple_project_roots_fail_closed_without_legacy_execution() {
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();
        let mut sources = HashMap::new();
        sources.insert("a.mec".to_string(), "x := 1".to_string());
        sources.insert("b.mec".to_string(), "y := 2".to_string());
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();
        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::MultipleRootsUnsupported,
        );
    }

    fn project_document(paths: &[&str]) -> MechConfigDocument {
        let paths = paths
            .iter()
            .map(|path| format!("\"{path}\""))
            .collect::<Vec<_>>()
            .join(" ");
        parse_config_document(
            "test.mcfg",
            &format!("config := {{ hosts: [] run: {{ paths: [{paths}] grants: [] }} }}"),
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    #[cfg(feature = "browser_host_dom")]
    #[derive(Clone, Debug, Default)]
    struct ResidentDomBackend {
        state: std::sync::Arc<std::sync::Mutex<ResidentDomState>>,
        read_delay: std::time::Duration,
    }

    #[cfg(feature = "browser_host_dom")]
    #[derive(Debug, Default)]
    struct ResidentDomState {
        reads: Vec<String>,
        writes: Vec<(String, String)>,
    }

    #[cfg(feature = "browser_host_dom")]
    impl ResidentDomBackend {
        fn with_read_delay(read_delay: std::time::Duration) -> Self {
            Self {
                read_delay,
                ..Self::default()
            }
        }

        fn reads(&self) -> Vec<String> {
            self.state.lock().unwrap().reads.clone()
        }

        fn writes(&self) -> Vec<(String, String)> {
            self.state.lock().unwrap().writes.clone()
        }
    }

    #[cfg(feature = "browser_host_dom")]
    impl mech_browser::BrowserDomBackend for ResidentDomBackend {
        fn read_dom_string(
            &self,
            _entry: &mech_browser::BrowserDomManifestEntry,
            requested_path: &mech_browser::BrowserDomPath,
        ) -> mech_core::MResult<String> {
            std::thread::sleep(self.read_delay);
            self.state
                .lock()
                .unwrap()
                .reads
                .push(requested_path.as_str().to_string());
            Ok("Ada".to_string())
        }

        fn write_dom_string(
            &mut self,
            _entry: &mech_browser::BrowserDomManifestEntry,
            requested_path: &mech_browser::BrowserDomPath,
            value: &str,
        ) -> mech_core::MResult<()> {
            self.state
                .lock()
                .unwrap()
                .writes
                .push((requested_path.as_str().to_string(), value.to_string()));
            Ok(())
        }
    }

    #[cfg(feature = "browser_host_dom")]
    fn browser_dom_document() -> MechConfigDocument {
        parse_config_document(
            "examples/browser-dom-demo/demo.mcfg",
            include_str!("../../../examples/browser-dom-demo/demo.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    #[cfg(feature = "browser_host_dom")]
    fn browser_dom_sources() -> HashMap<String, String> {
        HashMap::from([(
            "demo.mec".to_string(),
            include_str!("../../../examples/browser-dom-demo/demo.mec").to_string(),
        )])
    }

    #[cfg(feature = "browser_host_dom")]
    fn browser_dom_builder(
        document: &MechConfigDocument,
        backend: ResidentDomBackend,
    ) -> RuntimeBuilder {
        let runtime_config = mech_runtime::RuntimeConfig::default()
            .apply_patch(&document.runtime)
            .unwrap();
        let mut builder = browser_runtime_builder()
            .config(runtime_config)
            .source_resolver(project_source_resolver(&browser_dom_sources()).unwrap())
            .host_factory(Box::new(BrowserHostFactory::new(backend).unwrap()))
            .unwrap();
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        builder
    }

    #[cfg(feature = "browser_host_dom")]
    fn assert_browser_dom_result(runtime: &MechRuntime, backend: &ResidentDomBackend) {
        assert_eq!(
            runtime.program_route(),
            RuntimeProgramRoute::ResidentExternal
        );
        let info = runtime.program_execution_info();
        assert_eq!(info.resident_accepted_turns, 1);
        assert_eq!(info.observation_count, 1, "{info:?}");
        assert_eq!(
            backend.reads(),
            vec!["body/content/mech-sandbox/input/_value".to_string()],
        );
        let mut writes = backend.writes();
        writes.sort();
        let mut expected = vec![
            (
                "body/content/mech-sandbox/output/_value".to_string(),
                "Hello, Ada — computed in Mech".to_string(),
            ),
            (
                "body/content/mech-sandbox/status".to_string(),
                "Read `Ada` from the DOM and wrote the computed result back.".to_string(),
            ),
            (
                "body/content/mech-sandbox/status/_class".to_string(),
                "ready".to_string(),
            ),
            (
                "body/content/mech-sandbox/title".to_string(),
                "Hello, Ada".to_string(),
            ),
        ];
        expected.sort();
        assert_eq!(writes, expected);
    }

    #[cfg(feature = "browser_host_dom")]
    #[test]
    fn unchanged_browser_dom_demo_runs_source_and_bytecode_residently() {
        let document = browser_dom_document();

        let source_backend = ResidentDomBackend::default();
        let mut source_runtime = browser_dom_builder(&document, source_backend.clone())
            .build()
            .unwrap();
        run_project_sources(&mut source_runtime, &document).unwrap();
        assert_browser_dom_result(&source_runtime, &source_backend);

        let planning_backend = ResidentDomBackend::default();
        let mut compiler = browser_dom_builder(&document, planning_backend.clone())
            .build_compiler()
            .unwrap();
        let bytecode = compiler
            .compile_canonical_interactive_root(SourceRequest::new("demo.mec"))
            .unwrap()
            .into_parts()
            .1;
        assert!(planning_backend.reads().is_empty());
        assert!(planning_backend.writes().is_empty());

        let bytecode_backend = ResidentDomBackend::default();
        let mut bytecode_runtime = browser_dom_builder(&document, bytecode_backend.clone())
            .build()
            .unwrap();
        bytecode_runtime
            .load_bytecode_program(&bytecode, mech_runtime::ResidentDurabilityPolicy::Volatile)
            .unwrap();
        assert_browser_dom_result(&bytecode_runtime, &bytecode_backend);
        assert_eq!(
            source_runtime.program_execution_info().program_revision,
            bytecode_runtime.program_execution_info().program_revision,
        );
    }

    #[cfg(feature = "browser_host_dom")]
    #[test]
    fn rejected_browser_dom_candidate_performs_zero_writes() {
        let document = browser_dom_document();
        let backend = ResidentDomBackend::with_read_delay(std::time::Duration::from_millis(5));
        let mut config = mech_runtime::RuntimeConfig::default();
        config.limits.max_turn_duration_ms = Some(1);
        let mut runtime = browser_dom_builder(&document, backend.clone())
            .config(config)
            .build()
            .unwrap();

        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::ActivationFailure,
        );
        assert!(backend.writes().is_empty());
    }

    #[cfg(feature = "browser_host_dom")]
    #[test]
    fn browser_dom_demo_authority_denial_fails_before_writes() {
        let mut document = browser_dom_document();
        document.run.as_mut().unwrap().grants.clear();
        let backend = ResidentDomBackend::default();
        let mut runtime = browser_dom_builder(&document, backend.clone())
            .build()
            .unwrap();

        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::AuthorizationDenied,
        );
        assert!(backend.reads().is_empty());
        assert!(backend.writes().is_empty());
    }

    #[test]
    fn scalar_string_concatenation_uses_resident_execution() {
        let document = project_document(&["demo.mec"]);

        let mut sources = HashMap::new();
        sources.insert(
            "demo.mec".to_string(),
            r#"greeting := "Hello, " + "Ada""#.to_string(),
        );

        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();

        run_project_sources(&mut runtime, &document).unwrap();
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::ResidentPure);
        assert!(matches!(
            runtime
                .program_output_value()
                .unwrap()
                .unwrap()
                .value()
                .data(),
            mech_core::ValueData::String(value) if value.as_ref() == "Hello, Ada"
        ));
    }

    #[test]
    fn encoded_document_controller_loads_residently_without_legacy_execution() {
        let encoded = test_document_payload("document.mec", "~answer := 0\nanswer += 42\nanswer")
            .encode()
            .unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
    }

    #[test]
    fn encoded_document_controller_accepts_resident_state_machines() {
        let source = r#"#Drive(phase<f64>) => <f64>
  └ :Done(next-phase<f64>).

#Drive(phase<f64>) -> :Done(phase)
  :Done(next-phase) => next-phase.

~phase := 0.0
next-phase := #Drive(phase)
        phase = next-phase
phase"#;
        let encoded = document_payload("document.mec", source).encode().unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
    }

    #[test]
    fn encoded_document_controller_accepts_wildcard_imported_resident_atan2() {
        let source = "Resident Atan2\n================\n+> math/*\n================\n\nangle := atan2(1.0, 0.0)\nangle";
        let module = ["math".to_string()];
        assert!(
            mech_stdlib::source_catalog()
                .resident_factory(&module, "atan2")
                .is_some(),
            "the standard source catalog must install resident math/atan2",
        );
        let bootstrap = document_bootstrap("document.mec", source, HashMap::new(), Vec::new());
        let document = crate::repl::WasmRepl::from_document(bootstrap);
        assert!(document.is_ok(), "{:#?}", document.err());
        let document = document.unwrap();

        assert_eq!(
            document.session.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
    }

    #[test]
    fn encoded_document_controller_accepts_resident_matrix_scalar_access() {
        let source = "matrix := [1.0 2.0; 3.0 4.0]\nselected := matrix[2,1]\nselected";
        let encoded = document_payload("document.mec", source).encode().unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("selected")
                .unwrap(),
            3.0,
        );
    }

    #[test]
    fn document_console_queries_and_updates_the_same_resident_program() {
        let encoded = document_payload("document.mec", "~answer := 0\nanswer += 42\nanswer")
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_eq!(
            document
                .repl
                .session
                .symbols(&["answer".to_string()])
                .unwrap()[0]
                .1
                .to_string(),
            "42",
        );
        assert_eq!(
            document
                .repl
                .session
                .submit("answer += 1\nanswer")
                .unwrap()
                .to_string(),
            "43",
        );
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap()
                .to_string(),
            "43",
            "console mutations must drive the runtime rendered by the document",
        );
        let answer = document
            .repl
            .session
            .symbol("answer")
            .unwrap()
            .expect("answer must be resident");
        document.repl.session.select_value("answer", answer);
        assert_eq!(
            document.repl.session.submit("ans + 1").unwrap().to_string(),
            "44",
            "a clicked value must become the next interactive ans",
        );
        document.repl.session.select_value(
            "another-output",
            mech_runtime::RuntimeValueSnapshot::from_value(
                mech_runtime::RuntimeHostInputValue::F64(7.0)
                    .into_value()
                    .unwrap(),
            )
            .unwrap(),
        );
        assert_eq!(
            document.repl.session.submit("ans").unwrap().to_string(),
            "7",
            "subsequent clicks must replace ans without duplicate declarations",
        );
    }

    #[cfg(feature = "browser_compute")]
    #[test]
    fn accepted_generation_rebuild_preserves_source_and_resident_state() {
        let encoded = document_payload("document.mec", "~answer := 0\nanswer += 41\nanswer")
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        document.repl.session.submit("answer += 1").unwrap();
        let accepted_source = document.repl.session.source().to_owned();
        let generation = document.compute_generation();

        document
            .repl
            .session
            .rebuild_runtime_preserving_state()
            .unwrap();

        assert_eq!(document.repl.session.source(), accepted_source);
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap()
                .to_string(),
            "42",
        );
        assert_ne!(document.compute_generation(), generation);
    }

    #[test]
    fn selecting_a_large_document_value_is_runtime_local_until_ans_is_consumed() {
        let encoded = document_payload(
            "document.mec",
            include_str!("../../../examples/working/fizzbuzz.mec"),
        )
        .encode()
        .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let accepted_source = document.repl.session.source().to_string();
        let revision = document
            .runtime()
            .unwrap()
            .program_execution_info()
            .program_revision;
        let selected = document
            .repl
            .session
            .symbol("x")
            .unwrap()
            .expect("FizzBuzz x must be resident");

        document.repl.session.select_value("x", selected.clone());

        assert_eq!(document.repl.session.source(), accepted_source);
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .program_execution_info()
                .program_revision,
            revision,
            "selecting a document value must not compile or reactivate the program",
        );
        assert_eq!(
            document.repl.session.symbol("ans").unwrap(),
            Some(selected.clone()),
            "the selected snapshot must be immediately visible as ans",
        );
        let explicit_ans = document.repl.session.symbols(&["ans".to_string()]).unwrap();
        assert_eq!(
            explicit_ans,
            vec![("ans".to_string(), selected)],
            "explicit multi-symbol queries must inject pending ans before runtime lookup",
        );
        assert_eq!(
            document
                .repl
                .session
                .submit("ans[100]")
                .unwrap()
                .to_string(),
            "100",
            "the deferred ans expression must materialize on the next source submission",
        );
        assert_ne!(
            document
                .runtime()
                .unwrap()
                .program_execution_info()
                .program_revision,
            revision,
            "ordinary source submission must still compile the accepted overlay",
        );
    }

    fn assert_f64(value: mech_runtime::RuntimeValueSnapshot, expected: f64) {
        match value.value().data() {
            mech_core::ValueData::F64(value) => assert_eq!(value.to_f64(), expected),
            other => panic!("expected f64 value, got {other:?}"),
        }
    }

    #[test]
    fn multiple_module_roots_fail_closed_before_legacy_resolution() {
        let document = project_document(&["app/main.mec", "nested/main.mec"]);
        let sources = HashMap::from([
            (
                "app/main.mec".to_string(),
                "+> ./lib.mec\nanswer := lib/value + 1\nanswer\n".to_string(),
            ),
            (
                "app/lib.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
            (
                "nested/main.mec".to_string(),
                "+> ../shared/lib.mec\nparent-answer := lib/value + 1\n".to_string(),
            ),
            (
                "shared/lib.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
        ]);
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();

        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::MultipleRootsUnsupported,
        );
    }

    #[test]
    fn source_backed_document_resolves_relative_imports_from_its_root_specifier() {
        let source = "+> ./math.mec\n~answer := 0\nanswer += math/value + 1\nanswer\n";
        let source_map = HashMap::from([
            ("docs/main.mec".to_string(), source.to_string()),
            (
                "docs/math.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
        ]);

        let document = WasmDocument::try_from_bootstrap(document_bootstrap(
            "docs/main.mec",
            source,
            source_map,
            Vec::new(),
        ))
        .unwrap();

        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
    }

    #[test]
    fn project_and_document_resolvers_retain_nominal_provenance() {
        let origin =
            mech_core::CanonicalNominalPath::new(["test-package".to_string(), "main".to_string()])
                .unwrap();
        let sources = HashMap::from([(
            "main.mec".to_string(),
            "<event> := :idle | :busy\n".to_string(),
        )]);
        let provenance = HashMap::from([(
            "main.mec".to_string(),
            ServedSourceProvenance {
                nominal_origin: origin.clone(),
                nominal_package_id: Some("sha256:fixture".to_string()),
            },
        )]);
        let resolver =
            project_source_resolver_with_resolutions_and_provenance(&sources, &[], &provenance)
                .unwrap();
        let resolved =
            mech_runtime::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
                .unwrap()
                .unwrap();
        assert_eq!(resolved.nominal_origin.as_ref(), Some(&origin));
        assert_eq!(
            resolved.nominal_package_id.as_deref(),
            Some("sha256:fixture")
        );

        let mut bootstrap = document_bootstrap(
            "main.mec",
            "<event> := :idle | :busy\n",
            sources,
            Vec::new(),
        );
        bootstrap.provenance = provenance;
        let document = bootstrap.initial_document();
        let resolver = document_source_resolver(&document, &bootstrap).unwrap();
        let resolved =
            mech_runtime::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
                .unwrap()
                .unwrap();
        assert_eq!(resolved.nominal_origin.as_ref(), Some(&origin));
        assert_eq!(
            resolved.nominal_package_id.as_deref(),
            Some("sha256:fixture")
        );
    }

    #[test]
    fn runtime_capture_rewrite_retains_root_nominal_provenance() {
        let source = "<event> := :idle | :busy\nanswer := 42\nanswer\n";
        let origin =
            mech_core::CanonicalNominalPath::new(["test-package".to_string(), "main".to_string()])
                .unwrap();
        let mut bootstrap = document_bootstrap(
            "main.mec",
            source,
            HashMap::from([("main.mec".to_string(), source.to_string())]),
            Vec::new(),
        );
        bootstrap.provenance.insert(
            "main.mec".to_string(),
            ServedSourceProvenance {
                nominal_origin: origin.clone(),
                nominal_package_id: Some("sha256:fixture".to_string()),
            },
        );
        let candidate = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            format!("{source}\nnext := 1\n"),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap()
        .with_standalone_nominal_origin()
        .with_nominal_package_id("stale-editor-package");
        let (runtime_source, output) = runtime_document(&bootstrap, &candidate).unwrap();
        assert!(output.is_some());
        assert_eq!(runtime_source.nominal_origin(), Some(&origin));
        assert_eq!(runtime_source.nominal_package_id(), Some("sha256:fixture"));
    }

    #[cfg(all(feature = "enum", feature = "kind_define", feature = "variable_define"))]
    #[test]
    fn standalone_document_enum_owner_survives_staged_edits_and_reset() {
        let source = "<color> := :red | :green | :blue\nmy-color<color> := :red\n";
        let edited = source.replace("my-color<color> := :red", "my-color<color> := :blue");
        let encoded = document_payload("document.mec", source).encode().unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let original = document.repl.session.symbol("my-color").unwrap().unwrap();
        let origin = document
            .repl
            .session
            .source_document()
            .unwrap()
            .nominal_origin()
            .cloned();
        let replacement = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            edited,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        document.bootstrap.stage_document_base(replacement.clone());
        document.repl.session.replace_document(replacement).unwrap();
        document.refresh_document_output_ordinals(None).unwrap();
        assert_eq!(
            document.bootstrap.document_base().nominal_origin(),
            origin.as_ref()
        );
        let output_id = document.bootstrap.program_output_id().unwrap().unwrap();
        let edited = document
            .runtime()
            .unwrap()
            .output_value(output_id)
            .unwrap()
            .unwrap();
        assert_eq!(original.schema_key(), edited.schema_key());
        assert_ne!(original, edited);

        document.reset(&encoded).unwrap();
        assert_eq!(
            document.bootstrap.document_base().nominal_origin(),
            origin.as_ref()
        );
        assert_eq!(
            document.repl.session.symbol("my-color").unwrap().unwrap(),
            original
        );
        let independent = WasmDocument::from_encoded(&encoded).unwrap();
        assert_ne!(
            independent
                .repl
                .session
                .symbol("my-color")
                .unwrap()
                .unwrap()
                .schema_key(),
            original.schema_key()
        );
    }

    #[cfg(all(feature = "enum", feature = "kind_define"))]
    #[test]
    fn root_provenance_without_package_id_replaces_stale_metadata_without_outputs() {
        let origin =
            mech_core::CanonicalNominalPath::new(["package".to_owned(), "events".to_owned()])
                .unwrap();
        for source in ["", "<event> := :idle | :busy\n"] {
            let mut bootstrap = document_bootstrap("main.mec", source, HashMap::new(), Vec::new());
            bootstrap.provenance.insert(
                "main.mec".to_owned(),
                ServedSourceProvenance {
                    nominal_origin: origin.clone(),
                    nominal_package_id: None,
                },
            );
            let candidate = bootstrap
                .document
                .document()
                .clone()
                .with_nominal_package_id("stale-editor-package");
            let (runtime_source, output) = runtime_document(&bootstrap, &candidate).unwrap();
            assert!(output.is_none());
            assert_eq!(runtime_source.nominal_origin(), Some(&origin));
            assert_eq!(runtime_source.nominal_package_id(), None);
            let resolver = document_source_resolver(&candidate, &bootstrap).unwrap();
            let resolved =
                mech_runtime::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
                    .unwrap()
                    .unwrap();
            assert_eq!(resolved.nominal_origin.as_ref(), Some(&origin));
            assert_eq!(resolved.nominal_package_id, None);
            assert_eq!(
                resolved.source_document().unwrap().nominal_package_id(),
                None
            );
        }
    }

    #[cfg(all(
        feature = "enum",
        feature = "kind_define",
        feature = "variable_define",
        feature = "state_machines",
    ))]
    #[test]
    fn standalone_bundle_document_reload_and_edit_preserve_enum_and_fsm_identity() {
        for source in [
            "<color> := :red | :blue\nmy-color<color> := :red\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n",
        ] {
            let retained = SourceDocument::parse_resolved(
                "bundle:///document.mec",
                mech_syntax::document::Revision(0),
                source,
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap()
            .with_standalone_nominal_origin();
            let mut compiler = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build_compiler()
                .unwrap();
            let product = compiler.compile_document(&retained).unwrap();
            let bundle =
                CanonicalProgramBundle::from_product("bundle:///document.mec", &retained, &product)
                    .unwrap();
            let encoded = bundle.encode().unwrap();
            let source_map = HashMap::from([("document.mec".to_owned(), source.to_owned())]);
            let decoded =
                decode_document_bundle(&encoded, "document.mec", &source_map, &HashMap::new())
                    .unwrap();
            let edited_source = source
                .replace("my-color<color> := :red", "my-color<color> := :blue")
                .replace("=> 1.0.", "=> 2.0.");
            assert!(
                decode_document_bundle(
                    &encoded,
                    "document.mec",
                    &HashMap::from([("document.mec".to_owned(), edited_source.clone())]),
                    &HashMap::new(),
                )
                .is_err()
            );
            let mismatched = HashMap::from([(
                "document.mec".to_owned(),
                ServedSourceProvenance {
                    nominal_origin: SourceDocument::new_standalone_origin(),
                    nominal_package_id: None,
                },
            )]);
            assert!(
                decode_document_bundle(&encoded, "document.mec", &source_map, &mismatched,)
                    .is_err()
            );

            let mut bootstrap = document_bootstrap("document.mec", source, source_map, Vec::new());
            bootstrap.document =
                retain_browser_root_provenance(bootstrap.document, None, Some(&decoded));
            bootstrap.document_base.borrow_mut().active = bootstrap.document.document().clone();
            bootstrap.initial_bundle = Some(decoded);
            let mut document = WasmDocument::try_from_bootstrap(bootstrap).unwrap();
            assert_eq!(
                document
                    .repl
                    .session
                    .source_document()
                    .unwrap()
                    .nominal_origin(),
                retained.nominal_origin(),
            );
            let replacement = SourceDocument::parse_resolved(
                "runtime:interactive",
                mech_syntax::document::Revision(1),
                edited_source,
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap();
            document.bootstrap.stage_document_base(replacement.clone());
            document.repl.session.replace_document(replacement).unwrap();
            let edited_product = compiler
                .compile_document(document.repl.session.source_document().unwrap())
                .unwrap();
            let enum_keys = |artifact: &mech_engine::ProgramArtifact| {
                artifact
                    .schemas()
                    .entries()
                    .filter_map(|entry| {
                        matches!(entry.schema().body(), mech_core::SchemaBody::Enum { .. })
                            .then_some(entry.key())
                    })
                    .collect::<Vec<_>>()
            };
            let original_keys = enum_keys(product.artifact());
            assert!(!original_keys.is_empty());
            assert_eq!(enum_keys(edited_product.artifact()), original_keys);
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                retained.nominal_origin(),
            );
        }
    }

    #[cfg(all(feature = "enum", feature = "kind_define", feature = "variable_define"))]
    #[test]
    fn browser_root_admission_restores_bundle_origin_and_prefers_resolver_pair() {
        let source = "<color> := :red | :blue\nmy-color<color> := :red\n";
        let retained = SourceDocument::parse_resolved(
            "bundle:///document.mec",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap()
        .with_standalone_nominal_origin();
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()
            .unwrap();
        let product = compiler.compile_document(&retained).unwrap();
        let bundle =
            CanonicalProgramBundle::from_product("bundle:///document.mec", &retained, &product)
                .unwrap();
        let unowned = || {
            CanonicalWasmDocument::retain(
                "runtime:interactive",
                mech_syntax::document::Revision(0),
                source,
            )
            .unwrap()
        };
        let restored = retain_browser_root_provenance(unowned(), None, Some(&bundle));
        assert_eq!(
            restored.document().nominal_origin(),
            retained.nominal_origin()
        );
        let restored_product = compiler.compile_document(restored.document()).unwrap();
        assert_eq!(
            restored_product
                .artifact()
                .schemas()
                .get(restored_product.artifact().outputs()[0].schema)
                .unwrap()
                .body(),
            product
                .artifact()
                .schemas()
                .get(product.artifact().outputs()[0].schema)
                .unwrap()
                .body()
        );
        let configured = ServedSourceProvenance {
            nominal_origin: mech_core::CanonicalNominalPath::new([
                "package".to_owned(),
                "colors".to_owned(),
            ])
            .unwrap(),
            nominal_package_id: None,
        };
        let stale = CanonicalWasmDocument::from_document(
            unowned()
                .document()
                .clone()
                .with_nominal_package_id("stale-editor-package"),
        );
        let admitted = retain_browser_root_provenance(stale, Some(&configured), Some(&bundle));
        assert_eq!(
            admitted.document().nominal_origin(),
            Some(&configured.nominal_origin)
        );
        assert_eq!(admitted.document().nominal_package_id(), None);
    }

    #[test]
    fn source_backed_document_rejects_a_stale_payload() {
        let decoded_source = "+> ./math.mec\nanswer := math/value + 1\nanswer\n";
        let source_map = HashMap::from([
            (
                "docs/main.mec".to_string(),
                "answer := 999\nanswer\n".to_string(),
            ),
            (
                "docs/math.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
        ]);

        assert!(
            WasmDocument::from_payload_with_sources(
                document_payload("docs/main.mec", decoded_source),
                "docs/main.mec",
                source_map,
                Vec::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn source_backed_document_preserves_explicit_resolution_edges_across_reset() {
        let source = "+> ./math.mec\n~answer := 0\nanswer += math/value + 1\nanswer\n";
        let encoded = document_payload("bundle/000000.mec", source)
            .encode()
            .unwrap();
        let source_map = HashMap::from([
            ("bundle/000000.mec".to_string(), source.to_string()),
            (
                "bundle/000001.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
        ]);
        let resolutions = vec![SourceResolutionEntry::new(
            "bundle/000000.mec",
            "./math.mec",
            "bundle/000001.mec",
        )];

        let mut document = WasmDocument::from_payload_with_sources(
            document_payload("bundle/000000.mec", source),
            "bundle/000000.mec",
            source_map,
            resolutions,
        )
        .unwrap();
        document.repl.inherit_host_request_generation(41);
        document.repl.inherit_step_request_generation(73);
        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );

        document.reset(&encoded).unwrap();
        assert_eq!(document.repl.host_request_generation(), 41);
        assert_eq!(document.repl.step_request_generation(), 73);
        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
    }

    #[test]
    fn detached_session_reset_reactivates_its_complete_source_baseline() {
        let encoded = document_payload("document.mec", "~answer := 0\nanswer += 42\nanswer")
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        document.repl.session.submit("answer += 1\nanswer").unwrap();
        assert_f64(
            document.repl.session.symbol("answer").unwrap().unwrap(),
            43.0,
        );

        document.repl.session.reset().unwrap();

        assert!(document.repl.session.source().contains("answer"));
        assert_f64(
            document.repl.session.symbol("answer").unwrap().unwrap(),
            42.0,
        );
        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
    }

    #[test]
    fn document_repl_clear_owns_the_complete_workspace_source() {
        let encoded = document_payload("document.mec", "x := 1\ny := 2\ny")
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();

        document
            .repl
            .session
            .clear_variables(&["x".to_string()])
            .unwrap();
        let symbols = document.repl.session.symbols(&[]).unwrap();
        assert!(!symbols.iter().any(|(name, _)| name == "x"));
        assert!(symbols.iter().any(|(name, _)| name == "y"));

        document.repl.session.clear_variables(&[]).unwrap();
        assert!(document.repl.session.source().is_empty());
        assert!(document.repl.session.symbols(&[]).unwrap().is_empty());
        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::None,
        );
    }

    #[test]
    fn document_repl_formats_only_complete_mech_source_entries() {
        let encoded = document_payload("document.mec", "baseline := 1")
            .encode()
            .unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        let formatted = document
            .repl_format_source("answer:=1+1")
            .expect("valid source should format");
        assert!(formatted.contains("mech-code-block"));
        assert!(formatted.contains("mech-variable-define"));
        let suppressed = document
            .repl_format_source("answer + 2; -- suppress this value")
            .expect("a suppressed entry is complete Mech source");
        let terminator = suppressed.find("mech-code-terminal").unwrap();
        let comment = suppressed.find("mech-comment").unwrap();
        assert!(terminator < comment, "{suppressed}");
        assert!(document.repl_format_source("answer := (").is_none());
        assert!(document.repl_format_source(":help").is_none());
    }

    #[test]
    fn appended_document_fragments_follow_the_canonical_program_capture() {
        let baseline = "Baseline {1}.\n\nanswer := 1\nanswer\n";
        let fragment = "\nDocumentation {2}.\n";
        let bootstrap = document_bootstrap("document.mec", baseline, HashMap::new(), Vec::new());
        let candidate = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            format!("{baseline}{fragment}"),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let (runtime_document, program_output) = runtime_document(&bootstrap, &candidate).unwrap();
        let program_output = program_output.expect("the original result is captured");
        let program = CanonicalSourceFrontend
            .compile_document(&runtime_document.document())
            .unwrap();
        assert!(program.document_outputs().iter().any(|output| {
            output.kind == SourceDocumentOutputKind::Inline && output.output > program_output.get()
        }));
    }

    #[test]
    fn appended_document_fragment_mounts_its_live_outputs() {
        let baseline = "answer := 1\nanswer\n";
        let fragment = "\nResult {answer + 1}.\n\n```mech\nanswer + 2\n```\n";
        let bootstrap = document_bootstrap("document.mec", baseline, HashMap::new(), Vec::new());
        let candidate = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(1),
            format!("{baseline}{fragment}"),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let (runtime, _) = activate_document_repl_runtime_document(
            &bootstrap,
            MechEventBuffer::default(),
            &candidate,
        )
        .unwrap();
        let addresses =
            live_document_fragment_addresses(&candidate, &runtime, fragment, baseline.len())
                .unwrap();
        assert_eq!(addresses.len(), 2);
        let parsed = SourceDocument::parse_resolved(
            "browser:documentation:test",
            mech_syntax::document::Revision(0),
            fragment,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let html = mech_runtime::CanonicalDocumentRenderer
            .format_html_body_live(&parsed.document(), &addresses)
            .unwrap();
        assert!(html.contains("class='mech-inline-mech-code'"), "{html}");
        assert!(html.contains("class='mech-block-output'"), "{html}");
    }

    #[test]
    fn fixed_program_capture_preserves_trailing_comment_output_order() {
        let source = "answer := 40 + 2\n\nThe next value is {answer + 1}.\n\nanswer\n";
        let bootstrap = document_bootstrap("document.mec", source, HashMap::new(), Vec::new());
        let (runtime_source, program_output) =
            runtime_document(&bootstrap, bootstrap.document.document()).unwrap();
        let program_output = program_output.expect("the final statement is captured");
        let program = CanonicalSourceFrontend
            .compile_document(&runtime_source.document())
            .unwrap();
        assert!(program.document_outputs().iter().any(|output| {
            output.kind == SourceDocumentOutputKind::Inline && output.output < program_output.get()
        }));

        let encoded = document_payload("document.mec", source).encode().unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();
        let output = document
            .bootstrap
            .program_output_id()
            .unwrap()
            .expect("the fenced statement is the implicit document output");
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .output_value(output)
                .unwrap()
                .unwrap()
                .to_string(),
            "42",
        );
    }

    #[test]
    fn internal_repl_console_avoids_every_configured_host_namespace() {
        let hosts = vec![
            HostInstanceConfig {
                name: "repl".to_string(),
                provider: "console".to_string(),
                settings: ConfigValue::Map(Default::default()),
            },
            HostInstanceConfig {
                name: "repl-console".to_string(),
                provider: "scene".to_string(),
                settings: ConfigValue::Map(Default::default()),
            },
        ];
        assert_eq!(internal_repl_console_instance(&hosts), "repl-console-2");
    }

    #[test]
    fn project_sources_report_missing_module_dependencies() {
        let document = project_document(&["main.mec"]);
        let sources = HashMap::from([(
            "main.mec".to_string(),
            "+> ./missing.mec\nanswer := 1\n".to_string(),
        )]);
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();

        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert!(
            error
                .kind_as::<mech_runtime::RuntimeModuleDependencyMissingError>()
                .is_some(),
            "{error:?}",
        );
    }

    #[test]
    fn configured_multiple_roots_fail_closed_without_reading_unused_sources() {
        let document = project_document(&["first.mec", "second.mec"]);
        let sources = HashMap::from([
            ("first.mec".to_string(), "marker := 1\n".to_string()),
            (
                "second.mec".to_string(),
                "answer := marker + 1\n".to_string(),
            ),
            (
                "unused.mec".to_string(),
                "this is not valid Mech\n".to_string(),
            ),
        ]);
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();

        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::MultipleRootsUnsupported,
        );
    }

    #[cfg(feature = "served_project_authority")]
    fn authority_config(
        hosts: Vec<mech_runtime::HostInstanceConfig>,
        grants: Vec<mech_runtime::RunResourceGrantConfig>,
    ) -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts,
            run_grants: grants,
        }
    }

    #[cfg(feature = "served_project_authority")]
    fn host(name: &str, provider: &str) -> mech_runtime::HostInstanceConfig {
        mech_runtime::HostInstanceConfig {
            name: name.to_string(),
            provider: provider.to_string(),
            settings: mech_runtime::ConfigValue::Map(Default::default()),
        }
    }

    #[cfg(feature = "served_project_authority")]
    fn grant(
        target: &str,
        operations: &[&str],
        paths: &[&str],
    ) -> mech_runtime::RunResourceGrantConfig {
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
            &format!(
                r#"config := {{
  hosts: [{{ name: "view" provider: "scene" settings: {{}} }}]
  run: {{
    paths: ["main.mec"]
    grants: [{{ target: "view/frame" operations: ["{operation}"] paths: ["{path}"] }}]
  }}
}}"#
            ),
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn split_grants_for_one_target_authorize_project_request() {
        let doc = document_with_grant("replace", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![
                grant("view/frame", &["read"], &["replace"]),
                grant("view/frame", &["write"], &["replace"]),
            ],
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
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![grant("view/frame", &["read"], &["replace"])],
        );
        assert!(validate_served_authority(&doc, &authority).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn broader_path_request_is_rejected() {
        let doc = document_with_grant("hands/*", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![grant("view/frame", &["write"], &["hands/second"])],
        );
        assert!(validate_served_authority(&doc, &authority).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn crossed_operation_and_path_grants_are_rejected() {
        let doc = document_with_grant("secret/file", "write");
        let authority = authority_config(
            vec![host("view", "scene")],
            vec![
                grant("view/frame", &["write"], &["public/*"]),
                grant("view/frame", &["read"], &["secret/*"]),
            ],
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

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn analog_clock_served_authority_is_accepted() {
        let document = parse_config_document(
            "examples/analog-clock/mech.mcfg",
            include_str!("../../../examples/analog-clock/mech.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let authority = authority_config(
            document.hosts.clone(),
            document.run.as_ref().unwrap().grants.clone(),
        );

        validate_served_authority(&document, &authority).unwrap();
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn served_pointer_host_plans_and_delivers_live_input() {
        let config_source = r#"config := {
  hosts: [{ name: "mouse" provider: "pointer" settings: {} }]
  run: {
    paths: ["main.mec"]
    grants: [{ target: "mouse/frame" operations: ["read"] paths: ["pulse", "position", "pressed", "delta-seconds"] }]
  }
}"#;
        let source =
            "@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\npulse\n";
        let config =
            parse_config_document("mech.mcfg", config_source, ConfigProfileOptions::default())
                .unwrap();
        let mut bootstrap = document_bootstrap("main.mec", source, HashMap::new(), Vec::new());
        bootstrap.console_instance = internal_repl_console_instance(&config.hosts);
        bootstrap.served = Some(ServedDocumentBootstrap {
            config_source: config_source.to_owned(),
            authority: authority_config(
                config.hosts.clone(),
                config.run.as_ref().unwrap().grants.clone(),
            ),
        });
        let mut document = WasmDocument::try_from_bootstrap(bootstrap).unwrap();
        assert!(!document.has_pointer_input());
        document.start().unwrap();
        assert!(document.has_pointer_input());
        document.pointer_input(0.5, -0.25, true, 0.016).unwrap();
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .pending_host_input_count()
                .unwrap(),
            1
        );
        document.repl.session.drain_pending_inputs(1).unwrap();
        let pulse = document
            .runtime()
            .unwrap()
            .root_symbol_values(&["pulse"])
            .unwrap()
            .pop()
            .unwrap()
            .1;
        assert_eq!(pulse.format_canonical_inline(), "1");
        document.stop().unwrap();
        assert!(!document.has_pointer_input());
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn served_pointer_liveness_tracks_accepted_source_replacement() {
        let config_source = r#"config := {
  hosts: [{ name: "mouse" provider: "pointer" settings: {} }]
  run: {
    paths: ["main.mec"]
    grants: [{ target: "mouse/frame" operations: ["read"] paths: ["pulse", "position", "pressed", "delta-seconds"] }]
  }
}"#;
        let config =
            parse_config_document("mech.mcfg", config_source, ConfigProfileOptions::default())
                .unwrap();
        let mut bootstrap = document_bootstrap(
            "main.mec",
            "answer := 42\nanswer\n",
            HashMap::new(),
            Vec::new(),
        );
        bootstrap.console_instance = internal_repl_console_instance(&config.hosts);
        bootstrap.served = Some(ServedDocumentBootstrap {
            config_source: config_source.to_owned(),
            authority: authority_config(
                config.hosts.clone(),
                config.run.as_ref().unwrap().grants.clone(),
            ),
        });
        let mut document = WasmDocument::try_from_bootstrap(bootstrap).unwrap();
        document.start().unwrap();
        assert!(
            !document.has_pointer_input(),
            "configuration alone does not start a driver"
        );
        let source =
            "@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\npulse\n";
        let active = document_payload("main.mec", source).encode().unwrap();
        document.reset(&active).unwrap();
        assert!(
            document.has_pointer_input(),
            "accepted live read starts its driver"
        );
        let previous = document.bootstrap.source().lifecycle.pointer().unwrap();
        document.pointer_input(0.25, -0.5, false, 0.016).unwrap();
        assert_eq!(
            document
                .runtime()
                .unwrap()
                .pending_host_input_count()
                .unwrap(),
            1
        );
        document.repl.session.drain_pending_inputs(1).unwrap();
        let inactive = document_payload("main.mec", "answer := 43\nanswer\n")
            .encode()
            .unwrap();
        document.reset(&inactive).unwrap();
        assert!(
            !document.has_pointer_input(),
            "removing the last read stops pointer ingress"
        );
        assert!(
            !previous.is_running(),
            "replacement retires the previous input driver"
        );
        document.reset(&active).unwrap();
        assert!(
            document.has_pointer_input(),
            "a valid replacement can reactivate ingress"
        );
        document.stop().unwrap();
        assert!(!document.has_pointer_input());
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn static_project_retains_the_pointer_handle_used_by_its_driver() {
        let config_source = r#"config := {
  hosts: [{ name: "mouse" provider: "pointer" settings: {} }]
  run: {
    paths: ["main.mec"]
    grants: [{ target: "mouse/frame" operations: ["read"] paths: ["pulse", "position", "pressed", "delta-seconds"] }]
  }
}"#;
        let document =
            parse_config_document("mech.mcfg", config_source, ConfigProfileOptions::default())
                .unwrap();
        let sources = HashMap::from([(
            "main.mec".to_owned(),
            "@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\npulse\n"
                .to_owned(),
        )]);
        let pointer = configured_project_pointer(&document).unwrap().unwrap();
        assert!(pointer.submit(0.5, -0.25, true, 0.016).is_err());
        let authority = authority_config(
            document.hosts.clone(),
            document.run.as_ref().unwrap().grants.clone(),
        );
        let scenes = BrowserSceneRegistry::new();
        let mut runtime = build_runtime_from_authority(
            &document,
            &authority,
            project_source_resolver(&sources).unwrap(),
            scenes.clone(),
            Some(pointer.clone()),
        )
        .unwrap();
        run_project_sources(&mut runtime, &document).unwrap();
        let mut project = WasmProject::from_runtime(runtime, scenes, Some(pointer.clone()));
        assert!(!project.has_pointer_input());
        project.start().unwrap();
        assert!(project.has_pointer_input());
        project.pointer_input(0.5, -0.25, true, 0.016).unwrap();
        assert_eq!(project.runtime.pending_host_input_count().unwrap(), 1);
        project.runtime.drain_host_inputs(1).unwrap();
        assert_eq!(
            project
                .runtime
                .root_symbol_value("pulse")
                .unwrap()
                .format_canonical_inline(),
            "1"
        );
        project.stop().unwrap();
        assert!(!project.has_pointer_input());
        assert!(pointer.submit(0.5, -0.25, true, 0.016).is_err());
        let mut duplicate = document.clone();
        duplicate.hosts.push(HostInstanceConfig {
            name: "second".into(),
            provider: "pointer".into(),
            settings: ConfigValue::Map(Default::default()),
        });
        assert!(configured_project_pointer(&duplicate).is_err());
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn static_source_admission_uses_the_compiled_scalar_closure() {
        let document = parse_config_document(
            "mech.mcfg",
            "config := { run: { paths: [\"main.mec\"] } }",
            ConfigProfileOptions::default(),
        )
        .unwrap();
        for source in [
            "value := 7u8\n",
            "value := -7i32\n",
            "value<u8> := 7\n",
            "value<i32> := -7\n",
            "value := 7u8 + 2u8\n",
            "value := 7i32 + 2i32\n",
            "value := 7\n",
            "+> math\nvalue := math/fmod(5.3, 2.0)\n",
        ] {
            let sources = HashMap::from([("main.mec".to_owned(), source.to_owned())]);
            let accepted = project_source_resolver(&sources)
                .and_then(|resolver| {
                    let mut runtime = browser_runtime_builder()
                        .source_resolver(resolver)
                        .build()?;
                    run_project_sources(&mut runtime, &document)
                })
                .is_ok();
            assert_eq!(
                validate_static_project_sources(&document, &sources, &[], &HashMap::new()).is_ok(),
                accepted,
                "{source}"
            );
        }
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn static_compute_admission_and_loading_share_mixed_preparation() {
        let source = "@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\n@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- pulse\nanswer := @compute/sample/result\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let sources = HashMap::from([("main.mec".to_owned(), source.to_owned())]);
        for backend in ["cpu", "auto", "wgpu"] {
            let config = format!(
                r#"config := {{
  hosts: [{{ name: "filters" provider: "compute" settings: {{ region: "calculation" backend: "{backend}" }} }} {{ name: "mouse" provider: "pointer" settings: {{}} }}]
  run: {{ paths: ["main.mec"] grants: [{{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] }} {{ target: "mouse/frame" operations: ["read"] paths: ["pulse"] }}] }}
}}"#
            );
            let document =
                parse_config_document("mech.mcfg", &config, ConfigProfileOptions::default())
                    .unwrap();
            // Negative control reproduces the reviewed ordinary-loader failure.
            // No device or program is activated by either admission path.
            let mut ordinary = runtime_builder_with_factories(
                None,
                #[cfg(feature = "browser_host_scene")]
                BrowserSceneRegistry::new(),
                None,
            )
            .unwrap();
            for host in &document.hosts {
                ordinary = ordinary.host_instance(host.clone());
            }
            let missing_compute = ordinary
                .build_compiler()
                .err()
                .expect("the ordinary loader has no prepared compute factory");
            assert_eq!(missing_compute.kind_name(), "RuntimeHostProviderNotFound");
            assert_eq!(
                missing_compute.kind_message(),
                "host provider `compute` is not registered"
            );
            validate_static_project_sources(&document, &sources, &[], &HashMap::new()).unwrap();
            if backend != "cpu" {
                continue;
            }
            let authority = authority_config(
                document.hosts.clone(),
                document.run.as_ref().unwrap().grants.clone(),
            );
            let resolver = project_source_resolver(&sources).unwrap();
            let prepared =
                prepare_project_compute_region(&document, resolver.clone(), Some(&authority))
                    .unwrap()
                    .unwrap();
            let factory = prepare_browser_compute_admission_factory(&document, &prepared).unwrap();
            let pointer = configured_project_pointer(&document).unwrap().unwrap();
            let mut runtime = project_runtime_builder(
                &document,
                resolver,
                Some(&authority),
                false,
                Some(factory),
                #[cfg(feature = "browser_host_scene")]
                BrowserSceneRegistry::new(),
                Some(pointer.clone()),
            )
            .unwrap()
            .build()
            .unwrap();
            assert_eq!(runtime.program_execution_info().resident_accepted_turns, 0);
            let durability = runtime.config().resident_durability;
            runtime
                .load_compiled_program(prepared.coordinator, durability)
                .unwrap();
            // A driver-backed sample observation is dormant on installation.
            // Only the admitted pointer trigger may advance this program;
            // neither preparation nor a manual step may fabricate that input.
            assert_eq!(
                runtime
                    .root_symbol_value("answer")
                    .unwrap()
                    .format_canonical_inline(),
                "0"
            );
            assert_eq!(runtime.program_execution_info().resident_accepted_turns, 0);
            assert!(runtime.step_active_program().is_err());
            runtime.start_input_drivers().unwrap();
            pointer.submit(0.25, -0.5, false, 0.016).unwrap();
            for _ in 0..4 {
                let pending = runtime.pending_host_input_count().unwrap();
                if pending == 0 {
                    break;
                }
                runtime.drain_host_inputs(pending).unwrap();
            }
            assert_eq!(
                runtime
                    .root_symbol_value("answer")
                    .unwrap()
                    .format_canonical_inline(),
                "1"
            );
            assert_eq!(
                runtime.program_route(),
                RuntimeProgramRoute::ResidentExternal
            );
            runtime.shutdown().unwrap();
        }
        for (backend, candidate) in [
            ("cpu-jit", source.to_owned()),
            ("cpu", source.replace("sample/result", "sample/unknown")),
        ] {
            let config = format!(
                r#"config := {{
  hosts: [{{ name: "filters" provider: "compute" settings: {{ region: "calculation" backend: "{backend}" }} }} {{ name: "mouse" provider: "pointer" settings: {{}} }}]
  run: {{ paths: ["main.mec"] grants: [{{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "sample/unknown", "turn"] }} {{ target: "mouse/frame" operations: ["read"] paths: ["pulse"] }}] }}
}}"#
            );
            let document =
                parse_config_document("mech.mcfg", &config, ConfigProfileOptions::default())
                    .unwrap();
            let candidates = HashMap::from([("main.mec".to_owned(), candidate)]);
            let declined = validate_static_project_sources(
                &document,
                &candidates,
                &[],
                &HashMap::new(),
            )
            .expect_err(
                "unsupported backends and undeclared sample paths must decline during admission",
            );
            if backend == "cpu-jit" {
                assert_eq!(declined.kind_name(), "ComputeBackendSelection");
                assert!(declined.kind_message().contains("cpu-jit"));
            }
        }
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn served_compute_preparation_uses_issued_backend_settings() {
        let source = "@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- 1\nanswer := @compute/sample/result\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let sources = HashMap::from([("main.mec".to_owned(), source.to_owned())]);
        for (requested, issued, expected) in [
            ("cpu", "wgpu", "wgpu"),
            ("wgpu", "cpu", "cpu-scalar"),
            ("auto", "cpu", "cpu-scalar"),
        ] {
            let config = format!(
                r#"config := {{
  hosts: [{{ name: "filters" provider: "compute" settings: {{ region: "calculation" backend: "{requested}" }} }}]
  run: {{ paths: ["main.mec"] grants: [{{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] }}] }}
}}"#
            );
            let document =
                parse_config_document("mech.mcfg", &config, ConfigProfileOptions::default())
                    .unwrap();
            let mut authority = authority_config(
                document.hosts.clone(),
                document.run.as_ref().unwrap().grants.clone(),
            );
            let ConfigValue::Map(settings) = &mut authority.hosts[0].settings else {
                panic!("fixture settings must be a map")
            };
            settings.insert("backend".to_owned(), ConfigValue::String(issued.to_owned()));
            // Settings overrides are valid served authority, not a source
            // mismatch to reject or a permission to broaden source grants.
            validate_served_authority(&document, &authority).unwrap();
            let effective = compute_document_with_issued_settings(&document, &authority).unwrap();
            assert_eq!(effective.hosts[0].name, document.hosts[0].name);
            assert_eq!(effective.hosts[0].provider, document.hosts[0].provider);
            assert_eq!(
                effective.run.as_ref().unwrap().paths,
                document.run.as_ref().unwrap().paths
            );
            assert_eq!(
                effective.run.as_ref().unwrap().grants,
                document.run.as_ref().unwrap().grants
            );
            let resolver = project_source_resolver(&sources).unwrap();
            let prepared = prepare_project_compute_region(&effective, resolver, Some(&authority))
                .unwrap()
                .unwrap();
            let factory = prepare_browser_compute_admission_factory(&effective, &prepared).unwrap();
            assert_eq!(
                factory
                    .resolved_backend_id(&effective.hosts[0].settings)
                    .unwrap()
                    .as_str(),
                expected
            );
            assert!(
                factory
                    .state_snapshot_handle()
                    .snapshot_retained(&prepared.retained_outputs)
                    .unwrap()
                    .is_none(),
                "pure preparation must not activate a compute session"
            );
            // Negative control uses the original source-only preparation owner:
            // it resolves the wrong backend for these legitimate overrides.
            let original = prepare_browser_compute_admission_factory(&document, &prepared).unwrap();
            assert_ne!(
                original
                    .resolved_backend_id(&document.hosts[0].settings)
                    .unwrap()
                    .as_str(),
                expected
            );
            let mut mismatched = authority.clone();
            let ConfigValue::Map(settings) = &mut mismatched.hosts[0].settings else {
                unreachable!()
            };
            settings.insert(
                "region".to_owned(),
                ConfigValue::String("not-the-source-region".to_owned()),
            );
            let invalid = compute_document_with_issued_settings(&document, &mismatched).unwrap();
            assert_eq!(
                prepare_browser_compute_admission_factory(&invalid, &prepared)
                    .err()
                    .unwrap()
                    .kind_name(),
                "ComputeHostConfiguration",
                "issued settings do not replace compiled region identity"
            );
        }
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_compute"))]
    #[test]
    fn static_compute_keeps_retained_import_and_nominal_authority() {
        let config = r#"config := {
  hosts: [{ name: "filters" provider: "compute" settings: { region: "calculation" backend: "cpu" } } { name: "mouse" provider: "pointer" settings: {} }]
  run: { paths: ["main.mec"] grants: [{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] } { target: "mouse/frame" operations: ["read"] paths: ["pulse"] }] }
}"#;
        let document =
            parse_config_document("mech.mcfg", config, ConfigProfileOptions::default()).unwrap();
        let source = "+> ./seed\n@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\n@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- pulse\nanswer := @compute/sample/result\nadjusted := answer + seed/value\nadjusted\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let sources = HashMap::from([
            ("main.mec".to_owned(), source.to_owned()),
            (
                "helpers/index.mec".to_owned(),
                "<event> := :idle | :busy\nvalue := 2f32\n<+ value\n".to_owned(),
            ),
        ]);
        let resolutions = [SourceResolutionEntry {
            referrer: "main.mec".to_owned(),
            specifier: "./seed".to_owned(),
            target: "helpers/index.mec".to_owned(),
        }];
        let provenance = HashMap::from([(
            "helpers/index.mec".to_owned(),
            ServedSourceProvenance {
                nominal_origin: mech_core::CanonicalNominalPath::new([
                    "static-compute-fixture".to_owned(),
                    "helper".to_owned(),
                ])
                .unwrap(),
                nominal_package_id: Some("sha256:static-compute-fixture".to_owned()),
            },
        )]);
        assert!(
            validate_static_project_sources(&document, &sources, &resolutions, &HashMap::new())
                .is_err()
        );
        assert!(validate_static_project_sources(&document, &sources, &[], &provenance).is_err());
        validate_static_project_sources(&document, &sources, &resolutions, &provenance).unwrap();
        let authority = authority_config(
            document.hosts.clone(),
            document.run.as_ref().unwrap().grants.clone(),
        );
        let resolver = project_source_resolver_with_resolutions_and_provenance(
            &sources,
            &resolutions,
            &provenance,
        )
        .unwrap();
        let prepared =
            prepare_project_compute_region(&document, resolver.clone(), Some(&authority))
                .unwrap()
                .unwrap();
        let factory = prepare_browser_compute_admission_factory(&document, &prepared).unwrap();
        let pointer = configured_project_pointer(&document).unwrap().unwrap();
        let mut runtime = project_runtime_builder(
            &document,
            resolver,
            Some(&authority),
            false,
            Some(factory),
            #[cfg(feature = "browser_host_scene")]
            BrowserSceneRegistry::new(),
            Some(pointer.clone()),
        )
        .unwrap()
        .build()
        .unwrap();
        let durability = runtime.config().resident_durability;
        runtime
            .load_compiled_program(prepared.coordinator, durability)
            .unwrap();
        assert_eq!(
            runtime
                .root_symbol_value("adjusted")
                .unwrap()
                .format_canonical_inline(),
            "0"
        );
        assert!(runtime.step_active_program().is_err());
        runtime.start_input_drivers().unwrap();
        pointer.submit(0.25, -0.5, false, 0.016).unwrap();
        for _ in 0..4 {
            let pending = runtime.pending_host_input_count().unwrap();
            if pending == 0 {
                break;
            }
            runtime.drain_host_inputs(pending).unwrap();
        }
        assert_eq!(
            runtime
                .root_symbol_value("adjusted")
                .unwrap()
                .format_canonical_inline(),
            "3"
        );
        runtime.shutdown().unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    #[test]
    fn old_static_nominal_sources_require_regeneration_instead_of_new_identity() {
        let sources = HashMap::from([(
            "main.mec".to_owned(),
            "<event> := :idle | :busy\n".to_owned(),
        )]);
        let error = validate_static_nominal_provenance(&sources, &HashMap::new()).unwrap_err();
        assert!(error.kind_message().contains("missing nominal provenance"));
        let provenance = HashMap::from([(
            "main.mec".to_owned(),
            ServedSourceProvenance {
                nominal_origin: mech_core::CanonicalNominalPath::new([
                    "test-package".to_owned(),
                    "main".to_owned(),
                ])
                .unwrap(),
                nominal_package_id: Some("sha256:fixture".to_owned()),
            },
        )]);
        validate_static_nominal_provenance(&sources, &provenance).unwrap();
    }

    #[cfg(all(feature = "served_project_authority", feature = "state_machines"))]
    #[test]
    fn static_fsm_sources_require_retained_nominal_provenance() {
        let source = "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n";
        let sources = HashMap::from([("main.mec".to_owned(), source.to_owned())]);
        let error = validate_static_nominal_provenance(&sources, &HashMap::new()).unwrap_err();
        assert!(error.kind_message().contains("missing nominal provenance"));
        let origin = SourceDocument::new_standalone_origin();
        let provenance = HashMap::from([(
            "main.mec".to_owned(),
            ServedSourceProvenance {
                nominal_origin: origin.clone(),
                nominal_package_id: None,
            },
        )]);
        validate_static_nominal_provenance(&sources, &provenance).unwrap();
        let resolver = project_source_resolver_with_provenance(&sources, &provenance).unwrap();
        let resolved =
            mech_runtime::SourceResolver::resolve(&resolver, &SourceRequest::new("main.mec"))
                .unwrap()
                .unwrap();
        assert_eq!(
            resolved.source_document().unwrap().nominal_origin(),
            Some(&origin)
        );
    }

    #[cfg(all(feature = "served_project_authority", feature = "browser_host_scene"))]
    #[test]
    fn rejected_document_candidate_cannot_mutate_the_active_scene_registry() {
        let config_source = r##"config := {
  hosts: [{
    name: "view"
    provider: "scene"
    settings: { selector: "#view" renderer: "svg" }
  }]
  run: {
    paths: ["main.mec"]
    grants: [{ target: "view/frame" operations: ["write"] paths: ["replace"] }]
  }
}"##;
        let source = r##"@view := scene://view/frame{:write(replace)}
scene := {
  width: 10
  height: 10
  background: "#000"
  circles: ()
  lines: ()
}
@view/replace <- scene
"##;
        let config = parse_config_document(
            "scene-registry/mech.mcfg",
            config_source,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let authority = authority_config(
            config.hosts.clone(),
            config.run.as_ref().unwrap().grants.clone(),
        );
        validate_served_authority(&config, &authority).unwrap();
        let mut bootstrap = document_bootstrap("main.mec", source, HashMap::new(), Vec::new());
        bootstrap.console_instance = internal_repl_console_instance(&config.hosts);
        bootstrap.served = Some(ServedDocumentBootstrap {
            config_source: config_source.to_owned(),
            authority,
        });
        let mut document = WasmDocument::try_from_bootstrap(bootstrap).unwrap();
        let active = document.bootstrap.source().lifecycle.scenes();
        let accepted_scene = active
            .latest("view")
            .expect("the accepted program must publish its initial scene");

        assert!(
            document
                .repl
                .session
                .submit("broken := missing + 1")
                .is_err()
        );

        let retained = document.bootstrap.source().lifecycle.scenes();
        assert_eq!(retained.target_count(), 1);
        assert_eq!(retained.latest("view"), Some(accepted_scene));
    }

    #[cfg(all(
        feature = "browser_host_time",
        feature = "browser_host_console",
        feature = "browser_host_scene"
    ))]
    #[derive(Debug)]
    struct TestManualTimeHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        snapshot: mech_time::SharedTimeSnapshot,
        driver: mech_time::ManualTimeInputDriver,
    }

    #[cfg(all(
        feature = "browser_host_time",
        feature = "browser_host_console",
        feature = "browser_host_scene"
    ))]
    impl TestManualTimeHostFactory {
        fn new() -> Self {
            let snapshot = mech_time::new_shared_snapshot(mech_time::TimeSnapshot::default());
            Self {
                manifest: mech_time::time_host_manifest().unwrap(),
                driver: mech_time::ManualTimeInputDriver::new("clock", snapshot.clone()),
                snapshot,
            }
        }
    }

    #[cfg(all(
        feature = "browser_host_time",
        feature = "browser_host_console",
        feature = "browser_host_scene"
    ))]
    impl mech_runtime::RuntimeHostFactory for TestManualTimeHostFactory {
        fn provider_name(&self) -> &str {
            "time"
        }

        fn manifest(&self) -> &mech_runtime::HostManifestConfig {
            &self.manifest
        }

        fn validate_settings(
            &self,
            _instance_name: &str,
            settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<()> {
            mech_time::time_settings_from_config(settings).map(|_| ())
        }

        fn instantiate(
            &self,
            instance_name: &str,
            _settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<mech_runtime::RuntimeHostInstallation> {
            assert_eq!(instance_name, "clock");
            Ok(mech_runtime::RuntimeHostInstallation {
                interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
                resource_providers: vec![Box::new(mech_time::TimeResourceProvider::new(
                    instance_name,
                    self.snapshot.clone(),
                ))],
                input_drivers: vec![Box::new(self.driver.clone())],
            })
        }
    }

    #[cfg(all(
        feature = "browser_host_time",
        feature = "browser_host_console",
        feature = "browser_host_scene"
    ))]
    #[test]
    fn analog_clock_scene_advances_on_every_resident_time_packet() {
        let document = parse_config_document(
            "examples/analog-clock/mech.mcfg",
            include_str!("../../../examples/analog-clock/mech.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let source = include_str!("../../../examples/analog-clock/clock.mec").to_string();
        let sources = HashMap::from([("clock.mec".to_string(), source)]);
        let time_factory = TestManualTimeHostFactory::new();
        let time_driver = time_factory.driver.clone();
        let scene_backend = mech_scene::RecordingSceneBackend::new();
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .host_input_capacity(16)
            .host_factory(Box::new(time_factory))
            .unwrap()
            .host_factory(Box::new(
                mech_console::ConsoleHostFactory::with_backend(
                    mech_console::RecordingConsoleBackend::new(),
                )
                .unwrap(),
            ))
            .unwrap()
            .host_factory(Box::new(
                mech_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap(),
            ))
            .unwrap();
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        let mut runtime = builder.build().unwrap();
        run_project_sources(&mut runtime, &document).unwrap();
        runtime.start_input_drivers().unwrap();

        let rotation = |snapshot: &mech_scene::SceneSnapshot| {
            snapshot
                .lines
                .iter()
                .find(|line| line.id == "clock-second-hand")
                .unwrap()
                .rotation
        };
        for second in 1..=3 {
            time_driver
                .publish(mech_time::TimeSnapshot {
                    second: f64::from(second),
                    ..Default::default()
                })
                .unwrap();
            runtime.drain_host_inputs(1).unwrap();
            let scene = scene_backend.latest().unwrap();
            assert_eq!(rotation(&scene), f64::from(second) * 6.0);
        }
        assert_eq!(runtime.program_execution_info().resident_accepted_turns, 3);
        assert_eq!(scene_backend.generation(), 3);
    }

    #[cfg(not(feature = "browser_compute"))]
    #[test]
    fn generic_table_project_runs_without_legacy_execution() {
        let document = parse_config_document(
            "generic-table.mcfg",
            r#"config := { hosts: [] run: { paths: ["generic-table.mec"] grants: [] } }"#,
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let mut sources = HashMap::new();
        sources.insert(
            "generic-table.mec".to_string(),
            r#"delta := 0.25
rows := |id<string> x<f64>|
  | "row-a" 1 + delta |
  | "row-b" 2 + delta |"#
                .to_string(),
        );
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();
        run_project_sources(&mut runtime, &document).unwrap();
        assert_eq!(runtime.program_route(), RuntimeProgramRoute::ResidentPure);
        assert!(runtime.program_output_value().unwrap().is_some());
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[derive(Clone, Debug)]
    struct SharedManualTimerInputDriver(
        std::sync::Arc<std::sync::Mutex<mech_timer::ManualTimerInputDriver>>,
    );

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl SharedManualTimerInputDriver {
        fn new(
            instance: &str,
            frequency_hz: u64,
            max_catch_up_steps: u64,
            queue_policy: mech_timer::TimerQueuePolicy,
        ) -> Self {
            Self(std::sync::Arc::new(std::sync::Mutex::new(
                mech_timer::ManualTimerInputDriver::with_backend_and_policy(
                    instance,
                    mech_timer::ManualMonotonicTimerBackend::new(),
                    frequency_hz,
                    max_catch_up_steps,
                    queue_policy,
                ),
            )))
        }

        fn snapshot(&self) -> mech_timer::SharedTimerSnapshot {
            self.0.lock().unwrap().snapshot()
        }

        fn publish_steps(&self, count: usize) -> mech_core::MResult<usize> {
            self.0.lock().unwrap().publish_steps(count)
        }
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl mech_runtime::RuntimeHostInputDriver for SharedManualTimerInputDriver {
        fn drives(&self, source: &mech_runtime::RuntimeHostInputSource) -> bool {
            self.0.lock().unwrap().drives(source)
        }

        fn attach(&mut self, ingress: mech_runtime::RuntimeIngress) -> mech_core::MResult<()> {
            self.0.lock().unwrap().attach(ingress)
        }

        fn start(&mut self) -> mech_core::MResult<()> {
            self.0.lock().unwrap().start()
        }

        fn stop(&mut self) -> mech_core::MResult<()> {
            self.0.lock().unwrap().stop()
        }

        fn is_live(&self) -> bool {
            self.0.lock().unwrap().is_live()
        }
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[derive(Clone, Debug)]
    struct TestManualTimerHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        driver: SharedManualTimerInputDriver,
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl TestManualTimerHostFactory {
        #[cfg(feature = "browser_compute")]
        fn new() -> Self {
            Self::with_instance("clock")
        }

        fn with_instance(instance: &str) -> Self {
            Self {
                manifest: mech_timer::timer_host_manifest().unwrap(),
                driver: SharedManualTimerInputDriver::new(
                    instance,
                    60,
                    1,
                    mech_timer::TimerQueuePolicy::Latest,
                ),
            }
        }
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl mech_runtime::RuntimeHostFactory for TestManualTimerHostFactory {
        fn provider_name(&self) -> &str {
            "timer"
        }
        fn manifest(&self) -> &mech_runtime::HostManifestConfig {
            &self.manifest
        }
        fn validate_settings(
            &self,
            _instance_name: &str,
            settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<()> {
            mech_timer::timer_settings_from_config(settings).map(|_| ())
        }
        fn instantiate(
            &self,
            instance_name: &str,
            settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<mech_runtime::RuntimeHostInstallation> {
            mech_timer::timer_settings_from_config(settings)?;
            Ok(mech_runtime::RuntimeHostInstallation {
                interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
                resource_providers: vec![Box::new(mech_timer::TimerResourceProvider::new(
                    instance_name,
                    self.driver.snapshot(),
                ))],
                input_drivers: vec![Box::new(self.driver.clone())],
            })
        }
    }

    #[cfg(all(
        feature = "browser_host_timer",
        feature = "browser_host_scene",
        feature = "browser_compute"
    ))]
    pub(super) fn assert_ekf_scene_advances_on_every_resident_timer_packet() {
        use nalgebra::{SMatrix, SVector};

        let document = parse_config_document(
            "examples/ekf/mech.mcfg",
            include_str!("../../../examples/ekf/mech.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let source = include_str!("../../../examples/ekf/localization.mec").to_string();
        assert!(
            source.contains("ekf-predict(μ<[f32]:3,1>, Σ<[f32]:3,3>"),
            "the EKF prediction must remain an ordinary fixed-shape Mech function",
        );
        assert!(
            source.contains("ekf-correct(μ-<[f32]:3,1>, Σ-<[f32]:3,3>"),
            "the EKF correction must remain an ordinary fixed-shape Mech function",
        );
        assert!(
            source.contains("K := (S \\ B)'"),
            "the Kalman gain must use the language's matrix solve operation",
        );
        assert!(
            source.contains("observation-guard := 1f32 - ceil(z[3])"),
            "only a fully invisible camera may perturb singular observation geometry",
        );
        assert!(
            source.contains("+> math/*"),
            "the EKF document must import the math module's public functions",
        );
        for qualified in [
            "math/abs(",
            "math/atan2(",
            "math/ceil(",
            "math/cos(",
            "math/floor(",
            "math/sin(",
            "math/sqrt(",
        ] {
            assert!(
                !source.contains(qualified),
                "wildcard-imported math calls must be unqualified: {qualified}",
            );
        }
        assert!(
            !source.contains("innovation-inverse") && !source.contains("innovation-determinant"),
            "the example must not expand a matrix inverse by hand",
        );
        assert!(
            !source.contains("matrix/vertcat")
                && source.contains("advanced-truth-path := [truth-path[2..=376,:]; truth-screen']"),
            "vertical concatenation must use matrix literal syntax",
        );
        let compact_source = source
            .chars()
            .filter(|character| !character.is_whitespace())
            .collect::<String>();
        assert!(
            compact_source.contains("scene-line-strips:=|id<string>positions<*>")
                && compact_source.contains("scene-text:=|id<string>x<f64>y<f64>fill<f64>")
                && compact_source.contains("font-weight<f64>"),
            "scene collections must remain table values",
        );
        let sources = HashMap::from([("localization.mec".to_string(), source.clone())]);
        let timer_factory = TestManualTimerHostFactory::new();
        let timer_driver = timer_factory.driver.clone();
        let scene_backend = mech_scene::RecordingSceneBackend::new();
        #[cfg(feature = "browser_compute")]
        let coordinator = {
            let mut planning = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_native_plan_catalog())
                .source_resolver(project_source_resolver(&sources).unwrap())
                .host_factory(Box::new(timer_factory.clone()))
                .unwrap()
                .host_factory(Box::new(
                    mech_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap(),
                ))
                .unwrap();
            for host in document
                .hosts
                .iter()
                .filter(|host| host.provider != "compute")
            {
                planning = planning.host_instance(host.clone());
            }
            for grant in &document.run.as_ref().unwrap().grants {
                planning = planning.run_resource_grant(grant.clone());
            }
            let mut compiler = planning.build_compiler().unwrap();
            let retained = SourceDocument::parse_resolved(
                "memory:main.mec",
                mech_syntax::document::Revision(0),
                source.as_str(),
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap();
            let prepared = crate::mixed_compute::prepare_compute_document_region(
                &mut compiler,
                &retained,
                0.0,
                0.0,
            )
            .unwrap();
            assert!(!prepared.coordinator.nodes().is_empty());
            assert!(prepared.coordinator.nodes().iter().all(|node| {
                let node = node.as_operation().expect("EKF coordinator operation");
                matches!(
                    prepared.coordinator.contracts().get(node.contract),
                    Some(mech_core::ResolvedOperationContract::Declared(_))
                )
            }));
            let command =
                crate::mixed_compute::ComputeCommandHandle::new(prepared.region.clone(), 1);
            let registry = crate::mixed_compute::browser_compute_backend_registry(
                command.clone(),
                crate::mixed_compute::BrowserOutputHandle::default(),
                false,
            )
            .unwrap();
            let storage = prepared
                .program
                .fixed_shape_storage()
                .expect("EKF must lower to a fixed-shape kernel");
            assert_eq!(
                storage
                    .inputs
                    .iter()
                    .map(|input| input.name.as_ref())
                    .collect::<std::collections::BTreeSet<_>>(),
                std::collections::BTreeSet::from(["camera", "control", "measurement"]),
                "the fixed-shape kernel must retain every declared activation input",
            );
            assert_eq!(
                storage.instances, 1_000,
                "the browser EKF must execute 1,000 independent filter lanes",
            );
            assert!(
                storage.inputs.len() + storage.states.len() * 2 + 1 <= 8,
                "the checked EKF kernel must fit WebGPU's portable minimum of eight storage buffers",
            );
            let factory = mech_gpu::ComputeHostFactory::new(
                prepared.region,
                prepared.placement,
                prepared.program,
                prepared.initializers,
                registry,
                mech_compute::ComputePlatform::Browser,
            )
            .unwrap()
            .with_retained_outputs(prepared.retained_outputs)
            .unwrap();
            (prepared.coordinator, factory, command)
        };
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .host_input_capacity(16)
            .host_factory(Box::new(timer_factory))
            .unwrap()
            .host_factory(Box::new(
                mech_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap(),
            ))
            .unwrap();
        #[cfg(feature = "browser_compute")]
        {
            builder = builder.host_factory(Box::new(coordinator.1)).unwrap();
        }
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        let mut runtime = builder.build().unwrap();
        #[cfg(not(feature = "browser_compute"))]
        let root = document.run.as_ref().unwrap().paths[0]
            .to_string_lossy()
            .to_string();
        let durability = runtime.config().resident_durability;
        #[cfg(feature = "browser_compute")]
        runtime
            .load_compiled_program(coordinator.0, durability)
            .unwrap();
        #[cfg(not(feature = "browser_compute"))]
        runtime
            .load_interactive_root_program(
                SourceRequest::new(root),
                browser_module_options(),
                durability,
            )
            .unwrap();
        runtime.start_input_drivers().unwrap();

        let rendered_truth = |snapshot: &mech_scene::SceneSnapshot| {
            let truth = snapshot
                .circles
                .iter()
                .find(|circle| circle.id == "truth")
                .unwrap();
            let heading = snapshot
                .lines
                .iter()
                .find(|line| line.id == "truth-heading")
                .unwrap();
            [
                (truth.x - 120.0) / 62.0,
                (700.0 - truth.y) / 62.0,
                (-(heading.y2 - heading.y1)).atan2(heading.x2 - heading.x1),
            ]
        };
        let rendered_estimate = |snapshot: &mech_scene::SceneSnapshot| {
            let estimate = snapshot
                .circles
                .iter()
                .find(|circle| circle.id == "estimate")
                .unwrap();
            let heading = snapshot
                .lines
                .iter()
                .find(|line| line.id == "estimate-heading")
                .unwrap();
            SVector::<f64, 3>::new(
                (estimate.x - 120.0) / 62.0,
                (700.0 - estimate.y) / 62.0,
                (-(heading.y2 - heading.y1)).atan2(heading.x2 - heading.x1),
            )
        };
        assert_eq!(
            runtime.program_execution_info().observation_count,
            6,
            "the resident artifact must observe the timer, two pointer fields, compute completion, packed sample, and sampled visibility"
        );
        assert!(scene_backend.latest().is_none());
        let pi = 3.141592654_f64;
        let dt = 0.05_f64;
        let cruise_speed = 1.15_f64;
        let turn_rate_limit = 1.75_f64;
        let heading_gain = 4.0_f64;
        let low_bound = 2.5_f64;
        let high_bound = 7.5_f64;
        let field_span = high_bound - low_bound;
        let side_ticks = 94_usize;
        let lap_ticks = side_ticks * 4;
        let camera_max_range = 3.6_f64;
        let camera_range_fade = 0.18_f64;
        let mut expected_truth = [2.5_f64, 2.5_f64, 0.0_f64];
        let mut expected_estimate = SVector::<f32, 3>::new(2.9, 2.15, 0.16);
        let mut expected_covariance = SMatrix::<f32, 3, 3>::new(
            0.45, 0.0, 0.0, //
            0.0, 0.45, 0.0, //
            0.0, 0.0, 0.08,
        );
        let identity = SMatrix::<f32, 3, 3>::identity();
        let process_covariance = SMatrix::<f32, 2, 2>::new(0.08, 0.0, 0.0, 0.018);
        let measurement_covariance = SMatrix::<f32, 2, 2>::new(0.0225, 0.0, 0.0, 0.0004);
        let camera_positions = [
            SVector::<f64, 2>::new(1.0, 1.0),
            SVector::<f64, 2>::new(9.0, 1.0),
            SVector::<f64, 2>::new(9.0, 9.0),
            SVector::<f64, 2>::new(1.0, 9.0),
        ];
        let camera_screen_positions = [
            [182.0_f64, 638.0_f64],
            [678.0_f64, 638.0_f64],
            [678.0_f64, 142.0_f64],
            [182.0_f64, 142.0_f64],
        ];

        let positive_part = |value: f64| (value + value.abs()) / 2.0;
        let clamp_unit = |value: f64| positive_part(value) - positive_part(value - 1.0);
        let mut saw_prediction_only_turn = false;
        let mut saw_camera_update = false;
        let mut saw_faded_camera_update = false;
        let mut saw_zero_camera_turn = false;
        let mut maximum_visible_cameras = 0_usize;
        let mut visited_sides = [false; 4];
        let mut maximum_guide_deviation = 0.0_f64;

        // Publish five scheduler steps at a time. The configured latest-value
        // policy delivers only one packet, whose absolute tick jumps by five.
        // The vectorized phase, camera selection, and filter must nevertheless
        // advance exactly once per accepted resident packet. A full lap plus
        // part of the next side exercises every camera and every dead zone.
        for turn in 1..=420 {
            assert_eq!(timer_driver.publish_steps(5).unwrap(), 1);
            // The compute driver publishes one initial telemetry packet before
            // the first timer packet, then one sampled-output packet after each
            // accepted dispatch. Allow all three on the first iteration; later
            // iterations normally consume the timer and its sample together.
            let mut outcomes = runtime.drain_host_inputs(3).unwrap();
            let command = coordinator
                .2
                .take_command_data()
                .unwrap()
                .expect("every timer token must queue one compute command");
            assert!(!command.acknowledgement_required);
            for (name, width) in [
                ("control", 3_000),
                ("camera", 2_000),
                ("measurement", 3_000),
            ] {
                if let Some(values) = command.changed_inputs.get(name) {
                    assert_eq!(
                        values.len(),
                        width,
                        "compute input `{name}` had the wrong batch extent"
                    );
                }
            }
            outcomes.extend(runtime.drain_host_inputs(1).unwrap_or_else(|error| {
                let events = runtime.list_events(Some(32)).unwrap_or_default();
                panic!("compute completion turn failed: {error:?}; recent events={events:?}")
            }));
            assert_eq!(
                outcomes.len(),
                2,
                "each timer packet must produce exactly one bounded compute-sample turn",
            );

            let delivered_turn = (turn - 1) as usize;
            let drive_phase = ((delivered_turn / side_ticks) % 4) as f64;
            let phase_progress = (delivered_turn % side_ticks) as f64 / side_ticks as f64;
            let lookahead_parameter = (drive_phase + phase_progress + 0.15) % 4.0;
            let target_x = low_bound
                + field_span
                    * (clamp_unit(lookahead_parameter) - clamp_unit(lookahead_parameter - 2.0));
            let target_y = low_bound
                + field_span
                    * (clamp_unit(lookahead_parameter - 1.0)
                        - clamp_unit(lookahead_parameter - 3.0));
            let target_heading = (target_y - expected_truth[1]).atan2(target_x - expected_truth[0]);
            let heading_error = (target_heading - expected_truth[2])
                .sin()
                .atan2((target_heading - expected_truth[2]).cos());
            let requested_omega = heading_gain * heading_error;
            let commanded_omega = ((requested_omega + turn_rate_limit).powi(2).sqrt()
                - (requested_omega - turn_rate_limit).powi(2).sqrt())
                / 2.0;
            let turn_fraction = commanded_omega.powi(2).sqrt() / turn_rate_limit;
            let commanded_speed = cruise_speed * (1.0 - 0.42 * turn_fraction);
            // The application advances simulation time once per accepted
            // resident timer packet. Absolute timer values may skip when a
            // latest-only host coalesces packets, but those scheduling gaps
            // must not change the physical or measurement input sequence.
            let simulation_time = turn as f64 * dt;
            let actual_speed = commanded_speed * (0.965 + 0.025 * (simulation_time * 0.73).sin());
            let actual_omega =
                commanded_omega * (0.99 + 0.01 * (simulation_time * 0.51 + 0.8).sin());
            let midpoint_heading = expected_truth[2] + actual_omega * dt / 2.0;
            expected_truth = [
                expected_truth[0] + actual_speed * midpoint_heading.cos() * dt,
                expected_truth[1] + actual_speed * midpoint_heading.sin() * dt,
                expected_truth[2] + actual_omega * dt,
            ];

            // Reproduce the complete filter independently from the Mech
            // resident graph. This oracle checks the state and covariance,
            // not merely that a scene changed after each accepted packet.
            let filter_dt = dt as f32;
            let filter_speed = commanded_speed as f32;
            let filter_omega = commanded_omega as f32;
            let estimate_mid_heading = expected_estimate[2] + filter_omega * filter_dt / 2.0_f32;
            let estimate_mid_cos = estimate_mid_heading.cos();
            let estimate_mid_sin = estimate_mid_heading.sin();
            let motion_jacobian = SMatrix::<f32, 3, 3>::new(
                1.0,
                0.0,
                -filter_speed * estimate_mid_sin * filter_dt,
                0.0,
                1.0,
                filter_speed * estimate_mid_cos * filter_dt,
                0.0,
                0.0,
                1.0,
            );
            let control_jacobian = SMatrix::<f32, 3, 2>::new(
                estimate_mid_cos * filter_dt,
                -filter_speed * estimate_mid_sin * filter_dt.powi(2) / 2.0_f32,
                estimate_mid_sin * filter_dt,
                filter_speed * estimate_mid_cos * filter_dt.powi(2) / 2.0_f32,
                0.0,
                filter_dt,
            );
            let predicted_estimate = expected_estimate
                + SVector::<f32, 3>::new(
                    filter_speed * estimate_mid_cos * filter_dt,
                    filter_speed * estimate_mid_sin * filter_dt,
                    filter_omega * filter_dt,
                );
            let predicted_covariance =
                motion_jacobian * expected_covariance * motion_jacobian.transpose()
                    + control_jacobian * process_covariance * control_jacobian.transpose();

            let camera_index = ((delivered_turn + side_ticks / 2) / side_ticks) % 4;
            let active_camera = camera_positions[camera_index];
            let truth_dx = active_camera[0] - expected_truth[0];
            let truth_dy = active_camera[1] - expected_truth[1];
            let measured_range = (truth_dx.powi(2) + truth_dy.powi(2)).sqrt()
                + 0.11 * (simulation_time * 1.57 + (camera_index + 1) as f64 * 0.7).sin();
            let measured_bearing = truth_dy.atan2(truth_dx) - expected_truth[2]
                + 0.011 * (simulation_time * 1.91 + (camera_index + 1) as f64).sin();

            let predicted_dx = active_camera[0] as f32 - predicted_estimate[0];
            let predicted_dy = active_camera[1] as f32 - predicted_estimate[1];
            let predicted_q = predicted_dx.powi(2) + predicted_dy.powi(2);
            let predicted_range = predicted_q.sqrt();
            let observation_jacobian = SMatrix::<f32, 2, 3>::new(
                -predicted_dx / predicted_range,
                -predicted_dy / predicted_range,
                0.0,
                predicted_dy / predicted_q,
                -predicted_dx / predicted_q,
                -1.0,
            );
            let innovation_covariance =
                observation_jacobian * predicted_covariance * observation_jacobian.transpose()
                    + measurement_covariance;
            let kalman_gain = predicted_covariance
                * observation_jacobian.transpose()
                * innovation_covariance
                    .try_inverse()
                    .expect("EKF innovation covariance must remain invertible");
            let innovation = SVector::<f32, 2>::new(
                measured_range as f32 - predicted_range,
                (measured_bearing as f32
                    - (predicted_dy.atan2(predicted_dx) - predicted_estimate[2]))
                    .sin()
                    .atan2(
                        (measured_bearing as f32
                            - (predicted_dy.atan2(predicted_dx) - predicted_estimate[2]))
                            .cos(),
                    ),
            );
            let observed_estimate = predicted_estimate + kalman_gain * innovation;
            let joseph_a = identity - kalman_gain * observation_jacobian;
            let observed_covariance = joseph_a * predicted_covariance * joseph_a.transpose()
                + kalman_gain * measurement_covariance * kalman_gain.transpose();
            let observed_covariance =
                0.5_f32 * (observed_covariance + observed_covariance.transpose());
            let visibility = clamp_unit(
                (camera_max_range
                    - (camera_positions[camera_index]
                        - SVector::<f64, 2>::new(expected_truth[0], expected_truth[1]))
                    .norm())
                    / camera_range_fade,
            );
            let filter_visibility = visibility as f32;
            expected_estimate =
                predicted_estimate + filter_visibility * (observed_estimate - predicted_estimate);
            expected_covariance = predicted_covariance
                + filter_visibility * (observed_covariance - predicted_covariance);
            saw_prediction_only_turn |= visibility == 0.0;
            saw_camera_update |= visibility == 1.0;
            saw_faded_camera_update |= visibility > 0.0 && visibility < 1.0;

            let snapshot = scene_backend.latest().unwrap();
            let actual_truth = rendered_truth(&snapshot);
            for (component, (actual, expected)) in
                actual_truth.iter().zip(expected_truth).enumerate()
            {
                let error = if component == 2 {
                    (actual - expected)
                        .sin()
                        .atan2((actual - expected).cos())
                        .abs()
                } else {
                    (actual - expected).abs()
                };
                assert!(
                    error < 1.0e-9,
                    "EKF truth component {component} diverged on delivered turn {turn}: actual={actual:?} expected={expected:?}; outcomes={outcomes:?}",
                );
            }

            let rendered_covariance = snapshot
                .line_strips
                .iter()
                .find(|strip| strip.id == "covariance")
                .expect("EKF scene must render its covariance outline");
            assert_eq!(
                rendered_covariance.positions.len(),
                65,
                "the covariance outline must retain every 0..=64 sample"
            );
            let ellipse_a = f64::from(expected_covariance[(0, 0)]);
            let ellipse_b = f64::from(expected_covariance[(0, 1)]);
            let ellipse_c = f64::from(expected_covariance[(1, 1)]);
            let ellipse_root = (((ellipse_a - ellipse_c) / 2.0).powi(2) + ellipse_b.powi(2)).sqrt();
            let ellipse_major = ((ellipse_a + ellipse_c) / 2.0 + ellipse_root).sqrt() * 2.0;
            let ellipse_minor = ((ellipse_a + ellipse_c) / 2.0 - ellipse_root).sqrt() * 2.0;
            let ellipse_rotation = 0.5 * (2.0 * ellipse_b).atan2(ellipse_a - ellipse_c);
            let display_major = (ellipse_major.powi(2) + 0.04_f64.powi(2)).sqrt();
            let display_minor = (ellipse_minor.powi(2) + 0.04_f64.powi(2)).sqrt();
            let rotation_cos = ellipse_rotation.cos();
            let rotation_sin = ellipse_rotation.sin();
            for (sample, actual) in rendered_covariance.positions.iter().enumerate() {
                let angle = sample as f64 * (2.0 * pi / 64.0);
                let angle_cos = angle.cos();
                let angle_sin = angle.sin();
                let expected = [
                    120.0
                        + f64::from(expected_estimate[0]) * 62.0
                        + 62.0
                            * (display_major * angle_cos * rotation_cos
                                - display_minor * angle_sin * rotation_sin),
                    700.0
                        - f64::from(expected_estimate[1]) * 62.0
                        - 62.0
                            * (display_major * angle_cos * rotation_sin
                                + display_minor * angle_sin * rotation_cos),
                ];
                for axis in 0..2 {
                    assert!(
                        (actual[axis] - expected[axis]).abs() < 1.0e-3,
                        "rendered covariance sample {sample} axis {axis} diverged on delivered turn {turn}: actual={actual:?} expected={expected:?}",
                    );
                }
            }

            let scene_estimate = rendered_estimate(&snapshot);
            for component in 0..3 {
                let error = if component == 2 {
                    (scene_estimate[component] - f64::from(expected_estimate[component]))
                        .sin()
                        .atan2(
                            (scene_estimate[component] - f64::from(expected_estimate[component]))
                                .cos(),
                        )
                        .abs()
                } else {
                    (scene_estimate[component] - f64::from(expected_estimate[component])).abs()
                };
                assert!(
                    error < 2.0e-5,
                    "rendered EKF estimate component {component} diverged on delivered turn {turn}: actual={scene_estimate:?} expected={expected_estimate:?}",
                );
            }

            let expected_distances = camera_positions.map(|camera| {
                (camera - SVector::<f64, 2>::new(expected_truth[0], expected_truth[1])).norm()
            });
            let expected_visibilities = expected_distances
                .map(|distance| clamp_unit((camera_max_range - distance) / camera_range_fade));
            let visible_camera_count = expected_visibilities
                .iter()
                .filter(|visibility| **visibility > 0.0)
                .count();
            saw_zero_camera_turn |= visible_camera_count == 0;
            maximum_visible_cameras = maximum_visible_cameras.max(visible_camera_count);
            for (camera, expected_visibility) in expected_visibilities.iter().enumerate() {
                if expected_distances[camera] >= camera_max_range {
                    assert_eq!(*expected_visibility, 0.0);
                }
                let ring = snapshot
                    .circles
                    .iter()
                    .find(|circle| circle.id == format!("camera-range-{}", camera + 1))
                    .unwrap();
                let ray = snapshot
                    .lines
                    .iter()
                    .find(|line| line.id == format!("ray-{}", camera + 1))
                    .unwrap();
                let body = snapshot
                    .circles
                    .iter()
                    .find(|circle| circle.id == format!("camera-{}", camera + 1))
                    .unwrap();
                assert_eq!(
                    [body.x, body.y],
                    camera_screen_positions[camera],
                    "camera {} body moved on delivered turn {turn}",
                    camera + 1,
                );
                assert_eq!(body.radius, 9.0);
                assert_eq!(body.opacity, 1.0);
                assert_eq!([ring.x, ring.y], camera_screen_positions[camera]);
                assert_eq!([ray.x1, ray.y1], camera_screen_positions[camera]);
                assert!(
                    (ring.radius - camera_max_range * 62.0).abs() < 1.0e-9,
                    "camera {} sensing footprint has the wrong rendered radius on delivered turn {turn}: actual={}",
                    camera + 1,
                    ring.radius,
                );
                assert!((ring.opacity - 0.16).abs() < 1.0e-9);
                assert!((ray.opacity - 0.4 * expected_visibility).abs() < 1.0e-9);
            }

            let x = expected_truth[0];
            let y = expected_truth[1];
            let guide_deviation = [
                (y - low_bound).abs(),
                (x - high_bound).abs(),
                (y - high_bound).abs(),
                (x - low_bound).abs(),
            ]
            .into_iter()
            .fold(f64::INFINITY, f64::min);
            maximum_guide_deviation = maximum_guide_deviation.max(guide_deviation);
            if delivered_turn < lap_ticks {
                visited_sides[(delivered_turn / side_ticks) % 4] = true;
            }
        }
        assert!(
            saw_prediction_only_turn,
            "the finite camera ranges must create a dead zone"
        );
        assert!(
            saw_camera_update,
            "at least one camera must reacquire the robot"
        );
        assert!(
            saw_faded_camera_update,
            "the oracle must exercise a fractional camera fade update"
        );
        assert!(
            saw_zero_camera_turn,
            "the finite sensing footprints must leave part of the square unseen"
        );
        assert_eq!(
            maximum_visible_cameras, 1,
            "the four sensing footprints must not overlap on the driven square"
        );
        assert_eq!(visited_sides, [true; 4]);
        assert!(
            maximum_guide_deviation < 0.5,
            "the closed-loop square controller drifted {maximum_guide_deviation} field units from its guide",
        );
        let final_snapshot = scene_backend.latest().unwrap();
        for id in ["truth-path", "estimate-path"] {
            let path = final_snapshot
                .line_strips
                .iter()
                .find(|strip| strip.id == id)
                .unwrap_or_else(|| panic!("EKF scene must render `{id}`"));
            let (minimum_x, maximum_x) = path.positions.iter().fold(
                (f64::INFINITY, f64::NEG_INFINITY),
                |(minimum, maximum), point| (minimum.min(point[0]), maximum.max(point[0])),
            );
            assert!(
                maximum_x - minimum_x > 100.0,
                "`{id}` must advance across compute/timer scheduling boundaries",
            );
        }
        assert_eq!(
            runtime.program_execution_info().resident_accepted_turns,
            840
        );
        assert_eq!(
            scene_backend.generation(),
            840,
            "the asynchronous scene must publish current robot truth on timer turns and completed estimates on filter turns",
        );
    }

    #[cfg(all(
        not(target_arch = "wasm32"),
        feature = "browser_host_timer",
        feature = "browser_host_scene",
        feature = "browser_compute"
    ))]
    #[test]
    fn ekf_scene_advances_on_every_resident_timer_packet() {
        assert_ekf_scene_advances_on_every_resident_timer_packet();
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    fn generic_fixture_document() -> MechConfigDocument {
        parse_config_document(
            "generic-timer-table-scene/mech.mcfg",
            include_str!("../tests/fixtures/generic-timer-table-scene/mech.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap()
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    fn generic_fixture_sources() -> HashMap<String, String> {
        let mut sources = HashMap::new();
        sources.insert(
            "table-scene.mec".to_string(),
            include_str!("../tests/fixtures/generic-timer-table-scene/table-scene.mec").to_string(),
        );
        sources
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[test]
    fn timer_table_scene_runs_residently_and_publishes_its_tables() {
        let document = generic_fixture_document();
        let source_paths = required_path_strings(include_str!(
            "../tests/fixtures/generic-timer-table-scene/mech.mcfg"
        ))
        .unwrap();
        assert_eq!(source_paths, vec!["table-scene.mec".to_string()]);

        let scene_backend = mech_scene::RecordingSceneBackend::new();
        let timer_factory = TestManualTimerHostFactory::with_instance("tick");
        let timer_driver = timer_factory.driver.clone();
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&generic_fixture_sources()).unwrap())
            .host_input_capacity(16)
            .host_factory(Box::new(timer_factory))
            .unwrap()
            .host_factory(Box::new(
                mech_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap(),
            ))
            .unwrap();
        for host in &document.hosts {
            builder = builder.host_instance(host.clone());
        }
        for grant in &document.run.as_ref().unwrap().grants {
            builder = builder.run_resource_grant(grant.clone());
        }
        let mut runtime = builder.build().unwrap();
        run_project_sources(&mut runtime, &document).unwrap();
        assert_eq!(
            runtime.program_route(),
            RuntimeProgramRoute::ResidentExternal
        );
        runtime.start_input_drivers().unwrap();
        assert_eq!(timer_driver.publish_steps(1).unwrap(), 1);
        runtime.drain_host_inputs(1).unwrap();
        let scene = scene_backend.latest().unwrap();
        assert_eq!(scene.circles.len(), 2);
        assert_eq!(scene.lines.len(), 3);
        assert_eq!(scene_backend.generation(), 1);
    }

    #[test]
    fn from_sources_rejects_missing_source() {
        let mut runtime = browser_runtime_builder().build().unwrap();
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();
        assert!(run_project_sources(&mut runtime, &document).is_err());
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
        }])
        .unwrap();
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
            let result =
                decode_injected_host_delegation_keys(vec![InjectedHostDelegationPublicKey {
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
    use js_sys::{Array, Object};
    use mech_runtime::MAX_RESIDENT_STEP_COUNT;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[cfg(all(feature = "served_project_authority", feature = "state_machines"))]
    #[wasm_bindgen_test]
    fn public_static_fsm_documents_require_retained_nominal_provenance() {
        let source = "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n";
        let config = r#"config := { hosts: [] run: { paths: ["main.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(source),
        )
        .unwrap();
        let documents = Object::new();
        Reflect::set(
            &documents,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(&encoded_document_at("main.mec", source)),
        )
        .unwrap();
        let roots = Array::new();
        roots.push(&JsValue::from_str("main.mec"));
        let restore = |provenance| {
            WasmProject::from_served_documents(
                config,
                sources.clone().into(),
                documents.clone().into(),
                roots.clone().into(),
                Array::new().into(),
                provenance,
            )
        };
        let error = restore(JsValue::NULL).err().unwrap();
        let message = Reflect::get(&error, &JsValue::from_str("message"))
            .unwrap()
            .as_string()
            .expect("project rejection returns a JavaScript Error");
        assert!(message.contains("missing nominal provenance"), "{message}");

        let document =
            parse_config_document("mech.mcfg", config, ConfigProfileOptions::default()).unwrap();
        install_served_authority(&BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts: document.hosts,
            run_grants: document.run.unwrap().grants,
        });
        let origin = SourceDocument::new_standalone_origin();
        let provenance = js_sys::JSON::parse(
            &serde_json::json!({
                "main.mec": {"nominalOrigin": origin}
            })
            .to_string(),
        )
        .unwrap();
        let mut project = restore(provenance).unwrap();
        assert_eq!(
            project
                .runtime
                .root_symbol_value("result")
                .unwrap()
                .format_canonical_inline(),
            "1"
        );
        project.stop().unwrap();
    }

    #[cfg(all(feature = "enum", feature = "kind_define", feature = "variable_define"))]
    #[wasm_bindgen_test]
    fn public_enum_document_edits_reset_and_rejection_preserve_nominal_identity() {
        let source = "<color> := :red | :green | :blue\nmy-color<color> := :red\n";
        let edited = source.replace("my-color<color> := :red", "my-color<color> := :blue");
        let invalid = source.replace("my-color<color> := :red", "my-color<color> := :yellow");
        let encoded = encoded_document(source);
        for packaged in [false, true] {
            let mut document = if packaged {
                let sources = Object::new();
                Reflect::set(
                    &sources,
                    &JsValue::from_str("document.mec"),
                    &JsValue::from_str(source),
                )
                .unwrap();
                let provenance = js_sys::JSON::parse(
                    &serde_json::json!({
                        "document.mec": {
                            "nominalOrigin": {"segments": ["package", "colors"]},
                            "nominalPackageId": null
                        }
                    })
                    .to_string(),
                )
                .unwrap();
                WasmDocument::from_encoded_with_bundle(
                    &encoded,
                    "document.mec",
                    sources.into(),
                    Array::new().into(),
                    provenance,
                )
                .unwrap()
            } else {
                WasmDocument::from_encoded(&encoded).unwrap()
            };
            let origin = document
                .repl
                .session
                .source_document()
                .unwrap()
                .nominal_origin()
                .unwrap()
                .clone();
            if packaged {
                assert_eq!(origin.segments(), &["package", "colors"]);
            }
            assert_eq!(
                document.bootstrap.document.document().nominal_origin(),
                Some(&origin)
            );
            let original = document.repl.session.symbol("my-color").unwrap().unwrap();
            document.repl_replace_source(&edited).unwrap();
            let updated = document.repl.session.symbol("my-color").unwrap().unwrap();
            assert_eq!(original.schema_key(), updated.schema_key());
            assert_ne!(original, updated);
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                Some(&origin)
            );
            let output_id = document.bootstrap.program_output_id().unwrap().unwrap();
            assert_eq!(
                document
                    .runtime()
                    .unwrap()
                    .output_value(output_id)
                    .unwrap()
                    .unwrap(),
                updated
            );

            assert!(document.repl_replace_source(&invalid).is_err());
            assert_eq!(document.repl_source(), edited);
            assert_eq!(
                document.repl.session.symbol("my-color").unwrap().unwrap(),
                updated
            );
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                Some(&origin)
            );
            document.reset(&encoded).unwrap();
            assert_eq!(
                document.repl.session.symbol("my-color").unwrap().unwrap(),
                original
            );
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                Some(&origin)
            );
            document.repl_invoke(":clear").unwrap();
            document.repl_invoke(":reset").unwrap();
            assert_eq!(
                document.repl.session.symbol("my-color").unwrap().unwrap(),
                original
            );
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                Some(&origin)
            );
            document.stop().unwrap();
        }
    }

    #[cfg(all(
        feature = "enum",
        feature = "kind_define",
        feature = "variable_define",
        feature = "state_machines",
    ))]
    #[wasm_bindgen_test]
    fn public_standalone_bundle_edits_and_reset_preserve_enum_and_fsm_identity() {
        for source in [
            "<color> := :red | :blue\nmy-color<color> := :red\n",
            "#Drive() => <f64>\n  | :Done.\n#Drive() -> :Done\n  :Done => 1.0.\nresult := #Drive()\nresult\n",
        ] {
            let retained = SourceDocument::parse_resolved(
                "bundle:///document.mec",
                mech_syntax::document::Revision(0),
                source,
                mech_syntax::document::ParseConfig::default(),
            )
            .unwrap()
            .with_standalone_nominal_origin();
            let mut compiler = RuntimeBuilder::new()
                .function_catalog(mech_stdlib::source_catalog())
                .build_compiler()
                .unwrap();
            let product = compiler.compile_document(&retained).unwrap();
            let bundle =
                CanonicalProgramBundle::from_product("bundle:///document.mec", &retained, &product)
                    .unwrap();
            let sources = Object::new();
            Reflect::set(
                &sources,
                &JsValue::from_str("document.mec"),
                &JsValue::from_str(source),
            )
            .unwrap();
            let mut document = WasmDocument::from_encoded_with_bundle(
                &bundle.encode().unwrap(),
                "document.mec",
                sources.into(),
                Array::new().into(),
                Object::new().into(),
            )
            .unwrap();
            let enum_keys = |artifact: &mech_engine::ProgramArtifact| {
                artifact
                    .schemas()
                    .entries()
                    .filter_map(|entry| {
                        matches!(entry.schema().body(), mech_core::SchemaBody::Enum { .. })
                            .then_some(entry.key())
                    })
                    .collect::<Vec<_>>()
            };
            let original_keys = enum_keys(product.artifact());
            assert!(!original_keys.is_empty());
            let edited = source
                .replace("my-color<color> := :red", "my-color<color> := :blue")
                .replace("=> 1.0.", "=> 2.0.");
            document.repl_replace_source(&edited).unwrap();
            let edited_product = compiler
                .compile_document(document.repl.session.source_document().unwrap())
                .unwrap();
            assert_eq!(enum_keys(edited_product.artifact()), original_keys);
            assert_eq!(
                document.bootstrap.document_base().nominal_origin(),
                retained.nominal_origin(),
            );
            document.reset(&encoded_document(source)).unwrap();
            let reset_product = compiler
                .compile_document(document.repl.session.source_document().unwrap())
                .unwrap();
            assert_eq!(enum_keys(reset_product.artifact()), original_keys);
            document.stop().unwrap();
        }
    }

    #[cfg(all(feature = "enum", feature = "kind_define", feature = "variable_define"))]
    #[wasm_bindgen_test]
    fn public_standalone_bundle_scalar_revision_preserves_enum_owner() {
        let enum_source = "<color> := :red | :blue\nmy-color<color> := :red\n";
        let scalar_source = "answer := 42\n";
        let mut document = WasmDocument::from_encoded(&encoded_document(enum_source)).unwrap();
        let original = document.repl.session.symbol("my-color").unwrap().unwrap();
        let origin = document
            .repl
            .session
            .source_document()
            .unwrap()
            .nominal_origin()
            .unwrap()
            .clone();

        document.repl_replace_source(scalar_source).unwrap();
        let retained = SourceDocument::parse_resolved(
            "bundle:///document.mec",
            mech_syntax::document::Revision(1),
            document.repl_source(),
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap()
        .with_nominal_origin(origin.clone());
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()
            .unwrap();
        let product = compiler.compile_document(&retained).unwrap();
        let bundle =
            CanonicalProgramBundle::from_product("bundle:///document.mec", &retained, &product)
                .unwrap();
        assert_eq!(bundle.root_nominal_origin.as_ref(), Some(&origin));

        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("document.mec"),
            &JsValue::from_str(scalar_source),
        )
        .unwrap();
        let mut restored = WasmDocument::from_encoded_with_bundle(
            &bundle.encode().unwrap(),
            "document.mec",
            sources.into(),
            Array::new().into(),
            Object::new().into(),
        )
        .unwrap();
        assert_eq!(
            restored.bootstrap.document_base().nominal_origin(),
            Some(&origin),
        );
        restored.repl_replace_source(enum_source).unwrap();
        let recovered = restored.repl.session.symbol("my-color").unwrap().unwrap();
        assert_eq!(recovered.schema_key(), original.schema_key());
        document.stop().unwrap();
        restored.stop().unwrap();
    }

    #[cfg(all(
        feature = "browser_host_timer",
        feature = "browser_host_scene",
        feature = "browser_compute"
    ))]
    #[wasm_bindgen_test]
    fn ekf_filter_recurrence_runs_in_browser_wasm() {
        super::tests::assert_ekf_scene_advances_on_every_resident_timer_packet();
    }

    #[wasm_bindgen_test]
    fn browser_document_profile_executes_both_stats_sum_axes() {
        let source = "+> stats\n\
             column-totals := stats/sum/column([1.0 2.0; 3.0 4.0])\n\
             row-totals := stats/sum/row([1.0 2.0; 3.0 4.0])\n\
             row-totals";
        let encoded = encoded_document(source);
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_eq!(
            document
                .repl
                .session
                .symbol("column-totals")
                .unwrap()
                .unwrap()
                .format_canonical_inline(),
            "[3; 7]",
        );
        assert_eq!(
            document
                .repl
                .session
                .symbol("row-totals")
                .unwrap()
                .unwrap()
                .format_canonical_inline(),
            "[4 6]",
        );
    }

    fn rendered_text(value: &JsValue) -> String {
        Reflect::get(value, &JsValue::from_str("inlineHtml"))
            .unwrap()
            .as_string()
            .unwrap()
    }

    fn program_selection(document: &mut WasmDocument) -> String {
        Reflect::get(
            &document.rendered_program_output().unwrap(),
            &JsValue::from_str("selectionToken"),
        )
        .unwrap()
        .as_string()
        .unwrap()
    }

    fn edit_all(document: &mut WasmDocument, source: &str) {
        let end = document.repl_source().encode_utf16().count() as u32;
        document.repl_apply_edit(0, end, source).unwrap();
        assert_eq!(document.repl_source(), source);
    }

    #[wasm_bindgen_test]
    fn editor_result_snapshots_are_owned_and_bounded_without_releasing_user_selections() {
        let original = "value := [1 2; 3 4]\nvalue\n";
        let mut document = WasmDocument::from_encoded(&encoded_document(original)).unwrap();
        let selected = document.repl_select_symbol("value", false).unwrap();
        let user_token = Reflect::get(&selected, &JsValue::from_str("identity"))
            .unwrap()
            .as_string()
            .unwrap();
        let user_value = document
            .repl
            .session
            .retained_selection(&user_token)
            .unwrap()
            .1
            .format_canonical_inline();
        let mut internal_tokens = Vec::new();
        for cycle in 0..3 {
            let owned = program_selection(&mut document);
            internal_tokens.push(owned.clone());
            for index in 2..12 {
                let source = format!(
                    "value := [{index} {}; {} {}]\nvalue\n",
                    index + 1,
                    index + 2,
                    index + 3
                );
                edit_all(&mut document, &source);
                assert_eq!(program_selection(&mut document), owned);
                assert_eq!(document.repl.session.retained_selection_count(), 2);
                assert_eq!(
                    internal_tokens
                        .iter()
                        .filter(|token| document.repl.session.retained_selection(token).is_some())
                        .count(),
                    1
                );
                assert_eq!(
                    document
                        .repl
                        .session
                        .retained_selection(&user_token)
                        .unwrap()
                        .1
                        .format_canonical_inline(),
                    user_value
                );
            }
            let accepted = document.repl_source();
            assert!(document.repl_apply_edit(0, 0, "[\n").is_err());
            assert_eq!(document.repl_source(), accepted);
            assert_eq!(program_selection(&mut document), owned);
            edit_all(&mut document, "");
            assert!(document.rendered_program_output().unwrap().is_null());
            assert!(document.repl.session.retained_selection(&owned).is_none());
            assert_eq!(document.repl.session.retained_selection_count(), 1);
            assert_eq!(
                internal_tokens
                    .iter()
                    .filter(|token| document.repl.session.retained_selection(token).is_some())
                    .count(),
                0
            );
            assert_eq!(
                document
                    .repl
                    .session
                    .retained_selection(&user_token)
                    .unwrap()
                    .1
                    .format_canonical_inline(),
                user_value
            );
            edit_all(
                &mut document,
                &format!("value := [{} 2; 3 4]\nvalue\n", cycle + 20),
            );
            assert_ne!(program_selection(&mut document), owned);
        }
        document.repl_select_retained(&user_token, false).unwrap();
        assert_eq!(
            document
                .repl
                .session
                .symbol("ans")
                .unwrap()
                .unwrap()
                .format_canonical_inline(),
            user_value
        );
    }

    #[wasm_bindgen_test]
    fn public_documentation_response_keeps_live_fences_without_a_terminal_newline() {
        for delimiter in ["```", "~~~"] {
            let mut document =
                WasmDocument::from_encoded(&encoded_document("answer := 1\nanswer")).unwrap();
            let request = document.repl_invoke(":docs browser/test").unwrap();
            let request_id = Reflect::get(&request, &JsValue::from_str("hostRequestId"))
                .unwrap()
                .as_string()
                .unwrap();
            let submitted =
                format!("Result {{answer + 1}}.\n\n{delimiter}mech\nanswer + 2\n{delimiter}");
            let response = document
                .repl_load_documentation(&request_id, "browser/test", &submitted)
                .unwrap();
            let html = Reflect::get(&response, &JsValue::from_str("html"))
                .unwrap()
                .as_string()
                .unwrap();
            assert!(html.contains("class='mech-inline-mech-code'"), "{html}");
            assert!(html.contains("class='mech-block-output'"), "{html}");
            let addresses = html
                .split("id='")
                .skip(1)
                .filter_map(|suffix| {
                    suffix
                        .split_once(":0'")
                        .and_then(|(address, _)| address.parse::<u64>().ok())
                })
                .collect::<Vec<_>>();
            assert_eq!(addresses.len(), 2, "{html}");
            for (address, expected) in addresses.into_iter().zip(["2", "3"]) {
                assert_eq!(
                    rendered_text(&document.rendered_output(address).unwrap()),
                    expected
                );
            }
        }
    }

    #[wasm_bindgen_test]
    fn repeated_title_fields_transfer_the_existing_slot_on_public_source_edits() {
        for name in [
            "author", "date", "kicker", "section", "summary", "next", "previous", "hero", "Author",
            "DATE", "Kicker", "Section", "SUMMARY", "Next", "Previous", "Hero",
        ] {
            let field = |value: &str| {
                if name.eq_ignore_ascii_case("hero") {
                    format!("{name}: | ![Result {value}](hero.svg) |\n")
                } else {
                    format!("{name}: {value}\n")
                }
            };
            for replace in [false, true] {
                let original = format!(
                    "Document\n========\n{}========\n\nVisible {{1}}.\n",
                    field("{1}")
                );
                let mut document =
                    WasmDocument::from_encoded(&encoded_document(&original)).unwrap();
                let ids = document.bootstrap.presentation_output_ids.clone();
                assert_eq!(ids.len(), 2, "{name}");
                let slot = ids[0];
                let body = ids[1];
                let check = |document: &WasmDocument, expected: Option<&str>| {
                    let rendered = document.rendered_output(slot).unwrap();
                    if let Some(expected) = expected {
                        assert_eq!(
                            rendered_text(&rendered),
                            expected,
                            "{name}, replace={replace}"
                        );
                    } else {
                        assert!(rendered.is_null(), "{name}, replace={replace}");
                    }
                    assert_eq!(rendered_text(&document.rendered_output(body).unwrap()), "1");
                };
                check(&document, Some("1"));
                for value in ["{2}", "{3}", "literal", "{4}"] {
                    let source = document.repl_source();
                    let insertion = source.rfind("========").unwrap();
                    let addition = field(value);
                    if replace {
                        let next = format!(
                            "{}{}{}",
                            &source[..insertion],
                            addition,
                            &source[insertion..]
                        );
                        document.repl_replace_source(&next).unwrap();
                    } else {
                        document
                            .repl_apply_edit(insertion as u32, insertion as u32, &addition)
                            .unwrap();
                    }
                    check(
                        &document,
                        (value != "literal").then(|| &value[1..value.len() - 1]),
                    );
                    let accepted = document.repl_source();
                    assert!(document.repl_apply_edit(0, 0, "[\n").is_err());
                    assert_eq!(document.repl_source(), accepted);
                    check(
                        &document,
                        (value != "literal").then(|| &value[1..value.len() - 1]),
                    );
                }
                for (removed, expected) in [
                    ("{4}", None),
                    ("literal", Some("3")),
                    ("{3}", Some("2")),
                    ("{2}", Some("1")),
                ] {
                    let source = document.repl_source();
                    let removal = field(removed);
                    let start = source.find(&removal).unwrap();
                    if replace {
                        let next = source.replacen(&removal, "", 1);
                        document.repl_replace_source(&next).unwrap();
                    } else {
                        document
                            .repl_apply_edit(start as u32, (start + removal.len()) as u32, "")
                            .unwrap();
                    }
                    check(&document, expected);
                }
            }
        }
    }

    #[wasm_bindgen_test]
    fn suppressed_fence_keeps_its_address_across_boundary_insertions() {
        for prefix in ["Prose before the fence.\n\n", "~~~mech\n22\n~~~\n\n"] {
            let original = "~~~mech\n11\n~~~\n";
            let mut document = WasmDocument::from_encoded(&encoded_document(original)).unwrap();
            let id = document.bootstrap.presentation_output_ids[0];
            document.repl_apply_edit(7, 7, "{output: false}").unwrap();
            let body = document.repl_source().find("11").unwrap() as u32;
            document.repl_apply_edit(body, body + 2, "22").unwrap();
            document.repl_apply_edit(0, 0, prefix).unwrap();
            let end = document.repl_source().len() as u32;
            document
                .repl_apply_edit(end, end, "\nProse after the fence.\n")
                .unwrap();
            let accepted = document.repl_source();
            assert!(document.repl_apply_edit(0, 0, "[\n").is_err());
            assert_eq!(document.repl_source(), accepted);
            assert!(document.rendered_output(id).unwrap().is_null());
            let start = accepted.find("{output: false}").unwrap() as u32;
            document.repl_apply_edit(start, start + 15, "").unwrap();
            assert_eq!(rendered_text(&document.rendered_output(id).unwrap()), "22");
        }
    }

    #[wasm_bindgen_test]
    fn unpublished_fence_occurrence_keeps_its_original_address_through_body_edits() {
        let original = "~~~mech\n11\n~~~\n";
        let mut document = WasmDocument::from_encoded(&encoded_document(original)).unwrap();
        let id = document.bootstrap.presentation_output_ids[0];
        assert_eq!(rendered_text(&document.rendered_output(id).unwrap()), "11");
        document.repl_apply_edit(7, 7, "{output: false}").unwrap();
        assert!(document.rendered_output(id).unwrap().is_null());
        let suppressed = document.repl_source();
        let body = suppressed.find("11").unwrap() as u32;
        document.repl_apply_edit(body, body + 2, "22").unwrap();
        assert!(document.repl_apply_edit(body, body + 2, "[").is_err());
        assert!(document.rendered_output(id).unwrap().is_null());
        assert!(document.repl_source().contains("22"));
        document.repl_apply_edit(7, 22, "").unwrap();
        assert_eq!(rendered_text(&document.rendered_output(id).unwrap()), "22");
        // A full occurrence replacement while unpublished must not redirect its old placeholder.
        document.repl_apply_edit(7, 7, "{output: false}").unwrap();
        edit_all(&mut document, "~~~mech\n33\n~~~\n");
        assert!(document.rendered_output(id).unwrap().is_null());
        let changed = document.repl_source();
        document
            .repl_apply_edit(
                changed.len() as u32,
                changed.len() as u32,
                "\n~~~mech\n22\n~~~\n\n~~~mech\n22\n~~~\n",
            )
            .unwrap();
        assert!(document.rendered_output(id).unwrap().is_null());
    }

    #[wasm_bindgen_test]
    fn console_clear_and_reset_commit_one_document_boundary_with_the_runtime() {
        let original = "first := 1\nsecond := 2\nsecond\n\nVisible {second}.\n";
        let mut document = WasmDocument::from_encoded(&encoded_document(original)).unwrap();
        let address = document.bootstrap.presentation_output_ids[0];
        let check = |document: &mut WasmDocument, first: bool, extra: bool| {
            assert_eq!(
                document.repl.session.symbol("first").unwrap().is_some(),
                first
            );
            assert_eq!(
                document.repl.session.symbol("extra").unwrap().is_some(),
                extra
            );
            assert_eq!(
                rendered_text(&document.rendered_program_output().unwrap()),
                "2"
            );
            assert_eq!(
                rendered_text(&document.rendered_output(address).unwrap()),
                "2"
            );
            assert_eq!(
                document.bootstrap.presentation_state.borrow().active.source,
                document.repl_source()
            );
        };
        check(&mut document, true, false);
        document.repl_invoke("extra := 99").unwrap();
        check(&mut document, true, true);
        document.repl_invoke(":clear first").unwrap();
        check(&mut document, false, true);
        assert!(document.repl_source().trim_end().ends_with("extra := 99"));
        assert!(!document.bootstrap.initial_repl_source().contains("extra"));
        let accepted = document.repl_source();
        let boundary = document.bootstrap.initial_repl_source();
        document.repl_invoke(":clear second").unwrap();
        assert_eq!(document.repl_source(), accepted);
        assert_eq!(document.bootstrap.initial_repl_source(), boundary);
        check(&mut document, false, true);
        document.repl_invoke(":reset").unwrap();
        check(&mut document, true, false);
        assert_eq!(document.repl_source(), original);
        assert_eq!(document.bootstrap.initial_repl_source(), original);
        document.repl_invoke("extra := 99").unwrap();
        document.repl_invoke(":clear extra").unwrap();
        check(&mut document, true, false);
        document.repl_invoke("extra := 99").unwrap();
        check(&mut document, true, true);
    }

    #[wasm_bindgen_test]
    fn imported_document_output_survives_console_and_editor_changes() {
        let source = "+> ./dep.mec\nanswer := dep/value + 1.0\nanswer\n\nResult {answer}.\n";
        let encoded = BrowserDocumentPayload::new("main.mec", source)
            .unwrap()
            .with_presentation_output_ids(
                mech_runtime::canonical_document_presentation_output_ids(
                    &SourceDocument::parse_resolved(
                        "main.mec",
                        mech_syntax::document::Revision(0),
                        source,
                        mech_syntax::document::ParseConfig::default(),
                    )
                    .unwrap()
                    .document(),
                )
                .unwrap(),
            )
            .encode()
            .unwrap();
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(source),
        )
        .unwrap();
        Reflect::set(
            &sources,
            &JsValue::from_str("dep.mec"),
            &JsValue::from_str("value := 41.0\n<+ value\n"),
        )
        .unwrap();
        let mut document =
            WasmDocument::from_encoded_with_sources(&encoded, "main.mec", sources.into()).unwrap();
        let address = document.bootstrap.presentation_output_ids[0];
        document.repl_invoke("next := answer + 1.0\nnext").unwrap();
        assert_eq!(
            rendered_text(&document.rendered_program_output().unwrap()),
            "42"
        );
        assert_eq!(
            rendered_text(&document.rendered_output(address).unwrap()),
            "42"
        );
        document
            .repl_replace_source(&source.replace("+ 1.0", "+ 2.0"))
            .unwrap();
        document.repl_invoke("next := answer + 1.0\nnext").unwrap();
        assert_eq!(
            rendered_text(&document.rendered_program_output().unwrap()),
            "43"
        );
        assert_eq!(
            rendered_text(&document.rendered_output(address).unwrap()),
            "43"
        );
    }

    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    #[wasm_bindgen_test]
    fn static_compute_document_uses_the_registered_mixed_runtime_and_bridge() {
        let config = r#"config := {
  hosts: [{ name: "filters" provider: "compute" settings: { region: "calculation" backend: "cpu" } } { name: "mouse" provider: "pointer" settings: {} }]
  run: { paths: ["main.mec"] grants: [{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] } { target: "mouse/frame" operations: ["read"] paths: ["pulse"] }] }
}"#;
        let source = "@mouse := pointer://mouse/frame{:read(pulse)}\npulse := @mouse/pulse\n@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- pulse\nanswer := @compute/sample/result\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let document =
            parse_config_document("mech.mcfg", config, ConfigProfileOptions::default()).unwrap();
        let authority = BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts: document.hosts.clone(),
            run_grants: document.run.as_ref().unwrap().grants.clone(),
        };
        install_served_authority(&authority);
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(source),
        )
        .unwrap();
        let documents = Object::new();
        Reflect::set(
            &documents,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(&encoded_document_at("main.mec", source)),
        )
        .unwrap();
        let roots = Array::new();
        roots.push(&JsValue::from_str("main.mec"));
        let mut project = WasmProject::from_served_documents(
            config,
            sources.into(),
            documents.into(),
            roots.into(),
            Array::new().into(),
            JsValue::NULL,
        )
        .unwrap();
        assert_eq!(project.compute_backend(), "cpu-scalar");
        assert!(!project.compute_manifest().is_null());
        assert_eq!(project.compute_generation(), "1");
        assert_eq!(
            project
                .runtime
                .root_symbol_value("answer")
                .unwrap()
                .format_canonical_inline(),
            "0"
        );
        assert!(project.runtime.step_active_program().is_err());
        assert!(!project.has_pointer_input());
        project.start().unwrap();
        assert!(project.has_pointer_input());
        project.pointer_input(0.25, -0.5, false, 0.016).unwrap();
        for _ in 0..4 {
            project.frame(8).unwrap();
        }
        assert_eq!(
            project
                .runtime
                .root_symbol_value("answer")
                .unwrap()
                .format_canonical_inline(),
            "1"
        );
        let before = project
            .runtime
            .program_execution_info()
            .resident_accepted_turns;
        let frame = project.frame(8).unwrap();
        assert!(
            Reflect::get(&frame, &JsValue::from_str("computeCommand"))
                .unwrap()
                .is_null()
        );
        assert_eq!(
            project
                .runtime
                .program_execution_info()
                .resident_accepted_turns,
            before
        );
        project.stop().unwrap();
        assert!(!project.has_pointer_input());

        // The ordinary source constructor must select the same mixed owner,
        // not depend on the served-document entry point to special-case it.
        let mut project = WasmProject::from_project_sources(
            document,
            HashMap::from([("main.mec".to_owned(), source.to_owned())]),
            Vec::new(),
            HashMap::new(),
        )
        .unwrap();
        assert_eq!(project.compute_backend(), "cpu-scalar");
        assert_eq!(
            project
                .runtime
                .root_symbol_value("answer")
                .unwrap()
                .format_canonical_inline(),
            "0"
        );
        assert!(project.runtime.step_active_program().is_err());
        project.start().unwrap();
        assert!(project.has_pointer_input());
        project.pointer_input(0.25, -0.5, false, 0.016).unwrap();
        for _ in 0..4 {
            project.frame(8).unwrap();
        }
        assert_eq!(
            project
                .runtime
                .root_symbol_value("answer")
                .unwrap()
                .format_canonical_inline(),
            "1"
        );
        project.stop().unwrap();
    }

    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    #[wasm_bindgen_test]
    fn served_static_and_document_compute_metadata_follow_issued_settings() {
        let source = "@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- 1\nanswer := @compute/sample/result\nanswer\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let window = web_sys::window().unwrap();
        let previous =
            Reflect::get(window.as_ref(), &JsValue::from_str("__MECH_GPU_AVAILABLE")).unwrap();
        Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__MECH_GPU_AVAILABLE"),
            &JsValue::TRUE,
        )
        .unwrap();
        for (requested, issued, expected) in [
            ("cpu", "wgpu", "wgpu"),
            ("wgpu", "cpu", "cpu-scalar"),
            ("auto", "cpu", "cpu-scalar"),
        ] {
            let config = format!(
                r#"config := {{
  hosts: [{{ name: "filters" provider: "compute" settings: {{ region: "calculation" backend: "{requested}" }} }}]
  run: {{ paths: ["main.mec"] grants: [{{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] }}] }}
}}"#
            );
            let document =
                parse_config_document("mech.mcfg", &config, ConfigProfileOptions::default())
                    .unwrap();
            let mut authority = BrowserRuntimeInjectionConfig {
                runtime: mech_browser::BrowserHostRuntimeConfig::from(
                    &mech_runtime::RuntimeConfig::default(),
                ),
                hosts: document.hosts.clone(),
                run_grants: document.run.as_ref().unwrap().grants.clone(),
            };
            let ConfigValue::Map(settings) = &mut authority.hosts[0].settings else {
                unreachable!()
            };
            settings.insert("backend".to_owned(), ConfigValue::String(issued.to_owned()));
            install_served_authority(&authority);
            let encoded = encoded_document_at("main.mec", source);
            let sources = Object::new();
            Reflect::set(
                &sources,
                &JsValue::from_str("main.mec"),
                &JsValue::from_str(source),
            )
            .unwrap();
            let documents = Object::new();
            Reflect::set(
                &documents,
                &JsValue::from_str("main.mec"),
                &JsValue::from_str(&encoded),
            )
            .unwrap();
            let roots = Array::new();
            roots.push(&JsValue::from_str("main.mec"));
            let mut project = WasmProject::from_served_documents(
                &config,
                sources.clone().into(),
                documents.into(),
                roots.into(),
                Array::new().into(),
                JsValue::NULL,
            )
            .unwrap();
            assert_eq!(project.compute_backend(), expected);
            assert_eq!(
                Reflect::get(
                    &project.compute_manifest(),
                    &JsValue::from_str("requestedBackend")
                )
                .unwrap()
                .as_string()
                .as_deref(),
                Some(issued)
            );
            assert_eq!(
                project
                    .runtime
                    .program_execution_info()
                    .resident_accepted_turns,
                0
            );
            project.stop().unwrap();
            let mut document =
                WasmDocument::from_served_encoded(&encoded, "main.mec", &config, sources.into())
                    .unwrap();
            assert_eq!(document.compute_backend(), expected);
            assert_eq!(
                Reflect::get(
                    &document.compute_manifest(),
                    &JsValue::from_str("requestedBackend")
                )
                .unwrap()
                .as_string()
                .as_deref(),
                Some(issued)
            );
            assert_eq!(
                document
                    .runtime()
                    .unwrap()
                    .program_execution_info()
                    .resident_accepted_turns,
                0
            );
            document.stop().unwrap();
        }
        Reflect::set(
            window.as_ref(),
            &JsValue::from_str("__MECH_GPU_AVAILABLE"),
            &previous,
        )
        .unwrap();
    }

    #[cfg(all(feature = "browser_compute", feature = "served_project_authority"))]
    #[wasm_bindgen_test]
    fn served_compute_sample_document_constructs_and_accepts_editor_changes() {
        let config = r#"config := {
  hosts: [{ name: "filters" provider: "compute" settings: { region: "calculation" backend: "cpu" } }]
  run: { paths: ["main.mec"] grants: [{ target: "filters/kernel" operations: ["read", "write"] paths: ["sample/result", "turn"] }] }
}"#;
        let source = "@compute := compute://filters/kernel{:write(turn), :read(sample/result)}\n@compute/turn <- 1\nanswer := @compute/sample/result\nanswer\n\nResult {answer}.\n\ncalculation @compute\n-------------------\n~counter := 0f32\ncounter += 1f32\ncounter\n";
        let config_document =
            parse_config_document("mech.mcfg", config, ConfigProfileOptions::default()).unwrap();
        let authority = BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts: config_document.hosts.clone(),
            run_grants: config_document.run.as_ref().unwrap().grants.clone(),
        };
        install_served_authority(&authority);
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str(source),
        )
        .unwrap();
        let mut document = WasmDocument::from_served_encoded(
            &encoded_document_at("main.mec", source),
            "main.mec",
            config,
            sources.into(),
        )
        .unwrap();
        let address = document.bootstrap.presentation_output_ids[0];
        assert_eq!(document.compute_backend(), "cpu-scalar");
        assert!(!document.rendered_output(address).unwrap().is_null());
        assert!(!document.rendered_program_output().unwrap().is_null());
        let generation = document.compute_generation();
        let updated = source.replace(
            "answer := @compute/sample/result",
            "answer := @compute/sample/result + 1",
        );
        document.repl_replace_source(&updated).unwrap();
        assert_ne!(document.compute_generation(), generation);
        assert!(!document.rendered_output(address).unwrap().is_null());
        let start = updated.find("+ 1\n").unwrap();
        document
            .repl_apply_edit(start as u32, (start + 3) as u32, "+ 2")
            .unwrap();
        assert!(
            document
                .repl_source()
                .contains("@compute/sample/result + 2")
        );
        assert!(!document.rendered_program_output().unwrap().is_null());
        document.stop().unwrap();
        Reflect::delete_property(
            &web_sys::window().unwrap(),
            &JsValue::from_str("__MECH_HOST_CONFIG"),
        )
        .unwrap();
    }

    #[wasm_bindgen_test]
    fn title_only_document_binds_canonical_presentation_addresses() {
        let source = "Document\n========\nauthor: {40 + 2}\nhero: | ![Result {41 + 1}](hero.svg) |\n========\n";
        let document = WasmDocument::from_encoded(&encoded_document(source)).unwrap();
        assert_eq!(document.bootstrap.presentation_output_ids.len(), 2);
        for address in &document.bootstrap.presentation_output_ids {
            assert_eq!(
                rendered_text(&document.rendered_output(*address).unwrap()),
                "42"
            );
        }
    }

    fn encoded_document(source: &str) -> String {
        encoded_document_at("document.mec", source)
    }

    fn encoded_document_at(root_specifier: &str, source: &str) -> String {
        let document = SourceDocument::parse_resolved(
            "runtime:interactive",
            mech_syntax::document::Revision(0),
            source,
            mech_syntax::document::ParseConfig::default(),
        )
        .unwrap();
        let presentation_output_ids =
            mech_runtime::canonical_document_presentation_output_ids(&document.document()).unwrap();
        BrowserDocumentPayload::new(root_specifier, source)
            .unwrap()
            .with_presentation_output_ids(presentation_output_ids)
            .encode()
            .unwrap()
    }

    #[cfg(feature = "served_project_authority")]
    fn served_document_authority() -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts: vec![mech_runtime::HostInstanceConfig {
                name: "clock".to_string(),
                provider: "time".to_string(),
                settings: mech_runtime::ConfigValue::Map(Default::default()),
            }],
            run_grants: vec![mech_runtime::RunResourceGrantConfig {
                target: "clock/clock".to_string(),
                operations: vec!["read".to_string()],
                paths: vec!["second".to_string()],
            }],
        }
    }

    #[cfg(feature = "served_project_authority")]
    fn served_document_config() -> &'static str {
        r#"config := {
  hosts: [{ name: "clock" provider: "time" settings: {} }]
  run: {
    paths: ["docs/main.mec"]
    grants: [{ target: "clock/clock" operations: ["read"] paths: ["second"] }]
  }
}"#
    }

    #[cfg(feature = "served_project_authority")]
    fn served_document_source() -> &'static str {
        "+> ./math.mec\n~configured-answer := 0\nconfigured-answer += math/value\nconfigured-answer\n"
    }

    #[cfg(feature = "served_project_authority")]
    fn served_document_sources() -> JsValue {
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("docs/main.mec"),
            &JsValue::from_str(served_document_source()),
        )
        .unwrap();
        Reflect::set(
            &sources,
            &JsValue::from_str("docs/math.mec"),
            &JsValue::from_str("value := 41\n<+ value\n"),
        )
        .unwrap();
        sources.into()
    }

    #[cfg(feature = "served_project_authority")]
    fn install_served_authority(authority: &BrowserRuntimeInjectionConfig) {
        Reflect::set(
            &web_sys::window().unwrap(),
            &JsValue::from_str("__MECH_HOST_CONFIG"),
            &serde_wasm_bindgen::to_value(authority).unwrap(),
        )
        .unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    fn assert_configured_answer(document: &WasmDocument) {
        let configured_answer = document.rendered_symbol("configured-answer").unwrap();
        assert_eq!(
            Reflect::get(&configured_answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("41"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_project_reports_served_project_capabilities() {
        assert_eq!(
            WasmProject::supports_served_authority(),
            cfg!(feature = "served_project_authority")
        );
        assert_eq!(
            WasmProject::supports_served_document_resolutions(),
            cfg!(feature = "served_project_authority")
        );
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen_test]
    fn wasm_document_reset_reuses_validated_served_authority() {
        let authority = served_document_authority();
        install_served_authority(&authority);
        let encoded = encoded_document_at("docs/main.mec", served_document_source());
        let mut document = WasmDocument::from_served_encoded(
            &encoded,
            "docs/main.mec",
            served_document_config(),
            served_document_sources(),
        )
        .unwrap();

        document.reset(&encoded).unwrap();
        assert_configured_answer(&document);
        Reflect::delete_property(
            &web_sys::window().unwrap(),
            &JsValue::from_str("__MECH_HOST_CONFIG"),
        )
        .unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen_test]
    fn wasm_document_reset_does_not_adopt_replaced_global_authority() {
        let authority = served_document_authority();
        install_served_authority(&authority);
        let encoded = encoded_document_at("docs/main.mec", served_document_source());
        let mut document = WasmDocument::from_served_encoded(
            &encoded,
            "docs/main.mec",
            served_document_config(),
            served_document_sources(),
        )
        .unwrap();

        let replacement = BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(
                &mech_runtime::RuntimeConfig::default(),
            ),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        };
        install_served_authority(&replacement);
        document.reset(&encoded).unwrap();
        assert_configured_answer(&document);
        Reflect::delete_property(
            &web_sys::window().unwrap(),
            &JsValue::from_str("__MECH_HOST_CONFIG"),
        )
        .unwrap();
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen_test]
    fn wasm_document_reset_survives_removed_global_authority() {
        let authority = served_document_authority();
        install_served_authority(&authority);
        let encoded = encoded_document_at("docs/main.mec", served_document_source());
        let mut document = WasmDocument::from_served_encoded(
            &encoded,
            "docs/main.mec",
            served_document_config(),
            served_document_sources(),
        )
        .unwrap();

        Reflect::delete_property(
            &web_sys::window().unwrap(),
            &JsValue::from_str("__MECH_HOST_CONFIG"),
        )
        .unwrap();
        document.reset(&encoded).unwrap();
        assert_configured_answer(&document);
    }

    #[wasm_bindgen_test]
    fn generic_project_starts_and_stops_idempotently() {
        let config = r#"config := { hosts: [] run: { paths: ["main.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("main.mec"),
            &JsValue::from_str("~x := 0\nx += 1\nx"),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        project.start().unwrap();
        project.start().unwrap();
        project.stop().unwrap();
        project.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn encoded_document_executes_and_exposes_detached_render_queries() {
        let encoded = test_document_payload("document.mec", "~answer := 0\nanswer += 42\nanswer")
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let rendered = document.rendered_symbol("answer").unwrap();
        assert!(!rendered.is_null());
        assert_eq!(
            Reflect::get(&rendered, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("42"),
        );
        assert!(document.rendered_output(u64::MAX).unwrap().is_null());
        let selection = document.repl_select_symbol("answer", true).unwrap();
        let identity = Reflect::get(&selection, &JsValue::from_str("identity"))
            .unwrap()
            .as_string()
            .expect("resident selections must expose their binding identity");
        assert!(identity.starts_with("selection:"), "{identity}");
        let ans_selection = document.repl_select_symbol("ans", true).unwrap();
        assert_eq!(
            Reflect::get(&ans_selection, &JsValue::from_str("identity"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some(identity.as_str()),
            "ans must preserve the selected value identity for popup deduplication",
        );
        document.start().unwrap();
        assert!(document.frame(1).is_ok());
        document.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn wasm_inline_values_use_html_escaped_canonical_mech_strings() {
        let value =
            mech_core::ValueCell::from_exact("a\"b\\c\nα\u{2028}line\u{2029}paragraph".to_string())
                .unwrap()
                .snapshot()
                .unwrap();
        let snapshot = mech_runtime::RuntimeValueSnapshot::try_from(value).unwrap();
        let rendered = rendered_value(snapshot, 500).unwrap();
        assert_eq!(
            Reflect::get(&rendered, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("&quot;a\\&quot;b\\\\c\\nα\\u{2028}line\\u{2029}paragraph&quot;"),
        );
    }

    #[wasm_bindgen_test]
    fn configured_preview_limit_bounds_document_rows_and_selection_popups() {
        let encoded = encoded_document("x := 1..=10\nx");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        document.repl_set_value_element_limit(2).unwrap();

        let symbol = document.rendered_symbol("x").unwrap();
        let inline = Reflect::get(&symbol, &JsValue::from_str("inlineHtml"))
            .unwrap()
            .as_string()
            .unwrap();
        let block = Reflect::get(&symbol, &JsValue::from_str("blockHtml"))
            .unwrap()
            .as_string()
            .unwrap();
        assert!(inline.contains('…'), "{inline}");
        assert!(block.contains("mech-value-elided"), "{block}");
        assert!(!block.contains(">10<"), "{block}");

        let selection = document.repl_select_symbol("x", true).unwrap();
        let rendered = Reflect::get(&selection, &JsValue::from_str("rendered")).unwrap();
        let popup = Reflect::get(&rendered, &JsValue::from_str("blockHtml"))
            .unwrap()
            .as_string()
            .unwrap();
        assert!(popup.contains("mech-value-elided"), "{popup}");
        assert!(!popup.contains(">10<"), "{popup}");
    }

    #[wasm_bindgen_test]
    fn encoded_inline_document_executes_in_the_resident_browser_product() {
        let encoded =
            encoded_document("The document evaluates {answer + 1} inline.\n\nanswer := 41");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let answer = document.rendered_symbol("answer").unwrap();
        assert_eq!(
            Reflect::get(&answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("41"),
        );
        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
        document.start().unwrap();
        assert!(document.frame(1).is_ok());
        document.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn wasm_document_reset_restores_initial_program() {
        let initial = encoded_document("~answer := 0\nanswer += 1\nanswer");
        let mut document = WasmDocument::from_encoded(&initial).unwrap();
        let replacement = encoded_document("~answer := 0\nanswer += 7\nanswer");

        document.reset(&replacement).unwrap();

        let answer = document.rendered_symbol("answer").unwrap();
        assert_eq!(
            Reflect::get(&answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("7"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_document_reset_retires_old_step_and_host_request_ownership() {
        let initial = encoded_document("~counter := 0\ncounter += 1\ncounter");
        let replacement = encoded_document("~counter := 0\ncounter += 7\ncounter");
        let mut document = WasmDocument::from_encoded(&initial).unwrap();

        let old_step = document.repl_invoke(":step 2").unwrap();
        let old_step_id = Reflect::get(&old_step, &JsValue::from_str("stepRequestId"))
            .unwrap()
            .as_string()
            .expect("pending cooperative step must expose its request id");
        document.reset(&replacement).unwrap();
        let new_step = document.repl_invoke(":step 2").unwrap();
        let new_step_id = Reflect::get(&new_step, &JsValue::from_str("stepRequestId"))
            .unwrap()
            .as_string()
            .expect("replacement cooperative step must expose its request id");
        assert_ne!(old_step_id, new_step_id);
        document.repl.session.emit_message_diagnostic(
            mech_runtime::Severity::Info,
            mech_runtime::DiagnosticPhase::Host,
            "CurrentStepOwnerMarker",
            "owned by the current step request",
        );
        assert!(document.repl_continue_step(1, &old_step_id).is_err());
        let still_pending = document.repl_invoke(":step 1").unwrap();
        assert_eq!(
            Reflect::get(&still_pending, &JsValue::from_str("remaining"))
                .unwrap()
                .as_f64(),
            Some(2.0),
            "a stale continuation must not advance the replacement request",
        );
        let still_pending_events: Vec<mech_runtime::MechEventEnvelope> =
            serde_wasm_bindgen::from_value(
                Reflect::get(&still_pending, &JsValue::from_str("events")).unwrap(),
            )
            .unwrap();
        assert!(still_pending_events.iter().any(|event| matches!(
            &event.event,
            mech_runtime::MechEvent::Diagnostic(diagnostic)
                if diagnostic.code.as_deref() == Some("CurrentStepOwnerMarker")
        )));
        let continued = document.repl_continue_step(1, &new_step_id).unwrap();
        assert_eq!(
            Reflect::get(&continued, &JsValue::from_str("remaining"))
                .unwrap()
                .as_f64(),
            Some(1.0),
        );

        document.repl_interrupt().unwrap();
        let old_host = document.repl_invoke(":docs browser/old").unwrap();
        let old_host_id = Reflect::get(&old_host, &JsValue::from_str("hostRequestId"))
            .unwrap()
            .as_string()
            .expect("pending host request must expose its request id");
        document.repl_interrupt().unwrap();
        let new_host = document.repl_invoke(":docs browser/new").unwrap();
        let new_host_id = Reflect::get(&new_host, &JsValue::from_str("hostRequestId"))
            .unwrap()
            .as_string()
            .expect("replacement host request must expose its request id");
        document.repl.session.emit_message_diagnostic(
            mech_runtime::Severity::Info,
            mech_runtime::DiagnosticPhase::Host,
            "CurrentOwnerMarker",
            "owned by the current request",
        );
        assert!(document.repl_finish_host_request(&old_host_id).is_err());
        assert!(document.repl.host_request_pending(&new_host_id));
        let finished = document.repl_finish_host_request(&new_host_id).unwrap();
        let events: Vec<mech_runtime::MechEventEnvelope> = serde_wasm_bindgen::from_value(
            Reflect::get(&finished, &JsValue::from_str("events")).unwrap(),
        )
        .unwrap();
        assert!(events.iter().any(|event| matches!(
            &event.event,
            mech_runtime::MechEvent::Diagnostic(diagnostic)
                if diagnostic.code.as_deref() == Some("CurrentOwnerMarker")
        )));
    }

    #[wasm_bindgen_test]
    fn wasm_document_step_rejects_counts_outside_the_shared_resident_limit() {
        let encoded = encoded_document("~answer := 0\nanswer += 1\nanswer");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        assert!(document.step(0).is_err());
        assert!(document.step(MAX_RESIDENT_STEP_COUNT + 1).is_err());
    }

    #[wasm_bindgen_test]
    fn wasm_document_rendered_symbols_returns_detached_rows() {
        let initial = encoded_document("~answer := 0\nanswer += 42\nanswer");
        let replacement = encoded_document("~answer := 0\nanswer += 7\nanswer");
        let mut document = WasmDocument::from_encoded(&initial).unwrap();
        let requested = Array::new();
        requested.push(&JsValue::from_str("answer"));
        let rows = Array::from(&document.rendered_symbols(requested.into()).unwrap());
        assert_eq!(rows.length(), 1);
        let row = rows.get(0);
        assert_eq!(
            Reflect::get(&row, &JsValue::from_str("name"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("answer"),
        );
        assert_eq!(
            Reflect::get(&row, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("42"),
        );

        document.reset(&replacement).unwrap();
        assert_eq!(
            Reflect::get(&row, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("42"),
            "rendered symbol rows must not retain a live runtime value",
        );
    }

    #[wasm_bindgen_test]
    fn encoded_fizzbuzz_document_executes_in_the_resident_browser_product() {
        let source = include_str!("../../../examples/working/fizzbuzz.mec");
        let output_id = 29_884_140_763_677_669;
        assert_eq!(
            output_id, 29_884_140_763_677_669,
            "the WASM output key must match the native formatter key",
        );

        let encoded = BrowserDocumentPayload::new("document.mec", source)
            .unwrap()
            .with_presentation_output_ids([output_id])
            .encode()
            .unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
        assert!(document.rendered_symbol("first-fifteen!").is_err());
        let invariant = document.rendered_document_value("first-fifteen!").unwrap();
        assert_eq!(
            Reflect::get(&invariant, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("true"),
        );
        let output = document.rendered_output(output_id).unwrap();
        assert_eq!(
            Reflect::get(&output, &JsValue::from_str("outputId"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("29884140763677669"),
            "u64 output identities must cross JavaScript losslessly",
        );
        let block_html = Reflect::get(&output, &JsValue::from_str("blockHtml"))
            .unwrap()
            .as_string()
            .expect("the FizzBuzz source output must render as HTML");
        assert!(block_html.contains("✨🐝"), "{block_html}");
        let program_output = document.rendered_program_output().unwrap();
        assert!(
            Reflect::get(&program_output, &JsValue::from_str("name"))
                .unwrap()
                .is_null(),
            "integrity constraints must not replace the browser program output",
        );
        document.start().unwrap();
        assert!(document.frame(1).is_ok());
        document.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn factorial_document_exposes_its_unfenced_final_statement() {
        let encoded = encoded_document(include_str!("../../../examples/working/factorial.mec"));
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let output = document.rendered_program_output().unwrap();

        assert!(
            Reflect::get(&output, &JsValue::from_str("name"))
                .unwrap()
                .is_null(),
        );
        assert_eq!(
            Reflect::get(&output, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("120"),
        );
        let selection_token = Reflect::get(&output, &JsValue::from_str("selectionToken"))
            .unwrap()
            .as_string()
            .expect("detached program output must expose a selection token");

        document.repl_invoke("1 + 1").unwrap();
        let after_repl = document.rendered_program_output().unwrap();
        assert_eq!(
            Reflect::get(&after_repl, &JsValue::from_str("selectionToken"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some(selection_token.as_str()),
            "REPL evaluation must not replace the document program output identity",
        );
        assert_eq!(
            Reflect::get(&after_repl, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("120"),
            "REPL evaluation must not replace the document program output value",
        );
        document
            .repl_select_retained(&selection_token, false)
            .unwrap();
        assert_eq!(
            document
                .repl
                .session
                .symbol("ans")
                .unwrap()
                .unwrap()
                .to_string(),
            "120",
        );
    }

    #[wasm_bindgen_test]
    fn fixed_program_output_resolves_its_live_value_after_steps() {
        let encoded = encoded_document("~answer := 0\nanswer += 1\nanswer");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let initial = document.rendered_program_output().unwrap();
        let selection_token = Reflect::get(&initial, &JsValue::from_str("selectionToken"))
            .unwrap()
            .as_string()
            .unwrap();
        assert_eq!(
            Reflect::get(&initial, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("1"),
        );

        document.repl.step_immediate(2).unwrap();

        let stepped = document.rendered_program_output().unwrap();
        assert_eq!(
            Reflect::get(&stepped, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("3"),
        );
        assert_eq!(
            Reflect::get(&stepped, &JsValue::from_str("selectionToken"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some(selection_token.as_str()),
            "live updates must retain the fixed output identity",
        );
        document
            .repl_select_retained(&selection_token, false)
            .unwrap();
        assert_eq!(
            document
                .repl
                .session
                .symbol("ans")
                .unwrap()
                .unwrap()
                .to_string(),
            "3",
            "the stable selection identity must retain the current live snapshot",
        );
    }

    #[wasm_bindgen_test]
    fn generic_project_frame_respects_input_bound() {
        let config = r#"config := { hosts: [] run: { paths: ["generic-table.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("generic-table.mec"),
            &JsValue::from_str("~x := 0\nx += 1\nx"),
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
            &JsValue::from_str("~x := 0\nx += 1\nx"),
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
            &JsValue::from_str("~x := 0\nx += 1\nx"),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        let result = project.frame(1).unwrap();
        assert_eq!(
            Reflect::get(&result, &JsValue::from_str("rendered"))
                .unwrap()
                .as_f64(),
            Some(0.0)
        );
    }

    #[wasm_bindgen_test]
    fn generic_project_with_time_console_and_scene_runs_clock_source() {
        assert!(
            required_path_strings(include_str!("../../../examples/analog-clock/mech.mcfg")).is_ok()
        );
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[wasm_bindgen_test]
    fn generic_timer_table_scene_is_supported_by_the_resident_browser_product() {
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
            &JsValue::from_str(include_str!(
                "../tests/fixtures/generic-timer-table-scene/table-scene.mec"
            )),
        )
        .unwrap();
        let project = WasmProject::from_sources(config, sources.into());
        assert!(project.is_ok());
        canvas.remove();
    }
}
