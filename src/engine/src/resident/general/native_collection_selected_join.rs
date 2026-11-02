//! Genuine owner-bound selection, not a canonical Matrix or a visit permit.
use super::{
    InvokeError, NativePreparedLibraryCall, NativeSelectedStringSpanSelector, NativeTextLineMatrix,
    finite_limits,
};
use crate::resident::{
    budget::{NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext},
    numeric::selected_string_matrix::{self, Geometry},
};
use mech_core::{
    IntegerWidth, ManagedMemoryBudget, MemoryBudgetLimits, ProgramRevision, ResidentKernelError,
    ResourceDemand, SchemaBody, SchemaId, SchemaKey, Value, ValueData, snapshot::SequenceView,
};
use std::rc::Rc;

#[path = "native_collection_selected_lines.rs"]
mod lines;

enum Source {
    Canonical(Value),
    Lines(Rc<NativeTextLineMatrix>),
}

enum SelectorOwner {
    Canonical(Value),
    UnitRange(NativeSelectedStringSpanSelector),
}
impl SelectorOwner {
    fn geometry(&self) -> selected_string_matrix::Selector<'_> {
        match self {
            Self::Canonical(value) => selected_string_matrix::Selector::Canonical(value),
            Self::UnitRange(range) => selected_string_matrix::Selector::UnitRange {
                start: range.start(),
                count: range.count(),
            },
        }
    }
    fn retained_bytes(&self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        match self {
            Self::Canonical(value) => value
                .memory_budget_retained_bytes(memory)
                .ok_or(InvokeError::UnadmittedValue),
            // Fixed inline metadata belongs to the caller's admitted state;
            // there is no selector allocation or canonical owner to charge.
            Self::UnitRange(_) => Ok(0),
        }
    }
    fn payload_nodes(&self, count: usize) -> u64 {
        match self {
            Self::Canonical(_) => count as u64,
            Self::UnitRange(_) => 0,
        }
    }
}

/// Constructed only by the actual prepared access factory. The geometry cannot
/// be paired with different Values: these strong, immutable owners travel with
/// it until the selected join finishes or the invocation fails.
pub struct NativeSelectedStringMatrix {
    source: Source,
    selector: SelectorOwner,
    geometry: Geometry,
    schema: SchemaId,
    key: SchemaKey,
    revision: ProgramRevision,
}
fn invalid_shape() -> ResidentKernelError {
    ResidentKernelError::InvalidShape
}
fn add(a: u64, b: u64) -> Result<u64, ResidentKernelError> {
    a.checked_add(b).ok_or_else(invalid_shape)
}
fn mul(a: u64, b: u64) -> Result<u64, ResidentKernelError> {
    a.checked_mul(b).ok_or_else(invalid_shape)
}
impl NativeSelectedStringMatrix {
    /// Representation/lifetime eligibility only; this grants no operation visit.
    pub fn can_consume_ascii_join_chunks(&self) -> bool {
        self.geometry.contiguous_range().is_some()
            && matches!(&self.source, Source::Lines(lines) if lines.is_ascii()
                && lines.captured_chunks().is_some() && Rc::strong_count(lines) == 1
                && Rc::weak_count(lines) == 0)
    }
    pub fn consuming_join_first(&self) -> Option<u64> {
        self.geometry
            .contiguous_range()
            .and_then(|(first, _)| (first as u64).checked_add(1))
    }
    pub fn consuming_join_chunk_bound(&self) -> Option<u64> {
        match &self.source {
            Source::Lines(lines) => Some(lines.captured_chunks()?.retained_chunk_count()),
            _ => None,
        }
    }
    /// Append one fully initialized physical payload, allowing a generated
    /// consumer to transform that output chunk while it is still cache-local.
    /// The caller has prepaid the complete original selection/join traversal.
    pub fn append_consuming_ascii_join_chunk(
        &mut self,
        next: u64,
        target: &mut Vec<u8>,
    ) -> Result<Option<u64>, InvokeError> {
        if !self.can_consume_ascii_join_chunks() {
            return Err(InvokeError::InvalidInput);
        }
        let (first, end) = self
            .geometry
            .contiguous_range()
            .ok_or(InvokeError::InvalidInput)?;
        let lower = (first as u64)
            .checked_add(1)
            .ok_or(InvokeError::InvalidInput)?;
        let upper = (end as u64)
            .checked_add(1)
            .ok_or(InvokeError::InvalidInput)?;
        if next < lower || next > upper {
            return Err(InvokeError::InvalidInput);
        }
        let Source::Lines(lines) = &mut self.source else {
            return Err(InvokeError::InvalidInput);
        };
        Rc::get_mut(lines)
            .ok_or(InvokeError::InvalidInput)?
            .append_capture_join_chunk(next, upper, target)
    }
    /// Transfer one immutable physical chunk to a generated transform/copy.
    /// The original selected join has prepaid all traversal and retirement;
    /// this owner grants no source visit and contains no copied row directory.
    pub fn take_consuming_ascii_join_chunk(
        &mut self,
        next: u64,
    ) -> Result<Option<super::NativeTextLineJoinChunk>, InvokeError> {
        if !self.can_consume_ascii_join_chunks() {
            return Err(InvokeError::InvalidInput);
        }
        let (first, end) = self
            .geometry
            .contiguous_range()
            .ok_or(InvokeError::InvalidInput)?;
        let lower = (first as u64)
            .checked_add(1)
            .ok_or(InvokeError::InvalidInput)?;
        let upper = (end as u64)
            .checked_add(1)
            .ok_or(InvokeError::InvalidInput)?;
        if next < lower || next > upper {
            return Err(InvokeError::InvalidInput);
        }
        let Source::Lines(lines) = &mut self.source else {
            return Err(InvokeError::InvalidInput);
        };
        Rc::get_mut(lines)
            .ok_or(InvokeError::InvalidInput)?
            .take_capture_join_chunk(next, upper)
    }
    pub const fn count(&self) -> usize {
        self.geometry.count
    }
    pub const fn payload_bytes(&self) -> u64 {
        self.geometry.payload
    }
    pub fn retained_bytes(&self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        let a = match &self.source {
            Source::Canonical(value) => value
                .memory_budget_retained_bytes(memory)
                .ok_or(InvokeError::UnadmittedValue)?,
            Source::Lines(view) => view.retained_bytes(memory)?,
        };
        let b = self.selector.retained_bytes(memory)?;
        Ok(add(a, b)?)
    }
    pub fn retained_nodes_bound(&self) -> Result<u64, InvokeError> {
        let count = match &self.source {
            Source::Canonical(value) => {
                let ValueData::Matrix(source) = value.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                source.elements().len()
            }
            Source::Lines(view) => view.count(),
        };
        // Actual source cells or private line rows, numeric selector cells and
        // root/shape headers. Never a claim about nonexistent output cells.
        Ok(add(
            add(count as u64, self.selector.payload_nodes(self.count()))?,
            16,
        )?)
    }
}

