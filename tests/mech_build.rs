#![cfg(feature = "build")]

use std::path::{Path, PathBuf};
use std::process::{Command, Output};

fn temp_root(label: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!(
        "mech-build-cli-{label}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    root
}

fn run_build(root: &Path, input: &Path, emit: &str, output: &Path, keep: bool) -> Output {
    let mut command = Command::new(env!("CARGO_BIN_EXE_mech"));
    command
        .current_dir(root)
        .arg("--no-config")
        .arg("build")
        .arg(input)
        .arg("--emit")
        .arg(emit)
        .arg("--name")
        .arg("demo")
        .arg("--out")
        .arg(output)
        .arg("--workspace-root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--offline");
    if keep {
        command.arg("--keep-project");
    }
    command.output().unwrap()
}

fn run_configured_build(root: &Path, input: &Path, config: &Path, output: &Path) -> Output {
    run_configured_emit(
        root,
        input,
        config,
        "cargo-project",
        output,
        "configured-host-free",
    )
}

fn run_configured_emit(
    root: &Path,
    input: &Path,
    config: &Path,
    emit: &str,
    output: &Path,
    name: &str,
) -> Output {
    Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(root)
        .arg("--config")
        .arg(config)
        .arg("build")
        .arg(input)
        .arg("--emit")
        .arg(emit)
        .arg("--name")
        .arg(name)
        .arg("--out")
        .arg(output)
        .arg("--workspace-root")
        .arg(env!("CARGO_MANIFEST_DIR"))
        .arg("--offline")
        .output()
        .unwrap()
}

fn assert_success(output: Output, label: &str) {
    assert!(
        output.status.success(),
        "{label} failed\nstdout:\n{}\nstderr:\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr),
    );
}

fn plan_digest(path: &Path) -> String {
    let plan: serde_json::Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    plan.get("plan_sha256")
        .and_then(serde_json::Value::as_str)
        .unwrap()
        .to_owned()
}

fn assert_workspace_project(project: &Path, cargo_target: &Path) {
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR"));
    let manifest = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();

    let mut resolved = Vec::new();
    for suffix in manifest.split("path = \"").skip(1) {
        let path = suffix.split('"').next().unwrap();
        assert!(
            !Path::new(path).is_absolute(),
            "{} contains absolute Cargo path {path}",
            project.display()
        );
        if !path.starts_with("..") {
            continue;
        }
        let candidate = project.join(path);
        let resolved_path = candidate.canonicalize().unwrap_or_else(|error| {
            panic!(
                "failed to resolve Cargo path {} from {}: {error}",
                candidate.display(),
                project.display()
            )
        });
        assert!(resolved_path.starts_with(workspace));
        resolved.push(resolved_path);
    }
    resolved.sort();
    resolved.dedup();
    assert_eq!(
        resolved,
        [
            workspace.join("src/core").canonicalize().unwrap(),
            workspace.join("src/engine").canonicalize().unwrap(),
            workspace.join("src/syntax").canonicalize().unwrap(),
        ]
    );

    assert_eq!(
        std::fs::read_to_string(project.join("rust-toolchain.toml")).unwrap(),
        concat!(
            "[toolchain]\n",
            "channel = \"nightly-2026-03-03\"\n",
            "profile = \"minimal\"\n",
        ),
    );
    let output = Command::new("cargo")
        .current_dir(project)
        .arg("build")
        .arg("--manifest-path")
        .arg(project.join("Cargo.toml"))
        .arg("--locked")
        .arg("--offline")
        .env_remove("RUSTC_BOOTSTRAP")
        .env_remove("RUSTUP_TOOLCHAIN")
        .env("CARGO_TARGET_DIR", cargo_target)
        .output()
        .unwrap();
    assert_success(
        output,
        &format!("building exported project {}", project.display()),
    );
}

#[test]
fn bytecode_only_build_accepts_a_non_cargo_input_stem() {
    let root = temp_root("bytecode-non-cargo-stem");
    let source = root.join("2026-demo.mec");
    let bytecode = root.join("out.mecb");
    std::fs::write(&source, "answer := 42\nanswer\n").unwrap();

    let output = Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(&root)
        .arg("--no-config")
        .arg("build")
        .arg(&source)
        .arg("--emit")
        .arg("bytecode")
        .arg("--out")
        .arg(&bytecode)
        .arg("--offline")
        .output()
        .unwrap();
    assert_success(output, "bytecode-only build with a non-Cargo input stem");
    assert!(bytecode.is_file());

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn distribution_source_bytecode_native_canary() {
    let root = temp_root("distribution-canary");
    let source = root.join("canary.mec");
    let bytecode = root.join("canary.mecb");
    let native = root.join(if cfg!(windows) {
        "canary-native.exe"
    } else {
        "canary-native"
    });
    std::fs::write(&source, "answer := 20.0 + 22.0\nanswer\n").unwrap();

    assert_success(
        run_build(&root, &source, "bytecode", &bytecode, false),
        "distribution canary source to bytecode",
    );
    assert_success(
        run_build(&root, &bytecode, "native", &native, false),
        "distribution canary bytecode to native",
    );

    let output = Command::new(&native).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout).into_owned();
    assert_success(output, "distribution canary native execution");
    assert_eq!(stdout.trim(), "42");

    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn source_and_bytecode_cover_every_authoritative_build_emit() {
    let root = temp_root("all-emits");
    let workspace_export_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/mech/relocation-tests")
        .join(root.file_name().unwrap());
    let cargo_target = root.join("exported-cargo-target");
    let source = root.join("demo.mec");
    std::fs::write(&source, "answer := 42\nanswer\n").unwrap();

    let bytecode = root.join("exact-output.mecb");
    assert_success(
        run_build(&root, &source, "bytecode", &bytecode, false),
        "source to bytecode",
    );
    assert!(bytecode.is_file());

    let source_plan = root.join("source.build-plan.json");
    assert_success(
        run_build(&root, &source, "plan", &source_plan, true),
        "source to plan",
    );
    assert!(source_plan.is_file());
    assert!(
        root.join("source.build-plan.json.project/Cargo.lock")
            .is_file()
    );

    let source_project = workspace_export_root.join("source.cargo");
    assert_success(
        run_build(&root, &source, "cargo-project", &source_project, false),
        "source to Cargo project",
    );
    assert!(source_project.join("Cargo.toml").is_file());
    assert!(source_project.join("Cargo.lock").is_file());

    let external_project = root.join("external.cargo");
    assert_success(
        run_build(&root, &source, "cargo-project", &external_project, false),
        "source to external Cargo project",
    );
    assert!(external_project.join("Cargo.lock").is_file());

    let source_native = root.join("source-native-exact");
    assert_success(
        run_build(&root, &source, "native", &source_native, true),
        "source to native",
    );
    assert!(source_native.is_file());
    assert!(
        root.join("source-native-exact.project/Cargo.lock")
            .is_file()
    );
    let cached_manifest = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("target/mech-native/projects")
        .join(plan_digest(&source_plan))
        .join("Cargo.toml");
    let cached_manifest_before_exports = std::fs::read(&cached_manifest).unwrap();

    let copied_bytecode = root.join("copied-exact.mecb");
    assert_success(
        run_build(&root, &bytecode, "bytecode", &copied_bytecode, true),
        "bytecode validation and copy",
    );
    assert_eq!(
        std::fs::read(&bytecode).unwrap(),
        std::fs::read(&copied_bytecode).unwrap()
    );
    assert!(root.join("copied-exact.mecb.project/Cargo.lock").is_file());

    let bytecode_plan = root.join("bytecode.build-plan.json");
    assert_success(
        run_build(&root, &bytecode, "plan", &bytecode_plan, false),
        "bytecode to plan",
    );
    assert_eq!(plan_digest(&source_plan), plan_digest(&bytecode_plan));

    let bytecode_project = workspace_export_root.join("deeply/nested/bytecode.cargo");
    assert_success(
        run_build(&root, &bytecode, "cargo-project", &bytecode_project, false),
        "bytecode to Cargo project",
    );
    assert!(bytecode_project.join("Cargo.lock").is_file());

    let bytecode_native = root.join("bytecode-native-exact");
    assert_success(
        run_build(&root, &bytecode, "native", &bytecode_native, false),
        "bytecode to native",
    );
    assert!(bytecode_native.is_file());

    for emit in ["bytecode", "plan", "native"] {
        let protected_output = root.join(format!("protected-{emit}"));
        let protected_bytes = format!("preserve-{emit}").into_bytes();
        std::fs::write(&protected_output, &protected_bytes).unwrap();
        std::fs::create_dir(project_output_path_for_test(&protected_output)).unwrap();

        let output = run_build(&root, &bytecode, emit, &protected_output, true);
        assert!(
            !output.status.success(),
            "{emit} unexpectedly overwrote an output with an occupied project sidecar",
        );
        assert!(
            String::from_utf8_lossy(&output.stderr)
                .contains("refusing to overwrite existing Cargo project output"),
        );
        assert_eq!(std::fs::read(&protected_output).unwrap(), protected_bytes);
    }

    for project in [
        &source_project,
        &external_project,
        &bytecode_project,
        &root.join("source.build-plan.json.project"),
        &root.join("source-native-exact.project"),
        &root.join("copied-exact.mecb.project"),
    ] {
        assert_workspace_project(project, &cargo_target);
    }
    assert_eq!(
        std::fs::read(&cached_manifest).unwrap(),
        cached_manifest_before_exports
    );

    std::fs::remove_dir_all(workspace_export_root).unwrap();
    std::fs::remove_dir_all(root).unwrap();
}

fn project_output_path_for_test(output: &Path) -> PathBuf {
    PathBuf::from(format!("{}.project", output.display()))
}

#[test]
fn host_free_build_preserves_configured_runtime_settings() {
    let root = temp_root("host-free-runtime-config");
    let source = root.join("calculation.mec");
    std::fs::write(&source, "answer := 42\nanswer\n").unwrap();
    let config = root.join("application.mcfg");
    std::fs::write(
        &config,
        r#"config := {runtime: {name: "configured-runtime", limits: {max-steps-per-turn: 47}, diagnostics: {trace-enabled: true, log-level: "debug"}}}"#,
    )
    .unwrap();
    let project = root.join("configured.cargo");

    assert_success(
        run_configured_build(&root, &source, &config, &project),
        "host-free configured Cargo project",
    );

    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(project.join("build-plan.json")).unwrap()).unwrap();
    assert_eq!(plan["application_kind"], "hosted");
    assert_eq!(plan["runtime_config"]["name"], "configured-runtime");
    assert_eq!(plan["runtime_config"]["limits"]["max_steps_per_turn"], 47);
    assert_eq!(plan["runtime_config"]["diagnostics"]["trace_enabled"], true);
    assert_eq!(plan["runtime_config"]["diagnostics"]["log_level"], "Debug");
    assert_eq!(plan["hosts"], serde_json::json!([]));
    assert_eq!(plan["run_grants"], serde_json::json!([]));

    let runtime = std::fs::read_to_string(project.join("src/runtime.rs")).unwrap();
    assert!(runtime.contains("\"configured-runtime\".to_string()"));
    assert!(runtime.contains("max_steps_per_turn: Some(47u64)"));
    assert!(runtime.contains("trace_enabled: true"));
    assert!(runtime.contains("log_level: LogLevel::Debug"));

    std::fs::remove_dir_all(root).unwrap();
}
