//! Exact liveness at one start node, bounded by an explicit search-work budget.
//! Reads occur before local definitions; a successor fact is killed by a local
//! definition. Each value's reverse search stops as soon as it reaches start.
use std::collections::BTreeSet;

pub(super) const MAX_SEARCH_STEPS: usize = 16 * 1024 * 1024;

pub(super) fn solve(
    next: &[Vec<usize>],
    reads: &[BTreeSet<u32>],
    definitions: &[Vec<u32>],
    start: usize,
    max_steps: usize,
) -> (Option<BTreeSet<u32>>, usize) {
    let mut uses = reads
        .iter()
        .enumerate()
        .flat_map(|(site, values)| values.iter().map(move |&value| (value, site)))
        .collect::<Vec<_>>();
    if uses.is_empty() {
        return (Some(BTreeSet::new()), 0);
    }
    if max_steps == 0 {
        return (None, 0);
    }
    assert_eq!(next.len(), reads.len());
    assert_eq!(next.len(), definitions.len());
    assert!(start < next.len());
    uses.sort_unstable();

    let mut offsets = vec![0usize; next.len() + 1];
    for successors in next {
        for &successor in successors {
            offsets[successor + 1] += 1;
        }
    }
    for index in 1..offsets.len() {
        offsets[index] += offsets[index - 1];
    }
    let mut parents = vec![0usize; offsets[next.len()]];
    let mut cursor = offsets[..next.len()].to_vec();
    for (site, successors) in next.iter().enumerate() {
        for &successor in successors {
            parents[cursor[successor]] = site;
            cursor[successor] += 1;
        }
    }
    drop(cursor);
    let mut visited = vec![0usize; next.len()];
    let mut generation = 0;
    let mut pending = Vec::new();
    let mut live = BTreeSet::new();
    let mut steps = 0;
    let mut begin = 0;
    while begin < uses.len() {
        let value = uses[begin].0;
        let mut end = begin + 1;
        while end < uses.len() && uses[end].0 == value {
            end += 1;
        }
        generation += 1;
        let mut reaches_start = false;
        'seeds: for &(_, seed) in &uses[begin..end] {
            if visited[seed] == generation {
                continue;
            }
            visited[seed] = generation;
            pending.push(seed);
            while let Some(site) = pending.pop() {
                if steps == max_steps {
                    return (None, steps);
                }
                steps += 1;
                if site == start {
                    reaches_start = true;
                    pending.clear();
                    break 'seeds;
                }
                for &parent in parents[offsets[site]..offsets[site + 1]].iter().rev() {
                    if steps == max_steps {
                        return (None, steps);
                    }
                    steps += 1;
                    if visited[parent] != generation
                        && definitions[parent].binary_search(&value).is_err()
                    {
                        visited[parent] = generation;
                        pending.push(parent);
                    }
                }
            }
        }
        if reaches_start {
            live.insert(value);
        }
        begin = end;
    }
    (Some(live), steps)
}

#[cfg(test)]
#[path = "numeric_point_liveness_tests.rs"]
mod tests;
