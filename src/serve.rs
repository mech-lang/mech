use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};
use std::time::Instant;

use colored::*;
use ignore::WalkBuilder;
use mech_core::*;
use mech_host_browser::BrowserRuntimeInjectionConfig;
use mech_runtime::{
    DefaultIdGenerator, EventId, EventSink, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE,
    FS_WATCH, HostFilesystemAuthority, MECH_TOOL_SUBJECT, ModuleBuildOptions, RuntimeConfig,
    RuntimeEvent, RuntimeWorkspaceFolder, RuntimeWorkspaceSnapshot, RuntimeWorkspaceTarget,
    RuntimeWorkspaceWatchEvent, SERVE_HOST_SUBJECT, ServerWorkspaceSession, SourceKind,
    check_fs_capability,
};
use warp::{Filter, Reply};

use crate::*;

#[derive(Clone, Debug)]
struct ServerAsset {
    bytes: Vec<u8>,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    backing_paths: Vec<PathBuf>,
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for path in paths {
        if seen.insert(path.clone()) {
            out.push(path);
        }
    }
    out
}

#[derive(Clone, Copy, Debug)]
struct ServerRegistrySummary {
    assets: usize,
    raw_sources: usize,
    html_sources: usize,
    code_sources: usize,
    static_assets: usize,
}

#[derive(Debug, Default)]
struct ServerSourceRegistry {
    assets: HashMap<String, ServerAsset>,
    raw_sources: HashMap<String, ServerAsset>,
    html_sources: HashMap<String, ServerAsset>,
    code_sources: HashMap<String, ServerAsset>,
    source_paths: HashMap<String, PathBuf>,
    workspace_keys: HashSet<String>,
    static_asset_paths: HashMap<String, PathBuf>,
    user_assets: HashSet<String>,
    index_source: Option<String>,
    preferred_index_source: Option<String>,
    listing_asset: Option<ServerAsset>,
    capability_kernel: Option<mech_runtime::SharedCapabilityKernel>,
    capability_subject: Option<String>,
}

impl ServerSourceRegistry {
    fn with_capabilities(
        &mut self,
        kernel: mech_runtime::SharedCapabilityKernel,
        subject: impl Into<String>,
    ) {
        self.capability_kernel = Some(kernel);
        self.capability_subject = Some(subject.into());
    }

    fn check(&self, operation: &str, path: &Path) -> MResult<()> {
        if let (Some(kernel), Some(subject)) = (&self.capability_kernel, &self.capability_subject) {
            check_fs_capability(&mut kernel.clone(), subject, operation, path)?;
        }
        Ok(())
    }

    fn insert_asset(&mut self, key: impl Into<String>, asset: ServerAsset) {
        self.assets.insert(key.into(), asset);
    }

    fn insert_user_asset(&mut self, key: impl Into<String>, asset: ServerAsset) {
        let key = key.into();
        self.user_assets.insert(key.clone());
        self.insert_asset(key, asset);
    }

    fn summary(&self) -> ServerRegistrySummary {
        ServerRegistrySummary {
            assets: self.assets.len(),
            raw_sources: self.raw_sources.len(),
            html_sources: self.html_sources.len(),
            code_sources: self.code_sources.len(),
            static_assets: self.static_asset_paths.len(),
        }
    }

    fn source_keys(&self) -> Vec<String> {
        self.raw_sources.keys().cloned().collect()
    }

    fn static_asset_keys(&self) -> Vec<String> {
        self.static_asset_paths.keys().cloned().collect()
    }

    fn get_route(&self, path: &str) -> Option<ServerAsset> {
        self.get_route_with_trace(path).map(|(asset, _)| asset)
    }

    fn effective_index_source(&self) -> Option<&str> {
        if let Some(preferred) = self.preferred_index_source.as_deref() {
            if self.html_sources.contains_key(preferred) {
                return Some(preferred);
            }
        }
        if let Some(index_source) = self.index_source.as_deref() {
            if self.html_sources.contains_key(index_source) {
                return Some(index_source);
            }
        }
        if self.html_sources.len() == 1 {
            return self.html_sources.keys().next().map(String::as_str);
        }
        None
    }

    fn set_preferred_index_source(&mut self, source: impl Into<String>) {
        self.preferred_index_source = Some(source.into());
    }

    fn rebuild_listing(&mut self) {
        let mut keys = self.raw_sources.keys().cloned().collect::<Vec<_>>();
        keys.sort();
        if keys.is_empty() {
            self.listing_asset = None;
            return;
        }
        let mut html = "<!doctype html>\n<html>\n<head>\n  <meta charset=\"utf-8\">\n  <title>Mech Sources</title>\n</head>\n<body>\n  <h1>Mech Sources</h1>\n  <ul>\n".to_string();
        for key in keys {
            let escaped_key = escape_html(&key);
            html.push_str(&format!(
                "    <li><a href=\"/{0}\">{0}</a> <a href=\"/source/{0}\">source</a>",
                escaped_key
            ));
            if self.code_sources.contains_key(&key) {
                html.push_str(&format!(" <a href=\"/code/{0}\">code</a>", escaped_key));
            }
            html.push_str("</li>\n");
        }
        html.push_str("  </ul>\n</body>\n</html>\n");
        self.listing_asset = Some(ServerAsset {
            bytes: html.into_bytes(),
            content_type: "text/html",
            content_encoding: None,
            backing_paths: Vec::new(),
        });
    }

    fn get_route_with_trace(&self, path: &str) -> Option<(ServerAsset, String)> {
        let root_alias = path.strip_prefix('/').unwrap_or(path);
        if matches!(root_alias, "source" | "source/") {
            let source = self.effective_index_source()?;
            return self
                .raw_sources
                .get(source)
                .cloned()
                .map(|asset| (asset, format!("raw source `{}`", source)));
        }
        if matches!(root_alias, "code" | "code/") {
            let source = self.effective_index_source()?;
            return self
                .code_sources
                .get(source)
                .cloned()
                .map(|asset| (asset, format!("code source `{}`", source)));
        }
        let normalized = normalize_url_path(path)?;
        if let Some(source) = normalized.strip_prefix("source/") {
            return self
                .raw_sources
                .get(source)
                .cloned()
                .map(|asset| (asset, format!("raw source `{}`", source)));
        }
        if let Some(source) = normalized.strip_prefix("code/") {
            return self
                .code_sources
                .get(source)
                .cloned()
                .map(|asset| (asset, format!("code source `{}`", source)));
        }
        if normalized == "index.html" {
            if self.user_assets.contains("index.html") {
                return self
                    .assets
                    .get("index.html")
                    .cloned()
                    .map(|asset| (asset, "user asset `index.html`".to_string()));
            }
            if let Some(source) = self.effective_index_source() {
                let trace = if self.preferred_index_source.as_deref() == Some(source) {
                    "preferred generated html"
                } else if self.index_source.as_deref() == Some(source) {
                    "generated index html"
                } else {
                    "single generated html"
                };
                return self
                    .html_sources
                    .get(source)
                    .cloned()
                    .map(|asset| (asset, format!("{} `{}`", trace, source)));
            }
            if let Some(asset) = &self.listing_asset {
                return Some((asset.clone(), "generated source listing".to_string()));
            }
            return self
                .assets
                .get("_mech/index.html")
                .cloned()
                .map(|asset| (asset, "bundled asset `_mech/index.html`".to_string()))
                .or_else(|| {
                    self.assets
                        .get("index.html")
                        .cloned()
                        .map(|asset| (asset, "bundled asset `index.html`".to_string()))
                });
        }
        if normalized.ends_with(".mec") || normalized.ends_with(".🤖") {
            return self
                .html_sources
                .get(&normalized)
                .cloned()
                .map(|asset| (asset, format!("generated html `{}`", normalized)));
        }
        if normalized.ends_with(".html") || normalized.ends_with(".htm") {
            if let Some(asset) = self.assets.get(&normalized) {
                return Some((asset.clone(), format!("asset `{}`", normalized)));
            }
            let source = Path::new(&normalized).with_extension("mec");
            let key = url_key(&source)?;
            return self
                .html_sources
                .get(&key)
                .cloned()
                .map(|asset| (asset, format!("generated html fallback `{}`", key)));
        }
        self.assets
            .get(&normalized)
            .cloned()
            .map(|asset| (asset, format!("asset `{}`", normalized)))
    }

    fn insert_static_file(&mut self, root: &Path, path: &Path) -> MResult<()> {
        if !is_allowed_static_file(path) {
            return Ok(());
        }
        let path = path.canonicalize()?;
        self.check(FS_READ, &path)?;
        let relative = path.strip_prefix(root).map_err(|error| {
            Error::new(
                ErrorKind::InvalidInput,
                format!("static asset is outside workspace root: {}", error),
            )
        })?;
        let Some(key) = url_key(relative) else {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid static asset path").into());
        };
        self.insert_user_asset(
            key.clone(),
            ServerAsset {
                bytes: std::fs::read(&path)?,
                content_type: content_type_for_path(&key),
                content_encoding: content_encoding_for_path(&key),
                backing_paths: vec![path.clone()],
            },
        );
        self.static_asset_paths.insert(key, path);
        Ok(())
    }

