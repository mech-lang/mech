//! Typed ownership transfer into the existing packed numeric Matrix storage.
//! Callers admit their complete construction envelope before producing cells.
use super::*;
use crate::snapshot::{F32Bits, F64Bits};
use core::borrow::Borrow;

fn from_numeric_matrix<S: Borrow<ShapeInstance> + Into<Arc<ShapeInstance>>>(
    schema: SchemaId,
    shape: S,
    expected_element: SchemaBody,
    elements: SequenceStorage,
    schemas: Arc<SchemaTable>,
) -> Result<Value, SnapshotValueError> {
    let current_shape: &ShapeInstance = shape.borrow();
    let entry = schemas
        .entry(schema)
        .ok_or(SnapshotValueError::UnknownSnapshotSchema { schema })?;
    let mismatch = || SnapshotValueError::SnapshotSchemaDefinitionMismatch { key: entry.key() };
    entry
        .schema()
        .validate_shape(current_shape)
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
            .checked_mul(current_shape.resolve_dimension(dimension)?)
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
    fn into_storage(self) -> (IntegerWidth, SequenceStorage) {
        use IntegerWidth as W;
        match self {
            Self::U8(words) => (W::W8, SequenceStorage::U8(words)),
            Self::U16(words) => (W::W16, SequenceStorage::U16(words)),
            Self::U32(words) => (W::W32, SequenceStorage::U32(words)),
            Self::U64(words) => (W::W64, SequenceStorage::U64(words)),
        }
    }
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

/// Detached plain scalar storage for shape-preserving numeric updates. The
/// unsigned family keeps its capacity-bearing representation; the remaining
/// canonical scalar families keep their existing boxed slices.
#[derive(Debug)]
pub enum NumericMatrixBuffer {
    Unsigned(UnsignedMatrixBuffer),
    Bool(Box<[bool]>),
    I64(Box<[i64]>),
    F64(Box<[F64Bits]>),
}
impl NumericMatrixBuffer {
    fn into_storage(self) -> (SchemaBody, SequenceStorage) {
        match self {
            Self::Unsigned(buffer) => {
                let (width, storage) = buffer.into_storage();
                (SchemaBody::UnsignedInteger(width), storage)
            }
            Self::Bool(values) => (SchemaBody::Bool, SequenceStorage::Bool(values)),
            Self::I64(values) => (
                SchemaBody::SignedInteger(IntegerWidth::W64),
                SequenceStorage::I64(values),
            ),
            Self::F64(values) => (
                SchemaBody::FloatingPoint(FloatWidth::W64),
                SequenceStorage::F64(values),
            ),
        }
    }
    pub fn elements(&self) -> super::super::SequenceView<'_> {
        use super::super::SequenceView;
        match self {
            Self::Unsigned(buffer) => buffer.elements(),
            Self::Bool(values) => SequenceView::Bool(values),
            Self::I64(values) => SequenceView::I64(values),
            Self::F64(values) => SequenceView::F64(values),
        }
    }
    pub fn capacity_bytes(&self) -> usize {
        match self {
            Self::Unsigned(buffer) => buffer.capacity_bytes(),
            Self::Bool(values) => core::mem::size_of_val(values.as_ref()),
            Self::I64(values) => core::mem::size_of_val(values.as_ref()),
            Self::F64(values) => core::mem::size_of_val(values.as_ref()),
        }
    }
    /// Change exactly one initialized cell. Canonical narrow/Boolean words
    /// remain checked; signed and floating-point lanes preserve all bits.
    /// No raw mutable pointer or permission to resize storage is granted.
    pub fn replace_word(&mut self, at: usize, word: u64) -> Option<()> {
        macro_rules! unsigned {
            ($values:expr, $kind:ty) => {
                *$values.get_mut(at)? = <$kind>::try_from(word).ok()?
            };
        }
        match self {
            Self::Unsigned(UnsignedMatrixBuffer::U8(values)) => unsigned!(values, u8),
            Self::Unsigned(UnsignedMatrixBuffer::U16(values)) => unsigned!(values, u16),
            Self::Unsigned(UnsignedMatrixBuffer::U32(values)) => unsigned!(values, u32),
            Self::Unsigned(UnsignedMatrixBuffer::U64(values)) => *values.get_mut(at)? = word,
            Self::Bool(values) if word <= 1 => *values.get_mut(at)? = word != 0,
            Self::Bool(_) => return None,
            Self::I64(values) => *values.get_mut(at)? = word as i64,
            Self::F64(values) => *values.get_mut(at)? = F64Bits::from_bits(word),
        }
        Some(())
    }
}

