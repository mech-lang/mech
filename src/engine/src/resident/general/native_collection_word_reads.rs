//! A selected linear unsigned word read from a genuine immutable canonical matrix.
//! No selector/result Value, temporary vector, or runtime call plan is built.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::{
    native_artifact::NativeCollectionPrimitive as Primitive,
    resident::budget::{
        NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
    },
};
use mech_core::{
    IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData,
    snapshot::SequenceView,
};

impl NativePreparedLibraryCall {
    fn unsigned_matrix_element_width(&self) -> Option<IntegerWidth> {
        let [matrix, selector] = self.template.inputs() else {
            return None;
        };
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = matrix.body()
        else {
            return None;
        };
        let SchemaBody::UnsignedInteger(width) = element.as_ref() else {
            return None;
        };
        if !matches!(
            width,
            IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64
        ) || dimensions.len() != 2
            || self.template.primitive() != Primitive::MatrixScalarAccess
            || selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !selector.dimension_parameters().is_empty()
            || self.template.output().body() != element.as_ref()
            || !self.template.output().dimension_parameters().is_empty()
        {
            return None;
        }
        Some(*width)
    }

    /// Only the privately witnessed maintained linear scalar access qualifies.
    pub fn supports_unsigned_matrix_element(&self) -> bool {
        self.unsigned_matrix_element_width().is_some()
    }

    /// Compatibility with the original exact U64 projection contract.
    pub fn supports_u64_matrix_element(&self) -> bool {
        self.unsigned_matrix_element_width() == Some(IntegerWidth::W64)
    }

    pub fn u64_matrix_element(
        &self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if !self.supports_u64_matrix_element() {
            return Err(InvokeError::InvalidInput);
        }
        self.unsigned_matrix_element(source, index, limits, work, coexistence)
    }

    /// Return the scalar word into already admitted native storage. The caller
    /// supplies its complete current owner/caller census and original visit.
    /// This pure read grants no visit, loop, output-publication, or owner permit.
    pub fn unsigned_matrix_element(
        &self,
        source: &Value,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let width = self
            .unsigned_matrix_element_width()
            .ok_or(InvokeError::InvalidInput)?;
        let output_bytes = match width {
            IntegerWidth::W8 => 1,
            IntegerWidth::W16 => 2,
            IntegerWidth::W32 => 4,
            IntegerWidth::W64 => 8,
            _ => return Err(InvokeError::InvalidInput),
        };
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                // Include the result even though its physical scalar lane already
                // belongs to the native frame. No new canonical owner is invented.
                coexistence.output_elements = coexistence
                    .output_elements
                    .checked_add(1)
                    .ok_or(InvokeError::InvalidInput)?;
                // Logical output limits still describe the exact unsigned width;
                // absence of a new allocation does not waive those limits.
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    ResourceDemand {
                        output_elements: 1,
                        ..ResourceDemand::default()
                    },
                    output_bytes,
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
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(self.lookup_work, 0)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
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
                let bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let nodes = source
                    .memory_budget_retained_nodes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                // Call-scoped coexistence includes the actual immutable owner
                // and two already admitted scalar words, not unrelated roots
                // elsewhere in the program's separate shared memory account.
                let context = context.with_added_capacity(
                    bytes.checked_add(16).ok_or(InvokeError::InvalidInput)?,
                    nodes.checked_add(2).ok_or(InvokeError::InvalidInput)?,
                )?;
                context.check_capacity(0, 0)?;
                context.charge(128, 0)?;
                let ValueData::Matrix(matrix) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let cells = matrix.elements();
                // Reuse the maintained linear scalar selector's one-based check
                // and error, including zero, empty input, and oversized ordinals.
                let ordinal =
                    crate::resident::numeric::selected_string_matrix::available_scalar_index(
                        index,
                        cells.len(),
                    )?;
                match (width, cells) {
                    (IntegerWidth::W8, SequenceView::U8(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (IntegerWidth::W16, SequenceView::U16(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (IntegerWidth::W32, SequenceView::U32(values)) => values
                        .get(ordinal)
                        .copied()
                        .map(u64::from)
                        .ok_or(InvokeError::InvalidInput),
                    (IntegerWidth::W64, SequenceView::U64(values)) => values
                        .get(ordinal)
                        .copied()
                        .ok_or(InvokeError::InvalidInput),
                    (width, SequenceView::Values(values)) => match (width, values.get(ordinal)) {
                        (IntegerWidth::W8, Some(ValueData::U8(value))) => Ok(u64::from(*value)),
                        (IntegerWidth::W16, Some(ValueData::U16(value))) => Ok(u64::from(*value)),
                        (IntegerWidth::W32, Some(ValueData::U32(value))) => Ok(u64::from(*value)),
                        (IntegerWidth::W64, Some(ValueData::U64(value))) => Ok(*value),
                        _ => Err(InvokeError::InvalidInput),
                    },
                    _ => Err(InvokeError::InvalidInput),
                }
            },
        )?
    }
}
