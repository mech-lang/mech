use super::*;

fn read(terms: &mut Terms, array: usize, index: Id, version: Version) -> Id {
    let id = terms
        .intern(Expr::Definition(terms.terms.len(), 0))
        .unwrap();
    terms.changes.record(
        id,
        Value::Read {
            array,
            index,
            version,
        },
    );
    id
}
fn scalar(terms: &mut Terms, op: Op, args: &[Id]) -> Id {
    let id = terms
        .intern(Expr::Definition(terms.terms.len(), 0))
        .unwrap();
    terms.changes.record(id, Value::Scalar(op, args.to_vec()));
    id
}
fn wrapping(terms: &mut Terms, old: Id, delta: u64, width: u8) -> Id {
    let delta = terms.constant(delta).unwrap();
    let widened = match width {
        1 => scalar(terms, Op::U8ToU64, &[old]),
        2 => scalar(terms, Op::U16ToU64, &[old]),
        4 => scalar(terms, Op::U32ToU64, &[old]),
        8 => old,
        _ => unreachable!(),
    };
    let sum = scalar(terms, Op::U64WrappingAdd, &[widened, delta]);
    let mask = terms
        .constant(if width == 8 {
            u64::MAX
        } else {
            (1u64 << (width * 8)) - 1
        })
        .unwrap();
    let masked = scalar(terms, Op::U64BitAnd, &[sum, mask]);
    match width {
        1 => scalar(terms, Op::U64ToU8, &[masked]),
        2 => scalar(terms, Op::U64ToU16, &[masked]),
        4 => scalar(terms, Op::U64ToU32, &[masked]),
        8 => masked,
        _ => unreachable!(),
    }
}

#[test]
fn nonzero_modular_delta_proves_change_at_all_unsigned_widths() {
    for width in [1, 2, 4, 8] {
        let mut terms = Terms::default();
        let index = terms.intern(Expr::Initial(0)).unwrap();
        let old = read(&mut terms, 0, index, Version::Entry);
        let state = State::default();
        for delta in [0, 1, 2, 255, 256, 65536, 1u64 << 32, u64::MAX] {
            let next = wrapping(&mut terms, old, delta, width);
            let mask = if width == 8 {
                u64::MAX
            } else {
                (1u64 << (width * 8)) - 1
            };
            assert_eq!(
                state.changes(&terms, 0, index, next, width),
                delta & mask != 0
            );
        }
    }
}

#[test]
fn successful_default_read_is_recovered_only_for_the_exact_write_index() {
    let mut terms = Terms::default();
    let at = terms.intern(Expr::Initial(0)).unwrap();
    let other = terms.intern(Expr::Initial(1)).unwrap();
    let length = terms.intern(Expr::ArrayLength(0)).unwrap();
    let one = terms.constant(1).unwrap();
    let zero = terms.constant(0).unwrap();
    let guard = terms
        .intern(Expr::Compare(Relation::Lt, at, length))
        .unwrap();
    let read_index = terms
        .intern(Expr::Arithmetic(Arithmetic::Add, at, one))
        .unwrap();
    let index = terms
        .intern(Expr::Wrapping(Arithmetic::Add, at, one))
        .unwrap();
    let old = read(&mut terms, 0, read_index, Version::Entry);
    let selected = terms.intern(Expr::Phi(100, 0)).unwrap();
    terms.changes.record(
        selected,
        Value::Select {
            condition: guard,
            yes: old,
            no: zero,
        },
    );
    let next = wrapping(&mut terms, selected, 1, 4);
    let state = State::default();
    assert!(state.changes(&terms, 0, index, next, 4));
    assert!(!state.changes(&terms, 1, index, next, 4));
    assert!(!state.changes(&terms, 0, other, next, 4));
    let different_length = terms.intern(Expr::ArrayLength(1)).unwrap();
    let guard = terms
        .intern(Expr::Compare(Relation::Lt, at, different_length))
        .unwrap();
    let selected = terms.intern(Expr::Phi(101, 0)).unwrap();
    terms.changes.record(
        selected,
        Value::Select {
            condition: guard,
            yes: old,
            no: zero,
        },
    );
    let next = wrapping(&mut terms, selected, 1, 4);
    assert!(!state.changes(&terms, 0, index, next, 4));
}

#[test]
fn an_intervening_write_or_join_invalidates_current_content_even_through_an_alias() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let old = read(&mut terms, 0, index, Version::Entry);
    let next = wrapping(&mut terms, old, 1, 4);
    let mut state = State::default();
    assert!(state.changes(&terms, 0, index, next, 4));
    state.content_version = Version::Write(10);
    assert!(!state.changes(&terms, 0, index, next, 4));
    let fresh = read(&mut terms, 0, index, state.content_version);
    let next = wrapping(&mut terms, fresh, 1, 4);
    assert!(state.changes(&terms, 0, index, next, 4));
    let mut other = state.clone();
    other.content_version = Version::Entry;
    state.merge(&other, 30, &mut terms).unwrap();
    assert_eq!(state.content_version, Version::Join(30));
    assert!(!state.changes(&terms, 0, index, next, 4));
}

#[test]
fn zero_delta_partial_masks_unknown_values_and_ambiguous_recipes_keep_comparison() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let old = read(&mut terms, 0, index, Version::Entry);
    let one = terms.constant(1).unwrap();
    let state = State::default();
    assert!(!state.changes(&terms, 0, index, old, 4));
    let unknown = terms.intern(Expr::Initial(1)).unwrap();
    let next = scalar(&mut terms, Op::U64WrappingAdd, &[old, unknown]);
    assert!(!state.changes(&terms, 0, index, next, 4));
    let next = scalar(&mut terms, Op::U64WrappingAdd, &[old, one]);
    let mask = terms.constant(255).unwrap();
    let masked = scalar(&mut terms, Op::U64BitAnd, &[next, mask]);
    assert!(!state.changes(&terms, 0, index, masked, 4));
    terms
        .changes
        .record(next, Value::Scalar(Op::U64WrappingSub, vec![old, one]));
    assert!(!state.changes(&terms, 0, index, next, 4));
}

#[test]
fn shared_expression_dags_are_processed_once_without_a_depth_limit() {
    let mut terms = Terms::default();
    let index = terms.intern(Expr::Initial(0)).unwrap();
    let old = read(&mut terms, 0, index, Version::Entry);
    let one = terms.constant(1).unwrap();
    let mut delta = one;
    for _ in 0..96 {
        delta = scalar(&mut terms, Op::U64WrappingAdd, &[delta, delta]);
    }
    let zero = scalar(&mut terms, Op::U64WrappingAdd, &[old, delta]);
    assert!(!State::default().changes(&terms, 0, index, zero, 8));
    let next = scalar(&mut terms, Op::U64WrappingAdd, &[zero, one]);
    assert!(State::default().changes(&terms, 0, index, next, 8));
}