    fn reload_static_path(&mut self, root: &Path, path: &Path) -> MResult<bool> {
        if is_workspace_target_source(path) {
            return Ok(false);
        }
        let Some(key) = static_key_for_path(root, path) else {
            return Ok(false);
        };
        if path.exists() && path.is_file() && is_allowed_static_file(path) {
            self.insert_static_file(root, path)?;
            return Ok(true);
        }
        if self.static_asset_paths.contains_key(&key) || self.user_assets.contains(&key) {
            self.assets.remove(&key);
            self.user_assets.remove(&key);
            self.static_asset_paths.remove(&key);
            return Ok(true);
        }
        Ok(false)
    }

    fn sync_workspace_snapshot(
        &mut self,
        root: &Path,
        snapshot: &RuntimeWorkspaceSnapshot,
        stylesheet: &str,
        shim: &str,
        generated_html_backing_paths: &[PathBuf],
    ) -> MResult<()> {
        for key in self.workspace_keys.drain() {
            self.raw_sources.remove(&key);
            self.html_sources.remove(&key);
            self.code_sources.remove(&key);
            self.source_paths.remove(&key);
        }
        self.index_source = None;

        for source in snapshot.sources.values() {
            let Some(path) = source.path.as_ref() else {
                continue;
            };
            if !is_renderable_mech_text_source(path) {
                continue;
            }
            let relative = path.strip_prefix(root).map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("workspace source is outside workspace root: {}", error),
                )
            })?;
            let Some(key) = url_key(relative) else {
                continue;
            };
            self.check(FS_READ, path)?;
            let source = std::fs::read_to_string(path)?;
            self.raw_sources.insert(
                key.clone(),
                ServerAsset {
                    bytes: source.as_bytes().to_vec(),
                    content_type: "text/x-mech",
                    content_encoding: None,
                    backing_paths: vec![path.clone()],
                },
            );
            match parser::parse(&source) {
                Ok(tree) => {
                    let mut formatter = Formatter::new();
                    let html =
                        formatter.format_html(&tree, stylesheet.to_string(), shim.to_string());
                    let mut backing_paths = vec![path.clone()];
                    backing_paths.extend(generated_html_backing_paths.iter().cloned());
                    self.html_sources.insert(
                        key.clone(),
                        ServerAsset {
                            bytes: html.into_bytes(),
                            content_type: "text/html",
                            content_encoding: None,
                            backing_paths: dedupe_paths(backing_paths),
                        },
                    );
                    #[cfg(feature = "serde")]
                    self.code_sources.insert(
                        key.clone(),
                        ServerAsset {
                            bytes: compress_and_encode(&tree)
                                .map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?
                                .into_bytes(),
                            content_type: "text/plain",
                            content_encoding: None,
                            backing_paths: vec![path.clone()],
                        },
                    );
                }
                Err(error) => {
                    let html = format!(
                        "<html><body><pre>{}</pre></body></html>",
                        escape_html(&format!("{:#?}", error))
                    );
                    self.html_sources.insert(
                        key.clone(),
                        ServerAsset {
                            bytes: html.into_bytes(),
                            content_type: "text/html",
                            content_encoding: None,
                            backing_paths: vec![path.clone()],
                        },
                    );
                }
            }
            self.source_paths.insert(key.clone(), path.clone());
            self.workspace_keys.insert(key.clone());
            if is_index_source_key(&key) {
                self.index_source = Some(key);
            }
        }
        self.rebuild_listing();
        Ok(())
    }
}

