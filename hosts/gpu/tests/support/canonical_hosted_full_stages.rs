//! Optional bounded stages around the SAME already-built full-input runner.
use super::{Path, PathBuf, Profile, RunSet, Value, config, env, fs, json, process};
#[path = "canonical_hosted_full_stages_config.rs"]
mod staged_config;

struct Prepared {
    name: String,
    requested: PathBuf,
    directory: PathBuf,
    configuration: process::Frozen,
}

fn save(directory: &Path, receipt: &Value) {
    fs::write(
        directory.join("staged-input-result.json"),
        serde_json::to_vec_pretty(receipt).unwrap(),
    )
    .unwrap();
}

fn refresh(receipt: &mut Value) {
    let stages = receipt["stages"].as_array().unwrap();
    let attempted = stages
        .iter()
        .filter(|stage| {
            ["running", "passed", "failed"]
                .iter()
                .any(|status| stage["status"] == *status)
        })
        .count();
    let running = stages
        .iter()
        .filter(|stage| stage["status"] == "running")
        .count();
    let passed = stages
        .iter()
        .filter(|stage| stage["status"] == "passed")
        .count();
    let failed = stages
        .iter()
        .filter(|stage| stage["status"] == "failed")
        .count();
    let skipped = stages
        .iter()
        .filter(|stage| stage["status"] == "skipped")
        .count();
    let mut counts = [[0usize; 4]; 2]; // attempts, confirmed starts, unknown starts, successes
    for stage in stages {
        for (i, key) in ["qualification", "samples"].into_iter().enumerate() {
            if let Some(rows) = stage["result"][key].as_array() {
                for row in rows {
                    counts[i][0] += 1;
                    counts[i][1] += usize::from(row["process_started"] == true);
                    counts[i][2] += usize::from(row["process_started"].is_null());
                    counts[i][3] += usize::from(row["ok"] == true);
                }
            }
        }
    }
    let all = !stages.is_empty() && passed == stages.len();
    receipt["ok"] = json!(all);
    receipt["qualified"] = json!(all);
    receipt["stages_attempted"] = json!(attempted);
    receipt["running_stages"] = json!(running);
    receipt["qualified_stages"] = json!(passed);
    receipt["failed_stages"] = json!(failed);
    receipt["skipped_stages"] = json!(skipped);
    receipt["qualification_attempts"] = json!(counts[0][0]);
    receipt["qualification_process_starts_confirmed"] = json!(counts[0][1]);
    receipt["qualification_process_starts_unknown"] = json!(counts[0][2]);
    receipt["successful_qualification_attempts"] = json!(counts[0][3]);
    receipt["timing_attempts"] = json!(counts[1][0]);
    receipt["timed_process_starts_confirmed"] = json!(counts[1][1]);
    receipt["timed_process_starts_unknown"] = json!(counts[1][2]);
    receipt["successful_timed_samples"] = json!(counts[1][3]);
    receipt["timing_samples"] = json!(if all { counts[1][3] } else { 0 });
}

