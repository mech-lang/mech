//! Linked managed CFG qualification shell, not a source evaluator or I/O host.
use crate::native_artifact::launch::program::BuildOwnedProgramBinding;
use crate::native_artifact::{NativeExecutionLimits, NativeProgramExecutionLimits};
use mech_core::ManagedMemoryBudget;
use mech_core::snapshot::{SnapshotValidationContext, ValueDraft};
use std::ffi::{OsStr, OsString};
use std::io::{Read, Write};
use std::sync::Arc;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}
fn unsigned(argument: &OsStr) -> Result<u64, String> {
    let text = argument.to_str().ok_or("policy is not Unicode")?;
    if text.is_empty() || !text.bytes().all(|byte| byte.is_ascii_digit()) {
        return Err("policy must be an exact U64".into());
    }
    text.parse().map_err(|_| "policy is outside U64".into())
}
pub fn run_program_application(
    binding: BuildOwnedProgramBinding,
    arguments: Vec<OsString>,
) -> Result<(), String> {
    if arguments.len() != 7 || arguments[5] != "--" {
        return Err(
            "expected WORK BUDGET CALL_COMPUTE CALL_COMPARISON CALL_TEMPORARY -- INPUT_JSON".into(),
        );
    }
    let mut limits = NativeProgramExecutionLimits {
        execution: NativeExecutionLimits::default(),
        library: mech_core::TargetMemoryProfile::current_resident_cpu()
            .map_err(|error| format!("{error:?}"))?
            .limits,
    };
    limits.execution.max_work = unsigned(&arguments[0])?;
    let memory = ManagedMemoryBudget::new(unsigned(&arguments[1])?);
    limits.library.max_compute_work = Some(unsigned(&arguments[2])?);
    limits.library.max_comparison_work = Some(unsigned(&arguments[3])?);
    limits.library.max_temporary_bytes = Some(unsigned(&arguments[4])?);
    // Metadata/factory selection only; no frontend, JIT or graph execution.
    let catalog = crate::native_artifact::launch::scalar::metadata_catalog(binding.0.catalog)?;
    let linked = binding
        .0
        .bind(&catalog)
        .map_err(|error| error.to_string())?;
    // This explicit bounded transport is for linked qualification, not an
    // application-visible resource capability or an alternate Mech evaluator.
    const INPUT_BYTES: u64 = 16 * 1024 * 1024;
    let mut bytes = Vec::new();
    std::fs::File::open(&arguments[6])
        .map_err(|error| error.to_string())?
        .take(INPUT_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|error| error.to_string())?;
    if bytes.len() as u64 > INPUT_BYTES {
        return Err("qualification input is too large".into());
    }
    let drafts: Vec<ValueDraft> =
        serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
    if drafts.len() != linked.descriptor().plan().inputs().len() {
        return Err("input count differs from canonical plan".into());
    }
    let schemas = Arc::new(linked.descriptor().schemas().clone());
    let mut inputs = Vec::with_capacity(drafts.len());
    for (draft, input) in drafts.into_iter().zip(linked.descriptor().plan().inputs()) {
        if draft.schema != input.ty().schema() {
            return Err("input schema differs from canonical plan".into());
        }
        let value = draft
            .finalize(&SnapshotValidationContext::with_shared_schemas(&schemas))
            .map_err(|error| format!("{error:?}"))?;
        let mut reservation = memory
            .reserve_capacity(
                value
                    .memory_budget_admission_bytes(&memory)
                    .map_err(|error| format!("{error:?}"))?,
            )
            .map_err(|error| format!("{error:?}"))?;
        inputs.push(
            value
                .into_memory_budget(&mut reservation)
                .map_err(|error| format!("{error:?}"))?,
        );
    }
    let mut outputs = vec![None; linked.descriptor().plan().outputs().len()];
    linked
        .evaluate_into(&inputs, &mut outputs, limits, &memory)
        .map_err(|error| error.to_string())?;
    // Complete canonical output after atomic native evaluation, not a checksum
    // or selected-result shortcut. The independent resident oracle uses the
    // same public export encoding, including exact schema, shape and payload.
    let mut output = String::new();
    for value in outputs {
        let value = value.ok_or("native output was not published")?;
        let bytes = value
            .canonical_snapshot_bytes(&schemas)
            .map_err(|error| format!("{error:?}"))?;
        output.push_str(&hex(&bytes));
        output.push('\n');
    }
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(output.as_bytes())
        .map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}
