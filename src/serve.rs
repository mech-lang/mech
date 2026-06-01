use std::collections::{HashMap, HashSet};
use std::io::{Error, ErrorKind};
use std::net::SocketAddr;
use std::path::{Component, Path, PathBuf};
use std::sync::{Arc, Mutex, RwLock};

use colored::*;
use ignore::WalkBuilder;
use mech_core::*;
use mech_runtime::{
  EventId, EventSink, ModuleBuildOptions, RuntimeEvent, RuntimeWorkspaceFolder,
  RuntimeWorkspaceSnapshot, RuntimeWorkspaceTarget, ServerWorkspaceSession,
};
use warp::{Filter, Reply};

use crate::*;

#[derive(Clone, Debug)]
struct ServerAsset {
  bytes: Vec<u8>,
  content_type: &'static str,
  content_encoding: Option<&'static str>,
}

#[derive(Debug, Default)]
struct ServerSourceRegistry {
  assets: HashMap<String, ServerAsset>,
  raw_sources: HashMap<String, ServerAsset>,
  html_sources: HashMap<String, ServerAsset>,
  code_sources: HashMap<String, ServerAsset>,
  source_paths: HashMap<String, PathBuf>,
  workspace_keys: HashSet<String>,
  user_assets: HashSet<String>,
  index_source: Option<String>,
}

impl ServerSourceRegistry {
  fn insert_asset(&mut self, key: impl Into<String>, asset: ServerAsset) {
    self.assets.insert(key.into(), asset);
  }

  fn insert_user_asset(&mut self, key: impl Into<String>, asset: ServerAsset) {
    let key = key.into();
    self.user_assets.insert(key.clone());
    self.insert_asset(key, asset);
  }

  fn get_route(&self, path: &str) -> Option<ServerAsset> {
    let path = normalize_url_path(path)?;
    if let Some(source) = path.strip_prefix("source/") {
      return self.raw_sources.get(source).cloned();
    }
    if let Some(source) = path.strip_prefix("code/") {
      return self.code_sources.get(source).cloned();
    }
    if path == "index.html" {
      if self.user_assets.contains("index.html") {
        return self.assets.get("index.html").cloned();
      }
      if let Some(index_source) = &self.index_source {
        if let Some(asset) = self.html_sources.get(index_source) {
          return Some(asset.clone());
        }
      }
      return self.assets.get("_mech/index.html")
        .cloned()
        .or_else(|| self.assets.get("index.html").cloned());
    }
    if path.ends_with(".mec") || path.ends_with(".🤖") {
      return self.html_sources.get(&path).cloned();
    }
    if path.ends_with(".html") || path.ends_with(".htm") {
      if let Some(asset) = self.assets.get(&path) {
        return Some(asset.clone());
      }
      let source = Path::new(&path).with_extension("mec");
      return self.html_sources.get(&url_key(&source)?).cloned();
    }
    self.assets.get(&path).cloned()
  }

  fn insert_static_file(&mut self, root: &Path, path: &Path) -> MResult<()> {
    if !is_allowed_static_file(path) {
      return Ok(());
    }
    let path = path.canonicalize()?;
    let relative = path.strip_prefix(root).map_err(|error| {
      Error::new(ErrorKind::InvalidInput, format!("static asset is outside workspace root: {}", error))
    })?;
    let Some(key) = url_key(relative) else {
      return Err(Error::new(ErrorKind::InvalidInput, "invalid static asset path").into());
    };
    self.insert_user_asset(key.clone(), ServerAsset {
      bytes: std::fs::read(&path)?,
      content_type: content_type_for_path(&key),
      content_encoding: content_encoding_for_path(&key),
    });
    Ok(())
  }

  fn sync_workspace_snapshot(
    &mut self,
    root: &Path,
    snapshot: &RuntimeWorkspaceSnapshot,
    stylesheet: &str,
    shim: &str,
  ) -> MResult<()> {
    for key in self.workspace_keys.drain() {
      self.raw_sources.remove(&key);
      self.html_sources.remove(&key);
      self.code_sources.remove(&key);
      self.source_paths.remove(&key);
    }
    self.index_source = None;

    for source in snapshot.sources.values() {
      let Some(path) = source.path.as_ref() else { continue; };
      if !is_mech_source(path) {
        continue;
      }
      let relative = path.strip_prefix(root).map_err(|error| {
        Error::new(ErrorKind::InvalidInput, format!("workspace source is outside workspace root: {}", error))
      })?;
      let Some(key) = url_key(relative) else { continue; };
      let source = std::fs::read_to_string(path)?;
      self.raw_sources.insert(key.clone(), ServerAsset {
        bytes: source.as_bytes().to_vec(),
        content_type: "text/x-mech",
        content_encoding: None,
      });
      match parser::parse(&source) {
        Ok(tree) => {
          let mut formatter = Formatter::new();
          let html = formatter.format_html(&tree, stylesheet.to_string(), shim.to_string());
          self.html_sources.insert(key.clone(), ServerAsset {
            bytes: html.into_bytes(),
            content_type: "text/html",
            content_encoding: None,
          });
          #[cfg(feature = "serde")]
          self.code_sources.insert(key.clone(), ServerAsset {
            bytes: compress_and_encode(&tree).map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?.into_bytes(),
            content_type: "text/plain",
            content_encoding: None,
          });
        }
        Err(error) => {
          let html = format!("<html><body><pre>{}</pre></body></html>", escape_html(&format!("{:#?}", error)));
          self.html_sources.insert(key.clone(), ServerAsset {
            bytes: html.into_bytes(),
            content_type: "text/html",
            content_encoding: None,
          });
        }
      }
      self.source_paths.insert(key.clone(), path.clone());
      self.workspace_keys.insert(key.clone());
      if self.index_source.is_none() || key == "index.mec" {
        self.index_source = Some(key);
      }
    }
    Ok(())
  }
}

