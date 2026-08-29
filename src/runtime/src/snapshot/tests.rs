use std::collections::BTreeSet;

use mech_core::structures::matrix::Matrix;
use mech_core::{
    Dictionary, LegacyValue, MechAtom, MechEnum, MechError, MechMap, MechRecord, MechSet,
    MechTuple, Ref, ValueKind, ValueSnapshotBorrowConflict, ValueSnapshotCollectionCollision,
    hash_str,
};

use crate::RuntimeValueSnapshot;

#[test]
fn runtime_value_snapshot_is_empty_matches_value_empty() {
    assert!(RuntimeValueSnapshot::empty().is_empty());

    let non_empty = RuntimeValueSnapshot::try_capture(&LegacyValue::Index(Ref::new(1))).unwrap();
    assert!(!non_empty.is_empty());
}

fn assert_cycle_error(error: MechError, node: &str) {
    assert_eq!(error.kind_name(), "ValueSnapshotCycleUnsupported",);
    assert!(error.kind_message().contains(node), "{error:?}",);
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("@0x"), "{rendered}");
    assert!(!rendered.contains("0x"), "{rendered}");
}

fn assert_borrow_conflict(error: MechError, phase: &str, node: &str) {
    assert_eq!(error.kind_name(), "ValueSnapshotBorrowConflict",);
    let conflict = error
        .kind_as::<ValueSnapshotBorrowConflict>()
        .expect("borrow conflict kind");
    assert_eq!(conflict.phase, phase);
    assert_eq!(conflict.node, node);
    assert!(!conflict.type_name.is_empty());
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("@0x"), "{rendered}");
    assert!(!rendered.contains("0x"), "{rendered}");
}

fn assert_collection_collision(error: MechError, collection: &str) {
    assert_eq!(error.kind_name(), "ValueSnapshotCollectionCollision",);
    let collision = error
        .kind_as::<ValueSnapshotCollectionCollision>()
        .expect("collection collision kind");
    assert_eq!(collision.collection, collection);
    assert_eq!(collision.first_index, 0);
    assert_eq!(collision.second_index, 1);
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("@0x"), "{rendered}");
}

#[test]
fn runtime_value_snapshot_returns_error_for_mutably_borrowed_leaf() {
    let cell = Ref::new(41.0);
    let _borrow = cell.borrow_mut();

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::F64(cell.clone())).unwrap_err();

    assert_borrow_conflict(error, "clone", "f64");
}

#[test]
fn runtime_value_snapshot_returns_error_for_mutably_borrowed_container() {
    let tuple = Ref::new(MechTuple::from_vec(vec![LegacyValue::F64(Ref::new(41.0))]));
    let _borrow = tuple.borrow_mut();

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::Tuple(tuple.clone())).unwrap_err();

    assert_borrow_conflict(error, "validate", "tuple");
}

#[test]
fn runtime_value_snapshot_returns_error_for_mutably_borrowed_matrix() {
    let matrix = Matrix::from_vec(vec![LegacyValue::Empty; 25], 5, 5);
    let Matrix::DMatrix(backing) = &matrix else {
        panic!("expected dynamic matrix fixture");
    };
    let _borrow = backing.borrow_mut();

    let error =
        RuntimeValueSnapshot::try_capture(&LegacyValue::MatrixValue(matrix.clone())).unwrap_err();

    assert_borrow_conflict(error, "validate", "matrix");
}

