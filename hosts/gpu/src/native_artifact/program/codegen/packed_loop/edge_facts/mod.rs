//! Packed-map adapter to the shared current-traversal fact engine.
use super::*;
use crate::native_artifact::program::codegen::traversal_facts::{self, Scope, Step};
pub(super) use traversal_facts::Proof;

pub(super) fn derive(
    code: CodePlan<'_>,
    closed: &Layout,
    id: usize,
    owner: &NativeMapOwnerWords,
    aggregates: &NativeScalarAggregates,
    slots: &BTreeSet<usize>,
) -> Option<Proof> {
    let region = &closed.regions[id];
    let owner_index = region.map?;
    let scope = Scope {
        header: region.tail.header() as usize,
        latch: region.tail.latch() as usize,
        exit: region.tail.exit(),
        sites: closed
            .sites
            .iter()
            .enumerate()
            .filter_map(|(site, owner)| (*owner == Some(id)).then_some(site))
            .collect(),
    };
    let tag = owner
        .aggregates()
        .slot(region.tail.carrier())
        .map(|(slot, _)| slot);
    traversal_facts::derive(code, &scope, slots, tag, |site| {
        if let Some((layout, action)) = aggregate_at(owner, aggregates, owner_index, site) {
            return Some(Step::Aggregate(layout, action));
        }
        if let Some(span) = owner.span_site(site).map(|i| &owner.spans()[i]) {
            // Only this admitted private Frame is changed, never source words.
            return (span.region() == owner_index).then_some(Step::Keep);
        }
        if owner.call_region(site) == Some(owner_index) {
            let Action::LibraryCall {
                template, output, ..
            } = code.blocks()[site].action()
            else {
                return None;
            };
            return matches!(
                template.primitive(),
                P::MapContainsKey | P::MapKeyAccess | P::MapInsert
            )
            .then_some(Step::Define(output.get() as usize));
        }
        match code.blocks()[site].action() {
            Action::Scalar(action) => Some(Step::Scalar(action)),
            _ => None,
        }
    })
}
