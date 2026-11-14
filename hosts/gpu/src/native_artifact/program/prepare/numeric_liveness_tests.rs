use super::*;

#[test]
fn unread_and_unselected_definition_facts_do_not_affect_liveness() {
    for seed in 0..256u32 {
        let next = (0..9)
            .map(|site| {
                (
                    site,
                    vec![(site + seed as usize + 1) % 9, (site * 3 + 2) % 9],
                )
            })
            .collect::<BTreeMap<_, _>>();
        let reads = (0..9)
            .map(|site| {
                let values = if (site as u32 + seed) % 5 == 0 {
                    BTreeSet::new()
                } else {
                    BTreeSet::from([(site as u32 * 3 + seed) % 6])
                };
                (site, values)
            })
            .collect::<BTreeMap<_, _>>();
        let mut definitions = (0..9)
            .map(|site| {
                (
                    site,
                    BTreeSet::from([(site as u32 + seed) % 6, 64, u32::MAX]),
                )
            })
            .collect::<BTreeMap<_, _>>();
        // Definitions outside the selected graph, including definitions of
        // actually read values, cannot kill a fact on a selected edge.
        definitions.insert(100_000, (0..6).collect());
        let read_values = reads.values().flatten().copied().collect::<BTreeSet<_>>();
        let filtered = definitions
            .iter()
            .filter(|(site, _)| next.contains_key(*site))
            .map(|(&site, values)| (site, values.intersection(&read_values).copied().collect()))
            .collect::<BTreeMap<_, _>>();
        let (expected, _, _) = reference_liveness(&next, &reads, &definitions);
        let (sparse, _, _) = solve_liveness(&next, &reads, &filtered);
        assert_eq!(sparse, expected, "sparse seed={seed}");
        for start in 0..9 {
            assert_eq!(
                live_before(&next, &reads, &filtered, start),
                expected[&start],
                "dense seed={seed} start={start}"
            );
        }
    }
}

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
            assert_eq!(
                live_before(&next, &reads, &definitions, start),
                live[&start]
            );
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
fn local_reads_survive_their_own_definition_but_successor_reads_do_not() {
    let next = BTreeMap::from([(7, vec![9]), (9, vec![9, 11]), (11, vec![])]);
    let reads = BTreeMap::from([
        (7, BTreeSet::from([0])),
        (9, BTreeSet::from([63, 64])),
        (11, BTreeSet::from([65, u32::MAX])),
    ]);
    let definitions = BTreeMap::from([
        (7, BTreeSet::from([63])),
        (9, BTreeSet::from([64, 65])),
        (11, BTreeSet::from([u32::MAX])),
    ]);
    assert_eq!(
        live_before(&next, &reads, &definitions, 7),
        BTreeSet::from([0, 64, u32::MAX])
    );
    assert_eq!(
        live_before(&next, &reads, &definitions, 9),
        BTreeSet::from([63, 64, u32::MAX])
    );
}

