//! One shared actual-process and exact-output boundary for every driver profile.
use super::{Binary, Inputs, Path, TrialPolicy, Value, fs, json, process};

pub(super) fn report_issues(
    report: &Value,
    revision: &str,
    launch: &str,
    input_cap: u64,
    mode: TrialPolicy,
) -> Vec<String> {
    let mut issues = Vec::new();
    for (key, expected) in [
        ("schema", json!("mech.canonical-hosted-native-execution.v1")),
        ("route", json!(mode.route())),
        ("policy", json!(mode.name())),
        ("program_revision", json!(revision)),
        ("launch_sha256", json!(launch)),
        ("native_aot", json!(true)),
        ("hosted_phases", json!("admit-activate-capture-resume")),
        ("source_parser", json!(false)),
        ("jit", json!(false)),
        ("resident_body_evaluation", json!(false)),
        ("shutdown", json!("completed")),
        ("requirements", json!(2)),
        ("observations", json!(1)),
        ("effects", json!(1)),
        ("input_acquisition_byte_cap", json!(input_cap)),
    ] {
        if report[key] != expected {
            issues.push(format!("report {key} differs"));
        }
    }
    for key in [
        "native_work",
        "native_comparison_work",
        "native_visits",
        "native_transport_bytes",
        "host_result_carrier_bytes",
    ] {
        if report[key].as_u64().is_none() {
            issues.push(format!("report {key} missing"));
        }
    }
    issues
}

pub(super) fn attempt(
    binary: &Binary,
    inputs: &Inputs,
    policy: &Value,
    directory: &Path,
    label: &str,
    deadline: u64,
    timing: bool,
    mode: TrialPolicy,
) -> Value {
    let before = inputs.check().and_then(|_| {
        if process::hash_file(&binary.path)? != binary.sha256 {
            Err("binary changed before invocation".into())
        } else {
            Ok(())
        }
    });
    if let Err(error) = before {
        return mode.annotate(json!({"variant":binary.variant,
            "label":label,"ok":false,"process_started":false,"driver_error":error}));
    }
    let capture = match process::invoke(
        &binary.path,
        &inputs.launch.path,
        &inputs.input.path,
        directory,
        label,
        deadline,
    ) {
        Ok(capture) => capture,
        Err(error) => {
            return mode.annotate(json!({"variant":binary.variant,
            "label":label,"ok":false,"driver_error":error,
            "stdout_path":directory.join(format!("{label}.stdout")),
            "stderr_path":directory.join(format!("{label}.stderr")),
            "report_path":directory.join(format!("{label}.report.json"))}));
        }
    };
    let mut issues = Vec::new();
    if !capture.success {
        issues.push("process did not succeed".into());
    }
    if capture.timed_out {
        issues.push("process deadline exceeded".into());
    }
    let observed = (|| -> Result<Value, String> {
        let stdout_bytes = fs::metadata(&capture.stdout)
            .map_err(|e| e.to_string())?
            .len();
        let stderr_bytes = fs::metadata(&capture.stderr)
            .map_err(|e| e.to_string())?
            .len();
        if !process::equal_files(&capture.stdout, &inputs.oracle.path)? {
            issues.push("stdout differs from exact oracle".into());
        }
        if stderr_bytes != 0 {
            issues.push("stderr is not empty".into());
        }
        let report = if capture.report.exists() {
            match serde_json::from_slice::<Value>(&process::small_file(&capture.report)?) {
                Ok(report) => {
                    issues.extend(report_issues(
                        &report,
                        &binary.revision,
                        &inputs.launch.sha256,
                        policy["input_text_bytes"].as_u64().unwrap(),
                        mode,
                    ));
                    report
                }
                Err(error) => {
                    issues.push(format!("report JSON: {error}"));
                    Value::Null
                }
            }
        } else {
            issues.push("no success report published".into());
            Value::Null
        };
        inputs.check()?;
        if process::hash_file(&binary.path)? != binary.sha256 {
            return Err("binary changed during invocation".into());
        }
        Ok(
            json!({"stdout_bytes":stdout_bytes,"stderr_bytes":stderr_bytes,
            "stdout_sha256":process::hash_file(&capture.stdout)?,
            "stderr_sha256":process::hash_file(&capture.stderr)?,"report":report,
            "report_sha256":if capture.report.exists() {Some(process::hash_file(&capture.report)?)} else {None}}),
        )
    })();
    let observed = match observed {
        Ok(value) => value,
        Err(error) => {
            issues.push(error);
            Value::Null
        }
    };
    mode.annotate(json!({"variant":binary.variant,
        "label":label,"ok":issues.is_empty(),"process_started":true,"exit":capture.exit,
        "deadline_exceeded":capture.timed_out,"status":capture.status,"issues":issues,
        "stdout_path":capture.stdout,"stderr_path":capture.stderr,"report_path":capture.report,
        "observed":observed,"binary":binary.path,"binary_sha256":binary.sha256,
        "elapsed_ns":if timing {Some(capture.elapsed_ns)} else {None}}))
}
