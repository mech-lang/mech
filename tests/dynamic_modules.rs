#![cfg(feature = "dynamic-modules")]

#[path = "support/intrinsic_runner.rs"]
mod intrinsic_runner;

fn run_ok(source: &str) {
    let result = intrinsic_runner::run(source);
    assert!(result.is_ok(), "expected program to run successfully");
}

fn run_err(source: &str) {
    let result = intrinsic_runner::run(source);
    assert!(result.is_err(), "expected program to fail");
}

#[test]
fn dynamic_math_module_item_and_glob_imports_work() {
    run_ok(
        "+> math
+> math/sin
+> math/*
a := math/sin(0.0)
b := sin(0.0)
c := cos(0.0)
d := tan(0.0)
",
    );
}

#[test]
fn dynamic_combinatorics_item_import_works() {
    run_ok(
        "+> combinatorics/n-choose-k
x := n-choose-k(5.0, 2.0)
",
    );
}

#[test]
fn dynamic_matrix_unary_math_import_works() {
    run_ok(
        "+> math/cos
x := cos([0.0 0.0])
",
    );
}

#[test]
fn dynamic_binary_broadcast_import_works() {
    run_ok(
        "+> combinatorics/n-choose-k
x := n-choose-k([10.0 20.0], 2.0)
",
    );
}

#[test]
fn dynamic_missing_module_errors() {
    run_err(
        "+> doesnotexist
x := doesnotexist/thing(1.0)
",
    );
}

#[test]
fn dynamic_missing_item_errors() {
    run_err(
        "+> math/doesnotexist
x := 1
",
    );
}

#[test]
fn dynamic_matrix_matrix_same_cells_different_shape_errors() {
    run_err(
        "+> combinatorics/n-choose-k
x := n-choose-k([10.0 20.0 30.0 40.0], [2.0 3.0; 4.0 5.0])
",
    );
}

#[test]
fn dynamic_item_alias_import_works() {
    run_ok("+> s := math/sin\nx := s(0.0)\n");
}

#[test]
fn dynamic_grouped_item_import_works() {
    run_ok("+> math/{sin, cos, tan}\nx := sin(0.0)\ny := cos(0.0)\nz := tan(0.0)\n");
}

#[test]
fn dynamic_multiline_grouped_item_import_works() {
    run_ok("+> math/{\n  sin\n  cos\n  tan\n}\nx := sin(0.0)\ny := cos(0.0)\nz := tan(0.0)\n");
}

#[test]
fn dynamic_grouped_item_import_does_not_import_other_items() {
    run_err("+> math/{sin, cos, tan}\nx := round(1.23)\n");
}

#[test]
fn dynamic_comma_shorthand_grouped_import_is_rejected() {
    run_err("+> math/sin, cos, tan\nx := sin(0.0)\n");
}

#[test]
fn dynamic_module_alias_import_is_rejected() {
    run_err("+> m := math\nx := m/sin(0.0)\n");
}

#[test]
fn dynamic_glob_alias_import_is_rejected() {
    run_err("+> f := math/*\nx := f(0.0)\n");
}

#[test]
fn dynamic_grouped_item_alias_import_is_rejected() {
    run_err("+> math/{s := sin}\nx := s(0.0)\n");
}

#[test]
fn production_catalog_discovers_dynamic_imports_without_test_preinstallation() {
    use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};
    let catalog = mech_stdlib::source_catalog();
    assert!(!catalog.has_module("status-test"));
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(std::sync::Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    for source in [
        "+> status-test\nstatus-test/unary(3.0)",
        "+> status-test/unary\nunary(3.0)",
        "+> f := status-test/unary\nf(3.0)",
        "+> status-test/*\nunary(3.0)",
        "+> status-test/{unary, binary}\nunary(3.0)",
        "~~~mech\n+> status-test/unary\nunary(3.0)\n~~~\n",
    ] {
        let product = compiler
            .compile_source(source)
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(std::sync::Arc::clone(&catalog))
            .build()
            .unwrap();
        let result = runtime
            .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)
            .unwrap();
        assert_eq!(result.initial_value.format_canonical_inline(), "30");
    }
    for source in [
        "+> absent-dynamic-module\n1.0",
        "+> status-test/absent\n1.0",
    ] {
        assert!(compiler.compile_source(source).is_err(), "{source}");
    }
    // Compilation extends its own immutable catalog; no imported names leak
    // into the caller's catalog or the next retained document.
    assert!(!catalog.has_module("status-test"));
    assert!(compiler.compile_source("unary(3.0)").is_err());
    assert!(compiler.compile_source("answer := 42.0\nanswer").is_ok());
}

#[cfg(feature = "run")]
#[test]
fn production_cli_loads_an_abi_import_from_the_module_path() {
    let directory = tempfile::tempdir().unwrap();
    std::fs::write(
        directory.path().join("main.mec"),
        "+> status-test/unary\nunary(3.0)\n",
    )
    .unwrap();
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(directory.path())
        .args(["--no-config", "run", "main.mec"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.lines().any(|line| line.trim() == "30"), "{stdout}");
}

#[cfg(feature = "build")]
#[test]
fn production_native_build_retains_dynamic_binding_without_compiler_features() {
    use std::process::Command;
    let directory = tempfile::tempdir().unwrap();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"));
    let source = directory.path().join("main.mec");
    let executable = directory.path().join(if cfg!(windows) {
        "dynamic-native.exe"
    } else {
        "dynamic-native"
    });
    std::fs::write(&source, "+> status-test/unary\nunary(3.0)\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(directory.path())
        .args([
            "--no-config",
            "build",
            "main.mec",
            "--profile",
            "debug",
            "--name",
            "dynamic-native",
            "--keep-project",
            "--offline",
            "--workspace-root",
        ])
        .arg(root)
        .arg("--out")
        .arg(&executable)
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let project = directory.path().join(format!(
        "{}.project",
        executable.file_name().unwrap().to_string_lossy()
    ));
    let plan: mech_build::NativeBuildPlan =
        serde_json::from_slice(&std::fs::read(project.join("build-plan.json")).unwrap()).unwrap();
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "dynamic-modules")
    );
    assert!(plan.runtime_functions.is_empty());
    assert!(plan.packages.iter().all(|package| {
        !["mech-syntax", "mech-stdlib", "mech-build"].contains(&package.package.as_str())
    }));
    let manifest = std::fs::read_to_string(project.join("Cargo.toml")).unwrap();
    let engine = manifest
        .lines()
        .find(|line| line.starts_with("mech_engine = "))
        .unwrap();
    assert!(engine.contains("default-features = false"), "{engine}");
    assert!(engine.contains("\"dynamic-modules\""), "{engine}");
    for forbidden in ["source", "compiler", "native-plan"] {
        assert!(!engine.contains(&format!("\"{forbidden}\"")), "{engine}");
    }
    // Execution is a separate process with only generated dependencies, so the
    // compiler's feature unification cannot mask a missing resident loader.
    for _ in 0..2 {
        let output = Command::new(&executable)
            .current_dir(directory.path())
            .output()
            .unwrap();
        assert!(output.status.success(), "{output:?}");
        assert_eq!(String::from_utf8(output.stdout).unwrap().trim(), "30");
    }
    let output = Command::new(&executable)
        .current_dir(directory.path())
        .env("MECH_MODULE_PATH", directory.path().join("absent-modules"))
        .output()
        .unwrap();
    assert!(
        !output.status.success(),
        "missing ABI module was accepted: {output:?}"
    );

    std::fs::write(&source, "+> status-test/unary\n(3.0 ? | * => unary(3.0))\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(directory.path())
        .args([
            "--no-config",
            "build",
            "main.mec",
            "--emit",
            "plan",
            "--offline",
            "--workspace-root",
        ])
        .arg(root)
        .args(["--out", "nested-plan.json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let plan: mech_build::NativeBuildPlan =
        serde_json::from_slice(&std::fs::read(directory.path().join("nested-plan.json")).unwrap())
            .unwrap();
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "dynamic-modules")
    );

    std::fs::write(&source, "left := 1.0\nright := 2.0\nleft + right\n").unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_mech"))
        .current_dir(directory.path())
        .args([
            "--no-config",
            "build",
            "main.mec",
            "--emit",
            "plan",
            "--offline",
            "--workspace-root",
        ])
        .arg(root)
        .args(["--out", "static-plan.json"])
        .output()
        .unwrap();
    assert!(output.status.success(), "{output:?}");
    let plan: mech_build::NativeBuildPlan =
        serde_json::from_slice(&std::fs::read(directory.path().join("static-plan.json")).unwrap())
            .unwrap();
    assert!(
        !plan
            .engine_features
            .iter()
            .any(|feature| feature == "dynamic-modules")
    );
}
