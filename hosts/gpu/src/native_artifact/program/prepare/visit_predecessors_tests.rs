use super::*;

fn reference(edges: &[[Option<u32>; 2]], sites: &BTreeSet<usize>, header: usize) -> Option<usize> {
    edges.iter().enumerate().find_map(|(source, next)| {
        (!sites.contains(&source)
            && next
                .iter()
                .flatten()
                .any(|&target| sites.contains(&(target as usize)) && target as usize != header))
        .then_some(source)
    })
}

#[test]
fn every_small_graph_scope_and_header_matches_the_original_scan() {
    let choices = [None, Some(0), Some(1), Some(2), Some(9), Some(u32::MAX)];
    let labels = [0, 1, 2, 9, u32::MAX as usize];
    for mut graph in 0..choices.len().pow(6) {
        let mut edges = [[None; 2]; 3];
        for row in &mut edges {
            for slot in row {
                *slot = choices[graph % choices.len()];
                graph /= choices.len();
            }
        }
        let index = VisitPredecessors::new(&edges);
        for mask in 0..(1 << labels.len()) {
            let sites = labels
                .iter()
                .enumerate()
                .filter_map(|(bit, &site)| (mask & (1 << bit) != 0).then_some(site))
                .collect();
            for &header in &labels {
                assert_eq!(
                    index.first_outside_entry(&sites, header),
                    reference(&edges, &sites, header),
                    "edges={edges:?} sites={sites:?} header={header}"
                );
                let targets = edges
                    .iter()
                    .enumerate()
                    .filter(|(source, _)| !sites.contains(source))
                    .flat_map(|(_, row)| row.iter().flatten().map(|&target| target as usize))
                    .filter(|target| *target != header && sites.contains(target))
                    .collect::<BTreeSet<_>>();
                assert_eq!(
                    index
                        .outside_targets(&sites, header)
                        .collect::<BTreeSet<_>>(),
                    targets,
                    "targets edges={edges:?} sites={sites:?} header={header}"
                );
            }
        }
    }
}

#[test]
fn sparse_large_graph_keeps_the_first_source_and_rechecks_changed_scopes() {
    let last = 65_535;
    let mut edges = vec![[None; 2]; last + 1];
    edges[1] = [Some(last as u32); 2];
    edges[last - 1] = [Some(last as u32), None];
    edges[last] = [Some(last as u32), None];
    let index = VisitPredecessors::new(&edges);
    let mut sites = BTreeSet::from([last]);
    assert_eq!(index.first_outside_entry(&sites, 0), Some(1));
    sites.insert(1);
    assert_eq!(index.first_outside_entry(&sites, 0), Some(last - 1));
    sites.insert(last - 1);
    assert_eq!(index.first_outside_entry(&sites, 0), None);
    sites.remove(&1);
    assert_eq!(index.first_outside_entry(&sites, 0), Some(1));
    assert_eq!(index.first_outside_entry(&sites, last), None);
    assert_eq!(
        index.first_outside_entry(&sites, 0),
        reference(&edges, &sites, 0)
    );
}

#[test]
fn topology_is_owned_and_distinct_prepared_owners_do_not_share_answers() {
    let mut edges = [[Some(2), None], [Some(2), None], [None, None]];
    let original = VisitPredecessors::new(&edges);
    let sites = BTreeSet::from([2]);
    assert_eq!(original.first_outside_entry(&sites, 1), Some(0));
    edges[0] = [None, None];
    let changed = VisitPredecessors::new(&edges);
    assert_eq!(changed.first_outside_entry(&sites, 1), Some(1));
    assert_eq!(original.first_outside_entry(&sites, 1), Some(0));
    assert_eq!(
        changed.first_outside_entry(&BTreeSet::from([1, 2]), 0),
        None
    );
}

#[test]
fn empty_graph_and_overflow_targets_preserve_full_scan_behavior() {
    let empty = VisitPredecessors::new(&[]);
    assert_eq!(empty.first_outside_entry(&BTreeSet::new(), 0), None);
    assert_eq!(
        empty.first_outside_entry(&BTreeSet::from([usize::MAX]), 0),
        None
    );
    let edges = [[Some(u32::MAX), Some(9)], [Some(9), None]];
    let index = VisitPredecessors::new(&edges);
    for sites in [
        BTreeSet::from([9]),
        BTreeSet::from([9, 0]),
        BTreeSet::from([u32::MAX as usize, 0]),
        BTreeSet::from([9, 0, 1]),
    ] {
        for header in [0, 9, u32::MAX as usize] {
            assert_eq!(
                index.first_outside_entry(&sites, header),
                reference(&edges, &sites, header)
            );
        }
    }
}

#[test]
fn omitted_predecessor_pruning_matches_full_scan_through_branches_and_cycles() {
    fn prune(
        edges: &[[Option<u32>; 2]],
        mut kept: BTreeSet<usize>,
        header: usize,
        latch: usize,
        mut pending: Vec<usize>,
    ) -> BTreeSet<usize> {
        while let Some(site) = pending.pop() {
            if site == header || !kept.remove(&site) {
                continue;
            }
            if site != latch {
                pending.extend(edges[site].into_iter().flatten().map(|site| site as usize));
            }
        }
        kept
    }

    for seed in 0..128usize {
        let edges = (0..9)
            .map(|site| {
                [
                    Some(((site * 7 + seed) % 9) as u32),
                    (site % 3 != seed % 3).then_some(((site + 1 + seed % 4) % 9) as u32),
                ]
            })
            .collect::<Vec<_>>();
        let index = VisitPredecessors::new(&edges);
        for mask in 0..1 << 9 {
            let kept = (0..9)
                .filter(|site| mask & (1 << site) != 0)
                .collect::<BTreeSet<_>>();
            for header in [0, 4, 8] {
                // The original scan is source ordered and may seed duplicates.
                let original = edges
                    .iter()
                    .enumerate()
                    .filter(|(source, _)| !kept.contains(source))
                    .flat_map(|(_, row)| row.iter().flatten().map(|&target| target as usize))
                    .filter(|target| *target != header && kept.contains(target))
                    .collect::<Vec<_>>();
                let indexed = index.outside_targets(&kept, header).collect();
                assert_eq!(
                    prune(&edges, kept.clone(), header, seed % 9, original),
                    prune(&edges, kept.clone(), header, seed % 9, indexed),
                    "seed={seed} kept={kept:?} header={header}"
                );
            }
        }
    }
}
