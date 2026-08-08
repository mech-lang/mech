//! Errors raised while constructing or canonicalizing semantic model values.

use crate::{MechError, MechErrorKind};

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
    DuplicateKindName {
        category: KindNameCategory,
        name: String,
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
