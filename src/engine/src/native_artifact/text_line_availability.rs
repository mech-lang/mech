//! Current-visit prefix availability, not permission to discard input.
//! Each borrowed matrix read must be protected by an available-count edge in
//! THIS Tail traversal. A previous traversal's successful check is irrelevant.
use super::super::{NativeBorrowedStringAction as Borrowed, NativeTailLoop};
use super::super::{NativeScalarAggregateAction as Aggregate, NativeScalarAggregates};
use super::{
    NativeControlLoop, NativeTextLineCountFrontier, Plan, Value, definitions, predicate_branches,
    successors,
};

/// The exact original read and the count-guard ordinals that collectively
/// dominate its successful read on its current Tail traversal. Repeated tests
/// of the unchanged current carrier have the same truth value. Zero-index
/// checks, selected-call admission and canonical errors remain at their sites.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTextLineReadGuard {
    site: u32,
    iteration: u32,
    index: Value,
    guards: Box<[u32]>,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeTextLineReadGuard {
    site: u32, iteration: u32, index: Value, guards: Box<[u32]>,
});
impl NativeTextLineReadGuard {
    pub const fn site(&self) -> u32 {
        self.site
    }
    pub const fn iteration(&self) -> u32 {
        self.iteration
    }
    pub const fn index(&self) -> Value {
        self.index
    }
    /// Ordinals in this SAME frontier's count().guards(), not arbitrary CFG
    /// sites or proof records accepted from another plan.
    pub fn guards(&self) -> &[u32] {
        &self.guards
    }
}

/// A plan-local upper-bound proof for every borrowed read of one private line
/// matrix. It does NOT prove monotonic indices or future range retention, and
/// therefore MUST NOT be used alone to discard a source prefix or enable I/O.
#[derive(Clone, Debug, serde::Serialize)]
pub struct NativeTextLineReadFrontier {
    count: NativeTextLineCountFrontier,
    reads: Box<[NativeTextLineReadGuard]>,
}
crate::native_artifact::program_image_proofs::image_struct!(NativeTextLineReadFrontier {
    count: NativeTextLineCountFrontier, reads: Box<[NativeTextLineReadGuard]>,
});
impl NativeTextLineReadFrontier {
    pub fn count(&self) -> &NativeTextLineCountFrontier {
        &self.count
    }
    pub fn reads(&self) -> &[NativeTextLineReadGuard] {
        &self.reads
    }
}

// Identity-only Copy chains. No arithmetic equality, initial carrier content,
// or one matching phi arm is mistaken for equality of current index words.
pub(super) fn identity(defs: &[Vec<Option<Value>>], mut value: Value) -> Option<Value> {
    for _ in 0..defs.len() {
        let entries = &defs[value.get() as usize];
        match entries.first().copied().flatten() {
            Some(source) if entries.iter().all(|entry| *entry == Some(source)) => value = source,
            _ => return Some(value),
        }
    }
    None
}

#[derive(Clone, Debug, Eq, PartialEq)]
enum IndexIdentity {
    Value(Value),
    Field {
        source: Value,
        offset: usize,
        guards: Box<[(usize, u32)]>,
    },
}
fn index_identity(
    defs: &[Vec<Option<Value>>],
    aggregates: &NativeScalarAggregates,
    sites: usize,
    value: Value,
) -> Option<IndexIdentity> {
    let value = identity(defs, value)?;
    if defs[value.get() as usize].as_slice() == [None] {
        for site in 0..sites {
            if let Some(Aggregate::Observe {
                source,
                offset,
                ordinal: None,
                guards,
                output,
            }) = aggregates.action(site)
                && *output == value
            {
                return Some(IndexIdentity::Field {
                    source: identity(defs, *source)?,
                    offset: *offset,
                    guards: guards.clone(),
                });
            }
        }
    }
    Some(IndexIdentity::Value(value))
}

/// A successful guarded field read fixes the current carrier's enum ordinal.
/// Repeated ordinal tests in this SAME traversal cannot take contradictory
/// arms. Never use the seed, next state, a phi's one matching arm, or a previous
/// visit's predicate. Unknown/escaping predicates contribute no excluded edge.
fn current_state_edges(
    plan: &Plan,
    defs: &[Vec<Option<Value>>],
    uses: &[Vec<super::Read>],
    aggregates: &NativeScalarAggregates,
    tail: &NativeTailLoop,
    index: &IndexIdentity,
) -> Vec<(u32, u32)> {
    let IndexIdentity::Field { source, guards, .. } = index else {
        return Vec::new();
    };
    let [(0, state)] = guards.as_ref() else {
        return Vec::new();
    };
    if *source != tail.carrier() {
        return Vec::new();
    }
    let mut cut = Vec::new();
    for site in 0..plan.blocks().len() {
        let Some(Aggregate::Observe {
            source,
            offset: 0,
            ordinal: Some(ordinal),
            guards,
            output,
        }) = aggregates.action(site)
        else {
            continue;
        };
        if !guards.is_empty() || identity(defs, *source) != Some(tail.carrier()) {
            continue;
        }
        if let Some(branches) = predicate_branches(plan, site as u32, *output, uses, defs) {
            cut.extend(
                branches
                    .into_iter()
                    .map(|(site, no, yes)| (site, if ordinal == state { no } else { yes })),
            );
        }
    }
    cut
}

