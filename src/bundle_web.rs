use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

use mech_core::*;
use mech_syntax::formatter::Formatter;
use mech_syntax::parser;

use crate::{HostAuthorityInjection, LoadedMechConfig};

#[derive(Clone, Debug)]
pub struct BundleWebOptions {
  pub project_dir: PathBuf,
  pub output_dir: PathBuf,
  pub source_paths: Vec<PathBuf>,
  pub shim_path: PathBuf,
  pub stylesheet_paths: Vec<PathBuf>,
  pub wasm_pkg: PathBuf,
  pub loaded_config: LoadedMechConfig,
  pub host_config_injection: Option<HostAuthorityInjection>,
}

#[derive(Debug)]
pub struct BundleWebResult {
  pub output_dir: PathBuf,
  pub index_html: PathBuf,
  pub source_count: usize,
}

pub fn bundle_web_project(options: BundleWebOptions) -> MResult<BundleWebResult> {
  if options.source_paths.is_empty() {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      "bundle-web requires serve.paths in the project config",
    )
    .into());
  }

  let project_dir = options.project_dir.canonicalize()?;
  let base_dir = options.loaded_config.base_dir.canonicalize()?;
  let wasm_pkg = options.wasm_pkg.canonicalize()?;
  fs::create_dir_all(&options.output_dir)?;
  let output_dir = options.output_dir.canonicalize()?;
  if output_dir == project_dir {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!(
        "bundle-web output directory must not be the project root: {}. Use a subdirectory such as dist/<name>.",
        output_dir.display(),
      ),
    )
    .into());
  }
  if output_dir == base_dir {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!(
        "bundle-web output directory must not be the config base directory: {}. Use a subdirectory such as dist/<name>.",
        output_dir.display(),
      ),
    )
    .into());
  }
  let stylesheet_string = read_stylesheets(&options.stylesheet_paths)?;
  let shim_string = read_shim(&options.shim_path)?;
  validate_static_web_shim(&shim_string)?;

  copy_project_static_assets(
    &project_dir,
    &output_dir,
    &[output_dir.clone(), wasm_pkg.clone()],
  )?;
  fs::write(output_dir.join("style.css"), &stylesheet_string)?;
  copy_wasm_package(&wasm_pkg, &output_dir.join("pkg"))?;

  let runtime_config = crate::apply_runtime_config_patch(
    mech_runtime::RuntimeConfig::default(),
    &options.loaded_config.document.runtime,
  )?;
  let host_config = mech_host_browser::BrowserHostConfig::from_document_and_runtime(
    &options.loaded_config.document,
    &runtime_config,
  );
  let index_html = output_dir.join("index.html");
  let injection = options
    .host_config_injection
    .unwrap_or_else(|| HostAuthorityInjection::BrowserUnsigned(host_config));
  let shim_with_config = crate::inject_host_authority_injection_script(&shim_string, &injection)?;
  fs::write(&index_html, shim_with_config)?;

  for source_path in &options.source_paths {
    let source_path = source_path.canonicalize()?;
    let relative = relative_source_path(&source_path, &base_dir, &project_dir)?;
    let source_text = fs::read_to_string(&source_path)?;
    let tree = parser::parse(&source_text)?;

    write_bundle_file(&output_dir, "source", &relative, source_text.as_bytes())?;

    let encoded = compress_and_encode(&tree)
      .map_err(|error| Error::new(ErrorKind::Other, error.to_string()))?;
    write_bundle_file(&output_dir, "code", &relative, encoded.as_bytes())?;

    let mut formatter = Formatter::new();
    let html = formatter.format_html(&tree, stylesheet_string.clone(), shim_string.clone());
    let html_relative = relative.with_extension("html");
    write_bundle_file(&output_dir, "html", &html_relative, html.as_bytes())?;
  }

  Ok(BundleWebResult {
    output_dir,
    index_html,
    source_count: options.source_paths.len(),
  })
}

fn read_stylesheets(paths: &[PathBuf]) -> MResult<String> {
  let mut combined = String::new();
  for path in paths {
    let stylesheet = fs::read_to_string(path)?;
    if !combined.is_empty() {
      combined.push('\n');
    }
    combined.push_str(&stylesheet);
  }
  Ok(combined)
}

