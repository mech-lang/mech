//! Actual producer's failure receipts, never native execution/benchmark credit.
use super::*;

fn assert_keys(value: &Value, expected: &[&str]) {
    let mut actual = value
        .as_object()
        .unwrap()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    let mut expected = expected.to_vec();
    actual.sort_unstable();
    expected.sort_unstable();
    assert_eq!(actual, expected);
}

fn failed_receipt(set: RunSet) -> (tempfile::TempDir, Value) {
    let temporary = tempfile::tempdir().unwrap();
    let supplied = temporary.path().join("supplied");
    fs::create_dir(&supplied).unwrap();
    let spec = |name: &str, bytes: &[u8]| {
        let path = supplied.join(name);
        fs::write(&path, bytes).unwrap();
        json!({"path":path,"sha256":super::super::hash(bytes)})
    };
    let configuration = json!({
        "schema":"mech.canonical-hosted-full-input-config.v1",
        "input":spec("input", b"GGTA"),"oracle":spec("oracle", b"0"),
        "launch":spec("launch.json", &serde_json::to_vec(&super::super::launch()).unwrap()),
        "deadline_seconds":1,"samples":0,"external_label":"failure-receipt unit fixture"
    });
    let config = supplied.join("configuration.json");
    fs::write(&config, serde_json::to_vec(&configuration).unwrap()).unwrap();
    // Not executable: the real invoke boundary fails without starting native
    // code. Synthetic qualifier metadata is ONLY for this negative unit case.
    let binary = supplied.join("not-an-executable");
    fs::write(&binary, b"not executable").unwrap();
    let results = (0..2)
        .map(|variant| {
            json!({"variant":variant,"binary":binary,
            "binary_sha256":process::hash_file(&binary).unwrap(),"program_revision":"unit",
            "rust_profile":"release","source_sha256":"unit","bytecode_sha256":"unit",
            "object_sha256":"unit","identity_sha256":"unit"})
        })
        .collect::<Vec<_>>();
    let directory = temporary.path().join("run");
    fs::create_dir(&directory).unwrap();
    let receipt = execute(
        Profile::Release,
        &directory,
        &results,
        config.as_os_str(),
        false,
        set,
    )
    .unwrap();
    assert_eq!(receipt["ok"], false);
    assert_eq!(receipt["timing_attempts"], 0);
    assert_eq!(receipt["timing_samples"], 0);
    (temporary, receipt)
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn review_regression_derived_launch_requested_path_authenticates_its_actual_bytes() {
    let (_temporary, receipt) = failed_receipt(RunSet::BenchmarkComparison);
    for item in receipt["policy_launches"].as_array().unwrap() {
        let launch = &item["launch"];
        let requested = Path::new(launch["requested_path"].as_str().unwrap());
        assert_eq!(process::hash_file(requested).unwrap(), launch["sha256"]);
        assert_eq!(item["base_launch"], receipt["launch"]);
        assert_ne!(
            launch["requested_path"],
            receipt["launch"]["requested_path"]
        );
        let mut derived: Value = serde_json::from_slice(&fs::read(requested).unwrap()).unwrap();
        assert_eq!(
            derived.as_object_mut().unwrap().remove("benchmark_policy"),
            Some(item["policy"].clone())
        );
        assert_eq!(derived, receipt["finite_scale_policy"]);
    }
}

#[test]
fn review_regression_production_v1_failure_receipt_has_no_comparison_fields() {
    let (_temporary, receipt) = failed_receipt(RunSet::Production);
    assert_eq!(
        receipt["schema"],
        "mech.canonical-hosted-full-input-result.v1"
    );
    for field in [
        "policy_launches",
        "expected_qualification_trials",
        "requested_samples_per_variant_and_policy",
        "qualification_policy_order",
        "timing_policy_order",
    ] {
        assert!(receipt.get(field).is_none(), "production v1 gained {field}");
    }
    assert_eq!(receipt["qualification"].as_array().unwrap().len(), 2);
    assert_keys(
        &receipt,
        &[
            "schema",
            "ok",
            "qualified",
            "configuration_path",
            "configuration_sha256",
            "external_input_label",
            "external_label_is_size_authority",
            "input",
            "oracle",
            "launch",
            "finite_scale_policy",
            "small_gate_policy_unchanged",
            "rust_profile",
            "object_optimization",
            "whole_program_optimized",
            "complete_source_sha256",
            "qualification",
            "samples",
            "timing_samples",
            "timing_attempts",
            "timed_process_starts_confirmed",
            "timed_process_starts_unknown",
            "successful_timed_samples",
            "timing_samples_definition",
            "requested_samples_per_variant",
            "deadline_seconds",
            "timing_scope",
            "cache_state",
            "no_overlap",
            "qualification_passed",
        ],
    );
    for row in receipt["qualification"].as_array().unwrap() {
        assert!(row.get("policy").is_none());
        assert!(row.get("route").is_none());
        assert_keys(
            row,
            &[
                "variant",
                "label",
                "ok",
                "driver_error",
                "stdout_path",
                "stderr_path",
                "report_path",
            ],
        );
    }
}

#[test]
fn production_v1_timing_summary_preserves_exact_keys_and_upper_median() {
    let samples = [
        json!({"variant":0,"elapsed_ns":9}),
        json!({"variant":1,"elapsed_ns":7}),
        json!({"variant":0,"elapsed_ns":3}),
        json!({"variant":1,"elapsed_ns":1}),
    ];
    for (variant, minimum, median) in [(0, 3, 9), (1, 1, 7)] {
        assert_eq!(
            RunSet::Production.summaries(variant, &samples),
            vec![
                json!({"variant":variant,"samples":2,"min_ns":minimum,"median_upper_ns":median,"max_ns":median})
            ]
        );
    }
    assert_eq!(
        RunSet::Production.summaries(0, &[]),
        vec![json!({"variant":0,"samples":0,"min_ns":null,"median_upper_ns":null,"max_ns":null})]
    );
}

#[test]
fn production_v1_annotation_leaves_all_attempt_row_forms_unchanged() {
    let rows = [
        json!({"variant":0,"label":"qualify-0","ok":false,"process_started":false,"driver_error":"before"}),
        json!({"variant":0,"label":"qualify-0","ok":false,"driver_error":"spawn", "stdout_path":"out","stderr_path":"err","report_path":"report"}),
        json!({"variant":0,"label":"sample-0-0","ok":true,"process_started":true,"exit":0,"deadline_exceeded":false,
            "status":"success","issues":[],"stdout_path":"out","stderr_path":"err","report_path":"report",
            "observed":{},"binary":"binary","binary_sha256":"pin","elapsed_ns":1}),
    ];
    for row in rows {
        assert_eq!(TrialPolicy::ProductionSecured.annotate(row.clone()), row);
    }
}

#[cfg(feature = "canonical-native-benchmark-policy")]
#[test]
fn comparison_timing_summaries_keep_exact_variant_policy_partitions() {
    let samples = [
        json!({"variant":0,"policy":"secured","elapsed_ns":9}),
        json!({"variant":0,"policy":"trusted","elapsed_ns":2}),
        json!({"variant":1,"policy":"secured","elapsed_ns":99}),
        json!({"variant":0,"policy":"secured","elapsed_ns":3}),
    ];
    let summaries = RunSet::BenchmarkComparison.summaries(0, &samples);
    assert_eq!(
        summaries,
        vec![
            json!({"variant":0,"policy":"secured","route":TrialPolicy::BenchmarkSecured.route(),
            "samples":2,"min_ns":3,"median_upper_ns":9,"max_ns":9}),
            json!({"variant":0,"policy":"trusted","route":TrialPolicy::BenchmarkTrusted.route(),
            "samples":1,"min_ns":2,"median_upper_ns":2,"max_ns":2})
        ]
    );
}
