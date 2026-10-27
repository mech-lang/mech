//! Qualify the compiled launcher owner used by the generated application.
use super::process::{bounded, closure, hash, workspace};
use mech_build::CargoInvocation;
use serde_json::{Value, json};
use std::path::Path;

const PACKAGE: &str = "canonical-launcher-contract";
const TARGET: &str = "canonical_launcher_contract";

// Reuse the generated application's runtime dependency resolution, but retain
// the workspace's original library-test profiles. No compiler dev-dependencies
// enter this private crate; its module paths load the actual launcher and tests.
fn contract_manifest(project: &Path, workspace: &Path) -> toml_edit::DocumentMut {
    use toml_edit::{Array, value};
    let read = |path: &Path| {
        std::fs::read_to_string(path)
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap()
    };
    let mut manifest = read(project);
    manifest["package"]["name"] = value(PACKAGE);
    manifest["package"].as_table_mut().unwrap().remove("build");
    manifest.remove("build-dependencies");
    manifest["lib"]["path"] = value(
        workspace
            .join("hosts/gpu/tests/support/launcher_owner/lib.rs")
            .to_str()
            .unwrap(),
    );
    manifest["features"]["canonical-native-program-runtime"] = value(Array::new());
    manifest["features"]["canonical-native-program-host-runtime"] =
        value(Array::from_iter(["canonical-native-program-runtime"]));
    manifest["features"]["canonical-native-benchmark-policy-runtime"] = value(Array::new());
    manifest["profile"] = read(&workspace.join("Cargo.toml"))["profile"].clone();
    let gpu = read(&workspace.join("hosts/gpu/Cargo.toml"));
    let mut terminal = gpu["dependencies"]["mech-terminal"].clone();
    terminal.as_table_like_mut().unwrap().remove("optional");
    terminal["path"] = value(workspace.join("hosts/terminal").to_str().unwrap());
    manifest["dependencies"]["mech-terminal"] = terminal;
    manifest
}

const TESTS: &[&str] = &[
    "native_artifact::launch::hosted::tests::exact_acquisition_limit_is_not_truncation",
    "native_artifact::launch::hosted::tests::payload_aggregate_cap_preserves_unicode_and_empty_arguments",
    "native_artifact::launch::hosted::tests::payload_aggregate_denial_stops_before_unused_later_arguments",
    "native_artifact::launch::hosted::tests::payload_backend_keeps_ordinary_indices_count_and_charge_lifetime",
    "native_artifact::launch::hosted::tests::payload_header_denial_precedes_iteration_and_limits_many_empty_arguments",
    "native_artifact::launch::hosted::tests::payload_memory_exact_and_one_short_include_capacity_and_vec_arc_peak",
    #[cfg(unix)]
    "native_artifact::launch::hosted::tests::payload_snapshot_does_not_decode_or_normalize_raw_os_arguments",
    "native_artifact::launch::hosted::tests::result_carriers_reject_spare_capacity_and_drop_before_live_claim",
];

fn suite_passed(stdout: &[u8]) -> bool {
    let Ok(stdout) = std::str::from_utf8(stdout) else {
        return false;
    };
    let passed = stdout
        .lines()
        .filter(|line| line.starts_with("test ") && line.ends_with(" ... ok"))
        .collect::<Vec<_>>();
    let summary = format!(
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured;",
        TESTS.len()
    );
    passed.len() == TESTS.len()
        && TESTS.iter().all(|name| {
            passed
                .iter()
                .filter(|line| **line == format!("test {name} ... ok"))
                .count()
                == 1
        })
        && stdout
            .lines()
            .filter(|line| line.starts_with(&summary))
            .count()
            == 1
}

