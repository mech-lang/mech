//! Explicit boundary adapters from the current mutable value model.

#[cfg(feature = "semantic-compiler")]
mod bytecode;
#[cfg(feature = "program")]
mod bytecode_aggregates;
#[cfg(feature = "semantic-compiler")]
mod compiler;
#[cfg(feature = "functions")]
mod function;
mod kind;
pub mod structures;
pub(crate) mod value;

#[cfg(all(test, feature = "f64", feature = "tuple"))]
#[path = "../state_journal/tests/hashed_cycles.rs"]
mod cycle_tests;

#[cfg(feature = "semantic-compiler")]
pub use self::bytecode::*;
#[cfg(feature = "program")]
pub use self::bytecode_aggregates::*;
#[cfg(feature = "semantic-compiler")]
pub use self::compiler::*;
#[cfg(feature = "functions")]
pub use self::function::*;
pub use self::kind::*;
pub use self::value::*;

/// Compatibility alias for the retired universal mutable-reference backing.
pub type MutableReference = crate::Ref<crate::LegacyValue>;

use crate::{
    DimensionEnvironmentBuilder, DimensionExpr, DimensionParameterDeclaration, EnumVariantSchema,
    KindId, NominalKey, NominalKind, SemanticModelError,
};

#[cfg(all(feature = "no_std", not(feature = "std")))]
use alloc::boxed::Box;
#[cfg(any(not(feature = "no_std"), feature = "std"))]
use std::boxed::Box;

pub trait LegacySemanticContext {
    fn resolve_named_kind(&mut self, legacy_id: u64) -> Result<KindId, SemanticModelError>;

    fn resolve_nominal(
        &mut self,
        nominal_kind: NominalKind,
        legacy_id: u64,
        legacy_name: &str,
    ) -> Result<LegacyNominalResolution, SemanticModelError>;

    fn resolve_unspecified_extent(
        &mut self,
        site: &LegacyExtentSite,
        dimensions: &mut DimensionEnvironmentBuilder,
    ) -> Result<LegacyResolvedExtent, SemanticModelError>;
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyNominalResolution {
    Atom {
        key: NominalKey,
    },
    Enum {
        key: NominalKey,
        variants: Box<[EnumVariantSchema]>,
    },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LegacyResolvedExtent {
    Dimensions(Box<[DimensionExpr]>),
    Cardinality(DimensionExpr),
    DynamicCardinality { upper_bound: Option<DimensionExpr> },
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LegacyTypeSource {
    Kind,
    ValueKind,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LegacyExtentRole {
    MatrixDimensions,
    TableRows,
    SetCardinality,
    MapCardinality,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct LegacyExtentSite {
    pub source: LegacyTypeSource,
    pub path: Box<[LegacyTypePathSegment]>,
    pub role: LegacyExtentRole,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LegacyTypePathSegment {
    MatrixElement,
    OptionElement,
    TupleElement(u32),
    RecordField(u32),
    TableColumn(u32),
    SetElement,
    MapKey,
    MapValue,
    TypeOf,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum LegacyValueKindTag {
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
    C64,
    R64,
    String,
    Bool,
    Id,
    Index,
    Empty,
    Any,
    None,
    Matrix,
    Enum,
    Record,
    Map,
    Atom,
    Table,
    Tuple,
    Reference,
    Set,
    Option,
    Kind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LegacyKindResolution {
    pub kind: crate::KindExpr,
    pub dimension_parameters: Box<[DimensionParameterDeclaration]>,
}
