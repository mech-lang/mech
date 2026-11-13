//! Bounded reachability memo for one immutable CFG discovery pass. A new graph
//! owns a fresh memo; no selected route is applied until discovery is complete.
use std::{cell::RefCell, sync::Arc};

const CACHE_ROWS: usize = 32;
type Entry = ((usize, usize), Arc<Vec<bool>>);

pub(super) struct Reachability {
    successors: Vec<Vec<usize>>,
    cache: RefCell<Vec<Entry>>,
}
impl Reachability {
    pub fn new(successors: Vec<Vec<usize>>) -> Self {
        Self {
            successors,
            cache: RefCell::new(Vec::new()),
        }
    }
    pub fn reachable(&self, start: usize, stop: usize) -> Arc<Vec<bool>> {
        let key = (start, stop);
        if let Some((_, found)) = self.cache.borrow().iter().find(|(query, _)| *query == key) {
            return found.clone();
        }
        let mut seen = vec![false; self.successors.len()];
        let mut pending = vec![start];
        while let Some(site) = pending.pop() {
            if site == stop || core::mem::replace(&mut seen[site], true) {
                continue;
            }
            pending.extend(&self.successors[site]);
        }
        let seen = Arc::new(seen);
        let mut cache = self.cache.borrow_mut();
        if cache.len() == CACHE_ROWS {
            cache.remove(0);
        }
        cache.push((key, seen.clone()));
        seen
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn reference(graph: &[Vec<usize>], start: usize, stop: usize) -> Vec<bool> {
        let mut seen = BTreeSet::new();
        let mut todo = vec![start];
        while let Some(site) = todo.pop() {
            if site != stop && seen.insert(site) {
                todo.extend(&graph[site]);
            }
        }
        (0..graph.len()).map(|site| seen.contains(&site)).collect()
    }

    #[test]
    fn memo_reuses_exact_context_and_evicts_without_changing_old_rows() {
        let graph = (0..16)
            .map(|site| vec![(site + 1) % 16, (site + 3) % 16])
            .collect::<Vec<_>>();
        let memo = Reachability::new(graph.clone());
        let first = memo.reachable(0, 8);
        assert!(Arc::ptr_eq(&first, &memo.reachable(0, 8)));
        for start in 0..16 {
            for stop in 0..16 {
                assert_eq!(*memo.reachable(start, stop), reference(&graph, start, stop));
            }
        }
        assert_eq!(memo.cache.borrow().len(), CACHE_ROWS);
        assert_eq!(*first, reference(&graph, 0, 8));
        let different = Reachability::new(vec![vec![]; 16]);
        assert_ne!(*different.reachable(0, 8), *first);
    }

    #[test]
    fn every_short_graph_start_and_stop_matches_independent_traversal() {
        for bits in 0..512 {
            let graph = (0..3)
                .map(|site| {
                    (0..3)
                        .filter(|&next| bits & (1 << (site * 3 + next)) != 0)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let memo = Reachability::new(graph.clone());
            for start in 0..3 {
                for stop in 0..3 {
                    assert_eq!(*memo.reachable(start, stop), reference(&graph, start, stop));
                }
            }
        }
    }
}
