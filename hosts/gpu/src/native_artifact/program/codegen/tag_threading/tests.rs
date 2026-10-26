use crate::native_artifact::program::codegen::tag_threading::*;

#[test]
fn parallel_carrier_copies_read_old_words_and_observations_keep_evaluated_values() {
    let positions = BTreeMap::from([(10, 0), (11, 1), (12, 2), (13, 3)]);
    let mut state = vec![
        Fact::Constant(0),
        Fact::Constant(1),
        Fact::Unknown,
        Fact::Unknown,
    ];
    Transfer {
        writes: vec![
            (12, Expr::Equal(Input::Word(10), 0)),
            (13, Expr::Copy(Input::Word(10))),
        ],
    }
    .apply(&mut state, &positions, &[0, 1]);
    Transfer {
        writes: vec![
            (10, Expr::Copy(Input::Word(11))),
            (11, Expr::Copy(Input::Word(10))),
        ],
    }
    .apply(&mut state, &positions, &[0, 1]);
    assert_eq!(
        state,
        vec![
            Fact::Constant(1),
            Fact::Constant(0),
            Fact::Constant(1),
            Fact::Constant(0)
        ]
    );
    // A saved predicate and alias refer to the observed value, not the
    // current carrier slot. An opaque redefinition kills only its output.
    Transfer {
        writes: vec![(10, Expr::Copy(Input::Unknown))],
    }
    .apply(&mut state, &positions, &[0, 1]);
    assert_eq!(
        state,
        vec![
            Fact::Unknown,
            Fact::Constant(0),
            Fact::Constant(1),
            Fact::Constant(0)
        ]
    );
}

#[test]
fn invalid_remainder_and_noncanonical_boolean_words_stay_conservative() {
    let positions = BTreeMap::from([(0, 0), (1, 1), (2, 2), (3, 3)]);
    let mut state = vec![Fact::Other, Fact::Unknown, Fact::Unknown, Fact::Unknown];
    Transfer {
        writes: vec![
            (1, Expr::Equal(Input::Word(0), 0)),
            (2, Expr::Equal(Input::Word(0), 7)),
            (3, Expr::Not(Input::Constant(2))),
        ],
    }
    .apply(&mut state, &positions, &[0, 1]);
    assert_eq!(
        state,
        vec![Fact::Other, Fact::Constant(0), Fact::Unknown, Fact::Unknown]
    );
    Transfer {
        writes: vec![(2, Expr::Not(Input::Word(1)))],
    }
    .apply(&mut state, &positions, &[0, 1]);
    assert_eq!(state[2], Fact::Constant(1));
    assert_eq!(literal_equal(Fact::Other, Fact::Other, &[0, 1]), None);
    assert_eq!(
        literal_equal(Fact::Other, Fact::Constant(0), &[0, 1]),
        Some(false)
    );
    assert_eq!(literal_equal(Fact::Constant(7), Fact::Other, &[0, 1]), None);
}

#[test]
fn latch_reset_uses_post_copy_tag_and_discards_every_other_traversal_fact() {
    let initial = vec![Fact::Unknown, Fact::Constant(19), Fact::Unknown];
    let positions = BTreeMap::from([(0, 0), (1, 1), (2, 2)]);
    let mut completed = vec![Fact::Constant(0), Fact::Constant(19), Fact::Constant(1)];
    Transfer {
        writes: vec![(0, Expr::Copy(Input::Word(2)))],
    }
    .apply(&mut completed, &positions, &[0, 1]);
    assert_eq!(
        next_header(&initial, 0, completed[0], &[0, 1]),
        vec![Fact::Constant(1), Fact::Constant(19), Fact::Unknown]
    );
    assert_eq!(next_header(&initial, 0, Fact::Unknown, &[0, 1]), initial);
    assert_eq!(
        next_header(&initial, 0, Fact::Constant(u64::MAX), &[0, 1]),
        vec![Fact::Other, Fact::Constant(19), Fact::Unknown]
    );
}

#[test]
fn product_interning_bounds_adversarial_state_expansion_without_merging_facts() {
    let mut builder = Builder {
        keys: Vec::new(),
        interned: BTreeMap::new(),
        sites: BTreeSet::from([7]),
        exit: 8,
    };
    for word in 0..MAX_NODES {
        assert_eq!(
            builder.intern(7, vec![Fact::Constant(word as u64)]),
            Some(Target::Node(word))
        );
    }
    assert_eq!(
        builder.intern(7, vec![Fact::Constant(0)]),
        Some(Target::Node(0))
    );
    assert_eq!(builder.intern(7, vec![Fact::Unknown]), None);
    assert_eq!(builder.intern(8, vec![Fact::Unknown]), Some(Target::Exit));
    assert_eq!(builder.intern(9, vec![Fact::Unknown]), None);
    assert_eq!(builder.keys.len(), MAX_NODES);
}
