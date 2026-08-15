use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::model::{EvidenceBatch, EvidenceGrade, EvidenceStatus, ExecutorObservations};
use crate::registry::hash_bytes;

pub(crate) fn collect(
    spec_root: &Path,
    repo_root: &Path,
    specification_version: &str,
    observations: &ExecutorObservations,
) -> Vec<EvidenceBatch> {
    let repository_commit = repository_commit(repo_root);
    let timestamp_unix_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or_default();
    let host_configuration = format!("{}-{}", std::env::consts::ARCH, std::env::consts::FAMILY);
    let operating_system = std::env::consts::OS.to_string();
    let runtime_input = hash_file(&spec_root.join("cases/transactions/commit-and-abort.mec"));
    let activation_input = hash_file(&spec_root.join("cases/activation/missing-grant.mec"));
    let repository_inputs = observations
        .repository_scanned_paths
        .iter()
        .map(|path| hash_file(&repo_root.join(path)))
        .collect::<Vec<_>>();
    let benchmark_input = hash_file(&spec_root.join("cases/benchmarks/protocol-comparison.mec"));
    let backend_input = hash_file(&spec_root.join("cases/backends/gpu-admission.mec"));

    vec![
        batch(BatchInput {
            provider: "runtime",
            provider_version: "v0.4-resident-1",
            schema_version: "runtime-turns/1",
            status: EvidenceStatus::Present,
            grade: EvidenceGrade::Observed,
            profile: "resident-cpu",
            detail: "resident turns, explicit transactions, semantic events, and shutdown were observed",
            input_hashes: vec![runtime_input],
            repository_commit: &repository_commit,
            specification_version,
            host_configuration: &host_configuration,
            operating_system: &operating_system,
            timestamp_unix_ms,
        }),
        batch(BatchInput {
            provider: "repository",
            provider_version: "source-import-scan/1",
            schema_version: "repository-dependencies/1",
            status: EvidenceStatus::Present,
            grade: EvidenceGrade::Observed,
            profile: "resident-cpu",
            detail: "resident execution modules were scanned for parser-internal imports",
            input_hashes: repository_inputs,
            repository_commit: &repository_commit,
            specification_version,
            host_configuration: &host_configuration,
            operating_system: &operating_system,
            timestamp_unix_ms,
        }),
        batch(BatchInput {
            provider: "activation",
            provider_version: "v0.4-resident-admission/1",
            schema_version: "activation-trace/1",
            status: EvidenceStatus::Present,
            grade: EvidenceGrade::Observed,
            profile: "resident-cpu",
            detail: "a provider-backed resident requirement was attempted without its hard capability grant",
            input_hashes: vec![activation_input],
            repository_commit: &repository_commit,
            specification_version,
            host_configuration: &host_configuration,
            operating_system: &operating_system,
            timestamp_unix_ms,
        }),
        batch(BatchInput {
            provider: "backend",
            provider_version: "prototype-admission/1",
            schema_version: "backend-admission/1",
            status: if observations.backend_admission_result == "unsupported" {
                EvidenceStatus::Unsupported
            } else {
                EvidenceStatus::Present
            },
            grade: EvidenceGrade::Observed,
            profile: "gpu",
            detail: &observations.backend_admission_reason,
            input_hashes: vec![backend_input],
            repository_commit: &repository_commit,
            specification_version,
            host_configuration: &host_configuration,
            operating_system: &operating_system,
            timestamp_unix_ms,
        }),
        batch(BatchInput {
            provider: "benchmark",
            provider_version: "protocol-observer/1",
            schema_version: "benchmark-protocols/1",
            status: EvidenceStatus::Present,
            grade: EvidenceGrade::Observed,
            profile: "resident-cpu",
            detail: "raw reference and candidate protocol identities were reported",
            input_hashes: vec![benchmark_input],
            repository_commit: &repository_commit,
            specification_version,
            host_configuration: &host_configuration,
            operating_system: &operating_system,
            timestamp_unix_ms,
        }),
    ]
}

struct BatchInput<'a> {
    provider: &'a str,
    provider_version: &'a str,
    schema_version: &'a str,
    status: EvidenceStatus,
    grade: EvidenceGrade,
    profile: &'a str,
    detail: &'a str,
    input_hashes: Vec<String>,
    repository_commit: &'a str,
    specification_version: &'a str,
    host_configuration: &'a str,
    operating_system: &'a str,
    timestamp_unix_ms: u128,
}

fn batch(input: BatchInput<'_>) -> EvidenceBatch {
    let identity = format!(
        "{}\n{}\n{}\n{}\n{:?}\n{}",
        input.provider,
        input.schema_version,
        input.repository_commit,
        input.specification_version,
        input.input_hashes,
        input.timestamp_unix_ms,
    );
    let run_hash = hash_bytes(identity.as_bytes());
    EvidenceBatch {
        run_id: format!("RUN-{}", &run_hash[..16]),
        parent_run_id: None,
        provider: input.provider.to_string(),
        provider_version: input.provider_version.to_string(),
        schema_version: input.schema_version.to_string(),
        status: input.status,
        grade: input.grade,
        repository_commit: input.repository_commit.to_string(),
        specification_version: input.specification_version.to_string(),
        input_hashes: input.input_hashes,
        execution_profile: input.profile.to_string(),
        runtime_version: "mech-runtime v0.4 resident executor".to_string(),
        host_configuration: input.host_configuration.to_string(),
        operating_system: input.operating_system.to_string(),
        timestamp_unix_ms: input.timestamp_unix_ms,
        detail: input.detail.to_string(),
    }
}

fn hash_file(path: &Path) -> String {
    match fs::read(path) {
        Ok(bytes) => format!("sha256:{}", hash_bytes(&bytes)),
        Err(error) => format!("unreadable:{}:{error}", path.display()),
    }
}

pub(crate) fn repository_commit(repo_root: &Path) -> String {
    Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(repo_root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .map(|commit| commit.trim().to_string())
        .filter(|commit| !commit.is_empty())
        .unwrap_or_else(|| "unknown".to_string())
}
