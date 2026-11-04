use super::*;

#[test]
fn liveness_agrees_with_read_before_definition_paths_through_cycles_and_branches() {
    for seed in 0..32usize {
        let next = (0..9)
            .map(|site| (site, vec![(site + seed + 1) % 9, (site * 3 + 2) % 9]))
            .collect::<BTreeMap<_, _>>();
        let reads = (0..9)
            .map(|site| (site, BTreeSet::from([((site * 3 + seed) % 6) as u32])))
            .collect::<BTreeMap<_, _>>();
        let definitions = (0..9)
            .filter(|site| (site + seed) % 3 == 0)
            .map(|site| (site, BTreeSet::from([((site + seed) % 6) as u32])))
            .collect::<BTreeMap<_, _>>();
        let (live, _, _) = solve_liveness(&next, &reads, &definitions);
        for start in 0..9 {
            for value in 0..6 {
                let mut pending = vec![start];
                let mut seen = BTreeSet::new();
                let mut readable = false;
                while let Some(site) = pending.pop() {
                    if !seen.insert(site) {
                        continue;
                    }
                    if reads[&site].contains(&value) {
                        readable = true;
                        break;
                    }
                    if !definitions.get(&site).is_some_and(|v| v.contains(&value)) {
                        pending.extend(&next[&site]);
                    }
                }
                assert_eq!(
                    live[&start].contains(&value),
                    readable,
                    "seed={seed} start={start} value={value}"
                );
            }
        }
    }
}

#[test]
fn reverse_numbered_liveness_cycle_does_not_require_quadratic_full_sweeps() {
    let nodes = 600;
    let next = (0..nodes)
        .map(|site| (site, vec![(site + nodes - 1) % nodes]))
        .collect::<BTreeMap<_, _>>();
    let reads = (0..nodes)
        .map(|site| {
            (
                site,
                if site == 0 {
                    BTreeSet::from([5])
                } else {
                    BTreeSet::new()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let (live, visits, _) = solve_liveness(&next, &reads, &BTreeMap::new());
    assert!(live.values().all(|values| values == &BTreeSet::from([5])));
    assert!(
        visits <= 4 * nodes,
        "{visits} block visits for {nodes} blocks: full resweeps must not scale quadratically"
    );
}

#[test]
fn each_live_fact_is_propagated_once_per_control_edge() {
    let nodes = 100;
    let next = (0..nodes)
        .map(|site| (site, vec![(site + nodes - 1) % nodes]))
        .collect::<BTreeMap<_, _>>();
    let reads = (0..nodes)
        .map(|site| {
            (
                site,
                if site % 5 == 0 {
                    BTreeSet::from([(site / 5) as u32])
                } else {
                    BTreeSet::new()
                },
            )
        })
        .collect::<BTreeMap<_, _>>();
    let (live, _, facts) = solve_liveness(&next, &reads, &BTreeMap::new());
    assert!(live.values().all(|values| values.len() == nodes / 5));
    assert!(
        facts <= nodes * (nodes / 5),
        "{facts} propagated facts exceed one pass of each live fact per edge"
    );
}

#[test]
fn liveness_projection_keeps_exactly_the_requested_values() {
    let next = BTreeMap::from([(0, vec![1, 2]), (1, vec![3]), (2, vec![0, 3]), (3, vec![1])]);
    let reads = BTreeMap::from([
        (0, BTreeSet::from([1, 5])),
        (1, BTreeSet::from([2])),
        (2, BTreeSet::from([3])),
        (3, BTreeSet::from([1, 4])),
    ]);
    let definitions = BTreeMap::from([
        (0, BTreeSet::from([2])),
        (1, BTreeSet::from([1, 3])),
        (3, BTreeSet::from([5])),
    ]);
    let (full, _, _) = solve_liveness(&next, &reads, &definitions);
    for mask in 0..64 {
        let wanted = (0..6)
            .filter(|value| mask & (1 << value) != 0)
            .collect::<BTreeSet<_>>();
        let projected = reads
            .iter()
            .map(|(&site, values)| (site, values.intersection(&wanted).copied().collect()))
            .collect();
        let (actual, _, _) = solve_liveness(&next, &projected, &definitions);
        for site in 0..4 {
            assert_eq!(
                actual[&site],
                full[&site].intersection(&wanted).copied().collect()
            );
        }
    }
}
