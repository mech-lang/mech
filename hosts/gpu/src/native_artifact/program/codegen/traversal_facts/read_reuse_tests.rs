use super::*;

fn loaded() -> (Terms, State, Id) {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let read = terms.intern(Expr::Definition(7, 1)).unwrap();
    terms.changes.record(
        read,
        write_changes::Value::Read {
            array: 0,
            index,
            version: write_changes::Version::Entry,
        },
    );
    let mut state = State {
        snapshots: Some(BTreeMap::new()),
        ..State::default()
    };
    state.words.extend([(0, index), (1, read)]);
    state.snapshot(7, 1);
    (terms, state, read)
}

#[test]
fn current_read_snapshot_survives_scalar_home_redefinition_but_not_memory_change() {
    let (mut terms, mut state, _) = loaded();
    let expected = Some(Snapshot { site: 7, word: 1 });
    assert_eq!(find(&[Some(state.clone())], &terms, 0, 0), expected);
    state.words.insert(1, terms.constant(99).unwrap());
    assert_eq!(find(&[Some(state.clone())], &terms, 0, 0), expected);
    assert_eq!(find(&[Some(state.clone())], &terms, 1, 0), None);
    let mut changed = state.clone();
    changed.words.insert(0, terms.constant(42).unwrap());
    assert_eq!(find(&[Some(changed)], &terms, 0, 0), None);
    state.content_version = write_changes::Version::Write(8);
    assert_eq!(find(&[Some(state)], &terms, 0, 0), None);
}

#[test]
fn read_snapshot_requires_availability_in_every_partition_and_at_join() {
    let (mut terms, state, _) = loaded();
    let mut absent = state.clone();
    absent.snapshots.as_mut().unwrap().clear();
    assert_eq!(
        find(&[Some(state.clone()), Some(absent.clone())], &terms, 0, 0),
        None
    );
    let mut joined = state.clone();
    joined.merge(&absent, 9, &mut terms).unwrap();
    assert!(joined.snapshots.unwrap().is_empty());
    let mut joined = state.clone();
    joined.merge(&state, 9, &mut terms).unwrap();
    assert_eq!(
        find(&[Some(joined)], &terms, 0, 0),
        Some(Snapshot { site: 7, word: 1 })
    );
    let mut changed = state.clone();
    changed.content_version = write_changes::Version::Write(10);
    let mut joined = state;
    joined.merge(&changed, 11, &mut terms).unwrap();
    assert_eq!(find(&[Some(joined)], &terms, 0, 0), None);
}

