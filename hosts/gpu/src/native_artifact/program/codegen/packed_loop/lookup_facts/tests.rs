use crate::native_artifact::program::codegen::packed_loop::lookup_facts::*;
fn step(effect: Effect, next: usize) -> Step {
    Step {
        effect,
        edge: Edge::Jump(Target::Node(next)),
        condition: Expr::Unknown,
    }
}
#[test]
fn lookup_facts_follow_membership_aliases_and_invalidate_on_insert_selection_and_headers() {
    let mut nodes = vec![
        step(Effect::Reset, 1),
        step(Effect::Select, 2),
        step(Effect::Contains(10), 3),
        step(Effect::Words(vec![(11, Expr::Not(10))]), 4),
        step(Effect::Words(Vec::new()), 5),
        step(Effect::Get(12), 7),
        step(Effect::Insert, 7),
        step(Effect::Select, 8),
        step(Effect::Contains(13), 9),
        step(Effect::Reset, 10),
        step(Effect::Words(Vec::new()), 0),
    ];
    nodes[4].edge = Edge::Branch {
        yes: Target::Node(6),
        no: Target::Node(5),
    };
    nodes[4].condition = Expr::Copy(11);
    let facts = solve(&nodes).unwrap();
    assert_eq!(
        (facts[2], facts[5], facts[6], facts[7], facts[8], facts[10]),
        (
            Fact::Fresh,
            Fact::Present,
            Fact::Absent,
            Fact::Present,
            Fact::Fresh,
            Fact::Unknown
        )
    );
    // A saved predicate cannot describe a newly selected key or a map
    // after insertion. Neither branch may manufacture presence there.
    for invalidation in [Effect::Select, Effect::Insert, Effect::Reset] {
        let mut altered = nodes.clone();
        altered[3].effect = invalidation;
        let facts = solve(&altered).unwrap();
        assert_eq!(facts[5], facts[6]);
    }
}
#[test]
fn lookup_facts_join_hits_and_misses_without_assuming_a_bypassed_lookup() {
    let mut nodes = vec![
        step(Effect::Select, 1),
        step(Effect::Contains(10), 2),
        step(Effect::Words(Vec::new()), 3),
        step(Effect::Words(Vec::new()), 5),
        step(Effect::Words(Vec::new()), 5),
        step(Effect::Words(Vec::new()), 0),
    ];
    nodes[2].edge = Edge::Branch {
        yes: Target::Node(3),
        no: Target::Node(4),
    };
    nodes[2].condition = Expr::Equal(10, 1);
    assert_eq!(solve(&nodes).unwrap()[5], Fact::Cached);
    nodes[0].edge = Edge::Branch {
        yes: Target::Node(1),
        no: Target::Node(5),
    };
    assert_eq!(solve(&nodes).unwrap()[5], Fact::Unknown);
    // A copy cycle is simultaneous; swapping a predicate into another
    // slot must not make both slots aliases of it.
    let mut state = State {
        lookup: Fact::Cached,
        aliases: BTreeMap::from([(1, true)]),
    };
    state
        .apply(&Effect::Words(vec![(1, Expr::Copy(2)), (2, Expr::Copy(1))]))
        .unwrap();
    assert_eq!(state.aliases, BTreeMap::from([(2, true)]));
    nodes[0].edge = Edge::Jump(Target::Node(MAX_NODES + 1));
    assert!(solve(&nodes).is_none());
}
