//! Selected-factory and finite coexistence checks for an elided inner matrix.
//! These bounds select a private optimization; ordinary execution retains its
//! source errors whenever the complete rectangle cannot be admitted.
use super::*;
use crate::native_artifact::program::prepare::numeric_maps::Region;

#[derive(Default)]
pub(super) struct Bound {
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub comparison: u64,
    pub setup: u64,
    pub replay: u64,
}
pub(super) fn bound(descriptor: &NativeProgramDescriptor, region: &Region) -> Option<Bound> {
    let Some(rectangle) = &region.rectangle else {
        return Some(Bound::default());
    };
    let metadata = descriptor.metadata();
    let mut bytes = 0u64;
    let mut work = 0u64;
    for site in [rectangle.range_prepare, rectangle.transpose] {
        let selected = metadata.calls.get(site as usize)?.as_ref()?;
        bytes = bytes.checked_add(selected.binding_metadata_bytes())?;
        work = work.checked_add(selected.binding_work())?;
    }
    // Keep a full ordinary construction/planning/finalization envelope for
    // both elided leaves. This is deliberately conservative and contains the
    // original draft bank even though no intermediate Value is constructed.
    let cells = rectangle.rows.checked_mul(
        (size_of::<ValueDataDraft>() as u64)
            .checked_mul(8)?
            .checked_add(256)?,
    )?;
    let temporary = bytes
        .checked_mul(16)?
        .checked_add(metadata.observation_schema_bytes.checked_mul(8)?)?
        .checked_add(cells)?
        .checked_add(8192)?;
    let nodes = rectangle.rows.checked_mul(4)?.checked_add(128)?;
    let output = rectangle
        .rows
        .checked_mul(size_of::<ValueDataDraft>() as u64)?
        .checked_add(metadata.observation_schema_bytes)?
        .checked_add(1024)?;
    let setup = work
        .checked_add(temporary.checked_mul(16)?)?
        .checked_add(4096)?;
    // Eligibility credit only, not a debit or new authority. A failed native
    // attempt must still leave enough work for the full original source path,
    // including cold or warm-cache binding/plan alternatives for both leaves.
    let replay = work
        .checked_mul(1024)?
        .checked_add(temporary.checked_mul(128)?)?
        .checked_add(65536)?;
    Some(Bound {
        temporary,
        nodes,
        output,
        // The elided transpose retains its original conservative comparison
        // admission even when its private output is never materialized.
        comparison: output,
        setup,
        replay,
    })
}
pub(super) fn validate(
    ctx: &mut CollectionExecution<'_, '_, '_, '_>,
    region: &Region,
    bound: &Bound,
) -> Result<bool, Error> {
    let Some(rectangle) = &region.rectangle else {
        return Ok(true);
    };
    let _scratch = ctx
        .library()
        .memory_budget()
        .reserve_capacity(bound.temporary)?;
    let metadata = ctx.descriptor().metadata();
    let plan = &metadata.plan;
    let NativeControlLoop::Collection(inner) =
        &plan.iterations()[rectangle.inner_iteration as usize]
    else {
        return Err(invalid("rectangular inner loop changed"));
    };
    let NativeCollectionSource::U64Range(cursor) = inner.source() else {
        return Err(invalid("rectangular range strategy changed"));
    };
    let range = metadata.calls[rectangle.range_prepare as usize]
        .as_ref()
        .ok_or_else(|| invalid("rectangular range factory disappeared"))?;
    let transpose = metadata.calls[rectangle.transpose as usize]
        .as_ref()
        .ok_or_else(|| invalid("rectangular transpose factory disappeared"))?;
    if range.template() != cursor.template()
        || transpose.template()
            != plan
                .prepared_library_template(rectangle.transpose as usize)
                .map_err(invalid)?
                .ok_or_else(|| invalid("rectangular transpose lacks template"))?
    {
        return Err(invalid("rectangular selected factory changed"));
    }
    if range
        .selected_u64_range_cardinality(rectangle.start, rectangle.end)
        .ok()
        != Some(rectangle.rows)
    {
        return Ok(false);
    }
    let input = &plan.values()[plan.collections()[rectangle.inner_collection as usize]
        .output()
        .get() as usize];
    if mech_core::shape_for_resolved_extents(input.definition(), &[1, rectangle.rows]).is_err()
        || mech_core::shape_for_resolved_extents(
            transpose.template().output(),
            &[rectangle.rows, 1],
        )
        .is_err()
    {
        return Ok(false);
    }
    Ok(true)
}
