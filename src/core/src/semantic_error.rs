//! Errors raised while constructing or canonicalizing semantic model values.

use crate::{
    DimensionOperator, DimensionParameterId, KindId, KindParameterId, MechError, MechErrorKind,
    SchemaKey,
};

#[cfg(feature = "no_std")]
use alloc::string::String;
#[cfg(not(feature = "no_std"))]
use std::string::String;

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SemanticIdentityKind {
    ReactiveInstanceGeneration,
    InstanceEpoch,
    PlanGeneration,
    LayoutGeneration,
    DimensionParameterId,
    ConstantHandle,
    ConstantId,
    SchemaHandle,
    SchemaId,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NominalPathError {
    EmptyPath,
    EmptySegment,
    DotSegment,
    DotDotSegment,
    ContainsNul,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NonInstantiableKind {
    Wildcard,
    Never,
    Hole,
    Parameter,
    Reference,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SchemaNameCategory {
    EnumVariant,
    RecordField,
    TableColumn,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum KindNameCategory {
    RecordField,
    TableColumn,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SemanticModelError {
    IdentityExhausted {
        identity: SemanticIdentityKind,
    },
    InvalidNominalPath {
        segment: Option<u32>,
        reason: NominalPathError,
    },
    UnknownNamedKind {
        id: KindId,
    },
    UnknownKindParameter {
        id: KindParameterId,
    },
    DuplicateKindParameter {
        id: KindParameterId,
    },
    ForwardKindParameterReference {
        parameter: KindParameterId,
        referenced: KindParameterId,
    },
    UnknownDimensionParameterV1 {
        id: DimensionParameterId,
    },
    DuplicateDimensionParameter {
        id: DimensionParameterId,
    },
    CyclicDimensionParameterBoundsV1,
    ForwardDimensionParameterReferenceV1 {
        parameter: DimensionParameterId,
        referenced: DimensionParameterId,
    },
    CompileTimeDimensionParameterV1,
    DimensionOverflowV1,
    EmptyMinMaxV1 {
        operator: DimensionOperator,
    },
    UnresolvedDimensionHole,
    UnresolvedKindHole,
    KindParameterNotClosed {
        id: KindParameterId,
    },
    NonInstantiableKind {
        kind: NonInstantiableKind,
    },
    InvalidVariadicKindScheme,
    DuplicateKindName {
        category: KindNameCategory,
        name: String,
    },
    DuplicateSchemaNameV1 {
        category: SchemaNameCategory,
        name: String,
    },
    SchemaNotKeyableV1,
    ShapeParameterCountMismatchV1 {
        expected: u32,
        actual: u32,
    },
    ShapeBoundViolationV1 {
        parameter: DimensionParameterId,
        value: u64,
        lower: u64,
        upper: Option<u64>,
    },
    SchemaIdExhausted,
    InvalidSchemaHandleV1,
    SchemaKeyCollision {
        key: SchemaKey,
    },
    BuiltinScalarKindUnresolved {
        scalar_id: u64,
    },
}

impl MechErrorKind for SemanticModelError {
    fn name(&self) -> &str {
        "SemanticModelError"
    }

    fn message(&self) -> String {
        format!("Semantic model error: {:?}", self)
    }
}

impl From<SemanticModelError> for MechError {
    fn from(error: SemanticModelError) -> Self {
        MechError::new(error, None).with_compiler_loc()
    }
}