impl NativePreparedLibraryCall {
    /// Keep the original selected factory, genuine owners, target capacity and
    /// memory-account checks before choosing the chunk-composed representation.
    pub fn selected_matrix_join_chunks_ready(
        &self,
        view: &NativeSelectedStringMatrix,
        separator: &Value,
        target: &String,
        memory: &ManagedMemoryBudget,
    ) -> Result<bool, InvokeError> {
        let (separator, _) = self.selected_matrix_join_target(view, separator, target, memory)?;
        Ok(separator.is_empty() && view.can_consume_ascii_join_chunks())
    }
    /// Representation preference only; actual projection still validates the
    /// selected schemas, current owners, finite policy and original geometry.
    pub fn supports_selected_string_matrix(&self, inputs: &[Option<Value>]) -> bool {
        matches!(inputs, [Some(source), Some(selector)]
            if matches!(source.data(), ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::String(_)))
            && matches!(selector.data(), ValueData::Matrix(matrix)
                if matches!(matrix.elements(), SequenceView::U64(_))))
    }

    /// Project at the ORIGINAL selection visit. Both inputs must already have
    /// genuine same-account owners; no source body is evaluated here.
    pub fn selected_string_matrix(
        &self,
        inputs: &[Option<Value>],
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<NativeSelectedStringMatrix, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let [Some(source), Some(selector)] = inputs else {
            return Err(InvokeError::InvalidInput);
        };
        let [source_schema, selector_schema] = self.template.inputs() else {
            return Err(InvokeError::InvalidInput);
        };
        if self.template.primitive() != Primitive::MatrixRangeAccess
            || !matches!(source_schema.body(), SchemaBody::Matrix { element, .. }
                if element.as_ref() == &SchemaBody::String)
            || !matches!(selector_schema.body(), SchemaBody::Matrix { element, .. }
                if element.as_ref() == &SchemaBody::UnsignedInteger(IntegerWidth::W64))
            || self.selection_shape_bounds.0 == 0
        {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.lookup_work, 0)?;
                let memory = work.memory_budget();
                self.validate_values(inputs, &None, &memory)?;
                let (ValueData::Matrix(strings), ValueData::Matrix(indices)) =
                    (source.data(), selector.data())
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if !matches!(strings.elements(), SequenceView::String(_))
                    || !matches!(indices.elements(), SequenceView::U64(_))
                {
                    return Err(InvokeError::InvalidInput);
                }
                let count = indices.elements().len() as u64;
                coexistence.turn_peak_bytes = add(
                    coexistence.turn_peak_bytes,
                    add(
                        source
                            .memory_budget_retained_bytes(&memory)
                            .ok_or(InvokeError::UnadmittedValue)?,
                        selector
                            .memory_budget_retained_bytes(&memory)
                            .ok_or(InvokeError::UnadmittedValue)?,
                    )?,
                )?;
                coexistence.retained_nodes = add(
                    coexistence.retained_nodes,
                    add(add(strings.elements().len() as u64, count)?, 16)?,
                )?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                let binding_bytes = add(self.binding_bytes, self.selection_shape_bounds.0)?;
                context.check_capacity(binding_bytes, self.binding_nodes)?;
                // One paid metadata/selector walk, no per-cell budget callback.
                // Payload bytes are not touched or copied during this scan.
                let compute = add(
                    self.binding_work,
                    add(self.selection_shape_bounds.1, add(mul(count, 128)?, 256)?)?,
                )?;
                context.charge(compute, 0)?;
                let _binding = memory.reserve_capacity(binding_bytes)?;
                let call = self.bind_values(inputs, &None, limits)?;
                let geometry = selected_string_matrix::prepare(&call.kernel, source, selector)?;
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                Ok(NativeSelectedStringMatrix {
                    source: Source::Canonical(source.clone()),
                    selector: SelectorOwner::Canonical(selector.clone()),
                    geometry,
                    schema: contract.outputs[0].schema,
                    key: self.output_key,
                    revision: self.template.program_revision(),
                })
            },
        )?
    }

    fn selected_matrix_join_target<'a>(
        &self,
        view: &NativeSelectedStringMatrix,
        separator: &'a Value,
        target: &String,
        memory: &ManagedMemoryBudget,
    ) -> Result<(&'a str, u64), InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        if self.template.primitive() != Primitive::TextJoin
            || self.input_keys.len() != 2
            || view.schema != contract.inputs[0].schema
            || view.key != self.input_keys[0]
            || view.revision != self.template.program_revision()
            || separator.schema() != contract.inputs[1].schema
            || separator.schema_key() != self.input_keys[1]
            || !separator.shape().parameter_values().is_empty()
            || !target.is_empty()
        {
            return Err(InvokeError::InvalidInput);
        }
        view.retained_bytes(memory)?;
        separator
            .memory_budget_retained_bytes(memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        let ValueData::String(separator) = separator.data() else {
            return Err(InvokeError::InvalidInput);
        };
        let payload = add(
            view.payload_bytes(),
            mul(
                separator.len() as u64,
                view.count().saturating_sub(1) as u64,
            )?,
        )?;
        if target.capacity() as u64 != payload {
            return Err(InvokeError::InvalidInput);
        }
        Ok((separator, payload))
    }

    /// SAME TextJoin append mechanics, after the original consumer's complete
    /// copy/selector work and exact output capacity have been admitted. This
    /// method cannot grow the target, run a program, or authorize a visit.
    pub fn selected_matrix_join_append(
        &self,
        view: &NativeSelectedStringMatrix,
        separator: &Value,
        target: &mut String,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), InvokeError> {
        let (separator, payload) =
            self.selected_matrix_join_target(view, separator, target, memory)?;
        let append = |index, text: &str| {
            crate::resident::text_ops::append_joined(target, separator, index != 0, text);
            Ok(())
        };
        match &view.source {
            Source::Canonical(source) => {
                view.geometry
                    .for_each(source, view.selector.geometry(), append)?
            }
            Source::Lines(lines) => {
                let mut chunk_cursor = 0;
                view.geometry.for_each_cells(
                    lines.count(),
                    view.selector.geometry(),
                    |i| lines.line_cached(i, &mut chunk_cursor),
                    append,
                )?;
            }
        }
        if target.len() as u64 != payload {
            return Err(InvokeError::InvalidInput);
        }
        Ok(())
    }

    /// Transfer the final consumer's owner after all row loans have ended.
    /// A unique captured source and the already validated ascending contiguous
    /// selector permit retirement during copying. Shared or reordered sources
    /// retain the ordinary immutable append behavior and result.
    pub fn selected_matrix_join_append_consuming(
        &self,
        mut view: NativeSelectedStringMatrix,
        separator: &Value,
        target: &mut String,
        memory: &ManagedMemoryBudget,
    ) -> Result<(), InvokeError> {
        let (text_separator, payload) =
            self.selected_matrix_join_target(&view, separator, target, memory)?;
        if let Some((first, end)) = view.geometry.contiguous_range()
            && let Source::Lines(lines) = &mut view.source
            && let Some(lines) = Rc::get_mut(lines)
            && lines.append_capture_join_consuming(first, end, text_separator, target)?
        {
            if target.len() as u64 != payload {
                return Err(InvokeError::InvalidInput);
            }
            return Ok(());
        }
        self.selected_matrix_join_append(&view, separator, target, memory)
    }
}
