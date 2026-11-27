//! Physical word liveness for a pre-SSA source region. No heap operations are
//! introduced at the boundary: words transport the original source state.
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Debug, Default)]
pub(super) struct Effects {
    pub reads: BTreeSet<usize>,
    /// Every successful traversal of this operation writes these words.
    pub writes: BTreeSet<usize>,
    /// Includes conditional clears; these cannot kill an incoming read.
    pub may_write: BTreeSet<usize>,
    /// Forward must-zero transfer, in the same successful operation order.
    zero_unknown: BTreeSet<usize>,
    zero_set: BTreeSet<usize>,
    zero_copies: std::collections::BTreeMap<usize, BTreeSet<usize>>,
}
impl Effects {
    pub fn read(&mut self, word: usize) {
        if !self.writes.contains(&word) {
            self.reads.insert(word);
        }
    }
    pub fn write(&mut self, word: usize) {
        self.writes.insert(word);
        self.may_write.insert(word);
        self.zero_unknown.insert(word);
        self.zero_set.remove(&word);
        self.zero_copies.remove(&word);
    }
    pub fn clear(&mut self, word: usize) {
        self.write(word);
        self.zero_unknown.remove(&word);
        self.zero_set.insert(word);
    }
    pub fn conditional_clear(&mut self, word: usize) {
        // Both possible results are zero when the incoming word is zero.
        self.may_write.insert(word);
    }
    /// Aggregate lanes read their source before overwriting any destination.
    /// All possible copy sources must be zero to prove a guarded lane zero.
    pub fn copy_lane(&mut self, destination: usize, source: usize) {
        self.reads.insert(source);
        self.zero_copies
            .entry(destination)
            .or_default()
            .insert(source);
    }
    fn touched(&self) -> impl Iterator<Item = usize> + '_ {
        self.reads.iter().chain(&self.may_write).copied()
    }
}

pub(super) struct Graph<'a> {
    pub reachable: &'a [bool],
    pub successors: &'a [Vec<usize>],
    pub actions: &'a [Effects],
    /// One settlement effect for each successor, in the same order.
    pub edges: &'a [Vec<Effects>],
}

/// Include guarded native routes in the same physical word proof. A source
/// header dispatches either to its original action or to a summarized bypass;
/// the bypass does not inherit actions it actually skips.
pub(super) struct Expanded {
    reachable: Vec<bool>,
    successors: Vec<Vec<usize>>,
    actions: Vec<Effects>,
    edges: Vec<Vec<Effects>>,
    source: Vec<usize>,
    original: Vec<Option<usize>>,
}
impl Expanded {
    pub fn new(
        reachable: Vec<bool>,
        successors: Vec<Vec<usize>>,
        actions: Vec<Effects>,
        edges: Vec<Vec<Effects>>,
    ) -> Self {
        let n = reachable.len();
        Self {
            reachable,
            successors,
            actions,
            edges,
            source: (0..n).collect(),
            original: vec![None; n],
        }
    }
    pub fn bypass_before(
        &mut self,
        header: usize,
        exit: usize,
        effect: Effects,
    ) -> Result<(), &'static str> {
        if header >= self.original.len()
            || exit >= self.original.len()
            || !self.reachable[header]
            || !self.reachable[exit]
        {
            return Err("invalid native bypass boundary");
        }
        if self.original[header].is_none() {
            let ordinary = self.actions.len();
            let action = std::mem::take(&mut self.actions[header]);
            let successors = std::mem::take(&mut self.successors[header]);
            let edges = std::mem::take(&mut self.edges[header]);
            self.actions.push(action);
            self.successors.push(successors);
            self.edges.push(edges);
            self.reachable.push(true);
            self.source.push(header);
            self.successors[header].push(ordinary);
            self.edges[header].push(Effects::default());
            self.original[header] = Some(ordinary);
        }
        let bypass = self.actions.len();
        self.actions.push(effect);
        self.successors.push(vec![exit]);
        self.edges.push(vec![Effects::default()]);
        self.reachable.push(true);
        self.source.push(header);
        self.successors[header].push(bypass);
        self.edges[header].push(Effects::default());
        Ok(())
    }
    pub fn unit_sites(&self, source: &[usize]) -> Vec<usize> {
        self.source
            .iter()
            .enumerate()
            .filter_map(|(site, original)| source.binary_search(original).is_ok().then_some(site))
            .collect()
    }
    pub fn graph(&self) -> Graph<'_> {
        Graph {
            reachable: &self.reachable,
            successors: &self.successors,
            actions: &self.actions,
            edges: &self.edges,
        }
    }
}

#[derive(Debug, Eq, PartialEq)]
pub(super) struct Boundary {
    pub inputs: Vec<usize>,
    pub outputs: Vec<usize>,
    pub local_words: BTreeSet<usize>,
}

