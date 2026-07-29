use std::collections::BTreeSet;

use mech_core::{
  Dictionary, MechAtom, MechEnum, MechRecord, MechTuple, Ref,
  Value, hash_str,
};
use mech_core::structures::matrix::Matrix;

use crate::RuntimeValueSnapshot;

#[test]
fn runtime_value_snapshot_detaches_atom_dictionary() {
  let atom_id = hash_str("snapshot/atom");
  let source_dictionary = Ref::new(Dictionary::new());
  source_dictionary
    .borrow_mut()
    .insert(atom_id, "source-atom".to_string());
  let source_atom = Ref::new(MechAtom((
    atom_id,
    source_dictionary.clone(),
  )));

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::Atom(source_atom.clone()),
  )
  .into_value();
  let Value::Atom(snapshot_atom) = snapshot else {
    panic!("expected atom snapshot");
  };
  let snapshot_dictionary =
    snapshot_atom.borrow().dictionary();

  assert!(!snapshot_atom.same_handle(&source_atom));
  assert!(
    !snapshot_dictionary.same_handle(&source_dictionary),
  );
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
    variants: vec![(
      variant_id,
      Some(Value::F64(source_payload.clone())),
    )],
    names: source_names.clone(),
  });

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::Enum(source_enum.clone()),
  )
  .into_value();
  let Value::Enum(snapshot_enum) = snapshot else {
    panic!("expected enum snapshot");
  };
  let (snapshot_names, snapshot_payload) = {
    let snapshot_enum_value = snapshot_enum.borrow();
    let Some(Value::F64(payload)) =
      snapshot_enum_value.variants[0].1.as_ref()
    else {
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
fn runtime_value_snapshot_preserves_self_referential_mutable_cycle() {
  let source = Ref::new(Value::Empty);
  *source.borrow_mut() =
    Value::MutableReference(source.clone());

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::MutableReference(source.clone()),
  )
  .into_value();
  let Value::MutableReference(detached) = snapshot else {
    panic!("expected detached mutable-reference root");
  };
  let detached_payload = detached.borrow().clone();
  let Value::MutableReference(back_edge) = detached_payload
  else {
    panic!("expected detached mutable-reference back-edge");
  };

  assert!(back_edge.same_handle(&detached));
  assert!(!detached.same_handle(&source));
}

#[test]
fn runtime_value_snapshot_preserves_two_node_mutable_cycle() {
  let source_a = Ref::new(Value::Empty);
  let source_b = Ref::new(Value::Empty);
  *source_a.borrow_mut() =
    Value::MutableReference(source_b.clone());
  *source_b.borrow_mut() =
    Value::MutableReference(source_a.clone());

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::MutableReference(source_a.clone()),
  )
  .into_value();
  let Value::MutableReference(detached_a) = snapshot else {
    panic!("expected detached first mutable-reference node");
  };
  let detached_a_payload = detached_a.borrow().clone();
  let Value::MutableReference(detached_b) =
    detached_a_payload
  else {
    panic!("expected detached second mutable-reference node");
  };
  let detached_b_payload = detached_b.borrow().clone();
  let Value::MutableReference(back_edge) =
    detached_b_payload
  else {
    panic!("expected detached two-node back-edge");
  };

  assert!(back_edge.same_handle(&detached_a));
  assert!(!detached_a.same_handle(&source_a));
  assert!(!detached_a.same_handle(&source_b));
  assert!(!detached_b.same_handle(&source_a));
  assert!(!detached_b.same_handle(&source_b));
  assert!(!detached_a.same_handle(&detached_b));
}

