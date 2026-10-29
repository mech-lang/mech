//! Pre-admitted variable row length for the existing private packed owner.
//! Spare cells remain uninitialized until the native producer stores them.
use super::*;
use crate::{DimensionExpr, DimensionParameterId};

/// Metadata only: no source Value, data identity, payload or mutation authority.
/// A failed optional preparation leaves the original immutable owner intact.
pub struct PrivateUnsignedAppendPreparation {
    schema: SchemaId,
    key: SchemaKey,
    column: DimensionParameterId,
    scratch: Arc<ShapeInstance>,
    nodes: crate::ManagedNodeReservation,
    reservation: crate::ManagedMemoryReservation,
}
impl PrivateUnsignedAppendPreparation {
    pub fn retained_nodes(&self) -> u64 {
        self.nodes.nodes()
    }
    pub(super) fn retained_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
}

fn row_parameter(value: &Value) -> Option<DimensionParameterId> {
    let schema = value.schemas.as_ref()?.get(value.schema)?;
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = schema.body()
    else {
        return None;
    };
    if !matches!(
        element.as_ref(),
        SchemaBody::UnsignedInteger(
            IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64
        )
    ) || schema
        .dimension_parameters()
        .iter()
        .any(|p| p.lower_bound() != &DimensionExpr::Constant(0) || p.upper_bound().is_some())
    {
        return None;
    }
    let [rows, DimensionExpr::Parameter(column)] = dimensions.as_ref() else {
        return None;
    };
    if !matches!(
        rows,
        DimensionExpr::Constant(1) | DimensionExpr::Parameter(_)
    ) || rows == &DimensionExpr::Parameter(*column)
        || value.shape.resolve_dimension(rows).ok()? != 1
    {
        return None;
    }
    Some(*column)
}

impl Value {
    /// Optional append preparation for plain, dynamically sized rows. All
    /// metadata allocation/admission happens before a private loan or store.
    /// Other geometry keeps its ordinary selected buffer operation.
    pub fn prepare_private_unsigned_appends(
        &self,
        memory: &crate::ManagedMemoryBudget,
    ) -> Result<Option<PrivateUnsignedAppendPreparation>, SnapshotValueError> {
        let Some(column) = row_parameter(self) else {
            return Ok(None);
        };
        if self.memory_budget_retained_bytes(memory).is_none()
            || !matches!(self.data(), ValueData::Matrix(matrix)
                if matrix.unsigned_buffer_capacity().is_some())
        {
            return Ok(None);
        }
        let bytes = (core::mem::size_of::<ShapeInstance>()
            + 2 * core::mem::size_of::<usize>()
            + core::mem::size_of_val(self.shape.parameter_values())) as u64;
        let reservation = memory.reserve_capacity(bytes)?;
        let nodes = memory.reserve_nodes(2)?;
        memory.check_snapshot_import_allocation(
            bytes,
            core::mem::align_of::<ShapeInstance>() as u32,
        )?;
        let scratch = Arc::try_new(self.shape.as_ref().clone()).map_err(|_| {
            crate::MemoryRuntimeError::AllocationFailed {
                object: None,
                requested: bytes,
                alignment: core::mem::align_of::<ShapeInstance>() as u32,
                space: crate::MemorySpace::Host,
            }
        })?;
        Ok(Some(PrivateUnsignedAppendPreparation {
            schema: self.schema,
            key: self.schema_key,
            column,
            scratch,
            nodes,
            reservation,
        }))
    }
}

