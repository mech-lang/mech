//! Per-call topology for current-visit validation. Scratch ordinals are shared
//! only while discovering graphs; no definition or scope-validity proof is kept.
#[derive(Clone, Copy, Default)]
struct Ordinal {
    generation: usize,
    index: usize,
}

pub(super) struct Scratch {
    ordinals: Vec<Ordinal>,
    generation: usize,
}

pub(super) struct Node {
    pub(super) site: usize,
    pub(super) incoming: Vec<usize>,
    pub(super) successors: [Option<usize>; 2],
    pub(super) degree: usize,
    visited: bool,
}

impl Scratch {
    pub(super) fn new(blocks: usize) -> Self {
        Self {
            ordinals: vec![Ordinal::default(); blocks],
            generation: 0,
        }
    }

    fn ordinal(&mut self, site: u32, nodes: &mut Vec<Node>) -> Result<usize, &'static str> {
        let ordinal = self
            .ordinals
            .get_mut(site as usize)
            .ok_or("scope site outside blocks")?;
        if ordinal.generation != self.generation {
            *ordinal = Ordinal {
                generation: self.generation,
                index: nodes.len(),
            };
            nodes.push(Node {
                site: site as usize,
                incoming: Vec::new(),
                successors: [None, None],
                degree: 0,
                visited: false,
            });
        }
        Ok(ordinal.index)
    }

    pub(super) fn discover(
        &mut self,
        entry: u32,
        mut successors: impl FnMut(usize) -> Result<[Option<u32>; 2], &'static str>,
    ) -> Result<Vec<Node>, &'static str> {
        self.generation = self
            .generation
            .checked_add(1)
            .ok_or("scope generation overflow")?;
        let mut nodes = Vec::new();
        let entry = self.ordinal(entry, &mut nodes)?;
        let mut pending = vec![entry];
        while let Some(index) = pending.pop() {
            if std::mem::replace(&mut nodes[index].visited, true) {
                continue;
            }
            // Query in the original DFS pop order. In particular, do not
            // validate a newly queued successor before the old traversal did.
            let next = successors(nodes[index].site)?;
            for (edge, site) in next.into_iter().enumerate() {
                let Some(site) = site else { continue };
                let target = self.ordinal(site, &mut nodes)?;
                nodes[target].incoming.push(index);
                nodes[target].degree = nodes[target]
                    .degree
                    .checked_add(1)
                    .ok_or("scope degree overflow")?;
                nodes[index].successors[edge] = Some(target);
                pending.push(target);
            }
        }
        Ok(nodes)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::VecDeque;

    fn order(mut nodes: Vec<Node>) -> Vec<usize> {
        let mut ready = (0..nodes.len())
            .filter(|&i| nodes[i].degree == 0)
            .collect::<Vec<_>>();
        ready.sort_unstable_by_key(|&i| nodes[i].site);
        let mut ready = VecDeque::from(ready);
        let mut result = Vec::new();
        while let Some(i) = ready.pop_front() {
            result.push(nodes[i].site);
            for next in nodes[i].successors.into_iter().flatten() {
                nodes[next].degree -= 1;
                if nodes[next].degree == 0 {
                    ready.push_back(next)
                }
            }
        }
        result
    }

    #[test]
    fn topology_and_failure_order_match_original_dense_traversal() {
        for seed in 0..256usize {
            let graph = (0..31)
                .map(|i| {
                    [
                        Some(((i * 7 + seed) % 31) as u32),
                        (i % 3 == 0).then_some(((i + 1) % 31) as u32),
                    ]
                })
                .collect::<Vec<_>>();
            let mut seen = vec![false; graph.len()];
            let mut incoming = vec![Vec::new(); graph.len()];
            let mut pending = vec![0];
            let mut expected = Vec::new();
            while let Some(site) = pending.pop() {
                if std::mem::replace(&mut seen[site], true) {
                    continue;
                }
                expected.push(site);
                for next in graph[site].into_iter().flatten() {
                    incoming[next as usize].push(site);
                    pending.push(next as usize);
                }
            }
            let mut scratch = Scratch::new(graph.len());
            let mut visits = Vec::new();
            let nodes = scratch
                .discover(0, |site| {
                    visits.push(site);
                    Ok(graph[site])
                })
                .unwrap();
            assert_eq!(visits, expected);
            assert_eq!(nodes.len(), expected.len());
            for node in &nodes {
                assert_eq!(
                    node.incoming
                        .iter()
                        .map(|&i| nodes[i].site)
                        .collect::<Vec<_>>(),
                    incoming[node.site]
                );
                assert_eq!(node.degree, incoming[node.site].len());
                assert_eq!(
                    node.successors.map(|i| i.map(|i| nodes[i].site as u32)),
                    graph[node.site]
                );
            }
            for failure in expected {
                let mut visits = Vec::new();
                assert!(
                    scratch
                        .discover(0, |site| {
                            visits.push(site);
                            if site == failure {
                                Err("selected failure")
                            } else {
                                Ok(graph[site])
                            }
                        })
                        .is_err()
                );
                assert_eq!(visits.last(), Some(&failure));
                assert_eq!(visits, expected_prefix(&graph, failure));
            }
        }
    }

    fn expected_prefix(graph: &[[Option<u32>; 2]], stop: usize) -> Vec<usize> {
        let mut seen = vec![false; graph.len()];
        let mut pending = vec![0];
        let mut result = Vec::new();
        while let Some(site) = pending.pop() {
            if std::mem::replace(&mut seen[site], true) {
                continue;
            }
            result.push(site);
            if site == stop {
                break;
            }
            pending.extend(graph[site].into_iter().flatten().map(|v| v as usize));
        }
        result
    }

    #[test]
    fn repeated_and_nested_queries_keep_independent_local_edges() {
        let mut scratch = Scratch::new(100_001);
        let parent = scratch
            .discover(0, |site| {
                Ok(if site == 0 {
                    [Some(100_000), Some(100_000)]
                } else {
                    [None, None]
                })
            })
            .unwrap();
        assert_eq!(parent.len(), 2);
        let child = scratch.discover(60_000, |_| Ok([None, None])).unwrap();
        assert_eq!(child.len(), 1);
        assert_eq!(order(parent), [0, 100_000]);
        assert_eq!(order(child), [60_000]);
        let again = scratch.discover(0, |_| Ok([None, None])).unwrap();
        assert_eq!(again.len(), 1);
        assert_eq!(order(again), [0]);
    }

    #[test]
    fn acyclic_ready_order_preserves_original_source_and_edge_order() {
        for seed in 0..128usize {
            let graph = (0..47)
                .map(|i| {
                    let next = i + 1;
                    let other = next + seed % 7;
                    [
                        (next < 47).then_some(next as u32),
                        (other < 47).then_some(other as u32),
                    ]
                })
                .collect::<Vec<_>>();
            let mut degrees = vec![0; graph.len()];
            for edges in &graph {
                for next in edges.iter().flatten() {
                    degrees[*next as usize] += 1
                }
            }
            let mut ready = (0..graph.len())
                .filter(|&i| degrees[i] == 0)
                .collect::<VecDeque<_>>();
            let mut expected = Vec::new();
            while let Some(site) = ready.pop_front() {
                expected.push(site);
                for next in graph[site].into_iter().flatten() {
                    degrees[next as usize] -= 1;
                    if degrees[next as usize] == 0 {
                        ready.push_back(next as usize)
                    }
                }
            }
            assert_eq!(
                order(
                    Scratch::new(graph.len())
                        .discover(0, |site| Ok(graph[site]))
                        .unwrap()
                ),
                expected
            );
        }
    }

    #[test]
    fn closed_cycles_and_invalid_sites_fail_without_stale_ordinals() {
        let mut scratch = Scratch::new(2);
        assert!(
            order(
                scratch
                    .discover(0, |site| Ok([Some((1 - site) as u32), None]))
                    .unwrap()
            )
            .is_empty()
        );
        assert!(scratch.discover(2, |_| Ok([None, None])).is_err());
        assert!(scratch.discover(0, |_| Ok([Some(3), None])).is_err());
        assert_eq!(
            order(scratch.discover(0, |_| Ok([None, None])).unwrap()),
            [0]
        );
    }
}
