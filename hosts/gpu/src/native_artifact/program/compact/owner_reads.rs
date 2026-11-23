//! Exact unguarded read reachability for each old owner. This is only a query
//! filter: current predicates and heap exclusivity still decide permission.
use std::collections::VecDeque;

pub(super) fn derive(
    successors: &[Vec<usize>],
    definitions: &[Vec<usize>],
    reads: &[Vec<usize>],
    post_uses: &[Vec<usize>],
    width: usize,
) -> Vec<Vec<u64>> {
    let mut live = vec![vec![0_u64; width]; successors.len()];
    let mut parents = vec![Vec::new(); successors.len()];
    for (site, next) in successors.iter().enumerate() {
        for &next in next {
            parents[next].push(site);
        }
        // An action/post-use read precedes expiration of the old owner.
        for &id in reads[site].iter().chain(&post_uses[site]) {
            live[site][id / 64] |= 1_u64 << (id % 64);
        }
    }
    let mut changes = live.clone();
    let mut queued = changes
        .iter()
        .map(|row| row.iter().any(|&word| word != 0))
        .collect::<Vec<_>>();
    let mut pending = (0..successors.len())
        .rev()
        .filter(|&site| queued[site])
        .collect::<VecDeque<_>>();
    while let Some(site) = pending.pop_front() {
        queued[site] = false;
        for word in 0..width {
            let added = core::mem::take(&mut changes[site][word]);
            if added == 0 {
                continue;
            }
            for &parent in &parents[site] {
                let mut surviving = added;
                for &id in &definitions[parent] {
                    if id / 64 == word {
                        surviving &= !(1_u64 << (id % 64));
                    }
                }
                let added = surviving & !live[parent][word];
                if added != 0 {
                    live[parent][word] |= added;
                    changes[parent][word] |= added;
                    if !core::mem::replace(&mut queued[parent], true) {
                        pending.push_back(parent);
                    }
                }
            }
        }
    }
    live
}

#[cfg(test)]
#[path = "owner_reads_tests.rs"]
mod tests;
