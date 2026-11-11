use super::*;

#[test]
fn checked_increment_requires_current_cell_surviving_read_and_successful_addition() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let other = terms.intern(Expr::Initial(1)).unwrap();
    let old = terms.intern(Expr::Definition(10, 2)).unwrap();
    let one = terms.constant(1).unwrap();
    let added = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, old, one))
        .unwrap();
    let wrapped = terms
        .intern(Expr::Wrapping(Arithmetic::Add, old, one))
        .unwrap();
    let mut state = State::default();
    state
        .words
        .extend([(0, index), (1, other), (2, old), (3, index), (4, added)]);
    state.remember_read(0, index, 10, 2, old).unwrap();
    assert!(state.checked_increment(&terms, 0, 3, 4).is_none());
    state.successful.insert((Arithmetic::Add, old, one));
    assert_eq!(
        state.checked_increment(&terms, 0, 3, 4).unwrap().read_site,
        10
    );
    assert!(state.checked_increment(&terms, 1, 3, 4).is_none());
    assert!(state.checked_increment(&terms, 0, 1, 4).is_none());
    state.words.insert(4, wrapped);
    assert!(state.checked_increment(&terms, 0, 3, 4).is_none());
    state.words.insert(4, old);
    assert!(state.checked_increment(&terms, 0, 3, 4).is_none());
    state.words.insert(4, added);
    state
        .words
        .insert(2, terms.intern(Expr::Definition(11, 2)).unwrap());
    assert!(state.checked_increment(&terms, 0, 3, 4).is_none());
}

#[test]
fn checked_increment_merges_require_the_same_checked_read_on_every_path() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let old = terms.intern(Expr::Definition(10, 1)).unwrap();
    let one = terms.constant(1).unwrap();
    let added = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, old, one))
        .unwrap();
    let mut initial = State::default();
    initial.words.extend([(0, index), (1, old), (2, added)]);
    initial.successful.insert((Arithmetic::Add, old, one));
    initial.remember_read(0, index, 10, 1, old).unwrap();
    let mut same = initial.clone();
    same.merge(&initial, 20, &mut terms).unwrap();
    assert!(same.checked_increment(&terms, 0, 0, 2).is_some());
    for different_read in [false, true] {
        let mut other = initial.clone();
        other.checked_reads.clear();
        if different_read {
            other.remember_read(0, index, 12, 1, old).unwrap();
        }
        let mut joined = initial.clone();
        joined.merge(&other, 20, &mut terms).unwrap();
        assert!(joined.checked_increment(&terms, 0, 0, 2).is_none());
    }
}

#[test]
fn selected_normalized_booleans_retain_only_logically_forced_operands() {
    let mut terms = Terms::default();
    let a = terms.intern(Expr::Initial(0)).unwrap();
    let b = terms.intern(Expr::Initial(1)).unwrap();
    let c = terms.intern(Expr::Initial(2)).unwrap();
    let p = terms.intern(Expr::Compare(Relation::Le, a, b)).unwrap();
    let q = terms.intern(Expr::Compare(Relation::Lt, b, c)).unwrap();
    let and = terms.intern(Expr::And(p, q)).unwrap();
    let or = terms.intern(Expr::Or(p, q)).unwrap();
    assert!(terms.boolean(and) && terms.boolean(or));
    assert!(!terms.boolean(a));
    for (predicate, selected, forced) in [
        (and, true, Some(true)),
        (and, false, None),
        (or, true, None),
        (or, false, Some(false)),
    ] {
        let mut state = State::default();
        assert!(state.condition(&terms, predicate, selected).unwrap());
        assert_eq!(state.truth(&terms, Relation::Le, a, b), forced);
        assert_eq!(state.truth(&terms, Relation::Lt, b, c), forced);
    }
    let not = terms.intern(Expr::Not(and)).unwrap();
    let repeated = terms.intern(Expr::Or(not, not)).unwrap();
    let mut selected = State::default();
    assert!(selected.condition(&terms, repeated, false).unwrap());
    assert_eq!(selected.truth(&terms, Relation::Le, a, b), Some(true));
    assert_eq!(selected.truth(&terms, Relation::Lt, b, c), Some(true));
    let opaque = terms.intern(Expr::Boolean(10, 20)).unwrap();
    let mut unknown = State::default();
    assert!(unknown.condition(&terms, opaque, true).unwrap());
    assert!(unknown.relations.is_empty());
}

