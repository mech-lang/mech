//! Index the original sole-predecessor-block query. Parallel edges count once.
pub(super) fn derive(
    block_count: usize,
    edges: impl Iterator<Item = [Option<u32>; 2]>,
) -> Vec<(u8, usize)> {
    let mut result = vec![(0_u8, 0_usize); block_count];
    for (site, successors) in edges.enumerate() {
        for (ordinal, successor) in successors.into_iter().enumerate() {
            let Some(next) = successor else { continue };
            if ordinal == 1 && successors[0] == Some(next) {
                continue;
            }
            if let Some((count, first)) = result.get_mut(next as usize) {
                if *count == 0 {
                    *first = site;
                }
                *count = count.saturating_add(1).min(2);
            }
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn index_matches_original_scan_with_parallel_missing_and_outside_edges() {
        for mut pattern in 0..5_usize.pow(6) {
            let edges = (0..3)
                .map(|_| {
                    core::array::from_fn(|_| {
                        let value = pattern % 5;
                        pattern /= 5;
                        (value < 4).then_some(value as u32)
                    })
                })
                .collect::<Vec<[Option<u32>; 2]>>();
            let actual = derive(edges.len(), edges.iter().copied());
            for target in 0..3 {
                let incoming = edges
                    .iter()
                    .enumerate()
                    .filter_map(|(site, edges)| {
                        edges.contains(&Some(target as u32)).then_some(site)
                    })
                    .collect::<Vec<_>>();
                assert_eq!(
                    actual[target],
                    (
                        incoming.len().min(2) as u8,
                        incoming.first().copied().unwrap_or(0)
                    )
                );
            }
        }
    }
    #[test]
    fn empty_graph_has_no_predecessors() {
        assert!(derive(0, [].into_iter()).is_empty());
    }
}
