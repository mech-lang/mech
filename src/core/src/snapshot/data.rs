use super::{
    SnapshotValueError,
    sequence::{SequenceStorage, SequenceView},
};
use crate::{
    CanonicalNominalPath, DimensionParameterDeclaration, KindExpr, KindId, NamedKindPathResolver,
    SchemaKey, canonical_closed_kind_bytes,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct F32Bits(u32);

impl F32Bits {
    pub const fn from_bits(bits: u32) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u32 {
        self.0
    }

    pub fn from_f32(value: f32) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f32(self) -> f32 {
        f32::from_bits(self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[repr(transparent)]
pub struct F64Bits(u64);

impl F64Bits {
    pub const fn from_bits(bits: u64) -> Self {
        Self(bits)
    }

    pub const fn bits(self) -> u64 {
        self.0
    }

    pub fn from_f64(value: f64) -> Self {
        Self(value.to_bits())
    }

    pub fn to_f64(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Complex32Bits {
    real: F32Bits,
    imaginary: F32Bits,
}

impl Complex32Bits {
    pub const fn new(real: F32Bits, imaginary: F32Bits) -> Self {
        Self { real, imaginary }
    }

    pub const fn real(&self) -> F32Bits {
        self.real
    }

    pub const fn imaginary(&self) -> F32Bits {
        self.imaginary
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct Complex64Bits {
    real: F64Bits,
    imaginary: F64Bits,
}

impl Complex64Bits {
    pub const fn new(real: F64Bits, imaginary: F64Bits) -> Self {
        Self { real, imaginary }
    }

    pub const fn real(&self) -> F64Bits {
        self.real
    }

    pub const fn imaginary(&self) -> F64Bits {
        self.imaginary
    }
}

#[derive(Clone, Debug)]
pub struct Rational64Value {
    numerator: i64,
    denominator: u64,
}

impl Rational64Value {
    pub fn new(numerator: i64, denominator: u64) -> Result<Self, SnapshotValueError> {
        if denominator == 0
            || gcd(numerator.unsigned_abs(), denominator) != 1
            || (numerator == 0 && denominator != 1)
        {
            return Err(SnapshotValueError::NonCanonicalRationalV1);
        }
        Ok(Self {
            numerator,
            denominator,
        })
    }

    pub const fn numerator(&self) -> i64 {
        self.numerator
    }

    pub const fn denominator(&self) -> u64 {
        self.denominator
    }
}

const fn gcd(mut left: u64, mut right: u64) -> u64 {
    while right != 0 {
        let remainder = left % right;
        left = right;
        right = remainder;
    }
    left
}

#[derive(Clone, Debug)]
pub enum ValueData {
    U8(u8),
    U16(u16),
    U32(u32),
    U64(u64),
    U128(u128),
    I8(i8),
    I16(i16),
    I32(i32),
    I64(i64),
    I128(i128),
    F32(F32Bits),
    F64(F64Bits),
    Complex32(Complex32Bits),
    Complex64(Complex64Bits),
    Rational64(Rational64Value),
    Bool(bool),
    String(Box<str>),
    Id(u64),
    Index(u64),
    Atom,
    Enum(EnumValue),
    Option(Option<Box<ValueData>>),
    Tuple(Box<[ValueData]>),
    Record(RecordValue),
    Matrix(MatrixValue),
    Table(TableValue),
    Set(SetValue),
    Map(MapValue),
    Type(ReifiedType),
}

impl ValueData {
    pub const fn kind(&self) -> super::ValueDataKind {
        use super::ValueDataKind;
        match self {
            Self::U8(_) => ValueDataKind::U8,
            Self::U16(_) => ValueDataKind::U16,
            Self::U32(_) => ValueDataKind::U32,
            Self::U64(_) => ValueDataKind::U64,
            Self::U128(_) => ValueDataKind::U128,
            Self::I8(_) => ValueDataKind::I8,
            Self::I16(_) => ValueDataKind::I16,
            Self::I32(_) => ValueDataKind::I32,
            Self::I64(_) => ValueDataKind::I64,
            Self::I128(_) => ValueDataKind::I128,
            Self::F32(_) => ValueDataKind::F32,
            Self::F64(_) => ValueDataKind::F64,
            Self::Complex32(_) => ValueDataKind::Complex32,
            Self::Complex64(_) => ValueDataKind::Complex64,
            Self::Rational64(_) => ValueDataKind::Rational64,
            Self::Bool(_) => ValueDataKind::Bool,
            Self::String(_) => ValueDataKind::String,
            Self::Id(_) => ValueDataKind::Id,
            Self::Index(_) => ValueDataKind::Index,
            Self::Atom => ValueDataKind::Atom,
            Self::Enum(_) => ValueDataKind::Enum,
            Self::Option(_) => ValueDataKind::Option,
            Self::Tuple(_) => ValueDataKind::Tuple,
            Self::Record(_) => ValueDataKind::Record,
            Self::Matrix(_) => ValueDataKind::Matrix,
            Self::Table(_) => ValueDataKind::Table,
            Self::Set(_) => ValueDataKind::Set,
            Self::Map(_) => ValueDataKind::Map,
            Self::Type(_) => ValueDataKind::ReifiedType,
        }
    }
}

#[derive(Clone, Debug)]
pub struct EnumValue {
    pub(super) ordinal: u32,
    pub(super) payload: Option<Box<ValueData>>,
}

impl EnumValue {
    pub const fn ordinal(&self) -> u32 {
        self.ordinal
    }

    pub fn payload(&self) -> Option<&ValueData> {
        self.payload.as_deref()
    }
}

#[derive(Clone, Debug)]
pub struct RecordValue {
    /// Stored in schema field order.
    pub(super) fields: Box<[ValueData]>,
}

impl RecordValue {
    pub fn fields(&self) -> &[ValueData] {
        &self.fields
    }
}

#[derive(Clone, Debug)]
pub struct MatrixValue {
    pub(super) elements: SequenceStorage,
}

impl MatrixValue {
    pub fn elements(&self) -> SequenceView<'_> {
        self.elements.view()
    }
}

#[derive(Clone, Debug)]
pub struct TableValue {
    /// Stored in schema column order.
    pub(super) columns: Box<[SequenceStorage]>,
}

impl TableValue {
    pub fn len(&self) -> usize {
        self.columns.len()
    }

    pub fn is_empty(&self) -> bool {
        self.columns.is_empty()
    }

    pub fn column(&self, index: usize) -> Option<SequenceView<'_>> {
        self.columns.get(index).map(SequenceStorage::view)
    }
}

#[derive(Clone, Debug)]
pub struct SetValue {
    /// Canonical key order.
    pub(super) elements: Box<[CanonicalKeyValue]>,
}

impl SetValue {
    pub fn elements(&self) -> &[CanonicalKeyValue] {
        &self.elements
    }
}

#[derive(Clone, Debug)]
pub struct MapValue {
    /// Canonical key order.
    pub(super) entries: Box<[MapEntryValue]>,
}

impl MapValue {
    pub fn entries(&self) -> &[MapEntryValue] {
        &self.entries
    }
}

#[derive(Clone, Debug)]
pub struct MapEntryValue {
    pub(super) key: CanonicalKeyValue,
    pub(super) value: ValueData,
}

impl MapEntryValue {
    pub fn key(&self) -> &CanonicalKeyValue {
        &self.key
    }

    pub fn value(&self) -> &ValueData {
        &self.value
    }
}

#[derive(Clone, Debug)]
pub struct CanonicalKeyValue {
    pub(super) data: ValueData,
}

impl CanonicalKeyValue {
    pub fn data(&self) -> &ValueData {
        &self.data
    }
}

#[derive(Clone, Debug)]
pub enum ReifiedType {
    Kind(ReifiedKind),
    Schema(SchemaKey),
}

#[derive(Clone, Debug)]
pub struct ReifiedKind {
    canonical_bytes: Box<[u8]>,
}

struct NoNamedKinds;

impl NamedKindPathResolver for NoNamedKinds {
    fn canonical_path(&self, _id: KindId) -> Option<&CanonicalNominalPath> {
        None
    }
}

fn kind_requires_named_resolver(kind: &KindExpr) -> bool {
    match kind {
        KindExpr::Named(_) => true,
        KindExpr::Matrix { element, .. }
        | KindExpr::Option(element)
        | KindExpr::Set { element, .. }
        | KindExpr::Reference(element)
        | KindExpr::TypeOf(element) => kind_requires_named_resolver(element),
        KindExpr::Tuple(elements) => elements.iter().any(kind_requires_named_resolver),
        KindExpr::Record(fields) => fields
            .iter()
            .any(|field| kind_requires_named_resolver(&field.kind)),
        KindExpr::Table { columns, .. } => columns
            .iter()
            .any(|column| kind_requires_named_resolver(&column.kind)),
        KindExpr::Map { key, value, .. } => {
            kind_requires_named_resolver(key) || kind_requires_named_resolver(value)
        }
        KindExpr::Wildcard
        | KindExpr::Never
        | KindExpr::Hole
        | KindExpr::Parameter(_)
        | KindExpr::Id
        | KindExpr::Index
        | KindExpr::Atom(_)
        | KindExpr::Enum(_) => false,
    }
}

impl ReifiedKind {
    pub fn from_closed_kind(
        kind: &KindExpr,
        dimensions: &[DimensionParameterDeclaration],
        named_kinds: &dyn NamedKindPathResolver,
    ) -> Result<Self, SnapshotValueError> {
        Ok(Self {
            canonical_bytes: canonical_closed_kind_bytes(kind, dimensions, named_kinds)?,
        })
    }

    pub(crate) fn from_closed_kind_with_optional_resolver(
        kind: &KindExpr,
        dimensions: &[DimensionParameterDeclaration],
        named_kinds: Option<&dyn NamedKindPathResolver>,
    ) -> Result<Self, SnapshotValueError> {
        if kind_requires_named_resolver(kind) {
            let resolver = named_kinds.ok_or(SnapshotValueError::MissingNamedKindResolver)?;
            return Self::from_closed_kind(kind, dimensions, resolver);
        }
        Self::from_closed_kind(kind, dimensions, &NoNamedKinds)
    }

    pub fn canonical_bytes(&self) -> &[u8] {
        &self.canonical_bytes
    }
}
