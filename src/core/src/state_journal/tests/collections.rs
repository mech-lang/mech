use super::support::{
    as_scalar, map_get_scalar, record_value, scalar, scalar_payload, scalar_value,
    set_contains_scalar,
};
use crate::structures::matrix::Matrix as ValueMatrix;
use crate::{MechMap, MechSet, MechTable, Ref, Value, ValueKind, ValueStateJournal, hash_str};
use indexmap::{IndexMap, IndexSet};
use nalgebra::{DMatrix, DVector};
use std::collections::HashMap;

fn scalar_set_value(value: f64) -> Value {
    let member = scalar(value);
    Value::Set(Ref::new(MechSet::from_vec(vec![scalar_value(&member)])))
}

fn map_contains_scalar_set_key(map: &Ref<MechMap>, value: f64) -> bool {
    map.borrow().map.contains_key(&scalar_set_value(value))
}

#[test]
fn state_journal_record_delta_restores_order_and_removed_or_new_cells() {
    let old = scalar(1.0);
    let retained = scalar(2.0);
    let new = scalar(3.0);
    let old_id = hash_str("old");
    let retained_id = hash_str("retained");
    let new_id = hash_str("new");
    let (record, root) = record_value(vec![
        ("old", scalar_value(&old)),
        ("retained", scalar_value(&retained)),
    ]);

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut record = record.borrow_mut();
        record.data.shift_remove(&old_id);
        record.field_names.remove(&old_id);
        record.kinds.remove(0);
        record.data.insert(new_id, scalar_value(&new));
        record.field_names.insert(new_id, "new".to_string());
        record.kinds.push(ValueKind::F64);
        record.cols = 2;
    }
    *retained.borrow_mut() = 20.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    {
        let record = record.borrow();
        assert_eq!(
            record.data.keys().copied().collect::<Vec<_>>(),
            vec![old_id, retained_id]
        );
        assert_eq!(record.cols, 2);
        assert_eq!(record.kinds, vec![ValueKind::F64, ValueKind::F64]);
        assert_eq!(record.field_names.get(&old_id).unwrap(), "old");
        assert_eq!(
            as_scalar(record.data.get(&old_id).unwrap()).addr(),
            old.addr()
        );
        assert!(!record.data.contains_key(&new_id));
    }
    assert_eq!(*old.borrow(), 1.0);
    assert_eq!(*retained.borrow(), 2.0);

    *new.borrow_mut() = 30.0;
    delta.replay().unwrap();
    {
        let record = record.borrow();
        assert_eq!(
            record.data.keys().copied().collect::<Vec<_>>(),
            vec![retained_id, new_id]
        );
        assert!(!record.data.contains_key(&old_id));
        assert_eq!(
            as_scalar(record.data.get(&new_id).unwrap()).addr(),
            new.addr()
        );
    }
    assert_eq!(*retained.borrow(), 20.0);
    assert_eq!(*new.borrow(), 3.0);
}

#[test]
fn state_journal_map_restores_metadata_order_and_retained_value() {
    let removed = scalar(1.0);
    let retained = scalar(2.0);
    let added = scalar(3.0);
    let mut data = IndexMap::new();
    data.insert(Value::Id(1), scalar_value(&removed));
    data.insert(Value::Id(2), scalar_value(&retained));
    let map = Ref::new(MechMap {
        key_kind: ValueKind::Id,
        value_kind: ValueKind::F64,
        num_elements: 2,
        map: data,
    });
    let root = Value::Map(map.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut map = map.borrow_mut();
        map.map.shift_remove(&Value::Id(1));
        map.map.insert(Value::Id(3), scalar_value(&added));
        map.key_kind = ValueKind::Any;
        map.value_kind = ValueKind::Any;
        map.num_elements = 9;
    }
    *retained.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    let map = map.borrow();
    assert_eq!(map.key_kind, ValueKind::Id);
    assert_eq!(map.value_kind, ValueKind::F64);
    assert_eq!(map.num_elements, 2);
    assert_eq!(
        map.map.keys().cloned().collect::<Vec<_>>(),
        vec![Value::Id(1), Value::Id(2)]
    );
    assert_eq!(
        as_scalar(map.map.get(&Value::Id(1)).unwrap()).addr(),
        removed.addr()
    );
    assert_eq!(
        as_scalar(map.map.get(&Value::Id(2)).unwrap()).addr(),
        retained.addr()
    );
    assert_eq!(*retained.borrow(), 2.0);
}