#[test]
fn selected_default_read_snapshot_keeps_its_original_phi_and_unsigned_cast_value() {
    let (mut terms, mut state, read) = loaded();
    state.snapshots.as_mut().unwrap().clear();
    let initial = terms.intern(Expr::Initial(9)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let condition = terms
        .intern(Expr::Compare(Relation::Lt, initial, length))
        .unwrap();
    let zero = terms.constant(0).unwrap();
    let one = terms.constant(1).unwrap();
    let phi = terms.intern(Expr::Phi(10, 2)).unwrap();
    terms.changes.record(
        phi,
        write_changes::Value::Select {
            condition,
            yes: read,
            no: zero,
        },
    );
    let cast = terms.intern(Expr::Definition(11, 3)).unwrap();
    terms
        .changes
        .record(cast, write_changes::Value::Scalar(Op::U32ToU64, vec![phi]));
    state.words.insert(3, cast);
    state.snapshot(12, 3);
    assert_eq!(find(&[Some(state.clone())], &terms, 0, 0), None);
    let mut false_path = state.clone();
    false_path.literal(&terms, condition, one, false).unwrap();
    assert_eq!(find(&[Some(false_path)], &terms, 0, 0), None);
    assert_eq!(state.literal(&terms, condition, one, true), Some(true));
    assert_eq!(
        find(&[Some(state.clone())], &terms, 0, 0),
        Some(Snapshot { site: 12, word: 3 })
    );
    // A new source traversal starts with no reaching snapshots, even when
    // scalar arguments and source sites happen to be identical.
    let fresh = State {
        words: state.words,
        snapshots: Some(BTreeMap::new()),
        ..State::default()
    };
    assert_eq!(find(&[Some(fresh)], &terms, 0, 0), None);
}

#[test]
fn conflicting_read_recipes_and_unsigned_value_changes_decline() {
    let (mut terms, state, read) = loaded();
    let index = state.read(0).unwrap();
    terms.changes.record(
        read,
        write_changes::Value::Read {
            array: 1,
            index,
            version: write_changes::Version::Entry,
        },
    );
    assert_eq!(find(&[Some(state.clone())], &terms, 0, 0), None);
    let (mut terms, mut state, read) = loaded();
    let zero = terms.constant(0).unwrap();
    let changed = terms.intern(Expr::Definition(9, 2)).unwrap();
    terms.changes.record(
        changed,
        write_changes::Value::Scalar(Op::U64BitAnd, vec![read, zero]),
    );
    state.snapshots.as_mut().unwrap().clear();
    state.words.insert(2, changed);
    state.snapshot(10, 2);
    assert_eq!(find(&[Some(state)], &terms, 0, 0), None);
}

#[test]
fn two_evaluated_default_read_values_match_without_a_selected_guard() {
    let (mut terms, mut state, first_read) = loaded();
    state.snapshots.as_mut().unwrap().clear();
    let initial = terms.intern(Expr::Initial(20)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let condition = terms
        .intern(Expr::Compare(Relation::Lt, initial, length))
        .unwrap();
    let zero = terms.constant(0).unwrap();
    let first = terms.intern(Expr::Phi(10, 2)).unwrap();
    terms.changes.record(
        first,
        write_changes::Value::Select {
            condition,
            yes: first_read,
            no: zero,
        },
    );
    state.words.insert(2, first);
    state.snapshot(12, 2);
    let second_read = terms.intern(Expr::Definition(14, 3)).unwrap();
    let index = state.read(0).unwrap();
    terms.changes.record(
        second_read,
        write_changes::Value::Read {
            array: 0,
            index,
            version: write_changes::Version::Entry,
        },
    );
    let second = terms.intern(Expr::Phi(15, 4)).unwrap();
    terms.changes.record(
        second,
        write_changes::Value::Select {
            condition,
            yes: second_read,
            no: zero,
        },
    );
    assert!(equivalent(&state, &terms, first, second));
    state.content_version = write_changes::Version::Write(16);
    assert!(
        equivalent(&state, &terms, first, second),
        "already computed scalars remain equal"
    );
    assert!(
        !state.is_read(&terms, first, 0, index),
        "old scalar is not current-memory authority"
    );
    let changed_read = terms.intern(Expr::Definition(17, 5)).unwrap();
    terms.changes.record(
        changed_read,
        write_changes::Value::Read {
            array: 0,
            index,
            version: state.content_version,
        },
    );
    let changed = terms.intern(Expr::Phi(18, 6)).unwrap();
    terms.changes.record(
        changed,
        write_changes::Value::Select {
            condition,
            yes: changed_read,
            no: zero,
        },
    );
    assert!(
        !equivalent(&state, &terms, first, changed),
        "a later write invalidates new-read equivalence"
    );
    let other = terms
        .intern(Expr::Compare(Relation::Le, initial, length))
        .unwrap();
    let different_guard = terms.intern(Expr::Phi(19, 7)).unwrap();
    terms.changes.record(
        different_guard,
        write_changes::Value::Select {
            condition: other,
            yes: second_read,
            no: zero,
        },
    );
    assert!(!equivalent(&state, &terms, first, different_guard));
}

#[test]
fn conflicting_operand_proofs_stay_ambiguous_after_later_extensions() {
    let mut first = Proof::default();
    let mut second = Proof::default();
    first
        .reused_operands
        .insert((20, 1), Snapshot { site: 10, word: 0 });
    second
        .reused_operands
        .insert((20, 1), Snapshot { site: 11, word: 0 });
    first.extend(second);
    assert_eq!(first.reused_operand(20, 1), None);
    let mut later = Proof::default();
    later
        .reused_operands
        .insert((20, 1), Snapshot { site: 10, word: 0 });
    first.extend(later);
    assert_eq!(first.reused_operand(20, 1), None);
}
