#![cfg(feature = "standard-hosts")]

use std::fs;
use std::io::Write as _;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Arc;

use mech_build::{
    GeneratedDependencySource, MECH_COMPONENT_VERSION, NativeApplicationBuilder,
    NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest, NativeDependencySource,
    NativeEmit, NativeHostCatalog, NativeHostLinkage, NativeRuntimeConfig, NativeTargetFamily,
    generated_dependencies_from_plan, render_generated_native_project,
};
use mech_core::FunctionCatalogBuilder;
use mech_native_live_host_fixture::{
    TEST_LIVE_CONTEXT, TEST_LIVE_INSTANCE, TEST_LIVE_PATH, TEST_LIVE_PROVIDER, empty_settings,
    test_live_manifest, validate_settings,
};
use mech_runtime::{HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

const LITERAL_F64: &[u8] =
    include_bytes!("../../../tests/architecture/bytecode-v1/phase1/literal-f64.mecb");
const SYNTHETIC_LIVE_READ: &[u8] =
    include_bytes!("../../../tests/architecture/bytecode-v1/phase1/synthetic-live-read.mecb");

#[test]
fn registry_project_is_exact_unpatched_and_buildable_with_a_test_only_patch() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = workspace_root();
    let request = NativeBuildRequest {
        bytecode: LITERAL_F64.to_vec(),
        runtime_config: None,
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "registry_component_literal".to_owned(),
        output: temporary.path().join("ignored-output"),
        emit: NativeEmit::CargoProject,
        keep_project: true,
        offline: true,
    };
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: Arc::new(FunctionCatalogBuilder::new().build().unwrap()),
        host_catalog: mech_build::standard_native_host_catalog().unwrap(),
        dependency_source: NativeDependencySource::Registry {
            version: MECH_COMPONENT_VERSION.to_owned(),
        },
    });
    let plan = builder.plan(&request).unwrap();
    let original =
        render_generated_native_project(temporary.path().join("original"), &request, &plan)
            .unwrap();
    original.materialize().unwrap();

    for dependency in generated_dependencies_from_plan(&plan).unwrap() {
        assert!(matches!(
            &dependency.source,
            GeneratedDependencySource::Registry { exact_version }
                if !dependency.package.starts_with("mech-")
                    || *exact_version == format!("={MECH_COMPONENT_VERSION}")
        ));
    }
    let original_manifest = fs::read_to_string(original.manifest_path()).unwrap();
    assert!(!original_manifest.contains("[patch.crates-io]"));

    let target_dir = build_copied_project(&original.root, temporary.path(), &workspace);
    let executable = target_dir.join("debug/registry_component_literal");
    let output = Command::new(&executable).arg("--once").output().unwrap();
    assert!(output.status.success());
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "42");
    let output = Command::new(executable)
        .arg("--unexpected")
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(2));
    assert!(String::from_utf8_lossy(&output.stderr).contains("usage: generated-app [--once]"));
    assert_eq!(
        fs::read_to_string(original.manifest_path()).unwrap(),
        original_manifest
    );
    assert!(!original.lockfile_path().exists());
}

#[test]
fn live_registry_project_compiles_the_ctrlc_path_and_runs_once() {
    let temporary = tempfile::tempdir().unwrap();
    let workspace = workspace_root();
    let runtime_config = synthetic_live_runtime_config();
    let request = NativeBuildRequest {
        bytecode: SYNTHETIC_LIVE_READ.to_vec(),
        runtime_config: Some(runtime_config),
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "registry_synthetic_live".to_owned(),
        output: temporary.path().join("ignored-output"),
        emit: NativeEmit::CargoProject,
        keep_project: true,
        offline: true,
    };
    let builder = NativeApplicationBuilder::new(NativeBuildEnvironment {
        function_catalog: mech_stdlib::native_plan_catalog(),
        host_catalog: synthetic_live_host_catalog(),
        dependency_source: NativeDependencySource::Registry {
            version: MECH_COMPONENT_VERSION.to_owned(),
        },
    });
    let plan = builder.plan(&request).unwrap();
    assert!(plan.live);
    let original =
        render_generated_native_project(temporary.path().join("original-live"), &request, &plan)
            .unwrap();
    original.materialize().unwrap();
    assert!(original.cargo_manifest.contains("ctrlc = {"));
    assert!(original.cargo_manifest.contains("version = \"=3.5.2\""));

    let target_dir = build_copied_project(&original.root, temporary.path(), &workspace);
    let output = Command::new(target_dir.join("debug/registry_synthetic_live"))
        .arg("--once")
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "live --once project failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "0");
}

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .unwrap()
        .to_path_buf()
}

