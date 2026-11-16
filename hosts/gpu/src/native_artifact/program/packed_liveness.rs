//! Bounded packed liveness for one already selected immutable query graph.
//! Only topology and read/write sets enter; no plan, visit or predicate proof
//! is stored. Definitions are sorted by value; local reads survive local kills.
use std::collections::{BTreeSet, VecDeque};

// Two flat word planes use at most 256 MiB. Callers keep their existing
// fallback for larger products. Value ordinals are local to this query,
// never raw source IDs.
pub(super) const MAX_DENSE_CELLS: usize = 16 * 1024 * 1024;

pub(super) fn dense_live_before_indexed<V: Copy + Ord>(
    next: &[Vec<usize>],
    reads: &[BTreeSet<V>],
    definitions: &[Vec<V>],
    start: usize,
    max_cells: usize,
) -> Option<BTreeSet<V>> {
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
