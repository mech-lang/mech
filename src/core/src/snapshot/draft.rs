use super::{
    Complex32Bits, Complex64Bits, F32Bits, F64Bits, SnapshotValidationContext, SnapshotValueError,
    Value,
};
use crate::{DimensionParameterDeclaration, KindExpr, SchemaId, SchemaKey};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct ValueDraft {
    pub schema: SchemaId,
    pub shape_values: Box<[u64]>,
    pub data: ValueDataDraft,
}

impl ValueDraft {
    pub fn finalize(
        self,
        context: &SnapshotValidationContext<'_>,
    ) -> Result<Value, SnapshotValueError> {
        super::validation::finalize_value(self, context)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum ValueDataDraft {
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
    Rational64 { numerator: i64, denominator: u64 },
    Bool(bool),
    String(String),
    Id(u64),
    Index(u64),
    Atom,
    Enum(EnumDraft),
    Option(OptionDraft),
    Tuple(Box<[ValueDataDraft]>),
    Record(Box<[NamedValueDraft]>),
    Matrix(Box<[ValueDataDraft]>),
    Table(Box<[TableColumnDraft]>),
    Set(Box<[ValueDataDraft]>),
    Map(Box<[MapEntryDraft]>),
    Type(ReifiedTypeDraft),
}

impl ValueDataDraft {
    pub(crate) const fn kind(&self) -> super::ValueDataKind {
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
            Self::Rational64 { .. } => ValueDataKind::Rational64,
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

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct NamedValueDraft {
    pub name: String,
    pub value: ValueDataDraft,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct TableColumnDraft {
    pub name: String,
    pub values: Box<[ValueDataDraft]>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct MapEntryDraft {
    /// Must contain exactly [key, value].
    pub items: Box<[ValueDataDraft]>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct OptionDraft {
    pub present: bool,
    pub value: Option<Box<ValueDataDraft>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub struct EnumDraft {
    pub ordinal: u32,
    pub payload: Option<Box<ValueDataDraft>>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq)]
pub enum ReifiedTypeDraft {
    Kind {
        kind: KindExpr,
        dimension_parameters: Box<[DimensionParameterDeclaration]>,
    },
    CanonicalKind(Box<[u8]>),
    Schema(SchemaKey),
}