#[test]
fn state_journal_set_restores_metadata_order_and_retained_cell() {
    let removed = scalar(1.0);
    let retained = scalar(2.0);
    let added = scalar(3.0);
    let mut members = IndexSet::new();
    members.insert(scalar_value(&removed));
    members.insert(scalar_value(&retained));
    let set = Ref::new(MechSet {
        kind: ValueKind::F64,
        max_elements: Some(2),
        num_elements: 2,
        set: members,
    });
    let root = Value::Set(set.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut set = set.borrow_mut();
        set.set.shift_remove(&scalar_value(&removed));
        set.set.insert(scalar_value(&added));
        set.kind = ValueKind::Any;
        set.max_elements = Some(9);
        set.num_elements = 9;
    }
    // Perform this after structural edits so the temporarily stale hash does
    // not participate in another set operation.
    *retained.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    let set = set.borrow();
    assert_eq!(set.kind, ValueKind::F64);
    assert_eq!(set.max_elements, Some(2));
    assert_eq!(set.num_elements, 2);
    let members = set.set.iter().map(as_scalar).collect::<Vec<_>>();
    assert_eq!(
        members.iter().map(Ref::addr).collect::<Vec<_>>(),
        vec![removed.addr(), retained.addr()]
    );
    assert_eq!(*members[0].borrow(), 1.0);
    assert_eq!(*members[1].borrow(), 2.0);
}

#[test]
fn state_journal_set_membership_survives_repeated_rewind_and_replay() {
    let member = scalar(1.0);
    let set = Ref::new(MechSet::from_vec(vec![scalar_value(&member)]));
    let member_address = member.addr();
    let set_address = set.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&Value::Set(set.clone())).unwrap();
    assert_eq!(journal.cell_count(), 2);

    *member.borrow_mut() = 2.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    assert_eq!(delta.cell_count(), 2);

    for _ in 0..2 {
        delta.rewind().unwrap();
        assert_eq!(set.addr(), set_address);
        assert_eq!(member.addr(), member_address);
        assert_eq!(*member.borrow(), 1.0);
        assert!(set_contains_scalar(&set, 1.0));
        assert!(!set_contains_scalar(&set, 2.0));
        assert_eq!(
            set.borrow()
                .set
                .iter()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            member_address
        );

        delta.replay().unwrap();
        assert_eq!(set.addr(), set_address);
        assert_eq!(member.addr(), member_address);
        assert_eq!(*member.borrow(), 2.0);
        assert!(!set_contains_scalar(&set, 1.0));
        assert!(set_contains_scalar(&set, 2.0));
        assert_eq!(
            set.borrow()
                .set
                .iter()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            member_address
        );
    }
}

