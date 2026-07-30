#![cfg(feature = "formatter")]

#[path = "support/shim_contract.rs"]
mod shim_contract;

use std::path::{Path, PathBuf};
use std::process::Command;
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

fn format_fixture(shim: Option<&Path>, stylesheet: Option<&Path>, output: &Path) -> String {
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
    let output_result = command.output().expect("Cargo-built mech formatter must start");
    assert!(
        output_result.status.success(),
        "mech format failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output_result.stdout),
        String::from_utf8_lossy(&output_result.stderr),
    );
    std::fs::read_to_string(output).expect("formatter must write the requested HTML file")
}

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

#[test]
fn mech_format_default_shim_restores_rich_shell() {
    let directory = TestDirectory::new("default");
    let html = format_fixture(None, None, &directory.path().join("default.html"));
    shim_contract::assert_rich_shell(
        &html,
        &["id=\"header\"", "id=\"logo\"", "id=\"nav\"", "id=\"github\"", "id=\"resizer\"", "id=\"toggle-repl\""],
    );
}

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