fn is_index_source_key(key: &str) -> bool {
    key == "index.mec" || key.ends_with("/index.mec")
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct DelegationKey {
    path: PathBuf,
    recursive: bool,
}

fn planned_delegations(plan: &ServeInputPlan) -> BTreeMap<DelegationKey, BTreeSet<&'static str>> {
    let mut delegations = BTreeMap::<DelegationKey, BTreeSet<&'static str>>::new();
    let mut add = |path: PathBuf, recursive: bool, operations: &[&'static str]| {
        delegations
            .entry(DelegationKey { path, recursive })
            .or_default()
            .extend(operations);
    };
    for folder in &plan.folders {
        add(
            plan.root.join(&folder.specifier),
            true,
            &[FS_LIST, FS_WATCH, FS_READ, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
    }
    for target in &plan.targets {
        let path = plan.root.join(&target.specifier);
        add(
            path.clone(),
            false,
            &[FS_READ, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
        );
    }
    for static_path in &plan.static_paths {
        let path = plan.root.join(static_path);
        let recursive = path.is_dir();
        add(path, recursive, &[FS_READ, FS_WATCH, FS_SERVE]);
    }
    delegations
}

fn display_fs_resource(path: &Path) -> String {
    mech_runtime::fs_resource_key(path).unwrap_or_else(|_| path.display().to_string())
}

pub struct MechServer {
    name: String,
    init: bool,
    stylesheet: String,
    html_shim: String,
    host_config: Option<BrowserRuntimeInjectionConfig>,
    host_config_injection: Option<HostAuthorityInjection>,
    serve_configured_shim_at_root: bool,
    full_address: String,
    registry: Arc<RwLock<ServerSourceRegistry>>,
    events: Arc<RwLock<Vec<RuntimeEvent>>>,
    workspace_session: Option<Arc<Mutex<ServerWorkspaceSession>>>,
    workspace_root: Option<PathBuf>,
    js: Vec<u8>,
    wasm: Vec<u8>,
    html_shim_backing_paths: Vec<PathBuf>,
    stylesheet_backing_paths: Vec<PathBuf>,
    wasm_backing_paths: Vec<PathBuf>,
    js_backing_paths: Vec<PathBuf>,
    authority: HostFilesystemAuthority,
    serve_subject: String,
    runtime_config: RuntimeConfig,
}

impl MechServer {
    pub fn new(
        name: String,
        full_address: String,
        stylesheet: String,
        html_shim: String,
        wasm: Vec<u8>,
        js: Vec<u8>,
        authority: HostFilesystemAuthority,
    ) -> Self {
        Self::new_with_runtime_config(
            name,
            full_address,
            stylesheet,
            html_shim,
            wasm,
            js,
            authority,
            RuntimeConfig::default(),
        )
    }

    pub fn new_with_runtime_config(
        name: String,
        full_address: String,
        stylesheet: String,
        html_shim: String,
        wasm: Vec<u8>,
        js: Vec<u8>,
        authority: HostFilesystemAuthority,
        runtime_config: RuntimeConfig,
    ) -> Self {
        Self::new_with_runtime_config_and_host_config(
            name,
            full_address,
            stylesheet,
            html_shim,
            wasm,
            js,
            authority,
            runtime_config,
            None,
            None,
            false,
        )
    }

    pub fn new_with_runtime_config_and_host_config(
        name: String,
        full_address: String,
        stylesheet: String,
        html_shim: String,
        wasm: Vec<u8>,
        js: Vec<u8>,
        authority: HostFilesystemAuthority,
        runtime_config: RuntimeConfig,
        host_config: Option<BrowserRuntimeInjectionConfig>,
        host_config_injection: Option<HostAuthorityInjection>,
        serve_configured_shim_at_root: bool,
    ) -> Self {
        Self {
            name,
            init: false,
            stylesheet,
            html_shim,
            host_config,
            host_config_injection,
            serve_configured_shim_at_root,
            full_address,
            registry: Arc::new(RwLock::new(ServerSourceRegistry::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            workspace_session: None,
            workspace_root: None,
            js,
            wasm,
            html_shim_backing_paths: Vec::new(),
            stylesheet_backing_paths: Vec::new(),
            wasm_backing_paths: Vec::new(),
            js_backing_paths: Vec::new(),
            authority,
            serve_subject: SERVE_HOST_SUBJECT.to_string(),
            runtime_config,
        }
    }

    pub fn set_resource_backing_paths(
        &mut self,
        html_shim: Vec<PathBuf>,
        stylesheets: Vec<PathBuf>,
        wasm: Vec<PathBuf>,
        js: Vec<PathBuf>,
    ) {
        self.html_shim_backing_paths = dedupe_paths(html_shim);
        self.stylesheet_backing_paths = dedupe_paths(stylesheets);
        self.wasm_backing_paths = dedupe_paths(wasm);
        self.js_backing_paths = dedupe_paths(js);
    }

    fn generated_html_backing_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.html_shim_backing_paths.clone();
        paths.extend(self.stylesheet_backing_paths.iter().cloned());
        dedupe_paths(paths)
    }

    fn configured_resource_backing_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.html_shim_backing_paths.clone();
        paths.extend(self.stylesheet_backing_paths.iter().cloned());
        paths.extend(self.wasm_backing_paths.iter().cloned());
        paths.extend(self.js_backing_paths.iter().cloned());
        dedupe_paths(paths)
    }

    fn injected_html_shim(&self) -> MResult<String> {
        if let Some(injection) = &self.host_config_injection {
            inject_host_authority_injection_script(&self.html_shim, injection)
        } else if let Some(host_config) = &self.host_config {
            inject_browser_host_config_script(&self.html_shim, host_config)
        } else {
            Ok(self.html_shim.clone())
        }
    }

    pub async fn init(&mut self) -> MResult<()> {
        let mut ids = DefaultIdGenerator::new();
        for path in self.configured_resource_backing_paths() {
            self.authority.delegate_path_to(
                &mut ids,
                &self.serve_subject,
                &path,
                false,
                [FS_SERVE],
            )?;
        }
        let html_shim = self.injected_html_shim()?;
        let mut registry = self.registry.write().unwrap();
        let html = asset(
            html_shim.as_bytes(),
            "text/html",
            None,
            self.html_shim_backing_paths.clone(),
        );
        let css = asset(
            self.stylesheet.as_bytes(),
            "text/css",
            None,
            self.stylesheet_backing_paths.clone(),
        );
        let js = asset(
            &self.js,
            "application/javascript",
            None,
            self.js_backing_paths.clone(),
        );
        let wasm = asset(
            &self.wasm,
            "application/wasm",
            Some("br"),
            self.wasm_backing_paths.clone(),
        );
        if self.serve_configured_shim_at_root {
            registry.insert_user_asset("index.html", html.clone());
        } else {
            registry.insert_asset("index.html", html.clone());
        }
        registry.insert_asset("_mech/index.html", html);
        registry.insert_asset("_mech/style.css", css);
        registry.insert_asset("_mech/pkg/mech_wasm.js", js.clone());
        registry.insert_asset("_mech/pkg/mech_wasm_bg.wasm", wasm.clone());
        registry.insert_asset("_mech/pkg/mech_wasm_bg.wasm.br", wasm.clone());
        registry.insert_asset("pkg/mech_wasm.js", js);
        registry.insert_asset("pkg/mech_wasm_bg.wasm", wasm);
        self.init = true;
        Ok(())
    }

    fn badge(&self) -> ColoredString {
        if self.name.ends_with(" Server") {
            format!("[{}]", self.name).truecolor(34, 204, 187)
        } else {
            format!("[{}] Server", self.name).truecolor(34, 204, 187)
        }
    }

    fn delegate_plan(&self, plan: &ServeInputPlan) -> MResult<()> {
        let mut ids = DefaultIdGenerator::new();
        for (key, operations) in planned_delegations(plan) {
            let operations = operations.into_iter().collect::<Vec<_>>();
            let resource = display_fs_resource(&key.path);
            println!(
                "{} Capability requested: {} {} recursive={} operations={}",
                self.badge(),
                self.serve_subject,
                resource,
                key.recursive,
                operations.join(",")
            );
            self.authority.delegate_path_to(
                &mut ids,
                &self.serve_subject,
                &key.path,
                key.recursive,
                operations.iter().copied(),
            )?;
            println!(
                "{} Capability delegated: {} {} recursive={} operations={}",
                self.badge(),
                self.serve_subject,
                resource,
                key.recursive,
                operations.join(",")
            );
        }
        Ok(())
    }

    pub fn load_workspace(&mut self, paths: &Vec<String>) -> MResult<()> {
        if !self.init {
            return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
        }
        let started = Instant::now();
        let plan = plan_serve_inputs(paths)?;
        self.delegate_plan(&plan)?;
        self.registry
            .write()
            .unwrap()
            .with_capabilities(self.authority.kernel().clone(), self.serve_subject.clone());
        let root = plan.root.clone();
        self.workspace_root = Some(root.clone());
        println!("{} Loading workspace…", self.badge());
        println!("{} Serve input plan:", self.badge());
        println!("{}   root: {}", self.badge(), root.display());
        println!("{}   targets: {}", self.badge(), plan.targets.len());
        println!("{}   folders: {}", self.badge(), plan.folders.len());
        println!(
            "{}   static inputs: {}",
            self.badge(),
            plan.static_paths.len()
        );
        println!("{} Workspace root: {}", self.badge(), root.display());
        for target in &plan.targets {
            println!(
                "{} Target `{}` -> `{}`",
                self.badge(),
                target.name,
                target.specifier
            );
        }
        for folder in &plan.folders {
            println!(
                "{} Folder `{}` recursive={}",
                self.badge(),
                folder.specifier,
                folder.recursive
            );
        }
        for specifier in &plan.static_paths {
            println!("{} Static input: `{}`", self.badge(), specifier);
        }
        log_skipped_serve_inputs(paths)?;
        if paths.is_empty() {
            println!(
                "{} No serve inputs provided; recursively discovering sources from current directory `{}`.",
                self.badge(),
                root.display()
            );
        }
        let static_started = Instant::now();
        println!("{} Loading static assets…", self.badge());
        load_static_assets_from_paths(
            &mut self.registry.write().unwrap(),
            &root,
            &plan.static_paths,
        )?;
        println!(
            "{} Static assets loaded in {:?}.",
            self.badge(),
            static_started.elapsed()
        );
        let session_started = Instant::now();
        println!("{} Opening runtime workspace session…", self.badge());
        let mut session = ServerWorkspaceSession::open_with_capabilities_and_config(
            &root,
            plan.targets,
            plan.folders,
            module_options(),
            self.authority.kernel().clone(),
            self.serve_subject.clone(),
            self.runtime_config.clone(),
        )?;
        println!(
            "{} Runtime workspace session opened in {:?}.",
            self.badge(),
            session_started.elapsed()
        );
        for path in session.watcher().watched_paths() {
            println!("{} Watching: {}", self.badge(), path.display());
        }
        let mut registry = self.registry.write().unwrap();
        registry.preferred_index_source = None;
        if let Some(source) = plan.preferred_index_source {
            registry.set_preferred_index_source(source);
        }
        drop(registry);
        println!("{} Emitting initial workspace events…", self.badge());
        let mut sink = ServerEventSink {
            events: self.events.clone(),
        };
        session.emit_initial_events(&mut sink)?;
        let sync_started = Instant::now();
        println!("{} Building served source registry views…", self.badge());
        if let Some(snapshot) = session.snapshot() {
            for diagnostic in &snapshot.diagnostics {
                println!(
                    "{} Workspace diagnostic: {}",
                    self.badge(),
                    diagnostic.message
                );
                if diagnostic.message.contains("Capability denied")
                    || diagnostic.message.contains("CapabilityDenied")
                {
                    println!("{} Capability denied: {}", self.badge(), diagnostic.message);
                }
            }
            let html_shim = self.injected_html_shim()?;
            self.registry.write().unwrap().sync_workspace_snapshot(
                &root,
                snapshot,
                &self.stylesheet,
                &html_shim,
                &self.generated_html_backing_paths(),
            )?;
        }
        let registry = self.registry.read().unwrap();
        for key in registry.source_keys() {
            println!("{} Loaded source: {}", self.badge(), key);
        }
        for key in registry.static_asset_keys() {
            println!("{} Loaded static asset: {}", self.badge(), key);
        }
        let summary = registry.summary();
        drop(registry);
        println!(
            "{} Registry ready in {:?}: {} assets, {} static assets, {} raw sources, {} html sources, {} code sources.",
            self.badge(),
            sync_started.elapsed(),
            summary.assets,
            summary.static_assets,
            summary.raw_sources,
            summary.html_sources,
            summary.code_sources
        );
        println!(
            "{} Workspace loaded in {:?}.",
            self.badge(),
            started.elapsed()
        );
        self.workspace_session = Some(Arc::new(Mutex::new(session)));
        Ok(())
    }

    pub async fn serve(&self) -> MResult<()> {
        if !self.init {
            return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
        }

        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        let server_name = self.name.clone();

        ctrlc::set_handler(move || {
            let badge = if server_name.ends_with(" Server") {
                format!("[{}]", server_name).truecolor(34, 204, 187)
            } else {
                format!("[{}] Server", server_name).truecolor(34, 204, 187)
            };

            let _ = shutdown_tx.send(true);
            println!("{} Server received shutdown signal.", badge);
        })
        .map_err(|error| {
            MechError::new(
                GenericError {
                    msg: format!("Error setting Ctrl-C handler: {}", error),
                },
                None,
            )
            .with_compiler_loc()
        })?;

        self.serve_until_shutdown(shutdown_rx).await
    }

    async fn serve_until_shutdown(
        &self,
        mut shutdown_rx: tokio::sync::watch::Receiver<bool>,
    ) -> MResult<()> {
        if !self.init {
            return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
        }

        let root = self.workspace_root.clone();
        let registry = self.registry.clone();
        let capability_kernel = self.authority.kernel().clone();
        let capability_subject = self.serve_subject.clone();
        let routes = warp::get().and(warp::path::full()).map(move |path: warp::path::FullPath| {
      match registry.read().unwrap().get_route_with_trace(path.as_str()) {
        Some((asset, trace)) => {
          if let Err(error) = authorize_server_asset(&capability_kernel, &capability_subject, &asset) {
            println!("[Mech Server] GET {} -> 403 capability denied {:?}", path.as_str(), error);
            return response(b"<html><body><h1>403 Forbidden</h1><p>Capability denied.</p></body></html>".to_vec(), "text/html", None, warp::http::StatusCode::FORBIDDEN);
          }
          println!("[Mech Server] GET {} -> {} ({}, {} bytes)", path.as_str(), trace, asset.content_type, asset.bytes.len());
          response(asset.bytes, asset.content_type, asset.content_encoding, warp::http::StatusCode::OK)
        }
        None => {
          println!("[Mech Server] GET {} -> 404", path.as_str());
          response(
          format!("<html><body><h1>404 Not Found</h1><p>The requested URL {} was not found on this server.</p></body></html>", escape_html(path.as_str())).into_bytes(),
          "text/html",
          None,
          warp::http::StatusCode::NOT_FOUND,
          )
        }
      }
    });
        println!(
            "{} Awaiting connections at {}",
            self.badge(),
            self.full_address
        );
        let socket_address: SocketAddr = self.full_address.parse().unwrap();
        let mut server_shutdown_rx = shutdown_rx.clone();
        let (_addr, server) =
            warp::serve(routes).bind_with_graceful_shutdown(socket_address, async move {
                if !*server_shutdown_rx.borrow() {
                    let _ = server_shutdown_rx.changed().await;
                }
            });
        if let (Some(session), Some(root)) = (&self.workspace_session, &root) {
            let html_shim = self.injected_html_shim()?;
            let generated_html_backing_paths = self.generated_html_backing_paths();
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
            tokio::pin!(server);
            loop {
                tokio::select! {
                  _ = interval.tick() => {
                    let _ = poll_workspace_once(session, &self.registry, &self.events, &root, &self.stylesheet, &html_shim, &generated_html_backing_paths);
                  }
                  _ = shutdown_rx.changed() => {
                    let _ = (&mut server).await;
                    break;
                  }
                  _ = &mut server => break,
                }
            }
        } else {
            tokio::pin!(server);
            tokio::select! {
              _ = &mut server => {}
              _ = shutdown_rx.changed() => {
                let _ = (&mut server).await;
              }
            }
        }
        println!("{} Closing server.", self.badge());
        Ok(())
    }
}

#[derive(Debug)]
struct ServerEventSink {
    events: Arc<RwLock<Vec<RuntimeEvent>>>,
}

impl EventSink for ServerEventSink {
    fn emit(&mut self, event: RuntimeEvent) -> MResult<EventId> {
        let id = event.id;
        self.events.write().unwrap().push(event);
        Ok(id)
    }
}

fn poll_workspace_once(
    session: &Arc<Mutex<ServerWorkspaceSession>>,
    registry: &Arc<RwLock<ServerSourceRegistry>>,
    events: &Arc<RwLock<Vec<RuntimeEvent>>>,
    root: &Path,
    stylesheet: &str,
    shim: &str,
    generated_html_backing_paths: &[PathBuf],
) -> MResult<()> {
    let mut session = session.lock().unwrap();
    let mut sink = ServerEventSink {
        events: events.clone(),
    };
    let poll = session.poll_and_emit(module_options(), &mut sink)?;
    if !poll.events.is_empty() {
        println!("[Mech Server] Watch events: {}", poll.events.len());
        for event in &poll.events {
            println!(
                "[Mech Server] Watch event: {:?} {}",
                event.kind,
                event.path.display()
            );
        }
    }
    if let Some(refresh) = &poll.refresh {
        println!(
            "[Mech Server] Workspace refresh: {} changes, {} affected targets, {} diagnostics.",
            refresh.changes.len(),
            refresh.affected_targets.len(),
            refresh.refresh_diagnostics.len()
        );
        for change in &refresh.changes {
            println!(
                "[Mech Server] Workspace change: {:?} {}",
                change.kind,
                change
                    .path
                    .as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| change.canonical_uri.clone())
            );
        }
        for diagnostic in &refresh.refresh_diagnostics {
            println!("[Mech Server] Workspace diagnostic: {:?}", diagnostic);
        }
    }
    {
        let mut registry = registry.write().unwrap();
        if sync_static_assets_from_watch_events(&mut registry, root, &poll.events)? {
            println!("[Mech Server] Static assets updated from watch events.");
        }
        if poll.refresh.is_some() {
            if let Some(snapshot) = session.snapshot() {
                registry.sync_workspace_snapshot(
                    root,
                    snapshot,
                    stylesheet,
                    shim,
                    generated_html_backing_paths,
                )?;
            }
        }
    }
    Ok(())
}

fn sync_static_assets_from_watch_events(
    registry: &mut ServerSourceRegistry,
    root: &Path,
    events: &[RuntimeWorkspaceWatchEvent],
) -> MResult<bool> {
    let mut changed = false;
    for event in events {
        if registry.reload_static_path(root, &event.path)? {
            changed = true;
        }
    }
    Ok(changed)
}

#[derive(Debug)]
struct ServeInputPlan {
    root: PathBuf,
    targets: Vec<RuntimeWorkspaceTarget>,
    folders: Vec<RuntimeWorkspaceFolder>,
    static_paths: Vec<String>,
    preferred_index_source: Option<String>,
}

fn plan_serve_inputs(paths: &[String]) -> MResult<ServeInputPlan> {
    let current_dir = std::env::current_dir()?.canonicalize()?;
    if paths.is_empty() {
        return Ok(ServeInputPlan {
            root: current_dir,
            targets: Vec::new(),
            folders: vec![RuntimeWorkspaceFolder {
                specifier: ".".to_string(),
                recursive: true,
            }],
            static_paths: Vec::new(),
            preferred_index_source: None,
        });
    }

    let mut resolved = Vec::new();
    let mut root_paths = Vec::new();
    for input in paths {
        let input_path = Path::new(input);
        let candidate = if input_path.is_absolute() {
            input_path.to_path_buf()
        } else {
            current_dir.join(input_path)
        };
        if candidate.exists() {
            let canonical = candidate.canonicalize()?;
            if canonical.is_dir() {
                root_paths.push(canonical.clone());
            } else if let Some(parent) = canonical.parent() {
                root_paths.push(parent.to_path_buf());
            }
            resolved.push(canonical);
        } else if is_workspace_target_source(&candidate) {
            let parent = candidate.parent().ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Mech target `{}` has no parent directory", input),
                )
            })?;
            let parent = parent
                .canonicalize()
                .unwrap_or_else(|_| parent.to_path_buf());
            root_paths.push(parent.clone());
            resolved.push(parent.join(candidate.file_name().ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Mech target `{}` has no file name", input),
                )
            })?));
        } else {
            resolved.push(candidate);
        }
    }

    let root = common_ancestor(&root_paths).ok_or_else(|| {
        Error::new(
            ErrorKind::InvalidInput,
            "serve inputs do not have a common workspace root",
        )
    })?;
    let mut targets = Vec::new();
    let mut folders = Vec::new();
    let mut static_paths = Vec::new();
    let mut preferred_index_source = None;
    for path in resolved {
        if path.is_dir() {
            let specifier = relative_specifier(&root, &path).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "serve directory `{}` is outside workspace root",
                        path.display()
                    ),
                )
            })?;
            folders.push(RuntimeWorkspaceFolder {
                specifier: specifier.clone(),
                recursive: true,
            });
            static_paths.push(specifier);
        } else if is_workspace_target_source(&path) {
            let specifier = relative_specifier(&root, &path).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Mech target `{}` is outside workspace root", path.display()),
                )
            })?;
            preferred_index_source.get_or_insert_with(|| specifier.clone());
            targets.push(RuntimeWorkspaceTarget {
                name: target_name(&specifier),
                specifier,
            });
        } else if path.is_file() && is_allowed_static_file(&path) {
            let specifier = relative_specifier(&root, &path).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "static asset `{}` is outside workspace root",
                        path.display()
                    ),
                )
            })?;
            static_paths.push(specifier);
        }
    }
    Ok(ServeInputPlan {
        root,
        targets,
        folders,
        static_paths,
        preferred_index_source,
    })
}

