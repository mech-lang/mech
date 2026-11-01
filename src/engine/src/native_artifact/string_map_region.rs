//! Closed String iteration -> selected Map entry -> TextJoin compiler region.
use super::window_query_region::{Cursor, jump};
use super::{
    NativeCollectionResult, NativeControlLoop, NativeIterationAction as Action,
    NativeIterationTerminator as End, NativeMatchAction as Scalar, NativeMatchValue,
    NativeProgramArtifactPlan as Plan,
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize)]
pub struct NativeStringMapRegion {
    collection: u32,
    iteration: u32,
    source: NativeMatchValue,
    map: NativeMatchValue,
    selection_site: u32,
    owned_source_slots: Box<[NativeMatchValue]>,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeStringMapRegion {
    collection: u32, iteration: u32, source: NativeMatchValue, map: NativeMatchValue, selection_site: u32, owned_source_slots: Box<[NativeMatchValue]>,
});
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
    /// Complete last-use proof for the source and already-consumed Copy
    /// ancestors. Only matching CURRENT frozen owners may be retired; the
    /// runtime must still establish actual exclusive data ownership.
    pub fn owned_source_slots(&self) -> &[NativeMatchValue] {
        &self.owned_source_slots
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
        owned_source_slots: owned_source_slots(plan, *iteration, projection.source()),
    })
}

// This is not payload/source recognition. It uses the same complete original
// semantic inventory as all fusion passes, including cold/output/effect reads.
fn owned_source_slots(
    plan: &Plan,
    iteration: u32,
    source: NativeMatchValue,
) -> Box<[NativeMatchValue]> {
    use super::fusion_semantics::{Read, reads, semantic_actions};
    use super::{NativeIterationValueScope as Scope, NativeMatchTerminator as Term};
    let Some(NativeControlLoop::Collection(generator)) = plan.iterations().get(iteration as usize)
    else {
        return Box::new([]);
    };
    // A lexical read executed again by a parent loop is NOT a final use.
    if generator.parent().is_some()
        || plan.value_scopes().get(source.get() as usize)
            != Some(&Scope::Immutable { iteration: None })
    {
        return Box::new([]);
    }
    let Ok(actions) = semantic_actions(plan) else {
        return Box::new([]);
    };
    let Ok(reads) = reads(plan, &actions) else {
        return Box::new([]);
    };
    if !matches!(
        reads.get(source.get() as usize).map(Vec::as_slice),
        Some([Read::Other])
    ) {
        return Box::new([]);
    }
    // Numeric block order is not execution order. An ancestor is consumable
    // only when none of its reads is reachable after this generator begins.
    let mut future = vec![false; plan.blocks().len()];
    let mut pending = vec![generator.header()];
    while let Some(site) = pending.pop() {
        let Some(block) = plan.blocks().get(site as usize) else {
            return Box::new([]);
        };
        if core::mem::replace(&mut future[site as usize], true) {
            continue;
        }
        let successors = match block.terminator() {
            End::Scalar(Term::Jump(next)) => [Some(next), None],
            End::Scalar(Term::Branch { yes, no, .. } | Term::LiteralBranch { yes, no, .. }) => {
                [Some(yes), Some(no)]
            }
            End::Scalar(Term::Complete | Term::Reject) => [None, None],
            End::Header { body, exit, .. } => [Some(body), Some(exit)],
            End::TailHeader { body, .. } => [Some(body), None],
        };
        pending.extend(successors.into_iter().flatten());
    }
    let mut eligible = vec![false; plan.values().len()];
    eligible[source.get() as usize] = true;
    let mut slots = vec![source];
    loop {
        let before = slots.len();
        for action in actions.iter() {
            let Action::Scalar(Scalar::Copy {
                source: ancestor, ..
            }) = action
            else {
                continue;
            };
            let index = ancestor.get() as usize;
            if eligible[index]
                || plan.value_scopes().get(index) != Some(&Scope::Immutable { iteration: None })
            {
                continue;
            }
            if !reads[index].is_empty() && reads[index].iter().all(|read| {
                let Read::Action(site) = read else { return false; };
                !future[*site as usize]
                    && matches!(actions.get(*site as usize), Some(Action::Scalar(Scalar::Copy { output, .. }))
                        if eligible[output.get() as usize])
            }) {
                eligible[index] = true;
                slots.push(*ancestor);
            }
        }
        if slots.len() == before {
            break;
        }
    }
    slots.sort_unstable_by_key(|value| value.get());
    slots.into_boxed_slice()
}
