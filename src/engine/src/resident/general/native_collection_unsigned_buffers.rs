//! Selected packed-buffer leaves with optional consuming ownership.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::intrinsics::sequence_buffers::{self as primitive, Operation};
use crate::native_artifact::NativeCollectionPrimitive as P;
use crate::resident::budget::{
    NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::{
    MemoryBudgetLimits, ResourceDemand, Value, ValueData,
    snapshot::{UnsignedMatrixBuffer, UnsignedMatrixOwnershipTransfer},
};

impl NativePreparedLibraryCall {
    pub fn supports_unsigned_buffer(&self) -> bool {
        self.buffer_operation().is_some()
    }
    fn buffer_operation(&self) -> Option<Operation> {
        Some(match self.template.primitive() {
            P::SequenceWithCapacity => Operation::WithCapacity,
            P::SequenceReserve => Operation::Reserve,
            P::SequenceAppend => Operation::Append,
            P::SequenceClear => Operation::Clear,
            _ => return None,
        })
    }
    /// Executes this selected leaf only. The generated caller separately proves
    /// its current visit and, when requested, that retiring source slots is safe.
    /// Actual aliases/imports always retain the immutable copy route.
    pub fn unsigned_buffer(
        &self,
        source: Option<Value>,
        argument: u64,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Value, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let operation = self.buffer_operation().ok_or(InvokeError::InvalidInput)?;
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
                let output_schema = contract.outputs[0].schema;
                let element = primitive::element(self.template.output().body())
                    .ok_or(InvokeError::InvalidInput)?;
                let width = primitive::width(element).ok_or(InvokeError::InvalidInput)?;
                if operation == Operation::Append && width < 8 && argument >> (width * 8) != 0 {
                    return Err(InvokeError::InvalidInput);
                }
                let (source_bytes, source_nodes) = match source.as_ref() {
                    Some(source) if operation != Operation::WithCapacity => {
                        let mech_core::ResolvedOperationContract::Declared(contract) =
                            self.template.contract()
                        else {
                            return Err(InvokeError::InvalidInput);
                        };
                        if source.schema() != contract.inputs[0].schema
                            || source.schema_key() != self.input_keys[0]
                        {
                            return Err(InvokeError::InvalidInput);
                        }
                        source
                            .validate_against(&self.schemas)
                            .map_err(|_| InvokeError::InvalidInput)?;
                        (
                            source
                                .memory_budget_retained_bytes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                            source
                                .memory_budget_retained_nodes(&memory)
                                .ok_or(InvokeError::UnadmittedValue)?,
                        )
                    }
                    None if operation == Operation::WithCapacity => (0, 0),
                    _ => return Err(InvokeError::InvalidInput),
                };
                let geometry = primitive::geometry(operation, source.as_ref(), width, argument)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(source_bytes, source_nodes)?;
                context.check_capacity(0, 0)?;
                // Shape resolution, construction and eventual schema/owner retirement
                // use this exact selected template's existing descriptor envelope.
                let schema_bytes = self
                    .schemas
                    .clone_allocation_bound_bytes()
                    .ok_or(InvokeError::InvalidInput)?;
                let metadata = self
                    .binding_bytes
                    .checked_add(schema_bytes)
                    .and_then(|n| n.checked_add(4096))
                    .ok_or(InvokeError::InvalidInput)?;
                let metadata_work = metadata
                    .checked_mul(16)
                    .and_then(|n| n.checked_add(self.lookup_work))
                    .ok_or(InvokeError::InvalidInput)?;
                context.charge(metadata_work, 0)?;
                let footprint = geometry
                    .footprint(self.output_parameters)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let mut source = source;
                let mut owned = None;
                if may_consume && let Some(value) = source.take() {
                    match value.try_into_owned_unsigned_matrix(&memory)? {
                        UnsignedMatrixOwnershipTransfer::Owned(buffer) => owned = Some(buffer),
                        UnsignedMatrixOwnershipTransfer::Unavailable(value) => source = Some(value),
                    }
                }
                let grows = owned
                    .as_ref()
                    .is_none_or(|buffer| buffer.capacity_bytes() < geometry.capacity * width);
                let payload_allocation = if grows {
                    geometry.bytes().map_err(|_| InvokeError::InvalidInput)?
                } else {
                    0
                };
                let copy_bytes = if grows && operation != Operation::Clear {
                    let length = owned
                        .as_ref()
                        .map(|buffer| buffer.elements().len())
                        .or_else(|| {
                            source.as_ref().and_then(|value| match value.data() {
                                ValueData::Matrix(matrix) => Some(matrix.elements().len()),
                                _ => None,
                            })
                        })
                        .unwrap_or(0);
                    (length as u64)
                        .checked_mul(width as u64)
                        .ok_or(InvokeError::InvalidInput)?
                } else {
                    0
                };
                let allocation = metadata
                    .checked_add(payload_allocation)
                    .ok_or(InvokeError::InvalidInput)?;
                context.check_capacity(
                    allocation,
                    footprint
                        .node_count
                        .checked_add(8)
                        .ok_or(InvokeError::InvalidInput)?,
                )?;
                let demand = ResourceDemand {
                    output_elements: geometry.length as u64,
                    ..ResourceDemand::default()
                };
                if let Some(violation) = mech_core::evaluate_call_memory_budget(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    demand,
                    footprint.retained_bytes,
                    copy_bytes,
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
                context.charge(
                    copy_bytes
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(1024))
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                // Admit the worst-case old/new backing overlap before reallocating.
                // No-growth ownership transfers need only a fresh metadata envelope.
                let mut admission = memory.reserve_capacity(allocation)?;
                let shape = mech_core::shape_for_resolved_extents(
                    self.template.output(),
                    &[1, geometry.length as u64],
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                let buffer = if let Some(owned) = owned {
                    let (mut buffer, charge) = owned.into_parts();
                    admission.merge_capacity(charge)?;
                    mutate(&mut buffer, operation, geometry, argument)?;
                    buffer
                } else {
                    primitive::copy_buffer(operation, source.as_ref(), geometry, argument)
                        .map_err(|_| InvokeError::InvalidInput)?
                };
                let value = Value::from_unsigned_matrix_buffer(
                    output_schema,
                    shape,
                    buffer,
                    self.schemas.clone(),
                )
                .map_err(|_| InvokeError::InvalidInput)?;
                // Drop detached inputs only after the fresh candidate has frozen.
                let value = value.into_memory_budget(&mut admission)?;
                drop(source);
                Ok(value)
            },
        )?
    }
}
fn mutate(
    buffer: &mut UnsignedMatrixBuffer,
    operation: Operation,
    geometry: primitive::Geometry,
    argument: u64,
) -> Result<(), InvokeError> {
    macro_rules! words {
        ($words:expr, $ty:ty) => {{
            if $words.capacity() < geometry.capacity {
                $words
                    .try_reserve_exact(
                        geometry
                            .capacity
                            .checked_sub($words.len())
                            .ok_or(InvokeError::InvalidInput)?,
                    )
                    .map_err(|_| InvokeError::InvalidInput)?;
                if $words.capacity() != geometry.capacity {
                    return Err(InvokeError::InvalidInput);
                }
            }
            match operation {
                Operation::Clear => $words.clear(),
                Operation::Append => {
                    $words.push(<$ty>::try_from(argument).map_err(|_| InvokeError::InvalidInput)?)
                }
                Operation::Reserve => {}
                Operation::WithCapacity => return Err(InvokeError::InvalidInput),
            }
            if $words.len() != geometry.length {
                return Err(InvokeError::InvalidInput);
            }
        }};
    }
    match buffer {
        UnsignedMatrixBuffer::U8(v) => words!(v, u8),
        UnsignedMatrixBuffer::U16(v) => words!(v, u16),
        UnsignedMatrixBuffer::U32(v) => words!(v, u32),
        UnsignedMatrixBuffer::U64(v) => words!(v, u64),
    }
    Ok(())
}
