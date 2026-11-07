//! Shared checked numeric representation for leaves, parent pieces, and phases.
//! Source membership, ownership authority and prices are independently derived;
//! these records describe operations and word homes, never a second program.
use super::readonly_numeric::ResultRecipe;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Array {
    pub field: usize,
    pub width: u8,
    /// Exact selected site whose original factory validates the entry view.
    pub witness: usize,
    pub writable: bool,
    pub appends: bool,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct FrameWords {
    pub value: u32,
    /// Relative native scalar lane, or None for an invariant managed field.
    pub fields: Vec<Option<usize>>,
    pub arrays: Vec<Option<usize>>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum Operation {
    /// The summary edge is descriptive only; execute the original child here.
    Fallback,
    Frame,
    Aggregate,
    ArrayCopy,
    Length {
        array: usize,
        output: u32,
    },
    Read {
        array: usize,
        index: u32,
        output: u32,
    },
    Write {
        array: usize,
        index: u32,
        incoming: u32,
    },
    Append {
        array: usize,
        incoming: u32,
    },
    AppendLeBytes {
        array: usize,
        incoming: u32,
        count: u32,
    },
}
impl Operation {
    pub fn grows(&self) -> bool {
        matches!(self, Self::Append { .. } | Self::AppendLeBytes { .. })
    }
    #[cfg(feature = "canonical-native-program")]
    pub fn maximum_append(&self, array: usize) -> usize {
        match self {
            Self::Append { array: at, .. } if *at == array => 1,
            Self::AppendLeBytes { array: at, .. } if *at == array => 8,
            _ => 0,
        }
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) enum ExitKind {
    Unsupported,
    ChildInitialize { iteration: u32 },
    ChildHeader { iteration: u32 },
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Exit {
    pub from: usize,
    pub to: usize,
    pub frames: Vec<u32>,
    pub aliases: Vec<(u32, usize)>,
    /// Values read by this exact continuation before a new definition.
    pub values: Vec<u32>,
    /// All live fields have been exported, and the old root visit is dead.
    pub retire_carrier: bool,
    pub kind: ExitKind,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct BoundaryValues {
    /// Superset of every source word that this region can register or modify.
    pub candidates: Vec<u32>,
    pub header: Vec<u32>,
    pub exit: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Plan {
    pub arrays: Vec<Array>,
    pub frames: Vec<FrameWords>,
    pub actions: Vec<(usize, Operation)>,
    pub sites: Vec<usize>,
    pub prepaid_sites: Vec<usize>,
    pub scalar_words: usize,
    pub words: usize,
    pub result: ResultRecipe,
    pub phase: Option<super::numeric_phase::Schedule>,
    pub exits: Vec<Exit>,
    /// Source-local values that cannot observe their previous root traversal.
    pub fresh_values: Vec<u32>,
    pub boundary_values: BoundaryValues,
}