fn copy_tree(source: &Path, destination: &Path) {
    fs::create_dir_all(destination).unwrap();
    for entry in fs::read_dir(source).unwrap() {
        let entry = entry.unwrap();
        let target = destination.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

fn build_copied_project(original: &Path, temporary: &Path, workspace: &Path) -> PathBuf {
    let copied_root = temporary.join("copied");
    let target_dir = workspace.join("target/mech-native/registry-tests");
    copy_tree(original, &copied_root);
    append_test_only_workspace_patch(&copied_root.join("Cargo.toml"), workspace);
    let output = Command::new("cargo")
        .arg("+nightly-2026-03-03")
        .arg("build")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(copied_root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "test-patched registry project failed: stdout={} stderr={}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
    target_dir
}

fn synthetic_live_host_catalog() -> Arc<NativeHostCatalog> {
    let mut catalog = NativeHostCatalog::new();
    catalog
        .insert_provider(NativeHostLinkage {
            provider: TEST_LIVE_PROVIDER,
            package: "mech-native-live-host-fixture",
            crate_name: "mech_native_live_host_fixture",
            cargo_features: &[],
            factory_path: "mech_native_live_host_fixture::TestLiveHostFactory::native",
            supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
            manifest: test_live_manifest,
            validate_settings,
        })
        .unwrap();
    Arc::new(catalog)
}

fn synthetic_live_runtime_config() -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::new("registry-synthetic-live"),
        hosts: vec![HostInstanceConfig {
            name: TEST_LIVE_INSTANCE.to_owned(),
            provider: TEST_LIVE_PROVIDER.to_owned(),
            settings: empty_settings(),
        }],
        run_grants: vec![RunResourceGrantConfig {
            target: format!("{TEST_LIVE_INSTANCE}/{TEST_LIVE_CONTEXT}"),
            operations: vec!["read".to_owned()],
            paths: vec![TEST_LIVE_PATH.to_owned()],
        }],
    }
}

fn append_test_only_workspace_patch(manifest: &Path, workspace: &Path) {
    let mut patch = String::from("\n[patch.crates-io]\n");
    for (package, relative_path) in [
        ("mech-core", "src/core"),
        ("mech-engine", "src/engine"),
        ("mech-runtime", "src/runtime"),
        ("mech-syntax", "src/syntax"),
        ("mech-bytecode", "src/bytecode"),
        ("mech-math", "machines/math"),
        ("mech-compare", "machines/compare"),
        ("mech-logic", "machines/logic"),
        ("mech-range", "machines/range"),
        ("mech-matrix", "machines/matrix"),
        ("mech-set", "machines/set"),
        ("mech-string", "machines/string"),
        ("mech-stats", "machines/stats"),
        ("mech-combinatorics", "machines/combinatorics"),
        ("mech-host-cli", "hosts/cli"),
        ("mech-host-console", "hosts/console"),
        ("mech-host-time", "hosts/time"),
        ("mech-host-timer", "hosts/timer"),
        ("mech-host-scene", "hosts/scene"),
        ("mech-host-robot-arm", "hosts/robot-arm"),
        (
            "mech-native-live-host-fixture",
            "tests/fixtures/native-live-host",
        ),
    ] {
        let path = workspace.join(relative_path).display().to_string();
        patch.push_str(&format!(
            "{package} = {{ path = \"{}\" }}\n",
            path.replace('\\', "\\\\").replace('\"', "\\\"")
        ));
    }
    fs::OpenOptions::new()
        .append(true)
        .open(manifest)
        .unwrap()
        .write_all(patch.as_bytes())
        .unwrap();
}
