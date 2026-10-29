//! One selected plain unsigned replacement; immutable aliases keep their backing.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::NativeCollectionPrimitive as Primitive;
use crate::resident::budget::{
    NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::snapshot::{SequenceView, UnsignedMatrixBuffer, UnsignedMatrixOwnershipTransfer};
use mech_core::{IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData};

impl NativePreparedLibraryCall {
    fn unsigned_matrix_replace_width(&self) -> Option<usize> {
        let [matrix, incoming, selector] = self.template.inputs() else {
            return None;
        };
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = matrix.body()
        else {
            return None;
        };
        if self.template.primitive() != Primitive::MatrixIndexedReplace
            || matrix != self.template.output()
            || dimensions.len() != 2
            || element.as_ref() != incoming.body()
            || !incoming.dimension_parameters().is_empty()
            || selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
            || !selector.dimension_parameters().is_empty()
        {
            return None;
        }
        Some(match element.as_ref() {
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => 1,
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => 2,
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => 4,
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => 8,
            _ => return None,
        })
    }

    pub fn supports_unsigned_matrix_replace(&self) -> bool {
        self.unsigned_matrix_replace_width().is_some()
    }

    /// Executes this original maintained leaf, with the caller's current source
    /// and scalar words. Last use and visit authority remain caller obligations.
    /// Actual frozen exclusivity is checked again before any physical mutation.
    pub fn unsigned_matrix_replace(
        &self,
        source: Value,
        index: u64,
        incoming: u64,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(Value, bool), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        let width = self
            .unsigned_matrix_replace_width()
            .ok_or(InvokeError::InvalidInput)?;
        if width < 8 && incoming >> (width * 8) != 0 {
            return Err(InvokeError::InvalidInput);
        }
        work.with_call_limits(
            limits.max_compute_work.unwrap(),
            limits.max_comparison_work.unwrap(),
            || {
                let mech_core::ResolvedOperationContract::Declared(contract) =
                    self.template.contract()
                else {
                    return Err(InvokeError::InvalidInput);
                };
                if source.schema() != contract.inputs[0].schema
                    || source.schema_key() != self.input_keys[0]
                    || contract.inputs[0].schema != contract.outputs[0].schema
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
                let context = NativeProjectionIndexContext::new(work, limits, coexistence)
                    .with_added_capacity(bytes, nodes)?;
                context.check_capacity(0, 0)?;
                context.charge(
                    self.lookup_work
                        .checked_add(128)
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let ValueData::Matrix(matrix) = source.data() else {
                    return Err(InvokeError::InvalidInput);
                };
                let length = matrix.elements().len();
                let capacity = matrix
                    .unsigned_buffer_capacity()
                    .ok_or(InvokeError::InvalidInput)?;
                let ordinal = crate::resident::numeric::available_scalar_index(index, length)?;
                let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                    &source,
                    self.template.inputs()[0].body(),
                )?;
                let ordinal =
                    crate::resident::numeric::canonical_matrix_index(ordinal, rows, columns)?;
                let old = word(matrix.elements(), width, ordinal)?;
                let footprint = source
                    .retained_footprint(&self.schemas)
                    .map_err(|_| InvokeError::InvalidInput)?;
                check_output(
                    self.template.origin().node(),
                    limits,
                    length,
                    footprint.retained_bytes,
                    0,
                )?;
                // Plain unsigned equality and unchanged shape/schema make this one
                // selected-word comparison exactly the full snapshot changed bit.
                context.charge(16, 1)?;
                if old == incoming {
                    return Ok((source, false));
                }
                let metadata = self
                    .binding_bytes
                    .checked_add(
                        self.schemas
                            .clone_allocation_bound_bytes()
                            .ok_or(InvokeError::InvalidInput)?,
                    )
                    .and_then(|n| n.checked_add(4096))
                    .ok_or(InvokeError::InvalidInput)?;
                context.check_capacity(
                    metadata,
                    footprint
                        .node_count
                        .checked_add(8)
                        .ok_or(InvokeError::InvalidInput)?,
                )?;
                context.charge(
                    metadata.checked_mul(16).ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                // Shape cloning and ownership handoff use this already admitted
                // envelope. The detached charge remains live on every failure exit.
                let mut admission = memory.reserve_capacity(metadata)?;
                let shape = source.shape().clone();
                let (source, owned) = if may_consume {
                    match source.try_into_owned_unsigned_matrix(&memory)? {
                        UnsignedMatrixOwnershipTransfer::Owned(buffer) => (None, Some(buffer)),
                        UnsignedMatrixOwnershipTransfer::Unavailable(source) => {
                            (Some(source), None)
                        }
                    }
                } else {
                    (Some(source), None)
                };
                let allocation = if owned.is_some() {
                    0
                } else {
                    (capacity as u64)
                        .checked_mul(width as u64)
                        .ok_or(InvokeError::InvalidInput)?
                };
                let copied = if owned.is_some() {
                    0
                } else {
                    (length as u64)
                        .checked_mul(width as u64)
                        .ok_or(InvokeError::InvalidInput)?
                };
                context.check_capacity(
                    metadata
                        .checked_add(allocation)
                        .ok_or(InvokeError::InvalidInput)?,
                    footprint
                        .node_count
                        .checked_add(8)
                        .ok_or(InvokeError::InvalidInput)?,
                )?;
                check_output(
                    self.template.origin().node(),
                    limits,
                    length,
                    footprint.retained_bytes,
                    copied,
                )?;
                context.charge(
                    copied
                        .checked_mul(8)
                        .and_then(|n| n.checked_add(1024))
                        .ok_or(InvokeError::InvalidInput)?,
                    0,
                )?;
                let mut buffer = if let Some(owned) = owned {
                    let (buffer, charge) = owned.into_parts();
                    admission.merge_capacity(charge)?;
                    buffer
                } else {
                    admission.merge_capacity(memory.reserve_capacity(allocation)?)?;
                    let ValueData::Matrix(matrix) =
                        source.as_ref().ok_or(InvokeError::InvalidInput)?.data()
                    else {
                        return Err(InvokeError::InvalidInput);
                    };
                    copy(matrix.elements(), capacity)?
                };
                // All index, kind, work and capacity admission precedes this store.
                replace(&mut buffer, ordinal, incoming)?;
                let value = Value::from_unsigned_matrix_buffer(
                    contract.outputs[0].schema,
                    shape,
                    buffer,
                    self.schemas.clone(),
                )
                .map_err(|_| InvokeError::InvalidInput)?
                .into_memory_budget(&mut admission)?;
                drop(source);
                Ok((value, true))
            },
        )?
    }
}

fn check_output(
    node: mech_core::NodeId,
    limits: MemoryBudgetLimits,
    length: usize,
    bytes: u64,
    copied: u64,
) -> Result<(), InvokeError> {
    if let Some(violation) = mech_core::first_call_memory_budget_violation(
        mech_core::MemoryObjectOwner::NodeOutput { node, port: 0 },
        ResourceDemand {
            output_elements: length as u64,
            cloned_bytes: copied,
            ..ResourceDemand::default()
        },
        bytes,
        0,
        limits,
    ) {
        return Err(InvokeError::Plan(
            mech_core::MemoryPlanError::TargetLimitExceeded { violation },
        ));
    }
    Ok(())
}

fn word(values: SequenceView<'_>, width: usize, at: usize) -> Result<u64, InvokeError> {
    Ok(match (values, width) {
        (SequenceView::U8(v), 1) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U16(v), 2) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U32(v), 4) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U64(v), 8) => *v.get(at).ok_or(InvokeError::InvalidInput)?,
        _ => return Err(InvokeError::InvalidInput),
    })
}

