#![cfg(target_os = "linux")]

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static NEXT_TEMP_DIRECTORY: AtomicU64 = AtomicU64::new(0);

struct TestDirectory {
    path: PathBuf,
}

impl TestDirectory {
    fn new() -> Self {
        let sequence = NEXT_TEMP_DIRECTORY.fetch_add(1, Ordering::Relaxed);
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock must follow the Unix epoch")
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "mech-test-non-utf8-discovery-{}-{sequence}-{nanos}",
            std::process::id(),
        ));
        std::fs::create_dir_all(&path).expect("test directory must be created");
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

#[test]
fn mech_tests_preserve_discovered_non_utf8_filename() {
    let directory = TestDirectory::new();
    let source = directory
        .path()
        .join(OsString::from_vec(b"test-\xff.mec".to_vec()));
    let report = directory.path().join("report.json");
    std::fs::write(&source, "answer := 41\nfilename-pass! := answer == 41\n")
        .expect("non-UTF-8 source fixture must be written");

    let output = Command::new(env!("CARGO_BIN_EXE_mech"))
        .arg("test")
        .arg(directory.path())
        .arg("--out")
        .arg(&report)
        .output()
        .expect("Cargo-built mech test command must start");
    assert!(
        output.status.success(),
        "mech test failed:\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );

    let report = std::fs::read_to_string(&report).expect("JSON report must be written");
    assert!(report.contains("\"files-total\": 1"));
    assert!(report.contains("\"tests-passed\": 1"));
    assert!(report.contains("\"run-error\": null"));
}
