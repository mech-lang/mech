use super::support::{as_scalar, record_value, scalar, scalar_payload, scalar_value};
#[cfg(feature = "atom")]
use crate::MechAtom;
use crate::structures::matrix::Matrix as ValueMatrix;
use crate::{
    LegacyValue, MechEnum, MechTuple, Ref, ValueCell, ValueKind, ValueStateJournal, hash_str,
};
use nalgebra::DMatrix;
use std::collections::HashMap;

#[test]
fn state_journal_deduplicates_shared_cells_and_restores_aliases() {
    let shared = scalar(3.0);
    let (record, root) = record_value(vec![
        ("left", scalar_value(&shared)),
        ("right", scalar_value(&shared)),
    ]);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();

    assert_eq!(journal.cell_count(), 2);
    *shared.borrow_mut() = 8.0;
    journal.restore_before().unwrap();

    let record = record.borrow();
    let left = as_scalar(record.data.get(&hash_str("left")).unwrap());
    let right = as_scalar(record.data.get(&hash_str("right")).unwrap());
    assert_eq!(*left.borrow(), 3.0);
    assert_eq!(left.addr(), shared.addr());
    assert_eq!(right.addr(), shared.addr());
}

#[test]
fn state_journal_self_reference_terminates_in_both_phases() {
    let cell = ValueCell::new(LegacyValue::Empty);
    let reference = cell.legacy_ref();
    *cell.borrow_mut() = LegacyValue::MutableReference(reference.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value_cell(&cell).unwrap();
    assert_eq!(journal.cell_count(), 1);
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    match &*cell.borrow() {
        LegacyValue::MutableReference(inner) => assert!(inner.same_handle(&reference)),
        _ => panic!("self-reference was not restored"),
    }
    delta.replay().unwrap();
    match &*cell.borrow() {
        LegacyValue::MutableReference(inner) => assert!(inner.same_handle(&reference)),
        _ => panic!("self-reference was not replayed"),
    }
}

#[test]
fn state_journal_value_cell_tracks_replaced_root_and_after_only_cell() {
    let before = scalar(1.0);
    let after = scalar(2.0);
    let root = ValueCell::new(scalar_value(&before));
    let original_root = root.clone();

    let mut journal = ValueStateJournal::new();
    journal.capture_value_cell(&root).unwrap();
    *root.borrow_mut() = scalar_value(&after);
    journal.record_after().unwrap();
    assert_eq!(journal.cell_count(), 3);
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    assert!(root.same_cell(&original_root));
    assert_eq!(as_scalar(&root.borrow()).addr(), before.addr());

    *after.borrow_mut() = 22.0;
    delta.replay().unwrap();
    assert_eq!(as_scalar(&root.borrow()).addr(), after.addr());
    assert_eq!(*after.borrow(), 2.0);

    *before.borrow_mut() = 11.0;
    delta.rewind().unwrap();
    assert_eq!(as_scalar(&root.borrow()).addr(), before.addr());
    assert_eq!(*before.borrow(), 1.0);
}

#[test]
fn state_journal_deduplicates_repeated_clones_of_one_value_cell() {
    let nested = scalar(1.0);
    let root = ValueCell::new(scalar_value(&nested));
    let clone = root.clone();
    let mut journal = ValueStateJournal::new();

    journal.capture_value_cell(&root).unwrap();
    journal.capture_value_cell(&clone).unwrap();

    assert_eq!(journal.cell_count(), 2);
    *clone.borrow_mut() = LegacyValue::Empty;
    journal.restore_before().unwrap();
    assert!(root.same_cell(&clone));
    assert!(as_scalar(&root.borrow()).same_handle(&nested));
}

#[test]
fn state_journal_tuple_restores_structure_order_and_child_identity() {
    let first = scalar(1.0);
    let retained = scalar(2.0);
    let replacement = scalar(3.0);
    let tuple = Ref::new(MechTuple::from_vec(vec![
        scalar_value(&first),
        scalar_value(&retained),
    ]));
    let root = LegacyValue::Tuple(tuple.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut tuple = tuple.borrow_mut();
        tuple.elements.remove(0);
        tuple
            .elements
            .insert(0, Box::new(scalar_value(&replacement)));
        tuple.elements.swap(0, 1);
    }
    *retained.borrow_mut() = 22.0;
    journal.restore_before().unwrap();

    let tuple = tuple.borrow();
    assert_eq!(tuple.elements.len(), 2);
    assert_eq!(as_scalar(&tuple.elements[0]).addr(), first.addr());
    assert_eq!(as_scalar(&tuple.elements[1]).addr(), retained.addr());
    assert_eq!(scalar_payload(&tuple.elements[0]), 1.0);
    assert_eq!(scalar_payload(&tuple.elements[1]), 2.0);
}

#[test]
fn state_journal_enum_restores_names_variants_and_payload_identities() {
    let removed = scalar(1.0);
    let retained = scalar(2.0);
    let added = scalar(3.0);
    let enum_id = hash_str("choice");
    let removed_id = hash_str("removed");
    let retained_id = hash_str("retained");
    let added_id = hash_str("added");
    let mut dictionary = HashMap::new();
    dictionary.insert(enum_id, "choice".to_string());
    dictionary.insert(removed_id, "choice/removed".to_string());
    dictionary.insert(retained_id, "choice/retained".to_string());
    let names = Ref::new(dictionary);
    let names_address = names.addr();
    let enum_value = Ref::new(MechEnum {
        id: enum_id,
        variants: vec![
            (removed_id, Some(scalar_value(&removed))),
            (retained_id, Some(scalar_value(&retained))),
        ],
        names: names.clone(),
    });
    let root = LegacyValue::Enum(enum_value.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    {
        let mut value = enum_value.borrow_mut();
        value.id = hash_str("changed");
        value.variants.remove(0);
        value
            .variants
            .insert(0, (added_id, Some(scalar_value(&added))));
    }
    names.borrow_mut().clear();
    *retained.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    let value = enum_value.borrow();
    assert_eq!(value.id, enum_id);
    assert_eq!(value.names.addr(), names_address);
    assert_eq!(
        value.variants.iter().map(|(id, _)| *id).collect::<Vec<_>>(),
        vec![removed_id, retained_id]
    );
    assert_eq!(
        as_scalar(value.variants[0].1.as_ref().unwrap()).addr(),
        removed.addr()
    );
    assert_eq!(
        as_scalar(value.variants[1].1.as_ref().unwrap()).addr(),
        retained.addr()
    );
    assert_eq!(
        value.names.borrow().get(&retained_id).unwrap(),
        "choice/retained"
    );
    assert_eq!(*retained.borrow(), 2.0);
}

#[cfg(feature = "atom")]
#[test]
fn state_journal_atom_restores_its_reachable_dictionary_cell() {
    let atom_id = hash_str("ready");
    let mut dictionary = HashMap::new();
    dictionary.insert(atom_id, "ready".to_string());
    let names = Ref::new(dictionary);
    let names_address = names.addr();
    let atom = Ref::new(MechAtom((atom_id, names.clone())));
    let root = LegacyValue::Atom(atom.clone());

    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    assert_eq!(journal.cell_count(), 2);
    atom.borrow_mut().0.0 = hash_str("changed");
    names.borrow_mut().clear();
    journal.restore_before().unwrap();

    assert_eq!(atom.borrow().id(), atom_id);
    assert_eq!(atom.borrow().dictionary().addr(), names_address);
    assert_eq!(names.borrow().get(&atom_id).unwrap(), "ready");
}

#[test]
fn state_journal_value_matrix_restores_topology_and_nested_cells() {
    let first = scalar(1.0);
    let retained = scalar(2.0);
    let backing = Ref::new(DMatrix::from_vec(
        2,
        1,
        vec![scalar_value(&first), scalar_value(&retained)],
    ));
    let root = LegacyValue::MatrixValue(ValueMatrix::DMatrix(backing.clone()));
    let address = backing.addr();
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();

    *backing.borrow_mut() = DMatrix::from_vec(1, 1, vec![scalar_value(&retained)]);
    *retained.borrow_mut() = 20.0;
    journal.restore_before().unwrap();

    let matrix = backing.borrow();
    assert_eq!(backing.addr(), address);
    assert_eq!(matrix.shape(), (2, 1));
    assert_eq!(as_scalar(&matrix[0]).addr(), first.addr());
    assert_eq!(as_scalar(&matrix[1]).addr(), retained.addr());
    assert_eq!(scalar_payload(&matrix[0]), 1.0);
    assert_eq!(scalar_payload(&matrix[1]), 2.0);
}

#[test]
fn state_journal_typed_value_rewinds_nested_state_without_changing_kind() {
    let cell = scalar(1.0);
    let root = LegacyValue::Typed(Box::new(scalar_value(&cell)), ValueKind::F64);
    let mut journal = ValueStateJournal::new();
    journal.capture_value(&root).unwrap();
    *cell.borrow_mut() = 2.0;
    journal.record_after().unwrap();
    let delta = journal.into_delta().unwrap();

    delta.rewind().unwrap();
    assert_eq!(*cell.borrow(), 1.0);
    delta.replay().unwrap();
    assert_eq!(*cell.borrow(), 2.0);
    match root {
        LegacyValue::Typed(_, kind) => assert_eq!(kind, ValueKind::F64),
        _ => panic!("typed wrapper changed"),
    }
}
