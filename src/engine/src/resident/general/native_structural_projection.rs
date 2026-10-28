//! Canonical child binding for a checked native structural site. This copies
//! one projected value; it neither schedules control flow nor executes a user
//! function. The generated caller still owns the current-visit proof.
use crate::native_artifact::{
    NativeCollectionInvocationError as Error, NativeIterationAction, NativeLibraryWorkBudget,
    NativeProgramArtifactPlan, NativeStructuralObservation, NativeStructuralStep,
};
use crate::resident::budget::{
    self, ResidentBudgetMeter, checked_cost_product as mul, checked_cost_sum as add,
};
use mech_core::{
    CurrentMemoryFootprint, MemoryBudgetLimits, ResidentKernelError, ResourceDemand, SchemaBody,
    SchemaTable, Value, ValueData, ValueDraft,
};
use std::sync::Arc;

/// Materialize the exact canonical child selected by a compiler-owned site.
/// Values must already belong to this invocation's memory account. Descriptive
/// paths/schema names cannot confer a library factory or execution permit.
pub fn native_project_structural_value(
    plan: &NativeProgramArtifactPlan,
    site: usize,
    schemas: &Arc<SchemaTable>,
    source: &Value,
    prior: Option<&Value>,
    limits: MemoryBudgetLimits,
    work: &NativeLibraryWorkBudget,
) -> Result<Value, Error> {
    let block = plan.blocks().get(site).ok_or(Error::InvalidInput)?;
    let NativeIterationAction::Structural {
        source: input,
        path,
        observation: NativeStructuralObservation::CanonicalValue,
        output,
    } = block.action()
    else {
        return Err(Error::InvalidInput);
    };
    super::native_collection_calls::finite_limits(block.origin().node(), limits)
        .map_err(Error::Binding)?;
    let input_type = plan
        .values()
        .get(input.get() as usize)
        .ok_or(Error::InvalidInput)?;
    let output_type = plan
        .values()
        .get(output.get() as usize)
        .ok_or(Error::InvalidInput)?;
    let expected = schemas
        .get(input_type.schema())
        .ok_or(Error::InvalidInput)?;
    let target = schemas
        .get(output_type.schema())
        .ok_or(Error::InvalidInput)?;
    if source.schema() != input_type.schema()
        || source.schema_key() != expected.key()
        || prior.is_some_and(|value| {
            value.schema() != output_type.schema() || value.schema_key() != target.key()
        })
    {
        return Err(Error::InvalidInput);
    }
    let memory = work.memory_budget();
    if source.memory_budget_retained_bytes(&memory).is_none()
        || prior.is_some_and(|value| value.memory_budget_retained_bytes(&memory).is_none())
    {
        return Err(Error::UnadmittedValue);
    }
    let compute = limits.max_compute_work.ok_or(Error::InvalidInput)?;
    let comparison = limits.max_comparison_work.ok_or(Error::InvalidInput)?;
    work.with_call_limits(compute, comparison, || -> Result<Value, Error> {
        let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
        let schema_work = add(&[
            budget::checked_u64(
                schemas
                    .entry(input_type.schema())
                    .ok_or(Error::InvalidInput)?
                    .canonical_bytes()
                    .len(),
            )?,
            budget::checked_u64(
                schemas
                    .entry(output_type.schema())
                    .ok_or(Error::InvalidInput)?
                    .canonical_bytes()
                    .len(),
            )?,
        ])?;
        meter.charge_compute_work(mul(&[schema_work, 4])?)?;
        if expected != input_type.definition() || target != output_type.definition() {
            return Err(Error::InvalidInput);
        }
        let source_footprint =
            budget::measure_borrowed_canonical_value_footprint(&mut meter, source, schemas)?;
        let prior_footprint = prior
            .map(|value| {
                budget::measure_borrowed_canonical_value_footprint(&mut meter, value, schemas)
            })
            .transpose()?;
        let mut body = expected.body();
        let mut data = source.data();
        for step in path {
            meter.charge_compute_work(32)?;
            (body, data) = match (step, body, data) {
                (
                    NativeStructuralStep::EnumPayload { ordinal },
                    SchemaBody::Enum { variants, .. },
                    ValueData::Enum(value),
                ) if value.ordinal() == *ordinal => {
                    let variant = variants.get(*ordinal as usize).ok_or(Error::InvalidInput)?;
                    (
                        variant.payload.as_ref().ok_or(Error::InvalidInput)?,
                        value.payload().ok_or(Error::InvalidInput)?,
                    )
                }
                (
                    NativeStructuralStep::TupleItem { ordinal },
                    SchemaBody::Tuple(fields),
                    ValueData::Tuple(values),
                ) => (
                    fields.get(*ordinal as usize).ok_or(Error::InvalidInput)?,
                    values.get(*ordinal as usize).ok_or(Error::InvalidInput)?,
                ),
                _ => return Err(Error::InvalidInput),
            };
        }
        if !crate::native_artifact::structural_values::carrier_component_matches(
            target.body(),
            body,
        ) || !crate::native_artifact::structural_values::supported(body)
        {
            return Err(Error::InvalidInput);
        }
        // Ownership import retains the complete schema arena. The same
        // prepaid traversal also bounds shape closure before any schema tree
        // or dimension witness is cloned below.
        let schema_bytes = schemas
            .clone_allocation_bound_bytes_with_precharge(|bytes| meter.charge_compute_work(bytes))
            .map_err(|error| match error {
                mech_core::SchemaAllocationBoundError::Precharge(error) => error,
                mech_core::SchemaAllocationBoundError::ArithmeticOverflow => {
                    ResidentKernelError::InvalidShape
                }
            })?;
        let shape_bytes = if target.dimension_parameters().is_empty() && target.body() == body {
            0
        } else {
            // Full source closure, the selected child, resolver witnesses,
            // and target closure coexist. Dimension closure only replaces
            // expressions with constants, so the retained schema clone bound
            // covers each tree; the multiplier also covers vector growth and
            // shape tapes. No buffer elements are expanded in this stage.
            add(&[mul(&[schema_bytes, 16])?, 4096])?
        };
        let (_shape_reservation, closed, shape) = if shape_bytes == 0 {
            (
                None,
                None,
                target
                    .instantiate_shape(Box::new([]))
                    .map_err(|_| Error::InvalidInput)?,
            )
        } else {
            let demand = ResourceDemand {
                turn_peak_bytes: add(&[
                    source_footprint.retained_bytes,
                    prior_footprint.map_or(0, |v| v.retained_bytes),
                    shape_bytes,
                ])?,
                retained_nodes: add(&[
                    source_footprint.node_count,
                    prior_footprint.map_or(0, |v| v.node_count),
                ])?,
                ..ResourceDemand::default()
            };
            if let Some(violation) = mech_core::evaluate_call_memory_budget(
                mech_core::MemoryObjectOwner::NodeOutput {
                    node: block.origin().node(),
                    port: 0,
                },
                demand,
                0,
                0,
                limits,
            )
            .first()
            {
                return Err(Error::Plan(
                    mech_core::MemoryPlanError::TargetLimitExceeded {
                        violation: violation.clone(),
                    },
                ));
            }
            meter.charge_compute_work(mul(&[shape_bytes, 8])?)?;
            let reservation = memory.reserve_capacity(shape_bytes)?;
            let source_body = expected
                .closed_body(source.shape())
                .map_err(|_| Error::InvalidInput)?;
            let mut selected = &source_body;
            for step in path {
                selected = match (step, selected) {
                    (
                        NativeStructuralStep::EnumPayload { ordinal },
                        SchemaBody::Enum { variants, .. },
                    ) => variants
                        .get(*ordinal as usize)
                        .and_then(|variant| variant.payload.as_ref())
                        .ok_or(Error::InvalidInput)?,
                    (NativeStructuralStep::TupleItem { ordinal }, SchemaBody::Tuple(fields)) => {
                        fields.get(*ordinal as usize).ok_or(Error::InvalidInput)?
                    }
                    _ => return Err(Error::InvalidInput),
                };
            }
            let closed = selected.clone();
            let shape = mech_core::shape_for_schema_components(
                target,
                &[(target.body(), closed.clone())],
                None,
            )
            .map_err(|_| Error::InvalidInput)?;
            (Some(reservation), Some(closed), shape)
        };
        let body = closed.as_ref().unwrap_or(body);
        let item = budget::measure_canonical_data_footprint(&mut meter, body, data)?;
        let finalization_work =
            budget::preflight_canonical_data_finalization(&mut meter, body, data)?;
        let published =
            budget::projected_canonical_value_footprint(item, shape.parameter_values().len())?;
        let footprint = CurrentMemoryFootprint {
            logical_elements: item.node_count,
            payload_bytes: item.retained_bytes,
            encoded_bytes: item.encoded_bytes,
            retained_nodes: item.node_count,
            shape_parameter_count: budget::checked_u64(shape.parameter_values().len())?,
            schema_bytes: budget::checked_u64(
                schemas
                    .entry(output_type.schema())
                    .ok_or(Error::InvalidInput)?
                    .canonical_bytes()
                    .len(),
            )?,
            ..CurrentMemoryFootprint::default()
        };
        let owner_envelope = add(&[
            mul(&[
                add(&[
                    schema_bytes,
                    Value::canonical_owner_allocation_bytes(),
                    Value::memory_budget_claim_metadata_bytes(),
                ])?,
                4,
            ])?,
            4096,
        ])?;
        let packed = crate::resident::structural_values::word_matrix(body);
        let draft_bytes = if packed {
            // The typed constructor consumes one packed clone. It never
            // allocates an intermediate draft or canonical cell per word.
            0
        } else {
            add(&[
                budget::unsigned_word_draft_expansion(&mut meter, body, data)?,
                mech_core::canonical_snapshot_draft_bytes(footprint)?,
            ])?
        };
        let workspace = add(&[
            owner_envelope,
            draft_bytes,
            mech_core::canonical_snapshot_finalization_bytes(footprint)?,
            mul(&[Value::memory_budget_claim_metadata_bytes(), 2])?,
        ])?;
        let nodes = add(&[
            source_footprint.node_count,
            prior_footprint.map_or(0, |v| v.node_count),
            mul(&[published.node_count, 3])?,
        ])?;
        let demand = ResourceDemand {
            output_elements: 1,
            persistent_bytes: published.retained_bytes,
            cloned_bytes: item.retained_bytes,
            turn_peak_bytes: add(&[
                source_footprint.retained_bytes,
                prior_footprint.map_or(0, |v| v.retained_bytes),
                shape_bytes,
                workspace,
            ])?,
            retained_nodes: nodes,
            ..ResourceDemand::default()
        };
        if let Some(violation) = mech_core::evaluate_call_memory_budget(
            mech_core::MemoryObjectOwner::NodeOutput {
                node: block.origin().node(),
                port: 0,
            },
            demand,
            published.retained_bytes,
            0,
            limits,
        )
        .first()
        {
            return Err(Error::Plan(
                mech_core::MemoryPlanError::TargetLimitExceeded {
                    violation: violation.clone(),
                },
            ));
        }
        // The common constructors own all semantic validation. Admit copy,
        // finalization, import and failure retirement before the first draft.
        meter.charge_compute_work(add(&[mul(&[workspace, 8])?, 128])?)?;
        meter.charge_comparison_work(finalization_work)?;
        let mut reservation = memory.reserve_capacity(workspace)?;
        let canonical = mech_core::snapshot::SnapshotCanonicalizationBudget::new(finalization_work);
        let context = mech_core::snapshot::SnapshotValidationContext::with_shared_schemas(schemas)
            .with_canonicalization_budget(&canonical);
        let value = if packed {
            crate::resident::packed_word_matrix::copy(
                output_type.schema(),
                shape,
                data,
                Arc::clone(schemas),
            )?
        } else {
            let data = mech_core::snapshot::canonical_snapshot_data_draft_with_context(
                body, data, &context,
            )
            .map_err(|_| ResidentKernelError::InvalidInput)?;
            ValueDraft {
                schema: output_type.schema(),
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data,
            }
            .finalize(&context)
            .map_err(|_| ResidentKernelError::InvalidOutput)?
        };
        Ok(value.into_memory_budget(&mut reservation)?)
    })?
}
