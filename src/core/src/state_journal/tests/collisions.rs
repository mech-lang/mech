use super::support::{as_scalar, map_get_scalar, scalar, scalar_value, set_contains_scalar};
use crate::{
    MechMap, MechSet, Ref, Value, ValueKind, ValueStateCollectionCollision, ValueStateJournal,
};

#[test]
fn state_journal_set_collision_is_atomic_and_retryable() {
    let safe = scalar(10.0);
    let left = scalar(1.0);
    let right = scalar(2.0);
    let set = Ref::new(MechSet::from_vec(vec![
        scalar_value(&left),
        scalar_value(&right),
    ]));
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&safe)).unwrap();
    journal.capture_value(&Value::Set(set.clone())).unwrap();
    assert_eq!(journal.cell_count(), 4);

    *safe.borrow_mut() = 11.0;
    *right.borrow_mut() = 1.0;
    let error = journal.record_after().unwrap_err();
    let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
    assert_eq!(collision.phase, "capture-after");
    assert_eq!(collision.collection, "set element");
    assert_eq!(collision.first_index, 0);
    assert_eq!(collision.second_index, 1);
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert_eq!(journal.cell_count(), 4);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));
    assert_eq!(*safe.borrow(), 11.0);
    assert_eq!(set.borrow().kind, ValueKind::F64);
    assert_eq!(set.borrow().num_elements, 2);
    assert_eq!(
        set.borrow()
            .set
            .iter()
            .map(as_scalar)
            .map(|value| value.addr())
            .collect::<Vec<_>>(),
        vec![left.addr(), right.addr()]
    );

    journal.restore_before().unwrap();
    assert_eq!(*safe.borrow(), 10.0);
    assert_eq!((*left.borrow(), *right.borrow()), (1.0, 2.0));
    assert!(set_contains_scalar(&set, 1.0));
    assert!(set_contains_scalar(&set, 2.0));

    *safe.borrow_mut() = 12.0;
    *right.borrow_mut() = 3.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    assert_eq!(delta.cell_count(), 4);
    delta.rewind().unwrap();
    assert_eq!(*safe.borrow(), 10.0);
    assert!(set_contains_scalar(&set, 1.0));
    assert!(set_contains_scalar(&set, 2.0));
    delta.replay().unwrap();
    assert_eq!(*safe.borrow(), 12.0);
    assert!(set_contains_scalar(&set, 1.0));
    assert!(!set_contains_scalar(&set, 2.0));
    assert!(set_contains_scalar(&set, 3.0));
}

#[test]
fn state_journal_collection_collision_uses_payload_equality_not_hash_match() {
    let left = scalar(-0.0);
    let right = scalar(1.0);
    let set = Ref::new(MechSet::from_vec(vec![
        scalar_value(&left),
        scalar_value(&right),
    ]));
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&Value::Set(set.clone())).unwrap();

    *right.borrow_mut() = 0.0;
    let error = journal.record_after().unwrap_err();
    let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
    assert_eq!(collision.collection, "set element");
    assert_eq!(collision.first_index, 0);
    assert_eq!(collision.second_index, 1);
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));

    journal.restore_before().unwrap();
    assert_eq!(left.borrow().to_bits(), (-0.0f64).to_bits());
    assert_eq!(*right.borrow(), 1.0);
    assert!(set_contains_scalar(&set, -0.0));
    assert!(set_contains_scalar(&set, 1.0));
}

#[test]
fn state_journal_map_collision_is_atomic_and_retryable() {
    let safe = scalar(10.0);
    let left_key = scalar(1.0);
    let right_key = scalar(2.0);
    let left_value = scalar(10.0);
    let right_value = scalar(20.0);
    let map = Ref::new(MechMap::from_vec(vec![
        (scalar_value(&left_key), scalar_value(&left_value)),
        (scalar_value(&right_key), scalar_value(&right_value)),
    ]));
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&scalar_value(&safe)).unwrap();
    journal.capture_value(&Value::Map(map.clone())).unwrap();
    assert_eq!(journal.cell_count(), 6);

    *safe.borrow_mut() = 11.0;
    *right_key.borrow_mut() = 1.0;
    let error = journal.record_after().unwrap_err();
    let collision = error.kind_as::<ValueStateCollectionCollision>().unwrap();
    assert_eq!(collision.phase, "capture-after");
    assert_eq!(collision.collection, "map key");
    assert_eq!(collision.first_index, 0);
    assert_eq!(collision.second_index, 1);
    assert!(!journal.after_recorded);
    assert!(!journal.sealed);
    assert_eq!(journal.cell_count(), 6);
    assert!(journal.entries.iter().all(|entry| !entry.has_after()));
    assert_eq!(*safe.borrow(), 11.0);
    assert_eq!(map.borrow().key_kind, ValueKind::F64);
    assert_eq!(map.borrow().value_kind, ValueKind::F64);
    assert_eq!(map.borrow().num_elements, 2);
    assert_eq!(map.borrow().map.len(), 2);
    assert_eq!(
        map.borrow()
            .map
            .keys()
            .map(as_scalar)
            .map(|value| value.addr())
            .collect::<Vec<_>>(),
        vec![left_key.addr(), right_key.addr()]
    );
    assert_eq!(
        map.borrow()
            .map
            .values()
            .map(as_scalar)
            .map(|value| value.addr())
            .collect::<Vec<_>>(),
        vec![left_value.addr(), right_value.addr()]
    );

    journal.restore_before().unwrap();
    assert_eq!(*safe.borrow(), 10.0);
    assert_eq!((*left_key.borrow(), *right_key.borrow()), (1.0, 2.0));
    assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
    assert_eq!(
        map_get_scalar(&map, 2.0).unwrap().addr(),
        right_value.addr()
    );

    *safe.borrow_mut() = 12.0;
    *right_key.borrow_mut() = 3.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    assert_eq!(delta.cell_count(), 6);
    delta.rewind().unwrap();
    assert_eq!(*safe.borrow(), 10.0);
    assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
    assert_eq!(
        map_get_scalar(&map, 2.0).unwrap().addr(),
        right_value.addr()
    );
    delta.replay().unwrap();
    assert_eq!(*safe.borrow(), 12.0);
    assert_eq!(map_get_scalar(&map, 1.0).unwrap().addr(), left_value.addr());
    assert!(map_get_scalar(&map, 2.0).is_none());
    assert_eq!(
        map_get_scalar(&map, 3.0).unwrap().addr(),
        right_value.addr()
    );
}
