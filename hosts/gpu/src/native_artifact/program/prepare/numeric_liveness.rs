//! Pure liveness on one already selected graph. No plan, root-stop or predicate
//! context is cached here; the caller constructs the exact graph for each query.
use crate::native_artifact::program::packed_liveness::{
    MAX_DENSE_CELLS, dense_live_before_indexed,
};
use std::collections::{BTreeMap, BTreeSet, VecDeque};

#[path = "numeric_point_liveness.rs"]
mod point;

// Sparse words avoid storage proportional to the largest value ID or to the
// full block/value product. Each entry represents up to 64 independently live
// values, with no zero words retained.
#[derive(Clone, Default)]
struct Words(BTreeMap<u32, u64>);
impl Words {
    fn from_values(values: impl IntoIterator<Item = u32>) -> Self {
        let mut words = Self::default();
        for value in values {
            *words.0.entry(value / 64).or_default() |= 1u64 << (value % 64);
        }
        words
    }

    fn values(&self) -> BTreeSet<u32> {
        self.0
            .iter()
            .flat_map(|(&word, &bits)| {
                let mut remaining = bits;
                std::iter::from_fn(move || {
                    if remaining == 0 {
                        return None;
                    }
                    let bit = remaining.trailing_zeros();
                    remaining &= remaining - 1;
                    Some(word * 64 + bit)
                })
            })
            .collect()
    }
}

/// Nodes use query-local ordinals. No source-ID tree lookup or reindexing is
/// needed while building predecessors or solving the selected graph. Definition
/// lists are sorted/deduplicated by the caller's original ordered value loop.
pub(super) fn live_before_indexed(
    next: &[Vec<usize>],
    reads: &[BTreeSet<u32>],
    definitions: &[Vec<u32>],
    start: usize,
) -> BTreeSet<u32> {
    // On broad graphs with sparse reads, solve only the requested start point.
    // The caller still constructs and validates its complete exact query graph.
    // A bounded search falls back to the existing packed/sparse solvers.
    if next.len() >= 1024
        && reads.iter().map(BTreeSet::len).sum::<usize>() <= next.len()
        && let (Some(live), _) = point::solve(
            next,
            reads,
            definitions,
            start,
            next.len().saturating_mul(64).min(point::MAX_SEARCH_STEPS),
        )
    {
        return live;
    }
    if let Some(live) = dense_live_before_indexed(next, reads, definitions, start, MAX_DENSE_CELLS)
    {
        return live;
    }
    let (live, _, _) = solve_indexed(next, reads, definitions);
    live[start].values()
}

fn solve_indexed(
    next: &[Vec<usize>],
    reads: &[BTreeSet<u32>],
    definitions: &[Vec<u32>],
) -> (Vec<Words>, usize, usize) {
    let mut predecessors = vec![Vec::new(); next.len()];
    for (index, successors) in next.iter().enumerate() {
        for successor in successors {
            predecessors[*successor].push(index);
        }
    }
    let mut live = reads
        .iter()
        .map(|values| Words::from_values(values.iter().copied()))
        .collect::<Vec<_>>();
    let mut differences = live.clone();
    let kills = definitions
        .iter()
        .map(|values| Words::from_values(values.iter().copied()))
        .collect::<Vec<_>>();
    let mut queued = vec![true; next.len()];
    let mut pending = (0..next.len()).rev().collect::<VecDeque<_>>();
    let mut visits = 0usize;
    let mut propagated_facts = 0usize;
    while let Some(site) = pending.pop_front() {
        queued[site] = false;
        visits = visits.saturating_add(1);
        let changes = std::mem::take(&mut differences[site]);
        for bits in changes.0.values() {
            propagated_facts = propagated_facts.saturating_add(bits.count_ones() as usize);
        }
        for &predecessor in &predecessors[site] {
            let mut changed = false;
            for (&word, &bits) in &changes.0 {
                // Definitions kill facts arriving from successors, never a
                // read at the defining block itself. Seeded local reads are
                // therefore independent of this transfer, as in the old solver.
                let surviving = bits & !kills[predecessor].0.get(&word).copied().unwrap_or(0);
                if surviving == 0 {
                    continue;
                }
                let destination = live[predecessor].0.entry(word).or_default();
                let added = surviving & !*destination;
                if added != 0 {
                    *destination |= added;
                    *differences[predecessor].0.entry(word).or_default() |= added;
                    changed = true;
                }
            }
            if changed && !queued[predecessor] {
                queued[predecessor] = true;
                pending.push_back(predecessor);
            }
        }
    }
    (live, visits, propagated_facts)
}

// Preserve the source-ID adapters for the independent reference/property tests.
// Production constructs query-local ordinals directly in the original DFS order.
#[cfg(test)]
struct Liveness {
    sites: Vec<usize>,
    live: Vec<Words>,
}
#[cfg(test)]
impl Liveness {
    fn at(&self, site: usize) -> BTreeSet<u32> {
        self.live[self.sites.binary_search(&site).unwrap()].values()
    }
}
#[cfg(test)]
fn indexed_inputs(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (
    Vec<usize>,
    Vec<Vec<usize>>,
    Vec<BTreeSet<u32>>,
    Vec<Vec<u32>>,
) {
    let sites = next.keys().copied().collect::<Vec<_>>();
    let edges = next
        .values()
        .map(|edges| {
            edges
                .iter()
                .map(|site| sites.binary_search(site).unwrap())
                .collect()
        })
        .collect();
    let reads = sites.iter().map(|site| reads[site].clone()).collect();
    let definitions = sites
        .iter()
        .map(|site| {
            definitions
                .get(site)
                .map(|v| v.iter().copied().collect())
                .unwrap_or_default()
        })
        .collect();
    (sites, edges, reads, definitions)
}
#[cfg(test)]
fn live_before(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
    start: usize,
) -> BTreeSet<u32> {
    let (sites, next, reads, definitions) = indexed_inputs(next, reads, definitions);
    live_before_indexed(
        &next,
        &reads,
        &definitions,
        sites.binary_search(&start).unwrap(),
    )
}
#[cfg(test)]
fn dense_live_before(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
    start: usize,
    max_cells: usize,
) -> Option<BTreeSet<u32>> {
    let (sites, next, reads, definitions) = indexed_inputs(next, reads, definitions);
    dense_live_before_indexed(
        &next,
        &reads,
        &definitions,
        sites.binary_search(&start).unwrap(),
        max_cells,
    )
}
#[cfg(test)]
fn solve(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (Liveness, usize, usize) {
    let (sites, next, reads, definitions) = indexed_inputs(next, reads, definitions);
    let (live, visits, facts) = solve_indexed(&next, &reads, &definitions);
    (Liveness { sites, live }, visits, facts)
}

#[cfg(test)]
fn solve_liveness(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (BTreeMap<usize, BTreeSet<u32>>, usize, usize) {
    let (result, visits, facts) = solve(next, reads, definitions);
    let all = result
        .sites
        .iter()
        .map(|&site| (site, result.at(site)))
        .collect();
    (all, visits, facts)
}

#[cfg(test)]
#[path = "numeric_liveness_tests.rs"]
mod tests;
