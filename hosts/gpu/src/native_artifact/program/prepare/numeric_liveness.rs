//! Pure liveness on one already selected graph. No plan, root-stop or predicate
//! context is cached here; the caller constructs the exact graph for each query.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

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
    if let Some(live) = dense_live_before_indexed(next, reads, definitions, start, MAX_DENSE_CELLS)
    {
        return live;
    }
    let (live, _, _) = solve_indexed(next, reads, definitions);
    live[start].values()
}

// Two flat word planes use at most 256 MiB. Larger products retain the sparse
// solver below. Value ordinals are local to this query, never raw source IDs.
const MAX_DENSE_CELLS: usize = 16 * 1024 * 1024;

fn dense_live_before_indexed(
    next: &[Vec<usize>],
    reads: &[BTreeSet<u32>],
    definitions: &[Vec<u32>],
    start: usize,
    max_cells: usize,
) -> Option<BTreeSet<u32>> {
    let mut values = reads
        .iter()
        .flat_map(|values| values.iter().copied())
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    let width = values.len().div_ceil(64);
    let cells = next.len().checked_mul(width)?;
    if cells > max_cells {
        return None;
    }
    if width == 0 {
        return Some(BTreeSet::new());
    }
    let mut parents = vec![Vec::new(); next.len()];
    for (index, successors) in next.iter().enumerate() {
        for successor in successors {
            parents[*successor].push(index);
        }
    }
    let mut live = vec![0_u64; cells];
    let mut kills = vec![Vec::<(usize, u64)>::new(); next.len()];
    for index in 0..next.len() {
        for value in &reads[index] {
            let id = values.binary_search(value).unwrap();
            live[index * width + id / 64] |= 1_u64 << (id % 64);
        }
        {
            let definitions = &definitions[index];
            for value in definitions {
                // A value never read in this selected graph cannot be live.
                let Ok(id) = values.binary_search(value) else {
                    continue;
                };
                let word = id / 64;
                let bit = 1_u64 << (id % 64);
                if let Some((last, bits)) = kills[index].last_mut()
                    && *last == word
                {
                    *bits |= bit;
                } else {
                    kills[index].push((word, bit));
                }
            }
        }
    }
    let mut changes = live.clone();
    let mut queued = changes
        .chunks_exact(width)
        .map(|row| row.iter().any(|&bits| bits != 0))
        .collect::<Vec<_>>();
    let mut pending = (0..next.len())
        .rev()
        .filter(|&site| queued[site])
        .collect::<VecDeque<_>>();
    while let Some(site) = pending.pop_front() {
        queued[site] = false;
        for word in 0..width {
            let added = core::mem::take(&mut changes[site * width + word]);
            if added == 0 {
                continue;
            }
            for &parent in &parents[site] {
                let kill = kills[parent]
                    .binary_search_by_key(&word, |&(word, _)| word)
                    .map(|at| kills[parent][at].1)
                    .unwrap_or(0);
                let at = parent * width + word;
                let added = added & !kill & !live[at];
                if added != 0 {
                    live[at] |= added;
                    changes[at] |= added;
                    if !core::mem::replace(&mut queued[parent], true) {
                        pending.push_back(parent);
                    }
                }
            }
        }
    }
    let offset = start * width;
    Some(
        values
            .iter()
            .enumerate()
            .filter_map(|(id, &value)| {
                (live[offset + id / 64] & (1_u64 << (id % 64)) != 0).then_some(value)
            })
            .collect(),
    )
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
