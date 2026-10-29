//! Owner-bound line geometry from the actual prepared text/lines factory.
//! This is never a canonical Matrix, a program dispatcher or an I/O authority.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::resident::budget::{
    NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{
    ManagedMemoryBudget, ManagedMemoryReservation, MemoryBudgetDimension as Dimension,
    MemoryBudgetLimits, ProgramRevision, ResidentKernelError, ResidentResourcePhase,
    ResourceDemand, SchemaBody, SchemaId, SchemaKey, ShapeInstance, Value, ValueData,
};

#[cfg(test)]
#[path = "native_collection_captured_line_tests.rs"]
mod captured_line_tests;
#[path = "native_text_line_chunks.rs"]
mod chunks;
#[path = "native_collection_line_reads.rs"]
mod reads;
pub use chunks::{
    NativeTextLineCaptureFacts, NativeTextLineChunks, NativeTextLineJoinChunk, NativeTextLineWindow,
};

/// Immutable logical line matrix admitted by the maintained prepared factory.
/// Snapshot rows retain their scalar String; captured backing retains only
/// compiler-proven live chunks. Both carry the complete final logical shape.
pub struct NativeTextLineMatrix {
    backing: Backing,
    count: usize,
    shape: ShapeInstance,
    schema: SchemaId,
    key: SchemaKey,
    revision: ProgramRevision,
    ascii: bool,
    reservation: ManagedMemoryReservation,
}
enum Backing {
    Snapshot {
        source: Value,
        rows: Box<[[u64; 2]]>,
    },
    Captured(NativeTextLineChunks),
}

/// Both representations of ONE original call share its enclosing allowance.
pub enum NativeTextLineMatrixChoice {
    View(NativeTextLineMatrix),
    Canonical(Value),
}
impl NativeTextLineMatrix {
    pub fn source(&self) -> Option<&Value> {
        match &self.backing {
            Backing::Snapshot { source, .. } => Some(source),
            Backing::Captured(_) => None,
        }
    }
    pub fn rows(&self) -> Option<&[[u64; 2]]> {
        match &self.backing {
            Backing::Snapshot { rows, .. } => Some(rows),
            Backing::Captured(_) => None,
        }
    }
    pub fn captured_chunks(&self) -> Option<&NativeTextLineChunks> {
        match &self.backing {
            Backing::Captured(owner) => Some(owner),
            Backing::Snapshot { .. } => None,
        }
    }
    pub(super) fn append_capture_join_consuming(
        &mut self,
        first: usize,
        end: usize,
        separator: &str,
        target: &mut String,
    ) -> Result<bool, InvokeError> {
        let Backing::Captured(owner) = &mut self.backing else {
            return Ok(false);
        };
        owner.append_joined_consuming(
            (first as u64)
                .checked_add(1)
                .ok_or(InvokeError::InvalidInput)?,
            (end as u64)
                .checked_add(1)
                .ok_or(InvokeError::InvalidInput)?,
            separator,
            target,
        )?;
        Ok(true)
    }
    pub(super) fn append_capture_join_chunk(
        &mut self,
        first: u64,
        end: u64,
        target: &mut Vec<u8>,
    ) -> Result<Option<u64>, InvokeError> {
        let Backing::Captured(owner) = &mut self.backing else {
            return Err(InvokeError::InvalidInput);
        };
        owner.append_joined_chunk(first, end, target)
    }
    pub(super) fn take_capture_join_chunk(
        &mut self,
        first: u64,
        end: u64,
    ) -> Result<Option<NativeTextLineJoinChunk>, InvokeError> {
        let Backing::Captured(owner) = &mut self.backing else {
            return Err(InvokeError::InvalidInput);
        };
        owner.take_join_chunk(first, end)
    }
    pub fn shape(&self) -> &ShapeInstance {
        &self.shape
    }
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub const fn program_revision(&self) -> ProgramRevision {
        self.revision
    }
    pub const fn is_ascii(&self) -> bool {
        self.ascii
    }
    pub fn count(&self) -> usize {
        self.count
    }
    pub fn metadata_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    pub fn retained_bytes(&self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        if self.reservation.budget() != *memory {
            return Err(InvokeError::UnadmittedValue);
        }
        let source = match &self.backing {
            Backing::Snapshot { source, .. } => source
                .memory_budget_retained_bytes(memory)
                .ok_or(InvokeError::UnadmittedValue)?,
            Backing::Captured(owner) => owner.retained_bytes(memory)?,
        };
        Ok(add(source, self.metadata_bytes())?)
    }
    /// Borrow through the real immutable String, not a raw-pointer read. Rows
    /// are private-constructor facts and every requested extent stays checked.
    pub fn line(&self, ordinal: usize) -> Option<&str> {
        let Backing::Snapshot { source, rows } = &self.backing else {
            return self
                .captured_chunks()?
                .line((ordinal as u64).checked_add(1)?);
        };
        let [pointer, length] = *rows.get(ordinal)?;
        let ValueData::String(text) = source.data() else {
            return None;
        };
        let start = usize::try_from(pointer.checked_sub(text.as_ptr() as u64)?).ok()?;
        let end = start.checked_add(usize::try_from(length).ok()?)?;
        text.get(start..end)
    }