#[test]
fn runtime_value_snapshot_rejects_duplicate_equal_map_keys() {
    let left_key = Ref::new(1.0);
    let right_key = Ref::new(2.0);
    let map = Ref::new(MechMap::from_vec(vec![
        (
            LegacyValue::F64(left_key.clone()),
            LegacyValue::F64(Ref::new(10.0)),
        ),
        (
            LegacyValue::F64(right_key.clone()),
            LegacyValue::F64(Ref::new(20.0)),
        ),
    ]));
    *right_key.borrow_mut() = 1.0;

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::Map(map.clone())).unwrap_err();

    assert_collection_collision(error, "map key");
    let source = map.borrow();
    assert_eq!(source.num_elements, 2);
    assert_eq!(source.map.len(), 2);
    let keys = source
        .map
        .keys()
        .map(|value| {
            let LegacyValue::F64(value) = value else {
                panic!("expected scalar map key");
            };
            value.clone()
        })
        .collect::<Vec<_>>();
    assert!(keys[0].same_handle(&left_key));
    assert!(keys[1].same_handle(&right_key));
}

#[test]
fn runtime_value_snapshot_rejects_duplicate_equal_set_elements() {
    let left = Ref::new(-0.0);
    let right = Ref::new(1.0);
    let set = Ref::new(MechSet::from_vec(vec![
        LegacyValue::F64(left.clone()),
        LegacyValue::F64(right.clone()),
    ]));
    *right.borrow_mut() = 0.0;

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::Set(set.clone())).unwrap_err();

    assert_collection_collision(error, "set element");
    let source = set.borrow();
    assert_eq!(source.num_elements, 2);
    assert_eq!(source.set.len(), 2);
    let elements = source
        .set
        .iter()
        .map(|value| {
            let LegacyValue::F64(value) = value else {
                panic!("expected scalar set element");
            };
            value.clone()
        })
        .collect::<Vec<_>>();
    assert!(elements[0].same_handle(&left));
    assert!(elements[1].same_handle(&right));
    assert_eq!(left.borrow().to_bits(), (-0.0f64).to_bits());
    assert_eq!(*right.borrow(), 0.0);
}

#[test]
fn runtime_value_snapshot_detaches_atom_dictionary() {
    let atom_id = hash_str("snapshot/atom");
    let source_dictionary = Ref::new(Dictionary::new());
    source_dictionary
        .borrow_mut()
        .insert(atom_id, "source-atom".to_string());
    let source_atom = Ref::new(MechAtom((atom_id, source_dictionary.clone())));

    let snapshot = RuntimeValueSnapshot::try_capture(&LegacyValue::Atom(source_atom.clone()))
        .expect("acyclic fixture")
        .into_value();
    let LegacyValue::Atom(snapshot_atom) = snapshot else {
        panic!("expected atom snapshot");
    };
    let snapshot_dictionary = snapshot_atom.borrow().dictionary();

    assert!(!snapshot_atom.same_handle(&source_atom));
    assert!(!snapshot_dictionary.same_handle(&source_dictionary),);
    assert_eq!(snapshot_atom.borrow().name(), "source-atom");

    snapshot_dictionary
        .borrow_mut()
        .insert(atom_id, "snapshot-only".to_string());
    assert_eq!(snapshot_atom.borrow().name(), "snapshot-only");
    assert_eq!(source_atom.borrow().name(), "source-atom");

    source_dictionary
        .borrow_mut()
        .insert(atom_id, "source-only".to_string());
    assert_eq!(source_atom.borrow().name(), "source-only");
    assert_eq!(snapshot_atom.borrow().name(), "snapshot-only");
}