pub(super) fn qualify(project: &Path, target: &Path, evidence: &Path, release: bool) -> Value {
    let workspace = workspace();
    let contract = evidence.join("launcher-contract");
    std::fs::create_dir(&contract).unwrap();
    let manifest = contract.join("Cargo.toml");
    std::fs::write(
        &manifest,
        contract_manifest(project, &workspace).to_string(),
    )
    .unwrap();
    std::fs::copy(
        project.with_file_name("Cargo.lock"),
        contract.join("Cargo.lock"),
    )
    .unwrap();
    let metadata = bounded(
        &mut CargoInvocation::resolve_lockfile(&manifest, true)
            .with_current_dir(&workspace)
            .command(),
        evidence,
        "compiled-launcher-metadata",
        120,
    );
    assert!(metadata.status.success());
    let metadata: Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let mut command = CargoInvocation::test_library(
        &manifest,
        PACKAGE,
        &["canonical-native-program-host-runtime"],
        target,
        release,
        true,
    )
    .unwrap()
    .with_current_dir(&workspace)
    .command();
    command
        .args(["--no-run", "--message-format=json-render-diagnostics"])
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_BUILD_JOBS", "1")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_DEBUG", "0");
    // Preserve the original compile and execution bounds for the actual owner.
    let compiled = bounded(&mut command, evidence, "compiled-launcher-compile", 1080);
    assert!(
        compiled.status.success(),
        "compiled launcher build: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let runtime_units = closure(&metadata, &compiled.stdout);
    let executables = compiled
        .stdout
        .split(|&byte| byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == TARGET
                && message["profile"]["test"] == true
        })
        .filter_map(|message| {
            assert_eq!(
                message["profile"]["opt_level"],
                if release { "3" } else { "0" }
            );
            assert_eq!(message["profile"]["debug_assertions"], !release);
            message["executable"].as_str().map(std::path::PathBuf::from)
        })
        .collect::<Vec<_>>();
    assert_eq!(executables.len(), 1, "one actual compiled library harness");
    let executable = &executables[0];
    assert!(executable.is_absolute() && executable.is_file());
    let executable_hash = hash(&std::fs::read(executable).unwrap());
    let mut tests = std::process::Command::new(executable);
    tests.current_dir(workspace).args([
        "native_artifact::launch::hosted::tests::",
        "--test-threads=1",
        "--color",
        "never",
    ]);
    let output = bounded(&mut tests, evidence, "compiled-launcher-tests", 300);
    assert!(
        output.status.success(),
        "compiled launcher tests: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        suite_passed(&output.stdout),
        "missing/filtered/duplicate launcher tests: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert_eq!(hash(&std::fs::read(executable).unwrap()), executable_hash);
    json!({"implementation":"compiled-launcher-library", "passed": TESTS.len(), "tests": TESTS,
        "runtime_units": runtime_units, "manifest_sha256": hash(&std::fs::read(manifest).unwrap()),
        "executable": executable, "executable_sha256": executable_hash,
        "compile_stdout_sha256": hash(&compiled.stdout), "compile_stderr_sha256": hash(&compiled.stderr),
        "stdout_sha256": hash(&output.stdout), "stderr_sha256": hash(&output.stderr)})
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn contract_reuses_runtime_resolution_and_workspace_test_profiles() {
        let temporary = tempfile::tempdir().unwrap();
        let project = temporary.path().join("Cargo.toml");
        let original = r#"[package]
name = "generated-application"
build = "build.rs"
[workspace]
[dependencies]
mech-gpu = { path = "/gpu", default-features = false, features = ["canonical-native-program-host-runtime"] }
[build-dependencies]
sha2 = "0.11"
[profile.release]
lto = "thin"
[patch.crates-io]
mech-math = { path = "/math" }
"#;
        std::fs::write(&project, original).unwrap();
        let manifest = contract_manifest(&project, &workspace());
        let before = original.parse::<toml_edit::DocumentMut>().unwrap();
        assert_eq!(
            manifest["dependencies"]["mech-gpu"].to_string(),
            before["dependencies"]["mech-gpu"].to_string()
        );
        assert_eq!(manifest["patch"].to_string(), before["patch"].to_string());
        assert!(manifest["package"].get("build").is_none());
        assert!(manifest.get("build-dependencies").is_none());
        assert!(manifest.get("dev-dependencies").is_none());
        let current = std::fs::read_to_string(workspace().join("Cargo.toml"))
            .unwrap()
            .parse::<toml_edit::DocumentMut>()
            .unwrap();
        assert_eq!(
            manifest["profile"].to_string(),
            current["profile"].to_string()
        );
        assert_eq!(
            manifest["dependencies"]["mech-terminal"]["features"]
                .as_array()
                .unwrap()
                .iter()
                .map(|feature| feature.as_str().unwrap())
                .collect::<Vec<_>>(),
            ["provider"]
        );
        assert!(
            manifest["dependencies"]["mech-terminal"]
                .get("optional")
                .is_none()
        );
        assert!(Path::new(manifest["lib"]["path"].as_str().unwrap()).is_file());
        assert_eq!(std::fs::read_to_string(project).unwrap(), original);
    }

    fn complete() -> String {
        let mut output = TESTS
            .iter()
            .map(|name| format!("test {name} ... ok\n"))
            .collect::<String>();
        output.push_str(&format!("test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s\n", TESTS.len()));
        output
    }

    #[test]
    fn compiled_suite_requires_every_named_test_and_a_nonempty_complete_summary() {
        let full = complete();
        assert!(suite_passed(full.as_bytes()));
        assert!(!suite_passed(
            b"test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 8 filtered out;"
        ));
        assert!(!suite_passed(
            full.replace(
                &format!("{} passed", TESTS.len()),
                &format!("{} passed", TESTS.len() - 1)
            )
            .as_bytes()
        ));
        // Other library contracts are intentionally outside this exact suite;
        // all eight required names must still execute once and pass.
        assert!(suite_passed(
            full.replace("0 filtered out", "478 filtered out")
                .as_bytes()
        ));
        assert!(!suite_passed(
            full.replace(&format!("test {} ... ok\n", TESTS[0]), "")
                .as_bytes()
        ));
        assert!(!suite_passed(
            format!("{full}test {} ... ok\n", TESTS[0]).as_bytes()
        ));
        assert!(!suite_passed(format!("{full}{full}").as_bytes()));
        assert!(!suite_passed(&[0xff]));
    }
}
