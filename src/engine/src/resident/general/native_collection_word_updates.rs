//! One selected plain scalar replacement; immutable aliases keep their backing.
use super::{InvokeError, NativePreparedLibraryCall, finite_limits};
use crate::native_artifact::{NativeCollectionPrimitive as Primitive, NativeScalarKind as Kind};
use crate::resident::budget::{
    NativeLibraryWorkBudget, native_turn_plan_cache::NativeProjectionIndexContext,
};
use mech_core::snapshot::{
    NumericMatrixBuffer, NumericMatrixOwnershipTransfer, SequenceView, UnsignedMatrixBuffer,
};
use mech_core::{IntegerWidth, MemoryBudgetLimits, ResourceDemand, SchemaBody, Value, ValueData};

impl NativePreparedLibraryCall {
    fn unsigned_matrix_replace_width(&self) -> Option<usize> {
        if self.template.inputs().len() != 3 {
            return None;
        }
        Some(match self.scalar_matrix_replace_kind()? {
            Kind::U8 => 1,
            Kind::U16 => 2,
            Kind::U32 => 4,
            Kind::U64 => 8,
            _ => return None,
        })
    }

    /// Exact maintained scalar assignment, with one linear selector or two
    /// independent axis selectors. This does not admit arbitrary rectangles,
    /// interval kinds, cross-kind conversion or compound assignment.
    pub fn scalar_matrix_replace_kind(&self) -> Option<Kind> {
        let inputs = self.template.inputs();
        if !matches!(
            (self.template.primitive(), inputs.len()),
            (Primitive::MatrixIndexedReplace, 3) | (Primitive::MatrixCellReplace, 4)
        ) {
            return None;
        }
        let matrix = &inputs[0];
        let incoming = &inputs[1];
        let SchemaBody::Matrix {
            element,
            dimensions,
        } = matrix.body()
        else {
            return None;
        };
        if matrix != self.template.output()
            || dimensions.len() != 2
            || element.as_ref() != incoming.body()
            || !incoming.dimension_parameters().is_empty()
            || inputs[2..].iter().any(|selector| {
                selector.body() != &SchemaBody::UnsignedInteger(IntegerWidth::W64)
                    || !selector.dimension_parameters().is_empty()
            })
        {
            return None;
        }
        Kind::from_schema_body(element)
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
        self.matrix_replace_with::<_, true>(
            source,
            &[index],
            incoming,
            may_consume,
            limits,
            work,
            coexistence,
            |value, admission| match admission {
                Some(admission) => Ok(value.into_memory_budget(admission)?),
                None => Ok(value),
            },
        )
    }

