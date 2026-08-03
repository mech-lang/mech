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

#[test]
fn source_and_bytecode_cover_every_authoritative_build_emit() {
    let root = temp_root("all-emits");
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

    let source_project = root.join("source.cargo");
    assert_success(
        run_build(&root, &source, "cargo-project", &source_project, false),
        "source to Cargo project",
    );
    assert!(source_project.join("Cargo.toml").is_file());
    assert!(source_project.join("Cargo.lock").is_file());

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

    let bytecode_project = root.join("bytecode.cargo");
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

    std::fs::remove_dir_all(root).unwrap();
}
