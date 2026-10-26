//! Qualify the compiled launcher owner used by the generated application.
use super::process::{bounded, hash, workspace};
use mech_build::CargoInvocation;
use serde_json::{Value, json};
use std::path::Path;

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

pub(super) fn qualify(target: &Path, evidence: &Path, release: bool) -> Value {
    let workspace = workspace();
    let mut command = CargoInvocation::test_library(
        workspace.join("Cargo.toml"),
        "mech-gpu",
        &["canonical-native-program-host"],
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
    // A cold library build includes compiler dev-dependencies. Give compilation
    // the same bound as the hosted harness build, then execute its actual saved
    // library executable under the separate, unchanged test deadline.
    let compiled = bounded(&mut command, evidence, "compiled-launcher-compile", 1080);
    assert!(
        compiled.status.success(),
        "compiled launcher build: {}",
        String::from_utf8_lossy(&compiled.stderr)
    );
    let executables = compiled
        .stdout
        .split(|&byte| byte == b'\n')
        .filter_map(|line| serde_json::from_slice::<Value>(line).ok())
        .filter(|message| {
            message["reason"] == "compiler-artifact"
                && message["target"]["name"] == "mech_gpu"
                && message["profile"]["test"] == true
        })
        .filter_map(|message| message["executable"].as_str().map(std::path::PathBuf::from))
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
        "executable": executable, "executable_sha256": executable_hash,
        "compile_stdout_sha256": hash(&compiled.stdout), "compile_stderr_sha256": hash(&compiled.stderr),
        "stdout_sha256": hash(&output.stdout), "stderr_sha256": hash(&output.stderr)})
}

#[cfg(test)]
mod tests {
    use super::*;

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
