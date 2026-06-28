fn workspace_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn assert_not_contains(path: &str, haystack: &str, needle: &str) {
    assert!(!haystack.contains(needle), "{path} must not contain `{needle}`");
}

fn visit_rs_files(path: &std::path::Path, files: &mut Vec<std::path::PathBuf>) {
    for entry in std::fs::read_dir(path).unwrap() {
        let entry = entry.unwrap();
        let path = entry.path();
        if path.is_dir() {
            visit_rs_files(&path, files);
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("rs") {
            files.push(path);
        }
    }
}

#[test]
fn runtime_core_does_not_embed_concrete_host_policy() {
    let root = workspace_root();
    let runtime_path = "src/runtime/src/runtime/mod.rs";
    let runtime = std::fs::read_to_string(root.join(runtime_path)).unwrap();
    for needle in [
        "default_host_interfaces",
        "BROWSER_DOM_PROVIDER_URI",
        "BrowserDomPath",
        "browser_capability_error",
        "BrowserRuntimeResourceError",
        "BrowserRuntimeResource",
        "browser_runtime_resource_error",
        "browser runtime resource",
        "instance: \"cli\"",
        "instance: \"browser\"",
        "cli://",
        "browser://",
    ] {
        assert_not_contains(runtime_path, &runtime, needle);
    }

    let grant_path = "src/runtime/src/capability/grant.rs";
    let grant = std::fs::read_to_string(root.join(grant_path)).unwrap();
    for needle in [
        "default_host_resource_aliases_match",
        "\"cli\"",
        "\"browser\"",
    ] {
        assert_not_contains(grant_path, &grant, needle);
    }
}

#[test]
fn generic_src_does_not_reference_robot_arm_host_crate() {
    let root = workspace_root();
    let needle = ["mech", "_host", "_robot", "_arm"].concat();
    let mut files = Vec::new();
    visit_rs_files(&root.join("src"), &mut files);
    for path in files {
        let source = std::fs::read_to_string(&path).unwrap();
        let display = path.strip_prefix(&root).unwrap().display().to_string();
        assert_not_contains(&display, &source, &needle);
    }
}
