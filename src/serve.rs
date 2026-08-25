use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::future::Future;
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::pin::Pin;
use std::sync::{
    Arc, Mutex, RwLock,
    atomic::{AtomicBool, Ordering},
};
use std::time::{Duration, Instant};

use colored::*;
use ignore::WalkBuilder;
use mech_browser::BrowserRuntimeInjectionConfig;
use mech_core::*;
use mech_runtime::{
    DefaultIdGenerator, EventId, EventSink, FS_IMPORT, FS_LIST, FS_READ, FS_RESOLVE, FS_SERVE,
    FS_WATCH, HostFilesystemAuthority, ModuleBuildOptions, RuntimeConfig, RuntimeEvent,
    RuntimeWorkspaceFolder, RuntimeWorkspaceSnapshot, RuntimeWorkspaceTarget,
    RuntimeWorkspaceWatchEvent, SERVE_HOST_SUBJECT, ServerWorkspaceSession, SourceKind,
    SourceResolutionEntry, check_fs_capability, validate_source_resolution_entries,
};
use mech_syntax::{
    formatter::{Formatter, HtmlShimExtraSlots, HtmlStyleSheets, validate_shipped_shim_render},
    parser,
};
use warp::Filter;

use crate::*;

const SERVER_SHUTDOWN_GRACE: Duration = Duration::from_secs(2);

#[derive(Clone, Debug)]
struct ServerAsset {
    bytes: Vec<u8>,
    content_type: &'static str,
    content_encoding: Option<&'static str>,
    backing_paths: Vec<PathBuf>,
}

fn dedupe_paths(paths: Vec<PathBuf>) -> Vec<PathBuf> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for path in paths {
        if seen.insert(path.clone()) {
            deduped.push(path);
        }
    }
    deduped
}

#[derive(Clone, Copy, Debug)]
struct ServerRegistrySummary {
    assets: usize,
    raw_sources: usize,
    html_sources: usize,
    code_sources: usize,
    static_assets: usize,
}

#[derive(Clone, Debug, Default)]
struct ServerSourceRegistry {
    assets: HashMap<String, ServerAsset>,
    raw_sources: HashMap<String, ServerAsset>,
    html_sources: HashMap<String, ServerAsset>,
    code_sources: HashMap<String, ServerAsset>,
    source_specifiers: HashMap<String, String>,
    source_paths: HashMap<String, PathBuf>,
    source_roots: Vec<String>,
    source_resolutions: Vec<SourceResolutionEntry>,
    workspace_keys: HashSet<String>,
    static_asset_paths: HashMap<String, PathBuf>,
    user_assets: HashSet<String>,
    configured_root_asset: Option<ServerAsset>,
    index_source: Option<String>,
    preferred_index_source: Option<String>,
    listing_asset: Option<ServerAsset>,
    document_controller: Option<String>,
    shipped_document_shim: Option<String>,
    document_presentation: mech_runtime::ServePresentation,
    capability_kernel: Option<mech_runtime::SharedCapabilityKernel>,
    capability_subject: Option<String>,
}

impl ServerSourceRegistry {
    fn set_document_controller(
        &mut self,
        document_controller: Option<String>,
        shipped_document_shim: Option<String>,
    ) {
        self.document_controller = document_controller;
        self.shipped_document_shim = shipped_document_shim;
    }

    fn set_document_presentation(&mut self, presentation: mech_runtime::ServePresentation) {
        self.document_presentation = presentation;
    }

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