#[test]
fn smaller_sum_windows_require_selected_checked_bounds_and_current_views() {
    let mut terms = Terms::default();
    let left = terms.intern(Expr::Initial(0)).unwrap();
    let right = terms.intern(Expr::Initial(1)).unwrap();
    let maximum = terms.intern(Expr::Initial(2)).unwrap();
    let small = terms.intern(Expr::Initial(3)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let sum = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, left, maximum))
        .unwrap();
    let window = terms
        .intern(Expr::Compare(Relation::Le, sum, length))
        .unwrap();
    let ordered = terms
        .intern(Expr::Compare(Relation::Lt, right, left))
        .unwrap();
    let both = terms.intern(Expr::And(window, ordered)).unwrap();
    let mut initial = State::default();
    initial.words.extend([(0, left), (1, right), (2, small)]);
    initial
        .assume(&terms, Relation::Le, small, maximum)
        .unwrap();
    let fits = Check::Fits {
        array: 0,
        base: Some(1),
        count: 2,
        extra: 0,
    };
    for selected in [false, true] {
        let mut state = initial.clone();
        assert!(state.condition(&terms, both, selected).unwrap());
        assert_eq!(fits.holds(&state, &mut terms), Some(selected));
        assert_eq!(
            Check::Le(Operand::Word(1), Operand::Length(0)).holds(&state, &mut terms),
            Some(selected)
        );
    }
    let mut state = initial.clone();
    assert!(state.condition(&terms, both, true).unwrap());
    assert_eq!(
        Check::Fits {
            array: 1,
            base: Some(1),
            count: 2,
            extra: 0,
        }
        .holds(&state, &mut terms),
        Some(false)
    );
    state
        .words
        .insert(1, terms.intern(Expr::Definition(7, 1)).unwrap());
    assert_eq!(fits.holds(&state, &mut terms), Some(false));
    state.words.insert(1, right);
    state.words.insert(2, terms.constant(u64::MAX).unwrap());
    assert_eq!(fits.holds(&state, &mut terms), Some(false));
    let ring = terms
        .intern(Expr::Wrapping(Arithmetic::Add, left, maximum))
        .unwrap();
    initial.assume(&terms, Relation::Le, ring, length).unwrap();
    initial.assume(&terms, Relation::Lt, right, left).unwrap();
    assert_eq!(fits.holds(&initial, &mut terms), Some(false));
}

#[test]
fn child_checks_use_checked_sum_order_but_not_wrapping_or_fresh_words() {
    let mut terms = Terms::default();
    let start = terms.intern(Expr::Initial(0)).unwrap();
    let width = terms.constant(32).unwrap();
    let end = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, start, width))
        .unwrap();
    let wrapped = terms
        .intern(Expr::Wrapping(Arithmetic::Add, start, width))
        .unwrap();
    let mut state = State::default();
    state.words.extend([(0, start), (1, end)]);
    state.successful.insert((Arithmetic::Add, start, width));
    let ordered = Check::Le(Operand::Word(0), Operand::Word(1));
    let long = Check::Le(Operand::Constant(8), Operand::Word(1));
    assert_eq!(ordered.holds(&state, &mut terms), Some(true));
    assert_eq!(long.holds(&state, &mut terms), Some(true));
    assert_eq!(
        Check::Le(Operand::Constant(33), Operand::Word(1)).holds(&state, &mut terms),
        Some(false)
    );
    state.words.insert(1, wrapped);
    assert_eq!(ordered.holds(&state, &mut terms), Some(false));
    assert_eq!(long.holds(&state, &mut terms), Some(false));
    state
        .words
        .insert(1, terms.intern(Expr::Initial(1)).unwrap());
    assert_eq!(ordered.holds(&state, &mut terms), Some(false));
    assert_eq!(long.holds(&state, &mut terms), Some(false));
}