#[test]
fn state_journal_map_key_lookup_survives_repeated_rewind_and_replay() {
    let key = scalar(1.0);
    let value = scalar(10.0);
    let map = Ref::new(MechMap::from_vec(vec![(
        scalar_value(&key),
        scalar_value(&value),
    )]));
    let key_address = key.addr();
    let map_address = map.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&Value::Map(map.clone())).unwrap();
    assert_eq!(journal.cell_count(), 3);

    *key.borrow_mut() = 2.0;
    *value.borrow_mut() = 20.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    assert_eq!(delta.cell_count(), 3);

    for _ in 0..2 {
        delta.rewind().unwrap();
        assert_eq!(map.addr(), map_address);
        assert_eq!(key.addr(), key_address);
        assert!(map_get_scalar(&map, 2.0).is_none());
        let found = map_get_scalar(&map, 1.0).unwrap();
        assert_eq!(found.addr(), value.addr());
        assert_eq!(*found.borrow(), 10.0);
        assert_eq!(
            map.borrow()
                .map
                .keys()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            key_address
        );

        delta.replay().unwrap();
        assert_eq!(map.addr(), map_address);
        assert_eq!(key.addr(), key_address);
        assert!(map_get_scalar(&map, 1.0).is_none());
        let found = map_get_scalar(&map, 2.0).unwrap();
        assert_eq!(found.addr(), value.addr());
        assert_eq!(*found.borrow(), 20.0);
        assert_eq!(
            map.borrow()
                .map
                .keys()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            key_address
        );
    }
}

#[test]
fn state_journal_nested_hashed_collection_lookups_survive_delta() {
    let member = scalar(1.0);
    let inner_set = Ref::new(MechSet::from_vec(vec![scalar_value(&member)]));
    let outer_map = Ref::new(MechMap::from_vec(vec![(
        Value::Set(inner_set.clone()),
        Value::Id(7),
    )]));
    let outer_address = outer_map.addr();
    let inner_address = inner_set.addr();
    let member_address = member.addr();
    let mut journal = ValueStateJournal::new();
    journal
        .capture_value(&Value::Map(outer_map.clone()))
        .unwrap();
    assert_eq!(journal.cell_count(), 3);

    *member.borrow_mut() = 2.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();
    assert_eq!(delta.cell_count(), 3);

    for _ in 0..2 {
        delta.rewind().unwrap();
        assert_eq!(outer_map.addr(), outer_address);
        assert_eq!(inner_set.addr(), inner_address);
        assert_eq!(member.addr(), member_address);
        assert!(set_contains_scalar(&inner_set, 1.0));
        assert!(!set_contains_scalar(&inner_set, 2.0));
        assert!(map_contains_scalar_set_key(&outer_map, 1.0));
        assert!(!map_contains_scalar_set_key(&outer_map, 2.0));

        assert_eq!(outer_map.borrow().map.len(), 1);
        assert!(matches!(
            outer_map.borrow().map.values().next(),
            Some(Value::Id(7))
        ));
        let restored_key = outer_map.borrow().map.keys().next().cloned().unwrap();
        match restored_key {
            Value::Set(restored) => assert_eq!(restored.addr(), inner_address),
            _ => panic!("expected nested set key"),
        }
        assert_eq!(
            inner_set
                .borrow()
                .set
                .iter()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            member_address
        );

        delta.replay().unwrap();
        assert_eq!(outer_map.addr(), outer_address);
        assert_eq!(inner_set.addr(), inner_address);
        assert_eq!(member.addr(), member_address);
        assert!(!set_contains_scalar(&inner_set, 1.0));
        assert!(set_contains_scalar(&inner_set, 2.0));
        assert!(!map_contains_scalar_set_key(&outer_map, 1.0));
        assert!(map_contains_scalar_set_key(&outer_map, 2.0));

        assert_eq!(outer_map.borrow().map.len(), 1);
        assert!(matches!(
            outer_map.borrow().map.values().next(),
            Some(Value::Id(7))
        ));
        let restored_key = outer_map.borrow().map.keys().next().cloned().unwrap();
        match restored_key {
            Value::Set(restored) => assert_eq!(restored.addr(), inner_address),
            _ => panic!("expected nested set key"),
        }
        assert_eq!(
            inner_set
                .borrow()
                .set
                .iter()
                .next()
                .map(as_scalar)
                .unwrap()
                .addr(),
            member_address
        );
    }
}