/// Never follow the latch backedge or the return edge. In particular, starting
/// at the original program entry would unsoundly reuse an earlier visit's edge.
fn traversal(
    plan: &Plan,
    iteration: u32,
    tail: &NativeTailLoop,
    cut: &[(u32, u32)],
) -> Option<Vec<bool>> {
    let mut seen = vec![false; plan.blocks().len()];
    let mut pending = vec![tail.body()];
    while let Some(site) = pending.pop() {
        if seen[site as usize] {
            continue;
        }
        let block = &plan.blocks()[site as usize];
        if block.iteration() != Some(iteration) || plan.activation_boundary() == Some(site) {
            return None;
        }
        seen[site as usize] = true;
        if site == tail.latch() || site == tail.finish() {
            continue;
        }
        for next in successors(block.terminator()).into_iter().flatten() {
            if !cut.contains(&(site, next)) {
                pending.push(next);
            }
        }
    }
    Some(seen)
}

fn frontier(
    plan: &Plan,
    count: NativeTextLineCountFrontier,
    defs: &[Vec<Option<Value>>],
    uses: &[Vec<super::Read>],
    borrowed: &[super::super::NativeBorrowedStringRegion],
    aggregates: &NativeScalarAggregates,
) -> Option<NativeTextLineReadFrontier> {
    let mut reads = Vec::new();
    for &ordinal in count.lines().borrowed_regions() {
        let region = borrowed.get(ordinal as usize)?;
        let iteration = region.iteration();
        let NativeControlLoop::Tail(tail) = plan.iterations().get(iteration as usize)? else {
            return None;
        };
        if tail.parent().is_some() {
            return None;
        }
        for (site, action) in region.actions() {
            let Borrowed::MatrixElement { source, index, .. } = action else {
                continue;
            };
            if *source != count.lines().matrix() {
                continue;
            }
            let selected_index = index_identity(defs, aggregates, plan.blocks().len(), *index)?;
            let mut cut = current_state_edges(plan, defs, uses, aggregates, tail, &selected_index);
            let whole = traversal(plan, iteration, tail, &cut)?;
            let guards = count
                .guards()
                .iter()
                .enumerate()
                .filter_map(|(ordinal, guard)| {
                    (plan.blocks()[guard.branch() as usize].iteration() == Some(iteration)
                        && index_identity(defs, aggregates, plan.blocks().len(), guard.index())
                            == Some(selected_index.clone()))
                    .then_some(ordinal as u32)
                })
                .collect::<Vec<_>>();
            if guards.is_empty() || !whole[*site as usize] {
                return None;
            }
            cut.extend(guards.iter().map(|&ordinal| {
                let guard = &count.guards()[ordinal as usize];
                (guard.branch(), guard.available())
            }));
            if traversal(plan, iteration, tail, &cut)?[*site as usize] {
                return None;
            }
            reads.push(NativeTextLineReadGuard {
                site: *site,
                iteration,
                index: *index,
                guards: guards.into(),
            });
        }
    }
    if reads.is_empty() {
        return None;
    }
    reads.sort_by_key(|read| read.site);
    Some(NativeTextLineReadFrontier {
        count,
        reads: reads.into(),
    })
}

impl Plan {
    /// Rederive both EOF and current-visit read proofs from this opaque plan.
    /// Consumers must separately prove retention and the owning host lifecycle.
    pub fn text_line_read_frontiers(&self) -> Vec<NativeTextLineReadFrontier> {
        self.facts().text_line_read_frontiers()
    }
}

pub(in crate::native_artifact) fn derive(
    plan: &Plan,
    facts: &crate::native_artifact::NativeProgramFacts<'_>,
) -> Vec<NativeTextLineReadFrontier> {
    let counts = plan.text_line_count_frontiers();
    if counts.is_empty() {
        return Vec::new();
    }
    let Ok(actions) = facts.actions() else {
        return Vec::new();
    };
    let Ok(uses) = facts.fusion_reads() else {
        return Vec::new();
    };
    let defs = definitions(plan, &actions);
    let borrowed = plan.borrowed_string_regions();
    let aggregates = facts.scalar_aggregates();
    counts
        .into_iter()
        .filter_map(|count| frontier(plan, count, &defs, &uses, &borrowed, &aggregates))
        .collect()
}

#[cfg(all(test, feature = "source", feature = "semantic-compiler"))]
#[path = "text_line_availability_tests.rs"]
mod tests;