    /// Preserve canonical cell access while amortizing captured chunk lookup
    /// across one already-admitted selection or immutable append traversal.
    pub(super) fn line_cached(&self, ordinal: usize, cursor: &mut usize) -> Option<&str> {
        match &self.backing {
            Backing::Captured(owner) => owner.line_cached((ordinal as u64).checked_add(1)?, cursor),
            Backing::Snapshot { .. } => self.line(ordinal),
        }
    }

    /// Exact immutable geometry only; the selected visit has already funded
    /// its complete conservative scan and validated this owner's account.
    pub(super) fn captured_range_payload(&self, first: usize, end: usize) -> Option<u64> {
        let owner = self.captured_chunks()?;
        let first = u64::try_from(first).ok()?.checked_add(1)?;
        let end = u64::try_from(end).ok()?.checked_add(1)?;
        owner.final_range_payload(first, end)
    }
}
fn add(a: u64, b: u64) -> Result<u64, ResidentKernelError> {
    a.checked_add(b).ok_or(ResidentKernelError::InvalidShape)
}
fn mul(a: u64, b: u64) -> Result<u64, ResidentKernelError> {
    a.checked_mul(b).ok_or(ResidentKernelError::InvalidShape)
}
fn require(dimension: Dimension, required: u64, limit: Option<u64>) -> Result<(), InvokeError> {
    let limit = limit.ok_or(InvokeError::InvalidInput)?;
    if required > limit {
        return Err(ResidentKernelError::ResourceLimit {
            phase: ResidentResourcePhase::Admission,
            dimension,
            required,
            limit,
        }
        .into());
    }
    Ok(())
}