#[test]
fn child_window_checks_keep_exact_views_current_words_and_checked_extents() {
    let mut terms = Terms::default();
    let count = terms.intern(Expr::Initial(0)).unwrap();
    let three = terms.constant(3).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let end = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, count, three))
        .unwrap();
    let mut state = State::default();
    state.words.insert(0, count);
    state.successful.insert((Arithmetic::Add, count, three));
    state.assume(&terms, Relation::Le, end, length).unwrap();
    let exact = Check::Fits {
        array: 0,
        base: None,
        count: 0,
        extra: 3,
    };
    assert_eq!(exact.holds(&state, &mut terms), Some(true));
    assert_eq!(
        Check::Fits {
            array: 1,
            base: None,
            count: 0,
            extra: 3
        }
        .holds(&state, &mut terms),
        Some(false)
    );
    assert_eq!(
        Check::Fits {
            array: 0,
            base: None,
            count: 0,
            extra: 4
        }
        .holds(&state, &mut terms),
        Some(false)
    );
    let mut fresh = State::default();
    fresh.words.insert(0, count);
    assert_eq!(exact.holds(&fresh, &mut terms), Some(false));
    state
        .words
        .insert(0, terms.intern(Expr::Definition(7, 0)).unwrap());
    assert_eq!(exact.holds(&state, &mut terms), Some(false));
    state.words.insert(0, terms.constant(u64::MAX).unwrap());
    assert_eq!(
        Check::Addition { word: 0, extra: 3 }.holds(&state, &mut terms),
        Some(false)
    );
    assert_eq!(exact.holds(&state, &mut terms), Some(false));
}

#[test]
fn whole_windows_need_the_current_selected_interval_and_exact_views() {
    let mut terms = Terms::default();
    let k = terms.intern(Expr::Initial(0)).unwrap();
    let count = terms.intern(Expr::Initial(1)).unwrap();
    let other = terms.intern(Expr::Initial(2)).unwrap();
    let three = terms.constant(3).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let nodes = terms.intern(Expr::ArrayLength(1)).unwrap();
    let at = terms
        .intern(Expr::Wrapping(Arithmetic::Add, k, three))
        .unwrap();
    let back = terms
        .intern(Expr::Wrapping(Arithmetic::Sub, nodes, at))
        .unwrap();
    let mut state = State::default();
    state.windows = [false, true]
        .into_iter()
        .map(|reverse| Window {
            induction: 0,
            count: 1,
            array: usize::from(reverse),
            delta: 3,
            reverse,
        })
        .collect();
    assert_eq!(state.truth(&terms, Relation::Lt, at, length), None);
    let mut changed = state.clone();
    changed.assume(&terms, Relation::Lt, k, other).unwrap();
    assert_eq!(changed.truth(&terms, Relation::Lt, at, length), None);
    state.assume(&terms, Relation::Lt, k, count).unwrap();
    assert_eq!(state.truth(&terms, Relation::Lt, at, length), Some(true));
    assert_eq!(state.truth(&terms, Relation::Le, length, at), Some(false));
    assert_eq!(state.truth(&terms, Relation::Eq, at, length), Some(false));
    assert_eq!(state.truth(&terms, Relation::Ne, at, length), Some(true));
    assert_eq!(state.truth(&terms, Relation::Lt, back, nodes), Some(true));
    assert_eq!(state.truth(&terms, Relation::Lt, at, nodes), None);
    let new_at = terms
        .intern(Expr::Wrapping(Arithmetic::Add, other, three))
        .unwrap();
    assert_eq!(state.truth(&terms, Relation::Lt, new_at, length), None);
    assert_eq!(
        State::default().truth(&terms, Relation::Lt, at, length),
        None
    );
}

#[test]
fn zero_offset_windows_need_selected_current_bounds_and_mathematical_reads() {
    let mut terms = Terms::default();
    let k = terms.intern(Expr::Initial(0)).unwrap();
    let count = terms.intern(Expr::Initial(1)).unwrap();
    let other = terms.intern(Expr::Initial(2)).unwrap();
    let one = terms.constant(1).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let wrong_view = terms.intern(Expr::ArrayLength(1)).unwrap();
    let index = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, k, one))
        .unwrap();
    let ring = terms
        .intern(Expr::Wrapping(Arithmetic::Add, k, one))
        .unwrap();
    let fresh = terms.intern(Expr::Definition(7, 0)).unwrap();
    let changed_index = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, fresh, one))
        .unwrap();
    let mut state = State::default();
    assert_eq!(state.discover_window(&terms, k, length), None);
    state.assume(&terms, Relation::Lt, k, count).unwrap();
    let invalid_reverse = terms
        .intern(Expr::Wrapping(Arithmetic::Sub, length, k))
        .unwrap();
    assert_eq!(state.discover_window(&terms, invalid_reverse, length), None);
    let window = state.discover_window(&terms, k, length).unwrap();
    assert_eq!(
        window,
        Window {
            induction: 0,
            count: 1,
            array: 0,
            delta: 0,
            reverse: false
        }
    );
    assert_eq!(window.adjustment(), 0);
    assert!(!state.readable(&terms, index, length));
    state.windows.push(window);
    assert!(state.readable(&terms, index, length));
    assert!(!state.readable(&terms, index, wrong_view));
    assert!(!state.readable(&terms, ring, length));
    assert!(!state.readable(&terms, changed_index, length));
    let mut unselected = State::default();
    unselected.windows.push(window);
    assert!(!unselected.readable(&terms, index, length));
    unselected.assume(&terms, Relation::Lt, k, other).unwrap();
    assert!(!unselected.readable(&terms, index, length));
}

