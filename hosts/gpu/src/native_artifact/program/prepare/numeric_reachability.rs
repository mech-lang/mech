//! Exact edge-avoidance reachability for one immutable forest traversal.
//! The root, latch stop and graph belong to this owner; only the avoided edge
//! varies. No predicate truth or execution permission is cached.
use std::collections::BTreeMap;

type Edge = Option<(usize, usize)>;
const MAX_ROW_BYTES: usize = 32 * 1024 * 1024;
const MAX_ROWS: usize = 1024;

pub(super) struct Reachability {
    successors: Vec<Vec<usize>>,
    root: usize,
    stop: usize,
    rows: BTreeMap<Edge, Box<[bool]>>,
    row_bytes: usize,
    byte_limit: usize,
}

impl Reachability {
    pub fn new(successors: Vec<Vec<usize>>, root: usize, stop: usize) -> Self {
        Self::with_limit(successors, root, stop, MAX_ROW_BYTES)
    }

    fn with_limit(
        successors: Vec<Vec<usize>>,
        root: usize,
        stop: usize,
        byte_limit: usize,
    ) -> Self {
        Self {
            successors,
            root,
            stop,
            rows: BTreeMap::new(),
            row_bytes: 0,
            byte_limit,
        }
    }

    pub fn reaches(&mut self, target: usize, avoid: Edge) -> bool {
        if target == self.root {
            return true;
        }
        if target >= self.successors.len() {
            return false;
        }
        if let Some(row) = self.rows.get(&avoid) {
            return row[target];
        }
        if self.rows.len() < MAX_ROWS
            && let Some(bytes) = self
                .row_bytes
                .checked_add(self.successors.len())
                .filter(|&bytes| bytes <= self.byte_limit)
        {
            let (_, row) = self.walk(None, avoid);
            let found = row[target];
            self.rows.insert(avoid, row);
            self.row_bytes = bytes;
            return found;
        }
        // Exhausting the cache budget changes storage only. The exact walk
        // keeps its original early-target and stop behavior.
        self.walk(Some(target), avoid).0
    }

    fn walk(&self, target: Option<usize>, avoid: Edge) -> (bool, Box<[bool]>) {
        let mut seen = vec![false; self.successors.len()];
        let mut pending = vec![self.root];
        while let Some(site) = pending.pop() {
            if target == Some(site) {
                return (true, seen.into_boxed_slice());
            }
            if core::mem::replace(&mut seen[site], true) || site == self.stop {
                continue;
            }
            pending.extend(
                self.successors[site]
                    .iter()
                    .copied()
                    .filter(|&next| avoid != Some((site, next))),
            );
        }
        (false, seen.into_boxed_slice())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn reference(
        graph: &[Vec<usize>],
        root: usize,
        stop: usize,
        target: usize,
        avoid: Edge,
    ) -> bool {
        let mut pending = vec![root];
        let mut seen = BTreeSet::new();
        while let Some(site) = pending.pop() {
            if site == target {
                return true;
            }
            if !seen.insert(site) || site == stop {
                continue;
            }
            pending.extend(
                graph[site]
                    .iter()
                    .copied()
                    .filter(|&next| avoid != Some((site, next))),
            );
        }
        false
    }

    #[test]
    fn every_three_node_graph_root_stop_target_and_avoided_edge_matches_old_walk() {
        for bits in 0..512 {
            let graph = (0..3)
                .map(|site| {
                    (0..3)
                        .filter(|&next| bits & (1 << (site * 3 + next)) != 0)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            for root in 0..3 {
                for stop in 0..3 {
                    for limit in [0, 6, MAX_ROW_BYTES] {
                        let mut actual = Reachability::with_limit(graph.clone(), root, stop, limit);
                        for avoid in std::iter::once(None)
                            .chain((0..9).map(|edge| Some((edge / 3, edge % 3))))
                        {
                            for target in 0..4 {
                                assert_eq!(
                                    actual.reaches(target, avoid),
                                    reference(&graph, root, stop, target, avoid)
                                );
                            }
                        }
                        assert!(actual.row_bytes <= limit);
                    }
                }
            }
        }
    }

    #[test]
    fn latch_membership_parallel_edges_cycles_and_contexts_stay_exact() {
        let graph = vec![vec![1, 1, 2], vec![2], vec![0, 3], vec![4], vec![]];
        let mut actual = Reachability::new(graph.clone(), 0, 3);
        assert!(actual.reaches(3, None));
        assert!(!actual.reaches(4, None));
        assert!(actual.reaches(2, Some((0, 1))));
        assert!(!actual.reaches(1, Some((0, 1))));
        assert!(!actual.reaches(3, Some((2, 3))));
        assert!(actual.reaches(2, Some((2, 0))));
        for target in 0..5 {
            assert_eq!(
                actual.reaches(target, None),
                reference(&graph, 0, 3, target, None)
            );
        }
        let mut different = Reachability::new(graph, 4, 0);
        assert!(different.reaches(4, None));
        assert!(!different.reaches(3, None));
    }

    #[test]
    fn exhausted_row_and_byte_budgets_keep_exact_answers_and_existing_rows() {
        let graph = vec![vec![1], vec![2], vec![3], vec![0]];
        let mut limited = Reachability::with_limit(graph.clone(), 0, 2, 4);
        assert!(limited.reaches(2, None));
        assert_eq!(limited.rows.len(), 1);
        for from in 0..8 {
            for to in 0..8 {
                let avoid = Some((from, to));
                for target in 0..5 {
                    assert_eq!(
                        limited.reaches(target, avoid),
                        reference(&graph, 0, 2, target, avoid)
                    );
                }
            }
        }
        assert_eq!(limited.rows.len(), 1);
        assert!(limited.rows.contains_key(&None));
        let mut rows = Reachability::new(graph, 0, 2);
        for from in 0..MAX_ROWS + 17 {
            assert!(rows.reaches(2, Some((from + 4, 0))));
        }
        assert_eq!(rows.rows.len(), MAX_ROWS);
        assert_eq!(rows.row_bytes, MAX_ROWS * 4);
    }
}
