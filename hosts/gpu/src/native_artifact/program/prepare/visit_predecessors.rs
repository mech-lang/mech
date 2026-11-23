//! Reverse control edges for one immutable prepared-program owner. This stores
//! topology only; every scope query still checks its current membership/header.
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug)]
pub struct VisitPredecessors {
    offsets: Vec<usize>,
    incoming: Vec<usize>,
    // Keep the full scan's behavior even for out-of-range descriptive targets.
    // Valid prepared programs have no such edges.
    outside: BTreeMap<usize, Vec<usize>>,
}
impl VisitPredecessors {
    pub fn new(successors: &[[Option<u32>; 2]]) -> Self {
        let count = successors.len();
        let mut offsets = vec![0; count + 1];
        let mut outside = BTreeMap::<usize, Vec<usize>>::new();
        for (source, edges) in successors.iter().enumerate() {
            for &target in edges.iter().flatten() {
                let target = target as usize;
                if target < count {
                    offsets[target + 1] += 1;
                } else {
                    outside.entry(target).or_default().push(source);
                }
            }
        }
        for site in 0..count {
            offsets[site + 1] += offsets[site];
        }
        let mut incoming = vec![0; offsets[count]];
        let mut positions = offsets[..count].to_vec();
        for (source, edges) in successors.iter().enumerate() {
            for &target in edges.iter().flatten() {
                let target = target as usize;
                if target < count {
                    incoming[positions[target]] = source;
                    positions[target] += 1;
                }
            }
        }
        Self {
            offsets,
            incoming,
            outside,
        }
    }

    fn sources(&self, target: usize) -> &[usize] {
        if target < self.offsets.len() - 1 {
            &self.incoming[self.offsets[target]..self.offsets[target + 1]]
        } else {
            self.outside.get(&target).map(Vec::as_slice).unwrap_or(&[])
        }
    }

    /// Recheck the current set/header, retaining no scope-validity answer.
    /// These targets seed order-independent omitted-predecessor pruning.
    pub fn outside_targets<'a>(
        &'a self,
        sites: &'a BTreeSet<usize>,
        header: usize,
    ) -> impl Iterator<Item = usize> + 'a {
        sites.iter().copied().filter(move |&target| {
            target != header
                && self
                    .sources(target)
                    .iter()
                    .any(|source| !sites.contains(source))
        })
    }

    pub fn first_outside_entry(&self, sites: &BTreeSet<usize>, header: usize) -> Option<usize> {
        let mut first = None;
        for &target in sites {
            if target == header {
                continue;
            }
            for &source in self.sources(target) {
                if !sites.contains(&source) {
                    // Preserve the original source-ordered scan's first refusal,
                    // including diagnostics, rather than target iteration order.
                    first = Some(first.map_or(source, |old: usize| old.min(source)));
                    if source == 0 {
                        return first;
                    }
                    // Incoming sources are ordered; later entries in this row
                    // cannot improve this row's first outside predecessor.
                    break;
                }
            }
        }
        first
    }
}

#[cfg(test)]
#[path = "visit_predecessors_tests.rs"]
mod tests;