fn copy(values: SequenceView<'_>, capacity: usize) -> Result<UnsignedMatrixBuffer, InvokeError> {
    macro_rules! words {
        ($v:expr, $kind:ident) => {{
            let mut next = Vec::new();
            next.try_reserve_exact(capacity)
                .map_err(|_| InvokeError::InvalidInput)?;
            if next.capacity() != capacity {
                return Err(InvokeError::InvalidInput);
            }
            next.extend_from_slice($v);
            UnsignedMatrixBuffer::$kind(next)
        }};
    }
    Ok(match values {
        SequenceView::U8(v) => words!(v, U8),
        SequenceView::U16(v) => words!(v, U16),
        SequenceView::U32(v) => words!(v, U32),
        SequenceView::U64(v) => words!(v, U64),
        _ => return Err(InvokeError::InvalidInput),
    })
}

fn replace(buffer: &mut UnsignedMatrixBuffer, at: usize, incoming: u64) -> Result<(), InvokeError> {
    macro_rules! put {
        ($v:expr, $ty:ty) => {
            *$v.get_mut(at).ok_or(InvokeError::InvalidInput)? =
                <$ty>::try_from(incoming).map_err(|_| InvokeError::InvalidInput)?
        };
    }
    match buffer {
        UnsignedMatrixBuffer::U8(v) => put!(v, u8),
        UnsignedMatrixBuffer::U16(v) => put!(v, u16),
        UnsignedMatrixBuffer::U32(v) => put!(v, u32),
        UnsignedMatrixBuffer::U64(v) => put!(v, u64),
    }
    Ok(())
}
