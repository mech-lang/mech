//! Private format adapters. They neither derive semantic facts nor grant code
//! authority; the confined loader retains its independent association checks.
use super::proof;
use crate::native_artifact as native;
use mech_engine::native_artifact::{
    NativeBorrowedStringRegion, NativeSelectedTextJoin, NativeTailLoop, NativeTextLineMatrixRegion,
    NativeTextLineRetentionFrontier,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct BorrowedRegion {
    #[serde(deserialize_with = "proof")]
    proof: NativeBorrowedStringRegion,
    ready: usize,
    captures: usize,
    end: usize,
    slots: Box<[Option<usize>]>,
}
impl From<BorrowedRegion> for native::program_borrowed_layout::RegionLayout {
    fn from(w: BorrowedRegion) -> Self {
        Self {
            proof: w.proof,
            ready: w.ready,
            captures: w.captures,
            end: w.end,
            slots: w.slots,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Borrowed {
    regions: Vec<BorrowedRegion>,
    initializers: Vec<Option<usize>>,
    actions: Vec<Option<(usize, usize)>>,
    words: usize,
}
impl From<Borrowed> for native::program_borrowed_layout::Layout {
    fn from(w: Borrowed) -> Self {
        Self {
            regions: w.regions.into_iter().map(Into::into).collect(),
            initializers: w.initializers,
            actions: w.actions,
            words: w.words,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct TailRegion {
    iteration: u32,
    #[serde(deserialize_with = "proof")]
    tail: NativeTailLoop,
    borrowed: Option<usize>,
    map: Option<usize>,
    packed: Option<native::program_map_packed_layout::Proof>,
    words: usize,
    compute: u64,
    comparison: u64,
}
impl From<TailRegion> for native::program_tail_layout::Region {
    fn from(w: TailRegion) -> Self {
        Self {
            iteration: w.iteration,
            tail: w.tail,
            borrowed: w.borrowed,
            map: w.map,
            packed: w.packed,
            words: w.words,
            compute: w.compute,
            comparison: w.comparison,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Tails {
    regions: Vec<TailRegion>,
    sites: Vec<Option<usize>>,
    initializers: Vec<Option<usize>>,
    exits: Vec<Option<usize>>,
    words: usize,
}
impl From<Tails> for native::program_tail_layout::Layout {
    fn from(w: Tails) -> Self {
        Self {
            regions: w.regions.into_iter().map(Into::into).collect(),
            sites: w.sites,
            initializers: w.initializers,
            exits: w.exits,
            words: w.words,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Prefix {
    #[serde(deserialize_with = "proof")]
    frontier: NativeTextLineRetentionFrontier,
    #[serde(deserialize_with = "proof")]
    tail: NativeTailLoop,
    borrowed: usize,
    carrier: usize,
    header_work: u64,
}
impl From<Prefix> for native::program_prefix_layout::Region {
    fn from(w: Prefix) -> Self {
        Self {
            frontier: w.frontier,
            tail: w.tail,
            borrowed: w.borrowed,
            carrier: w.carrier,
            header_work: w.header_work,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct SelectedJoins {
    #[serde(deserialize_with = "proof")]
    regions: Box<[NativeSelectedTextJoin]>,
    sites: Box<[Option<native::program_selected_join_layout::Site>]>,
    pub(super) maps: Box<[Option<native::program_selected_join_layout::mapped::Map>]>,
}
impl From<SelectedJoins> for native::program_selected_join_layout::Layout {
    fn from(w: SelectedJoins) -> Self {
        Self {
            regions: w.regions,
            sites: w.sites,
            maps: w.maps,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct LineMatrices {
    #[serde(deserialize_with = "proof")]
    regions: Vec<NativeTextLineMatrixRegion>,
    sites: Vec<Option<native::program_line_matrix_layout::Site>>,
    matrices: Vec<Option<usize>>,
}
impl From<LineMatrices> for native::program_line_matrix_layout::Layout {
    fn from(w: LineMatrices) -> Self {
        Self {
            regions: w.regions,
            sites: w.sites,
            matrices: w.matrices,
        }
    }
}
