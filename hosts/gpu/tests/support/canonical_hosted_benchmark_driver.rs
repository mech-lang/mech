//! Four finite current-candidate executions; performance orchestration lives in benchmarks.
use super::*;

pub(super) fn qualify(
    evidence: &Path,
    results: &[Value],
    profile: profile::Profile,
    commit: &str,
) -> Value {
    if !profile.release() {
        return Value::Null;
    }
    assert_eq!(results.len(), 2);
    for field in [
        "binary_sha256",
        "source_sha256",
        "bytecode_sha256",
        "object_sha256",
        "identity_sha256",
    ] {
        assert!(results[0][field].as_str().is_some());
        assert_eq!(results[0][field], results[1][field]);
    }
    let directory = evidence.join("driver-protocol-control");
    fs::create_dir(&directory).unwrap();
    let input = directory.join("input");
    fs::write(&input, GGTA_INPUT).unwrap();
    let mut receipt = json!({"schema":"mech.current-policy-control.v1", "qualified":false,
        "qualification":[], "candidate_mech_revision":commit, "runner_mech_revision":commit, "expected_qualification_trials":4, "samples":[], "timing_samples":0});
    let receipt_path = directory.join("result.json");
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    let mut rows = Vec::new();
    for (variant, result) in results.iter().enumerate() {
        assert_eq!(result["variant"], variant);
        assert_eq!(result["rust_profile"], "release");
        let binary = Path::new(result["binary"].as_str().unwrap());
        assert!(binary.is_absolute());
        for mode in ["secured", "trusted"] {
            let trial = directory.join(format!("{variant}-{mode}"));
            fs::create_dir(&trial).unwrap();
            let mut policy = launch();
            policy["benchmark_policy"] = json!(mode);
            let launch = trial.join("launch.json");
            let report_path = trial.join("report.json");
            let bytes = serde_json::to_vec_pretty(&policy).unwrap();
            fs::write(&launch, &bytes).unwrap();
            assert_eq!(pin(binary), result["binary_sha256"]);
            assert_eq!(fs::read(&input).unwrap(), GGTA_INPUT);
            let mut command = Command::new(binary);
            command.arg(&launch).arg(&report_path).arg("--");
            let output = with_input(&mut command, &trial, "run", &input);
            let report: Value = fs::read(&report_path)
                .ok()
                .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                .unwrap_or(Value::Null);
            let mut ok = output.status.success()
                && output.stdout == GGTA_OUTPUT.as_bytes()
                && output.stderr.is_empty();
            for (field, expected) in [
                ("schema", json!("mech.canonical-hosted-native-execution.v1")),
                (
                    "route",
                    json!("canonical-artifact-program-object-benchmark-cli"),
                ),
                ("policy", json!(mode)),
                ("program_revision", result["program_revision"].clone()),
                ("launch_sha256", json!(hash(&bytes))),
                ("native_aot", json!(true)),
                ("hosted_phases", json!("admit-activate-capture-resume")),
                ("source_parser", json!(false)),
                ("jit", json!(false)),
                ("resident_body_evaluation", json!(false)),
                ("shutdown", json!("completed")),
                ("requirements", json!(2)),
                ("observations", json!(1)),
                ("effects", json!(1)),
                (
                    "input_acquisition_byte_cap",
                    policy["input_text_bytes"].clone(),
                ),
            ] {
                ok &= report[field] == expected;
            }
            ok &= pin(binary) == result["binary_sha256"]
                && fs::read(&input).unwrap() == GGTA_INPUT
                && fs::read(&launch).unwrap() == bytes;
            rows.push(json!({"variant":variant,"policy":mode,"ok":ok,"report":report,
                "exit":output.status.code(),"stdout_sha256":hash(&output.stdout),"stderr_sha256":hash(&output.stderr)}));
            receipt["qualification"] = json!(rows);
            fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
            assert!(
                ok,
                "current policy control failed; see {}",
                receipt_path.display()
            );
        }
    }
    assert_eq!(rows.len(), 4);
    for field in [
        "native_work",
        "native_comparison_work",
        "native_visits",
        "native_transport_bytes",
        "host_result_carrier_bytes",
    ] {
        let expected = rows[0]["report"][field].as_u64().unwrap();
        assert!(
            rows.iter()
                .all(|row| row["report"][field].as_u64() == Some(expected)),
            "four-way accounting differs: {field}"
        );
    }
    receipt["qualified"] = json!(true);
    fs::write(&receipt_path, serde_json::to_vec_pretty(&receipt).unwrap()).unwrap();
    receipt
}
