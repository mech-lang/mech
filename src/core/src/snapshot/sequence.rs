use super::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, Rational64Value, ValueData, ValueDataDraft,
};
use crate::{FloatWidth, IntegerWidth, SchemaBody};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::ToString, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

#[derive(Clone, Debug)]
pub(super) enum SequenceStorage {
    U8(Box<[u8]>),
    U16(Box<[u16]>),
    U32(Box<[u32]>),
    U64(Box<[u64]>),
    U128(Box<[u128]>),
    I8(Box<[i8]>),
    I16(Box<[i16]>),
    I32(Box<[i32]>),
    I64(Box<[i64]>),
    I128(Box<[i128]>),
    F32(Box<[F32Bits]>),
    F64(Box<[F64Bits]>),
    Complex32(Box<[Complex32Bits]>),
    Complex64(Box<[Complex64Bits]>),
    Rational64(Box<[Rational64Value]>),
    Bool(Box<[bool]>),
    String(Box<[Box<str>]>),
    Id(Box<[u64]>),
    Index(Box<[u64]>),
    Unit(u64),
    Values(Box<[ValueData]>),
}

#[derive(Clone, Copy, Debug)]
pub enum SequenceView<'a> {
    U8(&'a [u8]),
    U16(&'a [u16]),
    U32(&'a [u32]),
    U64(&'a [u64]),
    U128(&'a [u128]),
    I8(&'a [i8]),
    I16(&'a [i16]),
    I32(&'a [i32]),
    I64(&'a [i64]),
    I128(&'a [i128]),
    F32(&'a [F32Bits]),
    F64(&'a [F64Bits]),
    Complex32(&'a [Complex32Bits]),
    Complex64(&'a [Complex64Bits]),
    Rational64(&'a [Rational64Value]),
    Bool(&'a [bool]),
    String(&'a [Box<str>]),
    Id(&'a [u64]),
    Index(&'a [u64]),
    Unit(u64),
    Values(&'a [ValueData]),
}

impl SequenceView<'_> {
    pub fn len(self) -> usize {
        match self {
            Self::U8(values) => values.len(),
            Self::U16(values) => values.len(),
            Self::U32(values) => values.len(),
            Self::U64(values) => values.len(),
            Self::U128(values) => values.len(),
            Self::I8(values) => values.len(),
            Self::I16(values) => values.len(),
            Self::I32(values) => values.len(),
            Self::I64(values) => values.len(),
            Self::I128(values) => values.len(),
            Self::F32(values) => values.len(),
            Self::F64(values) => values.len(),
            Self::Complex32(values) => values.len(),
            Self::Complex64(values) => values.len(),
            Self::Rational64(values) => values.len(),
            Self::Bool(values) => values.len(),
            Self::String(values) => values.len(),
            Self::Id(values) => values.len(),
            Self::Index(values) => values.len(),
            Self::Unit(len) => usize::try_from(len).unwrap_or(usize::MAX),
            Self::Values(values) => values.len(),
        }
    }

    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    /// Materializes canonical sequence elements without changing their
    /// schema-directed representation.
    pub fn to_values(self) -> Vec<ValueData> {
        macro_rules! unpack {
            ($values:expr, $variant:ident) => {
                $values.iter().cloned().map(ValueData::$variant).collect()
            };
        }

        match self {
            Self::U8(values) => unpack!(values, U8),
            Self::U16(values) => unpack!(values, U16),
            Self::U32(values) => unpack!(values, U32),
            Self::U64(values) => unpack!(values, U64),
            Self::U128(values) => unpack!(values, U128),
            Self::I8(values) => unpack!(values, I8),
            Self::I16(values) => unpack!(values, I16),
            Self::I32(values) => unpack!(values, I32),
            Self::I64(values) => unpack!(values, I64),
            Self::I128(values) => unpack!(values, I128),
            Self::F32(values) => unpack!(values, F32),
            Self::F64(values) => unpack!(values, F64),
            Self::Complex32(values) => unpack!(values, Complex32),
            Self::Complex64(values) => unpack!(values, Complex64),
            Self::Rational64(values) => unpack!(values, Rational64),
            Self::Bool(values) => values.iter().copied().map(ValueData::Bool).collect(),
            Self::String(values) => values.iter().cloned().map(ValueData::String).collect(),
            Self::Id(values) => unpack!(values, Id),
            Self::Index(values) => unpack!(values, Index),
            Self::Unit(count) => (0..count).map(|_| ValueData::Atom).collect(),
            Self::Values(values) => values.to_vec(),
        }
    }
}

impl SequenceStorage {
    /// Reconstructs drafts directly from homogeneous packed scalar storage.
    /// This avoids first expanding every lane to `ValueData` and allocating a
    /// diagnostic path for every successful scalar conversion.
    pub(super) fn scalar_drafts(&self, schema: &SchemaBody) -> Option<Box<[ValueDataDraft]>> {
        macro_rules! drafts {
            ($values:expr, $variant:ident) => {
                $values
                    .iter()
                    .cloned()
                    .map(ValueDataDraft::$variant)
                    .collect::<Vec<_>>()
                    .into_boxed_slice()
            };
        }

        Some(match (schema, self) {
            (SchemaBody::UnsignedInteger(IntegerWidth::W8), Self::U8(values)) => {
                drafts!(values, U8)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W16), Self::U16(values)) => {
                drafts!(values, U16)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W32), Self::U32(values)) => {
                drafts!(values, U32)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W64), Self::U64(values)) => {
                drafts!(values, U64)
            }
            (SchemaBody::UnsignedInteger(IntegerWidth::W128), Self::U128(values)) => {
                drafts!(values, U128)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W8), Self::I8(values)) => {
                drafts!(values, I8)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W16), Self::I16(values)) => {
                drafts!(values, I16)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W32), Self::I32(values)) => {
                drafts!(values, I32)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W64), Self::I64(values)) => {
                drafts!(values, I64)
            }
            (SchemaBody::SignedInteger(IntegerWidth::W128), Self::I128(values)) => {
                drafts!(values, I128)
            }
            (SchemaBody::FloatingPoint(FloatWidth::W32), Self::F32(values)) => {
                drafts!(values, F32)
            }
            (SchemaBody::FloatingPoint(FloatWidth::W64), Self::F64(values)) => {
                drafts!(values, F64)
            }
            (SchemaBody::Complex(FloatWidth::W32), Self::Complex32(values)) => {
                drafts!(values, Complex32)
            }
            (SchemaBody::Complex(FloatWidth::W64), Self::Complex64(values)) => {
                drafts!(values, Complex64)
            }
            (SchemaBody::Rational64, Self::Rational64(values)) => values
                .iter()
                .map(|value| ValueDataDraft::Rational64 {
                    numerator: value.numerator(),
                    denominator: value.denominator(),
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            (SchemaBody::Bool, Self::Bool(values)) => drafts!(values, Bool),
            (SchemaBody::String, Self::String(values)) => values
                .iter()
                .map(|value| ValueDataDraft::String(value.to_string()))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            (SchemaBody::Id, Self::Id(values)) => drafts!(values, Id),
            (SchemaBody::Index, Self::Index(values)) => drafts!(values, Index),
            (SchemaBody::Atom(_), Self::Unit(count)) => (0..*count)
                .map(|_| ValueDataDraft::Atom)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            _ => return None,
        })
    }

    pub(super) fn from_values(schema: &SchemaBody, values: Vec<ValueData>) -> Self {
        macro_rules! pack {
            ($variant:ident, $target:ident) => {{
                let mut packed = Vec::with_capacity(values.len());
                for value in values {
                    let ValueData::$variant(value) = value else {
                        unreachable!("validated sequence changed representation")
                    };
                    packed.push(value);
                }
                Self::$target(packed.into_boxed_slice())
            }};
        }

        match schema {
            SchemaBody::UnsignedInteger(IntegerWidth::W8) => pack!(U8, U8),
            SchemaBody::UnsignedInteger(IntegerWidth::W16) => pack!(U16, U16),
            SchemaBody::UnsignedInteger(IntegerWidth::W32) => pack!(U32, U32),
            SchemaBody::UnsignedInteger(IntegerWidth::W64) => pack!(U64, U64),
            SchemaBody::UnsignedInteger(IntegerWidth::W128) => pack!(U128, U128),
            SchemaBody::SignedInteger(IntegerWidth::W8) => pack!(I8, I8),
            SchemaBody::SignedInteger(IntegerWidth::W16) => pack!(I16, I16),
            SchemaBody::SignedInteger(IntegerWidth::W32) => pack!(I32, I32),
            SchemaBody::SignedInteger(IntegerWidth::W64) => pack!(I64, I64),
            SchemaBody::SignedInteger(IntegerWidth::W128) => pack!(I128, I128),
            SchemaBody::FloatingPoint(FloatWidth::W32) => pack!(F32, F32),
            SchemaBody::FloatingPoint(FloatWidth::W64) => pack!(F64, F64),
            SchemaBody::Complex(FloatWidth::W32) => pack!(Complex32, Complex32),
            SchemaBody::Complex(FloatWidth::W64) => pack!(Complex64, Complex64),
            SchemaBody::Rational64 => pack!(Rational64, Rational64),
            SchemaBody::Bool => pack!(Bool, Bool),
            SchemaBody::String => pack!(String, String),
            SchemaBody::Id => pack!(Id, Id),
            SchemaBody::Index => pack!(Index, Index),
            SchemaBody::Atom(_) => Self::Unit(values.len() as u64),
            SchemaBody::Dynamic => Self::Values(values.into_boxed_slice()),
            _ => Self::Values(values.into_boxed_slice()),
        }
    }

    pub(super) fn view(&self) -> SequenceView<'_> {
        match self {
            Self::U8(values) => SequenceView::U8(values),
            Self::U16(values) => SequenceView::U16(values),
            Self::U32(values) => SequenceView::U32(values),
            Self::U64(values) => SequenceView::U64(values),
            Self::U128(values) => SequenceView::U128(values),
            Self::I8(values) => SequenceView::I8(values),
            Self::I16(values) => SequenceView::I16(values),
            Self::I32(values) => SequenceView::I32(values),
            Self::I64(values) => SequenceView::I64(values),
            Self::I128(values) => SequenceView::I128(values),
            Self::F32(values) => SequenceView::F32(values),
            Self::F64(values) => SequenceView::F64(values),
            Self::Complex32(values) => SequenceView::Complex32(values),
            Self::Complex64(values) => SequenceView::Complex64(values),
            Self::Rational64(values) => SequenceView::Rational64(values),
            Self::Bool(values) => SequenceView::Bool(values),
            Self::String(values) => SequenceView::String(values),
            Self::Id(values) => SequenceView::Id(values),
            Self::Index(values) => SequenceView::Index(values),
            Self::Unit(count) => SequenceView::Unit(*count),
            Self::Values(values) => SequenceView::Values(values),
        }
    }

    pub(super) fn to_values(&self) -> Vec<ValueData> {
        macro_rules! unpack {
            ($values:expr, $variant:ident) => {
                $values.iter().cloned().map(ValueData::$variant).collect()
            };
        }

        match self {
            Self::U8(values) => unpack!(values, U8),
            Self::U16(values) => unpack!(values, U16),
            Self::U32(values) => unpack!(values, U32),
            Self::U64(values) => unpack!(values, U64),
            Self::U128(values) => unpack!(values, U128),
            Self::I8(values) => unpack!(values, I8),
            Self::I16(values) => unpack!(values, I16),
            Self::I32(values) => unpack!(values, I32),
            Self::I64(values) => unpack!(values, I64),
            Self::I128(values) => unpack!(values, I128),
            Self::F32(values) => unpack!(values, F32),
            Self::F64(values) => unpack!(values, F64),
            Self::Complex32(values) => unpack!(values, Complex32),
            Self::Complex64(values) => unpack!(values, Complex64),
            Self::Rational64(values) => unpack!(values, Rational64),
            Self::Bool(values) => unpack!(values, Bool),
            Self::String(values) => values.iter().cloned().map(ValueData::String).collect(),
            Self::Id(values) => unpack!(values, Id),
            Self::Index(values) => unpack!(values, Index),
            Self::Unit(count) => (0..*count).map(|_| ValueData::Atom).collect(),
            Self::Values(values) => values.to_vec(),
        }
    }
}