#[test]
fn whole_window_admission_implies_initialized_one_based_indices() {
    let check = |count: u64, delta: u64, length: u64, reverse: bool, k: u64| {
        let Some(end) = count.checked_add(delta) else {
            return;
        };
        let required = end - u64::from(reverse);
        if length < required || k >= count {
            return;
        }
        let forward = k.wrapping_add(delta);
        let at = if reverse {
            length.wrapping_sub(forward)
        } else {
            forward
        };
        assert!(
            at < length,
            "count={count} delta={delta} length={length} k={k} reverse={reverse}"
        );
        let index = at.wrapping_add(1);
        assert!(index != 0 && index <= length);
    };
    for reverse in [false, true] {
        for count in 0..16 {
            for delta in 0..8 {
                if reverse && delta == 0 {
                    continue;
                }
                for length in 0..40 {
                    for k in 0..count {
                        check(count, delta, length, reverse, k);
                    }
                }
            }
        }
        for delta in [0, 1, 3, 258, u64::MAX] {
            if reverse && delta == 0 {
                continue;
            }
            for count in [0, 1, u64::MAX - delta, u64::MAX] {
                for length in [0, 1, u64::MAX - 1, u64::MAX] {
                    check(count, delta, length, reverse, 0);
                    if count > 0 {
                        check(count, delta, length, reverse, count - 1);
                    }
                }
            }
        }
    }
}

#[test]
fn current_array_windows_follow_selected_literals_aliases_and_word_versions() {
    let mut terms = Terms::default();
    let at = terms.intern(Expr::Initial(0)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let other_length = terms.intern(Expr::ArrayLength(1)).unwrap();
    let one = terms.constant(1).unwrap();
    let predicate = terms
        .intern(Expr::Compare(Relation::Lt, at, length))
        .unwrap();
    let index = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, at, one))
        .unwrap();
    let mut selected = State::default();
    assert!(selected.literal(&terms, predicate, one, true).unwrap());
    assert!(selected.safe(&terms, Arithmetic::Add, at, one));
    assert!(selected.readable(&terms, index, length));
    assert!(
        !selected.readable(&terms, index, other_length),
        "equal kinds do not identify a view"
    );
    selected.words.insert(0, index);
    selected.assign(&[(1, 0)], true).unwrap();
    let changed = terms.intern(Expr::Definition(7, 0)).unwrap();
    selected.words.insert(0, changed);
    assert!(!selected.readable(&terms, selected.read(0).unwrap(), length));
    assert!(
        selected.readable(&terms, selected.read(1).unwrap(), length),
        "an unchanged alias keeps its own current expression"
    );
    let mut unselected = State::default();
    assert!(unselected.literal(&terms, predicate, one, false).unwrap());
    assert!(!unselected.readable(&terms, index, length));
    let mut joined = selected.clone();
    joined.merge(&State::default(), 8, &mut terms).unwrap();
    assert!(
        !joined.readable(&terms, index, length),
        "a one-arm fact cannot cross the join"
    );
    assert!(
        !State::default().readable(&terms, index, length),
        "a fresh visit starts without old window facts"
    );
}