#[test]
fn sparse_graphs_and_high_value_ids_do_not_allocate_a_dense_matrix() {
    let nodes = 4096;
    let next = (0..nodes)
        .map(|site| (site * 1024, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    let reads = (0..nodes)
        .map(|site| (site * 1024, BTreeSet::from([u32::MAX - site as u32 * 64])))
        .collect::<BTreeMap<_, _>>();
    let (live, visits, facts) = solve(&next, &reads, &BTreeMap::new());
    assert_eq!(visits, nodes);
    assert_eq!(facts, nodes);
    assert_eq!(
        live.live.iter().map(|row| row.0.len()).sum::<usize>(),
        nodes
    );
    assert_eq!(live.at(0), BTreeSet::from([u32::MAX]));
    assert_eq!(live.at((nodes - 1) * 1024), reads[&((nodes - 1) * 1024)]);
}

#[test]
fn empty_graph_has_no_live_facts_or_work() {
    let (live, visits, facts) = solve(&BTreeMap::new(), &BTreeMap::new(), &BTreeMap::new());
    assert!(live.sites.is_empty());
    assert!(live.live.is_empty());
    assert_eq!((visits, facts), (0, 0));
}

#[test]
fn packed_liveness_matches_the_original_solver_on_seeded_sparse_graphs() {
    for seed in 0..256u64 {
        let mut random = seed + 1;
        let mut next_random = || {
            random = random
                .wrapping_mul(6364136223846793005)
                .wrapping_add(1442695040888963407);
            (random >> 32) as u32
        };
        let nodes = (next_random() % 24 + 1) as usize;
        let sites = (0..nodes).map(|site| site * 127 + 3).collect::<Vec<_>>();
        let mut next = BTreeMap::new();
        let mut reads = BTreeMap::new();
        let mut definitions = BTreeMap::new();
        for &site in &sites {
            let edges = next_random() % 4;
            next.insert(
                site,
                (0..edges)
                    .map(|_| sites[(next_random() as usize) % nodes])
                    .collect(),
            );
            for target in [&mut reads, &mut definitions] {
                let count = next_random() % 7;
                let values = (0..count)
                    .map(|_| {
                        let value = next_random();
                        if value % 4 == 0 {
                            u32::MAX - (value >> 2) % 4
                        } else {
                            value % 257
                        }
                    })
                    .collect();
                target.insert(site, values);
            }
        }
        let (expected, visits, facts) = reference_liveness(&next, &reads, &definitions);
        let (actual, actual_visits, actual_facts) = solve_liveness(&next, &reads, &definitions);
        assert_eq!(actual, expected, "seed={seed}");
        for &site in &sites {
            assert_eq!(
                dense_live_before(&next, &reads, &definitions, site, MAX_DENSE_CELLS),
                Some(expected[&site].clone()),
                "dense seed={seed}, site={site}"
            );
        }
        assert_eq!(
            (actual_visits, actual_facts),
            (visits, facts),
            "seed={seed}"
        );
        assert_eq!(
            live_before(&next, &reads, &definitions, sites[0]),
            expected[&sites[0]],
            "seed={seed}"
        );
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

// The complete pre-change transfer/worklist solver remains the comparison oracle.
fn reference_liveness(
    next: &BTreeMap<usize, Vec<usize>>,
    reads: &BTreeMap<usize, BTreeSet<u32>>,
    definitions: &BTreeMap<usize, BTreeSet<u32>>,
) -> (BTreeMap<usize, BTreeSet<u32>>, usize, usize) {
    // Seed each local read, then propagate only newly live facts through the
    // same graph. Definitions kill incoming successor facts, never local reads.
    let mut live = reads.clone();
    let mut differences = reads.clone();
    let mut predecessors = next
        .keys()
        .map(|&site| (site, Vec::new()))
        .collect::<BTreeMap<_, _>>();
    for (&site, successors) in next {
        for successor in successors {
            predecessors.get_mut(successor).unwrap().push(site);
        }
    }
    let mut queued = next.keys().copied().collect::<BTreeSet<_>>();
    let mut pending = next
        .keys()
        .rev()
        .copied()
        .collect::<std::collections::VecDeque<_>>();
    let mut visits = 0usize;
    let mut propagated_facts = 0usize;
    while let Some(site) = pending.pop_front() {
        queued.remove(&site);
        visits = visits.saturating_add(1);
        let values = std::mem::take(differences.get_mut(&site).unwrap());
        propagated_facts = propagated_facts.saturating_add(values.len());
        for &predecessor in &predecessors[&site] {
            let kills = definitions.get(&predecessor);
            let destination = live.get_mut(&predecessor).unwrap();
            let mut changed = false;
            for &value in &values {
                if !kills.is_some_and(|kills| kills.contains(&value)) && destination.insert(value) {
                    differences.get_mut(&predecessor).unwrap().insert(value);
                    changed = true;
                }
            }
            if changed && queued.insert(predecessor) {
                pending.push_back(predecessor);
            }
        }
    }
    (live, visits, propagated_facts)
}

#[test]
fn dense_scratch_limit_falls_back_without_using_raw_value_ordinals() {
    let next = BTreeMap::from([(10, vec![20]), (20, vec![10])]);
    let reads = BTreeMap::from([(10, BTreeSet::from([0])), (20, BTreeSet::from([u32::MAX]))]);
    assert_eq!(
        dense_live_before(&next, &reads, &BTreeMap::new(), 10, 1),
        None
    );
    assert_eq!(
        dense_live_before(&next, &reads, &BTreeMap::new(), 10, 2),
        Some(BTreeSet::from([0, u32::MAX]))
    );
    assert_eq!(
        live_before(&next, &reads, &BTreeMap::new(), 10),
        BTreeSet::from([0, u32::MAX])
    );
}

#[test]
fn dense_local_ordinals_cross_word_boundaries_and_preserve_definition_kills() {
    let next = BTreeMap::from([(0, vec![1]), (1, vec![0, 2]), (2, vec![1])]);
    let reads = BTreeMap::from([
        (0, (0..130).map(|v| u32::MAX - v * 127).collect()),
        (1, BTreeSet::new()),
        (2, BTreeSet::from([1, 2, 3])),
    ]);
    let definitions = BTreeMap::from([(
        1,
        (0..130)
            .filter(|v| v % 2 == 0)
            .map(|v| u32::MAX - v * 127)
            .chain([1, 2])
            .collect(),
    )]);
    let (expected, _, _) = reference_liveness(&next, &reads, &definitions);
    for site in 0..3 {
        assert_eq!(
            live_before(&next, &reads, &definitions, site),
            expected[&site]
        );
    }
}

#[test]
fn query_local_node_orders_match_reference_on_cycles_duplicates_and_high_ids() {
    const N: usize = 17;
    for seed in 0..128usize {
        let labels = (0..N).map(|i| i * 1024 + seed % 7).collect::<Vec<_>>();
        let next = (0..N)
            .map(|i| {
                (
                    labels[i],
                    vec![
                        labels[(i + seed + 1) % N],
                        labels[(i * 3 + 2) % N],
                        labels[(i * 3 + 2) % N],
                    ],
                )
            })
            .collect::<BTreeMap<_, _>>();
        let values = [0, 1, 63, 64, 65, 127, 128, u32::MAX];
        let reads = (0..N)
            .map(|i| {
                (
                    labels[i],
                    if (i + seed) % 5 == 0 {
                        BTreeSet::new()
                    } else {
                        BTreeSet::from([
                            values[(i + seed) % values.len()],
                            values[(i * 3 + seed) % values.len()],
                        ])
                    },
                )
            })
            .collect::<BTreeMap<_, _>>();
        let definitions = (0..N)
            .filter(|i| (i + seed) % 3 != 0)
            .map(|i| {
                (
                    labels[i],
                    BTreeSet::from([values[(i + seed + 1) % values.len()]]),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let (expected, _, _) = reference_liveness(&next, &reads, &definitions);
        for order in 0..4 {
            let mut nodes = labels.clone();
            match order {
                1 => nodes.reverse(),
                2 => nodes.rotate_left(seed % N),
                3 => {
                    for i in (1..N).rev() {
                        nodes.swap(i, (seed * 19 + i * 7) % (i + 1));
                    }
                }
                _ => {}
            }
            let ordinal = |label| nodes.iter().position(|&s| s == label).unwrap();
            let edges = nodes
                .iter()
                .map(|site| next[site].iter().map(|&label| ordinal(label)).collect())
                .collect::<Vec<Vec<usize>>>();
            let uses = nodes
                .iter()
                .map(|site| reads[site].clone())
                .collect::<Vec<_>>();
            let kills = nodes
                .iter()
                .map(|site| {
                    definitions
                        .get(site)
                        .map(|s| s.iter().copied().collect())
                        .unwrap_or_default()
                })
                .collect::<Vec<_>>();
            let (sparse, _, _) = solve_indexed(&edges, &uses, &kills);
            for (start, site) in nodes.iter().enumerate() {
                assert_eq!(
                    sparse[start].values(),
                    expected[site],
                    "sparse seed={seed} order={order} site={site}"
                );
                assert_eq!(
                    live_before_indexed(&edges, &uses, &kills, start),
                    expected[site],
                    "dense seed={seed} order={order} site={site}"
                );
            }
        }
    }
}

#[test]
fn indexed_definition_duplicates_preserve_read_before_kill_and_sparse_fallback() {
    let edges = vec![vec![2], vec![0, 1], vec![1]];
    let reads = vec![
        BTreeSet::from([64]),
        BTreeSet::from([u32::MAX]),
        BTreeSet::from([0]),
    ];
    let kills = vec![vec![0, 0, 64, 64], vec![u32::MAX, u32::MAX], vec![]];
    assert!(dense_live_before_indexed(&edges, &reads, &kills, 0, 0).is_none());
    let (sparse, _, _) = solve_indexed(&edges, &reads, &kills);
    for start in 0..edges.len() {
        assert_eq!(
            live_before_indexed(&edges, &reads, &kills, start),
            sparse[start].values()
        );
    }
    assert_eq!(sparse[0].values(), BTreeSet::from([64, u32::MAX]));
    assert!(sparse[1].values().contains(&u32::MAX));
}