fn execute(
    profile: Profile,
    directory: &Path,
    results: &[Value],
    configuration: &std::ffi::OsStr,
    single: Option<&std::ffi::OsStr>,
    receipt: &mut Value,
    set: RunSet,
) -> Result<(), String> {
    staged_config::exclusive(single, Some(configuration))?;
    if !profile.release() {
        return Err("staged input requires actual release qualifier binaries".into());
    }
    let config = staged_config::load(configuration)?;
    fs::write(directory.join("staged-input-config.json"), &config.bytes)
        .map_err(|e| e.to_string())?;
    receipt["configuration_path"] = json!(config.path);
    receipt["configuration_sha256"] = json!(super::super::hash(&config.bytes));
    receipt["requested_stages"] = json!(config.stages.len());
    receipt["stages"] =
        json!(config.stages.iter().map(|stage| json!({
        "name":stage.name, "status":"pending", "configuration":{
            "requested_path":stage.configuration.path, "sha256":stage.configuration.sha256}
    })).collect::<Vec<_>>());
    save(directory, receipt);
    staged_config::distinct_resolved_paths(
        config
            .stages
            .iter()
            .map(|stage| fs::canonicalize(&stage.configuration.path).map_err(|e| e.to_string()))
            .collect::<Result<Vec<_>, _>>()?,
    )?;
    let mut prepared = Vec::new();
    let mut requested_samples = 0usize;
    let mut requested_deadline_seconds = 0u64;
    // Freeze and parse ALL actual stage config bytes before ANY child process.
    // The existing runner later reads only these same owned readonly copies.
    for (index, stage) in config.stages.into_iter().enumerate() {
        let stage_directory = directory.join(&stage.name);
        fs::create_dir(&stage_directory).map_err(|e| e.to_string())?;
        let frozen = process::freeze(
            &stage.configuration,
            &stage_directory.join("stage-config.json"),
            65536,
        )?;
        let (inner, _) =
            config::load(Some(frozen.path.as_os_str()))?.ok_or("missing owned stage config")?;
        config::serial(inner.samples, env::var_os(config::SERIAL).as_deref())?;
        let attempts = set
            .trials()
            .checked_mul(1 + inner.samples)
            .ok_or("stage attempt overflow")?;
        requested_samples = requested_samples
            .checked_add(set.trials() * inner.samples)
            .ok_or("sample count overflow")?;
        requested_deadline_seconds = requested_deadline_seconds
            .checked_add(
                (attempts as u64)
                    .checked_mul(inner.deadline_seconds)
                    .ok_or("deadline total overflow")?,
            )
            .ok_or("deadline total overflow")?;
        receipt["stages"][index]["configuration"] = frozen.evidence(&stage.configuration.path);
        receipt["stages"][index]["requested_samples_per_variant"] = json!(inner.samples);
        receipt["stages"][index]["deadline_seconds"] = json!(inner.deadline_seconds);
        prepared.push(Prepared {
            name: stage.name,
            requested: stage.configuration.path,
            directory: stage_directory,
            configuration: frozen,
        });
        save(directory, receipt);
    }
    receipt["requested_timing_attempts"] = json!(requested_samples);
    receipt["maximum_process_attempts"] = json!(set.trials() * prepared.len() + requested_samples);
    receipt["sum_declared_process_deadlines_seconds"] = json!(requested_deadline_seconds);
    save(directory, receipt);
    // SAME result rows select the SAME two pinned binaries for every stage.
    let _ = super::binaries(results, set)?;
    for (index, stage) in prepared.iter().enumerate() {
        receipt["stages"][index]["status"] = json!("running");
        refresh(receipt);
        save(directory, receipt); // Durable dispatch even if the harness stops abruptly.
        let result = stage.configuration.check().and_then(|_| {
            super::execute(
                profile,
                &stage.directory,
                results,
                stage.configuration.path.as_os_str(),
                true,
                set,
            )
        });
        let mut result = result.unwrap_or_else(|error| {
            json!({
                "schema":set.result_schema(), "ok":false, "qualified":false,
                "preflight_error":error, "qualification":[], "samples":[], "timing_attempts":0,
                "timing_samples":0, "successful_timed_samples":0
            })
        });
        if let Err(error) = stage.configuration.check() {
            result["ok"] = json!(false);
            result["qualified"] = json!(false);
            result["stage_configuration_pin_error"] = json!(error);
            result["timing_samples"] = json!(0);
            result.as_object_mut().unwrap().remove("timing_summary");
        }
        super::save(&stage.directory, &result);
        let ok = result["ok"] == true && result["qualified"] == true;
        receipt["stages"][index] = json!({"name":stage.name,
            "status":if ok { "passed" } else { "failed" },
            "configuration":stage.configuration.evidence(&stage.requested),
            "result_path":stage.directory.join("full-input-result.json"), "result":result});
        if !ok {
            for later in receipt["stages"]
                .as_array_mut()
                .unwrap()
                .iter_mut()
                .skip(index + 1)
            {
                later["status"] = json!("skipped");
            }
            receipt["first_failed_stage"] = json!(stage.name);
        }
        refresh(receipt);
        save(directory, receipt); // Actual failed process/result precedes assert.
        if !ok {
            break;
        }
    }
    Ok(())
}

pub(super) fn run_if_configured(
    profile: Profile,
    evidence: &Path,
    results: &[Value],
    set: RunSet,
) -> Option<Value> {
    let configuration = env::var_os(staged_config::ENV)?;
    let directory = evidence.join("staged-input");
    fs::create_dir(&directory).unwrap();
    fs::write(
        directory.join("release-qualifier-results.json"),
        serde_json::to_vec_pretty(results).unwrap(),
    )
    .unwrap();
    let mut receipt = json!({"schema":set.stage_schema(),
        "ok":false, "qualified":false, "requested_stages":null, "stages":[],
        "stages_attempted":0, "running_stages":0, "qualified_stages":0, "failed_stages":0, "skipped_stages":0,
        "qualification_attempts":0, "qualification_process_starts_confirmed":0,
        "qualification_process_starts_unknown":0, "successful_qualification_attempts":0,
        "timing_attempts":0, "timed_process_starts_confirmed":0,
        "timed_process_starts_unknown":0, "successful_timed_samples":0, "timing_samples":0,
        "builds_reused":true, "stages_attempted_definition":"stage dispatch attempts, including pre-invocation pin/preflight refusals; not process-start claims",
        "process_counts_scope":"completed persisted attempt rows only; a running stage may have unrecorded starts/outcomes",
        "timing_samples_definition":"published count only if EVERY requested stage qualified; per-stage attempts and individual successes remain on partial failure",
        "cache_state":"warm-cache fresh-process end-to-end process wall; integrity hashes/correctness deliberately warm caches",
        "no_overlap":"nonzero samples in ANY stage require existing SERIAL=1 operator-exclusive attestation; no ambient process-scan claim"});
    if let Err(error) = execute(
        profile,
        &directory,
        results,
        &configuration,
        env::var_os(config::ENV).as_deref(),
        &mut receipt,
        set,
    ) {
        receipt["preflight_error"] = json!(error);
        for stage in receipt["stages"].as_array_mut().unwrap() {
            stage["status"] = json!("skipped");
        }
        refresh(&mut receipt);
    }
    save(&directory, &receipt);
    assert_eq!(
        receipt["ok"],
        true,
        "staged full-input qualification/timing failed; see {}",
        directory.display()
    );
    Some(receipt)
}

#[cfg(test)]
#[path = "canonical_hosted_full_stages_tests.rs"]
mod tests;