#[test]
fn runtime_value_snapshot_preserves_self_referential_tuple_cycle() {
  let source_tuple =
    Ref::new(MechTuple::from_vec(Vec::new()));
  source_tuple.borrow_mut().elements.push(Box::new(
    Value::Tuple(source_tuple.clone()),
  ));

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::Tuple(source_tuple.clone()),
  )
  .into_value();
  let Value::Tuple(detached_tuple) = snapshot else {
    panic!("expected detached tuple root");
  };
  let first_element = {
    detached_tuple.borrow().elements[0].as_ref().clone()
  };
  let Value::Tuple(back_edge) = first_element else {
    panic!("expected detached tuple back-edge");
  };

  assert!(back_edge.same_handle(&detached_tuple));
  assert!(!detached_tuple.same_handle(&source_tuple));
}

#[test]
fn runtime_value_snapshot_preserves_shared_detached_leaf() {
  let source_scalar = Ref::new(41.0);
  let source_tuple = Ref::new(MechTuple::from_vec(vec![
    Value::F64(source_scalar.clone()),
    Value::F64(source_scalar.clone()),
  ]));

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::Tuple(source_tuple),
  )
  .into_value();
  let Value::Tuple(snapshot_tuple) = snapshot else {
    panic!("expected detached tuple snapshot");
  };
  let (left, right) = {
    let tuple = snapshot_tuple.borrow();
    let Value::F64(left) = tuple.elements[0].as_ref()
    else {
      panic!("expected left scalar");
    };
    let Value::F64(right) = tuple.elements[1].as_ref()
    else {
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
  let inner = Ref::new(Value::F64(source_scalar.clone()));
  let outer =
    Ref::new(Value::MutableReference(inner.clone()));

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::MutableReference(outer),
  )
  .into_value();
  let Value::F64(snapshot_scalar) = snapshot else {
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
  let atom = Ref::new(MechAtom((
    atom_id,
    atom_names.clone(),
  )));

  let enum_id = hash_str("snapshot/composite/enum");
  let variant_id =
    hash_str("snapshot/composite/enum/variant");
  let enum_names = Ref::new(Dictionary::new());
  {
    let mut names = enum_names.borrow_mut();
    names.insert(enum_id, "composite-enum".to_string());
    names.insert(variant_id, "variant".to_string());
  }
  let enum_value = Ref::new(MechEnum {
    id: enum_id,
    variants: vec![(
      variant_id,
      Some(Value::F64(shared_scalar.clone())),
    )],
    names: enum_names.clone(),
  });
  let mutable = Ref::new(Value::F64(shared_scalar.clone()));
  let tuple = Ref::new(MechTuple::from_vec(vec![
    Value::F64(shared_scalar.clone()),
    Value::MutableReference(mutable),
  ]));
  let matrix = Matrix::from_vec(
    vec![Value::F64(shared_scalar.clone())],
    1,
    1,
  );
  let source = Value::Record(Ref::new(MechRecord::new(vec![
    ("atom", Value::Atom(atom)),
    ("enum", Value::Enum(enum_value)),
    ("tuple", Value::Tuple(tuple)),
    ("matrix", Value::MatrixValue(matrix)),
  ])));

  let snapshot =
    RuntimeValueSnapshot::capture(&source).into_value();
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
fn runtime_value_snapshot_preserves_matrix_value_cycle() {
  let source_matrix =
    Matrix::from_element(1, 1, Value::Empty);
  source_matrix.set_index1d(
    0,
    Value::MatrixValue(source_matrix.clone()),
  );

  let snapshot = RuntimeValueSnapshot::capture(
    &Value::MatrixValue(source_matrix.clone()),
  )
  .into_value();
  let Value::MatrixValue(snapshot_matrix) = snapshot
  else {
    panic!("expected detached matrix root");
  };
  let snapshot_element = snapshot_matrix.index1d(1);
  let Value::MatrixValue(back_edge) = snapshot_element
  else {
    panic!("expected detached matrix back-edge");
  };

  assert_eq!(back_edge.addr(), snapshot_matrix.addr());
  assert_ne!(snapshot_matrix.addr(), source_matrix.addr());
  assert_ne!(back_edge.addr(), source_matrix.addr());
}
