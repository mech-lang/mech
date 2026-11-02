//! Conversion-only demand added before managed candidate materialization.
#[cfg(all(test, feature = "matrix", feature = "set", feature = "u8"))]
#[path = "kind_conversion_set_sort_tests.rs"]
mod sort_tests;
#[cfg(all(test, feature = "matrix", feature = "set", feature = "u8"))]
#[path = "kind_conversion_set_demand_tests.rs"]
mod tests;
use super::*;

fn overflow(field: &'static str) -> MechError {
    MechError::new(
        mech_core::MemoryPlanError::ArithmeticOverflow { field },
        None,
    )
    .with_compiler_loc()
}
fn add(a: u64, b: u64) -> MResult<u64> {
    a.checked_add(b)
        .ok_or_else(|| overflow("conversion admitted staging sum"))
}
fn multiply(a: u64, b: u64) -> MResult<u64> {
    a.checked_mul(b)
        .ok_or_else(|| overflow("conversion admitted staging product"))
}

fn current_population(
    source: &ValueCell,
    plan: &ConversionPlan,
    known: Option<CurrentMemoryFootprint>,
) -> MResult<(u64, u64)> {
    let (expanded, work) = if let Some(actual) = source.current_canonical_draft_population()? {
        actual
    } else {
        match source.storage_capabilities().topology {
            StorageTopology::Scalar(_) | StorageTopology::DenseSequence { .. } => (),
            _ => {
                return Err(MechError::new(
                    mech_core::MemoryPlanError::UnsupportedStorageLayout,
                    None,
                )
                .with_compiler_loc());
            }
        }
        let footprint = match known {
            Some(value) => value,
            None => source.current_memory_footprint()?,
        };
        (
            add(
                add(footprint.retained_nodes, footprint.logical_elements)?,
                1,
            )?,
            0,
        )
    };
    Ok((
        multiply(
            expanded,
            add(conversion_option_present_depth(&plan.step), 1)?,
        )?,
        work,
    ))
}

pub(super) fn conversion_publication_population(
    source: &ValueCell,
    plan: &ConversionPlan,
    footprint: &mut CurrentMemoryFootprint,
) -> MResult<u64> {
    let (expanded, _) = current_population(source, plan, None)?;
    // Root logical elements miss packed lanes under Option and other wrappers.
    // Fund complete frozen key/value carriers, prefixes and logical nodes,
    // independently of the private draft/sort staging augmentation.
    let carriers = multiply(
        expanded,
        (core::mem::size_of::<ValueData>()
            + core::mem::size_of::<mech_core::snapshot::CanonicalKeyValue>()) as u64,
    )?;
    footprint.payload_bytes = add(footprint.payload_bytes, carriers)?;
    footprint.encoded_bytes = add(footprint.encoded_bytes, multiply(expanded, 16)?)?;
    footprint.retained_nodes = add(footprint.retained_nodes, multiply(expanded, 4)?)?;
    Ok(expanded)
}

pub(super) fn conversion_execution_demand(
    source: &ValueCell,
    output: &ValueCell,
    plan: &ConversionPlan,
    request: &mech_core::ExecutionDemandPlanningRequest<'_>,
) -> MResult<mech_core::PlannedExecutionDemand> {
    let mut demand = super::conversion_input_reconstruction_demand(source, output, request)?;
    if !plan.step.contains_matrix_to_set() {
        return Ok(demand);
    }
    let footprint = request
        .input_footprint(0)
        .map_err(|error| MechError::new(error, None).with_compiler_loc())?;
    let descriptor = request
        .inputs
        .first()
        .ok_or_else(|| overflow("conversion source descriptor"))?;
    let arena = output.retained_schema_table();
    let target = arena
        .get(output.schema())
        .ok_or_else(|| overflow("conversion target schema"))?;
    let metadata = conversion_schema_staging_bound(descriptor.schema(), target)
        .ok_or_else(|| overflow("conversion schema staging"))?;
    let (expanded, observation_work) = current_population(source, plan, Some(footprint))?;
    let payload = add(footprint.payload_bytes, footprint.fixed_bytes)?.max(multiply(expanded, 32)?);
    let payload = if let Some(bytes) = super::conversion_string_payload_bound(&plan.step) {
        add(payload, multiply(expanded, bytes)?)?
    } else {
        payload
    };
    let phases = conversion_materialization_phases(&plan.step);
    let (sort_work, sort_observation) =
        match source.current_canonical_conversion_comparison_work(target, plan, payload, phases)? {
            Some(actual) => actual,
            None => {
                if !matches!(
                    source.storage_capabilities().topology,
                    StorageTopology::DenseSequence { .. }
                ) {
                    return Err(MechError::new(
                        mech_core::MemoryPlanError::UnsupportedStorageLayout,
                        None,
                    )
                    .with_compiler_loc());
                }
                let count = usize::try_from(footprint.logical_elements)
                    .map_err(|_| overflow("dense conversion comparison population"))?;
                (
                    conversion_dense_comparison_work(
                        descriptor.schema().body(),
                        target.body(),
                        &plan.step,
                        count,
                        footprint.payload_bytes,
                        payload,
                        phases,
                    )
                    .ok_or_else(|| overflow("dense conversion comparison bound"))?,
                    0,
                )
            }
        };
    let extra = conversion_staging_demand(metadata, expanded, payload, phases)
        .ok_or_else(|| overflow("conversion complete private staging"))?;
    demand.temporary_bytes = add(demand.temporary_bytes, extra.temporary_bytes)?;
    demand.cloned_bytes = add(demand.cloned_bytes, extra.cloned_bytes)?;
    demand.retained_nodes = add(demand.retained_nodes, extra.retained_nodes)?;
    // Prospective planning, execution planning, and stage_conversion_output each
    // observe the current population. Sort facts are observed only by this hook.
    demand.work.compute = add(
        add(add(demand.work.compute, extra.work.compute)?, sort_work)?,
        add(multiply(observation_work, 3)?, sort_observation)?,
    )?;
    demand.work.comparison = add(
        add(demand.work.comparison, extra.work.comparison)?,
        sort_work,
    )?;
    demand.work.canonicalization = add(
        add(demand.work.canonicalization, extra.work.canonicalization)?,
        sort_work,
    )?;
    demand.work.scalar_instructions = add(
        demand.work.scalar_instructions,
        extra.work.scalar_instructions,
    )?;
    Ok(demand)
}