fn common_ancestor(paths: &[PathBuf]) -> Option<PathBuf> {
    let mut ancestor = paths.first()?.clone();
    for path in &paths[1..] {
        while !path.starts_with(&ancestor) {
            if !ancestor.pop() {
                return None;
            }
        }
    }
    Some(ancestor)
}

fn relative_specifier(root: &Path, path: &Path) -> Option<String> {
    let relative = path.strip_prefix(root).ok()?;
    if relative.as_os_str().is_empty() {
        Some(".".to_string())
    } else {
        url_key(relative)
    }
}

fn log_skipped_serve_inputs(paths: &[String]) -> MResult<()> {
    let current_dir = std::env::current_dir()?.canonicalize()?;
    for input in paths {
        let path = Path::new(input);
        let path = if path.is_absolute() {
            path.to_path_buf()
        } else {
            current_dir.join(path)
        };
        if !path.exists() && !is_workspace_target_source(&path) {
            println!(
                "[Mech Server] Warning: skipped missing non-Mech input `{}`.",
                input
            );
        } else if path.is_file()
            && !is_workspace_target_source(&path)
            && !is_allowed_static_file(&path)
        {
            println!(
                "[Mech Server] Warning: skipped unsupported file `{}`.",
                input
            );
        }
    }
    Ok(())
}

