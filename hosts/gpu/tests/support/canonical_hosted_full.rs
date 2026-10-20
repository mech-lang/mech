//! Optional full-input correctness and serial process-wall timing of SAME binaries.
use super::profile::Profile;
use serde_json::{Value, json};
use std::{
    env, fs,
    path::{Path, PathBuf},
};
#[path = "canonical_hosted_full_config.rs"]
mod config;
#[path = "canonical_hosted_full_attempt.rs"]
mod execution;
#[path = "canonical_hosted_full_policy.rs"]
mod policy;
#[path = "canonical_hosted_full_process.rs"]
mod process;
#[path = "canonical_hosted_full_stages.rs"]
mod stages;
use execution::{attempt, report_issues};
use policy::{RunSet, TrialPolicy};

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
fn binaries(results: &[Value], set: RunSet) -> Result<Vec<Binary>, String> {
    if results.len() != 2 {
        return Err("expected both already-qualified binaries".into());
    }
    if set != RunSet::Production {
        for field in [
            "binary_sha256",
            "source_sha256",
            "bytecode_sha256",
            "object_sha256",
            "identity_sha256",
        ] {
            if results[0][field].as_str().is_none() || results[0][field] != results[1][field] {
                return Err(format!(
                    "comparison requires the SAME original/decoded {field}"
                ));
            }
        }
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
fn save(directory: &Path, value: &Value) {
    fs::write(
        directory.join("full-input-result.json"),
        serde_json::to_vec_pretty(value).unwrap(),
    )
    .unwrap();
}
fn may_time(rows: &[Value], set: RunSet) -> bool {
    set.qualifies(rows)
}
fn execute(
    profile: Profile,
    directory: &Path,
    results: &[Value],
    configuration: &std::ffi::OsStr,
    stop_on_failure: bool,
    set: RunSet,
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
    let policy = policy::decode(&process::small_file(&launch.path)?)?;
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
    let binaries = binaries(results, set)?;
    let launches = set.launches(&inputs, &policy, &owned)?;
    let stop_on_failure = stop_on_failure || set != RunSet::Production;
    let mut receipt = json!({
        "schema":set.result_schema(), "ok":false, "qualified":false,
        "configuration_path":config.path, "configuration_sha256":super::hash(&bytes),
        "external_input_label":config.external_label, "external_label_is_size_authority":false,
        "input":inputs.input.evidence(&config.input.path), "oracle":inputs.oracle.evidence(&config.oracle.path),
        "launch":inputs.launch.evidence(&config.launch.path), "finite_scale_policy":policy,
        "small_gate_policy_unchanged":true, "rust_profile":"release",
        "object_optimization":"speed", "whole_program_optimized":false,
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
    if set != RunSet::Production {
        receipt["policy_launches"] = json!(
            launches
                .iter()
                .map(|(mode, trial)| json!({
            "policy":mode.name(),"route":mode.route(),
            "base_launch":inputs.launch.evidence(&config.launch.path),
            "launch":trial.launch.evidence(&trial.launch.path)}))
                .collect::<Vec<_>>()
        );
        receipt["expected_qualification_trials"] = json!(set.trials());
        receipt["requested_samples_per_variant_and_policy"] = json!(config.samples);
        receipt["qualification_policy_order"] = json!(
            set.modes()
                .iter()
                .map(|mode| mode.name())
                .collect::<Vec<_>>()
        );
        receipt["timing_policy_order"] =
            json!("selected policy order reverses on alternate rounds");
    }
    let mut qualified = Vec::new();
    for binary in &binaries {
        for (mode, inputs) in &launches {
            qualified.push(attempt(
                binary,
                inputs,
                &policy,
                directory,
                &mode.label(&format!("qualify-{}", binary.variant)),
                config.deadline_seconds,
                false,
                *mode,
            ));
            receipt["qualification"] = json!(qualified);
            save(directory, &receipt); // Actual exit/typed denial is durable BEFORE fail.
            if stop_on_failure && qualified.last().is_some_and(|row| row["ok"] != true) {
                receipt["qualification_passed"] = json!(false);
                save(directory, &receipt);
                return Ok(receipt);
            }
        }
    }
    receipt["qualification_passed"] = json!(may_time(&qualified, set));
    if !may_time(&qualified, set) {
        if set != RunSet::Production && qualified.iter().all(|row| row["ok"] == true) {
            receipt["qualification_error"] =
                json!("variant/policy coverage or native accounting differs");
        }
        save(directory, &receipt);
        return Ok(receipt);
    }
    let mut samples = Vec::new();
    for round in 0..config.samples {
        for binary in &binaries {
            for offset in 0..launches.len() {
                let index = set.sample_slot(round, offset);
                let (mode, inputs) = &launches[index];
                let row = attempt(
                    binary,
                    inputs,
                    &policy,
                    directory,
                    &mode.label(&format!("sample-{round}-{}", binary.variant)),
                    config.deadline_seconds,
                    true,
                    *mode,
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
    }
    receipt["ok"] = json!(true);
    receipt["qualified"] = json!(true);
    receipt["timing_samples"] = json!(samples.len());
    receipt["timing_summary"] = json!(
        binaries
            .iter()
            .flat_map(|binary| set.summaries(binary.variant, &samples))
            .collect::<Vec<_>>()
    );
    save(directory, &receipt);
    Ok(receipt)
}
/// Called only after BOTH builds and ALL pin-mutation/recovery builds ended.
pub(super) fn run(profile: Profile, evidence: &Path, results: &[Value]) -> Value {
    run_for(profile, evidence, results, RunSet::Production)
}
#[cfg(feature = "canonical-native-benchmark-policy")]
pub(super) fn run_comparison(profile: Profile, evidence: &Path, results: &[Value]) -> Value {
    run_for(profile, evidence, results, RunSet::BenchmarkComparison)
}
#[cfg(feature = "canonical-native-benchmark-policy")]
pub(super) fn comparison_control(
    profile: Profile,
    directory: &Path,
    results: &[Value],
    configuration: &std::ffi::OsStr,
) -> Result<Value, String> {
    execute(
        profile,
        directory,
        results,
        configuration,
        true,
        RunSet::BenchmarkComparison,
    )
}
fn run_for(profile: Profile, evidence: &Path, results: &[Value], set: RunSet) -> Value {
    if let Some(receipt) = stages::run_if_configured(profile, evidence, results, set) {
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
    let receipt =
        execute(profile, &directory, results, &configuration, false, set).unwrap_or_else(|error| {
            json!({"schema":set.result_schema(), "ok":false, "qualified":false,
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

#[cfg(test)]
#[path = "canonical_hosted_full_receipt_tests.rs"]
mod receipt_tests;
