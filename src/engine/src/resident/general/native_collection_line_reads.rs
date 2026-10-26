//! Checked native-word/borrowed-String consumers of a private line owner.
//! No canonical matrix or result allocation is invented for these reads.
use super::*;
use crate::native_artifact::NativeCollectionPrimitive as Primitive;
use mech_core::{IntegerWidth, ResolvedOperationContract};

impl NativePreparedLibraryCall {
    /// Successful private projection of an actual completed cell before EOF.
    /// The generated site's proof and caller establish the producer association.
    /// This grants no visit or source admission, and creates no partial Matrix
    /// or logical length; declared final shape checks remain due at EOF.
    pub fn text_line_window_element<'a>(
        &self,
        window: &'a NativeTextLineWindow<'_>,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<&'a str, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let [matrix, selector] = self.template.inputs() else {
            return Err(InvokeError::InvalidInput);
        };
        if self.template.primitive() != Primitive::MatrixScalarAccess
            || !matches!(matrix.body(), SchemaBody::Matrix { element, dimensions }
                if element.as_ref() == &SchemaBody::String && dimensions.len() == 2)
            || selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !selector.dimension_parameters().is_empty()
            || self.template.output().body() != &SchemaBody::String
            || !self.template.output().dimension_parameters().is_empty()
            || !window.shares_work(work)
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    window.retained_capacity_bytes(),
                )?;
                coexistence.retained_nodes = add(coexistence.retained_nodes, 1)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(add(self.lookup_work, 128)?, 0)?;
                let available = usize::try_from(window.available_lines())
                    .map_err(|_| InvokeError::InvalidInput)?;
                let ordinal =
                    crate::resident::numeric::selected_string_matrix::available_scalar_index(
                        index, available,
                    )?;
                window
                    .line(ordinal as u64 + 1)
                    .ok_or(InvokeError::InvalidInput)
            },
        )?
    }

    fn validate_line_owner(
        &self,
        lines: &NativeTextLineMatrix,
        memory: &ManagedMemoryBudget,
    ) -> Result<u64, InvokeError> {
        let ResolvedOperationContract::Declared(contract) = self.template.contract() else {
            return Err(InvokeError::InvalidInput);
        };
        if self.input_keys.is_empty()
            || contract.inputs.is_empty()
            || lines.schema() != contract.inputs[0].schema
            || lines.schema_key() != self.input_keys[0]
            || lines.program_revision() != self.template.program_revision()
        {
            return Err(InvokeError::InvalidInput);
        }
        lines.retained_bytes(memory)
    }

    /// Return an already allocated native scalar word. Publication or a later
    /// canonical use still constructs/adopts its real U64 through the backend.
    pub fn text_line_matrix_length(
        &self,
        lines: &NativeTextLineMatrix,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<u64, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::SequenceLength
            || self.template.inputs().len() != 1
            || self.template.output().body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !self.template.output().dimension_parameters().is_empty()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    self.validate_line_owner(lines, &memory)?,
                )?;
                coexistence.retained_nodes =
                    add(coexistence.retained_nodes, add(lines.count() as u64, 1)?)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(0, 0)?;
                context.charge(add(self.lookup_work, 128)?, 0)?;
                // text/lines proved a 1-by-count matrix with this exact schema and
                // shape. Cardinality therefore agrees with ordinary matrix length.
                Ok(lines.count() as u64)
            },
        )?
    }

    /// Bind the actual scalar selector and reuse its one-based error check.
    /// Successful Strings remain borrowed and cannot outlive their real owner.
    pub fn text_line_matrix_element<'a>(
        &self,
        lines: &'a NativeTextLineMatrix,
        index: u64,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<&'a str, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let [_, selector] = self.template.inputs() else {
            return Err(InvokeError::InvalidInput);
        };
        if self.template.primitive() != Primitive::MatrixScalarAccess
            || selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !selector.dimension_parameters().is_empty()
            || self.template.output().body() != &SchemaBody::String
            || !self.template.output().dimension_parameters().is_empty()
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let memory = work.memory_budget();
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    self.validate_line_owner(lines, &memory)?,
                )?;
                coexistence.retained_nodes =
                    add(coexistence.retained_nodes, add(lines.count() as u64, 1)?)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                context.check_capacity(self.binding_bytes, self.binding_nodes)?;
                context.charge(add(self.lookup_work, add(self.binding_work, 128)?)?, 0)?;
                let _binding = memory.reserve_capacity(self.binding_bytes)?;
                let call = super::super::bind_current(
                    &self.schemas,
                    self.template.clone(),
                    self.factory,
                    &[lines.shape().parameter_values(), &[]],
                    &[],
                    limits,
                )
                .map_err(InvokeError::Binding)?;
                let ordinal = crate::resident::numeric::selected_string_matrix::scalar_index(
                    &call.kernel,
                    index,
                    lines.count(),
                )?;
                lines.line(ordinal).ok_or(InvokeError::InvalidInput)
            },
        )?
    }
}
