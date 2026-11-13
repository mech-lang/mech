//! Field-sensitive ownership, independently of source-value initialization.
//!
//! A metadata/tag read needs its scalar words, not an allocation reference.
//! An array dereference always keeps the owner alive. Aggregate transport
//! propagates only owners required by the destination's actual future uses.
//! The concrete heap still checks final exclusivity before any mutation.
use super::{flow::Flow, guards::Guards, layout::Layout, recipes::Recipe};
use crate::native_artifact::{NativeCompileError, compiler::backend};
use mech_engine::native_artifact::{
    NativeAggregateWord as Word, NativeCollectionSource as Source, NativeControlLoop as Loop,
    NativeIterationAction as Action, NativeProgramArtifactPlan as Plan,
    NativeScalarAggregateAction as Aggregate,
};
use std::collections::BTreeMap;

#[path = "owner_reads.rs"]
mod owner_reads;
mod transfers;

pub(super) struct OwnerFlow {
    slots: Vec<usize>,
    ids: BTreeMap<usize, usize>,
    live_in: Vec<Vec<u64>>,
    guard_read_reachable: Vec<Vec<u64>>,
    definitions: Vec<Vec<usize>>,
    successors: Vec<Vec<usize>>,
    reads: Vec<Vec<usize>>,
    copies: Vec<Vec<(usize, usize)>>,
    post_uses: Vec<Vec<usize>>,
    alias_families: Vec<usize>,
}
fn contains(words: &[u64], id: usize) -> bool {
    words[id / 64] & (1u64 << (id % 64)) != 0
}
fn insert(words: &mut [u64], id: usize) {
    words[id / 64] |= 1u64 << (id % 64);
}
impl OwnerFlow {
    pub fn derive(
        plan: &Plan,
        layout: &Layout,
        flow: &Flow,
        recipes: &[Recipe],
    ) -> Result<Self, NativeCompileError> {
        let slots = layout
            .types
            .iter()
            .enumerate()
            .flat_map(|(value, ty)| {
                ty.owners
                    .iter()
                    .map(move |&lane| layout.slot(value as u32) + lane)
            })
            .collect::<Vec<_>>();
        let ids = slots
            .iter()
            .enumerate()
            .map(|(id, &slot)| (slot, id))
            .collect::<BTreeMap<_, _>>();
        let owners = |value: u32| {
            layout
                .ty(value)
                .owners
                .iter()
                .map(|&lane| ids[&(layout.slot(value) + lane)])
                .collect::<Vec<_>>()
        };
        let owner = |value: u32, lane: usize| {
            ids.get(&(layout.slot(value) + lane))
                .copied()
                .ok_or_else(|| backend("compact owner transfer does not refer to an owner field"))
        };
        let mut definitions = vec![vec![]; recipes.len()];
        let mut uses = vec![vec![]; recipes.len()];
        let mut post_uses = vec![vec![]; recipes.len()];
        let mut copies = vec![vec![]; recipes.len()];
        for (site, recipe) in recipes.iter().enumerate() {
            if !flow.reachable[site] {
                continue;
            }
            definitions[site] = flow.defs[site]
                .iter()
                .flat_map(|&value| owners(value))
                .collect();
            let mut copy =
                |source: u32, output: u32, offset: usize| -> Result<(), NativeCompileError> {
                    for &lane in &layout.ty(output).owners {
                        copies[site].push((owner(output, lane)?, owner(source, offset + lane)?));
                    }
                    Ok(())
                };
            if !flow.skip_action[site] {
                match recipe {
                    Recipe::Aggregate(Aggregate::Copy { source, output }) => {
                        copy(source.get(), output.get(), 0)?
                    }
                    Recipe::Aggregate(Aggregate::Project {
                        source,
                        offset,
                        output,
                        ..
                    }) => copy(source.get(), output.get(), *offset)?,
                    Recipe::Aggregate(Aggregate::Pack { output, words }) => {
                        for &lane in &layout.ty(output.get()).owners {
                            let from = match words.get(lane) {
                                Some(Word::Field(source, field)) => {
                                    Some(owner(source.get(), *field)?)
                                }
                                Some(Word::Constant(0)) => None,
                                _ => {
                                    return Err(backend(
                                        "compact aggregate owner has no checked owner-word source",
                                    ));
                                }
                            };
                            if let Some(from) = from {
                                copies[site].push((owner(output.get(), lane)?, from));
                            }
                        }
                    }
                    Recipe::Read { source, .. } => uses[site].extend(owners(source.get())),
                    Recipe::Buffer { operands, .. } => {
                        // Includes the helper's owner inspection and all source
                        // payloads, even when the resulting value is unused.
                        for value in operands {
                            uses[site].extend(owners(value.get()));
                        }
                    }
                    Recipe::None => match plan.blocks()[site].action() {
                        Action::Initialize { iteration } | Action::Latch { iteration } => {
                            if let Loop::Tail(tail) = &plan.iterations()[*iteration as usize] {
                                let source = if matches!(
                                    plan.blocks()[site].action(),
                                    Action::Initialize { .. }
                                ) {
                                    tail.seed()
                                } else {
                                    tail.next()
                                };
                                copy(source.get(), tail.carrier().get(), 0)?;
                            }
                            // Collection initialization reads length only; its
                            // selected element actions below hold the payload.
                        }
                        Action::CollectionElement { iteration } => {
                            let Loop::Collection(generator) =
                                &plan.iterations()[*iteration as usize]
                            else {
                                return Err(backend("compact owner element lacks collection"));
                            };
                            if generator.element().is_some() {
                                if let Source::Value(source) = generator.source() {
                                    uses[site].extend(owners(source.get()));
                                }
                            }
                        }
                        Action::CollectionPush { value, .. } => {
                            uses[site].extend(owners(value.get()))
                        }
                        _ => {}
                    },
                    Recipe::Aggregate(Aggregate::Observe { .. })
                    | Recipe::Length { .. }
                    | Recipe::StringEqual { .. } => {}
                    Recipe::Pending(_) => {
                        return Err(backend(
                            "compact owner proof encountered an unresolved operation",
                        ));
                    }
                }
            }
            if flow.complete[site] {
                for &value in &flow.outputs {
                    post_uses[site].extend(owners(value));
                }
            }
        }
        let width = slots.len().div_ceil(64);
        let mut live_in = vec![vec![0u64; width]; recipes.len()];
        let mut parents = vec![vec![]; recipes.len()];
        for (site, edges) in flow.successors.iter().enumerate() {
            if flow.reachable[site] {
                for &next in edges {
                    parents[next].push(site);
                }
            }
        }
        let mut todo = (0..recipes.len())
            .filter(|&site| flow.reachable[site])
            .collect::<Vec<_>>();
        let mut queued = flow.reachable.clone();
        while let Some(site) = todo.pop() {
            queued[site] = false;
            let mut after = vec![0; width];
            for &next in &flow.successors[site] {
                for (out, &word) in after.iter_mut().zip(&live_in[next]) {
                    *out |= word;
                }
            }
            for &id in &post_uses[site] {
                insert(&mut after, id);
            }
            let mut before = after.clone();
            for &id in &definitions[site] {
                before[id / 64] &= !(1u64 << (id % 64));
            }
            for &id in &uses[site] {
                insert(&mut before, id);
            }
            for &(destination, source) in &copies[site] {
                if contains(&after, destination) {
                    insert(&mut before, source);
                }
            }
            if before != live_in[site] {
                live_in[site] = before;
                for &parent in &parents[site] {
                    if !queued[parent] {
                        queued[parent] = true;
                        todo.push(parent);
                    }
                }
            }
        }
        // A conservative candidate filter derived from the existing copy
        // edges. Checked buffer results may return their input unchanged.
        // Connectivity never grants mutation or retirement permission.
        let mut alias_families = (0..slots.len()).collect::<Vec<_>>();
        fn root(parents: &[usize], mut at: usize) -> usize {
            while parents[at] != at {
                at = parents[at];
            }
            at
        }
        let mut join = |left: usize, right: usize| {
            let left = root(&alias_families, left);
            let right = root(&alias_families, right);
            if left != right {
                alias_families[left] = right;
            }
        };
        for &(destination, source) in copies.iter().flatten() {
            join(destination, source);
        }
        for recipe in recipes {
            if let Recipe::Buffer {
                operands, output, ..
            } = recipe
            {
                if let Some(source) = operands.first() {
                    let destination = owners(output.get());
                    let source = owners(source.get());
                    if let ([destination], [source]) = (destination.as_slice(), source.as_slice()) {
                        join(*destination, *source);
                    }
                }
            }
        }
        let alias_families = (0..slots.len())
            .map(|id| root(&alias_families, id))
            .collect();
        let reads = uses
            .iter()
            .zip(&copies)
            .map(|(used, copied)| {
                let mut all = used.clone();
                all.extend(copied.iter().map(|&(_, source)| source));
                all.sort_unstable();
                all.dedup();
                all
            })
            .collect::<Vec<_>>();
        let guard_read_reachable =
            owner_reads::derive(&flow.successors, &definitions, &reads, &post_uses, width);
        Ok(Self {
            slots,
            ids,
            live_in,
            guard_read_reachable,
            definitions,
            successors: flow.successors.clone(),
            reads,
            copies,
            post_uses,
            alias_families,
        })
    }
    /// Retire only other homes that are dead on every continuation allowed
    /// by the current must-facts. Current reads, outputs and overwritten homes
    /// remain held. The actual buffer operation still checks exclusivity.
    pub fn retire_before_indexed_change(
        &self,
        flow: &Flow,
        guards: &Guards,
        site: usize,
        source_slot: usize,
    ) -> Vec<usize> {
        let family = self.alias_families[self.ids[&source_slot]];
        self.slots_in(&self.live_in[site])
            .into_iter()
            .filter(|slot| {
                let id = self.ids[slot];
                self.alias_families[id] == family
                    && !self.reads[site].contains(&id)
                    && !self.definitions[site].contains(&id)
                    && !self.post_uses[site].contains(&id)
                    && self.last_use_with_guards(flow, guards, site, *slot)
            })
            .collect()
    }
    pub fn live_at_entry(&self, site: usize, slot: usize) -> bool {
        contains(&self.live_in[site], self.ids[&slot])
    }
    pub fn live_after(&self, site: usize, slot: usize) -> bool {
        let id = self.ids[&slot];
        self.successors[site]
            .iter()
            .any(|&next| contains(&self.live_in[next], id))
    }
    /// A pure copy may transport scalar metadata without another held array
    /// reference. This is the SAME owner-field liveness used for retirement.
    /// Same-site output ports remain observable uses. Fresh allocations and
    /// buffer operations are deliberately not candidates here.
    pub fn unused_copy_owner(&self, site: usize, slot: usize) -> bool {
        let id = self.ids[&slot];
        self.copies[site]
            .iter()
            .any(|&(destination, _)| destination == id)
            && !self.post_uses[site].contains(&id)
            && !self.live_after(site, slot)
    }
    /// Existing held/dead authority, including current outputs and overwrites.
    /// This is permission to move one reference, not to mutate shared backing.
    pub fn can_finish_source(
        &self,
        flow: &Flow,
        guards: &Guards,
        site: usize,
        slot: usize,
    ) -> bool {
        let id = self.ids[&slot];
        contains(&self.live_in[site], id)
            && !self.definitions[site].contains(&id)
            && !self.post_uses[site].contains(&id)
            && (!self.live_after(site, slot) || self.last_use_with_guards(flow, guards, site, slot))
    }
    /// Exclude only branches contradicted by facts that MUST hold at this
    /// operation. Any intervening predicate redefinition declines the proof.
    /// A new definition of this owner ends the old visit before its terminator.
    /// This grants a candidate only; the heap's current exclusivity gate stays.
    pub fn last_use_with_guards(
        &self,
        flow: &Flow,
        guards: &Guards,
        frontier: usize,
        slot: usize,
    ) -> bool {
        let id = self.ids[&slot];
        let known = guards.known(frontier);
        if known.is_empty() || self.post_uses[frontier].contains(&id) {
            return false;
        }
        let mut after = known.clone();
        guards.advance(flow, frontier, &mut after);
        let mut pending = guards
            .successors(flow, frontier, &after)
            .into_iter()
            .map(|next| (next, guards.edge(frontier, next, &after)))
            .collect::<Vec<_>>();
        let mut states = vec![None::<super::guards::State>; flow.reachable.len()];
        while let Some((site, state)) = pending.pop() {
            // No syntactic continuation reads this old owner before a fresh
            // definition. Guarded edges are a subset of that graph, so this
            // subtree cannot affect the answer or any relevant join state.
            if !contains(&self.guard_read_reachable[site], id) {
                continue;
            }
            if let Some(existing) = &mut states[site] {
                if !existing.merge(&state) {
                    continue;
                }
            } else {
                states[site] = Some(state);
            }
            if self.reads[site].contains(&id) || self.post_uses[site].contains(&id) {
                return false;
            }
            if self.definitions[site].contains(&id) {
                continue;
            }
            let mut after = states[site].as_ref().unwrap().clone();
            guards.advance(flow, site, &mut after);
            pending.extend(
                guards
                    .successors(flow, site, &after)
                    .into_iter()
                    .map(|next| (next, guards.edge(site, next, &after))),
            );
        }
        true
    }
    fn slots_in(&self, words: &[u64]) -> Vec<usize> {
        let mut slots = vec![];
        for (word, &bits) in words.iter().enumerate() {
            let mut remaining = bits;
            while remaining != 0 {
                let bit = remaining.trailing_zeros() as usize;
                slots.push(self.slots[word * 64 + bit]);
                remaining &= remaining - 1;
            }
        }
        slots
    }
    pub fn retire_after(&self, site: usize) -> Vec<usize> {
        let mut held = self.live_in[site].clone();
        for &id in &self.definitions[site] {
            insert(&mut held, id);
        }
        for &next in &self.successors[site] {
            for (word, &live) in held.iter_mut().zip(&self.live_in[next]) {
                *word &= !live;
            }
        }
        self.slots_in(&held)
    }
    pub fn retire_bypass(&self, entry: usize, exit: usize, outputs: &[usize]) -> Vec<usize> {
        let mut held = self.live_in[entry].clone();
        for slot in outputs {
            insert(&mut held, self.ids[slot]);
        }
        for (word, &live) in held.iter_mut().zip(&self.live_in[exit]) {
            *word &= !live;
        }
        self.slots_in(&held)
    }
    pub fn retire_on_edge(&self, site: usize, next: usize) -> Vec<usize> {
        let mut held = vec![0; self.live_in[next].len()];
        for &successor in &self.successors[site] {
            for (word, &live) in held.iter_mut().zip(&self.live_in[successor]) {
                *word |= live;
            }
        }
        for (word, &live) in held.iter_mut().zip(&self.live_in[next]) {
            *word &= !live;
        }
        self.slots_in(&held)
    }
}
