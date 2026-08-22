use std::cell::{Cell, RefCell};
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
#[cfg(feature = "browser_host_console")]
use mech_console::{BrowserConsoleHostFactory, ConsoleHostFactory};
use mech_core::{GenericError, MResult, MechError, MechErrorKind, MechSourceCode, OutputId};
use mech_engine::{
    insert_root_document_program_output_capture, root_document_inline_eval_count,
    root_document_output_ids, root_document_program_output_id,
};
use mech_runtime::{
    ConfigProfileOptions, ConfigValue, HostInstanceConfig, InMemorySourceResolver,
    MAX_RESIDENT_STEP_COUNT, MechConfigDocument, MechEvent, MechEventBuffer, MechEventBus,
    MechRuntime, ModuleBuildOptions, ResidentDurabilityPolicy, ResidentRouteFailure,
    ResidentRouteFailureClass, ResolvedSource, RunResourceGrantConfig, RuntimeBuilder,
    RuntimeProgramExecutionInfo, RuntimeProgramLoadOutcome, RuntimeProgramRoute, SourceKind,
    SourceRequest, SourceResolutionEntry, parse_config_document, validate_resident_step_count,
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
#[cfg(feature = "served_project_authority")]
use serde::Deserialize;

#[cfg(feature = "browser_host_dom")]
use crate::host::WasmBrowserDomBackend;

#[wasm_bindgen]
pub struct WasmProject {
    runtime: MechRuntime,
    events: MechEventBus,
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
        Self::from_project_sources(document, source_map, Vec::new())
    }

    #[wasm_bindgen(js_name = fromSourcesWithResolutions)]
    pub fn from_sources_with_resolutions(
        config_source: &str,
        sources: JsValue,
        resolutions: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
        Self::from_project_sources(document, source_map, resolutions)
    }

    fn from_project_sources(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
    ) -> Result<WasmProject, JsValue> {
        validate_compiled_host_providers(&document).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = project_source_resolver_with_resolutions(&source_map, &resolutions)
            .map_err(to_js_error)?;
        let mut runtime = build_runtime(
            &document,
            source_resolver,
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
        )?;
        run_project_sources(&mut runtime, &document).map_err(to_js_error)?;
        Ok(Self::from_runtime(
            runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
        ))
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedSources)]
    pub fn from_served_sources(
        config_source: &str,
        sources: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        Self::from_served_project(document, source_map, Vec::new())
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedSourcesWithResolutions)]
    pub fn from_served_sources_with_resolutions(
        config_source: &str,
        sources: JsValue,
        resolutions: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
        Self::from_served_project(document, source_map, resolutions)
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen(js_name = fromServedBundle)]
    pub fn from_served_bundle(
        config_source: &str,
        sources: JsValue,
        roots: JsValue,
    ) -> Result<WasmProject, JsValue> {
        let mut document = parse_project_config(config_source)?;
        let source_map = source_map_from_js(sources)?;
        replace_bundle_run_paths(&mut document, bundle_roots_from_js(roots)?)?;
        Self::from_served_project(document, source_map, Vec::new())
    }

    #[cfg(feature = "served_project_authority")]
    fn from_served_project(
        document: MechConfigDocument,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
    ) -> Result<WasmProject, JsValue> {
        let authority = served_browser_authority()?;
        validate_served_authority(&document, &authority).map_err(to_js_error)?;
        validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = project_source_resolver_with_resolutions(&source_map, &resolutions)
            .map_err(to_js_error)?;
        let mut runtime = build_runtime_from_authority(
            &document,
            &authority,
            source_resolver,
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
        )?;
        run_project_sources(&mut runtime, &document).map_err(to_js_error)?;
        Ok(Self::from_runtime(
            runtime,
            #[cfg(feature = "browser_host_scene")]
            scenes,
        ))
    }

    fn from_runtime(
        runtime: MechRuntime,
        #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
    ) -> Self {
        Self {
            runtime,
            events: MechEventBus::default(),
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

    /// Reconciles drivers against the retained runtime's current live input
    /// bindings. The runtime operation is idempotent: active drivers remain
    /// active and newly relevant drivers are started exactly once.
    fn refresh_relevant_input_drivers(&mut self) -> Result<(), JsValue> {
        self.runtime.start_input_drivers().map_err(to_js_error)?;
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
pub(crate) enum WasmDocumentBootstrap {
    Detached(DetachedDocumentBootstrap),
    SourceBacked(SourceBackedDocumentBootstrap),
    #[cfg(feature = "served_project_authority")]
    Served(ServedDocumentBootstrap),
}

#[derive(Clone)]
struct DetachedDocumentBootstrap {
    source: SourceBackedDocumentBootstrap,
}

#[derive(Clone)]
struct SourceBackedDocumentBootstrap {
    root_specifier: String,
    source_map: HashMap<String, String>,
    resolutions: Vec<SourceResolutionEntry>,
    tree: mech_core::nodes::Program,
    console_instance: String,
    lifecycle: DocumentRuntimeLifecycle,
}

#[derive(Clone, Default)]
struct DocumentRuntimeLifecycle {
    drivers_started: Rc<Cell<bool>>,
    #[cfg(feature = "browser_host_scene")]
    scenes: Rc<RefCell<BrowserSceneRegistry>>,
    #[cfg(feature = "browser_host_scene")]
    pending_scenes: Rc<RefCell<Option<BrowserSceneRegistry>>>,
}

impl DocumentRuntimeLifecycle {
    fn drivers_started(&self) -> bool {
        self.drivers_started.get()
    }

    fn set_drivers_started(&self, started: bool) {
        self.drivers_started.set(started);
    }

    #[cfg(feature = "browser_host_scene")]
    fn scenes(&self) -> BrowserSceneRegistry {
        self.scenes.borrow().clone()
    }

    #[cfg(feature = "browser_host_scene")]
    fn stage_scenes(&self, scenes: BrowserSceneRegistry) {
        *self.pending_scenes.borrow_mut() = Some(scenes);
    }

    #[cfg(feature = "browser_host_scene")]
    fn commit_scenes(&self) {
        if let Some(scenes) = self.pending_scenes.borrow_mut().take() {
            *self.scenes.borrow_mut() = scenes;
        }
    }

    #[cfg(feature = "browser_host_scene")]
    fn abort_scenes(&self) {
        self.pending_scenes.borrow_mut().take();
    }
}

#[cfg(feature = "served_project_authority")]
#[derive(Clone)]
struct ServedDocumentBootstrap {
    source: SourceBackedDocumentBootstrap,
    config_source: String,
    authority: BrowserRuntimeInjectionConfig,
}

impl WasmDocumentBootstrap {
    fn source(&self) -> &SourceBackedDocumentBootstrap {
        match self {
            Self::Detached(detached) => &detached.source,
            Self::SourceBacked(source) => source,
            #[cfg(feature = "served_project_authority")]
            Self::Served(served) => &served.source,
        }
    }

    pub(crate) fn initial_repl_source(&self) -> String {
        let source = self.source();
        if let Some(text) = source
            .source_map
            .get(&source.root_specifier)
            .filter(|text| !text.trim().is_empty())
        {
            if mech_syntax::parser::parse(text.trim()).ok().as_ref() == Some(&source.tree) {
                return text.clone();
            }
        }
        mech_syntax::formatter::Formatter::new().format(&source.tree)
    }

    pub(crate) fn initial_repl_tree(&self) -> mech_core::nodes::Program {
        self.source().tree.clone()
    }

    fn program_output_id(&self) -> MResult<Option<OutputId>> {
        let (_, output_id) = document_runtime_tree(self.source(), self.source().tree.clone())?;
        Ok(output_id)
    }

    fn interactive_tree(&self, candidate_source: &str) -> MResult<mech_core::nodes::Program> {
        mech_syntax::parser::parse(candidate_source.trim())
    }

    pub(crate) fn console_output_context(&self) -> String {
        format!("console://{}/output", self.source().console_instance)
    }

    pub(crate) fn prepare_commit(&self, runtime: &mut MechRuntime) -> MResult<()> {
        if self.source().lifecycle.drivers_started() {
            runtime.start_input_drivers()?;
        }
        Ok(())
    }

    pub(crate) fn commit(&self) {
        #[cfg(feature = "browser_host_scene")]
        self.source().lifecycle.commit_scenes();
    }

    pub(crate) fn abort(&self) {
        #[cfg(feature = "browser_host_scene")]
        self.source().lifecycle.abort_scenes();
    }
}

pub(crate) fn build_document_repl_runtime(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
) -> MResult<MechRuntime> {
    let tree = bootstrap.source().tree.clone();
    build_document_repl_runtime_for_tree(bootstrap, events, tree).map(|candidate| candidate.runtime)
}

pub(crate) fn activate_document_repl_runtime(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    source: &str,
) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
    let tree = bootstrap.interactive_tree(source)?;
    activate_document_repl_runtime_tree(bootstrap, events, source, tree)
}

pub(crate) fn activate_document_repl_runtime_tree(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    source: &str,
    tree: mech_core::nodes::Program,
) -> MResult<(MechRuntime, RuntimeProgramLoadOutcome)> {
    let mut candidate = build_document_repl_runtime_for_tree(bootstrap, events, tree)?;
    if source.trim().is_empty() {
        #[cfg(feature = "browser_host_scene")]
        bootstrap.source().lifecycle.stage_scenes(candidate.scenes);
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
    let activation = runtime.load_interactive_root_program(
        SourceRequest::new(&bootstrap.source().root_specifier),
        browser_module_options(),
        durability,
    );
    let outcome = match activation {
        Ok(outcome) => outcome,
        Err(error) => {
            let _ = candidate.runtime.shutdown();
            bootstrap.abort();
            return Err(error);
        }
    };
    #[cfg(feature = "browser_host_scene")]
    bootstrap.source().lifecycle.stage_scenes(candidate.scenes);
    Ok((candidate.runtime, outcome))
}

struct DocumentRuntimeCandidate {
    runtime: MechRuntime,
    #[cfg(feature = "browser_host_scene")]
    scenes: BrowserSceneRegistry,
}

fn build_document_repl_runtime_for_tree(
    bootstrap: &WasmDocumentBootstrap,
    events: MechEventBuffer,
    candidate_tree: mech_core::nodes::Program,
) -> MResult<DocumentRuntimeCandidate> {
    let source = bootstrap.source();
    let (candidate_tree, _) = document_runtime_tree(source, candidate_tree)?;
    let resolver = document_source_resolver(candidate_tree, source)?;
    #[cfg(feature = "browser_host_scene")]
    let candidate_scenes = BrowserSceneRegistry::new();
    let mut builder = runtime_builder_with_factories(
        Some(events),
        #[cfg(feature = "browser_host_scene")]
        candidate_scenes.clone(),
    )
    .map_err(js_value_to_mech_error)?;

    match bootstrap {
        WasmDocumentBootstrap::Detached(_) | WasmDocumentBootstrap::SourceBacked(_) => {
            builder = builder
                .config(mech_runtime::RuntimeConfig::new("wasm-document-repl"))
                .source_resolver(resolver);
        }
        #[cfg(feature = "served_project_authority")]
        WasmDocumentBootstrap::Served(served) => {
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
        #[cfg(feature = "browser_host_scene")]
        scenes: candidate_scenes,
    })
}

/// Add a runtime-only capture at the last ordinary source statement and
/// publish it at the original document boundary. Appended console sections
/// remain after this boundary, so they cannot replace the fixed Output pane.
fn document_runtime_tree(
    source: &SourceBackedDocumentBootstrap,
    mut candidate_tree: mech_core::nodes::Program,
) -> MResult<(mech_core::nodes::Program, Option<OutputId>)> {
    let boundary = source
        .tree
        .body
        .sections
        .len()
        .min(candidate_tree.body.sections.len());
    let console_sections = candidate_tree.body.sections.split_off(boundary);
    let mut capture_tree = mech_syntax::parser::parse("~~~mech:hidden\nans\n~~~")?;
    let capture = capture_tree
        .body
        .sections
        .iter_mut()
        .flat_map(|section| section.elements.drain(..))
        .find_map(|element| match element {
            mech_core::nodes::SectionElement::FencedMechCode(block) => Some(block),
            _ => None,
        })
        .ok_or_else(|| document_runtime_error("program output capture syntax did not parse"))?;
    let inserted = insert_root_document_program_output_capture(&mut candidate_tree, capture);
    candidate_tree.body.sections.extend(console_sections);
    let output_id = if inserted {
        root_document_output_ids(&candidate_tree)
            .iter()
            .position(|candidate| *candidate == root_document_program_output_id())
            .and_then(|ordinal| u32::try_from(ordinal).ok())
            .map(OutputId::new)
    } else {
        None
    };
    Ok((candidate_tree, output_id))
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

    pub(super) fn document_output_ordinals(tree: &mech_core::nodes::Program) -> HashMap<u64, u64> {
        root_document_output_ids(tree)
            .into_iter()
            .enumerate()
            .map(|(ordinal, output_id)| (output_id, ordinal as u64))
            .collect()
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
    ) -> Result<Option<DocumentProgramOutput>, JsValue> {
        let (output_id, captured) = {
            let Some(runtime) = repl.session.runtime() else {
                return Ok(None);
            };
            let Some(output_id) = bootstrap.program_output_id().map_err(to_js_error)? else {
                return Ok(None);
            };
            (
                output_id,
                runtime.output_value(output_id).map_err(to_js_error)?,
            )
        };
        let Some(snapshot) = captured.filter(|snapshot| !snapshot.is_empty()) else {
            return Ok(None);
        };
        let selection_token = repl
            .session
            .retain_selection("ans", snapshot.clone(), None)
            .map_err(to_js_error)?;
        Ok(Some(DocumentProgramOutput {
            selection_token,
            output_id,
        }))
    }

    #[wasm_bindgen]
    pub struct WasmDocument {
        pub(super) repl: crate::repl::WasmRepl,
        pub(super) bootstrap: WasmDocumentBootstrap,
        document_output_ordinals: HashMap<u64, u64>,
        program_output: Option<DocumentProgramOutput>,
        started: bool,
        stopped: bool,
    }

    #[wasm_bindgen]
    impl WasmDocument {
        #[wasm_bindgen(js_name = fromEncoded)]
        pub fn from_encoded(encoded: &str) -> Result<WasmDocument, JsValue> {
            let tree = decode_document_tree(encoded)?;
            let document_output_ordinals = document_output_ordinals(&tree);
            let source = SourceBackedDocumentBootstrap {
                root_specifier: "document.mec".to_string(),
                source_map: HashMap::from([("document.mec".to_string(), String::new())]),
                resolutions: Vec::new(),
                tree,
                console_instance: "repl".to_string(),
                lifecycle: DocumentRuntimeLifecycle::default(),
            };
            let bootstrap = WasmDocumentBootstrap::Detached(DetachedDocumentBootstrap { source });
            let mut repl =
                crate::repl::WasmRepl::from_document(bootstrap.clone()).map_err(to_js_error)?;
            let program_output = capture_program_output(&mut repl, &bootstrap)?;
            Ok(Self {
                repl,
                bootstrap,
                document_output_ordinals,
                program_output,
                started: false,
                stopped: false,
            })
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
            let tree = decode_document_tree(encoded)?;
            let source_map = source_map_from_js(sources)?;
            Self::from_tree_with_sources(tree, root_specifier, source_map, Vec::new())
        }

        #[wasm_bindgen(js_name = fromEncodedWithBundle)]
        pub fn from_encoded_with_bundle(
            encoded: &str,
            root_specifier: &str,
            sources: JsValue,
            resolutions: JsValue,
        ) -> Result<WasmDocument, JsValue> {
            let tree = decode_document_tree(encoded)?;
            let source_map = source_map_from_js(sources)?;
            let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
            Self::from_tree_with_sources(tree, root_specifier, source_map, resolutions)
        }

        pub(super) fn from_tree_with_sources(
            tree: mech_core::nodes::Program,
            root_specifier: &str,
            source_map: HashMap<String, String>,
            resolutions: Vec<SourceResolutionEntry>,
        ) -> Result<WasmDocument, JsValue> {
            let bootstrap = SourceBackedDocumentBootstrap {
                root_specifier: root_specifier.to_string(),
                source_map,
                resolutions,
                tree,
                console_instance: "repl".to_string(),
                lifecycle: DocumentRuntimeLifecycle::default(),
            };
            let document_output_ordinals = document_output_ordinals(&bootstrap.tree);
            let bootstrap = WasmDocumentBootstrap::SourceBacked(bootstrap);
            let mut repl =
                crate::repl::WasmRepl::from_document(bootstrap.clone()).map_err(to_js_error)?;
            let program_output = capture_program_output(&mut repl, &bootstrap)?;
            Ok(Self {
                repl,
                bootstrap,
                document_output_ordinals,
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
            let tree = decode_document_tree(encoded)?;
            let document = parse_project_config(config_source)?;
            let source_map = source_map_from_js(sources)?;
            let authority = served_browser_authority()?;
            Self::from_served_tree(
                tree,
                root_specifier,
                document,
                config_source,
                source_map,
                Vec::new(),
                authority,
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
        ) -> Result<WasmDocument, JsValue> {
            let tree = decode_document_tree(encoded)?;
            let document = parse_project_config(config_source)?;
            let source_map = source_map_from_js(sources)?;
            let resolutions = document_resolutions_from_js(resolutions, &source_map)?;
            let authority = served_browser_authority()?;
            Self::from_served_tree(
                tree,
                root_specifier,
                document,
                config_source,
                source_map,
                resolutions,
                authority,
            )
        }

        #[cfg(feature = "served_project_authority")]
        pub(super) fn from_served_tree(
            tree: mech_core::nodes::Program,
            root_specifier: &str,
            document: MechConfigDocument,
            config_source: &str,
            source_map: HashMap<String, String>,
            resolutions: Vec<SourceResolutionEntry>,
            authority: BrowserRuntimeInjectionConfig,
        ) -> Result<WasmDocument, JsValue> {
            let document_output_ordinals = document_output_ordinals(&tree);
            let source = SourceBackedDocumentBootstrap {
                root_specifier: root_specifier.to_string(),
                source_map,
                resolutions,
                tree,
                console_instance: internal_repl_console_instance(&document.hosts),
                lifecycle: DocumentRuntimeLifecycle::default(),
            };
            validate_served_authority(&document, &authority).map_err(to_js_error)?;
            validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
            let bootstrap = WasmDocumentBootstrap::Served(ServedDocumentBootstrap {
                source,
                config_source: config_source.to_string(),
                authority,
            });
            let mut repl =
                crate::repl::WasmRepl::from_document(bootstrap.clone()).map_err(to_js_error)?;
            let program_output = capture_program_output(&mut repl, &bootstrap)?;
            Ok(Self {
                repl,
                bootstrap,
                document_output_ordinals,
                program_output,
                started: false,
                stopped: false,
            })
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
            let mut replacement = match &self.bootstrap {
                WasmDocumentBootstrap::Detached(_) => Self::from_encoded(encoded)?,
                WasmDocumentBootstrap::SourceBacked(bootstrap) => {
                    let tree = decode_document_tree(encoded)?;
                    Self::from_tree_with_sources(
                        tree,
                        &bootstrap.root_specifier,
                        bootstrap.source_map.clone(),
                        bootstrap.resolutions.clone(),
                    )?
                }
                #[cfg(feature = "served_project_authority")]
                WasmDocumentBootstrap::Served(bootstrap) => {
                    let tree = decode_document_tree(encoded)?;
                    let document = parse_project_config(&bootstrap.config_source)?;
                    Self::from_served_tree(
                        tree,
                        &bootstrap.source.root_specifier,
                        document,
                        &bootstrap.config_source,
                        bootstrap.source.source_map.clone(),
                        bootstrap.source.resolutions.clone(),
                        bootstrap.authority.clone(),
                    )?
                }
            };
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
            let response = self.repl.invoke(source)?;
            if self.repl.session.source() != accepted_before {
                self.refresh_document_output_ordinals()
                    .map_err(to_js_error)?;
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
            if source.trim().is_empty() || source.trim_start().starts_with(':') {
                return None;
            }
            let suppresses_value = mech_syntax::submission_terminal(source)
                .is_some_and(|terminal| terminal.suppresses_value);
            let tree = mech_syntax::parser::parse(source.trim()).ok()?;
            let mut formatter = mech_syntax::formatter::Formatter::new();
            formatter.html = true;
            let mut html = String::new();
            let mut found_code = false;
            for section in &tree.body.sections {
                for element in &section.elements {
                    let mech_core::nodes::SectionElement::MechCode(code) = element else {
                        return None;
                    };
                    if code
                        .iter()
                        .any(|(node, _)| matches!(node, mech_core::nodes::MechCode::Error(_, _)))
                    {
                        return None;
                    }
                    html.push_str(&formatter.mech_code(code));
                    found_code = true;
                }
            }
            if suppresses_value {
                const TERMINATOR: &str = "<span class=\"mech-code-terminal\">;</span>";
                if let Some(comment) = html.rfind("<span class=\"mech-comment\">") {
                    html.insert_str(comment, &format!("{TERMINATOR} "));
                } else {
                    html.push_str(TERMINATOR);
                }
            }
            found_code.then_some(html)
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
            let tree = mech_syntax::parser::parse(source).map_err(to_js_error)?;
            let inline_offset = root_document_inline_eval_count(
                &self.current_interactive_tree().map_err(to_js_error)?,
            );
            let mut formatter = mech_syntax::formatter::Formatter::new();
            formatter.html = true;
            formatter.set_root_inline_eval_offset(inline_offset);
            let html = formatter.program(&tree);
            let accepted = match self.repl.session.submit_host_source(source) {
                Ok(_) => {
                    self.refresh_document_output_ordinals()
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
            let result = Object::new();
            Reflect::set(
                &result,
                &JsValue::from_str("topic"),
                &JsValue::from_str(topic),
            )?;
            Reflect::set(
                &result,
                &JsValue::from_str("html"),
                &if accepted {
                    JsValue::from_str(&html)
                } else {
                    JsValue::NULL
                },
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
        pub(super) fn runtime(&self) -> Result<&MechRuntime, JsValue> {
            self.repl
                .session
                .runtime()
                .ok_or_else(|| js_error("document runtime is not active"))
        }

        pub(super) fn current_interactive_tree(&self) -> MResult<mech_core::nodes::Program> {
            self.repl
                .session
                .source_tree()
                .cloned()
                .map(Ok)
                .unwrap_or_else(|| self.bootstrap.interactive_tree(self.repl.session.source()))
        }

        fn refresh_document_output_ordinals(&mut self) -> MResult<()> {
            let current = self.current_interactive_tree()?;
            let (runtime_tree, _) = document_runtime_tree(self.bootstrap.source(), current)?;
            self.document_output_ordinals = document_output_ordinals(&runtime_tree);
            Ok(())
        }

        fn runtime_output_id(&self, output_id: u64) -> Option<OutputId> {
            let output_id = self
                .document_output_ordinals
                .get(&output_id)
                .copied()
                .unwrap_or(output_id);
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

fn decode_document_tree(encoded: &str) -> Result<mech_core::nodes::Program, JsValue> {
    mech_core::nodes::decode_and_decompress(encoded)
        .map_err(|error| js_error(format!("failed to decode Mech document: {error}")))
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

fn build_runtime(
    document: &MechConfigDocument,
    source_resolver: InMemorySourceResolver,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    let runtime_config = mech_runtime::RuntimeConfig::default()
        .apply_patch(&document.runtime)
        .map_err(to_js_error)?;
    let mut builder = runtime_builder_with_factories(
        None,
        #[cfg(feature = "browser_host_scene")]
        scenes,
    )?
    .config(runtime_config)
    .source_resolver(source_resolver);
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
    source_resolver: InMemorySourceResolver,
    #[cfg(feature = "browser_host_scene")] scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    let runtime_config = authority.into_runtime_config().map_err(to_js_error)?;
    let mut builder = runtime_builder_with_factories(
        None,
        #[cfg(feature = "browser_host_scene")]
        scenes,
    )?
    // The served authority already contains the project patch that the server
    // verified and signed. Keep that runtime environment authoritative here.
    .config(runtime_config)
    .source_resolver(source_resolver);
    for required in &document.hosts {
        if let Some(host) = authority
            .hosts
            .iter()
            .find(|host| host.name == required.name && host.provider == required.provider)
        {
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
    _source_resolver: InMemorySourceResolver,
    #[cfg(feature = "browser_host_scene")] _scenes: BrowserSceneRegistry,
) -> Result<MechRuntime, JsValue> {
    Err(js_error(
        "served project authority support was not compiled into this WASM artifact",
    ))
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

fn validate_served_authority(
    document: &MechConfigDocument,
    #[cfg(feature = "served_project_authority")] authority: &BrowserRuntimeInjectionConfig,
    #[cfg(not(feature = "served_project_authority"))] _authority: &(),
) -> mech_core::MResult<()> {
    #[cfg(not(feature = "served_project_authority"))]
    {
        return Err(MechError::new(
            ProjectError {
                message:
                    "served project authority support was not compiled into this WASM artifact"
                        .into(),
            },
            None,
        ));
    }
    #[cfg(feature = "served_project_authority")]
    {
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

fn document_source_resolver(
    tree: mech_core::nodes::Program,
    source: &SourceBackedDocumentBootstrap,
) -> MResult<InMemorySourceResolver> {
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

    let mut resolver = project_source_resolver(&source.source_map)?;
    for resolution in &source.resolutions {
        resolver.insert_resolution_entry(resolution)?;
    }
    resolver.insert_source(
        &source.root_specifier,
        ResolvedSource::new(
            &source.root_specifier,
            format!("memory:{}", source.root_specifier),
            MechSourceCode::Tree(tree),
        )
        .with_kind(SourceKind::Mech),
    )?;
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
    runtime.load_root_program(
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
        _ => "invalid-production-route",
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

pub(super) fn rendered_value(
    snapshot: mech_runtime::RuntimeValueSnapshot,
    max_elements: usize,
) -> Result<JsValue, JsValue> {
    let value = snapshot.into_value();
    let rendered = Object::new();
    Reflect::set(
        &rendered,
        &JsValue::from_str("kind"),
        &JsValue::from_str(&value.format_kind_with_element_limit(max_elements)),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("blockHtml"),
        &JsValue::from_str(&value.to_html_with_element_limit(max_elements)),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("inlineHtml"),
        &JsValue::from_str(&mech_core::escape_html_text(
            &value.format_canonical_inline_with_element_limit(max_elements),
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
    fn document_output_hashes_map_to_resident_output_ordinals() {
        let tree =
            mech_syntax::parser::parse(include_str!("../../../examples/working/fizzbuzz.mec"))
                .unwrap();
        let outputs = document::document_output_ordinals(&tree);
        assert_eq!(outputs.get(&29_884_140_763_677_669), Some(&0));

        let tree =
            mech_syntax::parser::parse(include_str!("../../../tests/fixtures/shims/all-slots.mec"))
                .unwrap();
        let outputs = document::document_output_ordinals(&tree);
        assert_eq!(
            outputs.get(&mech_core::hash_str("inline-eval:0:0")),
            Some(&0)
        );
        assert_eq!(
            outputs.len(),
            2,
            "inline and fenced root outputs are mapped"
        );
    }

    #[test]
    fn browser_document_profile_runs_nbody_module_imports() {
        let tree = mech_syntax::parser::parse(
            "+> combinatorics\n\
             +> stats\n\
             pairs := combinatorics/n-choose-k(10.0, 2.0)\n\
             column-totals := stats/sum/column([1.0 2.0; 3.0 4.0])\n\
             row-totals := stats/sum/row([1.0 2.0; 3.0 4.0])\n\
             pairs",
        )
        .unwrap();
        let source = SourceBackedDocumentBootstrap {
            root_specifier: "document.mec".to_string(),
            source_map: HashMap::from([("document.mec".to_string(), String::new())]),
            resolutions: Vec::new(),
            tree: tree.clone(),
            console_instance: "repl".to_string(),
            lifecycle: DocumentRuntimeLifecycle::default(),
        };
        let bootstrap = WasmDocumentBootstrap::Detached(DetachedDocumentBootstrap { source });
        let source = bootstrap.initial_repl_source();
        let (runtime, outcome) = activate_document_repl_runtime_tree(
            &bootstrap,
            MechEventBuffer::default(),
            &source,
            tree,
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
        let tree =
            mech_syntax::parser::parse(include_str!("../../../tests/fixtures/shims/all-slots.mec"))
                .unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
            "a named fence still contributes the enclosing program result",
        );

        document.repl.session.submit("40 + 2").unwrap();

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
            .compile_root(SourceRequest::new("demo.mec"), browser_module_options())
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
        assert_eq!(
            runtime.root_symbol_value("greeting").unwrap().into_value(),
            mech_core::LegacyValue::String(mech_core::Ref::new("Hello, Ada".to_string())),
        );
    }

    #[test]
    fn encoded_document_controller_loads_residently_without_legacy_execution() {
        let tree = mech_syntax::parser::parse("~answer := 0\nanswer += 42\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree = mech_syntax::parser::parse(source).unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
        let document = WasmDocument::from_encoded(&encoded).unwrap();

        assert_eq!(
            document.runtime().unwrap().program_route(),
            RuntimeProgramRoute::ResidentPure,
        );
    }

    #[test]
    fn encoded_document_controller_accepts_resident_atan2() {
        let tree = mech_syntax::parser::parse(
            "Resident Atan2\n================\n+> math\n================\n\nangle := math/atan2(1.0, 0.0)\nangle",
        )
        .unwrap();
        let module = ["math".to_string()];
        assert!(
            mech_stdlib::source_catalog()
                .resident_factory(&module, "atan2")
                .is_some(),
            "the standard source catalog must install resident math/atan2",
        );
        let source = SourceBackedDocumentBootstrap {
            root_specifier: "document.mec".to_string(),
            source_map: HashMap::from([("document.mec".to_string(), String::new())]),
            resolutions: Vec::new(),
            tree,
            console_instance: "repl".to_string(),
            lifecycle: DocumentRuntimeLifecycle::default(),
        };
        let bootstrap = WasmDocumentBootstrap::Detached(DetachedDocumentBootstrap { source });
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
        let tree = mech_syntax::parser::parse(
            "matrix := [1.0 2.0; 3.0 4.0]\nselected := matrix[2,1]\nselected",
        )
        .unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree = mech_syntax::parser::parse("~answer := 0\nanswer += 42\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
            mech_runtime::RuntimeValueSnapshot::try_from(mech_core::LegacyValue::F64(
                mech_core::Ref::new(7.0),
            ))
            .unwrap(),
        );
        assert_eq!(
            document.repl.session.submit("ans").unwrap().to_string(),
            "7",
            "subsequent clicks must replace ans without duplicate declarations",
        );
    }

    #[test]
    fn selecting_a_large_document_value_is_runtime_local_until_ans_is_consumed() {
        let tree =
            mech_syntax::parser::parse(include_str!("../../../examples/working/fizzbuzz.mec"))
                .unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        match value.into_value() {
            mech_core::LegacyValue::F64(value) => assert_eq!(*value.borrow(), expected),
            mech_core::LegacyValue::MutableReference(value) => match &*value.borrow() {
                mech_core::LegacyValue::F64(value) => assert_eq!(*value.borrow(), expected),
                other => panic!("expected f64 value, got {other:?}"),
            },
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
        let tree = mech_syntax::parser::parse(source).unwrap();
        let source_map = HashMap::from([
            ("docs/main.mec".to_string(), source.to_string()),
            (
                "docs/math.mec".to_string(),
                "value := 41\n<+ value\n".to_string(),
            ),
        ]);

        let document =
            WasmDocument::from_tree_with_sources(tree, "docs/main.mec", source_map, Vec::new())
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
    fn source_backed_document_keeps_the_decoded_tree_authoritative() {
        let decoded_source = "+> ./math.mec\nanswer := math/value + 1\nanswer\n";
        let tree = mech_syntax::parser::parse(decoded_source).unwrap();
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

        let document =
            WasmDocument::from_tree_with_sources(tree, "docs/main.mec", source_map, Vec::new())
                .unwrap();

        assert_f64(
            document
                .runtime()
                .unwrap()
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
        assert_eq!(
            document.current_interactive_tree().unwrap(),
            document.repl.session.source_tree().unwrap().clone(),
            "document projections must use the session's accepted semantic tree",
        );
    }

    #[test]
    fn source_backed_document_preserves_explicit_resolution_edges_across_reset() {
        let source = "+> ./math.mec\n~answer := 0\nanswer += math/value + 1\nanswer\n";
        let tree = mech_syntax::parser::parse(source).unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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

        let mut document = WasmDocument::from_tree_with_sources(
            tree,
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
        let tree = mech_syntax::parser::parse("~answer := 0\nanswer += 42\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree = mech_syntax::parser::parse("x := 1\ny := 2\ny").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree = mech_syntax::parser::parse("baseline := 1").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
    fn appended_document_fragments_continue_the_root_inline_output_namespace() {
        let baseline = mech_syntax::parser::parse("Baseline {1}.\n\nanswer := 1\nanswer").unwrap();
        let fragment = mech_syntax::parser::parse("Documentation {2}.").unwrap();
        let offset = root_document_inline_eval_count(&baseline);
        assert_eq!(offset, 1);

        let mut formatter = mech_syntax::formatter::Formatter::new();
        formatter.html = true;
        formatter.set_root_inline_eval_offset(offset);
        let html = formatter.program(&fragment);
        let fragment_id = mech_core::hash_str("inline-eval:0:1");
        assert!(html.contains(&format!("id=\"{fragment_id}:0\"")), "{html}");

        let mut combined = baseline.clone();
        combined.body.sections.extend(fragment.body.sections);
        assert_eq!(
            root_document_output_ids(&combined),
            vec![
                mech_core::hash_str("inline-eval:0:0"),
                mech_core::hash_str("inline-eval:0:1"),
            ],
        );
        assert_eq!(
            document::document_output_ordinals(&combined).get(&fragment_id),
            Some(&1),
        );

        let source = SourceBackedDocumentBootstrap {
            root_specifier: "document.mec".to_string(),
            source_map: HashMap::new(),
            resolutions: Vec::new(),
            tree: baseline,
            console_instance: "repl".to_string(),
            lifecycle: DocumentRuntimeLifecycle::default(),
        };
        let (runtime_tree, program_output) = document_runtime_tree(&source, combined).unwrap();
        assert!(program_output.is_some());
        assert_eq!(
            document::document_output_ordinals(&runtime_tree).get(&fragment_id),
            Some(&2),
            "the hidden fixed program output owns the ordinal before appended documentation",
        );
    }

    #[test]
    fn fixed_program_capture_preserves_trailing_comment_output_order() {
        let tree = mech_syntax::parser::parse(
            "~~~mech\nanswer := 40 + 2\n-- The next value is {answer + 1}.\n~~~",
        )
        .unwrap();
        let source_output_ids = root_document_output_ids(&tree);
        assert_eq!(source_output_ids.len(), 2);
        assert_eq!(source_output_ids[0], mech_core::hash_str("inline-eval:0:0"));

        let source = SourceBackedDocumentBootstrap {
            root_specifier: "document.mec".to_string(),
            source_map: HashMap::new(),
            resolutions: Vec::new(),
            tree: tree.clone(),
            console_instance: "repl".to_string(),
            lifecycle: DocumentRuntimeLifecycle::default(),
        };
        let (runtime_tree, program_output) = document_runtime_tree(&source, tree.clone()).unwrap();
        let mut expected = source_output_ids;
        expected.push(root_document_program_output_id());
        assert_eq!(root_document_output_ids(&runtime_tree), expected);
        assert!(program_output.is_some());

        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
                .is_some()
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
        let tree = mech_syntax::parser::parse(source).unwrap();
        let authority = authority_config(
            config.hosts.clone(),
            config.run.as_ref().unwrap().grants.clone(),
        );
        let mut document = WasmDocument::from_served_tree(
            tree,
            "main.mec",
            config,
            config_source,
            HashMap::from([("main.mec".to_string(), source.to_string())]),
            Vec::new(),
            authority,
        )
        .unwrap();
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
    fn unsupported_generic_table_project_fails_closed_without_legacy_execution() {
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
        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::SemanticUnsupported,
        );
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[derive(Clone, Debug)]
    struct SharedManualTimerInputDriver(
        std::sync::Arc<std::sync::Mutex<mech_timer::ManualTimerInputDriver>>,
    );

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl SharedManualTimerInputDriver {
        fn new(instance: &str, frequency_hz: u64, max_catch_up_steps: u64) -> Self {
            Self(std::sync::Arc::new(std::sync::Mutex::new(
                mech_timer::ManualTimerInputDriver::new(instance, frequency_hz, max_catch_up_steps),
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
    #[derive(Debug)]
    struct TestManualTimerHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        driver: SharedManualTimerInputDriver,
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl TestManualTimerHostFactory {
        fn new() -> Self {
            Self {
                manifest: mech_timer::timer_host_manifest().unwrap(),
                driver: SharedManualTimerInputDriver::new("clock", 60, 1),
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

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[test]
    fn ekf_scene_advances_on_every_resident_timer_packet() {
        let document = parse_config_document(
            "examples/ekf/mech.mcfg",
            include_str!("../../../examples/ekf/mech.mcfg"),
            ConfigProfileOptions::default(),
        )
        .unwrap();
        let source = include_str!("../../../examples/ekf/localization.mec").to_string();
        let sources = HashMap::from([("localization.mec".to_string(), source)]);
        let timer_factory = TestManualTimerHostFactory::new();
        let timer_driver = timer_factory.driver.clone();
        let scene_backend = mech_scene::RecordingSceneBackend::new();
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
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
        runtime.start_input_drivers().unwrap();

        let truth_position = |snapshot: &mech_scene::SceneSnapshot| {
            let truth = snapshot
                .circles
                .iter()
                .find(|circle| circle.id == "truth")
                .unwrap();
            (truth.x, truth.y)
        };
        assert_eq!(
            runtime.program_execution_info().observation_count,
            1,
            "EKF timer read must remain in the resident artifact"
        );
        assert!(scene_backend.latest().is_none());
        let mut previous = None;
        for turn in 1..=4 {
            assert_eq!(timer_driver.publish_steps(1).unwrap(), 1);
            let outcomes = runtime.drain_host_inputs(1).unwrap();
            let current = truth_position(&scene_backend.latest().unwrap());
            if let Some(previous) = previous {
                assert_ne!(
                    current,
                    previous,
                    "EKF truth must advance on turn {turn}; outcomes={outcomes:?}; info={:?}; scene_generation={}",
                    runtime.program_execution_info(),
                    scene_backend.generation(),
                );
            }
            previous = Some(current);
        }
        assert_eq!(runtime.program_execution_info().resident_accepted_turns, 4);
        assert_eq!(scene_backend.generation(), 4);
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
    fn unsupported_timer_table_scene_fails_before_effects_or_legacy_execution() {
        let document = generic_fixture_document();
        let source_paths = required_path_strings(include_str!(
            "../tests/fixtures/generic-timer-table-scene/mech.mcfg"
        ))
        .unwrap();
        assert_eq!(source_paths, vec!["table-scene.mec".to_string()]);

        let scene_backend = mech_scene::RecordingSceneBackend::new();
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&generic_fixture_sources()).unwrap())
            .host_input_capacity(16)
            .host_factory(Box::new(TestManualTimerHostFactory::new()))
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
        let error = run_project_sources(&mut runtime, &document).unwrap_err();
        assert_production_route_failed_closed(
            &runtime,
            &error,
            ResidentRouteFailureClass::SemanticUnsupported,
        );
        assert_eq!(scene_backend.generation(), 0);
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
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    #[wasm_bindgen_test]
    fn browser_document_profile_executes_both_stats_sum_axes() {
        let tree = mech_syntax::parser::parse(
            "+> stats\n\
             column-totals := stats/sum/column([1.0 2.0; 3.0 4.0])\n\
             row-totals := stats/sum/row([1.0 2.0; 3.0 4.0])\n\
             row-totals",
        )
        .unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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

    fn encoded_document(source: &str) -> String {
        let tree = mech_syntax::parser::parse(source).unwrap();
        mech_core::nodes::compress_and_encode(&tree).unwrap()
    }

    fn assert_resident_rejection<T>(result: Result<T, JsValue>, expected: &str) {
        let error = match result {
            Ok(_) => panic!("expected resident production admission to reject the program"),
            Err(error) => error,
        };
        let message = error.as_string().unwrap_or_else(|| format!("{error:?}"));
        assert!(message.contains("ResidentRouteFailure"), "{message}");
        assert!(message.contains(expected), "{message}");
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
    fn wasm_project_reports_served_authority_capability() {
        assert_eq!(
            WasmProject::supports_served_authority(),
            cfg!(feature = "served_project_authority")
        );
    }

    #[cfg(feature = "served_project_authority")]
    #[wasm_bindgen_test]
    fn wasm_document_reset_reuses_validated_served_authority() {
        let authority = served_document_authority();
        install_served_authority(&authority);
        let encoded = encoded_document(served_document_source());
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
        let encoded = encoded_document(served_document_source());
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
        let encoded = encoded_document(served_document_source());
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
        let tree = mech_syntax::parser::parse("~answer := 0\nanswer += 42\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let value = mech_core::LegacyValue::String(mech_core::Ref::new(
            "a\"b\\c\nα\u{2028}line\u{2029}paragraph".to_string(),
        ));
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
        let tree =
            mech_syntax::parser::parse(include_str!("../../../examples/working/fizzbuzz.mec"))
                .unwrap();
        let output_id = tree
            .body
            .sections
            .iter()
            .flat_map(|section| &section.elements)
            .filter_map(|element| match element {
                mech_core::nodes::SectionElement::FencedMechCode(block) if block.config.output => {
                    block
                        .code
                        .last()
                        .map(|(code, _)| mech_core::hash_str(&format!("{code:?}")))
                }
                _ => None,
            })
            .last()
            .expect("FizzBuzz fixture must contain an output block");
        assert_eq!(
            output_id, 29_884_140_763_677_669,
            "the WASM output key must match the native formatter key",
        );

        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree =
            mech_syntax::parser::parse(include_str!("../../../examples/working/factorial.mec"))
                .unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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
        let tree = mech_syntax::parser::parse("~answer := 0\nanswer += 1\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
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

    #[wasm_bindgen_test]
    fn generic_timer_table_scene_is_excluded_from_the_resident_browser_product() {
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
        assert_resident_rejection(
            WasmProject::from_sources(config, sources.into()),
            "SemanticUnsupported",
        );
        canvas.remove();
    }
}
