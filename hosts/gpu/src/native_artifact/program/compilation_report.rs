//! Optional host measurements for the compact compiler diagnostic.
use super::compact::profile_inputs::SelectedProfiles;
use crate::native_artifact::codegen::compilation_inputs::CompilationInputs;
use crate::native_artifact::identity::{hex, transcript::bytes};
use cranelift_codegen::isa::TargetIsa;
use cranelift_object::ObjectProduct;
use sha2::{Digest, Sha256};
use std::time::Instant;

#[cfg(test)]
#[path = "compilation_report_tests.rs"]
mod tests;

/// Opaque inputs for an experimental native diagnostic compilation.
#[doc(hidden)]
#[derive(Clone)]
pub struct NativeCompilerConfiguration(pub(super) CompilationInputs);

impl NativeCompilerConfiguration {
    /// Capture the current legacy settings once.
    pub fn from_legacy_environment() -> Self {
        Self(CompilationInputs::legacy())
    }

    /// Capture explicit native settings once.
    pub fn capture(
        values: impl IntoIterator<Item = (std::ffi::OsString, std::ffi::OsString)>,
    ) -> Self {
        Self(CompilationInputs::capture(values))
    }
}

/// Optional measurements alongside the existing compact native artifact.
#[doc(hidden)]
#[derive(Default)]
pub struct NativeCompilationReport {
    source_revision: Option<String>,
    inputs: Option<serde_json::Value>,
    target: Option<String>,
    flags: Vec<(&'static str, String)>,
    phase_observer: bool,
    workers: Option<usize>,
    active_attempt: Option<usize>,
    stages: Vec<serde_json::Value>,
    attempts: Vec<serde_json::Value>,
    sizes: Option<serde_json::Value>,
}

impl NativeCompilationReport {
    /// Return host measurements and the available selections from the existing descriptor.
    pub fn json(&self, metadata: &serde_json::Value) -> serde_json::Value {
        let decisions = [
            "numeric_phases",
            "word_collections",
            "word_pack_sites",
            "physical_unit",
            "cleanup",
            "overwrite_result_reuse",
            "guarded_consuming_sites",
            "seeded_histograms",
            "calls",
            "workers",
        ];
        let selection: serde_json::Map<_, _> = decisions
            .into_iter()
            .filter_map(|name| {
                metadata
                    .get(name)
                    .map(|value| (name.to_owned(), value.clone()))
            })
            .collect();
        serde_json::json!({
            "schema": "mech.native-compilation-report.v2",
            "source_revision": self.source_revision, "configuration": self.inputs,
            "target_triple": self.target, "backend": "cranelift", "codegen_flags": self.flags,
            "phase_observer": self.phase_observer, "compile_workers": self.workers,
            "stages": self.stages, "object_attempts": self.attempts, "object_sizes": self.sizes,
            "selection": selection, "selection_scope": "available compact descriptor decisions",
            "construction": {
                "available": false, "reason": "requires a separate construction-census run",
            },
            "peak_process_rss": {
                "available": false, "reason": "requires a process-level measurement",
            },
        })
    }
}

pub(super) struct Measurements<'a>(pub Option<&'a mut NativeCompilationReport>);

impl Measurements<'_> {
    pub(super) fn identify(
        &mut self,
        source_revision: impl FnOnce() -> String,
        inputs: &CompilationInputs,
        observe: bool,
    ) -> Option<[u8; 32]> {
        let report = self.0.as_deref_mut()?;
        *report = NativeCompilationReport::default();
        report.source_revision = Some(source_revision());
        let (raw, count) = inputs.raw_settings_identity();
        report.inputs = Some(serde_json::json!({
            "raw_settings_sha256": hex(&raw),
            "raw_setting_count": count,
        }));
        report.phase_observer = observe;
        Some(raw)
    }

    pub(super) fn finish_inputs(&mut self, raw: Option<[u8; 32]>, selected: &SelectedProfiles) {
        if let Some(raw) = raw
            && let Some(report) = &mut self.0
            && let Some(inputs) = &mut report.inputs
        {
            let (resolved, rows) = selected.report_identity();
            let mut combined = Sha256::new();
            bytes(&mut combined, b"mech-native-inputs-v2");
            bytes(&mut combined, &raw);
            bytes(&mut combined, &resolved);
            inputs["resolved_profiles_sha256"] = hex(&resolved).into();
            inputs["resolved_profiles"] = rows.into();
            inputs["sha256"] = hex(&combined.finalize()).into();
            inputs["scope"] =
                "captured raw settings and profiles resolved by this compilation".into();
        }
    }

    pub(super) fn stage<T>(&mut self, name: &'static str, work: impl FnOnce() -> T) -> T {
        let start = self.0.as_ref().map(|_| Instant::now());
        let result = work();
        if let (Some(report), Some(start)) = (&mut self.0, start) {
            report.stages.push(serde_json::json!({
                "stage": name, "attempt": report.active_attempt, "elapsed_ns": start.elapsed().as_nanos() as u64,
            }));
        }
        result
    }

    pub(super) fn start_attempt(&mut self, direct_literals: bool) {
        if let Some(report) = &mut self.0 {
            report.active_attempt = Some(report.attempts.len());
            report
                .attempts
                .push(serde_json::json!({"direct_entry_literals": direct_literals}));
        }
    }

    pub(super) fn finish_attempt(
        &mut self,
        result: &Result<(Vec<u8>, serde_json::Value), crate::native_artifact::NativeCompileError>,
    ) {
        if let Some(report) = &mut self.0
            && let Some(at) = report.active_attempt.take()
        {
            let row = &mut report.attempts[at];
            row["status"] = serde_json::json!(if result.is_ok() { "emitted" } else { "failed" });
            if let Err(error) = result {
                row["error"] = serde_json::json!(error.to_string());
            }
        }
    }

    pub(super) fn module(
        &mut self,
        isa: &dyn TargetIsa,
        flags: &[(&'static str, &str)],
        workers: usize,
    ) {
        if let Some(report) = &mut self.0 {
            report.target = Some(isa.triple().to_string());
            report.flags = flags
                .iter()
                .map(|&(key, value)| (key, value.to_owned()))
                .collect();
            report.workers = Some(workers);
        }
    }

    pub(super) fn object(&mut self, product: &ObjectProduct) {
        if let Some(report) = &mut self.0 {
            let code: u64 = product
                .functions
                .values()
                .flatten()
                .filter(|(_, defined)| *defined)
                .map(|(id, _)| product.object.symbol(*id).size)
                .sum();
            let constants: u64 = product
                .data_objects
                .values()
                .flatten()
                .filter(|(_, defined)| *defined)
                .map(|(id, _)| product.object.symbol(*id).size)
                .sum();
            report.sizes = Some(serde_json::json!({
                "function_body_bytes": code,
                "constant_data_definition_bytes": constants,
                "scope": "emitted function symbols including inline literals; compact module constant data symbols",
            }));
        }
    }

    pub(super) fn object_bytes(&mut self, bytes: usize) {
        if let Some(report) = &mut self.0
            && let Some(sizes) = &mut report.sizes
        {
            sizes["object_file_bytes"] = serde_json::json!(bytes);
        }
    }
}