    fn insert_generated_asset(&mut self, key: impl Into<String>, asset: ServerAsset) {
        let key = key.into();
        self.user_assets.remove(&key);
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

    #[cfg(test)]
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

    /// Resolves a generated document alias without assuming that every renderable
    /// source uses the historical `.mec` extension. Keep `.mec` first when both
    /// source spellings exist so existing aliases retain their established target.
    fn generated_html_alias(&self, stem: &Path, trace: &str) -> Option<(ServerAsset, String)> {
        let mec_key = stem.with_extension("mec").to_string_lossy().into_owned();
        if let Some(asset) = self.html_sources.get(&mec_key) {
            return Some((asset.clone(), format!("{} `{}`", trace, mec_key)));
        }

        let mut candidates = self
            .html_sources
            .iter()
            .filter(|(key, _)| Path::new(key).with_extension("").as_path() == stem)
            .collect::<Vec<_>>();
        candidates.sort_by(|(left, _), (right, _)| left.cmp(right));
        let (key, asset) = candidates.first()?;
        Some(((*asset).clone(), format!("{} `{}`", trace, key)))
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
            let escaped_url = escape_html(&key);
            let logical = self
                .source_specifiers
                .get(&key)
                .map(String::as_str)
                .unwrap_or(&key);
            let escaped_label = escape_html(logical);
            html.push_str(&format!("    <li><a href=\"/{escaped_url}\">{escaped_label}</a> <a href=\"/source/{escaped_url}\">source</a>"));
            if self.code_sources.contains_key(&key) {
                html.push_str(&format!(" <a href=\"/code/{escaped_url}\">code</a>"));
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
            if let Some(asset) = &self.configured_root_asset {
                return Some((asset.clone(), "configured root shim".to_string()));
            }
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
        if let Some(asset) = self.html_sources.get(&normalized) {
            return Some((asset.clone(), format!("generated html `{}`", normalized)));
        }
        if let Some(asset) = self.assets.get(&normalized) {
            return Some((asset.clone(), format!("asset `{}`", normalized)));
        }
        if normalized.ends_with(".html") || normalized.ends_with(".htm") {
            let stem = Path::new(&normalized).with_extension("");
            return self.generated_html_alias(&stem, "generated html fallback");
        }
        if !normalized.starts_with("_mech/")
            && !normalized.starts_with("source/")
            && !normalized.starts_with("code/")
        {
            return self
                .generated_html_alias(Path::new(&normalized), "generated extensionless html");
        }
        None
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
        let Some(key) = transport_url_key(relative) else {
            return Err(Error::new(ErrorKind::InvalidInput, "invalid static asset path").into());
        };
        let path_text = path.to_string_lossy();
        self.insert_user_asset(
            key.clone(),
            ServerAsset {
                bytes: std::fs::read(&path)?,
                content_type: content_type_for_path(path_text.as_ref()),
                content_encoding: content_encoding_for_path(path_text.as_ref()),
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
        stylesheets: impl Into<HtmlStyleSheets>,
        shim: &str,
        generated_html_backing_paths: &[PathBuf],
    ) -> MResult<()> {
        let stylesheets = stylesheets.into();
        let root = root.canonicalize()?;
        for key in self.workspace_keys.drain() {
            self.raw_sources.remove(&key);
            self.html_sources.remove(&key);
            self.code_sources.remove(&key);
            self.source_specifiers.remove(&key);
            self.source_paths.remove(&key);
        }
        self.index_source = None;
        self.source_roots.clear();
        self.source_resolutions.clear();
        let mut module_specifiers = BTreeMap::new();

        for source in snapshot.sources.values() {
            let Some(path) = source.path.as_ref() else {
                continue;
            };
            if !is_renderable_mech_text_source(path) {
                continue;
            }
            let path = path.canonicalize()?;
            let relative = path.strip_prefix(&root).map_err(|error| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("workspace source is outside workspace root: {}", error),
                )
            })?;
            let Some(logical_specifier) = url_key(relative) else {
                continue;
            };
            let key = percent_encode_url_path(&logical_specifier);
            self.check(FS_READ, &path)?;
            let source_text =
                match source.source.as_ref() {
                    Some(MechSourceCode::String(source)) => source.clone(),
                    Some(other) => {
                        return Err(MechError::new(
                            GenericError {
                                msg: format!(
                                    "workspace source `{}` is not resolved Mech text: {:?}",
                                    path.display(),
                                    other,
                                ),
                            },
                            None,
                        )
                        .with_compiler_loc());
                    }
                    None => return Err(MechError::new(
                        GenericError {
                            msg: format!(
                                "workspace source `{}` has no resolver-authoritative source text",
                                path.display(),
                            ),
                        },
                        None,
                    )
                    .with_compiler_loc()),
                };
            self.raw_sources.insert(
                key.clone(),
                ServerAsset {
                    bytes: source_text.as_bytes().to_vec(),
                    content_type: "text/x-mech",
                    content_encoding: None,
                    backing_paths: vec![path.clone()],
                },
            );
            let fallback_tree = source
                .syntax_tree
                .is_none()
                .then(|| parser::parse(&source_text));
            let tree = match (source.syntax_tree.as_deref(), fallback_tree.as_ref()) {
                (Some(tree), _) => Ok(tree),
                (None, Some(tree)) => tree.as_ref(),
                (None, None) => unreachable!("missing parsed-tree fallback"),
            };
            match tree {
                Ok(tree) => {
                    let mut extra_slots = HtmlShimExtraSlots::default();
                    extra_slots.insert("SOURCE_URL_KEY", escape_html(&key));
                    extra_slots.insert(
                        "PRESENTATION",
                        self.document_presentation.as_str().to_string(),
                    );
                    if shim.contains("{{DOCUMENT_SCRIPT}}") {
                        let document_controller = self.document_controller.as_deref().ok_or_else(|| {
              MechError::new(
                GenericError {
                  msg: "selected HTML shim requests {{DOCUMENT_SCRIPT}}, but the embedded document controller is unavailable".to_string(),
                },
                None,
              )
              .with_compiler_loc()
            })?;
                        extra_slots.insert("DOCUMENT_SCRIPT", document_controller);
                        extra_slots.insert("WASM_MODULE_URL", "/_mech/pkg/mech_wasm.js");
                        // Served documents load their complete source map from the
                        // project manifest. Static formatter output supplies this slot
                        // with an embedded source bundle instead.
                        extra_slots.insert("DOCUMENT_SOURCES", "");
                    }
                    let mut formatter = Formatter::new();
                    let render = formatter.format_html_with_style_sheets_and_slots(
                        &tree,
                        stylesheets.clone(),
                        shim.to_string(),
                        &extra_slots,
                    );
                    if let Some(shim_name) = self.shipped_document_shim.as_deref() {
                        validate_shipped_shim_render(shim_name, &render)?;
                    }
                    let html = render.html;
                    let mut backing_paths = vec![path.clone()];
                    backing_paths.extend_from_slice(generated_html_backing_paths);
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
            self.source_specifiers
                .insert(key.clone(), logical_specifier.clone());
            if let Some(module_version) = source.module_version {
                module_specifiers.insert(module_version, logical_specifier);
            }
            self.workspace_keys.insert(key.clone());
            if is_index_source_key(&key) {
                self.index_source = Some(key);
            }
        }
        let mut resolution_set = BTreeSet::new();
        for edge in &snapshot.import_edges {
            let Some(referrer) = module_specifiers.get(&edge.importer) else {
                continue;
            };
            let target = module_specifiers.get(&edge.dependency).ok_or_else(|| {
        MechError::new(
          GenericError {
            msg: format!(
              "served source resolution `{}` from `{referrer}` targets a source outside the served manifest",
              edge.specifier,
            ),
          },
          None,
        ).with_compiler_loc()
      })?;
            resolution_set.insert(SourceResolutionEntry::new(
                referrer.clone(),
                edge.specifier.clone(),
                target.clone(),
            ));
        }
        let mut source_resolutions = resolution_set.into_iter().collect::<Vec<_>>();
        source_resolutions.sort();
        validate_source_resolution_entries(
            self.source_specifiers.values().map(String::as_str),
            &source_resolutions,
        )?;
        let mut source_roots = snapshot
            .targets
            .values()
            .filter_map(|target| module_specifiers.get(&target.module_version).cloned())
            .collect::<Vec<_>>();
        source_roots.sort();
        source_roots.dedup();
        self.source_roots = source_roots;
        self.source_resolutions = source_resolutions;
        self.sync_source_manifest()?;
        self.rebuild_listing();
        Ok(())
    }

    fn sync_source_manifest(&mut self) -> MResult<()> {
        let mut source_entries = self
            .source_specifiers
            .iter()
            .map(|(transport, specifier)| {
                serde_json::json!({
                  "specifier": specifier,
                  "url": format!("source/{transport}"),
                })
            })
            .collect::<Vec<_>>();
        source_entries.sort_by(|left, right| {
            left.get("specifier")
                .and_then(serde_json::Value::as_str)
                .cmp(&right.get("specifier").and_then(serde_json::Value::as_str))
        });
        let manifest = serde_json::to_vec(&serde_json::json!({
          "version": 2,
          "roots": self.source_roots,
          "sources": source_entries,
          "resolutions": self.source_resolutions,
        }))
        .map_err(|error| {
            Error::new(
                ErrorKind::InvalidData,
                format!("failed to serialize project source manifest: {error}"),
            )
        })?;
        self.insert_generated_asset(
            "_mech/project-sources.json",
            asset(
                &manifest,
                "application/json",
                None,
                self.source_paths.values().cloned().collect(),
            ),
        );
        Ok(())
    }

    fn sync_project_overlay(&mut self, project: &ConfiguredProjectOverlay) -> MResult<()> {
        self.insert_generated_asset(
            "mech.mcfg",
            ServerAsset {
                bytes: project.config_source.as_bytes().to_vec(),
                content_type: "text/x-mech",
                content_encoding: None,
                backing_paths: vec![project.config_path.clone()],
            },
        );
        self.sync_source_manifest()
    }
}

fn is_index_source_key(key: &str) -> bool {
    Path::new(key).file_stem().and_then(|stem| stem.to_str()) == Some("index")
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
    stylesheets: HtmlStyleSheets,
    html_shim: String,
    project_html: String,
    project_js: String,
    host_config: Option<BrowserRuntimeInjectionConfig>,
    host_config_injection: Option<HostAuthorityInjection>,
    serve_configured_shim_at_root: bool,
    full_address: String,
    registry: Arc<RwLock<ServerSourceRegistry>>,
    events: Arc<RwLock<Vec<RuntimeEvent>>>,
    workspace_session: Option<Arc<Mutex<ServerWorkspaceSession>>>,
    workspace_changed: Arc<tokio::sync::Notify>,
    workspace_root: Option<PathBuf>,
    project_overlay: Option<ConfiguredProjectOverlay>,
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

struct ServerShutdown {
    requested: AtomicBool,
    sender: tokio::sync::watch::Sender<bool>,
}

impl ServerShutdown {
    fn new() -> (Self, tokio::sync::watch::Receiver<bool>) {
        let (sender, receiver) = tokio::sync::watch::channel(false);
        (
            Self {
                requested: AtomicBool::new(false),
                sender,
            },
            receiver,
        )
    }

    fn request(&self) -> bool {
        if self
            .requested
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return false;
        }
        self.sender.send(true).is_ok()
    }
}

async fn graceful_server_timed_out<F>(server: Pin<&mut F>, grace: Duration) -> bool
where
    F: Future<Output = ()>,
{
    tokio::time::timeout(grace, server).await.is_err()
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
            concat!(
                include_str!("../include/browser-compute.js"),
                "\n",
                include_str!("../include/project.js")
            )
            .to_string(),
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
        project_js: String,
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
            project_js,
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
        stylesheets: impl Into<HtmlStyleSheets>,
        html_shim: String,
        project_js: String,
        wasm: Vec<u8>,
        js: Vec<u8>,
        authority: HostFilesystemAuthority,
        runtime_config: RuntimeConfig,
        host_config: Option<BrowserRuntimeInjectionConfig>,
        host_config_injection: Option<HostAuthorityInjection>,
        serve_configured_shim_at_root: bool,
    ) -> Self {
        Self::new_with_project_html_and_host_config(
            name,
            full_address,
            stylesheets,
            html_shim,
            include_str!("../include/project.html").to_string(),
            project_js,
            wasm,
            js,
            authority,
            runtime_config,
            host_config,
            host_config_injection,
            serve_configured_shim_at_root,
        )
    }

    pub(crate) fn new_with_project_html_and_host_config(
        name: String,
        full_address: String,
        stylesheets: impl Into<HtmlStyleSheets>,
        html_shim: String,
        project_html: String,
        project_js: String,
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
            stylesheets: stylesheets.into(),
            html_shim,
            project_html,
            project_js,
            host_config,
            host_config_injection,
            serve_configured_shim_at_root,
            full_address,
            registry: Arc::new(RwLock::new(ServerSourceRegistry::default())),
            events: Arc::new(RwLock::new(Vec::new())),
            workspace_session: None,
            workspace_changed: Arc::new(tokio::sync::Notify::new()),
            workspace_root: None,
            project_overlay: None,
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

    /// Installs the optional controller requested by a source-document shim.
    ///
    /// Custom shims that omit `{{DOCUMENT_SCRIPT}}` receive no injected runtime
    /// code. The shipped-shim name is only used to enforce the maintained shell
    /// slot contract during generated-page rendering.
    pub fn set_document_controller(
        &mut self,
        document_controller: Option<String>,
        shipped_document_shim: Option<String>,
    ) {
        self.registry
            .write()
            .unwrap()
            .set_document_controller(document_controller, shipped_document_shim);
    }

    /// Selects the initial presentation of generated source documents.
    pub fn set_document_presentation(&mut self, presentation: mech_runtime::ServePresentation) {
        self.registry
            .write()
            .unwrap()
            .set_document_presentation(presentation);
    }

    fn generated_html_backing_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.html_shim_backing_paths.clone();
        paths.extend(self.stylesheet_backing_paths.clone());
        dedupe_paths(paths)
    }

    fn configured_resource_backing_paths(&self) -> Vec<PathBuf> {
        let mut paths = self.html_shim_backing_paths.clone();
        paths.extend(self.stylesheet_backing_paths.clone());
        paths.extend(self.wasm_backing_paths.clone());
        paths.extend(self.js_backing_paths.clone());
        dedupe_paths(paths)
    }

    fn inject_authority_into_html(&self, html: &str) -> MResult<String> {
        if let Some(injection) = &self.host_config_injection {
            inject_host_authority_injection_script(html, injection)
        } else if let Some(host_config) = &self.host_config {
            inject_browser_host_config_script(html, host_config)
        } else {
            Ok(html.to_string())
        }
    }

    fn injected_html_shim(&self) -> MResult<String> {
        self.inject_authority_into_html(&self.html_shim)
    }

    pub async fn init(&mut self) -> MResult<()> {
        if self.js.is_empty() {
            return Err(MechError::new(
                GenericError {
                    msg: "browser JavaScript wrapper is missing".to_string(),
                },
                None,
            )
            .with_compiler_loc());
        }
        if !self.wasm.starts_with(b"\0asm") {
            return Err(MechError::new(
                GenericError {
                    msg:
                        "browser WASM asset is not a raw WebAssembly module (missing \\0asm magic)"
                            .to_string(),
                },
                None,
            )
            .with_compiler_loc());
        }
        let configured_paths = self.configured_resource_backing_paths();
        let mut ids = DefaultIdGenerator::new();
        for path in configured_paths {
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
        let stylesheet_bundle = self.stylesheets.bundle();
        let css = asset(
            stylesheet_bundle.as_bytes(),
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
        let project_js = asset(
            self.project_js.as_bytes(),
            "application/javascript",
            None,
            Vec::new(),
        );
        let project_html = asset(
            self.inject_authority_into_html(&self.project_html)?
                .as_bytes(),
            "text/html",
            None,
            Vec::new(),
        );
        let wasm = asset(
            &self.wasm,
            "application/wasm",
            None,
            self.wasm_backing_paths.clone(),
        );
        if self.serve_configured_shim_at_root {
            registry.configured_root_asset = Some(html.clone());
        }
        registry.insert_asset("_mech/index.html", html);
        registry.insert_asset("_mech/project.html", project_html);
        registry.insert_asset("_mech/style.css", css);
        if !self.project_js.is_empty() {
            registry.insert_asset("_mech/project.js", project_js);
        }
        registry.insert_asset("_mech/pkg/mech_wasm.js", js.clone());
        registry.insert_asset("_mech/pkg/mech_wasm_bg.wasm", wasm.clone());
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
        let plan = plan_serve_inputs(paths)?;
        self.load_serve_plan(plan, None)
    }

    pub(crate) fn load_serve_plan(
        &mut self,
        plan: ServeInputPlan,
        project: Option<ConfiguredProjectOverlay>,
    ) -> MResult<()> {
        if !self.init {
            return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
        }
        if let Some(project) = &project {
            if plan.root != project.root {
                return Err(Error::new(
                    ErrorKind::InvalidInput,
                    format!(
                        "configured project root `{}` does not match workspace root `{}`",
                        project.root.display(),
                        plan.root.display(),
                    ),
                )
                .into());
            }
        }
        let started = Instant::now();
        self.delegate_plan(&plan)?;
        self.registry
            .write()
            .unwrap()
            .with_capabilities(self.authority.kernel().clone(), self.serve_subject.clone());
        let root = plan.root.clone();
        self.workspace_root = Some(root.clone());
        self.project_overlay = project.clone();
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
        log_skipped_serve_inputs(&plan.inputs)?;
        if plan.inputs.is_empty() {
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
        let mut session =
            ServerWorkspaceSession::open_with_capabilities_config_and_function_catalog(
                &root,
                plan.targets,
                plan.folders,
                module_options(),
                self.authority.kernel().clone(),
                self.serve_subject.clone(),
                self.runtime_config.clone(),
                mech_stdlib::source_catalog(),
            )?;
        let workspace_changed = self.workspace_changed.clone();
        session.set_watch_event_notifier(move || workspace_changed.notify_one());
        // Drain any events queued while the workspace and notifier were being set up.
        self.workspace_changed.notify_one();
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
            registry.set_preferred_index_source(percent_encode_url_path(&source));
        }
        drop(registry);
        println!("{} Emitting initial workspace events…", self.badge());
        let mut sink = ServerEventSink {
            events: self.events.clone(),
            max_events: server_event_retention(&self.runtime_config),
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
            let generated_html_backing_paths = self.generated_html_backing_paths();
            self.registry.write().unwrap().sync_workspace_snapshot(
                &root,
                snapshot,
                &self.stylesheets,
                &html_shim,
                &generated_html_backing_paths,
            )?;
        }
        if let Some(project) = &project {
            self.registry
                .write()
                .unwrap()
                .sync_project_overlay(project)?;
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

        let (shutdown, shutdown_rx) = ServerShutdown::new();
        let shutdown = Arc::new(shutdown);
        let signal_shutdown = shutdown.clone();
        let server_name = self.name.clone();

        ctrlc::set_handler(move || {
            let badge = if server_name.ends_with(" Server") {
                format!("[{}]", server_name).truecolor(34, 204, 187)
            } else {
                format!("[{}] Server", server_name).truecolor(34, 204, 187)
            };

            if signal_shutdown.request() {
                println!("{} Server received shutdown signal.", badge);
            }
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
        let project_overlay = self.project_overlay.clone();
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
        let socket_address: SocketAddr = self.full_address.parse().map_err(|error| {
            MechError::new(
                GenericError {
                    msg: format!("invalid server address `{}`: {}", self.full_address, error),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        let mut server_shutdown_rx = shutdown_rx.clone();
        let (_addr, server) =
            warp::serve(routes).bind_with_graceful_shutdown(socket_address, async move {
                if !*server_shutdown_rx.borrow() {
                    drop(server_shutdown_rx.changed().await);
                }
            });
        if let (Some(session), Some(root)) = (&self.workspace_session, &root) {
            let html_shim = self.injected_html_shim()?;
            let generated_html_backing_paths = self.generated_html_backing_paths();
            tokio::pin!(server);
            let requested = loop {
                tokio::select! {
                  _ = self.workspace_changed.notified() => {
                    if let Err(error) = poll_workspace_once(session, &self.registry, &self.events, &root, &self.stylesheets, &html_shim, &generated_html_backing_paths, project_overlay.as_ref(), server_event_retention(&self.runtime_config)) {
                      eprintln!("[Mech Server] Workspace poll failed: {:?}", error);
                    }
                  }
                  _ = shutdown_rx.changed() => {
                    break true;
                  }
                  _ = &mut server => break false,
                }
            };
            if requested && graceful_server_timed_out(server.as_mut(), SERVER_SHUTDOWN_GRACE).await
            {
                eprintln!(
                    "{} Graceful shutdown timed out; forcing server close.",
                    self.badge(),
                );
            }
        } else {
            tokio::pin!(server);
            let requested = tokio::select! {
              _ = &mut server => false,
              _ = shutdown_rx.changed() => true,
            };
            if requested && graceful_server_timed_out(server.as_mut(), SERVER_SHUTDOWN_GRACE).await
            {
                eprintln!(
                    "{} Graceful shutdown timed out; forcing server close.",
                    self.badge(),
                );
            }
        }
        println!("{} Closing server.", self.badge());
        Ok(())
    }
}

#[derive(Debug)]
struct ServerEventSink {
    events: Arc<RwLock<Vec<RuntimeEvent>>>,
    max_events: Option<usize>,
}

impl EventSink for ServerEventSink {
    fn emit(&mut self, event: RuntimeEvent) -> MResult<EventId> {
        let id = event.id;
        let mut events = self.events.write().unwrap();
        events.push(event);
        if let Some(max_events) = self.max_events {
            if events.len() > max_events {
                let remove_count = events.len() - max_events;
                events.drain(0..remove_count);
            }
        }
        Ok(id)
    }
}

fn server_event_retention(config: &RuntimeConfig) -> Option<usize> {
    config
        .limits
        .max_in_memory_events
        .map(|value| usize::try_from(value).unwrap_or(usize::MAX))
}

fn poll_workspace_once(
    session: &Arc<Mutex<ServerWorkspaceSession>>,
    registry: &Arc<RwLock<ServerSourceRegistry>>,
    events: &Arc<RwLock<Vec<RuntimeEvent>>>,
    root: &Path,
    stylesheets: &HtmlStyleSheets,
    shim: &str,
    generated_html_backing_paths: &[PathBuf],
    project_overlay: Option<&ConfiguredProjectOverlay>,
    max_events: Option<usize>,
) -> MResult<()> {
    let mut session = session.lock().unwrap();
    let mut sink = ServerEventSink {
        events: events.clone(),
        max_events,
    };
    let poll = session.poll_and_emit(module_options(), &mut sink)?;
    if poll.events.is_empty() && poll.refresh.is_none() {
        return Ok(());
    }
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
        let mut candidate = registry.read().unwrap().clone();
        if sync_static_assets_from_watch_events(&mut candidate, root, &poll.events)? {
            println!("[Mech Server] Static assets updated from watch events.");
        }
        if poll.refresh.is_some() {
            if let Some(snapshot) = session.snapshot() {
                candidate.sync_workspace_snapshot(
                    root,
                    snapshot,
                    stylesheets,
                    shim,
                    generated_html_backing_paths,
                )?;
            }
        }
        if let Some(project) = project_overlay {
            candidate.sync_project_overlay(project)?;
        }
        *registry.write().unwrap() = candidate;
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

#[derive(Debug, Clone)]
pub(crate) struct ServeInputPlan {
    pub(crate) root: PathBuf,
    pub(crate) targets: Vec<RuntimeWorkspaceTarget>,
    pub(crate) folders: Vec<RuntimeWorkspaceFolder>,
    pub(crate) static_paths: Vec<String>,
    pub(crate) preferred_index_source: Option<String>,
    pub(crate) inputs: Vec<String>,
}

#[derive(Debug, Clone)]
pub(crate) struct ConfiguredProjectOverlay {
    pub(crate) root: PathBuf,
    pub(crate) config_path: PathBuf,
    pub(crate) config_source: String,
}

fn plan_serve_inputs(paths: &[String]) -> MResult<ServeInputPlan> {
    plan_serve_inputs_with_root(paths, None)
}

pub(crate) fn plan_cli_serve_inputs(
    paths: &[String],
    project_root: Option<&Path>,
) -> MResult<ServeInputPlan> {
    plan_serve_inputs_with_root(paths, project_root)
}

fn plan_serve_inputs_with_root(
    paths: &[String],
    fixed_root: Option<&Path>,
) -> MResult<ServeInputPlan> {
    // Resolve user-relative selectors before canonicalization. On Windows,
    // `canonicalize` returns a verbatim `\\?\` path; joining a subsequently
    // supplied relative selector onto that namespace can make an existing
    // directory fail `exists()` and leave the common-root planner empty.
    let current_dir = std::env::current_dir()?;
    if paths.is_empty() {
        let root = fixed_root
            .map(Path::to_path_buf)
            .unwrap_or_else(|| current_dir.clone())
            .canonicalize()?;
        return Ok(ServeInputPlan {
            root,
            targets: Vec::new(),
            folders: vec![RuntimeWorkspaceFolder {
                specifier: ".".to_string(),
                recursive: true,
            }],
            static_paths: fixed_root
                .map(|_| vec![".".to_string()])
                .unwrap_or_default(),
            preferred_index_source: None,
            inputs: Vec::new(),
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

    let root = if let Some(root) = fixed_root {
        root.canonicalize()?
    } else {
        common_ancestor(&root_paths).ok_or_else(|| {
            Error::new(
                ErrorKind::InvalidInput,
                "serve inputs do not have a common workspace root",
            )
        })?
    };
    let mut targets = Vec::new();
    let mut folders = Vec::new();
    let mut static_paths = Vec::new();
    let mut preferred_index_source = None;
    let mut target_keys = HashSet::new();
    let mut folder_keys = HashSet::new();
    let mut static_keys = HashSet::new();
    if fixed_root.is_some() {
        static_paths.push(".".to_string());
        static_keys.insert(".".to_string());
    }
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
            if folder_keys.insert(specifier.clone()) {
                folders.push(RuntimeWorkspaceFolder {
                    specifier: specifier.clone(),
                    recursive: true,
                });
            }
            if static_keys.insert(specifier.clone()) {
                static_paths.push(specifier);
            }
        } else if is_workspace_target_source(&path) {
            let specifier = relative_specifier(&root, &path).ok_or_else(|| {
                Error::new(
                    ErrorKind::InvalidInput,
                    format!("Mech target `{}` is outside workspace root", path.display()),
                )
            })?;
            if is_renderable_mech_text_source(&path) {
                preferred_index_source.get_or_insert_with(|| specifier.clone());
            }
            if target_keys.insert(specifier.clone()) {
                targets.push(RuntimeWorkspaceTarget {
                    name: target_name(&specifier),
                    specifier,
                });
            }
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
            if static_keys.insert(specifier.clone()) {
                static_paths.push(specifier);
            }
        }
    }
    Ok(ServeInputPlan {
        root,
        targets,
        folders,
        static_paths,
        preferred_index_source,
        inputs: paths.to_vec(),
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
    // Keep this resolution identical to `plan_serve_inputs_with_root`: append
    // relative user input before Windows introduces a verbatim path prefix.
    let current_dir = std::env::current_dir()?;
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
    if path.starts_with('/') {
        return None;
    }
    let mut decoded = Vec::with_capacity(path.len());
    let bytes = path.as_bytes();
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let high = *bytes.get(index + 1)?;
            let low = *bytes.get(index + 2)?;
            decoded.push((percent_decode_hex_digit(high)? << 4) | percent_decode_hex_digit(low)?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    let decoded = String::from_utf8(decoded).ok()?;
    if decoded.contains('\\') {
        return None;
    }
    let segments: Vec<&str> = decoded.split('/').collect();
    if segments
        .iter()
        .any(|segment| segment.is_empty() || *segment == ".." || segment.contains(':'))
    {
        return None;
    }
    Some(percent_encode_url_path(&segments.join("/")))
}

fn url_key(path: &Path) -> Option<String> {
    let mut segments = Vec::new();
    for component in path.components() {
        match component {
            Component::Normal(segment) => segments.push(segment.to_string_lossy().to_string()),
            _ => return None,
        }
    }
    if segments.is_empty()
        || segments.iter().any(|segment| {
            segment.is_empty() || segment == ".." || segment.contains(':') || segment.contains('\\')
        })
    {
        return None;
    }
    Some(segments.join("/"))
}

fn transport_url_key(path: &Path) -> Option<String> {
    url_key(path).map(|key| percent_encode_url_path(&key))
}

fn is_url_path_byte(byte: u8) -> bool {
    byte.is_ascii_alphanumeric()
        || matches!(
            byte,
            b'-' | b'.'
                | b'_'
                | b'~'
                | b'/'
                | b'!'
                | b'$'
                | b'&'
                | b'\''
                | b'('
                | b')'
                | b'*'
                | b'+'
                | b','
                | b';'
                | b'='
                | b'@'
        )
}

fn percent_encode_url_path(path: &str) -> String {
    let mut encoded = String::with_capacity(path.len());
    for &byte in path.as_bytes() {
        if is_url_path_byte(byte) {
            encoded.push(byte as char);
        } else {
            encoded.push('%');
            encoded.push(percent_encode_hex_digit(byte >> 4));
            encoded.push(percent_encode_hex_digit(byte & 0x0f));
        }
    }
    encoded
}

fn percent_encode_hex_digit(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'A' + value - 10) as char,
        _ => unreachable!(),
    }
}

fn percent_decode_hex_digit(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        b'A'..=b'F' => Some(value - b'A' + 10),
        _ => None,
    }
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
    transport_url_key(relative)
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
    use mech_runtime::MECH_TOOL_SUBJECT;
    use std::time::{SystemTime, UNIX_EPOCH};

    struct CurrentDirGuard {
        previous: PathBuf,
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl CurrentDirGuard {
        fn enter(path: &Path) -> Self {
            let lock = crate::cli::lock_current_dir();
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

    #[test]
    fn server_shutdown_request_is_idempotent() {
        let (shutdown, mut receiver) = ServerShutdown::new();
        assert!(shutdown.request());
        assert!(!shutdown.request());
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            receiver.changed().await.unwrap();
        });
        assert!(*receiver.borrow());
    }

    #[test]
    fn immediately_completed_server_exits_without_timeout() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let server = async {};
            tokio::pin!(server);
            assert!(!graceful_server_timed_out(server.as_mut(), Duration::from_millis(25),).await);
        });
    }

    #[test]
    fn permanently_pending_server_is_cut_off_by_timeout() {
        tokio::runtime::Runtime::new().unwrap().block_on(async {
            let server = std::future::pending::<()>();
            tokio::pin!(server);
            assert!(graceful_server_timed_out(server.as_mut(), Duration::from_millis(10),).await);
        });
    }

    #[test]
    fn configured_serve_plan_preserves_declared_target_and_folder_order() {
        let root = temp_root("source-inventory");
        std::fs::create_dir_all(root.join("app")).unwrap();
        std::fs::write(root.join("main.mec"), "main := 1\n").unwrap();
        std::fs::write(root.join("app/lib.mec"), "lib := 1\n").unwrap();
        let plan = plan_cli_serve_inputs(
            &[
                root.join("main.mec").to_string_lossy().to_string(),
                root.join("app").to_string_lossy().to_string(),
                root.join("main.mec").to_string_lossy().to_string(),
            ],
            Some(&root),
        )
        .unwrap();

        assert_eq!(
            plan.targets,
            vec![RuntimeWorkspaceTarget {
                name: "main".to_string(),
                specifier: "main.mec".to_string(),
            }],
        );
        assert_eq!(
            plan.folders,
            vec![RuntimeWorkspaceFolder {
                specifier: "app".to_string(),
                recursive: true,
            }],
        );
        assert_eq!(plan.preferred_index_source.as_deref(), Some("main.mec"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn percent_encode_url_path_encodes_reserved_and_utf8_bytes() {
        for (path, expected) in [
            ("assets/app+debug.js", "assets/app+debug.js"),
            ("assets/theme@2.css", "assets/theme@2.css"),
            ("assets/alert!.svg", "assets/alert!.svg"),
            ("assets/data;.json", "assets/data;.json"),
            ("assets/a&b=c.js", "assets/a&b=c.js"),
            ("app/a#b.mec", "app/a%23b.mec"),
            ("app/a?b.mec", "app/a%3Fb.mec"),
            ("app/100%.mec", "app/100%25.mec"),
            ("app/my file.mec", "app/my%20file.mec"),
            ("app/café.mec", "app/caf%C3%A9.mec"),
            ("app/a\\b.mec", "app/a%5Cb.mec"),
            ("app/a:b.mec", "app/a%3Ab.mec"),
        ] {
            assert_eq!(percent_encode_url_path(path), expected);
        }
    }

    #[test]
    fn project_manifest_uses_raw_source_routes() {
        let root = temp_root("source-manifest");
        let app = root.join("app");
        let mech_dir = root.join("_mech");
        std::fs::create_dir_all(&app).unwrap();
        std::fs::create_dir_all(&mech_dir).unwrap();

        let clock_path = app.join("clock.mec");
        let support_path = app.join("support.mec");
        let escaped_path = app.join("a#b.mec");
        std::fs::write(&clock_path, "clock := 1\n").unwrap();
        std::fs::write(&support_path, "support := 1\n").unwrap();
        std::fs::write(&escaped_path, "escaped := 1\n").unwrap();
        std::fs::write(
            mech_dir.join("project-sources.json"),
            r#"{"version":999,"sources":[]}"#,
        )
        .unwrap();
        std::fs::write(root.join("index.html"), "<!doctype html>\n").unwrap();
        std::fs::write(
            root.join("mech.mcfg"),
            r#"config := {
  hosts: []

  serve: {
    paths: ["app"]
  }

  run: {
    paths: ["app/clock.mec"]
    grants: []
  }
}
"#,
        )
        .unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let plan = plan_cli_serve_inputs(
            &[
                clock_path.to_string_lossy().to_string(),
                app.to_string_lossy().to_string(),
            ],
            Some(&root),
        )
        .unwrap();
        let project = ConfiguredProjectOverlay {
            root: root.clone(),
            config_path: root.join("mech.mcfg").canonicalize().unwrap(),
            config_source: std::fs::read_to_string(root.join("mech.mcfg")).unwrap(),
        };
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_serve_plan(plan, Some(project)).unwrap();

        let registry = server.registry.read().unwrap();
        assert!(server.workspace_session.is_some());
        assert!(!registry.user_assets.contains("_mech/project-sources.json"));
        let manifest_asset = registry
            .get_route("/_mech/project-sources.json")
            .expect("generated project source manifest route should exist");
        assert_eq!(manifest_asset.content_type, "application/json");
        assert_ne!(
            manifest_asset.bytes,
            br#"{"version":999,"sources":[]}"#.to_vec(),
        );
        let escaped_source_asset = registry
            .get_route("/app/a%23b.mec")
            .expect("encoded source route should exist");
        assert_eq!(escaped_source_asset.content_type, "text/html");
        assert_eq!(
            registry
                .get_route("/source/app/a%23b.mec")
                .expect("encoded raw source route should exist")
                .bytes,
            b"escaped := 1\n",
        );

        let manifest: serde_json::Value = serde_json::from_slice(&manifest_asset.bytes).unwrap();
        assert_eq!(
            manifest.get("version").and_then(serde_json::Value::as_u64),
            Some(2)
        );
        let sources = manifest
            .get("sources")
            .and_then(serde_json::Value::as_array)
            .expect("generated project source manifest should contain sources");
        assert_eq!(sources.len(), 3);
        let source_pairs = sources
            .iter()
            .map(|source| {
                let specifier = source
                    .get("specifier")
                    .and_then(serde_json::Value::as_str)
                    .expect("manifest source specifier should be a string")
                    .to_string();
                let url = source
                    .get("url")
                    .and_then(serde_json::Value::as_str)
                    .expect("manifest source URL should be a string")
                    .to_string();
                (specifier, url)
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            source_pairs,
            BTreeSet::from([
                (
                    "app/a#b.mec".to_string(),
                    "source/app/a%23b.mec".to_string()
                ),
                (
                    "app/clock.mec".to_string(),
                    "source/app/clock.mec".to_string()
                ),
                (
                    "app/support.mec".to_string(),
                    "source/app/support.mec".to_string()
                ),
            ]),
        );
        assert!(sources.iter().all(|source| {
            source.get("specifier").and_then(serde_json::Value::as_str) != Some("app")
        }));
        assert!(sources.iter().all(|source| {
            source.get("url").and_then(serde_json::Value::as_str) != Some("app")
        }));
        assert_eq!(manifest_asset.backing_paths.len(), 3);
        assert_eq!(manifest["roots"], serde_json::json!(["app/clock.mec"]));
        assert_eq!(manifest["resolutions"], serde_json::json!([]));

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn default_shim_owns_standalone_document_execution() {
        let root = temp_root("default-document-shim");
        std::fs::write(root.join("main.mec"), "answer := 42\nanswer\n").unwrap();
        let snapshot = snapshot(&root, "main.mec");
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                include_str!("../include/index.html"),
                &[],
            )
            .unwrap();

        let html = String::from_utf8(registry.get_route("/main.mec").unwrap().bytes).unwrap();
        assert!(html.contains("WasmDocument"));
        assert!(html.contains("/_mech/pkg/mech_wasm.js"));
        assert!(html.contains("fetch(`/code/${sourceUrlKey}`)"));
        assert!(html.contains("data-mech-source-url-key=\"main.mec\""));
        assert!(html.contains("data-mech-presentation=\"document\""));
        assert!(!html.contains("{{CODE}}"));
        assert!(!html.contains("{{SOURCE_URL_KEY}}"));
        assert!(!html.contains("/_mech/project.js"));
        assert_eq!(
            registry.get_route("/code/main.mec").unwrap().content_type,
            "text/plain",
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configured_output_presentation_reaches_generated_documents() {
        let root = temp_root("output-document-presentation");
        std::fs::write(root.join("main.mec"), "answer := 42\nanswer\n").unwrap();
        let snapshot = snapshot(&root, "main.mec");
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry.set_document_presentation(mech_runtime::ServePresentation::Output);
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                include_str!("../include/index.html"),
                &[],
            )
            .unwrap();

        let html = String::from_utf8(registry.get_route("/main.mec").unwrap().bytes).unwrap();
        assert!(html.contains("data-mech-presentation=\"output\""));
        assert!(!html.contains("{{PRESENTATION}}"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn standalone_document_manifest_includes_relative_import_dependencies() {
        let root = temp_root("standalone-document-imports");
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::write(
            root.join("docs/main.mec"),
            "+> ./math.mec\nanswer := math/value + 1\nanswer\n",
        )
        .unwrap();
        std::fs::write(root.join("docs/math.mec"), "value := 41\n<+ value\n").unwrap();
        let snapshot = snapshot_for_sources(&root, &["docs/main.mec", "docs/math.mec"]);
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                include_str!("../include/index.html"),
                &[],
            )
            .unwrap();

        let html = String::from_utf8(registry.get_route("/docs/main.mec").unwrap().bytes).unwrap();
        assert!(html.contains("fromEncodedWithBundle"));
        let manifest: serde_json::Value = serde_json::from_slice(
            &registry
                .get_route("/_mech/project-sources.json")
                .expect("standalone documents need a source manifest for relative imports")
                .bytes,
        )
        .unwrap();
        let source_pairs = manifest["sources"]
            .as_array()
            .unwrap()
            .iter()
            .map(|source| {
                (
                    source["specifier"].as_str().unwrap(),
                    source["url"].as_str().unwrap(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert!(source_pairs.contains(&("docs/main.mec", "source/docs/main.mec")));
        assert!(source_pairs.contains(&("docs/math.mec", "source/docs/math.mec")));
        assert_eq!(manifest["version"], 2);
        assert!(
            manifest["roots"]
                .as_array()
                .unwrap()
                .iter()
                .any(|root| root.as_str() == Some("docs/main.mec"))
        );
        assert_eq!(
            manifest["resolutions"],
            serde_json::json!([{
              "referrer": "docs/main.mec",
              "specifier": "./math.mec",
              "target": "docs/math.mec",
            }]),
        );
        assert!(registry.get_route("/mech.mcfg").is_none());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn served_document_manifest_records_extension_and_index_resolution_edges() {
        let root = temp_root("served-resolution-fallbacks");
        std::fs::create_dir_all(root.join("package")).unwrap();
        std::fs::write(
            root.join("main.mec"),
            "+> ./dep\n+> ./package\nanswer := dep/value + package/value\n",
        )
        .unwrap();
        std::fs::write(root.join("dep.mec"), "value := 19\n<+ value\n").unwrap();
        std::fs::write(root.join("package/index.mec"), "value := 23\n<+ value\n").unwrap();

        let snapshot = snapshot(&root, "main.mec");
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                include_str!("../include/index.html"),
                &[],
            )
            .unwrap();

        let manifest: serde_json::Value = serde_json::from_slice(
            &registry
                .get_route("/_mech/project-sources.json")
                .unwrap()
                .bytes,
        )
        .unwrap();
        let resolutions = manifest["resolutions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|entry| {
                (
                    entry["referrer"].as_str().unwrap(),
                    entry["specifier"].as_str().unwrap(),
                    entry["target"].as_str().unwrap(),
                )
            })
            .collect::<BTreeSet<_>>();
        assert_eq!(
            resolutions,
            BTreeSet::from([
                ("main.mec", "./dep", "dep.mec"),
                ("main.mec", "./package", "package/index.mec"),
            ]),
        );
        assert_eq!(manifest["roots"], serde_json::json!(["main.mec"]));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn served_document_formats_resolver_expanded_nested_includes() {
        let root = temp_root("served-nested-includes");
        std::fs::write(
            root.join("main.mec"),
            "{child.mec}\nanswer := child-value + nested-value\nanswer\n",
        )
        .unwrap();
        std::fs::write(root.join("child.mec"), "{nested.mec}\nchild-value := 17\n").unwrap();
        std::fs::write(root.join("nested.mec"), "nested-value := 25\n").unwrap();

        let snapshot = snapshot(&root, "main.mec");
        let mut registry = ServerSourceRegistry::default();
        registry.set_document_controller(
            Some(include_str!("../include/document.js").to_string()),
            Some("include/index.html".to_string()),
        );
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                include_str!("../include/index.html"),
                &[],
            )
            .unwrap();

        let raw = String::from_utf8(registry.get_route("/source/main.mec").unwrap().bytes).unwrap();
        assert!(raw.contains("child-value := 17"));
        assert!(raw.contains("nested-value := 25"));
        assert!(!raw.contains("{child.mec}"));
        assert!(!raw.contains("{nested.mec}"));
        let html = String::from_utf8(registry.get_route("/main.mec").unwrap().bytes).unwrap();
        assert!(html.contains("child-value"));
        assert!(html.contains("nested-value"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn custom_inline_and_external_shims_remain_authoritative() {
        let root = temp_root("custom-document-shims");
        std::fs::write(root.join("main.mec"), "  answer := 42\n").unwrap();
        let snapshot = snapshot(&root, "main.mec");
        for (shim, expected) in [
            (
                "<html><body>{{CONTENT}}<script type=\"module\">window.inlineShimRan = true; const code = `{{CODE}}`;</script></body></html>",
                "window.inlineShimRan = true",
            ),
            (
                "<html><body>{{CONTENT}}<script type=\"module\" src=\"/custom-bootstrap.js\"></script></body></html>",
                "src=\"/custom-bootstrap.js\"",
            ),
        ] {
            let mut registry = ServerSourceRegistry::default();
            registry
                .sync_workspace_snapshot(&root, &snapshot, "", shim, &[])
                .unwrap();
            let html = String::from_utf8(registry.get_route("/main.mec").unwrap().bytes).unwrap();
            assert!(html.contains(expected));
            assert!(!html.contains("WasmDocument"));
            assert!(!html.contains("/_mech/project.js"));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn static_custom_shim_does_not_receive_browser_bootstrap() {
        let root = temp_root("static-document-shim");
        std::fs::write(
            root.join("main.mec"),
            "Static document\n===============\n\n~~~mech\n  answer := 42\n~~~\n",
        )
        .unwrap();
        let snapshot = snapshot(&root, "main.mec");
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot,
                "",
                "<html><body>{{CONTENT}}</body></html>",
                &[],
            )
            .unwrap();
        let html = String::from_utf8(registry.get_route("/main.mec").unwrap().bytes).unwrap();
        assert!(html.starts_with("<html><body>"), "{html}");
        assert!(!html.contains("{{CONTENT}}"), "{html}");
        assert!(!html.contains("<script"));
        assert!(!html.contains("mech_wasm"));
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configured_project_composes_formatted_and_browser_routes() {
        let root = temp_root("configured-composition");
        std::fs::create_dir_all(root.join("lib")).unwrap();
        std::fs::write(root.join("main.mec"), "answer := 42\n").unwrap();
        std::fs::write(root.join("lib/support.mec"), "support := 1\n").unwrap();
        std::fs::write(
      root.join("mech.mcfg"),
      "config := { hosts: [] serve: { paths: [\"lib\"] } run: { paths: [\"main.mec\"] grants: [] } }\n",
    )
    .unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        let plan = plan_cli_serve_inputs(&["main.mec".to_string(), "lib".to_string()], Some(&root))
            .unwrap();
        server
            .load_serve_plan(plan, Some(configured_project_overlay(&root)))
            .unwrap();
        let registry = server.registry.read().unwrap();

        assert!(server.workspace_session.is_some());
        for route in [
            "/main.mec",
            "/main.html",
            "/main",
            "/lib/support.mec",
            "/lib/support.html",
            "/lib/support",
        ] {
            assert_eq!(
                registry.get_route(route).unwrap().content_type,
                "text/html",
                "{route}"
            );
        }
        assert_eq!(
            registry
                .get_route("/source/lib/support.mec")
                .unwrap()
                .content_type,
            "text/x-mech",
        );
        assert_eq!(
            registry
                .get_route("/code/lib/support.mec")
                .unwrap()
                .content_type,
            "text/plain"
        );
        assert_eq!(
            registry.get_route("/mech.mcfg").unwrap().content_type,
            "text/x-mech"
        );
        assert!(registry.get_route("/_mech/project.html").is_some());
        assert!(registry.get_route("/_mech/project.js").is_some());
        assert!(registry.get_route("/_mech/project-sources.json").is_some());
        assert!(registry.get_route("/_mech/pkg/mech_wasm.js").is_some());
        let wasm = registry.get_route("/_mech/pkg/mech_wasm_bg.wasm").unwrap();
        assert_eq!(wasm.content_type, "application/wasm");
        assert!(wasm.bytes.starts_with(b"\0asm"));

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configured_project_without_user_index_uses_generated_source_root() {
        let root = temp_root("configured-generated-root");
        std::fs::write(root.join("main.mec"), "answer := 42\n").unwrap();
        std::fs::write(
            root.join("mech.mcfg"),
            "config := { hosts: [] run: { paths: [\"main.mec\"] grants: [] } }\n",
        )
        .unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        let plan = plan_cli_serve_inputs(&["main.mec".to_string()], Some(&root)).unwrap();
        server
            .load_serve_plan(plan, Some(configured_project_overlay(&root)))
            .unwrap();
        let registry = server.registry.read().unwrap();
        let (root_asset, trace) = registry.get_route_with_trace("/").unwrap();
        let main_asset = registry.get_route("/main.mec").unwrap();

        assert!(trace.contains("preferred generated html"));
        assert_eq!(root_asset.bytes, main_asset.bytes);
        assert_ne!(root_asset.bytes, b"answer := 42\n");

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn configured_project_user_index_wins_without_hiding_source_routes() {
        let root = temp_root("configured-user-index");
        std::fs::write(root.join("main.mec"), "answer := 42\n").unwrap();
        std::fs::write(
      root.join("index.html"),
      "<!doctype html><p>user root</p><script type=\"module\">window.configuredInline = true;</script>",
    )
    .unwrap();
        std::fs::write(
            root.join("mech.mcfg"),
            "config := { hosts: [] run: { paths: [\"main.mec\"] grants: [] } }\n",
        )
        .unwrap();

        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        let plan = plan_cli_serve_inputs(&["main.mec".to_string()], Some(&root)).unwrap();
        server
            .load_serve_plan(plan, Some(configured_project_overlay(&root)))
            .unwrap();
        let registry = server.registry.read().unwrap();
        let (root_asset, trace) = registry.get_route_with_trace("/").unwrap();

        assert_eq!(trace, "user asset `index.html`");
        let root_html = String::from_utf8(root_asset.bytes).unwrap();
        assert!(root_html.contains("user root"));
        assert!(root_html.contains("window.configuredInline = true"));
        assert!(!root_html.contains("/_mech/project.js"));
        assert_eq!(
            registry.get_route("/main.mec").unwrap().content_type,
            "text/html"
        );
        assert_eq!(
            registry.get_route("/main.html").unwrap().content_type,
            "text/html"
        );
        assert_eq!(
            registry.get_route("/main").unwrap().content_type,
            "text/html"
        );
        assert_eq!(
            registry.get_route("/source/main.mec").unwrap().content_type,
            "text/x-mech"
        );
        assert_eq!(
            registry.get_route("/code/main.mec").unwrap().content_type,
            "text/plain"
        );

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
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
            b"\0asm\x01\0\0\0".to_vec(),
            b"export default async function init() {}".to_vec(),
            authority,
        )
    }

    fn empty_host_config() -> BrowserRuntimeInjectionConfig {
        BrowserRuntimeInjectionConfig {
            runtime: mech_browser::BrowserHostRuntimeConfig::from(&RuntimeConfig::default()),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        }
    }

    fn configured_project_overlay(root: &Path) -> ConfiguredProjectOverlay {
        let config_path = root.join("mech.mcfg").canonicalize().unwrap();
        ConfiguredProjectOverlay {
            root: root.canonicalize().unwrap(),
            config_source: std::fs::read_to_string(&config_path).unwrap(),
            config_path,
        }
    }

    fn initialized_server() -> MechServer {
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
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
        assert_eq!(plan.preferred_index_source, None);

        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn plan_serve_inputs_prefers_first_renderable_source_after_bytecode() {
        let root = temp_root("renderable-after-bytecode");
        std::fs::write(root.join("bootstrap.mecb"), b"bytecode").unwrap();
        std::fs::write(root.join("main.mec"), "answer := 42\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);

        let plan =
            plan_serve_inputs(&["bootstrap.mecb".to_string(), "main.mec".to_string()]).unwrap();

        assert_eq!(
            plan.targets
                .iter()
                .map(|target| target.specifier.as_str())
                .collect::<Vec<_>>(),
            vec!["bootstrap.mecb", "main.mec"],
        );
        assert_eq!(plan.preferred_index_source.as_deref(), Some("main.mec"));

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
                    shutdown_tx
                        .send(true)
                        .expect("test server shutdown receiver must remain live");
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
    fn wasm_project_export_is_declared() {
        let source_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/wasm/src/project.rs");
        let source = std::fs::read_to_string(&source_path)
            .unwrap_or_else(|error| panic!("failed to read {}: {error}", source_path.display()));
        assert!(source.contains("pub struct WasmProject"));
        assert!(source.contains("pub fn from_sources"));
        assert!(source.contains("pub fn required_paths"));
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
        assert_eq!(trace, "configured root shim");
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
    fn server_workspace_discovery_does_not_serve_mcfg_files() {
        let root = temp_root("dir-skips-mcfg");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("demo.mcfg"), "runtime: {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_workspace(&Vec::new()).unwrap();
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
    fn registry_static_assets_use_encoded_transport_keys() {
        let root = temp_root("static-encoded-keys");
        let assets = root.join("assets");
        std::fs::create_dir_all(&assets).unwrap();
        std::fs::write(assets.join("my logo.svg"), b"svg").unwrap();
        std::fs::write(assets.join("100%.json"), b"json").unwrap();
        std::fs::write(assets.join("café.md"), b"markdown").unwrap();
        std::fs::write(assets.join("app+debug.js"), b"plus").unwrap();
        std::fs::write(assets.join("theme@2.css"), b"at").unwrap();
        std::fs::write(assets.join("alert!.svg"), b"bang").unwrap();
        std::fs::write(assets.join("data;.json"), b"semicolon").unwrap();
        let mut registry = ServerSourceRegistry::default();

        load_static_assets_from_paths(&mut registry, &root, &["assets".to_string()]).unwrap();

        let svg = registry.get_route("/assets/my%20logo.svg").unwrap();
        assert_eq!(svg.bytes, b"svg");
        assert_eq!(svg.content_type, "image/svg+xml");
        let json = registry.get_route("/assets/100%25.json").unwrap();
        assert_eq!(json.bytes, b"json");
        assert_eq!(json.content_type, "application/json");
        let markdown = registry.get_route("/assets/caf%C3%A9.md").unwrap();
        assert_eq!(markdown.bytes, b"markdown");
        assert_eq!(markdown.content_type, "text/markdown");
        assert_eq!(
            registry.get_route("/assets/app+debug.js").unwrap().bytes,
            b"plus"
        );
        assert_eq!(
            registry.get_route("/assets/theme@2.css").unwrap().bytes,
            b"at"
        );
        assert_eq!(
            registry.get_route("/assets/alert!.svg").unwrap().bytes,
            b"bang"
        );
        assert_eq!(
            registry.get_route("/assets/data;.json").unwrap().bytes,
            b"semicolon"
        );

        let keys = registry.static_asset_keys();
        assert!(keys.contains(&"assets/my%20logo.svg".to_string()));
        assert!(keys.contains(&"assets/100%25.json".to_string()));
        assert!(keys.contains(&"assets/caf%C3%A9.md".to_string()));
        for key in [
            "assets/app+debug.js",
            "assets/theme@2.css",
            "assets/alert!.svg",
            "assets/data;.json",
        ] {
            assert!(keys.contains(&key.to_string()));
        }
        for key in [
            "assets/app%2Bdebug.js",
            "assets/theme%402.css",
            "assets/alert%21.svg",
            "assets/data%3B.json",
        ] {
            assert!(!keys.contains(&key.to_string()));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_updates_encoded_asset_key() {
        let root = temp_root("static-encoded-update");
        let css = root.join("theme dark.css");
        std::fs::write(&css, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &css).unwrap();
        assert_eq!(
            registry.get_route("theme%20dark.css").unwrap().bytes,
            b"old"
        );

        std::fs::write(&css, "new").unwrap();
        assert!(registry.reload_static_path(&root, &css).unwrap());
        assert_eq!(
            registry.get_route("theme%20dark.css").unwrap().bytes,
            b"new"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_updates_literal_url_path_character_key() {
        let root = temp_root("static-literal-update");
        let css = root.join("theme+dark.css");
        std::fs::write(&css, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &css).unwrap();
        assert_eq!(registry.get_route("theme+dark.css").unwrap().bytes, b"old");

        std::fs::write(&css, "new").unwrap();
        assert!(registry.reload_static_path(&root, &css).unwrap());
        assert_eq!(registry.get_route("theme+dark.css").unwrap().bytes, b"new");
        assert_eq!(
            registry.get_route("theme%2Bdark.css").unwrap().bytes,
            b"new"
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_removes_encoded_asset_key() {
        let root = temp_root("static-encoded-remove");
        let json = root.join("100%.json");
        std::fs::write(&json, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &json).unwrap();
        assert!(registry.get_route("100%25.json").is_some());

        std::fs::remove_file(&json).unwrap();
        assert!(registry.reload_static_path(&root, &json).unwrap());
        assert!(registry.get_route("100%25.json").is_none());
        for key in ["100%.json", "100%25.json"] {
            assert!(!registry.assets.contains_key(key));
            assert!(!registry.user_assets.contains(key));
            assert!(!registry.static_asset_paths.contains_key(key));
        }
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn registry_reload_static_path_removes_literal_url_path_character_key() {
        let root = temp_root("static-literal-remove");
        let json = root.join("data@2.json");
        std::fs::write(&json, "old").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry.insert_static_file(&root, &json).unwrap();
        assert!(registry.get_route("data@2.json").is_some());

        std::fs::remove_file(&json).unwrap();
        assert!(registry.reload_static_path(&root, &json).unwrap());
        assert!(registry.get_route("data@2.json").is_none());
        for key in ["data@2.json", "data%402.json"] {
            assert!(!registry.assets.contains_key(key));
            assert!(!registry.user_assets.contains(key));
            assert!(!registry.static_asset_paths.contains_key(key));
        }
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
    fn explicit_single_source_uses_workspace_formatter() {
        let root = temp_root("explicit-single-source");
        let raw_source = "answer := 42\n";
        std::fs::write(root.join("main.mec"), raw_source).unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server
            .load_workspace(&vec!["main.mec".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();

        assert!(server.workspace_session.is_some());
        for route in ["/", "/index.html", "/main.mec", "/main.html", "/main"] {
            assert_eq!(
                registry.get_route(route).unwrap().content_type,
                "text/html",
                "{route}"
            );
        }
        assert_eq!(
            registry.get_route("/source/main.mec").unwrap().content_type,
            "text/x-mech"
        );
        assert_eq!(
            registry.get_route("/code/main.mec").unwrap().content_type,
            "text/plain"
        );
        assert_ne!(
            registry.get_route("/main.mec").unwrap().bytes,
            raw_source.as_bytes()
        );
        assert!(registry.get_route("/mech.mcfg").is_none());

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn single_source_supports_html_and_extensionless_aliases() {
        let root = temp_root("single-source-aliases");
        std::fs::write(root.join("report.mec"), "value := 7\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server
            .load_workspace(&vec!["report.mec".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();

        let canonical = registry.get_route("/report.mec").unwrap();
        assert_eq!(
            registry.get_route("/report.html").unwrap().bytes,
            canonical.bytes
        );
        assert_eq!(
            registry.get_route("/report").unwrap().bytes,
            canonical.bytes
        );
        assert_eq!(registry.get_route("/").unwrap().bytes, canonical.bytes);

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn emoji_source_supports_html_and_extensionless_aliases() {
        let root = temp_root("emoji-source-aliases");
        std::fs::write(root.join("report.🤖"), "value := 7\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server
            .load_workspace(&vec!["report.🤖".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();

        let canonical = registry.get_route("/report.🤖").unwrap();
        assert_eq!(
            registry.get_route("/report.html").unwrap().bytes,
            canonical.bytes
        );
        assert_eq!(
            registry.get_route("/report").unwrap().bytes,
            canonical.bytes
        );
        assert_eq!(registry.get_route("/").unwrap().bytes, canonical.bytes);

        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_workspace_with_explicit_target_does_not_load_unrelated_mec() {
        let root = temp_root("explicit-no-discovery");
        std::fs::write(root.join("test2.mec"), "x := 1\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        std::fs::write(root.join("style.css"), "body {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec!["test2.mec".to_string(), "style.css".to_string()])
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
    fn server_index_route_prefers_first_explicit_workspace_target() {
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
            .load_workspace(&vec!["test2.mec".to_string(), "ROADMAP.mec".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        let (_, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("test2.mec"));
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn multiple_source_directory_builds_listing() {
        let root = temp_root("multiple-source-listing");
        std::fs::write(root.join("a.mec"), "a := 1\n").unwrap();
        std::fs::write(root.join("b.mec"), "b := 2\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server.load_workspace(&vec![".".to_string()]).unwrap();
        let registry = server.registry.read().unwrap();
        assert!(registry.get_route("a.mec").is_some());
        assert!(registry.get_route("b.mec").is_some());
        let (listing, trace) = registry.get_route_with_trace("/").unwrap();
        let listing = String::from_utf8(listing.bytes).unwrap();
        assert_eq!(trace, "generated source listing");
        assert!(listing.contains("/a.mec"));
        assert!(listing.contains("/b.mec"));
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
    fn configured_project_refresh_updates_formatted_source_and_manifest() {
        let root = temp_root("configured-refresh");
        std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
        std::fs::write(
      root.join("mech.mcfg"),
      "config := { hosts: [] serve: { paths: [\".\"] } run: { paths: [\"main.mec\"] grants: [] } }\n",
    )
    .unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server.html_shim = "<html><head></head><body></body></html>".to_string();
        server.host_config = Some(empty_host_config());
        let plan =
            plan_cli_serve_inputs(&["main.mec".to_string(), ".".to_string()], Some(&root)).unwrap();
        let project = configured_project_overlay(&root);
        server.load_serve_plan(plan, Some(project.clone())).unwrap();

        std::fs::write(root.join("main.mec"), "x := 2\n").unwrap();
        std::fs::write(root.join("added.mec"), "added := 3\n").unwrap();
        let session = server.workspace_session.as_ref().unwrap();
        let mut session = session.lock().unwrap();
        session.refresh(module_options()).unwrap();
        let html_shim = server.injected_html_shim().unwrap();
        {
            let mut registry = server.registry.write().unwrap();
            registry
                .sync_workspace_snapshot(
                    &root,
                    session.snapshot().unwrap(),
                    &server.stylesheets,
                    &html_shim,
                    &server.generated_html_backing_paths(),
                )
                .unwrap();
            registry.sync_project_overlay(&project).unwrap();
        }
        drop(session);
        let registry = server.registry.read().unwrap();
        let raw = registry.get_route("source/main.mec").unwrap();
        assert!(String::from_utf8(raw.bytes).unwrap().contains("x := 2"));
        assert!(registry.get_route("added.mec").is_some());
        let manifest = String::from_utf8(
            registry
                .get_route("_mech/project-sources.json")
                .unwrap()
                .bytes,
        )
        .unwrap();
        assert!(manifest.contains("source/added.mec"));
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
    fn explicit_directory_without_config_is_recursive_workspace() {
        let root = temp_root("explicit-directory-no-config");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("fizzbuzz.mec"), "x := 1\n").unwrap();
        std::fs::create_dir_all(dir.join("nested")).unwrap();
        std::fs::write(dir.join("nested/other.mec"), "other := 2\n").unwrap();
        std::fs::write(dir.join("style.css"), "body {}\n").unwrap();
        std::fs::write(root.join("ROADMAP.mec"), "roadmap := true\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);
        let mut server = initialized_server();
        server
            .load_workspace(&vec!["examples/working".to_string()])
            .unwrap();
        let registry = server.registry.read().unwrap();
        assert!(server.workspace_session.is_some());
        assert!(registry.get_route("fizzbuzz.mec").is_some());
        assert!(registry.get_route("nested/other.mec").is_some());
        assert!(registry.get_route("style.css").is_some());
        assert!(registry.get_route("ROADMAP.mec").is_none());
        assert!(registry.get_route("mech.mcfg").is_none());
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[cfg(windows)]
    #[test]
    fn windows_relative_project_directory_is_resolved_before_verbatim_canonicalization() {
        let root = temp_root("windows-relative-project-directory");
        let project = root.join("examples").join("ekf");
        std::fs::create_dir_all(&project).unwrap();
        std::fs::write(project.join("localization.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&root);

        let plan = plan_serve_inputs(&["examples/ekf".to_owned()]).unwrap();

        assert_eq!(plan.root, project.canonicalize().unwrap());
        assert_eq!(
            plan.folders,
            vec![RuntimeWorkspaceFolder {
                specifier: ".".to_owned(),
                recursive: true,
            }]
        );
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
    fn directory_index_mec_becomes_root() {
        let root = temp_root("serve-dir-index");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("index.mec"), "x := 1\n").unwrap();
        let guard = CurrentDirGuard::enter(&dir);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server.load_workspace(&Vec::new()).unwrap();
        let registry = server.registry.read().unwrap();
        let (_, trace) = registry.get_route_with_trace("/").unwrap();
        assert!(trace.contains("index.mec"));
        drop(registry);
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_mixed_workspace_directory_loads_static_assets_relative_to_directory() {
        let root = temp_root("serve-dir-static");
        let dir = root.join("examples").join("working");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("style.css"), "body {}\n").unwrap();
        let guard = CurrentDirGuard::enter(&dir);
        let mut server = test_server();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        server
            .load_workspace(&vec![".".to_string(), "style.css".to_string()])
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
    fn server_init_requires_fs_serve_for_local_resource() {
        let root = temp_root("configured-init-denied");
        let shim = root.join("shim.html");
        std::fs::write(&shim, "shim").unwrap();
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
            b"\0asm\x01\0\0\0".to_vec(),
            b"js".to_vec(),
            authority,
        );
        server.set_resource_backing_paths(vec![shim.clone()], Vec::new(), Vec::new(), Vec::new());
        assert!(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(server.init())
                .is_err()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_init_accepts_authorized_local_resource() {
        let root = temp_root("configured-init-allowed");
        let shim = root.join("shim.html");
        std::fs::write(&shim, "shim").unwrap();
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
            b"\0asm\x01\0\0\0".to_vec(),
            b"js".to_vec(),
            authority,
        );
        server.set_resource_backing_paths(vec![shim.clone()], Vec::new(), Vec::new(), Vec::new());
        assert!(
            tokio::runtime::Runtime::new()
                .unwrap()
                .block_on(server.init())
                .is_ok()
        );
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn server_asset_requires_all_backing_paths() {
        let root = temp_root("asset-all-paths");
        let one = root.join("one.html");
        let two = root.join("two.html");
        std::fs::write(&one, "one").unwrap();
        std::fs::write(&two, "two").unwrap();
        let mut ids = DefaultIdGenerator::new();
        let mut authority = HostFilesystemAuthority::new(
            MECH_TOOL_SUBJECT,
            mech_runtime::SharedCapabilityKernel::new(),
        );
        authority
            .grant_path(&mut ids, &one, false, [FS_SERVE])
            .unwrap();
        authority
            .grant_path(&mut ids, &two, false, [FS_SERVE])
            .unwrap();
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &one, false, [FS_SERVE])
            .unwrap();
        let asset = ServerAsset {
            bytes: Vec::new(),
            content_type: "text/html",
            content_encoding: None,
            backing_paths: vec![one.clone(), two.clone()],
        };
        assert!(authorize_server_asset(&authority.kernel(), SERVE_HOST_SUBJECT, &asset).is_err());
        authority
            .delegate_path_to(&mut ids, SERVE_HOST_SUBJECT, &two, false, [FS_SERVE])
            .unwrap();
        assert!(authorize_server_asset(&authority.kernel(), SERVE_HOST_SUBJECT, &asset).is_ok());
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn generated_source_html_tracks_shim_and_stylesheet_paths() {
        let root = temp_root("generated-html-backing");
        let source = root.join("main.mec");
        let shim = root.join("shim.html");
        let style = root.join("style.css");
        std::fs::write(&source, "x := 1\n").unwrap();
        std::fs::write(&shim, "shim").unwrap();
        std::fs::write(&style, "style").unwrap();
        let mut registry = ServerSourceRegistry::default();
        registry
            .sync_workspace_snapshot(
                &root,
                &snapshot(&root, "main.mec"),
                "",
                "",
                &[shim.clone(), style.clone(), shim.clone()],
            )
            .unwrap();
        let asset = registry.html_sources.get("main.mec").unwrap();
        assert_eq!(
            asset.backing_paths,
            vec![source.canonicalize().unwrap(), shim, style]
        );
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
            inputs: vec![],
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
    fn server_event_retention_is_bounded() {
        let events = Arc::new(RwLock::new(Vec::new()));
        let mut sink = ServerEventSink {
            events: events.clone(),
            max_events: Some(2),
        };
        for id in 1u64..=3 {
            sink.emit(RuntimeEvent::new(
                EventId(id as u128),
                id,
                mech_runtime::RuntimeEventKind::RuntimeError {
                    message: format!("event {id}"),
                },
            ))
            .unwrap();
        }
        let ids = events
            .read()
            .unwrap()
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>();
        assert_eq!(ids, vec![EventId(2), EventId(3)]);
    }

    #[test]
    fn empty_workspace_poll_does_not_clone_the_served_registry() {
        let root = temp_root("empty-poll-registry");
        let session =
            ServerWorkspaceSession::open(&root, vec![], vec![], module_options()).unwrap();
        let session = Arc::new(Mutex::new(session));

        let mut served = ServerSourceRegistry::default();
        served.insert_asset("known.txt", asset(b"known", "text/plain", None, Vec::new()));
        let registry = Arc::new(RwLock::new(served));
        let original_bytes = registry
            .read()
            .unwrap()
            .get_route("known.txt")
            .unwrap()
            .bytes
            .as_ptr();

        poll_workspace_once(
            &session,
            &registry,
            &Arc::new(RwLock::new(Vec::new())),
            &root,
            &HtmlStyleSheets::default(),
            "",
            &[],
            None,
            None,
        )
        .unwrap();

        let current_bytes = registry
            .read()
            .unwrap()
            .get_route("known.txt")
            .unwrap()
            .bytes
            .as_ptr();
        assert_eq!(current_bytes, original_bytes);

        drop(session);
        std::fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn invalid_server_address_returns_error() {
        let mut server = test_server();
        server.full_address = "not a socket address".to_string();
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.init())
            .unwrap();
        let (_tx, rx) = tokio::sync::watch::channel(false);
        let error = tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(server.serve_until_shutdown(rx))
            .unwrap_err();
        assert!(error.full_chain_message().contains("not a socket address"));
    }

    #[test]
    fn poll_workspace_error_preserves_last_known_good_registry() {
        let root = temp_root("poll-preserve");
        let mut registry = ServerSourceRegistry::default();
        registry.insert_asset("known.txt", asset(b"known", "text/plain", None, Vec::new()));
        let before = registry.clone();
        let mut candidate = registry.clone();
        let bad_snapshot = RuntimeWorkspaceSnapshot {
            root: root.clone(),
            sources: std::iter::once((
                "missing".to_string(),
                mech_runtime::RuntimeWorkspaceSourceSnapshot {
                    canonical_uri: "missing".to_string(),
                    path: Some(root.join("missing.mec")),
                    source: None,
                    syntax_tree: None,
                    module_version: None,
                    content_hash: 0,
                    modified_time: None,
                },
            ))
            .collect(),
            ..RuntimeWorkspaceSnapshot::default()
        };
        let result = candidate.sync_workspace_snapshot(&root, &bad_snapshot, "", "", &[]);
        assert!(result.is_err());
        assert_eq!(
            registry.get_route("known.txt").unwrap().bytes,
            before.get_route("known.txt").unwrap().bytes
        );
        std::fs::remove_dir_all(root).unwrap();
    }
}