pub struct MechServer {
  name: String,
  init: bool,
  stylesheet: String,
  html_shim: String,
  full_address: String,
  registry: Arc<RwLock<ServerSourceRegistry>>,
  events: Arc<RwLock<Vec<RuntimeEvent>>>,
  workspace_session: Option<Arc<Mutex<ServerWorkspaceSession>>>,
  workspace_root: Option<PathBuf>,
  js: Vec<u8>,
  wasm: Vec<u8>,
}

impl MechServer {
  pub fn new(name: String, full_address: String, stylesheet: String, html_shim: String, wasm: Vec<u8>, js: Vec<u8>) -> Self {
    Self {
      name,
      init: false,
      stylesheet,
      html_shim,
      full_address,
      registry: Arc::new(RwLock::new(ServerSourceRegistry::default())),
      events: Arc::new(RwLock::new(Vec::new())),
      workspace_session: None,
      workspace_root: None,
      js,
      wasm,
    }
  }

  pub async fn init(&mut self) -> MResult<()> {
    let mut registry = self.registry.write().unwrap();
    let html = asset(self.html_shim.as_bytes(), "text/html", None);
    let css = asset(self.stylesheet.as_bytes(), "text/css", None);
    let js = asset(&self.js, "application/javascript", None);
    let wasm = asset(&self.wasm, "application/wasm", Some("br"));
    registry.insert_asset("index.html", html.clone());
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

  pub fn load_workspace(&mut self, paths: &Vec<String>) -> MResult<()> {
    if !self.init {
      return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
    }
    let root = std::env::current_dir()?.canonicalize()?;
    self.workspace_root = Some(root.clone());
    let mut targets = Vec::new();
    let mut static_paths = Vec::new();
    for specifier in paths {
      let path = Path::new(specifier);
      if is_mech_source(path) {
        targets.push(RuntimeWorkspaceTarget {
          name: target_name(specifier),
          specifier: specifier.clone(),
        });
      } else {
        static_paths.push(specifier.clone());
      }
    }
    load_static_assets_from_paths(&mut self.registry.write().unwrap(), &root, &static_paths)?;
    let mut session = ServerWorkspaceSession::open(
      &root,
      targets,
      vec![RuntimeWorkspaceFolder { specifier: ".".to_string(), recursive: true }],
      module_options(),
    )?;
    let mut sink = ServerEventSink { events: self.events.clone() };
    session.emit_initial_events(&mut sink)?;
    if let Some(snapshot) = session.snapshot() {
      self.registry.write().unwrap().sync_workspace_snapshot(&root, snapshot, &self.stylesheet, &self.html_shim)?;
    }
    self.workspace_session = Some(Arc::new(Mutex::new(session)));
    Ok(())
  }

  pub async fn serve(&self) -> MResult<()> {
    if !self.init {
      return Err(MechError::new(ServerNotInitializedError, None).with_compiler_loc());
    }
    let server_badge = || { "[Mech Server]".truecolor(34, 204, 187) };
    ctrlc::set_handler(move || {
      println!("{} Server received shutdown signal. Process terminating.", server_badge());
      std::process::exit(0);
    }).expect("Error setting Ctrl-C handler");

    let root = self.workspace_root.clone();
    let registry = self.registry.clone();
    let routes = warp::get().and(warp::path::full()).map(move |path: warp::path::FullPath| {
      match registry.read().unwrap().get_route(path.as_str()) {
        Some(asset) => response(asset.bytes, asset.content_type, asset.content_encoding, warp::http::StatusCode::OK),
        None => response(
          format!("<html><body><h1>404 Not Found</h1><p>The requested URL {} was not found on this server.</p></body></html>", escape_html(path.as_str())).into_bytes(),
          "text/html",
          None,
          warp::http::StatusCode::NOT_FOUND,
        ),
      }
    });
    println!("{} Awaiting connections at {}", server_badge(), self.full_address);
    let socket_address: SocketAddr = self.full_address.parse().unwrap();
    let server = warp::serve(routes).run(socket_address);
    if let (Some(session), Some(root)) = (&self.workspace_session, &root) {
      let mut interval = tokio::time::interval(std::time::Duration::from_millis(100));
      tokio::pin!(server);
      loop {
        tokio::select! {
          _ = interval.tick() => {
            let _ = poll_workspace_once(session, &self.registry, &self.events, &root, &self.stylesheet, &self.html_shim);
          }
          _ = &mut server => break,
        }
      }
    } else {
      server.await;
    }
    println!("{} Closing server.", server_badge());
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
) -> MResult<()> {
  let mut session = session.lock().unwrap();
  let mut sink = ServerEventSink { events: events.clone() };
  let poll = session.poll_and_emit(module_options(), &mut sink)?;
  if poll.refresh.is_some() {
    if let Some(snapshot) = session.snapshot() {
      registry.write().unwrap().sync_workspace_snapshot(root, snapshot, stylesheet, shim)?;
    }
  }
  Ok(())
}

fn load_static_assets_from_paths(registry: &mut ServerSourceRegistry, root: &Path, paths: &[String]) -> MResult<()> {
  for input in paths {
    let path = Path::new(input);
    if path.is_file() {
      if !is_mech_source(path) {
        registry.insert_static_file(root, path)?;
      }
    } else if path.is_dir() {
      for entry in WalkBuilder::new(path).build() {
        let entry = entry.map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?;
        if entry.file_type().map(|kind| kind.is_file()).unwrap_or(false) && !is_mech_source(entry.path()) {
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
  if segments.iter().any(|segment| segment.is_empty() || *segment == ".." || segment.contains(':')) {
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

fn content_encoding_for_path(path: &str) -> Option<&'static str> {
  if path.ends_with(".wasm.br") {
    Some("br")
  } else {
    None
  }
}

fn content_type_for_path(path: &str) -> &'static str {
  if path.ends_with(".wasm.br") { return "application/wasm"; }
  match Path::new(path).extension().and_then(|ext| ext.to_str()).unwrap_or("") {
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

fn is_mech_source(path: &Path) -> bool {
  matches!(path.extension().and_then(|ext| ext.to_str()), Some("mec") | Some("🤖"))
}

fn is_allowed_static_file(path: &Path) -> bool {
  matches!(path.extension().and_then(|ext| ext.to_str()), Some("html") | Some("htm") | Some("css") | Some("js") | Some("wasm") | Some("br") | Some("png") | Some("jpg") | Some("jpeg") | Some("gif") | Some("svg") | Some("webp") | Some("md") | Some("csv") | Some("json"))
}

fn target_name(specifier: &str) -> String {
  let path = Path::new(specifier).with_extension("");
  let name = path.to_string_lossy().replace(['/', '\\', '.', ' '], "-");
  if name.is_empty() { "main".to_string() } else { name }
}

fn module_options() -> ModuleBuildOptions<'static> {
  ModuleBuildOptions::new("serve", "v0.3", "native", &[], &[])
}

fn asset(bytes: &[u8], content_type: &'static str, content_encoding: Option<&'static str>) -> ServerAsset {
  ServerAsset { bytes: bytes.to_vec(), content_type, content_encoding }
}

fn response(bytes: Vec<u8>, content_type: &'static str, content_encoding: Option<&'static str>, status: warp::http::StatusCode) -> warp::reply::Response {
  let mut response = warp::http::Response::builder().status(status).header("content-type", content_type);
  if let Some(content_encoding) = content_encoding {
    response = response.header("content-encoding", content_encoding);
  }
  response.body(bytes.into()).unwrap()
}

fn escape_html(text: &str) -> String {
  text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;").replace('\'', "&#39;")
}

#[derive(Debug, Clone)]
pub struct ServerNotInitializedError;
impl MechErrorKind for ServerNotInitializedError {
  fn name(&self) -> &str { "ServerNotInitializedError" }
  fn message(&self) -> String { "The server is not initialized.".to_string() }
}

#[derive(Debug, Clone)]
pub struct Utf8ConversionError { pub source_error: String }
impl MechErrorKind for Utf8ConversionError {
  fn name(&self) -> &str { "Utf8ConversionError" }
  fn message(&self) -> String { format!("Failed to convert bytes into UTF-8 string: {}", self.source_error) }
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
      Self { previous, _lock: lock }
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
      SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
  }

  fn snapshot(root: &Path, file: &str) -> RuntimeWorkspaceSnapshot {
    ServerWorkspaceSession::open(
      root,
      vec![RuntimeWorkspaceTarget { name: "main".to_string(), specifier: file.to_string() }],
      vec![],
      module_options(),
    ).unwrap().snapshot().unwrap().clone()
  }

  fn synced_registry(root: &Path, file: &str) -> ServerSourceRegistry {
    let mut registry = ServerSourceRegistry::default();
    registry.sync_workspace_snapshot(root, &snapshot(root, file), "", "").unwrap();
    registry
  }

  fn test_server() -> MechServer {
    MechServer::new(
      "test".to_string(),
      "127.0.0.1:0".to_string(),
      "style".to_string(),
      "shim".to_string(),
      vec![1, 2, 3],
      vec![4, 5, 6],
    )
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
    registry.insert_asset("index.html", asset(b"bundled", "text/html", None));
    registry.insert_asset("_mech/index.html", asset(b"bundled", "text/html", None));
    registry.sync_workspace_snapshot(&root, &snapshot(&root, "index.mec"), "", "").unwrap();
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
    registry.insert_asset("_mech/index.html", asset(b"bundled", "text/html", None));
    registry.insert_static_file(&root, &root.join("index.html")).unwrap();
    registry.sync_workspace_snapshot(&root, &snapshot(&root, "index.mec"), "", "").unwrap();
    let served = registry.get_route("/").unwrap();
    assert_eq!(served.bytes, b"user index");
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
    assert_eq!(registry.get_route("main.mec").unwrap().content_type, "text/html");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn registry_serves_raw_source_under_source_prefix() {
    let root = temp_root("raw");
    std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
    let registry = synced_registry(&root, "main.mec");
    assert!(String::from_utf8(registry.get_route("source/main.mec").unwrap().bytes).unwrap().contains("x := 1"));
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn registry_html_path_falls_back_to_mec_html() {
    let root = temp_root("fallback");
    std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
    let registry = synced_registry(&root, "main.mec");
    assert_eq!(registry.get_route("main.html").unwrap().content_type, "text/html");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn registry_exact_html_asset_wins_over_mec_fallback() {
    let root = temp_root("exact");
    std::fs::write(root.join("main.mec"), "x := 1\n").unwrap();
    let mut registry = synced_registry(&root, "main.mec");
    registry.insert_asset("main.html", asset(b"explicit", "text/html", None));
    assert_eq!(registry.get_route("main.html").unwrap().bytes, b"explicit");
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn registry_removes_stale_workspace_sources_on_resync() {
    let root = temp_root("stale");
    std::fs::write(root.join("a.mec"), "a := 1\n").unwrap();
    std::fs::write(root.join("b.mec"), "b := 2\n").unwrap();
    let mut registry = ServerSourceRegistry::default();
    registry.sync_workspace_snapshot(&root, &snapshot(&root, "a.mec"), "", "").unwrap();
    registry.sync_workspace_snapshot(&root, &snapshot(&root, "b.mec"), "", "").unwrap();
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
    tokio::runtime::Runtime::new().unwrap().block_on(server.init()).unwrap();
    server.load_workspace(&vec!["main.mec".to_string()]).unwrap();
    assert!(server.registry.read().unwrap().get_route("main.mec").is_some());
    assert!(server.registry.read().unwrap().get_route("source/main.mec").is_some());
    drop(guard);
    std::fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn server_load_workspace_preserves_missing_mech_target_diagnostic() {
    let root = temp_root("missing-target");
    let guard = CurrentDirGuard::enter(&root);
    let mut server = test_server();
    tokio::runtime::Runtime::new().unwrap().block_on(server.init()).unwrap();
    server.load_workspace(&vec!["missing.mec".to_string()]).unwrap();
    let session = server.workspace_session.as_ref().unwrap();
    let session = session.lock().unwrap();
    let snapshot = session.snapshot().unwrap();
    assert!(!snapshot.diagnostics.is_empty());
    assert!(snapshot.diagnostics.iter().any(|diagnostic| {
      diagnostic.target.as_deref() == Some("missing")
    }));
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
    tokio::runtime::Runtime::new().unwrap().block_on(server.init()).unwrap();
    server.load_workspace(&vec!["main.mec".to_string()]).unwrap();
    std::fs::write(root.join("main.mec"), "x := 2\n").unwrap();
    let session = server.workspace_session.as_ref().unwrap();
    let mut session = session.lock().unwrap();
    session.refresh(module_options()).unwrap();
    server.registry.write().unwrap().sync_workspace_snapshot(
      &root,
      session.snapshot().unwrap(),
      &server.stylesheet,
      &server.html_shim,
    ).unwrap();
    drop(session);
    let raw = server.registry.read().unwrap().get_route("source/main.mec").unwrap();
    assert!(String::from_utf8(raw.bytes).unwrap().contains("x := 2"));
    drop(guard);
    std::fs::remove_dir_all(root).unwrap();
  }
}
