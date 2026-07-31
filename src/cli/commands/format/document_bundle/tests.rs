use std::path::{Path, PathBuf};

use base64::Engine as _;

use super::*;

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mech-document-bundle-{label}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn decode_bundle(root: &Path) -> serde_json::Value {
    let encoded = resolve_document_source_bundle(root).unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    serde_json::from_slice(&decoded).unwrap()
}

fn source_specifiers(bundle: &serde_json::Value) -> Vec<&str> {
    bundle["sources"]
        .as_array()
        .unwrap()
        .iter()
        .map(|source| source["specifier"].as_str().unwrap())
        .collect()
}

fn resolutions(bundle: &serde_json::Value) -> Vec<(&str, &str, &str)> {
    bundle["resolutions"]
        .as_array()
        .unwrap()
        .iter()
        .map(|edge| {
            (
                edge["referrer"].as_str().unwrap(),
                edge["specifier"].as_str().unwrap(),
                edge["target"].as_str().unwrap(),
            )
        })
        .collect()
}

#[test]
fn standalone_bundle_records_every_dependency_resolution() {
    let root = temp_root("edges");
    std::fs::write(
        root.join("main.mec"),
        "+> ./dep.mec\nanswer := dep/value + 1\n",
    )
    .unwrap();
    std::fs::write(
        root.join("dep.mec"),
        "+> ./nested.mec\nvalue := nested/value\n<+ value\n",
    )
    .unwrap();
    std::fs::write(root.join("nested.mec"), "value := 41\n<+ value\n").unwrap();

    let bundle = decode_bundle(&root.join("main.mec"));

    assert_eq!(bundle["version"], 2);
    assert_eq!(bundle["rootSpecifier"], "bundle/000000.mec");
    assert_eq!(
        resolutions(&bundle),
        vec![
            ("bundle/000000.mec", "./dep.mec", "bundle/000001.mec",),
            ("bundle/000001.mec", "./nested.mec", "bundle/000002.mec",),
        ],
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[cfg(unix)]
#[test]
fn standalone_bundle_deduplicates_canonical_source_bodies() {
    use std::os::unix::fs::symlink;

    let root = temp_root("dedupe");
    std::fs::create_dir_all(root.join("vendor")).unwrap();
    std::fs::write(
        root.join("main.mec"),
        "+> ./vendor/first.mec\n+> ./vendor/second.mec\nanswer := 1\n",
    )
    .unwrap();
    std::fs::write(root.join("shared.mec"), "value := 21\n<+ value\n").unwrap();
    symlink("../shared.mec", root.join("vendor/first.mec")).unwrap();
    symlink("../shared.mec", root.join("vendor/second.mec")).unwrap();

    let bundle = decode_bundle(&root.join("main.mec"));

    assert_eq!(source_specifiers(&bundle).len(), 2);
    assert_eq!(
        resolutions(&bundle),
        vec![
            (
                "bundle/000000.mec",
                "./vendor/first.mec",
                "bundle/000001.mec",
            ),
            (
                "bundle/000000.mec",
                "./vendor/second.mec",
                "bundle/000001.mec",
            ),
        ],
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_preserves_two_aliases_to_one_source() {
    let mut edges = BTreeMap::new();
    record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./first.mec".to_string(),
        "bundle/000001.mec".to_string(),
    )
    .unwrap();
    record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./second.mec".to_string(),
        "bundle/000001.mec".to_string(),
    )
    .unwrap();

    assert_eq!(edges.len(), 2);
    assert!(edges.values().all(|target| target == "bundle/000001.mec"));
}

#[test]
fn standalone_bundle_rejects_conflicting_resolution_edges() {
    let mut edges = BTreeMap::new();
    record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./dep.mec".to_string(),
        "bundle/000001.mec".to_string(),
    )
    .unwrap();
    record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./dep.mec".to_string(),
        "bundle/000001.mec".to_string(),
    )
    .unwrap();

    let error = record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./dep.mec".to_string(),
        "bundle/000002.mec".to_string(),
    )
    .unwrap_err();

    assert!(format!("{error:?}").contains("conflicts"));
}

