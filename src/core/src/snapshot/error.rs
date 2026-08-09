use crate::{SchemaId, SchemaKey, SemanticModelError, ValueHash};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaDataKind {
    Bool,
    UnsignedInteger,
    SignedInteger,
    FloatingPoint,
    Complex,
    Rational64,
    String,
    Id,
    Index,
    Atom,
    Enum,
    Option,
    Tuple,
    Record,
    Matrix,
    Table,
    Set,
    Map,
    ReifiedType,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValueDataKind {
    U8,
    U16,
    U32,
    U64,
    U128,
    I8,
    I16,
    I32,
    I64,
    I128,
    F32,
    F64,
    Complex32,
    Complex64,
    Rational64,
    Bool,
    String,
    Id,
    Index,
    Atom,
    Enum,
    Option,
    Tuple,
    Record,
    Matrix,
    Table,
    Set,
    Map,
    ReifiedType,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SnapshotPath {
    segments: Box<[SnapshotPathSegment]>,
}

impl SnapshotPath {
    pub fn root() -> Self {
        Self {
            segments: Box::new([]),
        }
    }

    pub fn segments(&self) -> &[SnapshotPathSegment] {
        &self.segments
    }

    pub(crate) fn child(&self, segment: SnapshotPathSegment) -> Self {
        let mut segments = Vec::with_capacity(self.segments.len() + 1);
        segments.extend_from_slice(&self.segments);
        segments.push(segment);
        Self {
            segments: segments.into_boxed_slice(),
        }
    }
}

impl Default for SnapshotPath {
    fn default() -> Self {
        Self::root()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SnapshotPathSegment {
    OptionValue,
    EnumPayload(u32),
    TupleElement(u32),
    RecordField(u32),
    MatrixElement(u64),
    TableColumn(u32),
    TableRow(u64),
    SetElement(u64),
    MapKey(u64),
    MapValue(u64),
}

#[derive(Clone, Debug, PartialEq)]
pub enum SnapshotValueError {
    Semantic(SemanticModelError),
    UnknownSnapshotSchema {
        schema: SchemaId,
    },
    SnapshotSchemaTableMismatch {
        schema: SchemaId,
        expected: SchemaKey,
        actual: Option<SchemaKey>,
    },
    SnapshotSchemaDefinitionMismatch {
        key: SchemaKey,
    },
    SnapshotDataSchemaMismatch {
        path: SnapshotPath,
        expected: SchemaDataKind,
        actual: ValueDataKind,
    },
    AggregateArityMismatchV1 {
        path: SnapshotPath,
        expected: u64,
        actual: u64,
    },
    AggregateFieldMismatchV1 {
        path: SnapshotPath,
    },
    PayloadCardinalityMismatchV1 {
        path: SnapshotPath,
        expected: u64,
        actual: u64,
    },
    EnumOrdinalOutOfRangeV1 {
        path: SnapshotPath,
        ordinal: u32,
        variants: u32,
    },
    EnumPayloadMismatchV1 {
        path: SnapshotPath,
    },
    MapEntryArityMismatchV1 {
        path: SnapshotPath,
        actual: u64,
    },
    DuplicateCanonicalKeyV1 {
        path: SnapshotPath,
    },
    SchemaNotKeyableV1,
    NonCanonicalRationalV1,
    MissingNamedKindResolver,
    InvalidConstantHandleV1,
    ValueHashCollision {
        hash: ValueHash,
    },
}

impl From<SemanticModelError> for SnapshotValueError {
    fn from(error: SemanticModelError) -> Self {
        Self::Semantic(error)
    }
}
