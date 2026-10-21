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

#[path = "native_collection_line_reads.rs"]
mod reads;

/// Immutable owner and exact native pointer/length rows. Only the maintained
/// prepared factory can construct it; the original scalar String stays alive
/// until every row and shape tape drops, before its metadata charge releases.
pub struct NativeTextLineMatrix {
    source: Value,
    rows: Box<[[u64; 2]]>,
    shape: ShapeInstance,
    schema: SchemaId,
    key: SchemaKey,
    revision: ProgramRevision,
    ascii: bool,
    reservation: ManagedMemoryReservation,
}

/// Both representations of ONE original call share its enclosing allowance.
pub enum NativeTextLineMatrixChoice {
    View(NativeTextLineMatrix),
    Canonical(Value),
}
impl NativeTextLineMatrix {
    pub fn source(&self) -> &Value {
        &self.source
    }
    pub fn rows(&self) -> &[[u64; 2]] {
        &self.rows
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
        self.rows.len()
    }
    pub fn metadata_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    pub fn retained_bytes(&self, memory: &ManagedMemoryBudget) -> Result<u64, InvokeError> {
        if self.reservation.budget() != *memory {
            return Err(InvokeError::UnadmittedValue);
        }
        let source = self
            .source
            .memory_budget_retained_bytes(memory)
            .ok_or(InvokeError::UnadmittedValue)?;
        Ok(add(source, self.metadata_bytes())?)
    }
    /// Borrow through the real immutable String, not a raw-pointer read. Rows
    /// are private-constructor facts and every requested extent stays checked.
    pub fn line(&self, ordinal: usize) -> Option<&str> {
        let [pointer, length] = *self.rows.get(ordinal)?;
        let ValueData::String(text) = self.source.data() else {
            return None;
        };
        let start = usize::try_from(pointer.checked_sub(text.as_ptr() as u64)?).ok()?;
        let end = start.checked_add(usize::try_from(length).ok()?)?;
        text.get(start..end)
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
                    source: source.clone(),
                    rows: rows.into_boxed_slice(),
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
}