#[test]
fn standalone_bundle_contains_no_absolute_filesystem_paths() {
    let root = temp_root("portable");
    std::fs::write(root.join("main.mec"), "+> ./dep.mec\nanswer := dep/value\n").unwrap();
    std::fs::write(root.join("dep.mec"), "value := 41\n<+ value\n").unwrap();

    let encoded = resolve_document_source_bundle(&root.join("main.mec")).unwrap();
    let decoded = base64::engine::general_purpose::STANDARD
        .decode(encoded)
        .unwrap();
    let json = String::from_utf8(decoded).unwrap();

    assert!(!json.contains(&root.to_string_lossy().to_string()));
    assert!(!json.contains("file://"));
    assert!(!json.contains("bundle/../"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_preserves_extension_fallback_resolution() {
    let root = temp_root("extension");
    std::fs::write(root.join("main.mec"), "+> ./dep.mec\nanswer := dep/value\n").unwrap();
    std::fs::write(root.join("dep.mec.mec"), "value := 41\n<+ value\n").unwrap();

    let bundle = decode_bundle(&root.join("main.mec"));

    assert_eq!(
        resolutions(&bundle),
        vec![("bundle/000000.mec", "./dep.mec", "bundle/000001.mec",)],
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_preserves_index_fallback_resolution() {
    let root = temp_root("index");
    std::fs::create_dir_all(root.join("dep.mec")).unwrap();
    std::fs::write(root.join("main.mec"), "+> ./dep.mec\nanswer := dep/value\n").unwrap();
    std::fs::write(root.join("dep.mec/index.mec"), "value := 41\n<+ value\n").unwrap();

    let bundle = decode_bundle(&root.join("main.mec"));

    assert_eq!(
        resolutions(&bundle),
        vec![("bundle/000000.mec", "./dep.mec", "bundle/000001.mec",)],
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_preserves_unicode_request_spelling() {
    let root = temp_root("unicode");
    std::fs::write(root.join("main.mec"), "+> ./café.mec\nanswer := 1\n").unwrap();
    std::fs::write(root.join("café.mec"), "value := 41\n<+ value\n").unwrap();

    let bundle = decode_bundle(&root.join("main.mec"));

    assert_eq!(
        resolutions(&bundle),
        vec![("bundle/000000.mec", "./café.mec", "bundle/000001.mec",)],
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_preserves_literal_percent_request_spelling() {
    let mut edges = BTreeMap::new();
    record_resolution(
        &mut edges,
        "bundle/000000.mec".to_string(),
        "./rate%25.mec".to_string(),
        "bundle/000001.mec".to_string(),
    )
    .unwrap();

    assert_eq!(
        edges.get(&("bundle/000000.mec".to_string(), "./rate%25.mec".to_string(),)),
        Some(&"bundle/000001.mec".to_string()),
    );
}

#[cfg(unix)]
#[test]
fn standalone_bundle_preserves_symlinked_import_alias() {
    use std::os::unix::fs::symlink;

    let root = temp_root("symlink");
    let project = root.join("project");
    let shared = root.join("shared");
    std::fs::create_dir_all(project.join("vendor")).unwrap();
    std::fs::create_dir_all(&shared).unwrap();
    std::fs::write(
        project.join("main.mec"),
        "+> ./vendor/support.mec\nanswer := support/value + 1\n",
    )
    .unwrap();
    std::fs::write(
        shared.join("support.mec"),
        "+> ./nested.mec\nvalue := nested/value\n<+ value\n",
    )
    .unwrap();
    std::fs::write(shared.join("nested.mec"), "value := 41\n<+ value\n").unwrap();
    symlink(
        "../../shared/support.mec",
        project.join("vendor/support.mec"),
    )
    .unwrap();

    let bundle = decode_bundle(&project.join("main.mec"));

    assert_eq!(source_specifiers(&bundle).len(), 3);
    assert_eq!(
        resolutions(&bundle),
        vec![
            (
                "bundle/000000.mec",
                "./vendor/support.mec",
                "bundle/000001.mec",
            ),
            ("bundle/000001.mec", "./nested.mec", "bundle/000002.mec",),
        ],
    );
    let json = serde_json::to_string(&bundle).unwrap();
    assert!(!json.contains(&root.to_string_lossy().to_string()));
    assert!(!json.contains("file://"));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn standalone_bundle_errors_retain_dependency_referrer() {
    let root = temp_root("missing");
    std::fs::write(root.join("main.mec"), "+> ./missing.mec\nanswer := 1\n").unwrap();

    let error = format!(
        "{:?}",
        resolve_document_source_bundle(&root.join("main.mec")).unwrap_err(),
    );

    assert!(error.contains("dependency `./missing.mec`"), "{error}");
    assert!(error.contains("requested by `file://"), "{error}");
    std::fs::remove_dir_all(root).unwrap();
}
