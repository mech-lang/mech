//! Shared checked numeric representation for leaves, parent pieces, and phases.
//! Source membership, ownership authority and prices are independently derived;
//! these records describe operations and word homes, never a second program.
use super::readonly_numeric::ResultRecipe;
// Phases and leaves use the same checked carrier/capture storage description.
// The descriptor still rederives provenance and current-version eligibility.
pub(in crate::native_artifact) use super::readonly_numeric::Array;
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.FrameWords")]
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
    Reduce {
        fold: Box<super::numeric_phase::Fold>,
    },
    Map {
        replacement: Box<super::numeric_phase::matrix_maps::Replacement>,
    },
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
    CellRead {
        array: usize,
        row: u32,
        column: u32,
        output: u32,
    },
    CellWrite {
        array: usize,
        row: u32,
        column: u32,
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
    pub fn replaces(&self) -> bool {
        matches!(
            self,
            Self::Write { .. } | Self::CellWrite { .. } | Self::Map { .. }
        )
    }
    pub fn summary_after(&self) -> Option<u32> {
        match self {
            Self::Reduce { fold } => Some(fold.source.after),
            Self::Map { replacement } => Some(replacement.source.after),
            _ => None,
        }
    }
    /// The exact final canonical visit completed by this summary. Native CFG
    /// predecessors use its entry; ordinary replay resumes after this exit.
    pub fn summary_source_exit(&self) -> Option<u32> {
        match self {
            Self::Reduce { fold } => fold.source.region.reduction.as_ref().map(|r| r.select),
            Self::Map { replacement } => replacement.source.completion.last().copied(),
            _ => None,
        }
    }
    pub fn summary_budget(&self) -> Option<&super::numeric_phase::FoldBudget> {
        match self {
            Self::Reduce { fold } => Some(&fold.budget),
            Self::Map { replacement } => Some(&replacement.budget),
            _ => None,
        }
    }
    pub fn summary_values(&self) -> Vec<u32> {
        match self {
            Self::Reduce { fold } => fold.source.scalar_values().collect(),
            Self::Map { replacement } => replacement.source.scalar_values().collect(),
            _ => Vec::new(),
        }
    }
    pub fn replacement_array(&self) -> Option<usize> {
        match self {
            Self::Write { array, .. } | Self::CellWrite { array, .. } => Some(*array),
            Self::Map { replacement } => Some(replacement.target),
            _ => None,
        }
    }
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
#[serde(deny_unknown_fields, rename = "mech.native.image.Exit")]
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
impl Exit {
    pub fn source_predecessor(&self, actions: &[(usize, Operation)]) -> usize {
        actions
            .iter()
            .find(|(site, action)| {
                *site == self.from && action.summary_after() == Some(self.to as u32)
            })
            .and_then(|(_, action)| action.summary_source_exit())
            .map_or(self.from, |site| site as usize)
    }
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.BoundaryValues")]
pub(in crate::native_artifact) struct BoundaryValues {
    /// Superset of every source word that this region can register or modify.
    pub candidates: Vec<u32>,
    pub header: Vec<u32>,
    pub exit: Vec<u32>,
}
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Plan")]
pub(in crate::native_artifact) struct Plan {
    pub arrays: Vec<Array>,
    pub map_banks: Vec<super::numeric_phase::map_banks::Bank>,
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
