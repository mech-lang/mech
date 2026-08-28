use super::{Complex32Bits, Complex64Bits, F32Bits, F64Bits, Rational64Value, ValueData};
use crate::{FloatWidth, IntegerWidth, SchemaBody};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
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

impl SequenceStorage {
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

    pub(super) fn len(&self) -> Option<usize> {
        Some(match self {
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
            Self::Unit(count) => usize::try_from(*count).ok()?,
            Self::Values(values) => values.len(),
        })
    }

    pub(super) fn rebuild_with_values(&self, values: Vec<ValueData>) -> Option<Self> {
        macro_rules! pack {
            ($variant:ident, $target:ident) => {{
                let values = values
                    .into_iter()
                    .map(|value| match value {
                        ValueData::$variant(value) => Some(value),
                        _ => None,
                    })
                    .collect::<Option<Vec<_>>>()?;
                Self::$target(values.into_boxed_slice())
            }};
        }

        Some(match self {
            Self::U8(_) => pack!(U8, U8),
            Self::U16(_) => pack!(U16, U16),
            Self::U32(_) => pack!(U32, U32),
            Self::U64(_) => pack!(U64, U64),
            Self::U128(_) => pack!(U128, U128),
            Self::I8(_) => pack!(I8, I8),
            Self::I16(_) => pack!(I16, I16),
            Self::I32(_) => pack!(I32, I32),
            Self::I64(_) => pack!(I64, I64),
            Self::I128(_) => pack!(I128, I128),
            Self::F32(_) => pack!(F32, F32),
            Self::F64(_) => pack!(F64, F64),
            Self::Complex32(_) => pack!(Complex32, Complex32),
            Self::Complex64(_) => pack!(Complex64, Complex64),
            Self::Rational64(_) => pack!(Rational64, Rational64),
            Self::Bool(_) => pack!(Bool, Bool),
            Self::String(_) => pack!(String, String),
            Self::Id(_) => pack!(Id, Id),
            Self::Index(_) => pack!(Index, Index),
            Self::Unit(_) => {
                if values.iter().any(|value| !matches!(value, ValueData::Atom)) {
                    return None;
                }
                Self::Unit(u64::try_from(values.len()).ok()?)
            }
            Self::Values(expected) => {
                if expected.len() != values.len()
                    || expected
                        .iter()
                        .zip(values.iter())
                        .any(|(expected, value)| expected.kind() != value.kind())
                {
                    return None;
                }
                Self::Values(values.into_boxed_slice())
            }
        })
    }
}
