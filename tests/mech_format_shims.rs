#![cfg(feature = "formatter")]

#[path = "support/shim_contract.rs"]
mod shim_contract;

use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new(label: &str) -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mech-format-shim-{label}-{}-{sequence}-{nanos}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&path).expect("format shim temporary directory must be created");
        Self { path }
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TestDirectory {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn fixture_path(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("shims")
        .join(name)
}

fn format_fixture_output(
    shim: Option<&Path>,
    stylesheet: Option<&Path>,
    output: &Path,
) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
    command
        .arg("format")
        .arg(fixture_path("all-slots.mec"))
        .arg("--html")
        .arg("--out")
        .arg(output);
    if let Some(shim) = shim {
        command.arg("--shim").arg(shim);
    }
    if let Some(stylesheet) = stylesheet {
        command.arg("--stylesheet").arg(stylesheet);
    }
    command.output().expect("Cargo-built mech formatter must start")
}

fn format_fixture(shim: Option<&Path>, stylesheet: Option<&Path>, output: &Path) -> String {
    let output_result = format_fixture_output(shim, stylesheet, output);
    assert!(
        output_result.status.success(),
        "mech format failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output_result.stdout),
        String::from_utf8_lossy(&output_result.stderr),
    );
    std::fs::read_to_string(output).expect("formatter must write the requested HTML file")
}

#[test]
fn mech_format_static_custom_shim_emits_no_runtime_assets() {
    let directory = TestDirectory::new("static-custom");
    let output = directory.path().join("static.html");
    let html = format_fixture(
        Some(&fixture_path("static-no-controller.html")),
        None,
        &output,
    );
    assert!(html.contains("static-document"));
    assert!(!directory.path().join("_mech/pkg/mech_wasm.js").exists());
    assert!(!directory.path().join("_mech/pkg/mech_wasm_bg.wasm").exists());
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_custom_shim_renders_all_supported_slots() {
    let directory = TestDirectory::new("all-slots");
    let output = directory.path().join("all-slots.html");
    let html = format_fixture(
        Some(&fixture_path("all-slots.html")),
        Some(&fixture_path("all-slots.css")),
        &output,
    );

    shim_contract::assert_complete_slot_contract(&html, "");
    assert!(html.contains("41"), "encoded document program is unexpectedly empty");
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_default_shim_restores_rich_shell() {
    let directory = TestDirectory::new("default");
    let html = format_fixture(None, None, &directory.path().join("default.html"));
    shim_contract::assert_rich_shell(
        &html,
        &["id=\"header\"", "id=\"logo\"", "id=\"nav\"", "id=\"github\"", "id=\"resizer\"", "id=\"toggle-repl\""],
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_blog_shim_restores_rich_shell() {
    let directory = TestDirectory::new("blog");
    let html = format_fixture(
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/blog.html")),
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/blog.css")),
        &directory.path().join("blog.html"),
    );
    shim_contract::assert_rich_shell(
        &html,
        &["site-header", "contentShell", "articleIntro", "articleLayout", "console-pane", "footer"],
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_docs_shim_restores_rich_shell() {
    let directory = TestDirectory::new("docs");
    let html = format_fixture(
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/docs.html")),
        Some(&Path::new(env!("CARGO_MANIFEST_DIR")).join("include/docs.css")),
        &directory.path().join("docs.html"),
    );
    shim_contract::assert_rich_shell(
        &html,
        &["site-header", "contentShell", "articleIntro", "articleLayout", "console-pane", "footer"],
    );
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_bundles_relative_import_sources() {
    use base64::Engine as _;

    let directory = TestDirectory::new("relative-import-bundle");
    let main = directory.path().join("main.mec");
    let support = directory.path().join("support.mec");
    let output = directory.path().join("formatted/main.html");
    std::fs::write(
        &main,
        "+> ./support.mec\nanswer := support/value + 1\nanswer\n",
    )
    .expect("main fixture must be written");
    std::fs::write(&support, "value := 41\n<+ value\n")
        .expect("support fixture must be written");

    let output_result = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("format")
        .arg(&main)
        .arg("--html")
        .arg("--out")
        .arg(&output)
        .output()
        .expect("Cargo-built mech formatter must start");
    assert!(
        output_result.status.success(),
        "mech format failed:\n{}",
        String::from_utf8_lossy(&output_result.stderr),
    );

    let html = std::fs::read_to_string(&output).expect("formatted page must exist");
    let mount = html
        .split("data-mech-document-sources>")
        .nth(1)
        .and_then(|tail| tail.split("</script>").next())
        .map(str::trim)
        .expect("formatted page must contain the source bundle mount");
    let bundle: serde_json::Value = serde_json::from_slice(
        &base64::engine::general_purpose::STANDARD
            .decode(mount)
            .expect("source bundle must be base64"),
    )
    .expect("source bundle must be JSON");
    assert_eq!(bundle["version"], 1);
    assert_eq!(bundle["rootSpecifier"], "main.mec");
    assert_eq!(
        bundle["sources"]
            .as_array()
            .expect("source list")
            .iter()
            .map(|source| source["specifier"].as_str().expect("specifier"))
            .collect::<Vec<_>>(),
        vec!["main.mec", "support.mec"],
    );
    assert!(html.contains("data-mech-wasm-module=\"./_mech/pkg/mech_wasm.js\""));
    assert!(!html.contains(&directory.path().display().to_string()));
    assert!(directory.path().join("formatted/_mech/pkg/mech_wasm.js").is_file());
    assert!(directory.path().join("formatted/_mech/pkg/mech_wasm_bg.wasm").is_file());
}

#[cfg(has_file_wasm)]
#[test]
fn mech_format_missing_dependency_writes_no_partial_bundle() {
    let directory = TestDirectory::new("missing-import-bundle");
    let main = directory.path().join("main.mec");
    let output = directory.path().join("formatted/main.html");
    std::fs::write(&main, "+> ./missing.mec\nanswer := 1\n")
        .expect("main fixture must be written");
    let output_result = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("format")
        .arg(&main)
        .arg("--html")
        .arg("--out")
        .arg(&output)
        .output()
        .expect("Cargo-built mech formatter must start");
    assert!(!output_result.status.success());
    assert!(!output.exists());
    assert!(!directory.path().join("formatted/_mech").exists());
}

#[cfg(not(has_file_wasm))]
#[test]
fn mech_format_shipped_controller_explains_missing_embedded_runtime_assets() {
    let directory = TestDirectory::new("missing-runtime-assets");
    let output = directory.path().join("default.html");
    let output = format_fixture_output(None, None, &output);
    assert!(!output.status.success());
    assert!(
        String::from_utf8_lossy(&output.stderr).contains("embedded mech_wasm_bg.wasm is unavailable"),
    );
    assert!(!directory.path().join("_mech").exists());
}