fn read_shim(path: &Path) -> MResult<String> {
  Ok(fs::read_to_string(path)?)
}

fn validate_static_web_shim(shim: &str) -> MResult<()> {
  for (pattern, url, fix) in [
    ("\"/code/", "/code/", "./code/..."),
    ("'/code/", "/code/", "./code/..."),
    ("`/code/", "/code/", "./code/..."),
    ("\"/source/", "/source/", "./source/..."),
    ("'/source/", "/source/", "./source/..."),
    ("`/source/", "/source/", "./source/..."),
    ("\"/pkg/mech_wasm.js", "/pkg/mech_wasm.js", "./pkg/mech_wasm.js"),
    ("'/pkg/mech_wasm.js", "/pkg/mech_wasm.js", "./pkg/mech_wasm.js"),
    ("`/pkg/mech_wasm.js", "/pkg/mech_wasm.js", "./pkg/mech_wasm.js"),
    ("\"/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
    ("'/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
    ("`/_mech/", "/_mech/", "./pkg/mech_wasm.js"),
  ] {
    if shim.contains(pattern) {
      return Err(Error::new(
        ErrorKind::InvalidInput,
        format!(
          "bundle-web shim contains server-root Mech URL `{url}`.\nUse a relative URL such as `{fix}` or `./pkg/mech_wasm.js`.",
        ),
      )
      .into());
    }
  }

  Ok(())
}

pub fn copy_project_static_assets(
  project_dir: &Path,
  output_dir: &Path,
  excluded_dirs: &[PathBuf],
) -> MResult<()> {
  let project_dir = project_dir.canonicalize()?;
  let output_dir = output_dir.canonicalize()?;
  let excluded_dirs = excluded_dirs
    .iter()
    .filter_map(|path| path.canonicalize().ok())
    .collect::<Vec<_>>();
  copy_project_static_assets_inner(&project_dir, &project_dir, &output_dir, &excluded_dirs)
}

fn copy_project_static_assets_inner(
  project_dir: &Path,
  current_dir: &Path,
  output_dir: &Path,
  excluded_dirs: &[PathBuf],
) -> MResult<()> {
  for entry in fs::read_dir(current_dir)? {
    let entry = entry?;
    let path = entry.path();
    let canonical_path = path.canonicalize()?;

    if should_skip_static_asset_path(&canonical_path, output_dir, excluded_dirs) {
      continue;
    }

    if canonical_path.is_dir() {
      if should_skip_static_asset_dir(&canonical_path) {
        continue;
      }
      copy_project_static_assets_inner(project_dir, &canonical_path, output_dir, excluded_dirs)?;
      continue;
    }

    if !is_allowed_static_asset(&canonical_path) {
      continue;
    }

    let relative = canonical_path.strip_prefix(project_dir).map_err(|error| {
      Error::new(
        ErrorKind::InvalidInput,
        format!("bundle-web static asset is outside project root: {error}"),
      )
    })?;
    validate_safe_relative_path(relative)?;
    let output_path = output_dir.join(relative);
    if let Some(parent) = output_path.parent() {
      fs::create_dir_all(parent)?;
    }
    fs::copy(&canonical_path, output_path)?;
  }

  Ok(())
}

fn should_skip_static_asset_path(
  path: &Path,
  output_dir: &Path,
  excluded_dirs: &[PathBuf],
) -> bool {
  path == output_dir
    || path.starts_with(output_dir)
    || excluded_dirs
      .iter()
      .any(|excluded| path == excluded || path.starts_with(excluded))
}

fn should_skip_static_asset_dir(path: &Path) -> bool {
  matches!(
    path.file_name().and_then(|name| name.to_str()),
    Some("target" | "dist" | ".git")
  )
}

fn is_allowed_static_asset(path: &Path) -> bool {
  matches!(
    path.extension().and_then(|extension| extension.to_str()),
    Some(
      "html"
        | "htm"
        | "css"
        | "js"
        | "wasm"
        | "png"
        | "jpg"
        | "jpeg"
        | "gif"
        | "svg"
        | "webp"
        | "md"
        | "csv"
        | "json"
    )
  )
}

fn copy_wasm_package(wasm_pkg: &Path, output_pkg: &Path) -> MResult<()> {
  fs::create_dir_all(output_pkg)?;
  fs::copy(wasm_pkg.join("mech_wasm.js"), output_pkg.join("mech_wasm.js"))?;
  fs::copy(
    wasm_pkg.join("mech_wasm_bg.wasm"),
    output_pkg.join("mech_wasm_bg.wasm"),
  )?;
  Ok(())
}

fn relative_source_path(source: &Path, base_dir: &Path, project_dir: &Path) -> MResult<PathBuf> {
  let relative = if let Ok(relative) = source.strip_prefix(base_dir) {
    relative
  } else if let Ok(relative) = source.strip_prefix(project_dir) {
    relative
  } else {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!("bundle-web source is outside project/config root: {}", source.display()),
    )
    .into());
  };

  validate_safe_relative_path(relative)?;
  Ok(relative.to_path_buf())
}

