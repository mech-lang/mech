use super::*;

fn reachable(
    successors: &[Vec<usize>],
    definitions: &[Vec<usize>],
    reads: &[Vec<usize>],
    post_uses: &[Vec<usize>],
    start: usize,
    id: usize,
) -> bool {
    let mut pending = vec![start];
    let mut seen = vec![false; successors.len()];
    while let Some(site) = pending.pop() {
        if core::mem::replace(&mut seen[site], true) {
            continue;
        }
        if reads[site].contains(&id) || post_uses[site].contains(&id) {
            return true;
        }
        if !definitions[site].contains(&id) {
            pending.extend(&successors[site]);
        }
    }
    false
}

fn check(
    successors: &[Vec<usize>],
    definitions: &[Vec<usize>],
    reads: &[Vec<usize>],
    post_uses: &[Vec<usize>],
    count: usize,
) {
    let live = derive(
        successors,
        definitions,
        reads,
        post_uses,
        count.div_ceil(64),
    );
    for site in 0..successors.len() {
        for id in 0..count {
            assert_eq!(
                live[site][id / 64] & (1_u64 << (id % 64)) != 0,
                reachable(successors, definitions, reads, post_uses, site, id),
                "site {site}, owner {id}"
            );
        }
    }
}

#[test]
fn empty_graph_and_scalar_only_graph_need_no_owner_words() {
    assert!(derive(&[], &[], &[], &[], 0).is_empty());
    assert_eq!(
        derive(
            &[vec![1], vec![0]],
            &[vec![], vec![]],
            &[vec![], vec![]],
            &[vec![], vec![]],
            0
        ),
        vec![vec![], vec![]]
    );
}

#[test]
fn reads_precede_definitions_and_post_uses_are_observable() {
    let graph = vec![vec![1], vec![2], vec![3], vec![0]];
    let definitions = vec![vec![], vec![0, 1], vec![], vec![]];
    let reads = vec![vec![], vec![0], vec![], vec![1]];
    let post = vec![vec![], vec![], vec![2], vec![]];
    check(&graph, &definitions, &reads, &post, 3);
    let result = derive(&graph, &definitions, &reads, &post, 1);
    assert_eq!(result[0][0] & 1, 1);
    assert_eq!(result[0][0] & 2, 0);
    assert_eq!(result[0][0] & 4, 4);
}

#[test]
fn owner_word_boundaries_and_duplicate_edges_match_independent_traversal() {
    let graph = vec![vec![1, 1], vec![0, 2], vec![3], vec![1]];
    let definitions = vec![vec![0, 64], vec![63, 127], vec![], vec![129]];
    let reads = vec![vec![63], vec![64, 129], vec![0, 127], vec![]];
    let post = vec![vec![128], vec![], vec![], vec![]];
    check(&graph, &definitions, &reads, &post, 130);
}

#[test]
fn every_two_node_graph_read_definition_and_post_use_pattern_matches() {
    for graph_bits in 0..16 {
        let graph = (0..2)
            .map(|site| {
                (0..2)
                    .filter(|&next| graph_bits & (1 << (site * 2 + next)) != 0)
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        for read_bits in 0..4 {
            for definition_bits in 0..4 {
                for post_bits in 0..4 {
                    let rows = |bits| {
                        (0..2)
                            .map(|site| {
                                if bits & (1 << site) != 0 {
                                    vec![0]
                                } else {
                                    vec![]
                                }
                            })
                            .collect::<Vec<_>>()
                    };
                    check(
                        &graph,
                        &rows(definition_bits),
                        &rows(read_bits),
                        &rows(post_bits),
                        1,
                    );
                }
            }
        }
    }
}

#[test]
fn seeded_branching_cyclic_graphs_match_every_start_and_owner() {
    for seed in 0..256_u64 {
        let mut random = seed + 1;
        let mut next = || {
            random = random.wrapping_mul(6364136223846793005).wrapping_add(1);
            (random >> 32) as usize
        };
        let count = next() % 32 + 1;
        let mut graph = Vec::new();
        let mut definitions = Vec::new();
        let mut reads = Vec::new();
        let mut post = Vec::new();
        for _ in 0..count {
            graph.push(if next() % 3 == 0 {
                vec![]
            } else {
                vec![next() % count, next() % count]
            });
            definitions.push(vec![next() % 130, next() % 130]);
            reads.push(vec![next() % 130, next() % 130]);
            post.push(if next() % 3 == 0 {
                vec![next() % 130]
            } else {
                vec![]
            });
        }
        check(&graph, &definitions, &reads, &post, 130);
    }
}