#[test]
fn runtime_value_snapshot_detaches_enum_dictionary() {
    let enum_id = hash_str("snapshot/enum");
    let variant_id = hash_str("snapshot/enum/variant");
    let source_names = Ref::new(Dictionary::new());
    {
        let mut names = source_names.borrow_mut();
        names.insert(enum_id, "source-enum".to_string());
        names.insert(variant_id, "source-variant".to_string());
    }
    let source_payload = Ref::new(7.0);
    let source_enum = Ref::new(MechEnum {
        id: enum_id,
        variants: vec![(variant_id, Some(LegacyValue::F64(source_payload.clone())))],
        names: source_names.clone(),
    });

    let snapshot = RuntimeValueSnapshot::try_capture(&LegacyValue::Enum(source_enum.clone()))
        .expect("acyclic fixture")
        .into_value();
    let LegacyValue::Enum(snapshot_enum) = snapshot else {
        panic!("expected enum snapshot");
    };
    let (snapshot_names, snapshot_payload) = {
        let snapshot_enum_value = snapshot_enum.borrow();
        let Some(LegacyValue::F64(payload)) = snapshot_enum_value.variants[0].1.as_ref() else {
            panic!("expected detached enum payload");
        };
        (snapshot_enum_value.names.clone(), payload.clone())
    };

    assert!(!snapshot_enum.same_handle(&source_enum));
    assert!(!snapshot_names.same_handle(&source_names));
    assert!(!snapshot_payload.same_handle(&source_payload));
    assert_eq!(*snapshot_payload.borrow(), 7.0);

    snapshot_names
        .borrow_mut()
        .insert(enum_id, "snapshot-enum".to_string());
    assert_eq!(
        snapshot_names.borrow().get(&enum_id).cloned(),
        Some("snapshot-enum".to_string()),
    );
    assert_eq!(
        source_names.borrow().get(&enum_id).cloned(),
        Some("source-enum".to_string()),
    );

    source_names
        .borrow_mut()
        .insert(enum_id, "live-enum".to_string());
    assert_eq!(
        source_names.borrow().get(&enum_id).cloned(),
        Some("live-enum".to_string()),
    );
    assert_eq!(
        snapshot_names.borrow().get(&enum_id).cloned(),
        Some("snapshot-enum".to_string()),
    );

    *snapshot_payload.borrow_mut() = 9.0;
    assert_eq!(*snapshot_payload.borrow(), 9.0);
    assert_eq!(*source_payload.borrow(), 7.0);
}

#[test]
fn runtime_value_snapshot_rejects_self_referential_mutable_cycle() {
    let source = Ref::new(LegacyValue::Empty);
    *source.borrow_mut() = LegacyValue::MutableReference(source.clone());

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::MutableReference(source.clone()))
        .unwrap_err();

    assert_cycle_error(error, "mutable-reference");
}

#[test]
fn runtime_value_snapshot_rejects_two_node_mutable_cycle() {
    let source_a = Ref::new(LegacyValue::Empty);
    let source_b = Ref::new(LegacyValue::Empty);
    *source_a.borrow_mut() = LegacyValue::MutableReference(source_b.clone());
    *source_b.borrow_mut() = LegacyValue::MutableReference(source_a.clone());

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::MutableReference(source_a.clone()))
        .unwrap_err();

    assert_cycle_error(error, "mutable-reference");
}

#[test]
fn runtime_value_snapshot_rejects_cycle_after_acyclic_reference_prefix() {
    let source_a = Ref::new(LegacyValue::Empty);
    let source_b = Ref::new(LegacyValue::Empty);
    let source_c = Ref::new(LegacyValue::Empty);
    *source_a.borrow_mut() = LegacyValue::MutableReference(source_b.clone());
    *source_b.borrow_mut() = LegacyValue::MutableReference(source_a.clone());
    *source_c.borrow_mut() = LegacyValue::MutableReference(source_a.clone());

    let error =
        RuntimeValueSnapshot::try_capture(&LegacyValue::MutableReference(source_c)).unwrap_err();

    assert_cycle_error(error, "mutable-reference");
}

#[test]
fn runtime_value_snapshot_rejects_self_referential_tuple_cycle() {
    let source_tuple = Ref::new(MechTuple::from_vec(Vec::new()));
    source_tuple
        .borrow_mut()
        .elements
        .push(Box::new(LegacyValue::Tuple(source_tuple.clone())));

    let error =
        RuntimeValueSnapshot::try_capture(&LegacyValue::Tuple(source_tuple.clone())).unwrap_err();

    assert_cycle_error(error, "tuple");
}

