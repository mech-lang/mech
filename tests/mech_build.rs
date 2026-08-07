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

fn write_actor_config(path: &Path, subject: &str, kind: &str, payload: &str, state: Option<&str>) {
    let initial_state = state
        .map(|state| format!(r#", initial-state: "{state}""#))
        .unwrap_or_default();
    std::fs::write(
        path,
        format!(
            r#"config := {{build: {{actor: {{subject: "{subject}", message-kind: "{kind}", message-payload: "{payload}"{initial_state}}}}}}}"#,
        ),
    )
    .unwrap();
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

#[test]
fn actor_build_requires_explicit_bootstrap_and_covers_source_and_bytecode_emits() {
    let root = temp_root("actor-bootstrap-emits");
    let source = root.join("actor.mec");
    std::fs::write(
        &source,
        concat!(
            "kind := actor/message/kind()\n",
            "payload := actor/message/payload()\n",
            "updated := actor/state/put(kind)\n",
            "state-id := actor/state/id()\n",
            "state := actor/state/get()\n",
            "payload\n",
        ),
    )
    .unwrap();
    let alpha = root.join("alpha.mcfg");
    let beta = root.join("beta.mcfg");
    write_actor_config(&alpha, "actor:alpha", "alpha", "payload-a", None);
    write_actor_config(&beta, "actor:beta", "beta", "payload-b", Some("state-b"));

    let bytecode = root.join("actor.mecb");
    assert_success(
        run_configured_emit(
            &root,
            &source,
            &alpha,
            "bytecode",
            &bytecode,
            "actor-app-alpha",
        ),
        "actor source to bytecode",
    );

    let alpha_plan = root.join("alpha-plan.json");
    assert_success(
        run_configured_emit(
            &root,
            &source,
            &alpha,
            "plan",
            &alpha_plan,
            "actor-app-alpha",
        ),
        "actor source to plan",
    );
    let plan: serde_json::Value =
        serde_json::from_slice(&std::fs::read(&alpha_plan).unwrap()).unwrap();
    assert_eq!(plan["actor_bootstrap"]["subject"], "actor:alpha");
    assert_eq!(plan["actor_bootstrap"]["message_kind"], "alpha");
    assert_eq!(plan["actor_bootstrap"]["message_payload"], "payload-a");
    assert_eq!(
        plan["actor_bootstrap"]["initial_state"],
        serde_json::Value::Null
    );
    let contexts = plan["application_requirements"]
        .as_array()
        .unwrap()
        .iter()
        .map(|requirement| requirement["context"].as_str().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(contexts, vec!["actor_turn"; 5]);

    let alpha_project = root.join("alpha-project");
    assert_success(
        run_configured_emit(
            &root,
            &source,
            &alpha,
            "cargo-project",
            &alpha_project,
            "actor-app-alpha",
        ),
        "actor source to Cargo project",
    );
    let runtime = std::fs::read_to_string(alpha_project.join("src/runtime.rs")).unwrap();
    for value in ["actor:alpha", "alpha", "payload-a"] {
        assert!(runtime.contains(value), "runtime source omitted {value}");
    }

    let alpha_native = root.join("alpha-native");
    assert_success(
        run_configured_emit(
            &root,
            &source,
            &alpha,
            "native",
            &alpha_native,
            "actor-app-alpha",
        ),
        "actor source to native",
    );
    let alpha_output = Command::new(&alpha_native).output().unwrap();
    let alpha_stdout = String::from_utf8_lossy(&alpha_output.stdout).into_owned();
    assert_success(alpha_output, "running alpha actor native application");
    assert_eq!(alpha_stdout.trim(), "\"payload-a\"");

    let beta_plan = root.join("beta-plan.json");
    assert_success(
        run_configured_emit(
            &root,
            &bytecode,
            &beta,
            "plan",
            &beta_plan,
            "actor-app-beta",
        ),
        "actor bytecode to plan",
    );
    assert_ne!(plan_digest(&alpha_plan), plan_digest(&beta_plan));

    let beta_native = root.join("beta-native");
    assert_success(
        run_configured_emit(
            &root,
            &bytecode,
            &beta,
            "native",
            &beta_native,
            "actor-app-beta",
        ),
        "actor bytecode to native",
    );
    let beta_output = Command::new(&beta_native).output().unwrap();
    let beta_stdout = String::from_utf8_lossy(&beta_output.stdout).into_owned();
    assert_success(beta_output, "running beta actor native application");
    assert_eq!(beta_stdout.trim(), "\"payload-b\"");

    let missing_output = root.join("missing-plan.json");
    let missing = run_build(&root, &bytecode, "plan", &missing_output, false);
    assert!(!missing.status.success());
    assert!(String::from_utf8_lossy(&missing.stderr).contains("NativeActorBootstrapMissing"));
    assert!(!missing_output.exists());
    assert!(!root.join("missing-plan.json.project").exists());

    std::fs::remove_dir_all(root).unwrap();
}
