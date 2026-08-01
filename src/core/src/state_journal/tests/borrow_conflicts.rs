use super::support::{record_value, scalar, scalar_value};
use crate::{MechMap, MechSet, Ref, Value, ValueStateBorrowConflict, ValueStateJournal};
use core::any::type_name;
use std::panic::{AssertUnwindSafe, catch_unwind};

#[test]
fn state_journal_split_restore_preflights_before_apply() {
    let cell = scalar(1.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&cell)).unwrap();
    *cell.borrow_mut() = 2.0;

    let held_borrow = cell.borrow();
    let error = journal.preflight_restore_before().unwrap_err();
    assert_eq!(error.kind_name(), "ValueStateBorrowConflict");
    assert_eq!(*held_borrow, 2.0);
    drop(held_borrow);

    journal.preflight_restore_before().unwrap();
    journal.apply_restore_before();
    assert_eq!(*cell.borrow(), 1.0);
}

#[test]
fn state_journal_capture_conflict_is_structured_and_adds_nothing() {
    let cell = scalar(1.0);
    let root = scalar_value(&cell);
    let held = cell.borrow_mut();
    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&root).unwrap_err();

    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "capture-before");
    assert_eq!(conflict.type_name, type_name::<f64>());
    assert!(journal.is_empty());
    assert!(journal.roots.is_empty());
    drop(held);

    journal.capture_value(&root).unwrap();
    assert_eq!(journal.cell_count(), 1);
}

#[test]
fn state_journal_nested_capture_preflight_adds_no_partial_entries() {
    let first = scalar(1.0);
    let second = scalar(2.0);
    let (_, root) = record_value(vec![
        ("first", scalar_value(&first)),
        ("second", scalar_value(&second)),
    ]);
    let held = second.borrow_mut();
    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&root).unwrap_err();

    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "capture-before");
    assert!(journal.entries.is_empty());
    assert!(journal.entry_indices.is_empty());
    assert!(journal.roots.is_empty());
    drop(held);

    journal.capture_value(&root).unwrap();
    assert_eq!(journal.cell_count(), 3);
}

#[test]
fn state_journal_restore_preflight_is_atomic() {
    let first = scalar(1.0);
    let second = scalar(2.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&first)).unwrap();
    journal.capture_value(&scalar_value(&second)).unwrap();
    *first.borrow_mut() = 10.0;
    *second.borrow_mut() = 20.0;

    let held = second.borrow();
    let error = journal.restore_before().unwrap_err();
    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "restore-before");
    assert_eq!(*first.borrow(), 10.0);
    assert_eq!(*held, 20.0);
    drop(held);

    journal.restore_before().unwrap();
    assert_eq!(*first.borrow(), 1.0);
    assert_eq!(*second.borrow(), 2.0);
}

#[test]
fn state_journal_record_after_conflict_is_retryable() {
    let first = scalar(1.0);
    let second = scalar(2.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&first)).unwrap();
    journal.capture_value(&scalar_value(&second)).unwrap();
    *first.borrow_mut() = 10.0;
    *second.borrow_mut() = 20.0;

    let held = second.borrow_mut();
    let error = journal.record_after().unwrap_err();
    assert_eq!(
        error.kind_as::<ValueStateBorrowConflict>().unwrap().phase,
        "capture-after"
    );
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));
    drop(held);

    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    delta.rewind().unwrap();
    assert_eq!((*first.borrow(), *second.borrow()), (1.0, 2.0));
    delta.replay().unwrap();
    assert_eq!((*first.borrow(), *second.borrow()), (10.0, 20.0));
}

#[test]
fn state_journal_replay_preflight_is_atomic() {
    let first = scalar(1.0);
    let second = scalar(2.0);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&first)).unwrap();
    journal.capture_value(&scalar_value(&second)).unwrap();
    *first.borrow_mut() = 10.0;
    *second.borrow_mut() = 20.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    delta.rewind().unwrap();

    let held = second.borrow();
    let error = delta.replay().unwrap_err();
    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "restore-after");
    assert_eq!(*first.borrow(), 1.0);
    assert_eq!(*held, 2.0);
    drop(held);

    delta.replay().unwrap();
    assert_eq!((*first.borrow(), *second.borrow()), (10.0, 20.0));
}

#[test]
fn state_journal_map_key_borrow_conflict_is_structured_and_atomic() {
    let key = Ref::new(1.0);
    let map = Ref::new(MechMap::from_vec(vec![(
        Value::F64(key.clone()),
        Value::Id(1),
    )]));
    let root = Value::Map(map);
    let held = key.borrow_mut();
    let mut journal = ValueStateJournal::new();

    let result = catch_unwind(AssertUnwindSafe(|| journal.capture_value(&root)));
    assert!(result.is_ok());
    let error = result.unwrap().unwrap_err();
    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "capture-before");
    assert_eq!(conflict.type_name, type_name::<f64>());
    assert!(journal.entries.is_empty());
    assert!(journal.entry_indices.is_empty());
    assert!(journal.roots.is_empty());
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);

    drop(held);
    journal.capture_value(&root).unwrap();
}

#[test]
fn state_journal_set_element_borrow_conflict_is_structured_and_atomic() {
    let element = Ref::new(1.0);
    let set = Ref::new(MechSet::from_vec(vec![Value::F64(element.clone())]));
    let root = Value::Set(set);
    let held = element.borrow_mut();
    let mut journal = ValueStateJournal::new();

    let result = catch_unwind(AssertUnwindSafe(|| journal.capture_value(&root)));
    assert!(result.is_ok());
    let error = result.unwrap().unwrap_err();
    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "capture-before");
    assert_eq!(conflict.type_name, type_name::<f64>());
    assert!(journal.entries.is_empty());
    assert!(journal.entry_indices.is_empty());
    assert!(journal.roots.is_empty());
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);

    drop(held);
    journal.capture_value(&root).unwrap();
}

#[test]
fn state_journal_map_key_record_after_conflict_is_retryable() {
    let key = Ref::new(1.0);
    let value = Ref::new(10.0);
    let map = Ref::new(MechMap::from_vec(vec![(
        Value::F64(key.clone()),
        Value::F64(value.clone()),
    )]));
    let root = Value::Map(map);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    *value.borrow_mut() = 20.0;

    let held = key.borrow_mut();
    let result = catch_unwind(AssertUnwindSafe(|| journal.record_after()));
    assert!(result.is_ok());
    let error = result.unwrap().unwrap_err();
    let conflict = error.kind_as::<ValueStateBorrowConflict>().unwrap();
    assert_eq!(conflict.phase, "capture-after");
    assert_eq!(conflict.type_name, type_name::<f64>());
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));

    drop(held);
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    delta.rewind().unwrap();
    assert_eq!(*value.borrow(), 10.0);
    delta.replay().unwrap();
    assert_eq!(*value.borrow(), 20.0);
}
