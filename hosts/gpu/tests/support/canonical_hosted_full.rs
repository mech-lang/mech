//! Optional full-input correctness and serial process-wall timing of SAME binaries.
use super::profile::Profile;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
#[path = "canonical_hosted_full_config.rs"]
mod config;
#[path = "canonical_hosted_full_process.rs"]
mod process;
#[path = "canonical_hosted_full_stages.rs"]
mod stages;

struct Binary {
    variant: u64,
    path: PathBuf,
    sha256: String,
    revision: String,
}
struct Inputs {
    input: process::Frozen,
    oracle: process::Frozen,
    launch: process::Frozen,
}
impl Inputs {
    fn check(&self) -> Result<(), String> {
        self.input.check()?;
        self.oracle.check()?;
        self.launch.check()
    }
}
fn binaries(results: &[Value]) -> Result<Vec<Binary>, String> {
    if results.len() != 2 {
        return Err("expected both already-qualified binaries".into());
    }
    results
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let path = PathBuf::from(row["binary"].as_str().ok_or("missing binary path")?);
            if row["variant"].as_u64() != Some(index as u64)
                || row["rust_profile"] != "release"
                || !path.is_absolute()
            {
                return Err("not exact original/decoded release binaries".into());
            }
            let sha256 = row["binary_sha256"]
                .as_str()
                .ok_or("missing binary pin")?
                .to_owned();
            let revision = row["program_revision"]
                .as_str()
                .ok_or("missing program revision")?
                .to_owned();
            if process::hash_file(&path)? != sha256 {
                return Err("qualified binary changed".into());
            }
            let mut permissions = fs::metadata(&path)
                .map_err(|e| e.to_string())?
                .permissions();
            permissions.set_readonly(true);
            fs::set_permissions(&path, permissions).map_err(|e| e.to_string())?;
            Ok(Binary {
                variant: index as u64,
                path,
                sha256,
                revision,
            })
        })
        .collect()
}
fn report_issues(report: &Value, revision: &str, launch: &str, input_cap: u64) -> Vec<String> {
    let mut issues = Vec::new();
    for (key, expected) in [
        ("schema", json!("mech.canonical-hosted-native-execution.v1")),
        (
            "route",
            json!("canonical-artifact-program-object-secured-cli"),
        ),
        ("policy", json!("secured")),
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
fn attempt(
    binary: &Binary,
    inputs: &Inputs,
    policy: &Value,
    directory: &Path,
    label: &str,
    deadline: u64,
    timing: bool,
) -> Value {
    let before = inputs.check().and_then(|_| {
        if process::hash_file(&binary.path)? != binary.sha256 {
            Err("binary changed before invocation".into())
        } else {
            Ok(())
        }
    });
    if let Err(error) = before {
        return json!({"variant":binary.variant, "label":label, "ok":false,
            "process_started":false, "driver_error":error});
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
            return json!({"variant":binary.variant, "label":label, "ok":false,
            "driver_error":error, "stdout_path":directory.join(format!("{label}.stdout")),
            "stderr_path":directory.join(format!("{label}.stderr")),
            "report_path":directory.join(format!("{label}.report.json"))});
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
            json!({"stdout_bytes":stdout_bytes, "stderr_bytes":stderr_bytes,
            "stdout_sha256":process::hash_file(&capture.stdout)?,
            "stderr_sha256":process::hash_file(&capture.stderr)?, "report":report,
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
    json!({"variant":binary.variant, "label":label, "ok":issues.is_empty(),
        "process_started":true, "exit":capture.exit, "deadline_exceeded":capture.timed_out,
        "status":capture.status,
        "issues":issues, "stdout_path":capture.stdout, "stderr_path":capture.stderr,
        "report_path":capture.report, "observed":observed, "binary":binary.path,
        "binary_sha256":binary.sha256, "elapsed_ns":if timing {Some(capture.elapsed_ns)} else {None}})
}
fn save(directory: &Path, value: &Value) {
    fs::write(
        directory.join("full-input-result.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn may_time(rows: &[Value]) -> bool {
    rows.len() == 2
        && rows
            .iter()
            .enumerate()
            .all(|(index, row)| row["variant"].as_u64() == Some(index as u64) && row["ok"] == true)
}
fn execute(
    profile: Profile,
    directory: &Path,
    results: &[Value],
    configuration: &std::ffi::OsStr,
    stop_on_failure: bool,
) -> Result<Value, String> {
    if !profile.release() {
        return Err("full-input driver requires actual release qualifier binaries".into());
    }
    let (config, bytes) =
        config::load(Some(configuration))?.ok_or("missing explicit full configuration")?;
    config::serial(config.samples, env::var_os(config::SERIAL).as_deref())?;
    fs::write(directory.join("full-input-config.json"), &bytes).map_err(|e| e.to_string())?;
    let owned = directory.join("owned-inputs");
    fs::create_dir(&owned).map_err(|e| e.to_string())?;
    let launch = process::freeze(&config.launch, &owned.join("launch.json"), 65536)?;
    let policy: Value =
        serde_json::from_slice(&process::small_file(&launch.path)?).map_err(|e| e.to_string())?;
    config::policy(&policy, &super::launch(), 0, 0)?;
    let input = process::freeze(
        &config.input,
        &owned.join("input"),
        policy["input_text_bytes"].as_u64().unwrap(),
    )?;
    let oracle = process::freeze(
        &config.oracle,
        &owned.join("oracle"),
        policy["output_text_bytes"].as_u64().unwrap(),
    )?;
    config::policy(&policy, &super::launch(), input.bytes, oracle.bytes)?;
    let inputs = Inputs {
        input,
        oracle,
        launch,
    };
    let binaries = binaries(results)?;
    let mut receipt = json!({
        "schema":"mech.canonical-hosted-full-input-result.v1", "ok":false, "qualified":false,
        "configuration_path":config.path, "configuration_sha256":super::hash(&bytes),
        "external_input_label":config.external_label, "external_label_is_size_authority":false,
        "input":inputs.input.evidence(&config.input.path), "oracle":inputs.oracle.evidence(&config.oracle.path),
        "launch":inputs.launch.evidence(&config.launch.path), "finite_scale_policy":policy,
        "small_gate_policy_unchanged":true, "rust_profile":"release",
        "object_optimization":"none", "whole_program_optimized":false,
        "complete_source_sha256":super::hash(super::FULL.as_bytes()),
        "qualification":[], "samples":[], "timing_samples":0, "timing_attempts":0,
        "timed_process_starts_confirmed":0, "timed_process_starts_unknown":0,
        "successful_timed_samples":0,
        "timing_samples_definition":"published sample count only after the complete requested run qualifies; attempts/individual successes remain recorded on failure",
        "requested_samples_per_variant":config.samples, "deadline_seconds":config.deadline_seconds,
        "timing_scope":"serial spawn-through-observed-child-exit wall time incl CLI acquisition/output/report;1ms poll; excludes build, hashing and parent validation; not kernel timing",
        "cache_state":"integrity prehash and mandatory correctness runs deliberately warm executable/input filesystem caches; warm-cache fresh-process end-to-end wall time, not cold-cache or kernel timing",
        "no_overlap":"all owned build/pin-mutation jobs completed before this driver; samples require explicit operator SERIAL=1 attestation, not an ambient process-scan claim",
    });
    let mut qualified = Vec::new();
    for binary in &binaries {
        qualified.push(attempt(
            binary,
            &inputs,
            &policy,
            directory,
            &format!("qualify-{}", binary.variant),
            config.deadline_seconds,
            false,
        ));
        receipt["qualification"] = json!(qualified);
        save(directory, &receipt); // Real exit/typed denial is durable BEFORE fail.
        if stop_on_failure && qualified.last().is_some_and(|row| row["ok"] != true) {
            receipt["qualification_passed"] = json!(false);
            save(directory, &receipt);
            return Ok(receipt);
        }
    }
    receipt["qualification_passed"] = json!(may_time(&qualified));
    if !may_time(&qualified) {
        save(directory, &receipt);
        return Ok(receipt);
    }
    let mut samples = Vec::new();
    for round in 0..config.samples {
        for binary in &binaries {
            let row = attempt(
                binary,
                &inputs,
                &policy,
                directory,
                &format!("sample-{round}-{}", binary.variant),
                config.deadline_seconds,
                true,
            );
            let ok = row["ok"] == true;
            samples.push(row);
            receipt["samples"] = json!(samples);
            receipt["timing_attempts"] = json!(samples.len());
            receipt["timed_process_starts_confirmed"] = json!(
                samples
                    .iter()
                    .filter(|row| row["process_started"] == true)
                    .count()
            );
            receipt["timed_process_starts_unknown"] = json!(
                samples
                    .iter()
                    .filter(|row| row["process_started"].is_null())
                    .count()
            );
            receipt["successful_timed_samples"] =
                json!(samples.iter().filter(|row| row["ok"] == true).count());
            save(directory, &receipt);
            if !ok {
                return Ok(receipt);
            }
        }
    }
    receipt["ok"] = json!(true);
    receipt["qualified"] = json!(true);
    receipt["timing_samples"] = json!(samples.len());
    receipt["timing_summary"] = json!(
        binaries
            .iter()
            .map(|binary| {
                let mut ns = samples
                    .iter()
                    .filter(|row| row["variant"] == binary.variant)
                    .map(|row| row["elapsed_ns"].as_u64().unwrap())
                    .collect::<Vec<_>>();
                ns.sort_unstable();
                json!({"variant":binary.variant, "samples":ns.len(), "min_ns":ns.first(),
            "median_upper_ns":ns.get(ns.len()/2), "max_ns":ns.last()})
            })
            .collect::<Vec<_>>()
    );
    save(directory, &receipt);
    Ok(receipt)
}
/// Called only after BOTH builds and ALL pin-mutation/recovery builds ended.
pub(super) fn run(profile: Profile, evidence: &Path, results: &[Value]) -> Value {
    if let Some(receipt) = stages::run_if_configured(profile, evidence, results) {
        return receipt;
    }
    let Some(configuration) = env::var_os(config::ENV) else {
        return Value::Null;
    };
    let directory = evidence.join("full-input");
    fs::create_dir(&directory).unwrap();
    fs::write(
        directory.join("release-qualifier-results.json"),
        serde_json::to_vec_pretty(results).unwrap(),
    )
    .unwrap();
    let receipt = execute(profile, &directory, results, &configuration, false).unwrap_or_else(|error| {
        json!({"schema":"mech.canonical-hosted-full-input-result.v1", "ok":false, "qualified":false,
            "preflight_error":error, "qualification":[], "samples":[], "timing_samples":0,
            "timing_attempts":0, "timed_process_starts_confirmed":0,
            "timed_process_starts_unknown":0, "successful_timed_samples":0})
    });
    save(&directory, &receipt);
    assert_eq!(
        receipt["ok"],
        true,
        "full-input qualification/timing failed; see {}",
        directory.display()
    );
    receipt
}

#[cfg(test)]
#[path = "canonical_hosted_full_tests.rs"]
mod tests;
