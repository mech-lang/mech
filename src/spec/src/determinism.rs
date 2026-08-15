use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};

use crate::model::CheckReport;
use crate::registry::hash_bytes;
use crate::source_profile::SourceProfile;
use crate::{Result, SpecError, current, reference};

const BUNDLE_FORMAT: &str = "mech-determinism-bundle/1";

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredSource {
    pub path: String,
    pub profile: SourceProfile,
    pub content_hash: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BundlePayload {
    format: String,
    recorded_at_unix_ms: u128,
    reference_evaluator: String,
    current_evaluator: String,
    report_hash: String,
    sources: Vec<StoredSource>,
    report: CheckReport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
struct BundleManifest {
    bundle_hash: String,
    payload: BundlePayload,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RecordedBundle {
    pub bundle_hash: String,
    pub manifest_path: PathBuf,
    pub report: CheckReport,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ReplayReport {
    pub bundle_hash: String,
    pub format_valid: bool,
    pub content_hashes_valid: bool,
    pub report_hash_valid: bool,
    pub reference_replayed: bool,
    pub current_mech_replayed: bool,
    pub evaluators_agree: bool,
    pub judgments_match: bool,
    pub witnesses_match: bool,
    pub original_conforming: bool,
    pub passed: bool,
}

impl ReplayReport {
    pub fn render_text(&self) -> String {
        format!(
            concat!(
                "Determinism bundle replay\n",
                "  bundle:       {bundle}\n",
                "  format:       {format}\n",
                "  blobs:        {blobs}\n",
                "  report hash:  {report}\n",
                "  reference:    {reference}\n",
                "  resident Mech:{current}\n",
                "  evaluators:   {agreement}\n",
                "  judgments:    {judgments}\n",
                "  witnesses:    {witnesses}\n",
                "  result:       {result}",
            ),
            bundle = self.bundle_hash,
            format = pass_fail(self.format_valid),
            blobs = pass_fail(self.content_hashes_valid),
            report = pass_fail(self.report_hash_valid),
            reference = pass_fail(self.reference_replayed),
            current = pass_fail(self.current_mech_replayed),
            agreement = pass_fail(self.evaluators_agree),
            judgments = pass_fail(self.judgments_match),
            witnesses = pass_fail(self.witnesses_match),
            result = if self.passed {
                "REPLAYED IDENTICALLY"
            } else {
                "REPLAY MISMATCH"
            },
        )
    }
}

pub(crate) fn record(report: CheckReport, store: &Path) -> Result<RecordedBundle> {
    let blobs = store.join("blobs");
    let manifests = store.join("manifests");
    fs::create_dir_all(&blobs).map_err(|error| {
        SpecError::new(format!(
            "could not create blob store {}: {error}",
            blobs.display()
        ))
    })?;
    fs::create_dir_all(&manifests).map_err(|error| {
        SpecError::new(format!(
            "could not create manifest store {}: {error}",
            manifests.display()
        ))
    })?;

    let mut sources = Vec::new();
    for artifact in &report.artifacts {
        let path = Path::new(&artifact.source_path);
        let bytes = fs::read(path).map_err(|error| {
            SpecError::new(format!(
                "could not read bundle source {}: {error}",
                path.display()
            ))
        })?;
        let content_hash = hash_bytes(&bytes);
        let blob_path = blobs.join(&content_hash);
        if !blob_path.exists() {
            fs::write(&blob_path, bytes).map_err(|error| {
                SpecError::new(format!(
                    "could not write bundle blob {}: {error}",
                    blob_path.display()
                ))
            })?;
        }
        sources.push(StoredSource {
            path: artifact.source_path.clone(),
            profile: artifact.profile,
            content_hash,
        });
    }
    let report_json = serde_json::to_vec(&report)
        .map_err(|error| SpecError::new(format!("serialize bundle report: {error}")))?;
    let payload = BundlePayload {
        format: BUNDLE_FORMAT.to_string(),
        recorded_at_unix_ms: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|duration| duration.as_millis())
            .unwrap_or_default(),
        reference_evaluator: "mech-contract-reference/1".to_string(),
        current_evaluator: "mech-runtime-v0.4-resident/1".to_string(),
        report_hash: hash_bytes(&report_json),
        sources,
        report: report.clone(),
    };
    let payload_json = serde_json::to_vec(&payload)
        .map_err(|error| SpecError::new(format!("serialize bundle payload: {error}")))?;
    let bundle_hash = hash_bytes(&payload_json);
    let manifest = BundleManifest {
        bundle_hash: bundle_hash.clone(),
        payload,
    };
    let manifest_json = serde_json::to_string_pretty(&manifest)
        .map_err(|error| SpecError::new(format!("serialize bundle manifest: {error}")))?;
    let manifest_path = manifests.join(format!("{bundle_hash}.json"));
    fs::write(&manifest_path, format!("{manifest_json}\n")).map_err(|error| {
        SpecError::new(format!(
            "could not write bundle manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    Ok(RecordedBundle {
        bundle_hash,
        manifest_path,
        report,
    })
}

pub(crate) fn replay(bundle_hash: &str, store: &Path) -> Result<ReplayReport> {
    let manifest_path = store.join("manifests").join(format!("{bundle_hash}.json"));
    let manifest_source = fs::read_to_string(&manifest_path).map_err(|error| {
        SpecError::new(format!(
            "could not read bundle manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    let manifest: BundleManifest = serde_json::from_str(&manifest_source).map_err(|error| {
        SpecError::new(format!(
            "could not decode bundle manifest {}: {error}",
            manifest_path.display()
        ))
    })?;
    let payload_json = serde_json::to_vec(&manifest.payload)
        .map_err(|error| SpecError::new(format!("serialize replay payload: {error}")))?;
    let format_valid = manifest.payload.format == BUNDLE_FORMAT;
    let manifest_hash_valid =
        hash_bytes(&payload_json) == manifest.bundle_hash && manifest.bundle_hash == bundle_hash;
    let report_json = serde_json::to_vec(&manifest.payload.report)
        .map_err(|error| SpecError::new(format!("serialize replay report: {error}")))?;
    let report_hash_valid = hash_bytes(&report_json) == manifest.payload.report_hash;
    let mut content_hashes_valid = manifest_hash_valid;
    let mut specification_sources = Vec::new();
    for source in &manifest.payload.sources {
        let blob_path = store.join("blobs").join(&source.content_hash);
        let bytes = fs::read(&blob_path).map_err(|error| {
            SpecError::new(format!(
                "could not read bundle blob {}: {error}",
                blob_path.display()
            ))
        })?;
        content_hashes_valid &= hash_bytes(&bytes) == source.content_hash;
        if source.profile == SourceProfile::Specification {
            let text = String::from_utf8(bytes).map_err(|error| {
                SpecError::new(format!("specification blob is not UTF-8: {error}"))
            })?;
            if text.contains("contract-profile :=") {
                specification_sources.push(text);
            }
        }
    }
    if specification_sources.is_empty() {
        return Err(SpecError::new(
            "determinism bundle contains no executable specification source",
        ));
    }
    let observations = manifest.payload.report.observations.to_mech_source();
    let mut reference_replayed = true;
    let mut current_mech_replayed = true;
    let mut evaluators_agree = true;
    let mut replayed = Vec::new();
    for source in specification_sources {
        let reference = match reference::evaluate(&observations, &source) {
            Ok(reference) => reference,
            Err(_) => {
                reference_replayed = false;
                continue;
            }
        };
        let current = match current::evaluate(&observations, &reference) {
            Ok(current) => current,
            Err(_) => {
                current_mech_replayed = false;
                continue;
            }
        };
        for result in reference {
            let mech = current.get(&result.name).map(|value| value.passed);
            evaluators_agree &= mech == Some(result.passed);
            replayed.push((
                result.requirement,
                result.name,
                result.passed,
                mech,
                result.observed,
                result.expected,
            ));
        }
    }
    let judgments_match = manifest.payload.report.constraints.iter().all(|expected| {
        replayed
            .iter()
            .any(|(requirement, name, reference, mech, _, _)| {
                requirement == &expected.requirement
                    && name == &expected.name
                    && *reference == expected.reference_passed
                    && *mech == expected.mech_passed
            })
    }) && replayed.len() == manifest.payload.report.constraints.len();
    let witnesses_match = manifest.payload.report.constraints.iter().all(|expected| {
        let actual = replayed.iter().find(|(requirement, name, _, _, _, _)| {
            requirement == &expected.requirement && name == &expected.name
        });
        match (&expected.witness, actual) {
            (None, Some((_, _, true, Some(true), _, _))) => true,
            (Some(witness), Some((_, _, false, Some(false), observed, expected))) => {
                witness.observed == observed.as_str() && witness.expected == expected.as_str()
            }
            _ => false,
        }
    });
    let passed = format_valid
        && content_hashes_valid
        && report_hash_valid
        && reference_replayed
        && current_mech_replayed
        && evaluators_agree
        && judgments_match
        && witnesses_match;
    Ok(ReplayReport {
        bundle_hash: bundle_hash.to_string(),
        format_valid,
        content_hashes_valid,
        report_hash_valid,
        reference_replayed,
        current_mech_replayed,
        evaluators_agree,
        judgments_match,
        witnesses_match,
        original_conforming: manifest.payload.report.passed,
        passed,
    })
}

fn pass_fail(value: bool) -> &'static str {
    if value { "PASS" } else { "FAIL" }
}
