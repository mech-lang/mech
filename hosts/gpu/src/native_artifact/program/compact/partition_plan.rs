//! Source ownership and local coordinates before native SSA construction.
//! These summaries grant no runtime permission or authority to skip checks.
use std::collections::{BTreeSet, VecDeque};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum End {
    Continue,
    Failure,
    Complete,
}
pub(super) struct Graph<'a> {
    pub reachable: &'a [bool],
    pub successors: &'a [Vec<usize>],
    /// Reads preceding the action; same-site results read by its terminator are excluded.
    pub uses: &'a [Vec<u32>],
    pub defs: &'a [Vec<u32>],
    pub ends: &'a [End],
    pub words: &'a [usize],
    /// Initial constants that have no definition anywhere in the source plan.
    pub invariant_constants: &'a BTreeSet<u32>,
    /// Every reference to one hidden cursor/builder resource, from typed actions.
    pub scopes: &'a [Vec<usize>],
    pub inputs: &'a [u32],
    pub outputs: &'a [u32],
}
#[derive(Debug, Eq, PartialEq)]
pub(super) enum Error {
    Malformed,
    InvalidBoundary(usize, usize),
    NoNormalExit(usize),
    OutsideEntry(usize, usize),
    CompletionInside(usize),
    SplitScope(usize),
    Overlap(usize),
    WordCountOverflow,
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "early source partition: {self:?}")
    }
}
#[derive(Debug)]
pub(super) struct Region {
    pub entry: usize,
    pub exit: usize,
    pub sites: Vec<usize>,
    pub inputs: Vec<u32>,
    pub outputs: Vec<u32>,
    pub constants: Vec<u32>,
    /// Sorted global value IDs; their positions are dense unit-local IDs.
    pub local_values: Vec<u32>,
    pub local_words: usize,
    pub scopes: Vec<usize>,
    pub failures: Vec<usize>,
}
#[derive(Debug)]
pub(super) struct Plan {
    /// Zero is the root. Region k owns each original operation at k+1.
    /// Root calls are synthetic operations; they do not duplicate source nodes.
    pub owners: Vec<Option<usize>>,
    pub root_sites: Vec<usize>,
    pub root_values: Vec<u32>,
    pub root_words: usize,
    pub regions: Vec<Region>,
}
struct Live {
    sites: Vec<usize>,
    values: Vec<u32>,
    rows: Vec<Vec<u64>>,
}
impl Live {
    fn at(&self, site: usize) -> BTreeSet<u32> {
        let local = self.sites.binary_search(&site).unwrap();
        self.values
            .iter()
            .enumerate()
            .filter_map(|(i, &value)| {
                (self.rows[local][i / 64] & (1 << (i % 64)) != 0).then_some(value)
            })
            .collect()
    }
}
impl Graph<'_> {
    fn validate(&self) -> Result<(), Error> {
        let n = self.reachable.len();
        if [
            self.successors.len(),
            self.uses.len(),
            self.defs.len(),
            self.ends.len(),
        ]
        .iter()
        .any(|&len| len != n)
            || self.successors.iter().enumerate().any(|(s, ns)| {
                ns.iter()
                    .any(|&next| next >= n || (self.reachable[s] && !self.reachable[next]))
            })
            || self
                .uses
                .iter()
                .chain(self.defs)
                .flatten()
                .chain(self.inputs)
                .chain(self.outputs)
                .chain(self.invariant_constants)
                .any(|&value| value as usize >= self.words.len())
            || self
                .defs
                .iter()
                .flatten()
                .any(|v| self.invariant_constants.contains(v))
            || self
                .ends
                .iter()
                .enumerate()
                .any(|(s, end)| *end != End::Continue && !self.successors[s].is_empty())
            || self.scopes.iter().flatten().any(|&site| site >= n)
        {
            return Err(Error::Malformed);
        }
        Ok(())
    }
    fn predecessors(&self) -> Vec<Vec<usize>> {
        let mut result = vec![Vec::new(); self.reachable.len()];
        for (site, edges) in self.successors.iter().enumerate() {
            if self.reachable[site] {
                for &next in edges {
                    result[next].push(site);
                }
            }
        }
        result
    }
    fn touched(&self, sites: &[usize]) -> BTreeSet<u32> {
        sites
            .iter()
            .flat_map(|&s| self.uses[s].iter().chain(&self.defs[s]))
            .copied()
            .collect()
    }
    fn word_count(&self, values: &[u32]) -> Result<usize, Error> {
        values.iter().try_fold(0usize, |sum, &value| {
            sum.checked_add(self.words[value as usize])
                .ok_or(Error::WordCountOverflow)
        })
    }
    // Only the unit's rows and value IDs receive liveness storage. Canonical
    // IDs remain available for origin/effect checks through explicit maps.
    fn liveness(
        &self,
        sites: &[usize],
        predecessors: &[Vec<usize>],
        boundary: Option<(usize, &BTreeSet<u32>)>,
    ) -> Live {
        let values = self
            .touched(sites)
            .into_iter()
            .chain(
                boundary
                    .into_iter()
                    .flat_map(|(_, values)| values.iter().copied()),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let mut site_map = vec![usize::MAX; self.reachable.len()];
        for (local, &global) in sites.iter().enumerate() {
            site_map[global] = local;
        }
        let mut value_map = vec![usize::MAX; self.words.len()];
        for (local, &global) in values.iter().enumerate() {
            value_map[global as usize] = local;
        }
        let width = values.len().div_ceil(64);
        let mut rows = vec![vec![0u64; width]; sites.len()];
        let mut boundary_row = vec![0u64; width];
        if let Some((_, values)) = boundary {
            for &v in values {
                let i = value_map[v as usize];
                boundary_row[i / 64] |= 1 << (i % 64);
            }
        }
        let mut queue = (0..sites.len()).rev().collect::<VecDeque<_>>();
        let mut pending = vec![true; sites.len()];
        while let Some(local) = queue.pop_front() {
            pending[local] = false;
            let site = sites[local];
            let mut bits = vec![0u64; width];
            for &next in &self.successors[site] {
                let incoming = if boundary.is_some_and(|(exit, _)| exit == next) {
                    &boundary_row
                } else if site_map[next] != usize::MAX {
                    &rows[site_map[next]]
                } else {
                    continue;
                };
                for (out, value) in bits.iter_mut().zip(incoming) {
                    *out |= value;
                }
            }
            for &v in &self.defs[site] {
                let i = value_map[v as usize];
                bits[i / 64] &= !(1 << (i % 64));
            }
            for &v in &self.uses[site] {
                let i = value_map[v as usize];
                bits[i / 64] |= 1 << (i % 64);
            }
            if bits != rows[local] {
                rows[local] = bits;
                for &prior in &predecessors[site] {
                    let i = site_map[prior];
                    if i != usize::MAX && !pending[i] {
                        pending[i] = true;
                        queue.push_back(i);
                    }
                }
            }
        }
        Live {
            sites: sites.to_vec(),
            values,
            rows,
        }
    }
    pub fn partition(&self, windows: &[(usize, usize)]) -> Result<Plan, Error> {
        self.validate()?;
        let n = self.reachable.len();
        let predecessors = self.predecessors();
        let all_sites = (0..n).filter(|&s| self.reachable[s]).collect::<Vec<_>>();
        let root_live = self.liveness(&all_sites, &predecessors, None);
        let mut owners = self
            .reachable
            .iter()
            .map(|&active| active.then_some(0))
            .collect::<Vec<_>>();
        let mut regions = Vec::new();
        for &(entry, exit) in windows {
            if entry >= n
                || exit >= n
                || entry == exit
                || !self.reachable[entry]
                || !self.reachable[exit]
            {
                return Err(Error::InvalidBoundary(entry, exit));
            }
            let mut members = vec![false; n];
            let mut todo = vec![entry];
            let mut reaches_exit = false;
            while let Some(site) = todo.pop() {
                if site == exit {
                    reaches_exit = true;
                    continue;
                }
                if members[site] {
                    continue;
                }
                if owners[site] != Some(0) {
                    return Err(Error::Overlap(site));
                }
                members[site] = true;
                if self.ends[site] == End::Complete {
                    return Err(Error::CompletionInside(site));
                }
                if self.ends[site] == End::Continue && self.successors[site].is_empty() {
                    return Err(Error::Malformed);
                }
                todo.extend(&self.successors[site]);
            }
            if !reaches_exit {
                return Err(Error::NoNormalExit(entry));
            }
            let sites = (0..n).filter(|&s| members[s]).collect::<Vec<_>>();
            for &site in &sites {
                for &prior in &predecessors[site] {
                    if site != entry && !members[prior] {
                        return Err(Error::OutsideEntry(site, prior));
                    }
                }
            }
            let mut scopes = Vec::new();
            for (id, refs) in self.scopes.iter().enumerate() {
                if refs.iter().any(|&s| members[s]) {
                    if refs.iter().any(|&s| self.reachable[s] && !members[s]) {
                        return Err(Error::SplitScope(id));
                    }
                    scopes.push(id);
                }
            }
            let definitions = sites
                .iter()
                .flat_map(|&s| &self.defs[s])
                .copied()
                .collect::<BTreeSet<_>>();
            let outputs = root_live
                .at(exit)
                .intersection(&definitions)
                .copied()
                .collect::<BTreeSet<_>>();
            let local_live = self.liveness(&sites, &predecessors, Some((exit, &outputs)));
            let inputs = local_live
                .at(entry)
                .difference(self.invariant_constants)
                .copied()
                .collect();
            let touched = self.touched(&sites);
            let constants = touched
                .intersection(self.invariant_constants)
                .copied()
                .collect();
            let local_values = touched.into_iter().collect::<Vec<_>>();
            let local_words = self.word_count(&local_values)?;
            let failures = sites
                .iter()
                .copied()
                .filter(|&s| self.ends[s] == End::Failure)
                .collect();
            for &s in &sites {
                owners[s] = Some(regions.len() + 1);
            }
            regions.push(Region {
                entry,
                exit,
                sites,
                inputs,
                outputs: outputs.into_iter().collect(),
                constants,
                local_values,
                local_words,
                scopes,
                failures,
            });
        }
        let root_sites = all_sites
            .into_iter()
            .filter(|&s| owners[s] == Some(0))
            .collect::<Vec<_>>();
        let root_values = self
            .touched(&root_sites)
            .into_iter()
            .chain(self.inputs.iter().copied())
            .chain(self.outputs.iter().copied())
            .chain(
                regions
                    .iter()
                    .flat_map(|r| r.inputs.iter().chain(&r.outputs))
                    .copied(),
            )
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let root_words = self.word_count(&root_values)?;
        Ok(Plan {
            owners,
            root_sites,
            root_values,
            root_words,
            regions,
        })
    }
}

#[cfg(test)]
#[path = "partition_plan_tests.rs"]
mod tests;
