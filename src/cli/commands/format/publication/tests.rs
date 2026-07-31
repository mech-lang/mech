use std::path::{Path, PathBuf};

use super::*;

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mech-format-publication-{label}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn output(path: impl Into<PathBuf>, bytes: &[u8]) -> PlannedOutput {
    PlannedOutput {
        path: path.into(),
        bytes: bytes.to_vec(),
    }
}

fn assert_contents(path: &Path, expected: &[u8]) {
    assert_eq!(std::fs::read(path).unwrap(), expected);
}

fn publication_artifacts(root: &Path) -> Vec<PathBuf> {
    fn visit(path: &Path, artifacts: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(path) else { return };
        for entry in entries {
            let path = entry.unwrap().path();
            if path.is_dir() {
                visit(&path, artifacts);
            } else if path.file_name().and_then(|name| name.to_str()).is_some_and(|name| {
                name.ends_with(".stage")
                    || name.ends_with(".backup")
                    || name.ends_with(".tmp")
            }) {
                artifacts.push(path);
            }
        }
    }
    let mut artifacts = Vec::new();
    visit(root, &mut artifacts);
    artifacts
}

#[test]
fn publication_restores_js_when_wasm_installation_fails() {
    let root = temp_root("restore-js");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    std::fs::write(&js, b"old-js").unwrap();
    std::fs::write(&wasm, b"old-wasm").unwrap();
    let _faults = inject_publication_faults(&[("install", "mech_wasm_bg.wasm")]);

    let error = publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
    ]).unwrap_err();

    assert!(format!("{error:?}").contains("failed to install"));
    assert_contents(&js, b"old-js");
    assert_contents(&wasm, b"old-wasm");
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_restores_runtime_pair_when_html_installation_fails() {
    let root = temp_root("restore-pair");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    let html = root.join("main.html");
    for (path, bytes) in [
        (&js, b"old-js".as_slice()),
        (&wasm, b"old-wasm".as_slice()),
        (&html, b"old-html".as_slice()),
    ] {
        std::fs::write(path, bytes).unwrap();
    }
    let _faults = inject_publication_faults(&[("install", "main.html")]);

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
        output(&html, b"new-html"),
    ]).unwrap_err();

    assert_contents(&js, b"old-js");
    assert_contents(&wasm, b"old-wasm");
    assert_contents(&html, b"old-html");
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_failure_restores_existing_html_and_assets() {
    let root = temp_root("restore-all");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    let html = root.join("main.html");
    for path in [&js, &wasm, &html] {
        std::fs::write(path, format!("old-{}", path.file_name().unwrap().to_string_lossy())).unwrap();
    }
    let _faults = inject_publication_faults(&[("backup", "main.html")]);

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
        output(&html, b"new-html"),
    ]).unwrap_err();

    assert_contents(&js, b"old-mech_wasm.js");
    assert_contents(&wasm, b"old-mech_wasm_bg.wasm");
    assert_contents(&html, b"old-main.html");
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn first_publication_failure_leaves_no_outputs() {
    let root = temp_root("first-failure");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    let _faults = inject_publication_faults(&[("install", "mech_wasm.js")]);

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
    ]).unwrap_err();

    assert!(!js.exists());
    assert!(!wasm.exists());
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_replaces_all_existing_outputs() {
    let root = temp_root("replace-all");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    let html = root.join("main.html");
    for path in [&js, &wasm, &html] {
        std::fs::write(path, b"old").unwrap();
    }

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
        output(&html, b"new-html"),
    ]).unwrap();

    assert_contents(&js, b"new-js");
    assert_contents(&wasm, b"new-wasm");
    assert_contents(&html, b"new-html");
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_rejects_directory_destination_before_staging() {
    let root = temp_root("directory-destination");
    let directory = root.join("main.html");
    std::fs::create_dir(&directory).unwrap();

    let error = publish_outputs_recoverably(vec![
        output(root.join("new/mech_wasm.js"), b"new-js"),
        output(&directory, b"new-html"),
    ]).unwrap_err();

    assert!(format!("{error:?}").contains("not a regular file"));
    assert!(!root.join("new").exists());
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_rejects_symlink_destination_before_staging() {
    let root = temp_root("symlink-destination");
    let target = root.join("target.html");
    let destination = root.join("main.html");
    std::fs::write(&target, b"target").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink(&target, &destination).unwrap();
    #[cfg(windows)]
    std::os::windows::fs::symlink_file(&target, &destination).unwrap();

    let error = publish_outputs_recoverably(vec![
        output(root.join("new/mech_wasm.js"), b"new-js"),
        output(&destination, b"new-html"),
    ]).unwrap_err();

    assert!(format!("{error:?}").contains("destination is a symlink"));
    assert_contents(&target, b"target");
    assert!(!root.join("new").exists());
    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_rollback_removes_created_directories() {
    let root = temp_root("remove-directories");
    let package = root.join("output/_mech/pkg");
    let js = package.join("mech_wasm.js");
    let wasm = package.join("mech_wasm_bg.wasm");
    let _faults = inject_publication_faults(&[("install", "mech_wasm_bg.wasm")]);

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
    ]).unwrap_err();

    assert!(!root.join("output").exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_leaves_no_staging_or_backup_artifacts() {
    let root = temp_root("no-artifacts");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    std::fs::write(&js, b"old-js").unwrap();
    std::fs::write(&wasm, b"old-wasm").unwrap();
    let _faults = inject_publication_faults(&[("install", "mech_wasm_bg.wasm")]);

    publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
    ]).unwrap_err();

    assert!(publication_artifacts(&root).is_empty());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn publication_reports_rollback_failures() {
    let root = temp_root("rollback-failure");
    let js = root.join("mech_wasm.js");
    let wasm = root.join("mech_wasm_bg.wasm");
    std::fs::write(&js, b"old-js").unwrap();
    std::fs::write(&wasm, b"old-wasm").unwrap();
    let _faults = inject_publication_faults(&[
        ("install", "mech_wasm_bg.wasm"),
        ("restore-backup", "mech_wasm.js"),
    ]);

    let error = publish_outputs_recoverably(vec![
        output(&js, b"new-js"),
        output(&wasm, b"new-wasm"),
    ]).unwrap_err();
    let error = format!("{error:?}");

    assert!(error.contains("rollback failures"), "{error}");
    assert!(error.contains("failed to restore formatter output"), "{error}");
    assert_contents(&wasm, b"old-wasm");
    std::fs::remove_dir_all(root).unwrap();
}
