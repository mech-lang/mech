//! Select an owner-backed private line matrix through the SAME maintained
//! positional geometry, declared shape solver and TextJoin append leaf.
use super::*;
use crate::resident::numeric::selected_string_matrix;

impl NativePreparedLibraryCall {
    pub fn selected_text_line_matrix(
        &self,
        lines: &Rc<NativeTextLineMatrix>,
        selector: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<NativeSelectedStringMatrix, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if self.template.primitive() != Primitive::MatrixRangeAccess
            || self.input_keys.len() != 2
            || contract.inputs.len() != 2
            || self.selection_shape_bounds.0 == 0
            || lines.schema() != contract.inputs[0].schema
            || lines.schema_key() != self.input_keys[0]
            || lines.program_revision() != self.template.program_revision()
            || selector.schema() != contract.inputs[1].schema
            || selector.schema_key() != self.input_keys[1]
        {
            return Err(InvokeError::InvalidInput);
        }
        let [source_schema, selector_schema] = self.template.inputs() else {
            return Err(InvokeError::InvalidInput);
        };
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = source_schema.body()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if element.as_ref() != &SchemaBody::String
            || dimensions.len() != 2
            || !matches!(selector_schema.body(), SchemaBody::Matrix { element, .. }
                if element.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.lookup_work, 0)?;
                let memory = work.memory_budget();
                let source_bytes = lines.retained_bytes(&memory)?;
                let selector_bytes = selector
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                let ValueData::Matrix(indices) = selector.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                if !matches!(indices.elements(), SequenceView::U64(_)) {
                    return Err(InvokeError::InvalidInput);
                }
                let count = indices.elements().len() as u64;
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    add(source_bytes, selector_bytes)?,
                )?;
                coexistence.retained_nodes = add(
                    coexistence.retained_nodes,
                    add(add(lines.count() as u64, count)?, 16)?,
                )?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                let binding_bytes = add(self.binding_bytes, self.selection_shape_bounds.0)?;
                context.check_capacity(binding_bytes, self.binding_nodes)?;
                context.charge(
                    add(
                        self.binding_work,
                        add(self.selection_shape_bounds.1, add(mul(count, 128)?, 256)?)?,
                    )?,
                    0,
                )?;
                let _binding = memory.reserve_capacity(binding_bytes)?;
                // This is binding metadata only: no canonical source Matrix is
                // created or passed to a kernel. The opaque line owner supplied
                // its actual logical shape through the original text/lines leaf.
                let output_parameters = vec![0; self.output_parameters];
                let call = super::super::bind_current(
                    &self.schemas,
                    self.template.clone(),
                    self.factory,
                    &[
                        lines.shape().parameter_values(),
                        selector.shape().parameter_values(),
                    ],
                    &output_parameters,
                    limits,
                )
                .map_err(InvokeError::Binding)?;
                let rows = usize::try_from(
                    lines
                        .shape()
                        .resolve_dimension(&dimensions[0])
                        .map_err(|_| InvokeError::InvalidInput)?,
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let columns = usize::try_from(
                    lines
                        .shape()
                        .resolve_dimension(&dimensions[1])
                        .map_err(|_| InvokeError::InvalidInput)?,
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let geometry = selected_string_matrix::prepare_cells(
                    &call.kernel,
                    (rows, columns),
                    lines.count(),
                    selector,
                    |i| lines.line(i),
                )?;
                Ok(NativeSelectedStringMatrix {
                    source: Source::Lines(lines.clone()),
                    selector: selector.clone(),
                    geometry,
                    schema: contract.outputs[0].schema,
                    key: self.output_key,
                    revision: self.template.program_revision(),
                })
            },
        )?
    }
}
