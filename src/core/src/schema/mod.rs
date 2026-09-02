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

/// Declares whether an aggregate extent is fixed by its schema or may vary
/// while preserving the same semantic schema and cell identity.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CardinalitySpec {
    Exact(DimensionExpr),
    Dynamic { upper_bound: Option<DimensionExpr> },
}

/// General name for an exact or dynamic aggregate extent. The cardinality
/// alias remains available for source compatibility with the first set slice.
pub type ExtentSpec = CardinalitySpec;

impl From<DimensionExpr> for CardinalitySpec {
    fn from(value: DimensionExpr) -> Self {
        Self::Exact(value)
    }
}

impl SchemaDraft {
    pub fn finalize(self) -> Result<Schema, SemanticModelError> {
        validation::finalize_schema(self)
    }
}

impl Schema {
    pub fn type_memory_contract(&self) -> Result<crate::TypeMemoryContract, SemanticModelError> {
        crate::memory_contract::derive_type_memory_contract(&self.body, &self.dimension_parameters)
    }

    pub fn resolved_type_memory_contract(
        &self,
        shape: &crate::ShapeInstance,
    ) -> Result<crate::ResolvedTypeMemoryContract, SemanticModelError> {
        let validated_shape =
            self.instantiate_shape(shape.parameter_values().to_vec().into_boxed_slice())?;
        crate::memory_contract::resolve_type_memory_contract(
            self.type_memory_contract()?,
            &self.dimension_parameters,
            &validated_shape,
        )
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SchemaBody {
    /// A self-describing value whose concrete schema and shape are carried by
    /// the value itself. This is the instantiable semantic form of a source
    /// wildcard (`*`) inside heterogeneous aggregates such as table columns.
    Dynamic,
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
        rows: ExtentSpec,
    },
    Set {
        element: Box<SchemaBody>,
        cardinality: CardinalitySpec,
    },
    Map {
        key: Box<SchemaBody>,
        value: Box<SchemaBody>,
        cardinality: ExtentSpec,
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
