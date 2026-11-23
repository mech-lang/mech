//! Linked managed CFG qualification shell, not a source evaluator or I/O host.
use crate::native_artifact::launch::{policy::LibraryLimits, program::BuildOwnedProgramBinding};
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
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Launch {
    memory_bytes: u64,
    work: u64,
    transport_bytes: u64,
    native_stack_bytes: u64,
    library: LibraryLimits,
}
impl Launch {
    fn limits(&self) -> Result<NativeProgramExecutionLimits, String> {
        Ok(NativeProgramExecutionLimits {
            execution: NativeExecutionLimits {
                max_work: self.work,
                max_transport_bytes: self.transport_bytes,
                max_native_stack_bytes: self.native_stack_bytes,
            },
            library: self
                .library
                .limits()
                .map_err(|error| format!("{error:?}"))?,
        })
    }
}
fn policy(
    arguments: &[OsString],
) -> Result<(NativeProgramExecutionLimits, ManagedMemoryBudget, &OsStr), String> {
    if arguments.len() == 4 && arguments[0] == "--policy" && arguments[2] == "--" {
        let mut bytes = Vec::new();
        std::fs::File::open(&arguments[1])
            .map_err(|error| error.to_string())?
            .take(65537)
            .read_to_end(&mut bytes)
            .map_err(|error| error.to_string())?;
        if bytes.len() > 65536 {
            return Err("launch policy exceeds 64 KiB".into());
        }
        let launch: Launch = serde_json::from_slice(&bytes).map_err(|error| error.to_string())?;
        return Ok((
            launch.limits()?,
            ManagedMemoryBudget::new(launch.memory_bytes),
            &arguments[3],
        ));
    }
    if arguments.len() != 7 || arguments[5] != "--" {
        return Err("expected --policy LAUNCH_JSON -- INPUT_JSON or WORK BUDGET CALL_COMPUTE CALL_COMPARISON CALL_TEMPORARY -- INPUT_JSON".into());
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
    Ok((limits, memory, &arguments[6]))
}
pub fn run_program_application(
    binding: BuildOwnedProgramBinding,
    arguments: Vec<OsString>,
) -> Result<(), String> {
    let (limits, memory, input_path) = policy(&arguments)?;
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
    std::fs::File::open(input_path)
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
    drop(inputs);
    if memory.used_bytes() != 0 || memory.retained_snapshot_nodes() != 0 {
        return Err("linked managed execution retained an unpublished owner".into());
    }
    let stdout = std::io::stdout();
    let mut stdout = stdout.lock();
    stdout
        .write_all(output.as_bytes())
        .map_err(|error| error.to_string())?;
    stdout.flush().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn managed_launch_policy_requires_all_finite_runtime_dimensions() {
        let text = r#"{"memory_bytes":268435456,"work":10000000000000000,"transport_bytes":67108864,"native_stack_bytes":1048576,"library":{"output_elements":16777216,"output_bytes":134217728,"temporary_bytes":134217728,"cloned_bytes":134217728,"retained_nodes":1000000,"compute_work":10000000000000000,"comparison_work":1000000000000}}"#;
        let launch: Launch = serde_json::from_str(text).unwrap();
        let limits = launch.limits().unwrap();
        assert_eq!(limits.execution.max_work, 10000000000000000);
        assert_eq!(limits.library.max_output_elements, Some(16777216));
        assert!(
            serde_json::from_str::<Launch>(&text.replace("\"work\":10000000000000000,", ""))
                .is_err()
        );
        assert!(
            serde_json::from_str::<Launch>(&text.replace("\"memory_bytes\":", "\"max_nodes\":"))
                .is_err()
        );
    }
}
