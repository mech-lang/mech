//! Selected-factory and finite coexistence checks for an elided inner matrix.
//! These bounds select a private optimization; ordinary execution retains its
//! source errors whenever the complete rectangle cannot be admitted.
use super::*;
use crate::native_artifact::program::prepare::numeric_maps::{RectangleBounds as Bounds, Region};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact::program::runtime) struct Geometry {
    pub start: u64,
    pub end: u64,
    pub rows: u64,
}
pub(super) fn resolve(
    descriptor: &NativeProgramDescriptor,
    region: &Region,
    args: [u64; 3],
) -> Result<Option<Geometry>, Error> {
    let Some(rectangle) = &region.rectangle else {
        if args != [0; 3] {
            return Err(invalid("numeric map entry has unexpected endpoints"));
        }
        return Ok(Some(Geometry {
            start: 0,
            end: 0,
            rows: 1,
        }));
    };
    let NativeControlLoop::Collection(inner) =
        &descriptor.metadata().plan.iterations()[rectangle.inner_iteration as usize]
    else {
        return Err(invalid("rectangular inner source changed"));
    };
    let NativeCollectionSource::U64Range(cursor) = inner.source() else {
        return Err(invalid("rectangular inner range changed"));
    };
    if cursor.prepare() != rectangle.range_prepare {
        return Err(invalid("rectangular range prepare changed"));
    }
    let (start, end) = match rectangle.bounds {
        Bounds::Fixed { start, end, .. } => {
            if args != [0; 3] {
                return Err(invalid("fixed rectangle entry has endpoints"));
            }
            (start, end)
        }
        Bounds::Current { operands } => {
            if args[2] != 0 || cursor.operands().map(|value| value.get()) != operands {
                return Err(invalid("current rectangle endpoint transport differs"));
            }
            // These are the exact original U64 operand words, transported by
            // the source-derived emitter just as at PrepareU64Range. Their
            // immutable-before-outer proof is reconstructed on image admission.
            (args[0], args[1])
        }
    };
    let rows = (|| {
        let difference = end.checked_sub(start)?;
        let rows = match cursor.template().primitive() {
            mech_engine::native_artifact::NativeCollectionPrimitive::RangeExclusive => difference,
            mech_engine::native_artifact::NativeCollectionPrimitive::RangeInclusive => {
                difference.checked_add(1)?
            }
            _ => return None,
        };
        (rows != 0).then_some(rows)
    })();
    let Some(rows) = rows else { return Ok(None) };
    if rectangle
        .fixed()
        .is_some_and(|fixed| fixed != [start, end, rows])
    {
        return Err(invalid("fixed rectangle cardinality differs"));
    }
    Ok(Some(Geometry { start, end, rows }))
}

#[derive(Default)]
pub(super) struct Bound {
    pub temporary: u64,
    pub nodes: u64,
    pub output: u64,
    pub comparison: u64,
    /// Complete selected transpose source fees for one original inner row.
    pub setup: u64,
    pub compute: u64,
    pub cloned: u64,
    pub replay: u64,
}
pub(super) fn bound(
    descriptor: &NativeProgramDescriptor,
    region: &Region,
    geometry: Geometry,
) -> Option<Bound> {
    let Some(rectangle) = &region.rectangle else {
        return Some(Bound::default());
    };
    let metadata = descriptor.metadata();
    let range = metadata
        .calls
        .get(rectangle.range_prepare as usize)?
        .as_ref()?;
    let bytes = range.binding_metadata_bytes();
    let work = range.binding_work();
    let definition = metadata.plan.values()[metadata.plan.collections()
        [rectangle.inner_collection as usize]
        .output()
        .get() as usize]
        .definition();
    let values = data::CollectedValues::new(definition.body());
    if values.packed_width() != Some(u64::from(region.output_width)) {
        return None;
    }
    let payload = geometry.rows.checked_mul(u64::from(region.output_width))?;
    // The ordinary growing packed bank has capacity<4*n plus its initial
    // four-word bank. This bounds both exact native and ordinary cold rows.
    // Only empty type metadata is created here; no row payload is allocated.
    let capacity = geometry.rows.checked_add(4)?.checked_mul(4)?;
    let transpose = super::numeric_transpose_budget::bound(
        descriptor,
        rectangle.transpose,
        &region.sites,
        super::numeric_transpose_budget::Row {
            count: geometry.rows,
            width: region.output_width,
            capacity_bytes: capacity.checked_mul(u64::from(region.output_width))?,
            finalization_cloned: payload,
            output_bytes: values
                .output_bytes(
                    payload,
                    geometry.rows,
                    usize::try_from(capacity).ok()?,
                    definition.dimension_parameters().len(),
                )
                .ok()?,
        },
    )?;
    // Keep a full ordinary construction/planning/finalization envelope for
    // both elided leaves. This is deliberately conservative and contains the
    // original draft bank even though no intermediate Value is constructed.
    let cells = geometry.rows.checked_mul(
        (size_of::<ValueDataDraft>() as u64)
            .checked_mul(8)?
            .checked_add(256)?,
    )?;
    let temporary = bytes
        .checked_mul(16)?
        .checked_add(metadata.observation_schema_bytes.checked_mul(8)?)?
        .checked_add(cells)?
        .checked_add(8192)?;
    let nodes = geometry.rows.checked_mul(4)?.checked_add(128)?;
    let output = geometry
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
        temporary: temporary
            .checked_add(transpose.temporary)?
            .checked_add(transpose.envelope)?,
        nodes: nodes.checked_add(transpose.nodes)?,
        output: output.max(transpose.output_bound),
        // Reuse the ordinary row price even though no transpose Value exists.
        comparison: transpose.comparison,
        setup: setup.checked_add(transpose.setup)?,
        compute: transpose.compute,
        cloned: transpose.cloned,
        replay: replay.checked_add(transpose.replay)?,
    })
}
pub(super) fn validate(
    ctx: &mut CollectionExecution<'_, '_, '_, '_>,
    region: &Region,
    geometry: Geometry,
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
        .selected_u64_range_cardinality(geometry.start, geometry.end)
        .ok()
        != Some(geometry.rows)
    {
        return Ok(false);
    }
    let input = &plan.values()[plan.collections()[rectangle.inner_collection as usize]
        .output()
        .get() as usize];
    if mech_core::shape_for_resolved_extents(cursor.template().output(), &[1, geometry.rows])
        .is_err()
        || mech_core::shape_for_resolved_extents(input.definition(), &[1, geometry.rows]).is_err()
        || transpose.template().inputs().first().is_none_or(|input| {
            mech_core::shape_for_resolved_extents(input, &[1, geometry.rows]).is_err()
        })
        || mech_core::shape_for_resolved_extents(transpose.template().output(), &[geometry.rows, 1])
            .is_err()
    {
        return Ok(false);
    }
    Ok(true)
}