#[test]
fn runtime_value_snapshot_preserves_shared_detached_leaf() {
    let source_scalar = Ref::new(41.0);
    let source_tuple = Ref::new(MechTuple::from_vec(vec![
        LegacyValue::F64(source_scalar.clone()),
        LegacyValue::F64(source_scalar.clone()),
    ]));

    let snapshot = RuntimeValueSnapshot::try_capture(&LegacyValue::Tuple(source_tuple))
        .expect("acyclic fixture")
        .into_value();
    let LegacyValue::Tuple(snapshot_tuple) = snapshot else {
        panic!("expected detached tuple snapshot");
    };
    let (left, right) = {
        let tuple = snapshot_tuple.borrow();
        let LegacyValue::F64(left) = tuple.elements[0].as_ref() else {
            panic!("expected left scalar");
        };
        let LegacyValue::F64(right) = tuple.elements[1].as_ref() else {
            panic!("expected right scalar");
        };
        (left.clone(), right.clone())
    };

    assert!(left.same_handle(&right));
    assert!(!left.same_handle(&source_scalar));
    *left.borrow_mut() = 42.0;
    assert_eq!(*right.borrow(), 42.0);
    assert_eq!(*source_scalar.borrow(), 41.0);
}

#[test]
fn runtime_value_snapshot_still_unwraps_acyclic_mutable_reference_chain() {
    let source_scalar = Ref::new(41.0);
    let inner = Ref::new(LegacyValue::F64(source_scalar.clone()));
    let outer = Ref::new(LegacyValue::MutableReference(inner.clone()));

    let snapshot = RuntimeValueSnapshot::try_capture(&LegacyValue::MutableReference(outer))
        .expect("acyclic fixture")
        .into_value();
    let LegacyValue::F64(snapshot_scalar) = snapshot else {
        panic!("expected value-transparent scalar snapshot");
    };

    assert!(!snapshot_scalar.same_handle(&source_scalar));
    assert_eq!(*snapshot_scalar.borrow(), 41.0);
}

#[test]
fn runtime_value_snapshot_reachable_cells_are_disjoint_from_source() {
    let shared_scalar = Ref::new(41.0);
    let atom_id = hash_str("snapshot/composite/atom");
    let atom_names = Ref::new(Dictionary::new());
    atom_names
        .borrow_mut()
        .insert(atom_id, "composite-atom".to_string());
    let atom = Ref::new(MechAtom((atom_id, atom_names.clone())));

    let enum_id = hash_str("snapshot/composite/enum");
    let variant_id = hash_str("snapshot/composite/enum/variant");
    let enum_names = Ref::new(Dictionary::new());
    {
        let mut names = enum_names.borrow_mut();
        names.insert(enum_id, "composite-enum".to_string());
        names.insert(variant_id, "variant".to_string());
    }
    let enum_value = Ref::new(MechEnum {
        id: enum_id,
        variants: vec![(variant_id, Some(LegacyValue::F64(shared_scalar.clone())))],
        names: enum_names.clone(),
    });
    let mutable = Ref::new(LegacyValue::F64(shared_scalar.clone()));
    let tuple = Ref::new(MechTuple::from_vec(vec![
        LegacyValue::F64(shared_scalar.clone()),
        LegacyValue::MutableReference(mutable),
    ]));
    let matrix = Matrix::from_vec(vec![LegacyValue::F64(shared_scalar.clone())], 1, 1);
    let source = LegacyValue::Record(Ref::new(MechRecord::new(vec![
        ("atom", LegacyValue::Atom(atom)),
        ("enum", LegacyValue::Enum(enum_value)),
        ("tuple", LegacyValue::Tuple(tuple)),
        ("matrix", LegacyValue::MatrixValue(matrix)),
    ])));

    let snapshot = RuntimeValueSnapshot::try_capture(&source)
        .expect("acyclic fixture")
        .into_value();
    let source_ids = source
        .reactive_cell_ids()
        .into_iter()
        .map(|id| id.get())
        .collect::<BTreeSet<_>>();
    let snapshot_ids = snapshot
        .reactive_cell_ids()
        .into_iter()
        .map(|id| id.get())
        .collect::<BTreeSet<_>>();

    assert!(source_ids.is_disjoint(&snapshot_ids));
}

