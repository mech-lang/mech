//! Private format adapters. They neither derive semantic facts nor grant code
//! authority; the confined loader retains its independent association checks.
use crate::native_artifact as native;
use crate::native_artifact::program::image::decode::wire::proof;
use mech_engine::native_artifact::{
    NativeBorrowedStringRegion, NativeSelectedTextJoin, NativeTailLoop, NativeTextLineMatrixRegion,
    NativeTextLineRetentionFrontier,
};

#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.BorrowedRegion")]
struct BorrowedRegion {
    #[serde(deserialize_with = "proof")]
    proof: NativeBorrowedStringRegion,
    ready: usize,
    captures: usize,
    end: usize,
    slots: Box<[Option<usize>]>,
}
impl From<BorrowedRegion> for native::program::prepare::borrowed::RegionLayout {
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
#[serde(deny_unknown_fields, rename = "mech.native.image.Borrowed")]
pub(in crate::native_artifact::program::image::decode::wire) struct Borrowed {
    regions: Vec<BorrowedRegion>,
    initializers: Vec<Option<usize>>,
    actions: Vec<Option<(usize, usize)>>,
    words: usize,
}
impl From<Borrowed> for native::program::prepare::borrowed::Layout {
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
#[serde(deny_unknown_fields, rename = "mech.native.image.TailRegion")]
struct TailRegion<Frame> {
    iteration: u32,
    #[serde(deserialize_with = "proof")]
    tail: NativeTailLoop,
    borrowed: Option<usize>,
    map: Option<usize>,
    numeric: Option<native::program::prepare::readonly_numeric::Proof<Frame>>,
    numeric_tree: Option<NumericTree>,
    captures: Option<native::program::prepare::captured_numeric::Proof>,
    packed: Option<native::program::prepare::packed_map::Proof>,
    words: usize,
    shadow: Option<usize>,
    compute: u64,
    comparison: u64,
}
impl<Frame: Into<native::program::prepare::readonly_numeric::FrameWords>> From<TailRegion<Frame>>
    for native::program::prepare::tail::Region
{
    fn from(w: TailRegion<Frame>) -> Self {
        Self {
            iteration: w.iteration,
            tail: w.tail,
            borrowed: w.borrowed,
            map: w.map,
            numeric: w.numeric.map(|proof| proof.into_runtime()),
            numeric_tree: w.numeric_tree.map(Into::into),
            captures: w.captures,
            packed: w.packed,
            words: w.words,
            shadow: w.shadow,
            compute: w.compute,
            comparison: w.comparison,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.NumericTreeNode")]
struct NumericTreeNode {
    iteration: u32,
    parent: Option<usize>,
    #[serde(deserialize_with = "proof")]
    tail: NativeTailLoop,
    compute: u64,
    comparison: u64,
    scalar_bindings: Vec<native::program::prepare::readonly_numeric::ScalarBinding>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.NumericTree")]
struct NumericTree {
    nodes: Vec<NumericTreeNode>,
    sites: Vec<(usize, usize)>,
    counters: usize,
}
impl From<NumericTree> for native::program::prepare::numeric_tree::Proof {
    fn from(w: NumericTree) -> Self {
        Self {
            nodes: w
                .nodes
                .into_iter()
                .map(|node| native::program::prepare::numeric_tree::Node {
                    iteration: node.iteration,
                    parent: node.parent,
                    tail: node.tail,
                    compute: node.compute,
                    comparison: node.comparison,
                    scalar_bindings: node.scalar_bindings,
                })
                .collect(),
            sites: w.sites,
            counters: w.counters,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Tails")]
pub(in crate::native_artifact::program::image::decode::wire) struct Tails<Frame> {
    regions: Vec<TailRegion<Frame>>,
    sites: Vec<Option<usize>>,
    initializers: Vec<Option<usize>>,
    exits: Vec<Option<usize>>,
    words: usize,
    logical_words: usize,
}
impl<Frame: Into<native::program::prepare::readonly_numeric::FrameWords>> From<Tails<Frame>>
    for native::program::prepare::tail::Layout
{
    fn from(w: Tails<Frame>) -> Self {
        Self {
            regions: w.regions.into_iter().map(Into::into).collect(),
            sites: w.sites,
            initializers: w.initializers,
            exits: w.exits,
            words: w.words,
            logical_words: w.logical_words,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.Prefix")]
pub(in crate::native_artifact::program::image::decode::wire) struct Prefix {
    #[serde(deserialize_with = "proof")]
    frontier: NativeTextLineRetentionFrontier,
    #[serde(deserialize_with = "proof")]
    tail: NativeTailLoop,
    borrowed: usize,
    carrier: usize,
    header_work: u64,
}
impl From<Prefix> for native::program::prepare::prefix::Region {
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
#[serde(deny_unknown_fields, rename = "mech.native.image.SelectedJoins")]
pub(in crate::native_artifact::program::image::decode::wire) struct SelectedJoins {
    #[serde(deserialize_with = "proof")]
    regions: Box<[NativeSelectedTextJoin]>,
    sites: Box<[Option<native::program::prepare::selected_join::Site>]>,
    pub(in crate::native_artifact::program::image::decode::wire) maps:
        Box<[Option<native::program::prepare::selected_join::mapped::Map>]>,
}
impl From<SelectedJoins> for native::program::prepare::selected_join::Layout {
    fn from(w: SelectedJoins) -> Self {
        Self {
            regions: w.regions,
            sites: w.sites,
            maps: w.maps,
        }
    }
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields, rename = "mech.native.image.LineMatrices")]
pub(in crate::native_artifact::program::image::decode::wire) struct LineMatrices {
    #[serde(deserialize_with = "proof")]
    regions: Vec<NativeTextLineMatrixRegion>,
    sites: Vec<Option<native::program::prepare::line_matrix::Site>>,
    matrices: Vec<Option<usize>>,
}
impl From<LineMatrices> for native::program::prepare::line_matrix::Layout {
    fn from(w: LineMatrices) -> Self {
        Self {
            regions: w.regions,
            sites: w.sites,
            matrices: w.matrices,
        }
    }
}