pub(super) fn plain_numeric_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, .. }
        if matches!(element.as_ref(), SchemaBody::Bool
            | SchemaBody::UnsignedInteger(IntegerWidth::W8 | IntegerWidth::W16 | IntegerWidth::W32 | IntegerWidth::W64)
            | SchemaBody::SignedInteger(IntegerWidth::W64)
            | SchemaBody::FloatingPoint(FloatWidth::W64)))
}

/// Immutable same-schema metadata; it owns no payload or mutation authority.
#[derive(Debug)]
pub struct NumericMatrixSnapshotConstructor {
    schema: SchemaId,
    shape: Arc<ShapeInstance>,
    schemas: Arc<SchemaTable>,
}
impl NumericMatrixSnapshotConstructor {
    pub fn finish(self, buffer: NumericMatrixBuffer) -> Result<Value, SnapshotValueError> {
        let (kind, storage) = buffer.into_storage();
        from_numeric_matrix(self.schema, self.shape, kind, storage, self.schemas)
    }
}

/// A shape-preserving replacement retains immutable metadata only. It holds no
/// source payload, frozen root, registration, reservation or mutation authority.
#[derive(Debug)]
pub struct UnsignedMatrixSnapshotConstructor {
    schema: SchemaId,
    shape: Arc<ShapeInstance>,
    schemas: Arc<SchemaTable>,
}

impl UnsignedMatrixSnapshotConstructor {
    /// Recheck the exact kind, shape and cardinality and create fresh frozen
    /// data around a caller-admitted buffer. Sharing shape never shares the
    /// changed payload's frozen identity or grants budget authority.
    pub fn finish(self, buffer: UnsignedMatrixBuffer) -> Result<Value, SnapshotValueError> {
        let (width, storage) = buffer.into_storage();
        from_numeric_matrix(
            self.schema,
            self.shape,
            SchemaBody::UnsignedInteger(width),
            storage,
            self.schemas,
        )
    }
}

impl Value {
    /// Retain exact immutable metadata for a plain numeric update. Ownership
    /// transfer and complete current resource admission remain separate.
    pub fn numeric_matrix_replacement_constructor(
        &self,
        schemas: Arc<SchemaTable>,
    ) -> Result<NumericMatrixSnapshotConstructor, SnapshotValueError> {
        if !plain_numeric_matrix(self.validate_against(&schemas)?.body()) {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.schema_key,
            });
        }
        Ok(NumericMatrixSnapshotConstructor {
            schema: self.schema,
            shape: self.shape.clone(),
            schemas,
        })
    }

    /// Retain this value's immutable shape for a same-schema unsigned update.
    /// The supplied arena is checked by key, not just by numeric schema ID.
    /// Callers retain their ordinary work/capacity admission and ownership
    /// checks before consuming any source backing.
    pub fn unsigned_matrix_replacement_constructor(
        &self,
        schemas: Arc<SchemaTable>,
    ) -> Result<UnsignedMatrixSnapshotConstructor, SnapshotValueError> {
        use IntegerWidth as W;
        let definition = self.validate_against(&schemas)?;
        if !matches!(definition.body(), SchemaBody::Matrix { element, .. }
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(
                W::W8 | W::W16 | W::W32 | W::W64)))
        {
            return Err(SnapshotValueError::SnapshotSchemaDefinitionMismatch {
                key: self.schema_key,
            });
        }
        Ok(UnsignedMatrixSnapshotConstructor {
            schema: self.schema,
            shape: self.shape.clone(),
            schemas,
        })
    }

    /// Freeze a caller-admitted packed buffer without shrinking its allocation
    /// or initializing unused capacity. Ordinary schema/shape checks still run.
    /// This detached constructor grants no execution or budget authority.
    pub fn from_unsigned_matrix_buffer(
        schema: SchemaId,
        shape: ShapeInstance,
        buffer: UnsignedMatrixBuffer,
        schemas: Arc<SchemaTable>,
    ) -> Result<Self, SnapshotValueError> {
        let (width, storage) = buffer.into_storage();
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
    constructor!(from_bool_matrix, bool, Bool, SchemaBody::Bool);
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