fn validate_safe_relative_path(path: &Path) -> MResult<()> {
  if path.is_absolute()
    || path.components().any(|component| matches!(component, Component::ParentDir))
  {
    return Err(Error::new(
      ErrorKind::InvalidInput,
      format!("bundle-web rejected unsafe relative path: {}", path.display()),
    )
    .into());
  }
  Ok(())
}

fn write_bundle_file(output_dir: &Path, section: &str, relative: &Path, bytes: &[u8]) -> MResult<()> {
  validate_safe_relative_path(relative)?;
  let path = output_dir.join(section).join(relative);
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)?;
  }
  fs::write(path, bytes)?;
  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;
  use mech_core::nodes::Program;
  use std::time::{SystemTime, UNIX_EPOCH};

  fn temp_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
      "mech-bundle-web-{name}-{}",
      SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos(),
    ));
    fs::create_dir_all(&root).unwrap();
    root.canonicalize().unwrap()
  }

  fn walk_has_mcfg(root: &Path) -> bool {
    for entry in fs::read_dir(root).unwrap() {
      let path = entry.unwrap().path();
      if path.is_dir() && walk_has_mcfg(&path) {
        return true;
      }
      if path.extension().map(|extension| extension == "mcfg").unwrap_or(false) {
        return true;
      }
    }
    false
  }

  fn write_demo_project(root: &Path) -> LoadedMechConfig {
    fs::write(
      root.join("demo.mcfg"),
      r#"config := {
  runtime: {name: "bundle-test"}
  serve: {
    paths: ["demo.mec"]
    shim: "index.html"
    wasm: "pkg"
  }
}
"#,
    )
    .unwrap();
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
    fs::write(root.join("demo.mec"), "x := 1\n").unwrap();
    fs::create_dir_all(root.join("pkg")).unwrap();
    fs::write(root.join("pkg/mech_wasm.js"), "export default async function init() {}\n").unwrap();
    fs::write(root.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
    crate::load_mech_config_path(root.join("demo.mcfg"), Some(root.to_path_buf())).unwrap()
  }

  fn options(root: &Path, out: &Path, loaded: LoadedMechConfig) -> BundleWebOptions {
    BundleWebOptions {
      project_dir: root.to_path_buf(),
      output_dir: out.to_path_buf(),
      source_paths: vec![root.join("demo.mec")],
      shim_path: root.join("index.html"),
      stylesheet_paths: Vec::new(),
      wasm_pkg: root.join("pkg"),
      loaded_config: loaded,
      host_config_injection: None,
    }
  }

  #[test]
  fn bundle_web_requires_source_paths() {
    let root = temp_root("requires-source-paths");
    let loaded = write_demo_project(&root);
    let out = root.join("out");
    let mut options = options(&root, &out, loaded);
    options.source_paths.clear();

    let error = format!("{:?}", bundle_web_project(options).unwrap_err());
    assert!(error.contains("bundle-web requires serve.paths"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_writes_source_code_and_html() {
    let root = temp_root("writes");
    let loaded = write_demo_project(&root);
    let out = root.join("out");

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    assert!(out.join("index.html").is_file());
    assert!(out.join("style.css").is_file());
    assert!(out.join("pkg/mech_wasm.js").is_file());
    assert!(out.join("pkg/mech_wasm_bg.wasm").is_file());
    assert!(out.join("source/demo.mec").is_file());
    assert!(out.join("code/demo.mec").is_file());
    assert!(out.join("html/demo.html").is_file());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_code_payload_decodes() {
    let root = temp_root("decodes");
    let loaded = write_demo_project(&root);
    let out = root.join("out");

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    let source = fs::read_to_string(root.join("demo.mec")).unwrap();
    let encoded = fs::read_to_string(out.join("code/demo.mec")).unwrap();
    let decoded: Program = decode_and_decompress(&encoded).unwrap();
    assert_eq!(decoded, parser::parse(&source).unwrap());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_injects_browser_host_config() {
    let root = temp_root("host-config");
    let loaded = write_demo_project(&root);
    let out = root.join("out");

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    let index = fs::read_to_string(out.join("index.html")).unwrap();
    assert!(index.contains("window.__MECH_HOST_CONFIG"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_copies_static_assets() {
    let root = temp_root("static-assets");
    let loaded = write_demo_project(&root);
    let out = root.join("out");
    fs::write(root.join("app.js"), "console.log('app');\n").unwrap();
    fs::create_dir_all(root.join("assets")).unwrap();
    fs::write(root.join("assets/logo.svg"), "<svg></svg>\n").unwrap();

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    assert!(out.join("app.js").is_file());
    assert!(out.join("assets/logo.svg").is_file());
    assert!(!out.join("demo.mcfg").exists());
    assert!(!out.join("demo.mec").exists());
    assert!(out.join("source/demo.mec").is_file());
    assert!(out.join("code/demo.mec").is_file());
    assert!(out.join("html/demo.html").is_file());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_output_inside_project_does_not_copy_itself() {
    let root = temp_root("output-inside-project");
    let loaded = write_demo_project(&root);
    let out = root.join("dist/bundle");
    fs::create_dir_all(&out).unwrap();
    fs::write(out.join("stale.js"), "console.log('stale');\n").unwrap();

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    assert!(out.join("stale.js").is_file());
    assert!(!out.join("dist/bundle/stale.js").exists());
    assert!(!out.join("bundle/stale.js").exists());
    assert!(out.join("source/demo.mec").is_file());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_output_equal_project_root() {
    let root = temp_root("output-project-root");
    let loaded = write_demo_project(&root);
    let original_index = fs::read_to_string(root.join("index.html")).unwrap();

    let error = format!("{:?}", bundle_web_project(options(&root, &root, loaded)).unwrap_err());

    assert!(error.contains("bundle-web output directory must not be the project root"));
    assert_eq!(fs::read_to_string(root.join("index.html")).unwrap(), original_index);
    assert!(!root.join("style.css").exists());
    assert!(!root.join("source/demo.mec").exists());
    assert!(!root.join("code/demo.mec").exists());
    assert!(!root.join("html/demo.html").exists());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_output_equal_config_base_dir() {
    let root = temp_root("output-config-base");
    let app = root.join("app");
    let config = root.join("config");
    fs::create_dir_all(app.join("pkg")).unwrap();
    fs::create_dir_all(&config).unwrap();
    fs::write(
      app.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec");</script></body></html>"#,
    )
    .unwrap();
    fs::write(app.join("demo.mec"), "x := 1\n").unwrap();
    fs::write(app.join("pkg/mech_wasm.js"), "export default async function init() {}\n").unwrap();
    fs::write(app.join("pkg/mech_wasm_bg.wasm"), b"wasm").unwrap();
    fs::write(
      config.join("demo.mcfg"),
      r#"config := {
  runtime: {name: "bundle-config-base-test"}
  serve: {
    paths: ["../app/demo.mec"]
    shim: "../app/index.html"
    wasm: "../app/pkg"
  }
}
"#,
    )
    .unwrap();
    let loaded = crate::load_mech_config_path(
      config.join("demo.mcfg"),
      Some(config.clone()),
    )
    .unwrap();
    let options = BundleWebOptions {
      project_dir: app.clone(),
      output_dir: config.clone(),
      source_paths: vec![app.join("demo.mec")],
      shim_path: app.join("index.html"),
      stylesheet_paths: Vec::new(),
      wasm_pkg: app.join("pkg"),
      loaded_config: loaded,
      host_config_injection: None,
    };

    let error = format!("{:?}", bundle_web_project(options).unwrap_err());

    assert!(error.contains("bundle-web output directory must not be the config base directory"));
    assert!(config.join("demo.mcfg").is_file());
    assert!(!config.join("style.css").exists());
    assert!(!config.join("source/demo.mec").exists());
    assert!(!config.join("code/demo.mec").exists());
    assert!(!config.join("html/demo.html").exists());
    assert!(!config.join("pkg/mech_wasm.js").exists());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_root_relative_code_url() {
    let root = temp_root("root-code-url");
    let loaded = write_demo_project(&root);
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script>fetch("/code/demo.mec")</script></body></html>"#,
    )
    .unwrap();
    let out = root.join("out");

    let error = format!("{:?}", bundle_web_project(options(&root, &out, loaded)).unwrap_err());

    assert!(error.contains("bundle-web shim contains server-root Mech URL"));
    assert!(error.contains("./code/"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_root_relative_pkg_url() {
    let root = temp_root("root-pkg-url");
    let loaded = write_demo_project(&root);
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "/pkg/mech_wasm.js";</script></body></html>"#,
    )
    .unwrap();
    let out = root.join("out");

    let error = format!("{:?}", bundle_web_project(options(&root, &out, loaded)).unwrap_err());

    assert!(error.contains("bundle-web shim contains server-root Mech URL"));
    assert!(error.contains("./pkg/mech_wasm.js"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_root_relative_pkg_url_with_query() {
    let root = temp_root("root-pkg-query-url");
    let loaded = write_demo_project(&root);
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from "/pkg/mech_wasm.js?v=123";</script></body></html>"#,
    )
    .unwrap();
    let out = root.join("out");

    let error = format!("{:?}", bundle_web_project(options(&root, &out, loaded)).unwrap_err());

    assert!(error.contains("bundle-web shim contains server-root Mech URL"));
    assert!(error.contains("./pkg/mech_wasm.js"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_rejects_root_relative_pkg_url_with_fragment() {
    let root = temp_root("root-pkg-fragment-url");
    let loaded = write_demo_project(&root);
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body><script type="module">import init from '/pkg/mech_wasm.js#hash';</script></body></html>"#,
    )
    .unwrap();
    let out = root.join("out");

    let error = format!("{:?}", bundle_web_project(options(&root, &out, loaded)).unwrap_err());

    assert!(error.contains("bundle-web shim contains server-root Mech URL"));
    assert!(error.contains("./pkg/mech_wasm.js"));
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_does_not_reject_ordinary_external_urls() {
    let root = temp_root("external-urls");
    let loaded = write_demo_project(&root);
    fs::write(
      root.join("index.html"),
      r#"<!doctype html><html><head></head><body>
<a href="https://example.com/code/demo.mec">external</a>
<a href="http://localhost:8081/code/demo.mec">local</a>
<script src="//cdn.example.com/pkg/mech_wasm.js"></script>
<a href="mailto:test@example.com">mail</a>
<img src="data:text/plain,hello" />
<script type="module">import init from "./pkg/mech_wasm.js"; const code = await fetch("./code/demo.mec"); const source = await fetch("./source/demo.mec");</script>
</body></html>"#,
    )
    .unwrap();
    let out = root.join("out");

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    assert!(out.join("index.html").is_file());
    fs::remove_dir_all(root).unwrap();
  }

  #[test]
  fn bundle_web_does_not_copy_config() {
    let root = temp_root("no-config-copy");
    let loaded = write_demo_project(&root);
    let out = root.join("out");

    bundle_web_project(options(&root, &out, loaded)).unwrap();

    let has_config = walk_has_mcfg(&out);
    assert!(!has_config);
    fs::remove_dir_all(root).unwrap();
  }
}