fn load_static_assets_from_paths(
    registry: &mut ServerSourceRegistry,
    root: &Path,
    paths: &[String],
) -> MResult<()> {
    for input in paths {
        let path = root.join(input);
        if path.is_file() {
            if !is_workspace_target_source(&path) {
                registry.insert_static_file(root, &path)?;
            }
        } else if path.is_dir() {
            for entry in WalkBuilder::new(&path).build() {
                let entry =
                    entry.map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?;
                if entry
                    .file_type()
                    .map(|kind| kind.is_file())
                    .unwrap_or(false)
                    && !is_workspace_target_source(entry.path())
                {
                    registry.insert_static_file(root, entry.path())?;
                }
            }
        }
    }
    Ok(())
}

fn normalize_url_path(path: &str) -> Option<String> {
    let path = path.strip_prefix('/').unwrap_or(path);
    if path.is_empty() {
        return Some("index.html".to_string());
    }
    if path.starts_with('/') || path.contains('\\') {
        return None;
    }
    let segments: Vec<&str> = path.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == ".." || segment.contains(':'))
    {
        return None;
    }
    Some(segments.join("/"))
}

fn url_key(path: &Path) -> Option<String> {
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => segments.push(segment.to_string_lossy().to_string()),
            _ => return None,
        }
    }
    normalize_url_path(&segments.join("/"))
}

fn static_key_for_path(root: &Path, path: &Path) -> Option<String> {
    let candidate = if path.is_absolute() {
        path.to_path_buf()
    } else {
        root.join(path)
    };
    let normalized = if candidate.exists() {
        candidate.canonicalize().ok()?
    } else {
        candidate
    };
    let relative = normalized.strip_prefix(root).ok()?;
    url_key(relative)
}

fn content_encoding_for_path(path: &str) -> Option<&'static str> {
    if path.ends_with(".wasm.br") {
        Some("br")
    } else {
        None
    }
}

fn content_type_for_path(path: &str) -> &'static str {
    if path.ends_with(".wasm.br") {
        return "application/wasm";
    }
    match Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("")
    {
        "html" | "htm" => "text/html",
        "css" => "text/css",
        "js" => "application/javascript",
        "wasm" => "application/wasm",
        "mec" | "🤖" => "text/x-mech",
        "json" => "application/json",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "svg" => "image/svg+xml",
        "webp" => "image/webp",
        "csv" => "text/csv",
        "md" => "text/markdown",
        _ => "application/octet-stream",
    }
}

fn is_workspace_target_source(path: &Path) -> bool {
    SourceKind::from_path(path).is_executable_mech()
}

fn is_renderable_mech_text_source(path: &Path) -> bool {
    matches!(SourceKind::from_path(path), SourceKind::Mech)
}

fn is_allowed_static_file(path: &Path) -> bool {
    matches!(
        path.extension().and_then(|ext| ext.to_str()),
        Some("html")
            | Some("htm")
            | Some("css")
            | Some("js")
            | Some("wasm")
            | Some("br")
            | Some("png")
            | Some("jpg")
            | Some("jpeg")
            | Some("gif")
            | Some("svg")
            | Some("webp")
            | Some("md")
            | Some("csv")
            | Some("json")
    )
}

fn target_name(specifier: &str) -> String {
    let path = Path::new(specifier).with_extension("");
    let name = path.to_string_lossy().replace(['/', '\\', '.', ' '], "-");
    if name.is_empty() {
        "main".to_string()
    } else {
        name
    }
}

fn module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("serve", "v0.3", "native", &[], &[])
}

fn asset(
    bytes: &[u8],
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    backing_paths: Vec<PathBuf>,
) -> ServerAsset {
    ServerAsset {
        bytes: bytes.to_vec(),
        content_type,
        content_encoding,
        backing_paths: dedupe_paths(backing_paths),
    }
}

fn authorize_server_asset(
    kernel: &mech_runtime::SharedCapabilityKernel,
    subject: &str,
    asset: &ServerAsset,
) -> MResult<()> {
    for path in &asset.backing_paths {
        check_fs_capability(&mut kernel.clone(), subject, FS_SERVE, path)?;
    }
    Ok(())
}

fn response(
    bytes: Vec<u8>,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    status: warp::http::StatusCode,
) -> warp::reply::Response {
    let mut response = warp::http::Response::builder()
        .status(status)
        .header("content-type", content_type)
        .header("cache-control", "no-store");
    if let Some(content_encoding) = content_encoding {
        response = response.header("content-encoding", content_encoding);
    }
    response.body(bytes.into()).unwrap()
}

fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[derive(Debug, Clone)]
pub struct ServerNotInitializedError;
impl MechErrorKind for ServerNotInitializedError {
    fn name(&self) -> &str {
        "ServerNotInitializedError"
    }
    fn message(&self) -> String {
        "The server is not initialized.".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct Utf8ConversionError {
    pub source_error: String,
}
impl MechErrorKind for Utf8ConversionError {
    fn name(&self) -> &str {
        "Utf8ConversionError"
    }
    fn message(&self) -> String {
        format!(
            "Failed to convert bytes into UTF-8 string: {}",
            self.source_error
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    static CURRENT_DIR_LOCK: Mutex<()> = Mutex::new(());

    struct CurrentDirGuard {
        previous: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        fn enter(path: &Path) -> Self {
            let lock = CURRENT_DIR_LOCK.lock().unwrap();
            let previous = std::env::current_dir().unwrap();
            std::env::set_current_dir(path).unwrap();
            Self {
                previous,
                _lock: lock,
            }
        }
    }

    impl Drop for CurrentDirGuard {
        fn drop(&mut self) {
            std::env::set_current_dir(&self.previous).unwrap();
        }
    }

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-serve-{}-{}",
            name,
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
        ));
        std::fs::create_dir_all(&root).unwrap();
        root.canonicalize().unwrap()
    }

    fn snapshot(root: &Path, file: &str) -> RuntimeWorkspaceSnapshot {
        snapshot_for_sources(root, &[file])
    }

    fn snapshot_for_sources(root: &Path, files: &[&str]) -> RuntimeWorkspaceSnapshot {
        ServerWorkspaceSession::open(
            root,
            files
                .iter()
                .map(|file| RuntimeWorkspaceTarget {
                    name: target_name(file),
                    specifier: (*file).to_string(),
                })
                .collect(),
            vec![],
            module_options(),
        )
        .unwrap()
        .snapshot()
        .unwrap()
        .clone()
    }

    fn synced_registry(root: &Path, file: &str) -> ServerSourceRegistry {
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(root, &snapshot(root, file), "", "", &[])
            .unwrap();
        registry
    }

    fn test_server() -> MechServer {
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(
                &mut ids,
                &std::env::current_dir().unwrap(),
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
        MechServer::new(
            "test".to_string(),
            "127.0.0.1:0".to_string(),
            "style".to_string(),
            "shim".to_string(),
            vec![1, 2, 3],
            vec![4, 5, 6],
            authority,
        )
    }

    fn empty_host_config() -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_host_browser::BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        }
    }