#[test]
fn runtime_value_snapshot_observers_are_safe_for_accepted_values() {
    let shared = Ref::new(41.0);
    let atom_id = hash_str("snapshot/observer/atom");
    let atom_names = Ref::new(Dictionary::from([(atom_id, "observer-atom".to_string())]));
    let atom = LegacyValue::Atom(Ref::new(MechAtom((atom_id, atom_names))));
    let enum_id = hash_str("snapshot/observer/enum");
    let variant_id = hash_str("snapshot/observer/variant");
    let enum_names = Ref::new(Dictionary::from([
        (enum_id, "observer-enum".to_string()),
        (variant_id, "observer-variant".to_string()),
    ]));
    let enum_value = LegacyValue::Enum(Ref::new(MechEnum {
        id: enum_id,
        variants: vec![(variant_id, Some(LegacyValue::F64(shared.clone())))],
        names: enum_names,
    }));
    let tuple = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
        LegacyValue::F64(shared.clone()),
        LegacyValue::F64(shared.clone()),
    ])));
    let matrix = LegacyValue::MatrixValue(Matrix::from_vec(
        vec![LegacyValue::F64(shared.clone())],
        1,
        1,
    ));
    let source = LegacyValue::Record(Ref::new(MechRecord::new(vec![
        ("atom", atom),
        ("enum", enum_value),
        ("tuple", tuple),
        ("matrix", matrix),
        ("shared", LegacyValue::F64(shared)),
    ])));
    let snapshot = RuntimeValueSnapshot::try_capture(&source).expect("acyclic fixture");

    assert!(matches!(snapshot.kind(), ValueKind::Record(_),));
    let debug = format!("{snapshot:?}");
    let display = format!("{snapshot}");
    assert!(debug.contains("Record"), "{debug}");
    assert!(!display.is_empty());
    assert_eq!(snapshot, snapshot.clone());
}

#[test]
fn runtime_value_snapshot_clone_has_independent_cells() {
    let source = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![LegacyValue::F64(
        Ref::new(41.0),
    )])));
    let original = RuntimeValueSnapshot::try_capture(&source).expect("acyclic fixture");
    let consumed_clone = original.clone().into_value();
    let LegacyValue::Tuple(consumed_tuple) = &consumed_clone else {
        panic!("expected consumed tuple");
    };
    let consumed_scalar = {
        let tuple = consumed_tuple.borrow();
        let LegacyValue::F64(value) = tuple.elements[0].as_ref() else {
            panic!("expected consumed scalar");
        };
        value.clone()
    };
    *consumed_scalar.borrow_mut() = 99.0;

    let original_value = original.to_value();
    let LegacyValue::Tuple(original_tuple) = original_value else {
        panic!("expected original tuple");
    };
    let original_scalar = {
        let tuple = original_tuple.borrow();
        let LegacyValue::F64(value) = tuple.elements[0].as_ref() else {
            panic!("expected original scalar");
        };
        value.clone()
    };

    assert_eq!(*original_scalar.borrow(), 41.0);
    assert_eq!(*consumed_scalar.borrow(), 99.0);
}

#[test]
fn runtime_value_snapshot_rejects_matrix_value_cycle() {
    let source_matrix = Matrix::from_element(1, 1, LegacyValue::Empty);
    source_matrix.set_index1d(0, LegacyValue::MatrixValue(source_matrix.clone()));

    let error = RuntimeValueSnapshot::try_capture(&LegacyValue::MatrixValue(source_matrix.clone()))
        .unwrap_err();

    assert_cycle_error(error, "matrix");
}
