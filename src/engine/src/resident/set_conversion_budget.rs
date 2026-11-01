//! Complete current populations for the maintained schema-aware conversion.
use super::super::budget::{
    self, KernelCostEstimate, ResidentBudgetMeter, checked_cost_product as mul,
    checked_cost_sum as sum, checked_u64,
};
use super::{Plan, stats};
use mech_core::*;
use std::mem::size_of;

pub(super) struct Prepared {
    pub(super) cost: KernelCostEstimate,
    pub(super) canonicalization_work: u64,
}

pub(super) fn prepare(
    input: ResidentValueRef<'_>,
    previous: Option<&Value>,
    schemas: &SchemaTable,
    source: &Schema,
    target: &Schema,
    plan: &Plan,
    meter: &mut ResidentBudgetMeter,
) -> Result<Prepared, ResidentKernelError> {
    let mut stats = stats::Stats::default();
    let (input_bytes, input_nodes, draft_nodes, root_count, finalization, owned_shape_parameters) =
        match input {
            ResidentValueRef::Snapshot([Some(value)]) => {
                if value.schema_key() != plan.source_key || value.schema() != plan.source_schema {
                    return Err(ResidentKernelError::InvalidInput);
                }
                let footprint = budget::measure_canonical_value_footprint(meter, value, schemas)?;
                let drafts = stats::draft_nodes(value.data(), meter)?;
                let finalization = budget::preflight_canonical_data_finalization(
                    meter,
                    source.body(),
                    value.data(),
                )?;
                stats.snapshot(
                    &plan.conversion.step,
                    source.body(),
                    target.body(),
                    value.data(),
                    meter,
                )?;
                let count = match value.data() {
                    ValueData::Matrix(matrix) => matrix.elements().len(),
                    _ => 1,
                };
                (
                    footprint.retained_bytes,
                    footprint.node_count,
                    drafts,
                    count,
                    finalization,
                    Some(value.shape().parameter_values().len()),
                )
            }
            ResidentValueRef::Snapshot(_) => return Err(ResidentKernelError::InvalidInput),
            values => {
                let n = checked_u64(values.len())?;
                meter.charge_compute_work(mul(&[n, 2])?)?;
                let (bytes, longest) = if let ResidentValueRef::String(values) = values {
                    let bytes = values
                        .iter()
                        .try_fold(0u64, |n, value| n.checked_add(value.len() as u64))
                        .ok_or(ResidentKernelError::InvalidShape)?;
                    (
                        // Dense [String] has real inline String carriers.
                        // Include them before crediting their width against
                        // the canonical key's inline ValueData header.
                        sum(&[bytes, mul(&[n, size_of::<String>() as u64])?])?,
                        values.iter().map(|s| s.len() as u64).max().unwrap_or(0),
                    )
                } else {
                    (mul(&[n, 16])?, 0)
                };
                stats.dense(
                    &plan.conversion.step,
                    source.body(),
                    target.body(),
                    values,
                    longest,
                    meter,
                )?;
                (
                    bytes,
                    n.checked_add(1).ok_or(ResidentKernelError::InvalidShape)?,
                    n.checked_add(1).ok_or(ResidentKernelError::InvalidShape)?,
                    values.len(),
                    0,
                    None,
                )
            }
        };
    let previous = previous
        .map(|value| budget::measure_canonical_value_footprint(meter, value, schemas))
        .transpose()?;
    let expanded = sum(&[draft_nodes, stats.option_wrappers])?;
    // Scalar Display bound is the maintained 768-byte bound; identity payload
    // includes complete borrowed current bytes. Newly inserted Option boxes
    // were NOT part of that input: always add their actual frozen headers,
    // outside max(), rather than hiding them behind retained source bytes.
    let flat_payload = input_bytes.max(mul(&[draft_nodes, 32])?);
    let target_shape_bytes = mul(&[
        checked_u64(target.dimension_parameters().len())?,
        size_of::<u64>() as u64,
    ])?;
    let new_output_carrier = match owned_shape_parameters {
        // Snapshot retention already includes a real Value and source tape.
        // Only positive growth is missing; parameter ordinals are not copied.
        Some(source_parameters) => mul(&[
            checked_u64(
                target
                    .dimension_parameters()
                    .len()
                    .saturating_sub(source_parameters),
            )?,
            size_of::<u64>() as u64,
        ])?,
        // Dense borrowed slices carry neither an owned Value nor a tape.
        // Preserve the flat price and credit only its spare header allowance,
        // never bytes already needed for the actual source/String payload.
        None => sum(&[
            size_of::<Value>() as u64,
            target_shape_bytes,
            (size_of::<ValueData>() as u64).saturating_sub(flat_payload - input_bytes),
        ])?,
    };
    let payload = sum(&[
        flat_payload,
        new_output_carrier,
        // A packed source lane did not retain a ValueData per element.
        // Fund actual target carrier growth outside max(), not another key
        // wrapper fee. Values lanes already carry these headers and add zero.
        stats.storage_growth,
        mul(&[stats.option_wrappers, size_of::<ValueData>() as u64])?,
        mul(&[stats.string_casts, 768])?,
    ])?;
    let comparisons = sum(&[
        stats.fixed_comparison,
        mul(&[stats.compound_comparison_units, payload])?,
    ])?;
    // The unchanged 3*N*(log2(N)+1) bound already includes the flat
    // heapsort/dedup/publication comparisons. Only real ancestor rebuilds
    // repeat it; already-canonical source keys have their own round-trip cost.
    let canonicalization_work = sum(&[
        mul(&[comparisons, plan.phases])?,
        mul(&[
            finalization,
            plan.phases
                .checked_add(1)
                .ok_or(ResidentKernelError::InvalidShape)?,
        ])?,
    ])?;
    // SAME setup/evidence/dimension/draft obligations as the source owner.
    // Only parameter tapes and preserving dimension checks repeat per data
    // node; full schema/evidence reconstruction is paid once, not per key.
    let staging = conversion_staging_demand(plan.metadata, expanded, payload, plan.phases)
        .ok_or(ResidentKernelError::InvalidShape)?;
    let old_bytes = previous.map_or(0, |p| p.retained_bytes);
    let publication = sum(&[old_bytes, payload])?;
    let output_elements = if matches!(
        target.body(),
        SchemaBody::Set { .. } | SchemaBody::Matrix { .. }
    ) {
        root_count
    } else {
        1
    };
    let measured = meter.estimate();
    let output_bytes = sum(&[
        payload,
        mul(&[
            stats.sort_elements,
            size_of::<mech_core::snapshot::CanonicalKeyValue>() as u64,
        ])?,
    ])?;
    #[cfg(all(
        test,
        feature = "native-artifact",
        any(feature = "full_source", feature = "full_compiler"),
        any(feature = "pure_text", feature = "map_ops")
    ))]
    super::output_tests::record(
        output_bytes,
        stats.option_wrappers,
        owned_shape_parameters.is_none(),
    );
    let cost = budget::resident_cost! {
        comparison_work: sum(&[measured.comparison_work(), canonicalization_work, publication, staging.work.comparison])?,
        compute_work: sum(&[measured.compute_work(), canonicalization_work, publication,
            staging.work.compute, staging.work.canonicalization])?,
        output_elements,
        output_bytes,
        temporary_bytes: sum(&[input_bytes, old_bytes, staging.temporary_bytes])?,
        cloned_bytes: sum(&[input_bytes, staging.cloned_bytes])?,
        retained_nodes: sum(&[mul(&[input_nodes, 4])?, previous.map_or(0, |p| p.node_count),
            staging.retained_nodes, mul(&[stats.sort_elements, 4])?, 32])?,
        ..KernelCostEstimate::default()
    };
    Ok(Prepared {
        cost,
        canonicalization_work,
    })
}
