use crate::{
    LegacyValue, MechMap, MechSet, MechTuple, Ref, ValueStateHashedCycleUnsupported,
    ValueStateJournal,
};

fn assert_hashed_cycle(
    error: crate::MechError,
    phase: &'static str,
    collection: &'static str,
    index: usize,
) {
    assert_eq!(error.kind_name(), "ValueStateHashedCycleUnsupported");
    let cycle = error.kind_as::<ValueStateHashedCycleUnsupported>().unwrap();
    assert_eq!(cycle.phase, phase);
    assert_eq!(cycle.collection, collection);
    assert_eq!(cycle.index, index);
    let diagnostic = format!("{error:?}");
    assert!(diagnostic.contains(phase), "got {diagnostic}");
    assert!(diagnostic.contains(collection), "got {diagnostic}");
    assert!(
        diagnostic.contains(&format!("index {index}")),
        "got {diagnostic}"
    );
}

#[test]
fn state_journal_rejects_self_referential_set_element_before_hashing() {
    let key = Ref::new(LegacyValue::Id(1));
    let set = Ref::new(MechSet::from_vec(vec![LegacyValue::MutableReference(
        key.clone(),
    )]));
    *key.borrow_mut() = LegacyValue::MutableReference(key.clone());

    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&LegacyValue::Set(set)).unwrap_err();

    assert_hashed_cycle(error, "capture-before", "set element", 0);
}

#[test]
fn state_journal_rejects_two_node_cycle_in_set_element_before_hashing() {
    let first = Ref::new(LegacyValue::Id(1));
    let second = Ref::new(LegacyValue::MutableReference(first.clone()));
    let set = Ref::new(MechSet::from_vec(vec![LegacyValue::MutableReference(
        first.clone(),
    )]));
    *first.borrow_mut() = LegacyValue::MutableReference(second);

    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&LegacyValue::Set(set)).unwrap_err();

    assert_hashed_cycle(error, "capture-before", "set element", 0);
}

#[test]
fn state_journal_rejects_self_referential_map_key_before_hashing() {
    let key = Ref::new(LegacyValue::Id(1));
    let map = Ref::new(MechMap::from_vec(vec![(
        LegacyValue::MutableReference(key.clone()),
        LegacyValue::Id(10),
    )]));
    *key.borrow_mut() = LegacyValue::MutableReference(key.clone());

    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&LegacyValue::Map(map)).unwrap_err();

    assert_hashed_cycle(error, "capture-before", "map key", 0);
}

#[test]
fn state_journal_rejects_nested_cycle_inside_tuple_map_key() {
    let cell = Ref::new(LegacyValue::Id(1));
    let tuple = Ref::new(MechTuple::from_vec(vec![LegacyValue::MutableReference(
        cell.clone(),
    )]));
    let map = Ref::new(MechMap::from_vec(vec![(
        LegacyValue::Tuple(tuple.clone()),
        LegacyValue::Id(10),
    )]));
    *cell.borrow_mut() = LegacyValue::Tuple(tuple);

    let mut journal = ValueStateJournal::new();
    let error = journal.capture_value(&LegacyValue::Map(map)).unwrap_err();

    assert_hashed_cycle(error, "capture-before", "map key", 0);
}

#[test]
fn state_journal_allows_cycle_in_map_value() {
    let value = Ref::new(LegacyValue::Id(10));
    let map = Ref::new(MechMap::from_vec(vec![(
        LegacyValue::Id(1),
        LegacyValue::MutableReference(value.clone()),
    )]));
    *value.borrow_mut() = LegacyValue::MutableReference(value.clone());
    let mut journal = ValueStateJournal::new();

    journal.capture_value(&LegacyValue::Map(map)).unwrap();
    *value.borrow_mut() = LegacyValue::Id(20);
    journal.restore_before().unwrap();

    let restored = value.borrow();
    let LegacyValue::MutableReference(target) = &*restored else {
        panic!("expected restored mutable-reference cycle");
    };
    assert_eq!(target.addr(), value.addr());
}

#[test]
fn state_journal_hashed_cycle_capture_failure_is_atomic() {
    let valid = Ref::new(1.0);
    let key = Ref::new(LegacyValue::Id(1));
    let set = Ref::new(MechSet::from_vec(vec![LegacyValue::MutableReference(
        key.clone(),
    )]));
    *key.borrow_mut() = LegacyValue::MutableReference(key.clone());
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&LegacyValue::F64(valid)).unwrap();
    let cell_count = journal.cell_count();
    let root_count = journal.roots.len();

    let error = journal.capture_value(&LegacyValue::Set(set)).unwrap_err();

    assert_hashed_cycle(error, "capture-before", "set element", 0);
    assert_eq!(journal.cell_count(), cell_count);
    assert_eq!(journal.roots.len(), root_count);
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
}

#[test]
fn state_journal_hashed_cycle_record_after_failure_is_retryable() {
    let key = Ref::new(LegacyValue::Id(1));
    let map = Ref::new(MechMap::from_vec(vec![(
        LegacyValue::MutableReference(key.clone()),
        LegacyValue::Id(10),
    )]));
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&LegacyValue::Map(map)).unwrap();

    *key.borrow_mut() = LegacyValue::MutableReference(key.clone());
    let error = journal.record_after().unwrap_err();
    assert_hashed_cycle(error, "capture-after", "map key", 0);
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));

    *key.borrow_mut() = LegacyValue::Id(2);
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    delta.rewind().unwrap();
    assert!(matches!(&*key.borrow(), LegacyValue::Id(1)));
    delta.replay().unwrap();
    assert!(matches!(&*key.borrow(), LegacyValue::Id(2)));
}

#[test]
fn state_journal_hashed_cycle_reports_capture_after_phase() {
    let key = Ref::new(LegacyValue::Id(1));
    let set = Ref::new(MechSet::from_vec(vec![LegacyValue::MutableReference(
        key.clone(),
    )]));
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&LegacyValue::Set(set)).unwrap();
    *key.borrow_mut() = LegacyValue::MutableReference(key.clone());

    let error = journal.record_after().unwrap_err();

    assert_hashed_cycle(error, "capture-after", "set element", 0);
}
