//! Pure liveness on one already selected graph. No plan, root-stop or predicate
//! context is cached here; the caller constructs the exact graph for each query.
use std::collections::{BTreeMap, BTreeSet, VecDeque};

// Sparse words avoid storage proportional to the largest value ID or to the
// full block/value product. Each entry represents up to 64 independently live
// values, with no zero words retained.
#[derive(Clone, Default)]
struct Words(BTreeMap<u32, u64>);
impl Words {
    fn from_values(values: &BTreeSet<u32>) -> Self {
        let mut words = Self::default();
        for &value in values {
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

struct Liveness {
    sites: Vec<usize>,
    live: Vec<Words>,
}
impl Liveness {
    fn at(&self, site: usize) -> BTreeSet<u32> {
        self.live[self.sites.binary_search(&site).unwrap()].values()
    }
}

pub(super) fn live_before(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
    start: usize,
) -> BTreeSet<u32> {
    if let Some(live) = dense_live_before(next, reads, definitions, start, MAX_DENSE_CELLS) {
        return live;
    }
    let (live, _, _) = solve(next, reads, definitions);
    // Only the requested entry escapes the query. Do not expand the packed
    // facts at every other reachable block into separately allocated sets.
    live.at(start)
}

// Two flat word planes use at most 256 MiB. Larger products retain the sparse
// solver below. Value ordinals are local to this query, never raw source IDs.
const MAX_DENSE_CELLS: usize = 16 * 1024 * 1024;

fn dense_live_before(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
    start: usize,
    max_cells: usize,
) -> Option<BTreeSet<u32>> {
    let sites = next.keys().copied().collect::<Vec<_>>();
    let mut values = sites
        .iter()
        .flat_map(|site| reads[site].iter().copied())
        .collect::<Vec<_>>();
    values.sort_unstable();
    values.dedup();
    let width = values.len().div_ceil(64);
    let cells = sites.len().checked_mul(width)?;
    if cells > max_cells {
        return None;
    }
    if width == 0 {
        return Some(BTreeSet::new());
    }
    let mut parents = vec![Vec::new(); sites.len()];
    for (index, successors) in next.values().enumerate() {
        for successor in successors {
            parents[sites.binary_search(successor).unwrap()].push(index);
        }
    }
    let mut live = vec![0_u64; cells];
    let mut kills = vec![Vec::<(usize, u64)>::new(); sites.len()];
    for (index, site) in sites.iter().enumerate() {
        for value in &reads[site] {
            let id = values.binary_search(value).unwrap();
            live[index * width + id / 64] |= 1_u64 << (id % 64);
        }
        if let Some(definitions) = definitions.get(site) {
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
    let mut pending = (0..sites.len())
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
    let offset = sites.binary_search(&start).unwrap() * width;
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

fn solve(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (Liveness, usize, usize) {
    let sites = next.keys().copied().collect::<Vec<_>>();
    let mut predecessors = vec![Vec::new(); sites.len()];
    for (index, successors) in next.values().enumerate() {
        for successor in successors {
            predecessors[sites.binary_search(successor).unwrap()].push(index);
        }
    }
    let mut live = sites
        .iter()
        .map(|site| Words::from_values(&reads[site]))
        .collect::<Vec<_>>();
    let mut differences = live.clone();
    let kills = sites
        .iter()
        .map(|site| {
            definitions
                .get(site)
                .map(Words::from_values)
                .unwrap_or_default()
        })
        .collect::<Vec<_>>();
    let mut queued = vec![true; sites.len()];
    let mut pending = (0..sites.len()).rev().collect::<VecDeque<_>>();
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
    (Liveness { sites, live }, visits, propagated_facts)
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
