use super::*;

fn run(
    actions: Vec<Effects>,
    successors: Vec<Vec<usize>>,
    edges: Vec<Vec<Effects>>,
    sites: &[usize],
    entry: usize,
    exit: usize,
    constants: &[usize],
) -> Boundary {
    let reachable = vec![true; actions.len()];
    Graph {
        reachable: &reachable,
        successors: &successors,
        actions: &actions,
        edges: &edges,
    }
    .boundary(sites, entry, exit, &constants.iter().copied().collect())
    .unwrap()
}
fn read(word: usize) -> Effects {
    let mut e = Effects::default();
    e.read(word);
    e
}
fn linear(actions: Vec<Effects>, constants: &[usize]) -> Boundary {
    run(
        actions,
        vec![vec![1], vec![2], vec![]],
        vec![vec![Effects::default()], vec![Effects::default()], vec![]],
        &[0, 1],
        0,
        2,
        constants,
    )
}
#[test]
fn old_target_owner_is_read_before_it_is_replaced() {
    let mut action = read(15);
    action.write(15);
    let result = linear(vec![action, Effects::default(), read(15)], &[]);
    assert_eq!(result.inputs, vec![15]);
    assert_eq!(result.outputs, vec![15]);
    assert_eq!(result.local_words, BTreeSet::from([15]));
}
#[test]
fn conditional_clear_returns_actual_token_and_preserves_old_input() {
    let mut clear = read(5);
    clear.conditional_clear(5);
    let result = linear(vec![clear, Effects::default(), read(5)], &[]);
    assert_eq!(result.inputs, vec![5]);
    assert_eq!(result.outputs, vec![5]);
}
#[test]
fn successful_edge_clear_is_an_output_even_without_source_definition() {
    let mut edge = read(9);
    edge.write(9);
    let result = run(
        vec![Effects::default(), Effects::default(), read(9)],
        vec![vec![1], vec![2], vec![]],
        vec![vec![Effects::default()], vec![edge], vec![]],
        &[0, 1],
        0,
        2,
        &[],
    );
    assert_eq!(result.inputs, vec![9]);
    assert_eq!(result.outputs, vec![9]);
}
#[test]
fn owner_only_read_does_not_capture_uninitialized_descriptor_fields() {
    let mut action = read(105);
    action.write(105);
    action.read(7);
    let result = linear(vec![action, Effects::default(), Effects::default()], &[7]);
    assert_eq!(result.inputs, vec![105]);
    assert!(result.outputs.is_empty());
    assert_eq!(result.local_words, BTreeSet::from([7, 105]));
}
#[test]
fn one_branch_can_preserve_an_owner_while_another_clears_it() {
    let mut clear = Effects::default();
    clear.write(5);
    let result = run(
        vec![Effects::default(), clear, Effects::default(), read(5)],
        vec![vec![1, 2], vec![3], vec![3], vec![]],
        vec![
            vec![Effects::default(); 2],
            vec![Effects::default()],
            vec![Effects::default()],
            vec![],
        ],
        &[0, 1, 2],
        0,
        3,
        &[],
    );
    assert_eq!(result.inputs, vec![5]);
    assert_eq!(result.outputs, vec![5]);
}

#[test]
fn root_omits_region_internals_but_keeps_transport_and_settlement_words() {
    let mut region = read(1);
    region.write(2);
    region.write(3);
    let actions = vec![read(1), region, read(2)];
    let successors = vec![vec![1], vec![2], vec![]];
    let edges = vec![vec![Effects::default()], vec![Effects::default()], vec![]];
    let reachable = vec![true; 3];
    let graph = Graph {
        reachable: &reachable,
        successors: &successors,
        actions: &actions,
        edges: &edges,
    };
    let boundary = graph.boundary(&[1], 1, 2, &BTreeSet::new()).unwrap();
    assert_eq!(boundary.local_words, BTreeSet::from([1, 2, 3]));
    assert_eq!(graph.root_words(&[0, 2], &boundary), BTreeSet::from([1, 2]));
}

