use std::collections::{BTreeMap, HashMap};
#[cfg(feature = "served_project_authority")]
use std::path::Path;

use js_sys::{Array, Object, Reflect};
use wasm_bindgen::prelude::*;

#[cfg(feature = "served_project_authority")]
use base64::Engine as _;
use mech_core::{MechError, MechErrorKind, MechSourceCode};
#[cfg(feature = "browser_host_dom")]
use mech_host_browser::BrowserHostFactory;
#[cfg(feature = "served_project_authority")]
use mech_host_browser::BrowserRuntimeInjectionConfig;
#[cfg(feature = "served_project_authority")]
use mech_host_browser::{BrowserHostDelegationEnvelope, verify_browser_host_delegation};
#[cfg(feature = "browser_host_console")]
use mech_host_console::BrowserConsoleHostFactory;
#[cfg(feature = "browser_host_scene")]
use mech_host_scene::{BrowserSceneHostFactory, BrowserSceneRegistry};
#[cfg(feature = "browser_host_time")]
use mech_host_time::BrowserTimeHostFactory;
#[cfg(feature = "browser_host_timer")]
use mech_host_timer::BrowserTimerHostFactory;
use mech_runtime::{
    ConfigProfileOptions, InMemorySourceResolver, MechConfigDocument, MechRuntime,
    ModuleBuildOptions, ResolvedSource, RuntimeBuilder, SourceKind, SourceRequest,
    SourceResolutionEntry, parse_config_document, validate_source_resolution_entries,
};
#[cfg(feature = "served_project_authority")]
use mech_runtime::{
    HOST_DELEGATION_ALGORITHM_ED25519, HostDelegationKeyStore, HostDelegationPublicKey,
    HostDelegationVerificationRequest,
};
#[cfg(feature = "served_project_authority")]
use serde::Deserialize;

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
            #[cfg(feature = "browser_host_scene")]
            scenes,
            started: false,
            stopped: false,
        }
    }

    #[wasm_bindgen(js_name = rootInterpreterId)]
    pub fn root_interpreter_id(&self) -> u64 {
        self.runtime.root_interpreter_id()
    }

    #[wasm_bindgen(js_name = renderedOutput)]
    pub fn rendered_output(&self, interpreter_id: u64, output_id: u64) -> Result<JsValue, JsValue> {
        let interpreter_id = self.actual_interpreter_id(interpreter_id);
        self.runtime
            .output_value_for_interpreter(interpreter_id, output_id)
            .map_err(to_js_error)?
            .map(rendered_value)
            .transpose()
            .map(|value| value.unwrap_or(JsValue::NULL))
    }

    #[wasm_bindgen(js_name = renderedSymbol)]
    pub fn rendered_symbol(&self, interpreter_id: u64, name: &str) -> Result<JsValue, JsValue> {
        let interpreter_id = self.actual_interpreter_id(interpreter_id);
        let names = vec![name.to_string()];
        let value = self
            .runtime
            .symbol_values_for_interpreter(interpreter_id, &names)
            .map_err(to_js_error)?
            .and_then(|mut values| values.pop().map(|(_, value)| value));
        value
            .map(rendered_value)
            .transpose()
            .map(|value| value.unwrap_or(JsValue::NULL))
    }

    fn actual_interpreter_id(&self, interpreter_id: u64) -> u64 {
        if interpreter_id == 0 {
            self.runtime.root_interpreter_id()
        } else {
            interpreter_id
        }
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
enum WasmDocumentBootstrap {
    Detached,
    SourceBacked(SourceBackedDocumentBootstrap),
    #[cfg(feature = "served_project_authority")]
    Served(ServedDocumentBootstrap),
}

#[derive(Clone)]
struct SourceBackedDocumentBootstrap {
    root_specifier: String,
    source_map: HashMap<String, String>,
    resolutions: Vec<SourceResolutionEntry>,
}

#[cfg(feature = "served_project_authority")]
#[derive(Clone)]
struct ServedDocumentBootstrap {
    source: SourceBackedDocumentBootstrap,
    config_source: String,
    authority: BrowserRuntimeInjectionConfig,
}

#[wasm_bindgen]
pub struct WasmDocument {
    project: WasmProject,
    bootstrap: WasmDocumentBootstrap,
}

#[wasm_bindgen]
impl WasmDocument {
    #[wasm_bindgen(js_name = fromEncoded)]
    pub fn from_encoded(encoded: &str) -> Result<WasmDocument, JsValue> {
        let tree = decode_document_tree(encoded)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let mut runtime = runtime_builder_with_factories(
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
        )?
        .build()
        .map_err(to_js_error)?;
        runtime.run_tree(&tree).map_err(to_js_error)?;
        Ok(Self {
            project: WasmProject::from_runtime(
                runtime,
                #[cfg(feature = "browser_host_scene")]
                scenes,
            ),
            bootstrap: WasmDocumentBootstrap::Detached,
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

    fn from_tree_with_sources(
        tree: mech_core::nodes::Program,
        root_specifier: &str,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
    ) -> Result<WasmDocument, JsValue> {
        let bootstrap = SourceBackedDocumentBootstrap {
            root_specifier: root_specifier.to_string(),
            source_map,
            resolutions,
        };
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = document_source_resolver(tree, &bootstrap)?;
        let mut runtime = runtime_builder_with_factories(
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
        )?
        .source_resolver(source_resolver)
        .build()
        .map_err(to_js_error)?;
        runtime
            .resolve_and_run_root_module(
                SourceRequest::new(&bootstrap.root_specifier),
                browser_module_options(),
            )
            .map_err(to_js_error)?;

        Ok(Self {
            project: WasmProject::from_runtime(
                runtime,
                #[cfg(feature = "browser_host_scene")]
                scenes,
            ),
            bootstrap: WasmDocumentBootstrap::SourceBacked(bootstrap),
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
    fn from_served_tree(
        tree: mech_core::nodes::Program,
        root_specifier: &str,
        document: MechConfigDocument,
        config_source: &str,
        source_map: HashMap<String, String>,
        resolutions: Vec<SourceResolutionEntry>,
        authority: BrowserRuntimeInjectionConfig,
    ) -> Result<WasmDocument, JsValue> {
        let source = SourceBackedDocumentBootstrap {
            root_specifier: root_specifier.to_string(),
            source_map,
            resolutions,
        };
        validate_served_authority(&document, &authority).map_err(to_js_error)?;
        validate_compiled_host_providers_for_hosts(&document.hosts).map_err(to_js_error)?;
        #[cfg(feature = "browser_host_scene")]
        let scenes = BrowserSceneRegistry::new();
        let source_resolver = document_source_resolver(tree, &source)?;
        let mut runtime = build_runtime_from_authority(
            &document,
            &authority,
            source_resolver,
            #[cfg(feature = "browser_host_scene")]
            scenes.clone(),
        )?;
        runtime
            .resolve_and_run_root_module(
                SourceRequest::new(&source.root_specifier),
                browser_module_options(),
            )
            .map_err(to_js_error)?;

        Ok(Self {
            project: WasmProject::from_runtime(
                runtime,
                #[cfg(feature = "browser_host_scene")]
                scenes,
            ),
            bootstrap: WasmDocumentBootstrap::Served(ServedDocumentBootstrap {
                source,
                config_source: config_source.to_string(),
                authority,
            }),
        })
    }

    #[wasm_bindgen(js_name = rootInterpreterId)]
    pub fn root_interpreter_id(&self) -> u64 {
        self.project.root_interpreter_id()
    }

    #[wasm_bindgen(js_name = renderedOutput)]
    pub fn rendered_output(&self, interpreter_id: u64, output_id: u64) -> Result<JsValue, JsValue> {
        self.project.rendered_output(interpreter_id, output_id)
    }

    #[wasm_bindgen(js_name = renderedSymbol)]
    pub fn rendered_symbol(&self, interpreter_id: u64, name: &str) -> Result<JsValue, JsValue> {
        self.project.rendered_symbol(interpreter_id, name)
    }

    #[wasm_bindgen(js_name = reset)]
    pub fn reset(&mut self, encoded: &str) -> Result<(), JsValue> {
        // Construct before touching the live project. A malformed replacement
        // must leave the current document usable.
        let replacement = match &self.bootstrap {
            WasmDocumentBootstrap::Detached => Self::from_encoded(encoded)?,
            WasmDocumentBootstrap::SourceBacked(bootstrap) => Self::from_tree_with_sources(
                decode_document_tree(encoded)?,
                &bootstrap.root_specifier,
                bootstrap.source_map.clone(),
                bootstrap.resolutions.clone(),
            )?,
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
        let was_started = self.project.started && !self.project.stopped;

        self.project.stop()?;
        self.project = replacement.project;
        self.bootstrap = replacement.bootstrap;
        if was_started {
            self.project.start()?;
        }
        Ok(())
    }

    #[wasm_bindgen(js_name = step)]
    pub fn step(&mut self, count: u64) -> Result<(), JsValue> {
        if count == 0 {
            return Err(js_error("count must be greater than zero"));
        }
        #[cfg(feature = "functions")]
        {
            for _ in 0..count {
                self.project.runtime.step(0).map_err(to_js_error)?;
            }
            Ok(())
        }
        #[cfg(not(feature = "functions"))]
        {
            let _ = count;
            Err(js_error(
                "this WASM artifact was built without reactive step support",
            ))
        }
    }

    #[wasm_bindgen(js_name = renderedSymbols)]
    pub fn rendered_symbols(&self, names: JsValue) -> Result<JsValue, JsValue> {
        let names = rendered_symbol_names_from_js(names)?;
        let values = match names {
            Some(names) => {
                let names = names.iter().map(String::as_str).collect::<Vec<_>>();
                self.project
                    .runtime
                    .root_symbol_values(&names)
                    .map_err(to_js_error)?
            }
            None => self
                .project
                .runtime
                .root_symbol_values_all()
                .map_err(to_js_error)?,
        };
        let rows = Array::new();
        for (name, value) in values {
            rows.push(&rendered_symbol_row(&name, value)?);
        }
        Ok(rows.into())
    }

    #[wasm_bindgen(js_name = interpreterIdByName)]
    pub fn interpreter_id_by_name(&self, name: &str) -> Result<JsValue, JsValue> {
        self.project
            .runtime
            .interpreter_id_by_name(name)
            .map_err(to_js_error)
            .map(|id| {
                id.map(|id| js_sys::BigInt::from(id).into())
                    .unwrap_or(JsValue::NULL)
            })
    }

    pub fn evaluate(&mut self, source: &str) -> Result<JsValue, JsValue> {
        let rendered = self
            .project
            .runtime
            .run_string(source)
            .map_err(to_js_error)
            .and_then(rendered_value)?;
        if self.project.started && !self.project.stopped {
            self.project.refresh_relevant_input_drivers()?;
        }
        Ok(rendered)
    }

    pub fn start(&mut self) -> Result<(), JsValue> {
        self.project.start()
    }

    pub fn frame(&mut self, max_inputs: usize) -> Result<JsValue, JsValue> {
        self.project.frame(max_inputs)
    }

    pub fn stop(&mut self) -> Result<(), JsValue> {
        self.project.stop()
    }
}

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
fn browser_runtime_builder() -> RuntimeBuilder {
    RuntimeBuilder::new().function_catalog(mech_stdlib::source_catalog())
}

fn runtime_builder_with_factories(
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
        builder = builder
            .host_factory(Box::new(
                BrowserConsoleHostFactory::new().map_err(to_js_error)?,
            ))
            .map_err(to_js_error)?;
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
    let mut builder = runtime_builder_with_factories(
        #[cfg(feature = "browser_host_scene")]
        scenes,
    )?
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
    let mut builder = runtime_builder_with_factories(
        #[cfg(feature = "browser_host_scene")]
        scenes,
    )?
    .config(authority.into_runtime_config().map_err(to_js_error)?)
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
) -> Result<InMemorySourceResolver, JsValue> {
    if source.root_specifier.trim().is_empty() {
        return Err(js_error("document root specifier must not be empty"));
    }
    if !source.source_map.contains_key(&source.root_specifier) {
        return Err(js_error(format!(
            "document root `{}` is missing from the source map",
            source.root_specifier,
        )));
    }

    let mut resolver = project_source_resolver(&source.source_map).map_err(to_js_error)?;
    for resolution in &source.resolutions {
        resolver
            .insert_resolution_entry(resolution)
            .map_err(to_js_error)?;
    }
    resolver
        .insert_source(
            &source.root_specifier,
            ResolvedSource::new(
                &source.root_specifier,
                format!("memory:{}", source.root_specifier),
                MechSourceCode::Tree(tree),
            )
            .with_kind(SourceKind::Mech),
        )
        .map_err(to_js_error)?;
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
    for key in roots {
        runtime.resolve_and_run_root_module(
            SourceRequest::new(key.to_string()),
            browser_module_options(),
        )?;
    }
    Ok(())
}

fn rendered_value(snapshot: mech_runtime::RuntimeValueSnapshot) -> Result<JsValue, JsValue> {
    let value = snapshot.into_value();
    let rendered = Object::new();
    Reflect::set(
        &rendered,
        &JsValue::from_str("kind"),
        &JsValue::from_str(&format!("{:?}", value.kind())),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("blockHtml"),
        &JsValue::from_str(&value.to_html()),
    )?;
    Reflect::set(
        &rendered,
        &JsValue::from_str("inlineHtml"),
        &JsValue::from_str(&mech_core::escape_html_text(&value.format_value_inline())),
    )?;
    Ok(rendered.into())
}

fn rendered_symbol_names_from_js(names: JsValue) -> Result<Option<Vec<String>>, JsValue> {
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

fn rendered_symbol_row(
    name: &str,
    snapshot: mech_runtime::RuntimeValueSnapshot,
) -> Result<JsValue, JsValue> {
    let rendered_value = rendered_value(snapshot)?;
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

    #[test]
    fn from_sources_executes_paths_in_order() {
        let document =
            parse_config_document("test.mcfg", CONFIG, ConfigProfileOptions::default()).unwrap();
        let mut sources = HashMap::new();
        sources.insert("a.mec".to_string(), "x := 1".to_string());
        sources.insert("b.mec".to_string(), "y := 2".to_string());
        let mut runtime = browser_runtime_builder()
            .source_resolver(project_source_resolver(&sources).unwrap())
            .build()
            .unwrap();
        run_project_sources(&mut runtime, &document).unwrap();
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

    #[test]
    fn browser_project_profile_supports_string_concatenation() {
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
    }

    #[test]
    fn encoded_document_runs_on_runtime_and_exposes_root_output() {
        let tree = mech_syntax::parser::parse("answer := 41 + 1\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
        let mut runtime = browser_runtime_builder().build().unwrap();

        let decoded: mech_core::nodes::Program =
            mech_core::nodes::decode_and_decompress(&encoded).unwrap();
        runtime.run_tree(&decoded).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 42.0);
    }

    #[test]
    fn encoded_fizzbuzz_document_retains_fenced_block_output() {
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
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
        let decoded: mech_core::nodes::Program =
            mech_core::nodes::decode_and_decompress(&encoded).unwrap();
        let mut runtime = browser_runtime_builder().build().unwrap();

        assert_eq!(
            output_id, 29_884_140_763_677_669,
            "the browser runtime key must match the native formatter key",
        );
        runtime.run_tree(&decoded).unwrap();

        assert!(
            runtime
                .output_value_for_interpreter(runtime.root_interpreter_id(), output_id,)
                .unwrap()
                .is_some(),
            "FizzBuzz output must remain queryable by its formatted block id",
        );
    }

    fn assert_f64(value: mech_runtime::RuntimeValueSnapshot, expected: f64) {
        match value.into_value() {
            mech_core::Value::F64(value) => assert_eq!(*value.borrow(), expected),
            mech_core::Value::MutableReference(value) => match &*value.borrow() {
                mech_core::Value::F64(value) => assert_eq!(*value.borrow(), expected),
                other => panic!("expected f64 value, got {other:?}"),
            },
            other => panic!("expected f64 value, got {other:?}"),
        }
    }

    #[test]
    fn project_sources_resolve_sibling_and_parent_modules() {
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

        run_project_sources(&mut runtime, &document).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 42.0);
        assert_f64(runtime.root_symbol_value("parent-answer").unwrap(), 42.0);
    }

    #[test]
    fn source_backed_document_resolves_relative_imports_from_its_root_specifier() {
        let source = "The imported answer is {math/value + 1}.\n\n+> ./math.mec\nanswer := math/value + 1\nanswer\n";
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
                .project
                .runtime
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
        assert_f64(
            document
                .project
                .runtime
                .output_value_for_interpreter(
                    document.project.runtime.root_interpreter_id(),
                    mech_core::hash_str("inline-eval:0:0"),
                )
                .unwrap()
                .expect("formatted source root must retain inline output"),
            42.0,
        );
    }

    #[test]
    fn source_backed_document_preserves_explicit_resolution_edges_across_reset() {
        let source = "+> ./math.mec\nanswer := math/value + 1\nanswer\n";
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
        assert_f64(
            document
                .project
                .runtime
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );

        document.reset(&encoded).unwrap();
        assert_f64(
            document
                .project
                .runtime
                .root_symbol_value("answer")
                .unwrap(),
            42.0,
        );
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
    fn project_sources_only_execute_configured_roots() {
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

        run_project_sources(&mut runtime, &document).unwrap();

        assert_f64(runtime.root_symbol_value("answer").unwrap(), 2.0);
    }

    #[cfg(feature = "served_project_authority")]
    fn authority_config(
        hosts: Vec<mech_runtime::HostInstanceConfig>,
        grants: Vec<mech_runtime::RunResourceGrantConfig>,
    ) -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_host_browser::BrowserHostRuntimeConfig::from(
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

    #[test]
    fn generic_table_project_source_runs_through_runtime_loader() {
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
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[derive(Debug)]
    struct TestManualTimerHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        snapshot: mech_host_timer::SharedTimerSnapshot,
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    impl TestManualTimerHostFactory {
        fn new() -> Self {
            Self {
                manifest: mech_host_timer::timer_host_manifest().unwrap(),
                snapshot: mech_host_timer::new_shared_snapshot(
                    mech_host_timer::TimerSnapshot::new(0, 60, 0),
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
            mech_host_timer::timer_settings_from_config(settings).map(|_| ())
        }
        fn instantiate(
            &self,
            instance_name: &str,
            settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<mech_runtime::RuntimeHostInstallation> {
            let settings = mech_host_timer::timer_settings_from_config(settings)?;
            Ok(mech_runtime::RuntimeHostInstallation {
                interface: mech_runtime::materialize_host_manifest(instance_name, &self.manifest)?,
                resource_providers: vec![Box::new(mech_host_timer::TimerResourceProvider::new(
                    instance_name,
                    self.snapshot.clone(),
                ))],
                input_drivers: vec![Box::new(mech_host_timer::ManualTimerInputDriver::new(
                    instance_name,
                    settings.frequency_hz,
                    settings.max_catch_up_steps,
                ))],
            })
        }
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
    fn fixture_timer_packet(tick: u64, delta_seconds: f64) -> mech_runtime::RuntimeHostInput {
        mech_runtime::RuntimeHostInput::new(vec![
            mech_runtime::RuntimeHostInputUpdate {
                source: mech_runtime::RuntimeHostInputSource::new("timer://tick/tick", "tick")
                    .unwrap(),
                value: mech_runtime::RuntimeHostInputValue::F64(tick as f64),
            },
            mech_runtime::RuntimeHostInputUpdate {
                source: mech_runtime::RuntimeHostInputSource::new(
                    "timer://tick/tick",
                    "delta-seconds",
                )
                .unwrap(),
                value: mech_runtime::RuntimeHostInputValue::F64(delta_seconds),
            },
        ])
        .unwrap()
    }

    #[cfg(all(feature = "browser_host_timer", feature = "browser_host_scene"))]
    #[test]
    fn generic_timer_table_scene_fixture_executes_with_timer_and_scene_hosts() {
        let document = generic_fixture_document();
        let source_paths = required_path_strings(include_str!(
            "../tests/fixtures/generic-timer-table-scene/mech.mcfg"
        ))
        .unwrap();
        assert_eq!(source_paths, vec!["table-scene.mec".to_string()]);

        let scene_backend = mech_host_scene::RecordingSceneBackend::new();
        let mut builder = browser_runtime_builder()
            .source_resolver(project_source_resolver(&generic_fixture_sources()).unwrap())
            .host_input_capacity(16)
            .host_factory(Box::new(TestManualTimerHostFactory::new()))
            .unwrap()
            .host_factory(Box::new(
                mech_host_scene::SceneHostFactory::with_backend(scene_backend.clone()).unwrap(),
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

        let initial_scene = scene_backend.latest().unwrap();
        assert_eq!(initial_scene.circles.len(), 2);
        assert_eq!(initial_scene.lines.len(), 3);
        let initial_x = initial_scene.circles[0].x;

        runtime.start_input_drivers().unwrap();
        runtime
            .ingress()
            .submit(fixture_timer_packet(1, 0.25))
            .unwrap();
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
            runtime
                .ingress()
                .submit(fixture_timer_packet(tick, 0.25))
                .unwrap();
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
    use std::cell::RefCell;
    use std::rc::Rc;
    use wasm_bindgen_test::*;

    wasm_bindgen_test_configure!(run_in_browser);

    fn encoded_document(source: &str) -> String {
        let tree = mech_syntax::parser::parse(source).unwrap();
        mech_core::nodes::compress_and_encode(&tree).unwrap()
    }

    #[cfg(feature = "served_project_authority")]
    fn served_document_authority() -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_host_browser::BrowserHostRuntimeConfig::from(
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
        "+> ./math.mec\n@clock := time://clock/clock{:read(second)}\nconfigured-answer := math/value + @clock/second * 0\n~answer := 41\nanswer\n"
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
        let configured_answer = document.rendered_symbol(0, "configured-answer").unwrap();
        assert_eq!(
            Reflect::get(&configured_answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("41"),
        );
    }

    const DOCUMENT_TEST_INPUT_BASE_URI: &str = "test://clock/ticks";

    #[derive(Debug, Default)]
    struct DocumentInputDriverState {
        starts: usize,
        stops: usize,
        live: bool,
        ingress: Option<mech_runtime::RuntimeIngress>,
    }

    #[derive(Clone, Debug)]
    struct DocumentInputDriver {
        state: Rc<RefCell<DocumentInputDriverState>>,
    }

    impl mech_runtime::RuntimeHostInputDriver for DocumentInputDriver {
        fn drives(&self, source: &mech_runtime::RuntimeHostInputSource) -> bool {
            source.base_uri() == DOCUMENT_TEST_INPUT_BASE_URI && source.path() == "value"
        }

        fn attach(&mut self, ingress: mech_runtime::RuntimeIngress) -> mech_core::MResult<()> {
            self.state.borrow_mut().ingress = Some(ingress);
            Ok(())
        }

        fn start(&mut self) -> mech_core::MResult<()> {
            let mut state = self.state.borrow_mut();
            state.starts += 1;
            state.live = true;
            Ok(())
        }

        fn stop(&mut self) -> mech_core::MResult<()> {
            let mut state = self.state.borrow_mut();
            state.stops += 1;
            state.live = false;
            Ok(())
        }

        fn is_live(&self) -> bool {
            self.state.borrow().live
        }
    }

    #[derive(Debug)]
    struct DocumentInputProvider;

    impl mech_runtime::RuntimeResourceProvider for DocumentInputProvider {
        fn scheme(&self) -> &str {
            "test"
        }

        fn base_uris(&self) -> Vec<String> {
            vec![DOCUMENT_TEST_INPUT_BASE_URI.to_string()]
        }

        fn read(
            &self,
            request: mech_runtime::RuntimeResourceReadRequest,
        ) -> mech_core::MResult<mech_core::Value> {
            if request.base_uri == DOCUMENT_TEST_INPUT_BASE_URI && request.path == "value" {
                return Ok(mech_core::Value::F64(mech_core::Ref::new(0.0)));
            }
            Err(MechError::new(
                ProjectError {
                    message: "missing document test resource".to_string(),
                },
                None,
            ))
        }
    }

    #[derive(Debug)]
    struct DocumentInputHostFactory {
        manifest: mech_runtime::HostManifestConfig,
        state: Rc<RefCell<DocumentInputDriverState>>,
    }

    impl DocumentInputHostFactory {
        fn new(state: Rc<RefCell<DocumentInputDriverState>>) -> Self {
            Self {
                manifest: mech_runtime::HostManifestConfig {
                    provider: "document-test-input".to_string(),
                    contexts: vec![mech_runtime::HostContextManifest {
                        name: "ticks".to_string(),
                        base_uri_template: "test://{instance}/ticks".to_string(),
                        operations: vec!["read".to_string()],
                    }],
                },
                state,
            }
        }
    }

    impl mech_runtime::RuntimeHostFactory for DocumentInputHostFactory {
        fn provider_name(&self) -> &str {
            "document-test-input"
        }

        fn manifest(&self) -> &mech_runtime::HostManifestConfig {
            &self.manifest
        }

        fn validate_settings(
            &self,
            _instance_name: &str,
            _settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<()> {
            Ok(())
        }

        fn instantiate(
            &self,
            _instance_name: &str,
            _settings: &mech_runtime::ConfigValue,
        ) -> mech_core::MResult<mech_runtime::RuntimeHostInstallation> {
            Ok(mech_runtime::RuntimeHostInstallation {
                interface: mech_runtime::materialize_host_manifest("clock", &self.manifest)?,
                resource_providers: vec![Box::new(DocumentInputProvider)],
                input_drivers: vec![Box::new(DocumentInputDriver {
                    state: self.state.clone(),
                })],
            })
        }
    }

    fn document_with_manual_input_driver() -> (WasmDocument, Rc<RefCell<DocumentInputDriverState>>)
    {
        let state = Rc::new(RefCell::new(DocumentInputDriverState::default()));
        let runtime = browser_runtime_builder()
            .host_factory(Box::new(DocumentInputHostFactory::new(state.clone())))
            .unwrap()
            .host_instance(mech_runtime::HostInstanceConfig {
                name: "clock".to_string(),
                provider: "document-test-input".to_string(),
                settings: mech_runtime::ConfigValue::Map(Default::default()),
            })
            .run_resource_grant(mech_runtime::RunResourceGrantConfig {
                target: "clock/ticks".to_string(),
                operations: vec!["read".to_string()],
                paths: vec!["value".to_string()],
            })
            .build()
            .unwrap();
        (
            WasmDocument {
                project: WasmProject::from_runtime(
                    runtime,
                    #[cfg(feature = "browser_host_scene")]
                    BrowserSceneRegistry::new(),
                ),
                bootstrap: WasmDocumentBootstrap::Detached,
            },
            state,
        )
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
            runtime: mech_host_browser::BrowserHostRuntimeConfig::from(
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
    fn encoded_document_executes_and_exposes_detached_render_queries() {
        let tree = mech_syntax::parser::parse("answer := 41 + 1\nanswer").unwrap();
        let encoded = mech_core::nodes::compress_and_encode(&tree).unwrap();
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        let rendered = document.rendered_symbol(0, "answer").unwrap();
        assert!(!rendered.is_null());
        assert_eq!(
            Reflect::get(&rendered, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("42"),
        );
        assert!(document.rendered_output(0, u64::MAX).unwrap().is_null());
        document.start().unwrap();
        assert!(document.frame(1).is_ok());
        document.stop().unwrap();
    }

    #[wasm_bindgen_test]
    fn encoded_document_uses_the_formatter_root_namespace_for_inline_output() {
        let encoded =
            encoded_document("The document evaluates {answer + 1} inline.\n\nanswer := 41");
        let document = WasmDocument::from_encoded(&encoded).unwrap();
        let output_id = mech_core::hash_str("inline-eval:0:0");
        let rendered = document.rendered_output(0, output_id).unwrap();

        assert_eq!(
            Reflect::get(&rendered, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("42"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_document_evaluate_returns_rendered_value() {
        let encoded = encoded_document("answer := 41 + 1\nanswer");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();

        let rendered = document.evaluate("answer + 1").unwrap();
        assert_eq!(
            Reflect::get(&rendered, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("43"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_document_evaluate_refreshes_mutable_root_symbol() {
        let encoded = encoded_document("~answer := 41\nanswer");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();

        document.evaluate("answer = 7").unwrap();

        let answer = document.rendered_symbol(0, "answer").unwrap();
        assert_eq!(
            Reflect::get(&answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("7"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_document_evaluate_starts_newly_relevant_input_driver() {
        let (mut document, state) = document_with_manual_input_driver();
        document.start().unwrap();
        assert_eq!(state.borrow().starts, 0);

        document
            .evaluate("@tick := test://clock/ticks{:read(value)}\ncurrent := @tick/value\ncurrent")
            .unwrap();
        assert_eq!(state.borrow().starts, 1);

        document
            .project
            .runtime
            .ingress()
            .submit(mech_runtime::RuntimeHostInput::single(
                mech_runtime::RuntimeHostInputSource::new(DOCUMENT_TEST_INPUT_BASE_URI, "value")
                    .unwrap(),
                mech_runtime::RuntimeHostInputValue::F64(7.0),
            ))
            .unwrap();
        document.frame(1).unwrap();
        let current = document.rendered_symbol(0, "current").unwrap();
        assert_eq!(
            Reflect::get(&current, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("7"),
        );

        document.evaluate("unrelated := 1\nunrelated").unwrap();
        assert_eq!(state.borrow().starts, 1);
        document.stop().unwrap();
        assert_eq!(state.borrow().stops, 1);
    }

    #[wasm_bindgen_test]
    fn wasm_document_failed_evaluation_does_not_start_driver() {
        let (mut document, state) = document_with_manual_input_driver();
        document.start().unwrap();

        assert!(document.evaluate("missing-document-symbol").is_err());
        assert_eq!(state.borrow().starts, 0);
        document.stop().unwrap();
        assert_eq!(state.borrow().stops, 1);
    }

    #[wasm_bindgen_test]
    fn wasm_document_reset_restores_initial_program() {
        let initial = encoded_document("answer := 1\nanswer");
        let mut document = WasmDocument::from_encoded(&initial).unwrap();
        document.evaluate("temporary := 9\ntemporary").unwrap();
        assert!(!document.rendered_symbol(0, "temporary").unwrap().is_null());

        document.reset(&initial).unwrap();

        assert!(document.rendered_symbol(0, "temporary").unwrap().is_null());
        let answer = document.rendered_symbol(0, "answer").unwrap();
        assert_eq!(
            Reflect::get(&answer, &JsValue::from_str("inlineHtml"))
                .unwrap()
                .as_string()
                .as_deref(),
            Some("1"),
        );
    }

    #[wasm_bindgen_test]
    fn wasm_document_step_rejects_zero() {
        let encoded = encoded_document("answer := 1\nanswer");
        let mut document = WasmDocument::from_encoded(&encoded).unwrap();
        assert!(document.step(0).is_err());
    }

    #[wasm_bindgen_test]
    fn wasm_document_rendered_symbols_returns_detached_rows() {
        let initial = encoded_document("answer := 42\nanswer");
        let replacement = encoded_document("answer := 7\nanswer");
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
    fn wasm_document_named_interpreter_lookup_is_stable() {
        let encoded = encoded_document("~~~mech:foo\nanswer := 7\n~~~");
        let document = WasmDocument::from_encoded(&encoded).unwrap();
        let id = document.interpreter_id_by_name("foo").unwrap();
        assert!(id.is_bigint());
        assert_eq!(
            id,
            JsValue::from(js_sys::BigInt::from(mech_core::hash_str("foo"))),
        );
        assert!(
            document
                .interpreter_id_by_name("missing")
                .unwrap()
                .is_null()
        );
    }

    #[wasm_bindgen_test]
    fn encoded_fizzbuzz_document_uses_native_formatter_output_key() {
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
        let document = WasmDocument::from_encoded(&encoded).unwrap();
        assert!(
            !document.rendered_output(0, output_id).unwrap().is_null(),
            "the formatted FizzBuzz output must remain renderable in WASM",
        );
    }

    #[wasm_bindgen_test]
    fn generic_project_frame_respects_input_bound() {
        let config = r#"config := { hosts: [] run: { paths: ["generic-table.mec"] grants: [] } }"#;
        let sources = Object::new();
        Reflect::set(
            &sources,
            &JsValue::from_str("generic-table.mec"),
            &JsValue::from_str(
                r#"delta := 0.25
rows := |id<string> x<f64>|
  | "row-a" 1 + delta |
  | "row-b" 2 + delta |"#,
            ),
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
            &JsValue::from_str(include_str!(
                "../tests/fixtures/generic-timer-table-scene/table-scene.mec"
            )),
        )
        .unwrap();
        let mut project = WasmProject::from_sources(config, sources.into()).unwrap();
        project.start().unwrap();
        let result = project.frame(1).unwrap();
        assert_eq!(
            Reflect::get(&result, &JsValue::from_str("rendered"))
                .unwrap()
                .as_f64(),
            Some(1.0)
        );
        project.stop().unwrap();
        canvas.remove();
    }
}
