//! Validate ONE selected tuple/record pack without materializing its child payloads.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{NativeCollectionPrimitive as Primitive, NativeLibraryWorkBudget};
use crate::resident::budget::{
    ResidentBudgetMeter, measure_borrowed_canonical_value_footprint,
    native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{
    MemoryBudgetLimits, ResourceDemand, Value, snapshot::CompositeSnapshotConstructor,
};

impl NativePreparedLibraryCall {
    /// Validate the exact selected field operation for a compiler-proved private
    /// record. The pack already checked the complete record shape; its actual
    /// canonical child remains the output owner, without a payload clone.
    pub fn admit_buffer_record_field(
        &self,
        selector: &Value,
        ordinal: usize,
        child: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::RecordFieldAccess
            || self.template.inputs().len() != 2
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if selector.schema() != contract.inputs[1].schema
                    || selector.schema_key() != self.input_keys[1]
                    || child.schema() != contract.outputs[0].schema
                    || child.schema_key() != self.output_key
                {
                    return Err(InvokeError::InvalidInput);
                }
                selector
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                child
                    .validate_against(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let retained = child
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let nodes = child
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let selector_bytes = selector
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let selector_nodes = selector
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        retained
                            .checked_add(selector_bytes)
                            .ok_or(InvokeError::InvalidInput)?,
                        nodes
                            .checked_add(selector_nodes)
                            .ok_or(InvokeError::InvalidInput)?,
                    )?;
                context.check_capacity(0, 0)?;
                context.charge(
                    self.lookup_work
                        .checked_add(128)
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let (mech_core::SchemaBody::Record(fields), mech_core::ValueData::Id(selected)) =
                    (self.template.inputs()[0].body(), selector.data())
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let actual = crate::resident::numeric::named_schema_ordinal(
                    fields.iter().map(|f| f.name.as_str()),
                    *selected,
                    |characters| {
                        context.charge(
                            characters
                                .checked_mul(16)
                                .and_then(|n| n.checked_add(64))
                                .ok_or(mech_core::ResidentKernelError::InvalidShape)?,
                            characters,
                        )
                    },
                )?;
                if actual != ordinal {
                    return Err(InvokeError::InvalidInput);
                }
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let footprint =
                    measure_borrowed_canonical_value_footprint(&mut meter, child, &self.schemas)?;
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    footprint.retained_bytes,
                    0,
                    limits,
                )
                .first()
                {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation: violation.clone(),
                        },
                    ));
                }
                Ok(())
            },
        )?
    }

    /// The generated caller proves private frame uses and retains the actual
    /// already-admitted child owners in its planned invocation-local lanes.
    /// This method supplies neither a visit grant nor a canonical tuple Value.
    pub fn admit_buffer_frame(
        &self,
        inputs: &[Option<Value>],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::CompositePack(inputs.len())
            || !matches!(
                self.template.output().body(),
                mech_core::SchemaBody::Tuple(_) | mech_core::SchemaBody::Record(_)
            )
            || self.template.inputs().len() != inputs.len()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                let metadata = self
                    .binding_bytes
                    .checked_add(
                        self.schemas
                            .clone_allocation_bound_bytes()
                            .ok_or(InvokeError::InvalidInput)?,
                    )
                    .and_then(|n| n.checked_add((inputs.len() as u64).checked_mul(1024)?))
                    .and_then(|n| n.checked_add(4096))
                    .ok_or(InvokeError::InvalidInput)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                // Prepay the allocation-free schema preflight, then use the
                // core constructor's own shape/closure witness. Arity alone
                // does not bound dimension expressions or solver metadata.
                context.charge(metadata.checked_mul(8).ok_or(InvokeError::InvalidInput)?, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                let output = contract.outputs[0].schema;
                let normalization = CompositeSnapshotConstructor::binding_cost_for_inputs(
                    output,
                    inputs.len(),
                    |index| Some(contract.inputs[index].schema),
                    &self.schemas,
                )
                .ok_or(InvokeError::InvalidInput)?;
                let metadata = metadata
                    .checked_add(normalization.temporary_bytes)
                    .ok_or(InvokeError::InvalidInput)?;
                let nodes = normalization
                    .metadata_nodes
                    .checked_add(inputs.len() as u64)
                    .and_then(|n| n.checked_add(8))
                    .ok_or(InvokeError::InvalidInput)?;
                context.check_capacity(metadata, nodes)?;
                context.charge(
                    normalization
                        .temporary_bytes
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(normalization.compute_work))
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let _admission = memory.reserve_capacity(metadata)?;
                let _nodes = memory.reserve_nodes(nodes)?;
                let mut children = Vec::with_capacity(inputs.len());
                let mut meter = ResidentBudgetMeter::for_native_projection(work, limits)?;
                let mut logical_bytes = 0u64;
                for (index, value) in inputs.iter().enumerate() {
                    let value = value.as_ref().ok_or(InvokeError::InvalidInput)?;
                    if value.schema_key() != self.input_keys[index]
                        || value.memory_budget_retained_bytes(&memory).is_none()
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                    value
                        .validate_against(&self.schemas)
                        .map_err(|_| InvokeError::InvalidInput)?;
                    let footprint = measure_borrowed_canonical_value_footprint(
                        &mut meter,
                        value,
                        &self.schemas,
                    )?;
                    logical_bytes = logical_bytes
                        .checked_add(footprint.retained_bytes)
                        .ok_or(InvokeError::InvalidInput)?;
                    children.push((value.schema(), value.shape().clone()));
                }
                let shape = CompositeSnapshotConstructor::shape_for_children(
                    output,
                    &children,
                    &self.schemas,
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let _constructor = CompositeSnapshotConstructor::bind(
                    output,
                    shape,
                    &children,
                    self.schemas.clone(),
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                // Retain the logical output bound even though no payload is copied.
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: inputs.len() as u64,
                        ..ResourceDemand::default()
                    },
                    logical_bytes,
                    0,
                    limits,
                )
                .first()
                {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded {
                            violation: violation.clone(),
                        },
                    ));
                }
                Ok(())
            },
        )?
    }
}