    #[test]
    fn plan_serve_inputs_accepts_explicit_mecb_target() {
        let root = temp_root("explicit-mecb-target");
        std::fs::write(root.join("main.mecb"), b"bytecode").unwrap();
        let guard = CurrentDirGuard::enter(&root);

        let plan = plan_serve_inputs(&vec!["main.mecb".to_string()]).unwrap();

        assert_eq!(plan.targets.len(), 1);
        assert_eq!(plan.static_paths.len(), 0);
        assert_eq!(plan.targets[0].specifier, "main.mecb");

        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_keeps_mecb_out_of_static_paths() {
        let root = temp_root("mecb-not-static");
        std::fs::write(root.join("main.mecb"), b"bytecode").unwrap();
        let guard = CurrentDirGuard::enter(&root);

        let plan = plan_serve_inputs(&vec!["main.mecb".to_string()]).unwrap();

        assert!(plan.static_paths.is_empty());
        assert_eq!(
            plan.targets
                .iter()
                .map(|target| target.specifier.as_str())
                .collect::<Vec<_>>(),
            vec!["main.mecb"]
        );

        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn renderable_source_predicate_excludes_mecb() {
        assert!(is_workspace_target_source(Path::new("main.mecb")));
        assert!(!is_renderable_mech_text_source(Path::new("main.mecb")));
        assert!(is_renderable_mech_text_source(Path::new("main.mec")));
    }

    #[test]
    fn serve_until_shutdown_rejects_uninitialized_server() {
        let server = test_server();
        let (_shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        let result = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.serve_until_shutdown(shutdown_rx));

        assert!(result.is_err());
    }

    #[test]
    fn serve_until_shutdown_exits_when_shutdown_signal_changes() {
        let mut server = test_server();
        server.init = true;
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(async move {
                let server_future = server.serve_until_shutdown(shutdown_rx);
                let shutdown_future = async move {
                    tokio::time::sleep(std::time::Duration::from_millis(50)).await;
                    let _ = shutdown_tx.send(true);
                };

                tokio::time::timeout(std::time::Duration::from_secs(2), async move {
                    let (result, _) = tokio::join!(server_future, shutdown_future);
                    result
                })
                .await
                .expect("server did not shut down")
                .unwrap();
            });
    }

    #[test]
    fn server_init_does_not_mutate_html_shim_with_host_config() {
        let mut server = test_server();
        server.html_shim = "<html><head></head><body></body></html>".to_string();
        server.host_config = Some(empty_host_config());

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();

        assert!(!server.html_shim.contains("__MECH_HOST_CONFIG"));

        let registry = server.registry.read().unwrap();
        let html = String::from_utf8(registry.get_route("index.html").unwrap().bytes).unwrap();
        assert_eq!(html.matches("window.__MECH_HOST_CONFIG =").count(), 1);
    }

    #[test]
    fn generated_mech_html_uses_injected_host_config_shim() {
        let root = temp_root("generated-host-config-shim");
        std::fs::write(root.join("index.mec"), "x := 1\n").unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        server.html_shim = "<html><head></head><body></body></html>".to_string();
        server.host_config = Some(empty_host_config());

        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["index.mec".to_string()])
            .unwrap();

        let registry = server.registry.read().unwrap();
        let html = String::from_utf8(registry.get_route("/").unwrap().bytes).unwrap();

        assert!(html.contains("window.__MECH_HOST_CONFIG ="));
        assert_eq!(html.matches("window.__MECH_HOST_CONFIG =").count(), 1);

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn wasm_host_config_constructor_export_is_declared() {
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm/src/lib.rs");
        let source = std::fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_path.display()));
        assert!(source.contains("#[wasm_bindgen(js_name = \"fromHostConfig\")]"));
        assert!(source.contains("pub fn from_host_config() -> Result<WasmMech, JsValue>"));
        assert!(source.contains("JsValue::from_str(\"__MECH_HOST_CONFIG\")"));
    }