    /// The same checked update protocol for every maintained native scalar
    /// kind and both supported selector geometries. A private owner is reused
    /// only after the ordinary same-account exclusive transfer succeeds.
    pub fn scalar_matrix_replace(
        &self,
        source: Value,
        indices: &[u64],
        incoming: u64,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<(Value, bool), InvokeError> {
        self.matrix_replace_with::<_, false>(
            source,
            indices,
            incoming,
            may_consume,
            limits,
            work,
            coexistence,
            |value, admission| match admission {
                Some(admission) => Ok(value.into_memory_budget(admission)?),
                None => Ok(value),
            },
        )
    }

    /// The same selected replacement, with one final owner confined throughout
    /// the native private phase. All validation, copying and admission above
    /// the output boundary is shared with the canonical result path.
    pub fn unsigned_matrix_replace_private(
        &self,
        source: Value,
        index: u64,
        incoming: u64,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
    ) -> Result<mech_core::snapshot::PrivateUnsignedMatrix, InvokeError> {
        self.matrix_replace_with::<_, true>(
            source,
            &[index],
            incoming,
            may_consume,
            limits,
            work,
            coexistence,
            |value, admission| {
                let admission = admission.ok_or(InvokeError::InvalidInput)?;
                Ok(value.into_private_unsigned_matrix(admission)?)
            },
        )
        .map(|(private, _)| private)
    }

    fn matrix_replace_with<T, const UNSIGNED: bool>(
        &self,
        source: Value,
        indices: &[u64],
        incoming: u64,
        may_consume: bool,
        limits: MemoryBudgetLimits,
        work: &NativeLibraryWorkBudget,
        coexistence: ResourceDemand,
        finish: impl FnOnce(
            Value,
            Option<&mut mech_core::ManagedMemoryReservation>,
        ) -> Result<T, InvokeError>,
    ) -> Result<(T, bool), InvokeError> {
        finite_limits(self.template.origin().node(), limits).map_err(InvokeError::Binding)?;
        if UNSIGNED && self.unsigned_matrix_replace_width().is_none() {
            return Err(InvokeError::InvalidInput);
        }
        let kind = self
            .scalar_matrix_replace_kind()
            .ok_or(InvokeError::InvalidInput)?;
        let width = match kind {
            Kind::Bool | Kind::U8 => 1,
            Kind::U16 => 2,
            Kind::U32 => 4,
            Kind::U64 | Kind::I64 | Kind::F64 => 8,
        };
        if !kind.accepts_bits(incoming) || indices.len() + 2 != self.template.inputs().len() {
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
                let capacity = match kind {
                    Kind::Bool | Kind::I64 | Kind::F64 => length,
                    _ => matrix
                        .unsigned_buffer_capacity()
                        .ok_or(InvokeError::InvalidInput)?,
                };
                let ordinal = match indices {
                    [index] => {
                        let ordinal =
                            crate::resident::numeric::available_scalar_index(*index, length)?;
                        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                            &source,
                            self.template.inputs()[0].body(),
                        )?;
                        crate::resident::numeric::canonical_matrix_index(ordinal, rows, columns)?
                    }
                    [row, column] => {
                        let (rows, columns) = crate::resident::numeric::snapshot_matrix_dimensions(
                            &source,
                            self.template.inputs()[0].body(),
                        )?;
                        let row = crate::resident::numeric::available_scalar_index(*row, rows)?;
                        let column =
                            crate::resident::numeric::available_scalar_index(*column, columns)?;
                        context.charge(16, 0)?;
                        row.checked_mul(columns)
                            .and_then(|at| at.checked_add(column))
                            .ok_or(InvokeError::InvalidInput)?
                    }
                    _ => return Err(InvokeError::InvalidInput),
                };
                let old = word(matrix.elements(), kind, ordinal)?;
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
                // Exact scalar bits and unchanged shape/schema make this one
                // selected-word comparison exactly the snapshot changed bit,
                // including signed zero and distinct floating-point NaN payloads.
                context.charge(16, 1)?;
                if old == incoming {
                    return Ok((finish(source, None)?, false));
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
                // Shape retention and ownership handoff use this already admitted
                // envelope. The detached charge remains live on every failure exit.
                let mut admission = memory.reserve_capacity(metadata)?;
                let constructor = source
                    .numeric_matrix_replacement_constructor(self.schemas.clone())
                    .map_err(|_| InvokeError::InvalidInput)?;
                let (source, owned) = if may_consume {
                    match source.try_into_owned_numeric_matrix(&memory)? {
                        NumericMatrixOwnershipTransfer::Owned(buffer) => (None, Some(buffer)),
                        NumericMatrixOwnershipTransfer::Unavailable(source) => (Some(source), None),
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
                let value = constructor
                    .finish(buffer)
                    .map_err(|_| InvokeError::InvalidInput)?;
                let value = finish(value, Some(&mut admission))?;
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

fn word(values: SequenceView<'_>, kind: Kind, at: usize) -> Result<u64, InvokeError> {
    Ok(match (values, kind) {
        (SequenceView::U8(v), Kind::U8) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U16(v), Kind::U16) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U32(v), Kind::U32) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::U64(v), Kind::U64) => *v.get(at).ok_or(InvokeError::InvalidInput)?,
        (SequenceView::Bool(v), Kind::Bool) => {
            u64::from(*v.get(at).ok_or(InvokeError::InvalidInput)?)
        }
        (SequenceView::I64(v), Kind::I64) => *v.get(at).ok_or(InvokeError::InvalidInput)? as u64,
        (SequenceView::F64(v), Kind::F64) => v.get(at).ok_or(InvokeError::InvalidInput)?.bits(),
        _ => return Err(InvokeError::InvalidInput),
    })
}

fn copy(values: SequenceView<'_>, capacity: usize) -> Result<NumericMatrixBuffer, InvokeError> {
    macro_rules! words {
        ($v:expr, $kind:ident) => {{
            let mut next = Vec::new();
            next.try_reserve_exact(capacity)
                .map_err(|_| InvokeError::InvalidInput)?;
            if next.capacity() != capacity {
                return Err(InvokeError::InvalidInput);
            }
            next.extend_from_slice($v);
            next
        }};
    }
    Ok(match values {
        SequenceView::U8(v) => {
            NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U8(words!(v, U8)))
        }
        SequenceView::U16(v) => {
            NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U16(words!(v, U16)))
        }
        SequenceView::U32(v) => {
            NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U32(words!(v, U32)))
        }
        SequenceView::U64(v) => {
            NumericMatrixBuffer::Unsigned(UnsignedMatrixBuffer::U64(words!(v, U64)))
        }
        SequenceView::Bool(v) => NumericMatrixBuffer::Bool(words!(v, Bool).into_boxed_slice()),
        SequenceView::I64(v) => NumericMatrixBuffer::I64(words!(v, I64).into_boxed_slice()),
        SequenceView::F64(v) => NumericMatrixBuffer::F64(words!(v, F64).into_boxed_slice()),
        _ => return Err(InvokeError::InvalidInput),
    })
}

fn replace(buffer: &mut NumericMatrixBuffer, at: usize, incoming: u64) -> Result<(), InvokeError> {
    buffer
        .replace_word(at, incoming)
        .ok_or(InvokeError::InvalidInput)
}
