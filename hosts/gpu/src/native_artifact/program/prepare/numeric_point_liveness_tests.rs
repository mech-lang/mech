use std::collections::BTreeSet;

fn reference(
    next: &[Vec<usize>],
    reads: &[BTreeSet<u32>],
    defs: &[Vec<u32>],
    start: usize,
) -> BTreeSet<u32> {
    let mut live = reads.to_vec();
    loop {
        let mut changed = false;
        for site in (0..next.len()).rev() {
            let mut incoming = reads[site].clone();
            for &successor in &next[site] {
                incoming.extend(
                    live[successor]
                        .iter()
                        .copied()
                        .filter(|v| !defs[site].contains(v)),
                );
            }
            if live[site] != incoming {
                live[site] = incoming;
                changed = true;
            }
        }
        if !changed {
            return live[start].clone();
        }
    }
}
#[test]
fn bounded_point_queries_match_independent_fixed_point_on_varied_graphs() {
    let mut random = 0x9730_c0de_1277_491bu64;
    let mut draw = || {
        random ^= random << 13;
        random ^= random >> 7;
        random ^= random << 17;
        random
    };
    let values = [0, 1, 2, 7, 31, 63, 64, 65, 127, 128, 65536, u32::MAX];
    for nodes in 1..=32 {
        for _ in 0..128 {
            let next = (0..nodes)
                .map(|_| {
                    (0..draw() % 4)
                        .map(|_| draw() as usize % nodes)
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            let reads = (0..nodes)
                .map(|_| {
                    (0..draw() % 5)
                        .map(|_| values[draw() as usize % values.len()])
                        .collect::<BTreeSet<_>>()
                })
                .collect::<Vec<_>>();
            let defs = (0..nodes)
                .map(|_| {
                    (0..draw() % 4)
                        .map(|_| values[draw() as usize % values.len()])
                        .collect::<BTreeSet<_>>()
                        .into_iter()
                        .collect::<Vec<_>>()
                })
                .collect::<Vec<_>>();
            for start in 0..nodes.min(4) {
                let expected = reference(&next, &reads, &defs, start);
                assert_eq!(
                    super::super::live_before_indexed(&next, &reads, &defs, start),
                    expected
                );
                for budget in [0, 1, nodes, nodes * 4, usize::MAX] {
                    let (answer, steps) = super::solve(&next, &reads, &defs, start, budget);
                    assert!(steps <= budget);
                    if budget == usize::MAX {
                        assert!(answer.is_some());
                    }
                    if let Some(answer) = answer {
                        assert_eq!(
                            answer, expected,
                            "nodes={nodes}, start={start}, budget={budget}"
                        );
                    }
                }
            }
        }
    }
}
#[test]
fn own_reads_branch_kills_and_long_cycles_keep_exact_incoming_semantics() {
    let next = vec![vec![1, 2], vec![3], vec![3], vec![3]];
    let mut reads = vec![BTreeSet::new(); 4];
    reads[3].insert(42);
    let mut defs = vec![Vec::new(); 4];
    defs[1].push(42);
    assert_eq!(
        super::solve(&next, &reads, &defs, 0, usize::MAX).0,
        Some(BTreeSet::from([42]))
    );
    defs[0].push(42);
    assert_eq!(
        super::solve(&next, &reads, &defs, 0, usize::MAX).0,
        Some(BTreeSet::new())
    );
    reads[0].insert(42);
    assert_eq!(
        super::solve(&next, &reads, &defs, 0, usize::MAX).0,
        Some(BTreeSet::from([42]))
    );
    let nodes = 100_000;
    let mut next = (0..nodes)
        .map(|n| vec![(n + 1) % nodes])
        .collect::<Vec<_>>();
    next[nodes - 1].push(nodes - 1);
    let mut reads = vec![BTreeSet::new(); nodes];
    reads[nodes - 1].insert(u32::MAX);
    let mut defs = vec![Vec::new(); nodes];
    assert_eq!(
        super::solve(&next, &reads, &defs, 0, usize::MAX).0,
        Some(BTreeSet::from([u32::MAX]))
    );
    defs[0].push(u32::MAX);
    assert_eq!(
        super::solve(&next, &reads, &defs, 0, usize::MAX).0,
        Some(BTreeSet::new())
    );
    assert_eq!(super::solve(&[], &[], &[], 0, 0).0, Some(BTreeSet::new()));
}

#[test]
fn an_expensive_sparse_query_falls_back_to_the_existing_solver() {
    let nodes = 2048;
    let next = (0..nodes)
        .map(|site| (site + 1 < nodes).then_some(site + 1).into_iter().collect())
        .collect::<Vec<Vec<usize>>>();
    let expected = (0..96).collect::<BTreeSet<u32>>();
    let mut reads = vec![BTreeSet::new(); nodes];
    reads[nodes - 1] = expected.clone();
    let definitions = vec![Vec::new(); nodes];
    let budget = nodes * 64;
    let (answer, steps) = super::solve(&next, &reads, &definitions, 0, budget);
    assert!(answer.is_none());
    assert_eq!(steps, budget);
    assert_eq!(
        super::super::live_before_indexed(&next, &reads, &definitions, 0),
        expected
    );
}
