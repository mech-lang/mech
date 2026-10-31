//! Typed ownership transfer into the existing packed numeric Matrix storage.
//! Callers admit their complete construction envelope before producing cells.
use super::*;
use crate::snapshot::{F32Bits, F64Bits};

fn from_numeric_matrix(
    schema: SchemaId,
    shape: ShapeInstance,
    expected_element: SchemaBody,
    elements: SequenceStorage,
    schemas: Arc<SchemaTable>,
) -> Result<Value, SnapshotValueError> {
    let entry = schemas
        .entry(schema)
        .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
    let mismatch = || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: entry.key() };
    entry
        .schema()
        .validate_shape(&shape)
        .map_err(|_| mismatch())?;
    let SchemaBody::Matrix {
        element,
        dimensions,
    } = entry.schema().body()
    else {
        return Err(mismatch());
    };
    // A typed constructor never certifies interval constraints, another scalar
    // kind, or a differently shaped schema just because the bytes fit.
    if element.as_ref() != &expected_element {
        return Err(mismatch());
    }
    let count = dimensions.iter().try_fold(1u64, |count, dimension| {
        count
            .checked_mul(shape.resolve_dimension(dimension)?)
            .ok_or_else(mismatch)
    })?;
    ensure_cardinality(&SnapshotPath::root(), count, elements.view().len())?;
    if let SequenceStorage::Index(values) = &elements {
        if let Some(index) = values.iter().position(|value| *value == 0) {
            return Err(SnapshotValueError::InvalidIndexV1 {
                path: SnapshotPath::root().child(SnapshotPathSegment::MatrixElement(index as u64)),
                value: 0,
            });
        }
    }
    Ok(finalized_value(
        schema,
        entry.key(),
        shape,
        ValueData::Matrix(MatrixValue { elements }),
        Some(schemas),
    ))
}

/// Detached packed words. Length describes initialized logical elements;
/// capacity describes owned storage and is never encoded as matrix data.
#[derive(Debug)]
pub enum UnsignedMatrixBuffer {
    U8(Vec<u8>),
    U16(Vec<u16>),
    U32(Vec<u32>),
    U64(Vec<u64>),
}
impl UnsignedMatrixBuffer {
    pub fn elements(&self) -> super::super::SequenceView<'_> {
        use super::super::SequenceView;
        match self {
            Self::U8(words) => SequenceView::U8(words),
            Self::U16(words) => SequenceView::U16(words),
            Self::U32(words) => SequenceView::U32(words),
            Self::U64(words) => SequenceView::U64(words),
        }
    }
    pub fn capacity_bytes(&self) -> usize {
        match self {
            Self::U8(words) => words.capacity(),
            Self::U16(words) => words.capacity() * core::mem::size_of::<u16>(),
            Self::U32(words) => words.capacity() * core::mem::size_of::<u32>(),
            Self::U64(words) => words.capacity() * core::mem::size_of::<u64>(),
        }
    }
}

impl Value {
    /// Freeze a caller-admitted packed buffer without shrinking its allocation
    /// or initializing unused capacity. Ordinary schema/shape checks still run.
    /// This detached constructor grants no execution or budget authority.
    pub fn from_unsigned_matrix_buffer(
        schema: SchemaId,
        shape: ShapeInstance,
        buffer: UnsignedMatrixBuffer,
        schemas: Arc<SchemaTable>,
    ) -> Result<Self, SnapshotValueError> {
        use IntegerWidth as W;
        let (width, storage) = match buffer {
            UnsignedMatrixBuffer::U8(words) => (W::W8, SequenceStorage::U8(words)),
            UnsignedMatrixBuffer::U16(words) => (W::W16, SequenceStorage::U16(words)),
            UnsignedMatrixBuffer::U32(words) => (W::W32, SequenceStorage::U32(words)),
            UnsignedMatrixBuffer::U64(words) => (W::W64, SequenceStorage::U64(words)),
        };
        from_numeric_matrix(
            schema,
            shape,
            SchemaBody::UnsignedInteger(width),
            storage,
            schemas,
        )
    }
}

macro_rules! constructor {
    ($name:ident, $ty:ty, $storage:ident, $body:expr) => {
        /// Transfer an already-admitted numeric buffer into canonical packed
        /// Matrix storage without drafts, payload cloning or a repacking pass.
        /// The schema must be a Matrix of this exact scalar kind; shape,
        /// cardinality and scalar invariants are still checked.
        #[doc(hidden)]
        pub fn $name(
            schema: SchemaId,
            shape: ShapeInstance,
            elements: Box<[$ty]>,
            schemas: Arc<SchemaTable>,
        ) -> Result<Self, SnapshotValueError> {
            from_numeric_matrix(
                schema,
                shape,
                $body,
                SequenceStorage::$storage(elements.into()),
                schemas,
            )
        }
    };
}

impl Value {
    constructor!(
        from_u8_matrix,
        u8,
        U8,
        SchemaBody::UnsignedInteger(IntegerWidth::W8)
    );
    constructor!(
        from_u16_matrix,
        u16,
        U16,
        SchemaBody::UnsignedInteger(IntegerWidth::W16)
    );
    constructor!(
        from_u32_matrix,
        u32,
        U32,
        SchemaBody::UnsignedInteger(IntegerWidth::W32)
    );
    constructor!(
        from_u64_matrix,
        u64,
        U64,
        SchemaBody::UnsignedInteger(IntegerWidth::W64)
    );
    constructor!(
        from_u128_matrix,
        u128,
        U128,
        SchemaBody::UnsignedInteger(IntegerWidth::W128)
    );
    constructor!(
        from_i8_matrix,
        i8,
        I8,
        SchemaBody::SignedInteger(IntegerWidth::W8)
    );
    constructor!(
        from_i16_matrix,
        i16,
        I16,
        SchemaBody::SignedInteger(IntegerWidth::W16)
    );
    constructor!(
        from_i32_matrix,
        i32,
        I32,
        SchemaBody::SignedInteger(IntegerWidth::W32)
    );
    constructor!(
        from_i64_matrix,
        i64,
        I64,
        SchemaBody::SignedInteger(IntegerWidth::W64)
    );
    constructor!(
        from_i128_matrix,
        i128,
        I128,
        SchemaBody::SignedInteger(IntegerWidth::W128)
    );
    constructor!(
        from_f32_matrix,
        F32Bits,
        F32,
        SchemaBody::FloatingPoint(FloatWidth::W32)
    );
    constructor!(
        from_f64_matrix,
        F64Bits,
        F64,
        SchemaBody::FloatingPoint(FloatWidth::W64)
    );
    constructor!(from_index_matrix, u64, Index, SchemaBody::Index);
}

#[cfg(test)]
#[path = "numeric_matrix_tests.rs"]
mod tests;