    #[test]
    fn config_shim_at_root_prefers_custom_shim_over_listing() {
        let root = temp_root("config-shim-root");
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        server.html_shim = "<html><head></head><body>custom shim</body></html>".to_string();
        server.serve_configured_shim_at_root = true;
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_workspace(&Vec::new()).unwrap();
        let registry = server.registry.read().unwrap();
        let (asset, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("user asset `index.html`"));
        assert!(
            String::from_utf8(asset.bytes)
                .unwrap()
                .contains("custom shim")
        );
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_directory_input_does_not_serve_mcfg_files() {
        let root = temp_root("dir-skips-mcfg");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("demo.mcfg"), "runtime: {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_workspace(&vec![".".to_string()]).unwrap();
        let registry = server.registry.read().unwrap();
        assert!(registry.get_route("demo.mcfg").is_none());
        assert!(registry.get_route("source/demo.mcfg").is_none());
        assert!(registry.get_route("code/demo.mcfg").is_none());
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn static_directory_serve_path_skips_mcfg_assets() {
        let root = temp_root("static-skips-mcfg");
        std::fs::write(root.join("index.html"), "ok").unwrap();
        std::fs::write(root.join("demo.mcfg"), "runtime: {}\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        load_static_assets_from_paths(&mut registry, &root, &[".".to_string()]).unwrap();
        assert!(registry.get_route("index.html").is_some());
        assert!(registry.get_route("demo.mcfg").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn explicit_mcfg_serve_input_is_skipped() {
        let root = temp_root("explicit-mcfg-skipped");
        std::fs::write(root.join("demo.mcfg"), "runtime: {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&vec!["demo.mcfg".to_string()]).unwrap();
        assert!(plan.targets.is_empty());
        assert!(plan.static_paths.is_empty());
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_distinguishes_source_text_and_encoded_compiled_code_routes() {
        let root = temp_root("html-code-routes");
        let source_text = "x := 1\n";
        std::fs::write(root.join("main.mec"), source_text).unwrap();
        let registry = synced_registry(&root, "main.mec");
        let html = registry.get_route("main.mec").unwrap();
        let source = registry.get_route("source/main.mec").unwrap();
        let code = registry.get_route("code/main.mec").unwrap();

        assert_eq!(html.content_type, "text/html");
        assert_eq!(source.content_type, "text/x-mech");
        assert_eq!(String::from_utf8(source.bytes).unwrap(), source_text);
        assert_eq!(code.content_type, "text/plain");

        let encoded = String::from_utf8(code.bytes).unwrap();
        assert_ne!(encoded, source_text);
        assert!(!encoded.contains("x := 1"));
        let decoded: Program = decode_and_decompress(&encoded).unwrap();
        assert_eq!(decoded, parser::parse(source_text).unwrap());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn restricted_authority_blocks_workspace_escape() {
        let root = temp_root("restricted");
        let allowed = root.join("allowed");
        let outside = root.join("outside");
        std::fs::create_dir_all(&allowed).unwrap();
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("secret.mec"), "x := 1\n").unwrap();
        let _guard = CurrentDirGuard::enter(&root);
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(
                &mut ids,
                &allowed,
                true,
                [FS_READ, FS_LIST, FS_WATCH, FS_RESOLVE, FS_IMPORT, FS_SERVE],
            )
            .unwrap();
        let mut server = MechServer::new(
            "test".into(),
            "127.0.0.1:0".into(),
            "".into(),
            "".into(),
            vec![],
            vec![],
            authority,
        );
        server.init = true;
        assert!(
            server
                .load_workspace(&vec!["outside/secret.mec".to_string()])
                .is_err()
        );
    }

    #[test]
    fn user_backed_asset_requires_serve_capability() {
        let root = temp_root("serve-denied");
        let file = root.join("index.html");
        std::fs::write(&file, "secret").unwrap();
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(&mut ids, &root, true, [FS_READ])
            .unwrap();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &root, true, [FS_READ])
            .unwrap();
        let asset = ServerAsset {
            bytes: b"secret".to_vec(),
            content_type: "text/html",
            content_encoding: None,
            backing_paths: vec![file],
        };
        assert!(authorize_server_asset(&authority.kernel(), SERVE_HOST_SUBJECT, &asset).is_err());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn normalize_url_path_rejects_traversal() {
        assert_eq!(normalize_url_path("../secret"), None);
        assert_eq!(normalize_url_path("/foo/../bar"), None);
        assert_eq!(normalize_url_path("/foo//bar"), None);
        assert_eq!(normalize_url_path("/C:/secret"), None);
        assert_eq!(normalize_url_path("/"), Some("index.html".to_string()));
    }

    #[test]
    fn content_type_for_path_maps_common_assets() {
        assert_eq!(content_type_for_path("index.html"), "text/html");
        assert_eq!(content_type_for_path("style.css"), "text/css");
        assert_eq!(content_type_for_path("app.js"), "application/javascript");
        assert_eq!(content_type_for_path("app.wasm"), "application/wasm");
        assert_eq!(content_type_for_path("main.mec"), "text/x-mech");
        assert_eq!(content_type_for_path("image.png"), "image/png");
        assert_eq!(content_type_for_path("image.svg"), "image/svg+xml");
    }

    #[test]
    fn registry_index_prefers_generated_source_over_bundled_index() {
        let root = temp_root("index-generated");
        std::fs::write(root.join("index.mec"), "x := 1\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_asset(
            "index.html",
            asset(b"bundled", "text/html", None, Vec::new()),
        );
        registry.insert_asset(
            "_mech/index.html",
            asset(b"bundled", "text/html", None, Vec::new()),
        );
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "index.mec"), "", "", &[])
            .unwrap();
        let served = registry.get_route("/").unwrap();
        assert_ne!(served.bytes, b"bundled");
        assert_eq!(served.content_type, "text/html");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_index_prefers_user_index_over_generated_source() {
        let root = temp_root("index-user");
        std::fs::write(root.join("index.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("index.html"), "user index").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_asset(
            "_mech/index.html",
            asset(b"bundled", "text/html", None, Vec::new()),
        );
        registry
            .insert_static_file(&root, &root.join("index.html"))
            .unwrap();
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "index.mec"), "", "", &[])
            .unwrap();
        let served = registry.get_route("/").unwrap();
        assert_eq!(served.bytes, b"user index");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_updates_existing_asset() {
        let root = temp_root("static-update");
        let css = root.join("style.css");
        std::fs::write(&css, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &css).unwrap();
        assert_eq!(registry.get_route("style.css").unwrap().bytes, b"old");
        std::fs::write(&css, "new").unwrap();
        assert!(registry.reload_static_path(&root, &css).unwrap());
        assert_eq!(registry.get_route("style.css").unwrap().bytes, b"new");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_removes_deleted_asset() {
        let root = temp_root("static-remove");
        let css = root.join("style.css");
        std::fs::write(&css, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &css).unwrap();
        assert!(registry.get_route("style.css").is_some());
        std::fs::remove_file(&css).unwrap();
        assert!(registry.reload_static_path(&root, &css).unwrap());
        assert!(registry.get_route("style.css").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_ignores_mech_source() {
        let root = temp_root("static-ignore-mech");
        let source = root.join("main.mec");
        std::fs::write(&source, "x := 1\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        assert!(!registry.reload_static_path(&root, &source).unwrap());
        assert!(registry.get_route("main.mec").is_none());
        assert!(registry.get_route("source/main.mec").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn sync_static_assets_from_watch_events_reloads_static_asset() {
        let root = temp_root("static-watch-event");
        let css = root.join("style.css");
        std::fs::write(&css, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &css).unwrap();
        std::fs::write(&css, "new").unwrap();
        let events = vec![RuntimeWorkspaceWatchEvent {
            path: css,
            kind: mech_runtime::RuntimeWorkspaceWatchEventKind::Modified,
        }];
        assert!(sync_static_assets_from_watch_events(&mut registry, &root, &events).unwrap());
        assert_eq!(registry.get_route("style.css").unwrap().bytes, b"new");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_static_wasm_br_sets_brotli_encoding() {
        let root = temp_root("wasm-br");
        let wasm = root.join("app.wasm.br");
        std::fs::write(&wasm, b"wasm").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &wasm).unwrap();
        let asset = registry.get_route("app.wasm.br").unwrap();
        assert_eq!(asset.content_type, "application/wasm");
        assert_eq!(asset.content_encoding, Some("br"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_serves_mec_as_generated_html() {
        let root = temp_root("html");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let registry = synced_registry(&root, "main.mec");
        assert_eq!(
            registry.get_route("main.mec").unwrap().content_type,
            "text/html"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_serves_raw_source_under_source_prefix() {
        let root = temp_root("raw");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let registry = synced_registry(&root, "main.mec");
        assert!(
            String::from_utf8(registry.get_route("source/main.mec").unwrap().bytes)
                .unwrap()
                .contains("x := 1")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_html_path_falls_back_to_mec_html() {
        let root = temp_root("fallback");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let registry = synced_registry(&root, "main.mec");
        assert_eq!(
            registry.get_route("main.html").unwrap().content_type,
            "text/html"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_exact_html_asset_wins_over_mec_fallback() {
        let root = temp_root("exact");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let mut registry = synced_registry(&root, "main.mec");
        registry.insert_asset(
            "main.html",
            asset(b"explicit", "text/html", None, Vec::new()),
        );
        assert_eq!(registry.get_route("main.html").unwrap().bytes, b"explicit");
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_removes_stale_workspace_sources_on_resync() {
        let root = temp_root("stale");
        std::fs::write(root.join("a.mec"), "a := 1\n").unwrap();
        std::fs::write(root.join("b.mec"), "b := 2\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "a.mec"), "", "", &[])
            .unwrap();
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "b.mec"), "", "", &[])
            .unwrap();
        assert!(registry.get_route("source/a.mec").is_none());
        assert!(registry.get_route("source/b.mec").is_some());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_registers_workspace_source() {
        let root = temp_root("load");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["main.mec".to_string()])
            .unwrap();
        assert!(
            server
                .registry
                .read()
                .unwrap()
                .get_route("main.mec")
                .is_some()
        );
        assert!(
            server
                .registry
                .read()
                .unwrap()
                .get_route("source/main.mec")
                .is_some()
        );
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_with_explicit_target_does_not_load_unrelated_mec() {
        let root = temp_root("explicit-no-discovery");
        std::fs::write(root.join("test2.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["test2.mec".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        assert!(registry.get_route("test2.mec").is_some());
        assert!(registry.get_route("source/test2.mec").is_some());
        assert!(registry.get_route("ROADMAP.mec").is_none());
        assert!(registry.get_route("source/ROADMAP.mec").is_none());
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_index_route_prefers_explicit_target() {
        let root = temp_root("explicit-index");
        std::fs::write(root.join("test2.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["test2.mec".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        let (_, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("test2.mec"));
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_without_explicit_target_discovers_mec_files() {
        let root = temp_root("discovery-enabled");
        std::fs::write(root.join("a.mec"), "a := 1\n").unwrap();
        std::fs::write(root.join("b.mec"), "b := 2\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_workspace(&Vec::new()).unwrap();
        let registry = server.registry.read().unwrap();
        assert!(registry.get_route("a.mec").is_some());
        assert!(registry.get_route("b.mec").is_some());
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_preserves_missing_mech_target_diagnostic() {
        let root = temp_root("missing-target");
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["missing.mec".to_string()])
            .unwrap();
        let session = server.workspace_session.as_ref().unwrap();
        let session = session.lock().unwrap();
        let snapshot = session.snapshot().unwrap();
        assert!(!snapshot.diagnostics.is_empty());
        assert!(
            snapshot
                .diagnostics
                .iter()
                .any(|diagnostic| { diagnostic.target.as_deref() == Some("missing") })
        );
        drop(session);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn poll_workspace_once_updates_registry_after_manual_refresh() {
        let root = temp_root("refresh");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        server.html_shim = "<html><head></head><body></body></html>".to_string();
        server.host_config = Some(empty_host_config());
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["main.mec".to_string()])
            .unwrap();
        std::fs::write(root.join("main.mec"), "x := 2\n").unwrap();
        let session = server.workspace_session.as_ref().unwrap();
        let mut session = session.lock().unwrap();
        session.refresh(module_options()).unwrap();
        let html_shim = server.injected_html_shim().unwrap();
        server
            .registry
            .write()
            .unwrap()
            .sync_workspace_snapshot(
                &root,
                session.snapshot().unwrap(),
                &server.stylesheet,
                &html_shim,
                &server.generated_html_backing_paths(),
            )
            .unwrap();
        drop(session);
        let registry = server.registry.read().unwrap();
        let raw = registry.get_route("source/main.mec").unwrap();
        assert!(String::from_utf8(raw.bytes).unwrap().contains("x := 2"));
        let html = String::from_utf8(registry.get_route("main.mec").unwrap().bytes).unwrap();
        assert!(html.contains("window.__MECH_HOST_CONFIG ="));
        assert_eq!(html.matches("window.__MECH_HOST_CONFIG =").count(), 1);
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_single_directory_uses_directory_as_root() {
        let root = temp_root("serve-dir-root");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&vec!["examples/working".to_string()]).unwrap();
        assert_eq!(plan.root, dir.canonicalize().unwrap());
        assert!(plan.targets.is_empty());
        assert_eq!(
            plan.folders,
            vec![RuntimeWorkspaceFolder {
                specifier: ".".to_string(),
                recursive: true
            }]
        );
        assert!(plan.static_paths.iter().any(|path| path == "."));
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_directory_input_does_not_load_sibling_mec_files() {
        let root = temp_root("serve-dir-no-siblings");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["examples/working".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        assert!(registry.get_route("fizzbuzz.mec").is_some());
        assert!(registry.get_route("source/fizzbuzz.mec").is_some());
        assert!(registry.get_route("ROADMAP.mec").is_none());
        assert!(registry.get_route("source/ROADMAP.mec").is_none());
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_single_file_uses_parent_as_root() {
        let root = temp_root("serve-file-root");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&vec!["examples/working/fizzbuzz.mec".to_string()]).unwrap();
        assert_eq!(plan.root, dir.canonicalize().unwrap());
        assert_eq!(
            plan.targets,
            vec![RuntimeWorkspaceTarget {
                name: "fizzbuzz".to_string(),
                specifier: "fizzbuzz.mec".to_string()
            }]
        );
        assert!(plan.folders.is_empty());
        assert_eq!(plan.preferred_index_source.as_deref(), Some("fizzbuzz.mec"));
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_no_inputs_discovers_current_dir() {
        let root = temp_root("serve-no-inputs");
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&Vec::new()).unwrap();
        assert_eq!(plan.root, root.canonicalize().unwrap());
        assert_eq!(
            plan.folders,
            vec![RuntimeWorkspaceFolder {
                specifier: ".".to_string(),
                recursive: true
            }]
        );
        assert!(plan.targets.is_empty());
        assert!(plan.static_paths.is_empty());
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_mixed_inputs_use_common_root_without_root_discovery() {
        let root = temp_root("serve-mixed-root");
        let working = root.join("examples").join("working");
        let docs = root.join("docs").join("design");
        std::fs::create_dir_all(&working).unwrap();
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(working.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        std::fs::write(docs.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&vec![
            "examples/working".to_string(),
            "docs/design/ROADMAP.mec".to_string(),
        ])
        .unwrap();
        assert_eq!(plan.root, root.canonicalize().unwrap());
        assert_eq!(
            plan.folders,
            vec![RuntimeWorkspaceFolder {
                specifier: "examples/working".to_string(),
                recursive: true
            }]
        );
        assert_eq!(
            plan.targets,
            vec![RuntimeWorkspaceTarget {
                name: "docs-design-ROADMAP".to_string(),
                specifier: "docs/design/ROADMAP.mec".to_string()
            }]
        );
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_directory_index_serves_generated_html_at_root() {
        let root = temp_root("serve-dir-index");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["examples/working".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        let (_, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("index.mec"));
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_load_workspace_directory_loads_static_assets_relative_to_directory() {
        let root = temp_root("serve-dir-static");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("style.css"), "body {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["examples/working".to_string()])
            .unwrap();
        assert!(
            server
                .registry
                .read()
                .unwrap()
                .get_route("style.css")
                .is_some()
        );
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_mech_and_static_file_share_parent_root() {
        let root = temp_root("serve-file-static-root");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        std::fs::write(dir.join("style.css"), "body {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_serve_inputs(&vec![
            "examples/working/fizzbuzz.mec".to_string(),
            "examples/working/style.css".to_string(),
        ])
        .unwrap();
        assert_eq!(plan.root, dir.canonicalize().unwrap());
        assert_eq!(
            plan.targets,
            vec![RuntimeWorkspaceTarget {
                name: "fizzbuzz".to_string(),
                specifier: "fizzbuzz.mec".to_string()
            }]
        );
        assert!(plan.folders.is_empty());
        assert_eq!(plan.static_paths, vec!["style.css".to_string()]);
        assert_eq!(plan.preferred_index_source.as_deref(), Some("fizzbuzz.mec"));
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_root_serves_listing_for_multiple_sources_without_index() {
        let root = temp_root("listing-multiple");
        std::fs::write(root.join("bubble-sort.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("fizzbuzz.mec"), "y := 2\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot_for_sources(&root, &["bubble-sort.mec", "fizzbuzz.mec"]),
                "",
                "",
                &[],
            )
            .unwrap();
        let (asset, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("listing"));
        let html = String::from_utf8(asset.bytes).unwrap();
        assert!(html.contains("bubble-sort.mec"));
        assert!(html.contains("fizzbuzz.mec"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_root_serves_index_mec_when_present() {
        let root = temp_root("listing-index");
        std::fs::write(root.join("index.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("other.mec"), "y := 2\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot_for_sources(&root, &["index.mec", "other.mec"]),
                "",
                "",
                &[],
            )
            .unwrap();
        assert!(
            registry
                .get_route_with_trace("/")
                .unwrap()
                .1
                .contains("index.mec")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_root_serves_single_source_when_only_one_exists() {
        let root = temp_root("listing-single");
        std::fs::write(root.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "fizzbuzz.mec"), "", "", &[])
            .unwrap();
        assert!(
            registry
                .get_route_with_trace("/")
                .unwrap()
                .1
                .contains("fizzbuzz.mec")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_code_root_alias_requires_effective_index_source() {
        let root = temp_root("code-root-alias");
        std::fs::write(root.join("a.mec"), "a := 1\n").unwrap();
        std::fs::write(root.join("b.mec"), "b := 2\n").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot_for_sources(&root, &["a.mec", "b.mec"]),
                "",
                "",
                &[],
            )
            .unwrap();
        assert!(registry.get_route_with_trace("/code/").is_none());
        registry.set_preferred_index_source("a.mec");
        assert!(
            registry
                .get_route_with_trace("/code/")
                .unwrap()
                .1
                .contains("a.mec")
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn planned_directory_delegation_is_deduplicated() {
        let root = temp_root("delegation-dedupe");
        let plan = ServeInputPlan {
            root: root.clone(),
            targets: vec![],
            folders: vec![RuntimeWorkspaceFolder {
                specifier: ".".into(),
                recursive: true,
            }],
            static_paths: vec![".".into()],
            preferred_index_source: None,
        };
        let delegations = planned_delegations(&plan);
        assert_eq!(delegations.len(), 1);
        assert_eq!(delegations.values().next().unwrap().len(), 6);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn display_fs_resource_is_normalized() {
        let root = temp_root("display-fs-resource");
        let resource = display_fs_resource(&root);
        assert!(resource.starts_with("fs://"));
        assert!(!resource.contains(r"\\?\"));
        assert!(!resource.contains('\\'));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_init_requires_fs_serve_for_local_resource() {
        let root = temp_root("init-serve-denied");
        let shim = root.join("shim.html");
        std::fs::write(&shim, "<html></html>").unwrap();
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(&mut ids, &shim, false, [FS_READ])
            .unwrap();
        let mut server = MechServer::new(
            "test".into(),
            "127.0.0.1:0".into(),
            "".into(),
            "".into(),
            vec![],
            vec![],
            authority,
        );
        server.set_resource_backing_paths(vec![shim.clone()], Vec::new(), Vec::new(), Vec::new());
        let err = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .expect_err("init must fail without FS_SERVE");
        assert!(err.full_chain_message().contains("Capability"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_init_accepts_authorized_local_resource() {
        let root = temp_root("init-serve-allowed");
        let shim = root.join("shim.html");
        std::fs::write(&shim, "<html></html>").unwrap();
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(&mut ids, &shim, false, [FS_READ, FS_SERVE])
            .unwrap();
        let mut server = MechServer::new(
            "test".into(),
            "127.0.0.1:0".into(),
            "".into(),
            "".into(),
            vec![],
            vec![],
            authority,
        );
        server.set_resource_backing_paths(vec![shim.clone()], Vec::new(), Vec::new(), Vec::new());
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_asset_requires_all_backing_paths() {
        let root = temp_root("asset-all-paths");
        let first = root.join("one.html");
        let second = root.join("two.html");
        std::fs::write(&first, "one").unwrap();
        std::fs::write(&second, "two").unwrap();
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(&mut ids, &first, false, [FS_SERVE])
            .unwrap();
        authority
            .grant_path(&mut ids, &second, false, [FS_SERVE])
            .unwrap();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &first, false, [FS_SERVE])
            .unwrap();
        let asset = ServerAsset {
            bytes: vec![],
            content_type: "text/html",
            content_encoding: None,
            backing_paths: vec![first.clone(), second.clone()],
        };
        assert!(authorize_server_asset(&authority.kernel(), SERVE_HOST_SUBJECT, &asset).is_err());
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &second, false, [FS_SERVE])
            .unwrap();
        assert!(authorize_server_asset(&authority.kernel(), SERVE_HOST_SUBJECT, &asset).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generated_source_html_tracks_shim_and_stylesheet_paths() {
        let root = temp_root("generated-backing");
        let source = root.join("main.mec");
        let shim = root.join("shim.html");
        let css = root.join("style.css");
        std::fs::write(&source, "x := 1\n").unwrap();
        std::fs::write(&shim, "shim").unwrap();
        std::fs::write(&css, "css").unwrap();
        let generated = vec![shim.clone(), css.clone(), shim.clone()];
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(&root, &snapshot(&root, "main.mec"), "", "", &generated)
            .unwrap();
        let asset = registry.html_sources.get("main.mec").unwrap();
        assert_eq!(asset.backing_paths, vec![source, shim, css]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
