//! Qualify the actual generated Rust template, not a copied helper harness.
use super::process::{bounded, hash};
use mech_build::CargoInvocation;
use serde_json::{Value, json};
use std::path::Path;

const TESTS: &[&str] = &[
    "tests::exact_acquisition_limit_is_not_truncation",
    "tests::payload_aggregate_cap_preserves_unicode_and_empty_arguments",
    "tests::payload_aggregate_denial_stops_before_unused_later_arguments",
    "tests::payload_backend_keeps_ordinary_indices_count_and_charge_lifetime",
    "tests::payload_header_denial_precedes_iteration_and_limits_many_empty_arguments",
    "tests::payload_memory_exact_and_one_short_include_capacity_and_vec_arc_peak",
    #[cfg(unix)]
    "tests::payload_snapshot_does_not_decode_or_normalize_raw_os_arguments",
    "tests::result_carriers_reject_spare_capacity_and_drop_before_live_claim",
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
        "test result: ok. {} passed; 0 failed; 0 ignored; 0 measured; 0 filtered out;",
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

pub(super) fn qualify(
    manifest: &Path,
    binary: &str,
    target: &Path,
    evidence: &Path,
    release: bool,
) -> Value {
    let isolated = manifest
        .ancestors()
        .last()
        .expect("absolute generated manifest");
    let mut command = CargoInvocation::test_binary(manifest, binary, None, target, release, true)
        .unwrap()
        .with_current_dir(isolated)
        .command();
    command
        .args(["--", "--test-threads=1", "--color", "never"])
        .env("CARGO_INCREMENTAL", "0")
        .env("CARGO_BUILD_JOBS", "1")
        .env("CARGO_PROFILE_DEV_DEBUG", "0")
        .env("CARGO_PROFILE_TEST_DEBUG", "0");
    let output = bounded(&mut command, evidence, "generated-template-tests", 300);
    assert!(
        output.status.success(),
        "generated template tests: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(
        suite_passed(&output.stdout),
        "missing/filtered/duplicate generated tests: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    json!({"passed": TESTS.len(), "tests": TESTS,
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
    fn actual_template_suite_requires_every_named_test_and_a_nonempty_complete_summary() {
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
        assert!(!suite_passed(
            full.replace("0 filtered out", "1 filtered out").as_bytes()
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
