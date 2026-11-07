//! One selected String projection with a native scalar selector. The source
//! is already immutable and admitted; no general binding or source walk is due.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::NativeScalarValue;
use crate::resident::budget::{
    self, NativeLibraryWorkBudget, ResidentBudgetMeter,
    native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{
    MemoryBudgetLimits, ResourceDemand, Value, ValueData,
    snapshot::{
        SequenceView, SnapshotValidationContext, StringSnapshotConstructor, ValueFootprint,
        prospective_string_value_footprint,
    },
};

fn add(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_add(b).ok_or(InvokeError::InvalidInput)
}
fn mul(a: u64, b: u64) -> Result<u64, InvokeError> {
    a.checked_mul(b).ok_or(InvokeError::InvalidInput)
}

impl NativePreparedLibraryCall {
    pub fn supports_string_matrix_element(&self) -> bool {
        // Reuse the genuine selected factory's exact two-dimensional
        // String/U64/String contract, including its empty scalar shapes.
        self.supports_string_matrix_key_loan()
    }

    /// Materialize one selected canonical String. The native caller supplies
    /// every OTHER live owner/cache and retains the original source visit.
    /// This is a projection only, never a loop executor or Map operation.
    /// Nonpacked storage declines before any work or output allocation.
    pub fn string_matrix_element(
        &self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Option<Value>, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_string_matrix_element() {
            return Ok(None);
        }
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if source.schema() != contract.inputs[0].schema
            || source.schema_key() != self.input_keys[0]
            || source.shape().parameter_values().len()
                != self.template.inputs()[0].dimension_parameters().len()
        {
            return Err(InvokeError::InvalidInput);
        }
        source
            .validate_against(&self.schemas)
            .map_err(|_| InvokeError::InvalidInput)?;
        let memory = work.memory_budget();
        let source_bytes = source
            .memory_budget_retained_bytes(&memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        let source_nodes = source
            .memory_budget_retained_nodes(&memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        let ValueData::Matrix(matrix) = source.data() else {
            return Err(InvokeError::InvalidInput);
        };
        let SequenceView::String(strings) = matrix.elements() else {
            return Ok(None);
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(source_bytes, source_nodes)?;
                context.check_capacity(0, 0)?;
                // Fund schema/shape checks, the original scalar selector and
                // fixed native projection before inspecting dynamic geometry.
                context.charge(add(add(self.lookup_work, self.binding_work)?, 256)?, 0)?;
                // Match the maintained scalar access error order: an output
                // element cap is checked before traversing the selected index.
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: add(coexistence.output_elements, 1)?,
                        ..ResourceDemand::default()
                    },
                    0,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                let linear =
                    crate::resident::numeric::available_scalar_index(index, strings.len())?;
                let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                    source,
                    self.template.inputs()[0].body(),
                )?;
                if rows.checked_mul(columns) != Some(strings.len()) {
                    return Err(InvokeError::InvalidInput);
                }
                // Canonical storage is row-major; Mech linear access is
                // column-major. The original one-based check precedes division.
                let ordinal = if rows == 1 || columns == 1 {
                    linear
                } else {
                    (linear % rows) * columns + linear / rows
                };
                let text = strings.get(ordinal).ok_or(InvokeError::InvalidInput)?;
                let payload = text.len() as u64;
                let published = prospective_string_value_footprint(0, false, 1, payload)
                    .map_err(|_| InvokeError::InvalidInput)?;
                // Obtain the original selector/element logical clone floors
                // from the maintained footprint vocabulary. This is bounded
                // packed-cell metadata work, prepaid above, never a source walk.
                let mut meter = ResidentBudgetMeter::for_closed_analysis(u64::MAX, u64::MAX);
                let selector = budget::measure_native_scalar_frame_footprint(
                    &mut meter,
                    self.template.inputs()[1].body(),
                    NativeScalarValue::U64(index),
                )?;
                let mut element = ValueFootprint::default();
                let mut finalization = 0;
                crate::resident::numeric::selected_sequence_footprint_with_finalization(
                    &mut element,
                    &mut finalization,
                    &mut meter,
                    self.template.output().body(),
                    matrix.elements(),
                    ordinal,
                )?;
                let cost = meter.estimate();
                context.charge(
                    add(cost.compute_work(), finalization)?,
                    add(cost.comparison_work(), finalization)?,
                )?;
                // Retain logical output and clone caps despite replacing the
                // general selector Value and its binding/turn-plan machinery.
                let demand = ResourceDemand {
                    output_elements: add(coexistence.output_elements, 1)?,
                    cloned_bytes: add(
                        coexistence.cloned_bytes,
                        add(selector.retained_bytes, element.retained_bytes)?,
                    )?,
                    ..ResourceDemand::default()
                };
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    demand,
                    published.retained_bytes,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
                    ));
                }
                // One copied String plus canonical metadata, claim/import and
                // failure retirement. Physical schema storage is shared, but
                // this independent Value import still owes its complete schema
                // owner admission allowance. The immutable table caches this
                // bound; no schema traversal or clone occurs for each element.
                let schema_bytes = self
                    .schemas
                    .clone_allocation_bound_bytes()
                    .ok_or(InvokeError::InvalidInput)?;
                let workspace = add(
                    mul(
                        add(
                            add(
                                add(published.retained_bytes, schema_bytes)?,
                                Value::canonical_owner_allocation_bytes(),
                            )?,
                            Value::memory_budget_claim_metadata_bytes(),
                        )?,
                        4,
                    )?,
                    4096,
                )?;
                context.check_capacity(workspace, mul(published.node_count, 3)?)?;
                context.charge(add(mul(workspace, 8)?, 128)?, 0)?;
                let mut reservation = memory.reserve_capacity(workspace)?;
                let context = SnapshotValidationContext::with_shared_schemas(&self.schemas);
                let constructor = StringSnapshotConstructor::new(
                    contract.outputs[0].schema,
                    Box::new([]),
                    text.len(),
                    &context,
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let mut owned = String::new();
                owned
                    .try_reserve_exact(text.len())
                    .map_err(|_| InvokeError::InvalidInput)?;
                if owned.capacity() != text.len() {
                    return Err(InvokeError::InvalidInput);
                }
                owned.push_str(text);
                let value = constructor
                    .finish(owned)
                    .map_err(|_| InvokeError::InvalidInput)?;
                Ok(Some(value.into_memory_budget(&mut reservation)?))
            },
        )?
    }
}
