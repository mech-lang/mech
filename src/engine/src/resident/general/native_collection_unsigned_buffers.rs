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
            P::SequenceAppendLeBytes => Operation::AppendLeBytes,
            P::SequenceAppendSublist => Operation::AppendSublist,
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
        self.unsigned_buffer_with_count(
            source,
            argument,
            None,
            may_consume,
            limits,
            work,
            coexistence,
        )
    }
    /// The optional count is required only for the bulk little-endian byte leaf.
    pub fn unsigned_buffer_with_count(
        &self,
        source: Option<Value>,
        argument: u64,
        count: Option<u64>,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Value, InvokeError> {
        self.unsigned_buffer_with_sublist(
            source,
            argument,
            count,
            None,
            may_consume,
            limits,
            work,
            coexistence,
        )
    }
    /// The second managed input is borrowed; it is never retired with the destination.
    pub fn unsigned_buffer_with_sublist(
        &self,
        source: Option<Value>,
        argument: u64,
        count: Option<u64>,
        sublist: Option<Value>,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<Value, InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let operation = self.buffer_operation().ok_or(InvokeError::InvalidInput)?;
        if (operation == Operation::AppendSublist) != sublist.is_some() {
            return Err(InvokeError::InvalidInput);
        }
        let extent_argument = match (operation, count) {
            (Operation::AppendSublist, Some(count)) => count,
            (Operation::AppendSublist, None) => return Err(InvokeError::InvalidInput),
            (Operation::AppendLeBytes, Some(count @ 0..=8)) => count,
            (Operation::AppendLeBytes, _) | (_, Some(_)) => return Err(InvokeError::InvalidInput),
            (_, None) => argument,
        };
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
                let element = primitive::output_element(operation, self.template.output().body())
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
                let mut sublist_bytes = 0;
                let mut sublist_nodes = 0;
                let range = if let Some(value) = sublist.as_ref() {
                    if value.schema() != contract.inputs[1].schema
                        || value.schema_key() != self.input_keys[1]
                    {
                        return Err(InvokeError::InvalidInput);
                    }
                    value
                        .validate_against(&self.schemas)
                        .map_err(|_| InvokeError::InvalidInput)?;
                    sublist_bytes = value
                        .memory_budget_retained_bytes(&memory)
                        .ok_or(InvokeError::UnadmittedValue)?;
                    sublist_nodes = value
                        .memory_budget_retained_nodes(&memory)
                        .ok_or(InvokeError::UnadmittedValue)?;
                    Some(
                        primitive::sublist_range(value, width, argument, extent_argument)
                            .map_err(|_| InvokeError::InvalidInput)?,
                    )
                } else {
                    None
                };
                let extent_argument = range.as_ref().map_or(extent_argument, |r| r.len() as u64);
                // Identity applies only when no schema/shape adaptation is required.
                // Statically sized inputs retain the admitted immutable-copy fallback.
                let no_op = matches!(
                    operation,
                    Operation::AppendLeBytes | Operation::AppendSublist
                ) && extent_argument == 0
                    && source.as_ref().is_some_and(|value| {
                        value.schema_key() == self.template.output().key()
                            && value.schema()
                                == match self.template.contract() {
                                    mech_core::ResolvedOperationContract::Declared(contract) => {
                                        contract.outputs[0].schema
                                    }
                                    _ => return false,
                                }
                    });
                let geometry =
                    primitive::geometry(operation, source.as_ref(), width, extent_argument)
                        .map_err(|_| InvokeError::InvalidInput)?;
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(
                        source_bytes
                            .checked_add(sublist_bytes)
                            .ok_or(InvokeError::InvalidInput)?,
                        source_nodes
                            .checked_add(sublist_nodes)
                            .ok_or(InvokeError::InvalidInput)?,
                    )?;
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
                if !no_op
                    && may_consume
                    && let Some(value) = source.take()
                {
                    match value.try_into_owned_unsigned_matrix(&memory)? {
                        UnsignedMatrixOwnershipTransfer::Owned(buffer) => owned = Some(buffer),
                        UnsignedMatrixOwnershipTransfer::Unavailable(value) => source = Some(value),
                    }
                }
                let grows = !no_op
                    && owned
                        .as_ref()
                        .is_none_or(|buffer| buffer.capacity_bytes() < geometry.capacity * width);
                let payload_allocation = if grows {
                    geometry.bytes().map_err(|_| InvokeError::InvalidInput)?
                } else {
                    0
                };
                let copy_bytes = if grows && let Some(buffer) = owned.as_ref() {
                    // Allocator reallocation may move the complete old layout,
                    // including uninitialized spare capacity after a clear.
                    u64::try_from(buffer.capacity_bytes()).map_err(|_| InvokeError::InvalidInput)?
                } else if grows && operation != Operation::Clear {
                    // A shared backing is rebuilt separately and copies only
                    // initialized words; its spare storage is never read.
                    let length = source
                        .as_ref()
                        .and_then(|value| match value.data() {
                            ValueData::Matrix(matrix) => Some(matrix.elements().len()),
                            _ => None,
                        })
                        .unwrap_or(0);
                    (length as u64)
                        .checked_mul(width as u64)
                        .ok_or(InvokeError::InvalidInput)?
                } else {
                    0
                };
                let copy_bytes = copy_bytes
                    .checked_add(range.as_ref().map_or(0, |r| r.len() as u64 * width as u64))
                    .ok_or(InvokeError::InvalidInput)?;
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
                    cloned_bytes: copy_bytes,
                    ..ResourceDemand::default()
                };
                if let Some(violation) = mech_core::first_call_memory_budget_violation(
                    mech_core::MemoryObjectOwner::NodeOutput {
                        node: self.template.origin().node(),
                        port: 0,
                    },
                    demand,
                    footprint.retained_bytes,
                    0,
                    limits,
                ) {
                    return Err(InvokeError::Plan(
                        mech_core::MemoryPlanError::TargetLimitExceeded { violation },
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
                let value = if no_op {
                    // Keep the exact admitted owner and schema arena. Rebinding
                    // an equivalent table would create another budget import.
                    source.as_ref().ok_or(InvokeError::InvalidInput)?.clone()
                } else {
                    let buffer = if let Some(owned) = owned {
                        let (mut buffer, charge) = owned.into_parts();
                        admission.merge_capacity(charge)?;
                        mutate(
                            &mut buffer,
                            operation,
                            geometry,
                            argument,
                            sublist.as_ref().zip(range.clone()),
                        )?;
                        buffer
                    } else {
                        primitive::copy_buffer_with_sublist(
                            operation,
                            source.as_ref(),
                            geometry,
                            argument,
                            sublist.as_ref().zip(range.clone()),
                        )
                        .map_err(|_| InvokeError::InvalidInput)?
                    };
                    let value = Value::from_unsigned_matrix_buffer(
                        output_schema,
                        shape,
                        buffer,
                        self.schemas.clone(),
                    )
                    .map_err(|_| InvokeError::InvalidInput)?;
                    value
                };
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
    sublist: Option<(&Value, core::ops::Range<usize>)>,
) -> Result<(), InvokeError> {
    if operation == Operation::AppendLeBytes {
        let UnsignedMatrixBuffer::U8(words) = buffer else {
            return Err(InvokeError::InvalidInput);
        };
        let count = geometry
            .length
            .checked_sub(words.len())
            .filter(|&count| count <= 8)
            .ok_or(InvokeError::InvalidInput)?;
        if count == 0 {
            return Ok(());
        }
        if words.capacity() - words.len() >= 8 {
            // The owned backing has room for one full little-endian word. Only
            // count bytes become visible; initialized spare bytes remain slack.
            let bytes = argument.to_le_bytes();
            // SAFETY: the exclusive Vec owns at least eight spare bytes, the
            // local source does not overlap it, and count is in 1..=8.
            unsafe {
                core::ptr::copy_nonoverlapping(
                    bytes.as_ptr(),
                    words.as_mut_ptr().add(words.len()),
                    8,
                );
                words.set_len(geometry.length);
            }
            return Ok(());
        }
    }
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
                Operation::AppendLeBytes => {
                    let count = geometry
                        .length
                        .checked_sub($words.len())
                        .filter(|&n| n <= 8)
                        .ok_or(InvokeError::InvalidInput)?;
                    if geometry.width != 1 {
                        return Err(InvokeError::InvalidInput);
                    }
                    $words.extend(
                        argument.to_le_bytes()[..count]
                            .iter()
                            .map(|&byte| <$ty>::from(byte)),
                    );
                }
                Operation::Reserve | Operation::AppendSublist => {}
                Operation::WithCapacity => return Err(InvokeError::InvalidInput),
            }
            if operation != Operation::AppendSublist && $words.len() != geometry.length {
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
    if operation == Operation::AppendSublist {
        let (source, range) = sublist.ok_or(InvokeError::InvalidInput)?;
        primitive::append_sublist(buffer, source, range).map_err(|_| InvokeError::InvalidInput)?;
        if buffer.elements().len() != geometry.length {
            return Err(InvokeError::InvalidInput);
        }
    }
    Ok(())
}
