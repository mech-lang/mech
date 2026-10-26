//! Select an owner-backed private line matrix through the SAME maintained
//! positional geometry, declared shape solver and TextJoin append leaf.
use super::*;
use crate::resident::numeric::selected_string_matrix;

#[derive(Clone, Copy)]
enum SelectorInput<'a> {
    Canonical(&'a Value),
    UnitRange(&'a NativeSelectedStringSpanSelector),
}
impl<'a> SelectorInput<'a> {
    fn matches(self, schema: SchemaId, key: SchemaKey) -> bool {
        match self {
            Self::Canonical(value) => value.schema() == schema && value.schema_key() == key,
            Self::UnitRange(range) => range.schema_key() == key,
        }
    }
    fn count(self) -> Result<u64, InvokeError> {
        match self {
            Self::Canonical(value) => {
                let ValueData::Matrix(matrix) = value.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                if !matches!(matrix.elements(), SequenceView::U64(_)) {
                    return Err(InvokeError::InvalidInput);
                }
                Ok(matrix.elements().len() as u64)
            }
            Self::UnitRange(range) => Ok(range.count()),
        }
    }
    fn retained_bytes(self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        match self {
            Self::Canonical(value) => value
                .memory_budget_retained_bytes(memory)
                .ok_or(InvokeError::UnadmittedValue),
            Self::UnitRange(_) => Ok(0),
        }
    }
    fn geometry(self) -> selected_string_matrix::Selector<'a> {
        match self {
            Self::Canonical(value) => selected_string_matrix::Selector::Canonical(value),
            Self::UnitRange(range) => selected_string_matrix::Selector::UnitRange {
                start: range.start(),
                count: range.count(),
            },
        }
    }
    fn into_owner(self) -> SelectorOwner {
        match self {
            Self::Canonical(value) => SelectorOwner::Canonical(value.clone()),
            Self::UnitRange(range) => SelectorOwner::UnitRange(*range),
        }
    }
}

impl NativePreparedLibraryCall {
    pub fn selected_text_line_matrix(
        &self,
        lines: &Rc<NativeTextLineMatrix>,
        selector: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeSelectedStringMatrix, InvokeError> {
        self.selected_text_line_matrix_with_selector(
            lines,
            SelectorInput::Canonical(selector),
            limits,
            work,
            coexistence,
        )
    }

    /// Bind the original selection using already validated Range geometry.
    /// The caller retains/pays fixed state metadata; no selector Value or
    /// count-sized allocation is manufactured here. Source ownership, output
    /// shape, positional checks and the full real row traversal are unchanged.
    pub fn selected_text_line_matrix_range(
        &self,
        lines: &Rc<NativeTextLineMatrix>,
        selector: &NativeSelectedStringSpanSelector,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeSelectedStringMatrix, InvokeError> {
        self.selected_text_line_matrix_with_selector(
            lines,
            SelectorInput::UnitRange(selector),
            limits,
            work,
            coexistence,
        )
    }

    fn selected_text_line_matrix_with_selector(
        &self,
        lines: &Rc<NativeTextLineMatrix>,
        selector: SelectorInput<'_>,
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
            || !selector.matches(contract.inputs[1].schema, self.input_keys[1])
        {
            return Err(InvokeError::InvalidInput);
        }
        let compact = matches!(selector, SelectorInput::UnitRange(_));
        let extra_shape = if compact {
            if self.selection_range_shape_bounds.0 == 0 {
                return Err(InvokeError::InvalidInput);
            }
            self.selection_range_shape_bounds
        } else {
            (0, 0)
        };
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
                let selector_bytes = selector.retained_bytes(&memory)?;
                let count = selector.count()?;
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    add(source_bytes, selector_bytes)?,
                )?;
                coexistence.retained_nodes = add(
                    coexistence.retained_nodes,
                    add(
                        add(lines.count() as u64, if compact { 0 } else { count })?,
                        16,
                    )?,
                )?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                let binding_bytes = add(
                    add(self.binding_bytes, self.selection_shape_bounds.0)?,
                    extra_shape.0,
                )?;
                context.check_capacity(binding_bytes, self.binding_nodes)?;
                context.charge(
                    add(
                        self.binding_work,
                        add(
                            add(self.selection_shape_bounds.1, extra_shape.1)?,
                            add(mul(count, 128)?, 256)?,
                        )?,
                    )?,
                    0,
                )?;
                let _binding = memory.reserve_capacity(binding_bytes)?;
                let range_shape = if compact {
                    Some(
                        mech_core::shape_for_resolved_extents(selector_schema, &[1, count])
                            .map_err(|_| InvokeError::Kernel(ResidentKernelError::InvalidShape))?,
                    )
                } else {
                    None
                };
                let selector_parameters = match selector {
                    SelectorInput::Canonical(value) => value.shape().parameter_values(),
                    SelectorInput::UnitRange(_) => range_shape.as_ref().unwrap().parameter_values(),
                };
                // This is binding metadata only: no canonical source Matrix is
                // created or passed to a kernel. The opaque line owner supplied
                // its actual logical shape through the original text/lines leaf.
                let output_parameters = vec![0; self.output_parameters];
                let call = super::super::bind_current(
                    &self.schemas,
                    self.template.clone(),
                    self.factory,
                    &[lines.shape().parameter_values(), selector_parameters],
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
                let mut chunk_cursor = 0;
                let geometry = selected_string_matrix::prepare_cells(
                    &call.kernel,
                    (rows, columns),
                    lines.count(),
                    selector.geometry(),
                    |i| lines.line_cached(i, &mut chunk_cursor),
                    |first, end| lines.captured_range_payload(first, end),
                )?;
                Ok(NativeSelectedStringMatrix {
                    source: Source::Lines(lines.clone()),
                    selector: selector.into_owner(),
                    geometry,
                    schema: contract.outputs[0].schema,
                    key: self.output_key,
                    revision: self.template.program_revision(),
                })
            },
        )?
    }
}