#[test]
fn literal_bool_propagation_keeps_raw_remainder_and_overflow_unknown() {
    let mut terms = Terms::default();
    let raw = terms.intern(Expr::Initial(0)).unwrap();
    let zero = terms.constant(0).unwrap();
    let one = terms.constant(1).unwrap();
    let maximum = terms.constant(u64::MAX).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let mut state = State::default();
    assert!(state.literal(&terms, raw, one, false).unwrap());
    assert_eq!(state.truth(&terms, Relation::Eq, raw, zero), None);
    assert!(!state.safe(&terms, Arithmetic::Add, maximum, one));
    assert!(!state.readable(&terms, maximum, length));
    let predicate = terms
        .intern(Expr::Compare(Relation::Lt, raw, length))
        .unwrap();
    let mut selected_false = State::default();
    assert!(
        selected_false
            .literal(&terms, zero, predicate, true)
            .unwrap()
    );
    assert_eq!(
        selected_false.truth(&terms, Relation::Le, length, raw),
        Some(true)
    );
    assert_eq!(
        selected_false.truth(&terms, Relation::Lt, raw, length),
        Some(false)
    );
}

#[test]
fn overlapping_assignments_keep_current_tag_versions_in_the_emitted_order() {
    let mut terms = Terms::default();
    let zero = terms.constant(0).unwrap();
    let one = terms.constant(1).unwrap();
    let mut initial = State::default();
    initial.words.extend([(0, zero), (1, one), (2, zero)]);
    let moves = [(0, 1), (1, 0)];
    let mut packed = initial.clone();
    packed.assign(&moves, false).unwrap();
    let mut copied = initial.clone();
    copied.assign(&moves, true).unwrap();
    assert_eq!(
        packed.truth(&terms, Relation::Eq, packed.read(1).unwrap(), one),
        Some(true)
    );
    assert_eq!(
        copied.truth(&terms, Relation::Eq, copied.read(1).unwrap(), zero),
        Some(true)
    );
    assert_eq!(
        packed.truth(&terms, Relation::Eq, packed.read(1).unwrap(), zero),
        Some(false)
    );
    assert_eq!(
        copied.read(2),
        Some(zero),
        "an unmodified alias retains its version"
    );
    assert_eq!(
        initial.read(0),
        Some(zero),
        "a fresh traversal has no previous assignments"
    );
}

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

#[test]
fn checked_match_subtraction_retains_its_selected_interval() {
    let mut terms = Terms::default();
    let value = terms.intern(Expr::Bounded(1, 0, 65535)).unwrap();
    let three = terms.constant(3).unwrap();
    let maximum = terms.constant(258).unwrap();
    let two = terms.constant(2).unwrap();
    let mut state = State::default();
    state.assume(&terms, Relation::Le, three, value).unwrap();
    state.assume(&terms, Relation::Le, value, maximum).unwrap();
    let count = terms
        .intern(Expr::Arithmetic(Arithmetic::Sub, value, two))
        .unwrap();
    assert_eq!(
        (state.lower(&terms, count), state.upper(&terms, count)),
        (1, 256)
    );
    state.words.insert(0, count);
    assert_eq!(
        Check::Addition { word: 0, extra: 3 }.holds(&state, &mut terms),
        Some(true)
    );
    let ring = terms
        .intern(Expr::Wrapping(Arithmetic::Sub, value, two))
        .unwrap();
    assert_eq!(
        (state.lower(&terms, ring), state.upper(&terms, ring)),
        (0, u64::MAX)
    );
    assert_eq!(State::default().upper(&terms, count), 65533);
}

#[test]
fn phase_floor_proves_only_the_exact_current_window() {
    let mut terms = Terms::default();
    let count = terms.intern(Expr::Bounded(1, 0, 256)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let floor = terms.constant(259).unwrap();
    let mut state = State::default();
    state.words.insert(0, count);
    state.assume(&terms, Relation::Le, floor, length).unwrap();
    assert_eq!(state.lower(&terms, length), 259);
    assert_eq!(
        Check::Fits {
            array: 0,
            base: None,
            count: 0,
            extra: 3
        }
        .holds(&state, &mut terms),
        Some(true)
    );
    assert_eq!(
        Check::Fits {
            array: 0,
            base: None,
            count: 0,
            extra: 4
        }
        .holds(&state, &mut terms),
        Some(false)
    );
    let three = terms.constant(3).unwrap();
    let required = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, count, three))
        .unwrap();
    assert_eq!(state.upper(&terms, required), 259);
    let other_length = terms.intern(Expr::ArrayLength(1)).unwrap();
    assert_eq!(state.lower(&terms, other_length), 0);
    let mut missing = state.clone();
    missing.merge(&State::default(), 8, &mut terms).unwrap();
    assert_eq!(missing.lower(&terms, length), 0);
    state
        .words
        .insert(0, terms.intern(Expr::Initial(0)).unwrap());
    assert_eq!(
        Check::Addition { word: 0, extra: 3 }.holds(&state, &mut terms),
        Some(false)
    );
}

