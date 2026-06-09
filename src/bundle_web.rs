use std::fs;
use std::io::{Error, ErrorKind};
use std::path::{Component, Path, PathBuf};

use mech_core::*;
use mech_syntax::formatter::Formatter;
use mech_syntax::parser;

use crate::LoadedMechConfig;

#[derive(Clone, Debug)]
pub struct BundleWebOptions {
  pub project_dir: PathBuf,
  pub output_dir: PathBuf,
  pub source_paths: Vec<PathBuf>,
  pub shim_path: PathBuf,
  pub stylesheet_paths: Vec<PathBuf>,
  pub wasm_pkg: PathBuf,
  pub loaded_config: LoadedMechConfig,
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
  let output_dir = options.output_dir;
  let stylesheet_string = read_stylesheets(&options.stylesheet_paths)?;
  let shim_string = read_static_friendly_shim(&options.shim_path)?;

  fs::create_dir_all(&output_dir)?;
  fs::write(output_dir.join("style.css"), &stylesheet_string)?;
  copy_wasm_package(&options.wasm_pkg, &output_dir.join("pkg"))?;

  let runtime_config = crate::apply_runtime_config_patch(
    mech_runtime::RuntimeConfig::default(),
    &options.loaded_config.document.runtime,
  )?;
  let host_config = mech_runtime::BrowserHostConfig::from_document_and_runtime(
    &options.loaded_config.document,
    &runtime_config,
  );
  let index_html = output_dir.join("index.html");
  let shim_with_config = crate::inject_browser_host_config_script(&shim_string, &host_config)?;
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

fn read_static_friendly_shim(path: &Path) -> MResult<String> {
  let shim = fs::read_to_string(path)?;
  Ok(shim
    .replace("\"/code/", "\"./code/")
    .replace("\"/source/", "\"./source/")
    .replace("\"/pkg/mech_wasm.js\"", "\"./pkg/mech_wasm.js\"")
    .replace("\"/_mech/pkg/mech_wasm.js\"", "\"./pkg/mech_wasm.js\""))
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
      r#"<!doctype html><html><head></head><body><script type="module">import init from "/pkg/mech_wasm.js"; const code = await fetch("/code/demo.mec");</script></body></html>"#,
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
