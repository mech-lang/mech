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
        let (live, _) = solve_liveness(&next, &reads, &definitions);
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
    let (live, visits) = solve_liveness(&next, &reads, &BTreeMap::new());
    assert!(live.values().all(|values| values == &BTreeSet::from([5])));
    assert!(
        visits <= 4 * nodes,
        "{visits} block visits for {nodes} blocks: full resweeps must not scale quadratically"
    );
}
