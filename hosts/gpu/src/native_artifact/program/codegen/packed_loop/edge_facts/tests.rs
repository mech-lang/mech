use crate::native_artifact::program::codegen::packed_loop::edge_facts::*;

#[test]
fn unsigned_bounds_use_exact_current_values_and_successful_difference() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let n = terms.intern(Expr::Initial(1)).unwrap();
    let width = terms.intern(Expr::Initial(2)).unwrap();
    let other = terms.intern(Expr::Initial(3)).unwrap();
    let one = terms.constant(1).unwrap();
    let two = terms.constant(2).unwrap();
    let maximum = terms.constant(u64::MAX).unwrap();
    let difference = terms
        .intern(Expr::Arithmetic(Arithmetic::Sub, n, index))
        .unwrap();
    let mut state = State::default();
    assert!(!state.safe(&terms, Arithmetic::Sub, n, index));
    assert!(!state.safe(&terms, Arithmetic::Add, index, one));
    assert!(!state.safe(&terms, Arithmetic::Add, maximum, one));
    assert!(state.assume(&terms, Relation::Lt, index, n).unwrap());
    assert!(state.safe(&terms, Arithmetic::Sub, n, index));
    assert!(state.safe(&terms, Arithmetic::Add, index, one));
    assert!(!state.safe(&terms, Arithmetic::Add, index, two));
    assert!(
        state
            .assume(&terms, Relation::Le, width, difference)
            .unwrap()
    );
    assert!(
        !state.safe(&terms, Arithmetic::Add, index, width),
        "a symbolic subtraction must actually have succeeded"
    );
    state.successful.insert((Arithmetic::Sub, n, index));
    assert!(state.safe(&terms, Arithmetic::Add, index, width));
    assert!(state.safe(&terms, Arithmetic::Add, width, index));
    assert!(!state.safe(&terms, Arithmetic::Add, other, width));
    assert!(!state.safe(&terms, Arithmetic::Add, index, other));
    assert!(!state.safe(&terms, Arithmetic::Sub, other, index));
    let changed = terms.intern(Expr::Definition(7, 0)).unwrap();
    assert!(
        !state.safe(&terms, Arithmetic::Add, changed, width),
        "an overwritten slot is a different value"
    );
}

#[test]
fn joins_drop_one_arm_facts_and_preserve_only_unchanged_aliases() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let n = terms.intern(Expr::Initial(1)).unwrap();
    let one = terms.constant(1).unwrap();
    let changed = terms.intern(Expr::Definition(5, 0)).unwrap();
    let mut left = State::default();
    left.words.insert(0, index);
    left.words.insert(1, index); // A genuine alias survives another slot's write.
    let mut right = left.clone();
    left.assume(&terms, Relation::Lt, index, n).unwrap();
    let mut missing_edge = left.clone();
    missing_edge.merge(&right, 8, &mut terms).unwrap();
    assert!(!missing_edge.safe(&terms, Arithmetic::Add, index, one));
    right.assume(&terms, Relation::Lt, index, n).unwrap();
    right.words.insert(0, changed);
    left.merge(&right, 8, &mut terms).unwrap();
    assert!(!left.safe(&terms, Arithmetic::Add, left.read(0).unwrap(), one));
    assert!(left.safe(&terms, Arithmetic::Add, left.read(1).unwrap(), one));
    assert!(
        !State::default().safe(&terms, Arithmetic::Add, index, one),
        "a new traversal starts without predecessor facts"
    );
}

#[test]
fn tag_partitions_keep_invalid_remainder_and_boolean_words_opaque() {
    let mut terms = Terms::default();
    let tag = terms.intern(Expr::Initial(0)).unwrap();
    let zero = terms.constant(0).unwrap();
    let one = terms.constant(1).unwrap();
    let raw = terms.constant(2).unwrap();
    let is_zero = terms
        .intern(Expr::Compare(Relation::Eq, tag, zero))
        .unwrap();
    let is_one = terms.intern(Expr::Compare(Relation::Eq, tag, one)).unwrap();
    let mut remainder = State::default();
    assert!(remainder.condition(&terms, is_zero, false).unwrap());
    assert_eq!(
        remainder.truth(&terms, Relation::Eq, tag, one),
        None,
        "not variant zero does not establish variant one"
    );
    assert!(remainder.condition(&terms, is_one, false).unwrap());
    assert!(!remainder.condition(&terms, is_zero, true).unwrap());
    assert!(!remainder.condition(&terms, is_one, true).unwrap());
    let mut first = State::default();
    first.assume(&terms, Relation::Eq, tag, zero).unwrap();
    assert!(!first.condition(&terms, is_one, true).unwrap());
    assert!(terms.boolean(is_zero));
    assert!(terms.boolean(one));
    assert!(!terms.boolean(raw));
    assert!(!terms.boolean(tag));
}

#[test]
fn finite_limits_decline_without_turning_unknown_into_proof() {
    let mut terms = Terms::default();
    for i in 0..MAX_TERMS {
        assert!(terms.intern(Expr::Initial(i)).is_some());
    }
    assert!(terms.intern(Expr::Initial(MAX_TERMS)).is_none());
    let mut state = State::default();
    for i in 1..=MAX_RELATIONS {
        assert!(state.assume(&terms, Relation::Lt, 0, i).is_some());
    }
    assert!(
        state
            .assume(&terms, Relation::Lt, 0, MAX_RELATIONS + 1)
            .is_none()
    );
}