struct Live {
    rows: Vec<Vec<u64>>,
    sites: Vec<usize>,
    words: Vec<usize>,
}
impl Live {
    fn at(&self, site: usize) -> BTreeSet<usize> {
        let i = self
            .sites
            .binary_search(&site)
            .expect("local live source site");
        self.words
            .iter()
            .enumerate()
            .filter_map(|(j, &word)| (self.rows[i][j / 64] & (1 << (j % 64)) != 0).then_some(word))
            .collect()
    }
}
impl Graph<'_> {
    /// Prove physical owner zeros on every path. Intersections at joins and
    /// optimistic loop initialization compute the greatest must fixed point;
    /// the actual function entry is always constrained by its initial state.
    pub fn zeros_at(
        &self,
        entry: usize,
        domain: &BTreeSet<usize>,
        initial: &BTreeSet<usize>,
        requested: &[usize],
    ) -> Result<Vec<BTreeSet<usize>>, &'static str> {
        if entry >= self.reachable.len()
            || !self.reachable[entry]
            || !initial.is_subset(domain)
            || requested
                .iter()
                .any(|&s| s >= self.reachable.len() || !self.reachable[s])
        {
            return Err("invalid must-zero boundary");
        }
        if domain.is_empty() {
            return Ok(vec![BTreeSet::new(); requested.len()]);
        }
        let words = domain.iter().copied().collect::<Vec<_>>();
        let ids = words
            .iter()
            .enumerate()
            .map(|(i, &w)| (w, i))
            .collect::<std::collections::BTreeMap<_, _>>();
        let width = words.len().div_ceil(64);
        let mut top = vec![u64::MAX; width];
        if words.len() % 64 != 0 {
            top[width - 1] = (1 << (words.len() % 64)) - 1;
        }
        let mut seed = vec![0; width];
        for word in initial {
            let i = ids[word];
            seed[i / 64] |= 1 << (i % 64);
        }
        let mut predecessors = vec![Vec::new(); self.reachable.len()];
        for (site, nexts) in self.successors.iter().enumerate() {
            if self.reachable[site] {
                for (edge, &next) in nexts.iter().enumerate() {
                    predecessors[next].push((site, edge));
                }
            }
        }
        let apply = |bits: &mut [u64], effect: &Effects| {
            let copied = effect
                .zero_copies
                .iter()
                .filter_map(|(destination, sources)| {
                    ids.get(destination).map(|&id| {
                        (
                            id,
                            sources.iter().all(|source| {
                                ids.get(source)
                                    .is_some_and(|&i| bits[i / 64] & (1 << (i % 64)) != 0)
                            }),
                        )
                    })
                })
                .collect::<Vec<_>>();
            for word in &effect.zero_unknown {
                if let Some(&i) = ids.get(word) {
                    bits[i / 64] &= !(1 << (i % 64));
                }
            }
            for (i, zero) in copied {
                if zero {
                    bits[i / 64] |= 1 << (i % 64);
                } else {
                    bits[i / 64] &= !(1 << (i % 64));
                }
            }
            for word in &effect.zero_set {
                if let Some(&i) = ids.get(word) {
                    bits[i / 64] |= 1 << (i % 64);
                }
            }
        };
        let incoming = |site: usize, rows: &[Vec<u64>]| {
            let mut bits = if site == entry {
                seed.clone()
            } else {
                top.clone()
            };
            for &(prior, edge) in &predecessors[site] {
                let mut predecessor = rows[prior].clone();
                apply(&mut predecessor, &self.edges[prior][edge]);
                for (out, value) in bits.iter_mut().zip(predecessor) {
                    *out &= value;
                }
            }
            bits
        };
        let mut rows = vec![top.clone(); self.reachable.len()];
        let mut queue = (0..rows.len())
            .filter(|&s| self.reachable[s])
            .collect::<VecDeque<_>>();
        let mut pending = self.reachable.to_vec();
        while let Some(site) = queue.pop_front() {
            pending[site] = false;
            let mut bits = incoming(site, &rows);
            apply(&mut bits, &self.actions[site]);
            if bits != rows[site] {
                rows[site] = bits;
                for &next in &self.successors[site] {
                    if !pending[next] {
                        pending[next] = true;
                        queue.push_back(next);
                    }
                }
            }
        }
        Ok(requested
            .iter()
            .map(|&site| {
                let bits = incoming(site, &rows);
                words
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &w)| (bits[i / 64] & (1 << (i % 64)) != 0).then_some(w))
                    .collect()
            })
            .collect())
    }

    fn touched(&self, sites: &[usize]) -> BTreeSet<usize> {
        sites
            .iter()
            .flat_map(|&site| {
                self.actions[site]
                    .touched()
                    .chain(self.edges[site].iter().flat_map(Effects::touched))
            })
            .collect()
    }

    fn liveness(
        &self,
        sites: &[usize],
        boundary: Option<(usize, &BTreeSet<usize>)>,
        demand: Option<&BTreeSet<usize>>,
    ) -> Live {
        let words = demand
            .cloned()
            .unwrap_or_else(|| self.touched(sites))
            .into_iter()
            .chain(
                boundary
                    .into_iter()
                    .flat_map(|(_, words)| words.iter().copied()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let ids = words
            .iter()
            .enumerate()
            .map(|(i, &w)| (w, i))
            .collect::<std::collections::BTreeMap<_, _>>();
        let width = words.len().div_ceil(64);
        let mut local = vec![usize::MAX; self.reachable.len()];
        for (i, &site) in sites.iter().enumerate() {
            local[site] = i;
        }
        let mut predecessors = vec![Vec::new(); sites.len()];
        for &site in sites {
            for &next in &self.successors[site] {
                if local[next] != usize::MAX {
                    predecessors[local[next]].push(local[site]);
                }
            }
        }
        let mut boundary_bits = vec![0; width];
        if let Some((_, values)) = boundary {
            for value in values {
                let i = ids[value];
                boundary_bits[i / 64] |= 1 << (i % 64);
            }
        }
        let apply = |bits: &mut [u64], effect: &Effects| {
            for word in &effect.writes {
                if let Some(&i) = ids.get(word) {
                    bits[i / 64] &= !(1 << (i % 64));
                }
            }
            for word in &effect.reads {
                if let Some(&i) = ids.get(word) {
                    bits[i / 64] |= 1 << (i % 64);
                }
            }
        };
        let mut rows = vec![vec![0; width]; sites.len()];
        let mut queue = (0..sites.len()).rev().collect::<VecDeque<_>>();
        let mut pending = vec![true; sites.len()];
        while let Some(i) = queue.pop_front() {
            pending[i] = false;
            let site = sites[i];
            let mut bits = vec![0; width];
            for (edge, &next) in self.edges[site].iter().zip(&self.successors[site]) {
                let mut incoming = if boundary.is_some_and(|(exit, _)| exit == next) {
                    boundary_bits.clone()
                } else if local[next] != usize::MAX {
                    rows[local[next]].clone()
                } else {
                    continue;
                };
                apply(&mut incoming, edge);
                for (out, value) in bits.iter_mut().zip(incoming) {
                    *out |= value;
                }
            }
            apply(&mut bits, &self.actions[site]);
            if rows[i] != bits {
                rows[i] = bits;
                for &prior in &predecessors[i] {
                    if !pending[prior] {
                        pending[prior] = true;
                        queue.push_back(prior);
                    }
                }
            }
        }
        Live {
            rows,
            sites: sites.to_vec(),
            words,
        }
    }

    pub fn boundary(
        &self,
        sites: &[usize],
        entry: usize,
        exit: usize,
        invariant_words: &BTreeSet<usize>,
    ) -> Result<Boundary, &'static str> {
        let n = self.reachable.len();
        if self.successors.len() != n
            || self.actions.len() != n
            || self.edges.len() != n
            || entry >= n
            || exit >= n
            || !self.reachable[entry]
            || !self.reachable[exit]
            || sites.binary_search(&entry).is_err()
            || sites.binary_search(&exit).is_ok()
            || sites.windows(2).any(|s| s[0] >= s[1])
            || sites.iter().any(|&s| s >= n || !self.reachable[s])
            || self.successors.iter().enumerate().any(|(s, ns)| {
                ns.len() != self.edges[s].len()
                    || ns
                        .iter()
                        .any(|&next| next >= n || self.reachable[s] && !self.reachable[next])
            })
        {
            return Err("malformed physical source graph");
        }
        let all = (0..n).filter(|&s| self.reachable[s]).collect::<Vec<_>>();
        let written = sites
            .iter()
            .flat_map(|&s| {
                self.actions[s]
                    .may_write
                    .iter()
                    .chain(self.edges[s].iter().flat_map(|e| &e.may_write))
            })
            .copied()
            .collect::<BTreeSet<_>>();
        if all.iter().any(|&s| {
            self.actions[s]
                .may_write
                .iter()
                .chain(self.edges[s].iter().flat_map(|e| &e.may_write))
                .any(|w| invariant_words.contains(w))
        }) {
            return Err("written physical word cannot be an invariant");
        }
        // Only region-written words can become live results. Unrelated global
        // homes need no rows in the continuation liveness proof.
        let global = self.liveness(&all, None, Some(&written));
        let outputs = global
            .at(exit)
            .intersection(&written)
            .copied()
            .collect::<BTreeSet<_>>();
        let local = self.liveness(sites, Some((exit, &outputs)), None);
        let inputs = local
            .at(entry)
            .difference(invariant_words)
            .copied()
            .collect();
        let mut local_words = self.touched(sites);
        local_words.extend(&outputs);
        Ok(Boundary {
            inputs,
            outputs: outputs.into_iter().collect(),
            local_words,
        })
    }

    pub fn root_words(&self, sites: &[usize], boundary: &Boundary) -> BTreeSet<usize> {
        self.touched(sites)
            .into_iter()
            .chain(boundary.inputs.iter().chain(&boundary.outputs).copied())
            .collect()
    }
}

#[cfg(test)]
#[path = "word_boundary_tests.rs"]
mod tests;
