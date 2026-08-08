//! Finalized semantic schemas, shape instances, and deterministic schema tables.

mod encoding;
mod shape;
mod table;
mod validation;

pub use self::shape::*;
pub use self::table::*;

use crate::{
    DimensionExpr, DimensionParameter, DimensionParameterDeclaration, NominalKey,
    SemanticModelError,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, string::String};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, string::String};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaDraft {
    pub dimension_parameters: Box<[DimensionParameterDeclaration]>,
    pub body: SchemaBody,
}

#[cfg_attr(feature = "serde", derive(Serialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Schema {
    dimension_parameters: Box<[DimensionParameter]>,
    body: SchemaBody,
}

impl SchemaDraft {
    pub fn finalize(self) -> Result<Schema, SemanticModelError> {
        validation::finalize_schema(self)
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaBody {
    Bool,
    UnsignedInteger(IntegerWidth),
    SignedInteger(IntegerWidth),
    FloatingPoint(FloatWidth),
    Complex(FloatWidth),
    Rational64,
    String,
    Id,
    Index,
    Atom(NominalKey),
    Enum {
        key: NominalKey,
        variants: Box<[EnumVariantSchema]>,
    },
    Option(Box<SchemaBody>),
    Tuple(Box<[SchemaBody]>),
    Record(Box<[SchemaField]>),
    Matrix {
        element: Box<SchemaBody>,
        dimensions: Box<[DimensionExpr]>,
    },
    Table {
        columns: Box<[SchemaField]>,
        rows: DimensionExpr,
    },
    Set {
        element: Box<SchemaBody>,
        cardinality: DimensionExpr,
    },
    Map {
        key: Box<SchemaBody>,
        value: Box<SchemaBody>,
        cardinality: DimensionExpr,
    },
    ReifiedType,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum IntegerWidth {
    W8 = 8,
    W16 = 16,
    W32 = 32,
    W64 = 64,
    W128 = 128,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[repr(u16)]
pub enum FloatWidth {
    W32 = 32,
    W64 = 64,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaField {
    pub name: String,
    pub schema: SchemaBody,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EnumVariantSchema {
    pub name: String,
    pub payload: Option<SchemaBody>,
}
