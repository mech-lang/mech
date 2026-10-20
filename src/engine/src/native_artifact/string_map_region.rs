//! Closed String iteration -> selected Map entry -> TextJoin compiler region.
use super::window_query_region::{Cursor, jump};
use super::{
    NativeCollectionResult, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as End, NativeMatchAction as Scalar, NativeMatchValue,
    NativeProgramArtifactPlan as Plan,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NativeStringMapRegion {
    collection: u32,
    iteration: u32,
    source: NativeMatchValue,
    map: NativeMatchValue,
    selection_site: u32,
}
impl NativeStringMapRegion {
    pub const fn collection(&self) -> u32 {
        self.collection
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn source(&self) -> NativeMatchValue {
        self.source
    }
    pub const fn map(&self) -> NativeMatchValue {
        self.map
    }
    pub const fn selection_site(&self) -> u32 {
        self.selection_site
    }
}
impl Plan {
    pub fn string_map_regions(&self) -> Vec<NativeStringMapRegion> {
        self.collections()
            .iter()
            .enumerate()
            .filter_map(|(id, _)| select(self, u32::try_from(id).ok()?))
            .collect()
    }
}
fn select(plan: &Plan, id: u32) -> Option<NativeStringMapRegion> {
    let collection = plan.collections().get(id as usize)?;
    if !matches!(collection.result(), NativeCollectionResult::TextJoin(_)) {
        return None;
    }
    // This existing proof owns actual factory selection, sole-use source
    // elements/results and immutable source/map capture. Complete its CFG
    // proof here: no filters, hidden operations, extra consumers or loops.
    let projection = collection.string_element_map_projection()?;
    let [iteration] = collection.generators() else {
        return None;
    };
    if *iteration != projection.iteration() {
        return None;
    }
    let NativeControlLoop::Collection(generator) = plan.iterations().get(*iteration as usize)?
    else {
        return None;
    };
    if generator.collection() != id {
        return None;
    }
    let header = plan.blocks().get(generator.header() as usize)?;
    if !matches!((header.action(),header.terminator()),
        (Action::Scalar(Scalar::None),End::Header {iteration:owner,body,exit})
        if owner==*iteration && body==generator.body() && exit==generator.exit())
    {
        return None;
    }
    let latch = plan.blocks().get(generator.latch() as usize)?;
    if !matches!(latch.action(),Action::Latch { iteration: owner } if owner==iteration)
        || jump(latch.terminator()) != Some(generator.header())
    {
        return None;
    }
    let mut cursor = Cursor {
        plan,
        iteration: *iteration,
        visited: vec![generator.header(), generator.latch()],
    };
    let (site, Action::CollectionElement { iteration: owner }, end) =
        cursor.action(generator.body())?
    else {
        return None;
    };
    if owner != iteration || site != projection.element_site() {
        return None;
    }
    let (site, Action::ProjectStringMapEntry { collection: owner }, end) =
        cursor.action(jump(end)?)?
    else {
        return None;
    };
    if *owner != id || site != projection.selection_site() {
        return None;
    }
    let (
        site,
        Action::CollectionPush {
            collection: owner,
            value,
        },
        end,
    ) = cursor.action(jump(end)?)?
    else {
        return None;
    };
    if *owner != id
        || *value != projection.output()
        || site != collection.push()
        || jump(end) != Some(generator.latch())
    {
        return None;
    }
    if plan.blocks().iter().enumerate().any(|(site, block)| {
        block.iteration() == Some(*iteration) && !cursor.visited.contains(&(site as u32))
    }) {
        return None;
    }
    Some(NativeStringMapRegion {
        collection: id,
        iteration: *iteration,
        source: projection.source(),
        map: projection.map(),
        selection_site: projection.selection_site(),
    })
}