impl PrivateUnsignedMatrix {
    pub fn private_metadata_nodes(&self) -> u64 {
        self.append.as_ref().map_or(0, |a| a.retained_nodes())
    }
    pub(in crate::snapshot::validation) fn prepare_appends(
        schema: SchemaId,
        key: SchemaKey,
        shape: ShapeInstance,
        words: UnsignedMatrixBuffer,
        schemas: Arc<SchemaTable>,
        reservation: crate::ManagedMemoryReservation,
        preparation: PrivateUnsignedAppendPreparation,
    ) -> Result<Self, SnapshotValueError> {
        if schema != preparation.schema
            || key != preparation.key
            || reservation.budget() != preparation.reservation.budget()
        {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { key });
        }
        let mut private = Self::prepare(schema, shape, words, schemas, reservation)?;
        if row_parameter(&private.value) != Some(preparation.column) {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch { key });
        }
        private.append = Some(preparation);
        Ok(private)
    }

    pub fn capacity_elements(&self) -> usize {
        let ValueData::Matrix(matrix) = self.value.data() else {
            unreachable!("private matrix")
        };
        matrix
            .unsigned_buffer_capacity()
            .expect("plain unsigned buffer")
    }

    /// Safe scalar producer for the same reserved private phase. A miss changes
    /// neither initialized length nor cells and never reallocates or truncates.
    pub fn append_in_capacity(&mut self, word: u64) -> bool {
        if self.append.is_none() || self.elements().len() == self.capacity_elements() {
            return false;
        }
        let root = Arc::get_mut(&mut self.value.root).expect("private root");
        let data = Arc::get_mut(&mut root.data).expect("private data");
        let ValueData::Matrix(matrix) = &mut data.data else {
            unreachable!("private matrix")
        };
        macro_rules! push {
            ($words:expr, $ty:ty) => {{
                let Ok(word) = <$ty>::try_from(word) else {
                    return false;
                };
                $words.push(word);
            }};
        }
        match &mut matrix.elements {
            SequenceStorage::U8(words) => push!(words, u8),
            SequenceStorage::U16(words) => push!(words, u16),
            SequenceStorage::U32(words) => push!(words, u32),
            SequenceStorage::U64(words) => words.push(word),
            _ => unreachable!("plain unsigned buffer"),
        }
        true
    }

    /// Commit the initialized prefix produced through an exclusive native loan.
    /// Invalid geometry is rejected before changing the Vec length.
    ///
    /// # Safety
    /// Every cell between the previous initialized length and `length` must
    /// have been initialized as this exact unsigned kind. No native borrow may
    /// survive this call. The sealed producer must check reserved capacity
    /// before each traversal and advance length only after the typed store.
    pub unsafe fn assume_initialized_prefix(
        &mut self,
        length: usize,
    ) -> Result<(), SnapshotValueError> {
        if self.append.is_none()
            || length < self.elements().len()
            || length > self.capacity_elements()
        {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.value.schema_key,
            });
        }
        let root = Arc::get_mut(&mut self.value.root).expect("private root");
        let data = Arc::get_mut(&mut root.data).expect("private data");
        let ValueData::Matrix(matrix) = &mut data.data else {
            unreachable!("private matrix")
        };
        // SAFETY: the caller establishes the initialized prefix; the bounds
        // above keep it inside the same exclusive, non-reallocated backing.
        match &mut matrix.elements {
            SequenceStorage::U8(words) => unsafe { words.set_len(length) },
            SequenceStorage::U16(words) => unsafe { words.set_len(length) },
            SequenceStorage::U32(words) => unsafe { words.set_len(length) },
            SequenceStorage::U64(words) => unsafe { words.set_len(length) },
            _ => unreachable!("plain unsigned buffer"),
        }
        Ok(())
    }

    pub(super) fn finish_append_shape(&mut self) {
        let Some(preparation) = self.append.take() else {
            return;
        };
        let length = self.elements().len() as u64;
        let schema = self
            .ownership
            .schemas
            .get(self.value.schema)
            .expect("private schema");
        let registration =
            Arc::get_mut(&mut self.ownership.registration).expect("unpublished registration");
        let root = Arc::get_mut(&mut self.value.root).expect("private root");
        let data = Arc::get_mut(&mut root.data).expect("private data");
        // Temporarily release the two unpublished references to the final
        // shape. The pre-admitted scratch prevents allocation in make_mut.
        data.shape = preparation.scratch.clone();
        registration.shape = preparation.scratch.clone();
        Arc::get_mut(&mut self.value.shape)
            .expect("private shape")
            .update_parameter_checked(schema, preparation.column, length)
            .expect("prepared unbounded row shape");
        data.shape = self.value.shape.clone();
        registration.shape = self.value.shape.clone();
        // All scratch metadata retires before its capacity charge is released.
        drop(preparation);
    }
}
