//! All-path runtime ownership, independent of current inputs or native selection.
use mech_core::Value;
use mech_engine::native_artifact::NativeProgramFacts;

/// Keep every immutable plan constant for compilation/provenance. A zero bound
/// means its value has no canonical semantic reader, including virtual actions,
/// loop seeds/returns, effects and outputs. Live scalar-only values still own
/// their ordinary canonical representation; that is a separate optimization.
pub(in crate::native_artifact) fn admission_bounds(
    facts: &NativeProgramFacts<'_>,
) -> Result<Box<[u64]>, String> {
    let reads = facts.semantic_use_counts().map_err(str::to_owned)?;
    facts
        .plan()
        .constants()
        .iter()
        .map(|constant| {
            let &count = reads
                .get(constant.value().get() as usize)
                .ok_or("constant ownership value is outside the canonical read inventory")?;
            if count == 0 {
                return Ok(0);
            }
            constant
                .constant()
                .memory_budget_required_bytes()
                .map_err(|e| format!("constant ownership: {e:?}"))?
                .checked_add(Value::memory_budget_claim_metadata_bytes())
                .and_then(|n| n.checked_add(4096))
                .ok_or_else(|| "constant admission overflows".to_owned())
        })
        .collect()
}