#[test]
fn checked_interval_edges_saturate_conservatively_without_ring_proofs() {
    let mut terms = Terms::default();
    let low = terms.intern(Expr::Initial(0)).unwrap();
    let high = terms.constant(u64::MAX).unwrap();
    let one = terms.constant(1).unwrap();
    let mut state = State::default();
    state.assume(&terms, Relation::Lt, one, low).unwrap();
    assert_eq!(state.lower(&terms, low), 2);
    let sum = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, low, high))
        .unwrap();
    assert_eq!(state.upper(&terms, sum), u64::MAX);
    assert!(!state.safe(&terms, Arithmetic::Add, low, high));
    let difference = terms
        .intern(Expr::Arithmetic(Arithmetic::Sub, low, high))
        .unwrap();
    assert_eq!(state.lower(&terms, difference), 0);
    let changed = terms.intern(Expr::Definition(9, 0)).unwrap();
    assert_eq!(state.lower(&terms, changed), 0);
}

#[test]
fn shared_and_deep_checked_arithmetic_use_the_bounded_term_dag_once() {
    let mut terms = Terms::default();
    let mut shared = terms.intern(Expr::Bounded(1, 0, 1)).unwrap();
    for _ in 0..96 {
        shared = terms
            .intern(Expr::Arithmetic(Arithmetic::Add, shared, shared))
            .unwrap();
    }
    let state = State::default();
    assert_eq!(state.upper(&terms, shared), u64::MAX);
    assert_eq!(state.lower(&terms, shared), 0);
    let one = terms.constant(1).unwrap();
    let mut deep = terms.intern(Expr::Bounded(2, 0, 258)).unwrap();
    for _ in 0..64 {
        deep = terms
            .intern(Expr::Arithmetic(Arithmetic::Sub, deep, one))
            .unwrap();
    }
    assert_eq!(state.upper(&terms, deep), 194);
}

#[test]
fn masked_one_based_access_uses_only_its_current_guarded_length_floor() {
    let mut terms = Terms::default();
    let one = terms.constant(1).unwrap();
    let floor = terms.constant(16).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let other_length = terms.intern(Expr::ArrayLength(1)).unwrap();
    let base = terms.intern(Expr::Bounded(5, 6, 15)).unwrap();
    let index = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, base, one))
        .unwrap();
    let unknown = State::default();
    assert!(!unknown.readable(&terms, index, length));
    let mut state = State::default();
    state.assume(&terms, Relation::Le, floor, length).unwrap();
    assert!(state.readable(&terms, index, length));
    assert!(!state.readable(&terms, index, other_length));
    let too_high = terms.intern(Expr::Bounded(7, 8, 16)).unwrap();
    let outside = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, too_high, one))
        .unwrap();
    assert!(!state.readable(&terms, outside, length));
    let wrapped = terms
        .intern(Expr::Wrapping(Arithmetic::Add, base, one))
        .unwrap();
    assert!(!state.readable(&terms, wrapped, length));
    let zero = terms.constant(0).unwrap();
    assert!(!state.readable(&terms, zero, length));
}

#[test]
fn masked_products_bound_indices_without_assuming_wrapping_products_fit() {
    use crate::native_artifact::program::prepare::word_collections::bounds::Range;
    for max in [0, 1, 7, 32767, 65535, u64::MAX] {
        for factor in [0, 1, 2, u64::MAX] {
            let bound = Range(0, max).wrapping_mul(Range(factor, factor));
            for value in [0, max / 2, max] {
                assert!(value.wrapping_mul(factor) <= bound.1);
            }
            if max.checked_mul(factor).is_none() {
                assert_eq!(bound.1, u64::MAX);
            }
        }
    }
    assert_eq!(Range(0, 65535).wrapping_mul(Range(2, 2)), Range(0, 131070));
}