#[test]
fn must_zero_preserves_clears_through_loops_but_intersects_unknown_paths() {
    let mut clear = read(5);
    clear.clear(5);
    let mut conditional = Effects::default();
    conditional.conditional_clear(5);
    let mut unknown = Effects::default();
    unknown.write(9);
    let actions = vec![clear, conditional, unknown, Effects::default()];
    let successors = vec![vec![1], vec![1, 2, 3], vec![3], vec![]];
    let edges = vec![
        vec![Effects::default()],
        vec![Effects::default(); 3],
        vec![Effects::default()],
        vec![],
    ];
    let reachable = vec![true; 4];
    let graph = Graph {
        reachable: &reachable,
        successors: &successors,
        actions: &actions,
        edges: &edges,
    };
    let zeros = graph
        .zeros_at(0, &BTreeSet::from([5, 9]), &BTreeSet::from([9]), &[0, 1, 3])
        .unwrap();
    assert_eq!(
        zeros,
        vec![
            BTreeSet::from([9]),
            BTreeSet::from([5, 9]),
            BTreeSet::from([5])
        ]
    );
}

#[test]
fn unknown_write_loses_zero_and_successful_edge_clear_recovers_it() {
    let mut unknown = Effects::default();
    unknown.write(5);
    let mut edge = Effects::default();
    edge.clear(5);
    let actions = vec![unknown, Effects::default(), Effects::default()];
    let successors = vec![vec![1], vec![2], vec![]];
    let edges = vec![vec![Effects::default()], vec![edge], vec![]];
    let reachable = vec![true; 3];
    let graph = Graph {
        reachable: &reachable,
        successors: &successors,
        actions: &actions,
        edges: &edges,
    };
    assert_eq!(
        graph
            .zeros_at(0, &BTreeSet::from([5]), &BTreeSet::from([5]), &[1, 2])
            .unwrap(),
        vec![BTreeSet::new(), BTreeSet::from([5])]
    );
}

#[test]
fn aggregate_copies_read_pre_action_lanes_and_post_copy_clears_win() {
    let mut swap = Effects::default();
    swap.write(5);
    swap.write(9);
    swap.copy_lane(9, 5);
    swap.copy_lane(5, 9);
    let mut clear = Effects::default();
    clear.write(5);
    clear.copy_lane(5, 9);
    clear.clear(5);
    let actions = vec![swap, clear, Effects::default()];
    let successors = vec![vec![1], vec![2], vec![]];
    let edges = vec![vec![Effects::default()], vec![Effects::default()], vec![]];
    let reachable = vec![true; 3];
    let graph = Graph {
        reachable: &reachable,
        successors: &successors,
        actions: &actions,
        edges: &edges,
    };
    assert_eq!(
        graph
            .zeros_at(0, &BTreeSet::from([5, 9]), &BTreeSet::from([5]), &[1, 2])
            .unwrap(),
        vec![BTreeSet::from([9]), BTreeSet::from([5, 9])]
    );
}

#[test]
fn a_native_bypass_cannot_inherit_zeroes_from_skipped_source_actions() {
    let mut clear = read(5);
    clear.clear(5);
    let mut model = Expanded::new(
        vec![true; 2],
        vec![vec![1], vec![]],
        vec![clear, read(5)],
        vec![vec![Effects::default()], vec![]],
    );
    let mut bypass = read(5);
    bypass.conditional_clear(5);
    model.bypass_before(0, 1, bypass).unwrap();
    let graph = model.graph();
    let sites = model.unit_sites(&[0]);
    let boundary = graph.boundary(&sites, 0, 1, &BTreeSet::new()).unwrap();
    assert_eq!(boundary.inputs, vec![5]);
    assert_eq!(boundary.outputs, vec![5]);
    assert_eq!(
        graph
            .zeros_at(0, &BTreeSet::from([5]), &BTreeSet::new(), &[1])
            .unwrap(),
        vec![BTreeSet::new()]
    );
    assert_eq!(
        graph.root_words(&model.unit_sites(&[1]), &boundary),
        BTreeSet::from([5])
    );
}