impl NativePreparedLibraryCall {
    /// Conservative eligibility cost for bounded capture under the scheduling
    /// conditions documented by the storage owner. This grants no work credit:
    /// every real operation still debits the same original producer account.
    pub fn text_line_capture_work_bound(&self, input_limit: u64) -> Option<u64> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        if self.template.primitive() != Primitive::TextLines {
            return None;
        }
        let matrix_metadata = (core::mem::size_of::<NativeTextLineMatrix>() as u64)
            .checked_add((self.output_parameters as u64).checked_mul(8)?)?
            .checked_add(512)?;
        chunks::capture_work_bound(
            input_limit,
            self.lookup_work,
            self.binding_work
                .checked_add(self.selection_shape_bounds.1)?,
            matrix_metadata,
        )
    }

    /// Prepare private UTF-8 storage for this SAME selected text/lines site.
    /// This neither executes that source visit nor publishes a partial Matrix,
    /// String observation, numeric count, or permission to complete a program.
    /// The owning backend must still reconcile the original producer/length
    /// admissions and capture errors at EOF before consuming the final facts.
    pub fn prepare_text_line_chunks(
        &self,
        input_limit: u64,
        max_fragment: usize,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
    ) -> Result<NativeTextLineChunks, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::TextLines
            || self.template.inputs().len() != 1
            || self.template.inputs()[0].body() != &SchemaBody::String
            || !self.template.inputs()[0].dimension_parameters().is_empty()
            || self.selection_shape_bounds.0 == 0
        {
            return Err(InvokeError::InvalidInput);
        }
        chunks::prepare(
            input_limit,
            max_fragment,
            limits,
            work,
            self.lookup_work,
            Some(self.cache_identity.clone()),
        )
    }

    /// Bind a synchronous loan to this exact selected producer and invocation.
    /// The loan contains real completed cells, not a canonical partial Matrix.
    pub fn bind_text_line_window(
        &self,
        window: &NativeTextLineWindow<'_>,
        work: &NativeLibraryWorkBudget,
    ) -> Result<u64, InvokeError> {
        if self.template.primitive() != crate::native_artifact::NativeCollectionPrimitive::TextLines
        {
            return Err(InvokeError::InvalidInput);
        }
        window.bind(&self.cache_identity, work)
    }
    pub fn text_line_matrix_or_canonical(
        &self,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<NativeTextLineMatrixChoice, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::TextLines {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                work.charge(self.lookup_work, 0)?;
                let inputs = [Some(source.clone())];
                self.validate_values(&inputs, &None, &work.memory_budget())?;
                let ValueData::String(text) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                work.charge(mul(text.len() as u64, 4)?, 0)?;
                if text.is_ascii() {
                    return self
                        .text_line_matrix(source, limits, work, coexistence)
                        .map(NativeTextLineMatrixChoice::View);
                }
                let mut output = None;
                // Nested limits inherit already spent work; fallback does not
                // renew the original call's cap or replace its invocation account.
                self.invoke_current(&inputs, &mut output, limits, work)?;
                Ok(NativeTextLineMatrixChoice::Canonical(
                    output.ok_or(InvokeError::InvalidInput)?,
                ))
            },
        )?
    }

    /// At the ORIGINAL text/lines visit, validate the actual admitted String
    /// and exact output shape. The backend separately proves all consumers,
    /// retains this owner, and supplies ordinary semantics on unsupported paths.
    pub fn text_line_matrix(
        &self,
        source: &Value,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<NativeTextLineMatrix, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::TextLines
            || self.input_keys.len() != 1
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
                let inputs = [Some(source.clone())];
                self.validate_values(&inputs, &None, &memory)?;
                let ValueData::String(text) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                if !source.shape().parameter_values().is_empty()
                    || self.template.inputs()[0].body() != &SchemaBody::String
                {
                    return Err(InvokeError::InvalidInput);
                }
                let source_bytes = source
                    .memory_budget_retained_bytes(&memory)
                    .ok_or(InvokeError::UnadmittedValue)?;
                coexistence.turn_peak_bytes = add(coexistence.turn_peak_bytes, source_bytes)?;
                coexistence.retained_nodes = add(coexistence.retained_nodes, 1)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence);
                let binding_bytes = add(self.binding_bytes, self.selection_shape_bounds.0)?;
                context.check_capacity(binding_bytes, self.binding_nodes)?;
                context.charge(
                    add(
                        add(self.binding_work, self.selection_shape_bounds.1)?,
                        mul(text.len() as u64, 4)?,
                    )?,
                    0,
                )?;
                let _binding = memory.reserve_capacity(binding_bytes)?;
                let call = self.bind_values(&inputs, &None, limits)?;
                let (count, shape) = crate::resident::text_ops::line_shape(&call.kernel, text)?;
                let rows_bytes = mul(count as u64, core::mem::size_of::<[u64; 2]>() as u64)?;
                let metadata = add(
                    add(
                        rows_bytes,
                        core::mem::size_of::<NativeTextLineMatrix>() as u64,
                    )?,
                    add(mul(shape.parameter_values().len() as u64, 8)?, 512)?,
                )?;
                let retained = add(source_bytes, metadata)?;
                require(
                    Dimension::OutputElements,
                    count as u64,
                    limits.max_output_elements,
                )?;
                require(Dimension::OutputBytes, retained, limits.max_output_bytes)?;
                require(Dimension::ClonedBytes, 0, limits.max_cloned_bytes)?;
                let held = context.with_added_capacity(binding_bytes, self.binding_nodes)?;
                held.check_capacity(metadata, add(count as u64, 1)?)?;
                // Fund all construction, ASCII inspection and eventual retirement
                // before allocating or scanning. There is no per-line debit loop.
                held.charge(
                    add(
                        mul(text.len() as u64, 4)?,
                        add(mul(count as u64, 128)?, mul(metadata, 8)?)?,
                    )?,
                    0,
                )?;
                let reservation = memory.reserve_capacity(metadata)?;
                let mut rows = Vec::new();
                rows.try_reserve_exact(count)
                    .map_err(|_| InvokeError::InvalidInput)?;
                if rows.capacity() != count {
                    return Err(InvokeError::InvalidInput);
                }
                let ascii = text.is_ascii();
                for line in text.lines() {
                    rows.push([line.as_ptr() as u64, line.len() as u64]);
                }
                if rows.len() != count {
                    return Err(InvokeError::InvalidInput);
                }
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                Ok(NativeTextLineMatrix {
                    backing: Backing::Snapshot {
                        source: source.clone(),
                        rows: rows.into_boxed_slice(),
                    },
                    count,
                    shape,
                    schema: contract.outputs[0].schema,
                    key: self.output_key,
                    revision: self.template.program_revision(),
                    ascii,
                    reservation,
                })
            },
        )?
    }

    /// Admit the original selected producer using its genuine successful EOF
    /// facts. Framing and this final admission share one cumulative allowance;
    /// no full String, pointer table, or canonical partial value is constructed.
    pub fn text_line_matrix_from_capture(
        &self,
        owner: NativeTextLineChunks,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        mut coexistence: ResourceDemand,
    ) -> Result<NativeTextLineMatrix, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if self.template.primitive() != Primitive::TextLines
            || self.template.inputs().len() != 1
            || self.template.inputs()[0].body() != &SchemaBody::String
            || !self.template.inputs()[0].dimension_parameters().is_empty()
            || self.input_keys.len() != 1
            || self.selection_shape_bounds.0 == 0
        {
            return Err(InvokeError::InvalidInput);
        }
        let facts = owner.bind_final(&self.cache_identity, limits, work)?;
        let count = usize::try_from(facts.lines()).map_err(|_| InvokeError::InvalidInput)?;
        let memory = work.memory_budget();
        let source_bytes = owner.retained_bytes(&memory)?;
        coexistence.turn_peak_bytes = add(coexistence.turn_peak_bytes, source_bytes)?;
        coexistence.retained_nodes = add(coexistence.retained_nodes, 1)?;
        let context = NativeProjectionIndexContext::new(work, limits, coexistence);
        let binding_bytes = add(self.binding_bytes, self.selection_shape_bounds.0)?;
        context.check_capacity(binding_bytes, self.binding_nodes)?;
        owner.debit(add(self.binding_work, self.selection_shape_bounds.1)?)?;
        let _binding = memory.reserve_capacity(binding_bytes)?;
        let shape =
            crate::resident::text_ops::line_shape_from_count(self.template.output(), count)?;
        require(
            Dimension::OutputElements,
            facts.lines(),
            limits.max_output_elements,
        )?;
        let call = super::super::bind_current(
            &self.schemas,
            self.template.clone(),
            self.factory,
            self.descriptor_templates.as_deref(),
            &[&[]],
            shape.parameter_values(),
            None,
            limits,
        )
        .map_err(InvokeError::Binding)?;
        // The actual selected text/lines factory must retain precisely the
        // resolved final shape we supplied; this is not a generic retagging.
        if call.kernel.snapshot_output().map(|output| &output.shape) != Some(&shape) {
            return Err(InvokeError::InvalidInput);
        }
        let metadata = add(
            core::mem::size_of::<NativeTextLineMatrix>() as u64,
            add(mul(shape.parameter_values().len() as u64, 8)?, 512)?,
        )?;
        require(
            Dimension::OutputBytes,
            add(source_bytes, metadata)?,
            limits.max_output_bytes,
        )?;
        require(Dimension::ClonedBytes, 0, limits.max_cloned_bytes)?;
        context
            .with_added_capacity(binding_bytes, self.binding_nodes)?
            .check_capacity(metadata, add(facts.lines(), 1)?)?;
        owner.debit(add(mul(facts.lines(), 128)?, mul(metadata, 8)?)?)?;
        let reservation = memory.reserve_capacity(metadata)?;
        let mech_core::ResolvedOperationContract::Declared(contract) = self.template.contract()
        else {
            return Err(InvokeError::InvalidInput);
        };
        Ok(NativeTextLineMatrix {
            backing: Backing::Captured(owner),
            count,
            shape,
            schema: contract.outputs[0].schema,
            key: self.output_key,
            revision: self.template.program_revision(),
            ascii: facts.is_ascii(),
            reservation,
        })
    }
}
