//! One original packed-row transpose price for materialized and elided rows.
//! Pure geometry grants no loan, owner, source visit or publication authority.
use super::*;

pub(super) struct Row {
    pub count: u64,
    pub width: u8,
    pub capacity_bytes: u64,
    pub finalization_cloned: u64,
    pub output_bytes: u64,
}
pub(super) struct Funding {
    pub envelope: u64,
    pub temporary: u64,
    pub cloned: u64,
    pub compute: u64,
    pub setup: u64,
    pub replay: u64,
    pub output_bound: u64,
    pub nodes: u64,
    pub comparison: u64,
}
pub(super) fn bound(
    descriptor: &NativeProgramDescriptor,
    transpose: u32,
    continuation: &[u32],
    row: Row,
) -> Option<Funding> {
    let metadata = descriptor.metadata();
    let selected = metadata.calls.get(transpose as usize)?.as_ref()?;
    if !selected.supports_numeric_matrix_transpose() {
        return None;
    }
    let bytes = selected.binding_metadata_bytes();
    let binding_work = selected.binding_work();
    let payload = row.count.checked_mul(u64::from(row.width))?;
    let envelope = metadata
        .scalar_bytes
        .checked_add(row.capacity_bytes.checked_mul(4)?)?
        .checked_add(4096)?;
    // Complete ordinary builder/transpose, including draft staging, selected
    // schema planning and both roots. Preserve these source prices when a
    // private rectangle avoids constructing the intermediate row altogether.
    let temporary = bytes
        .checked_mul(128)?
        .checked_add(metadata.observation_schema_bytes.checked_mul(32)?)?
        .checked_add(
            row.count.checked_add(4)?.checked_mul(
                (size_of::<ValueDataDraft>() as u64)
                    .checked_mul(16)?
                    .checked_add(512)?,
            )?,
        )?
        .checked_add(65536)?;
    let cloned = row
        .finalization_cloned
        .checked_add(payload.checked_mul(2)?)?
        .checked_add(bytes)?;
    let compute = envelope
        .checked_mul(16)?
        .checked_add(temporary.checked_mul(16)?)?
        .checked_add(binding_work.checked_mul(16)?)?
        .checked_add(8192)?;
    let setup = bytes.checked_mul(128)?.checked_add(4096)?;
    let output_bound = row
        .output_bytes
        .checked_mul(4)?
        .checked_add(bytes)?
        .checked_add(metadata.observation_schema_bytes)?
        .checked_add(4096)?;
    let nodes = row
        .count
        .checked_mul(4)?
        .checked_add(bytes)?
        .checked_add(metadata.observation_schema_bytes)?
        .checked_add(256)?;
    let comparison = output_bound.checked_add(temporary)?;
    let visit_work = continuation.iter().try_fold(0u64, |n, &site| {
        n.checked_add(metadata.visits.get(site as usize)?.work)
    })?;
    let replay = compute
        .checked_add(setup)?
        .checked_add(visit_work)?
        .checked_add(metadata.scalar_bytes.checked_mul(16)?)?
        .checked_mul(128)?
        .checked_add(65536)?;
    Some(Funding {
        envelope,
        temporary,
        cloned,
        compute,
        setup,
        replay,
        output_bound,
        nodes,
        comparison,
    })
}