#[test]
fn state_journal_table_restores_columns_backings_and_nested_cells() {
    let first = scalar(1.0);
    let retained = scalar(2.0);
    let added = scalar(3.0);
    let original_backing = Ref::new(DVector::from_vec(vec![
        scalar_value(&first),
        scalar_value(&retained),
    ]));
    let original_matrix = ValueMatrix::DVector(original_backing.clone());
    let added_backing = Ref::new(DVector::from_vec(vec![scalar_value(&added)]));
    let added_matrix = ValueMatrix::DVector(added_backing);
    let original_id = hash_str("original");
    let added_id = hash_str("added");
    let mut data = IndexMap::new();
    data.insert(original_id, (ValueKind::F64, original_matrix));
    let mut names = HashMap::new();
    names.insert(original_id, "original".to_string());
    let table = Ref::new(MechTable {
        rows: 2,
        cols: 1,
        data,
        col_names: names,
    });
    let root = Value::Table(table.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut table = table.borrow_mut();
        let original = table.data.shift_remove(&original_id).unwrap();
        table.data.insert(added_id, (ValueKind::F64, added_matrix));
        table.data.insert(original_id, original);
        table.col_names.insert(added_id, "added".to_string());
        table.rows = 7;
        table.cols = 2;
    }
    *retained.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    let table = table.borrow();
    assert_eq!(table.rows, 2);
    assert_eq!(table.cols, 1);
    assert_eq!(
        table.data.keys().copied().collect::<Vec<_>>(),
        vec![original_id]
    );
    assert_eq!(table.col_names.len(), 1);
    assert_eq!(table.col_names.get(&original_id).unwrap(), "original");
    let (kind, matrix) = table.data.get(&original_id).unwrap();
    assert_eq!(*kind, ValueKind::F64);
    assert_eq!(matrix.addr(), original_backing.addr());
    assert_eq!(
        matrix
            .as_vec()
            .iter()
            .map(scalar_payload)
            .collect::<Vec<_>>(),
        vec![1.0, 2.0]
    );
    assert_eq!(as_scalar(&matrix.as_vec()[1]).addr(), retained.addr());
}

#[test]
fn state_journal_dynamic_matrix_restores_shape_contents_and_backing() {
    let backing = Ref::new(DMatrix::from_vec(2, 2, vec![1.0, 2.0, 3.0, 4.0]));
    let root = Value::MatrixF64(ValueMatrix::DMatrix(backing.clone()));
    let address = backing.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();

    *backing.borrow_mut() = DMatrix::from_vec(1, 3, vec![8.0, 9.0, 10.0]);
    journal.restore_before().unwrap();

    let matrix = backing.borrow();
    assert_eq!(backing.addr(), address);
    assert_eq!(matrix.shape(), (2, 2));
    assert_eq!(matrix.as_slice(), &[1.0, 2.0, 3.0, 4.0]);
}

#[test]
fn state_journal_dynamic_vector_restores_contents_and_backing() {
    let backing = Ref::new(DVector::from_vec(vec![1.0, 2.0, 3.0]));
    let root = Value::MatrixF64(ValueMatrix::DVector(backing.clone()));
    let address = backing.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();

    *backing.borrow_mut() = DVector::from_vec(vec![9.0]);
    journal.restore_before().unwrap();

    assert_eq!(backing.addr(), address);
    assert_eq!(backing.borrow().as_slice(), &[1.0, 2.0, 3.0]);
}

#[cfg(feature = "matrix2")]
#[test]
fn state_journal_fixed_matrix_restores_contents_and_backing() {
    let backing = Ref::new(nalgebra::Matrix2::from_vec(vec![1.0, 2.0, 3.0, 4.0]));
    let root = Value::MatrixF64(ValueMatrix::Matrix2(backing.clone()));
    let address = backing.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();

    *backing.borrow_mut() = nalgebra::Matrix2::from_element(9.0);
    journal.restore_before().unwrap();

    assert_eq!(backing.addr(), address);
    assert_eq!(backing.borrow().as_slice(), &[1.0, 2.0, 3.0, 4.0]);
}
