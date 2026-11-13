//! A checked read can borrow an aggregate copy while the exact source owner
//! stays live. Refcount-observing operations end the proof, even on a route
//! that would otherwise keep the backing alive.
use super::{OwnerFlow, contains};
use crate::native_artifact::program::compact::{
    flow::Flow,
    layout::{ARRAY_OWNER, Kind, Layout},
    recipes::Recipe,
};
use mech_engine::native_artifact::{
    NativeIterationAction as Action, NativeIterationTerminator as Term,
    NativeMatchAction as Scalar, NativeProgramArtifactPlan as Plan,
};
use std::collections::BTreeSet;

const MAX_BORROW_SITES: usize = 32;

impl OwnerFlow {
    pub(in crate::native_artifact::program::compact) fn borrow_read_copies(
        &self,
        plan: &Plan,
        layout: &Layout,
        flow: &Flow,
        recipes: &[Recipe],
        transfers: &[Vec<(usize, usize)>],
        boundaries: &BTreeSet<usize>,
    ) -> Vec<Vec<usize>> {
        let mut borrows = vec![vec![]; recipes.len()];
        let mut borrowed_inputs = vec![vec![]; recipes.len()];
        for (site, recipe) in recipes.iter().enumerate() {
            if !flow.reachable[site]
                || flow.skip_action[site]
                || boundaries.contains(&site)
                || !matches!(plan.blocks()[site].terminator(), Term::Scalar(_))
            {
                continue;
            }
            let Recipe::Aggregate(_) = recipe else {
                continue;
            };
            for &(destination, source) in &self.copies[site] {
                if self.definitions[site].contains(&source)
                    || self.post_uses[site].contains(&destination)
                    || !contains(&self.live_in[site], source)
                    || !self.live_after(site, self.slots[destination])
                    || borrowed_inputs[site].contains(&source)
                    || transfers[site]
                        .iter()
                        .any(|&(_, from)| from == self.slots[source])
                {
                    continue;
                }
                let mut proof = Borrow {
                    owner: source,
                    root: destination,
                    recipes,
                    layout,
                    plan,
                    owners: self,
                    boundaries,
                    seen: BTreeSet::new(),
                    copies: vec![],
                    reads: 0,
                };
                if !proof.visit(site, destination) || proof.reads == 0 {
                    continue;
                }
                let copies = proof.copies;
                borrows[site].push(self.slots[destination]);
                for (at, id) in copies {
                    borrowed_inputs[at].push(id);
                }
            }
        }
        borrows
    }
}

struct Borrow<'a> {
    owner: usize,
    root: usize,
    recipes: &'a [Recipe],
    layout: &'a Layout,
    plan: &'a Plan,
    owners: &'a OwnerFlow,
    boundaries: &'a BTreeSet<usize>,
    seen: BTreeSet<(usize, usize)>,
    copies: Vec<(usize, usize)>,
    reads: usize,
}
impl Borrow<'_> {
    fn visit(&mut self, first: usize, loan: usize) -> bool {
        // Trace the current value on every live edge. An incoming edge that
        // ends this loan already retires it, so has no later owner observer.
        self.owners.successors[first]
            .iter()
            .copied()
            .filter(|&next| contains(&self.owners.live_in[next], loan))
            .all(|next| self.visit_use(first, next, loan))
    }

    fn visit_use(&mut self, previous: usize, site: usize, loan: usize) -> bool {
        let o = self.owners;
        // A decreasing edge may enter another visit or a join with different
        // backing. Leave those cases to ordinary ownership.
        if site <= previous
            || self.seen.len() == MAX_BORROW_SITES
            || self.boundaries.contains(&site)
            || !matches!(self.plan.blocks()[site].terminator(), Term::Scalar(_))
            || !contains(&o.live_in[site], self.owner)
            || o.definitions[site].contains(&self.owner)
            || o.definitions[site].contains(&loan)
            || o.post_uses[site].contains(&loan)
        {
            return false;
        }
        if !self.seen.insert((site, loan)) {
            return true;
        }
        self.copies.push((site, loan));
        match &self.recipes[site] {
            Recipe::Read { source, output, .. } => {
                if o.reads[site].contains(&loan) {
                    if !matches!(self.layout.ty(source.get()).kind, Kind::Array { .. })
                        || !matches!(self.layout.ty(output.get()).kind, Kind::Scalar { .. })
                        || self.layout.slot(source.get()) + ARRAY_OWNER != o.slots[loan]
                    {
                        return false;
                    }
                    self.reads += 1;
                }
            }
            Recipe::Aggregate(_) => {
                for &(dst, src) in &o.copies[site] {
                    if src == self.owner {
                        return false;
                    }
                    if src == loan
                        && (o.post_uses[site].contains(&dst)
                            || (o.live_after(site, o.slots[dst])
                                && (dst == self.owner
                                    || dst == self.root
                                    || !self.visit(site, dst))))
                    {
                        return false;
                    }
                }
            }
            Recipe::Length { .. } | Recipe::StringEqual { .. } => {}
            Recipe::None
                if matches!(
                    self.plan.blocks()[site].action(),
                    Action::Scalar(Scalar::None | Scalar::Primitive { .. })
                ) => {}
            _ => return false,
        }
        self.visit(site, loan)
    }
}
