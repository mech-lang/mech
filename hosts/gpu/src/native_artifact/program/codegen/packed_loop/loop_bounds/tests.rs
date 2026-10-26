use crate::native_artifact::program::codegen::packed_loop::loop_bounds::*;
#[test]
fn guarded_loop_rejects_alternate_state_join_and_previous_visit_guard() {
    use crate::native_artifact::program::codegen::tag_threading::Node;
    let node = |site, edge, header_tag| Node {
        site,
        edge,
        header_tag,
        folded_branch: false,
        known_latch: false,
    };
    let mut graph = Graph {
        carrier_word: 0,
        variants: vec![0, 1],
        nodes: vec![
            node(
                10,
                Edge::Dispatch {
                    cases: vec![(0, Target::Node(2)), (1, Target::Node(6))],
                    other: Target::Node(6),
                },
                None,
            ),
            node(10, Edge::Jump(Target::Node(2)), Some(0)),
            node(11, Edge::Jump(Target::Node(3)), None), // Active body prefix.
            node(
                12,
                Edge::Branch {
                    yes: Target::Node(6),
                    no: Target::Node(4),
                },
                None,
            ), // First guard.
            node(13, Edge::Jump(Target::Node(5)), None), // Certified range.
            node(14, Edge::Jump(Target::Node(1)), None), // Original latch.
            node(15, Edge::Reject, None),
        ],
    };
    let certified = BTreeSet::from([4]);
    assert!(guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified));
    graph.nodes[6].edge = Edge::Jump(Target::Node(4));
    assert!(
        !guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified),
        "Other must not join after the guard"
    );
    graph.nodes[6].edge = Edge::Jump(Target::Node(2));
    assert!(
        !guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified),
        "a copied tag is not a certified prefix entry"
    );
    graph.nodes[6].edge = Edge::Reject;
    graph
        .nodes
        .push(node(10, Edge::Jump(Target::Node(4)), Some(1)));
    assert!(
        !guarded_entry_dominates(&graph, 10, 0, 2, 3, &certified),
        "every later header starts a fresh dominance root"
    );
}
#[test]
fn guarded_loop_stop_equivalence_includes_noncarrier_live_words() {
    let a = State::from([(1, 10), (2, 20), (3, 30), (4, 40)]);
    // Same tag and map payload are insufficient if a later source action
    // also observes a word defined by the bypassed guard prefix.
    let b = State::from([(1, 10), (2, 20), (3, 31), (4, 41)]);
    assert!(same_live(&a, &b, &BTreeSet::from([1, 2])));
    assert!(!same_live(&a, &b, &BTreeSet::from([1, 2, 3])));
    assert!(!same_live(&a, &b, &BTreeSet::from([1, 2, 5])));
}
#[test]
fn guarded_loop_terms_are_finite_and_preserve_exact_value_identity() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Input(1)).unwrap();
    let one = terms.constant(1).unwrap();
    let two = terms.constant(2).unwrap();
    assert_eq!(terms.add(index, one), terms.add(one, index));
    assert_ne!(terms.add(index, one), terms.add(index, two));
    for word in 0..MAX_TERMS {
        if terms.intern(Expr::Input(word)).is_none() {
            break;
        }
    }
    assert_eq!(terms.values.len(), MAX_TERMS);
    assert!(terms.intern(Expr::Input(MAX_TERMS + 1)).is_none());
    assert_eq!(
        terms.intern(Expr::Input(1)),
        Some(index),
        "known identities survive proof-budget exhaustion"
    );
}
