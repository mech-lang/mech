use super::super::{Value, hash_str, snapshot_runtime_value};
use mech_core::Ref;

#[cfg(all(feature = "record", feature = "tuple", feature = "f64"))]
#[test]
fn activation_effect_payload_snapshot_deeply_detaches_scene_values() {
    let live = Ref::new(1.0);
    let scene = Value::Record(Ref::new(mech_core::MechRecord::new(vec![(
        "position",
        Value::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
            Value::F64(live.clone()),
            Value::F64(Ref::new(2.0)),
        ]))),
    )])));
    let snapshot = snapshot_runtime_value(&scene)
        .expect("acyclic fixture");
    *live.borrow_mut() = 9.0;

    let Value::Record(snapshot) = snapshot else {
        panic!("expected record snapshot");
    };
    let position = {
        let snapshot = snapshot.borrow();
        let Value::Tuple(position) = snapshot.data.get(&hash_str("position")).unwrap() else {
            panic!("expected tuple field");
        };
        position.clone()
    };
    let position = position.borrow();
    let Value::F64(x) = position.elements[0].as_ref() else {
        panic!("expected scalar tuple element");
    };
    assert_eq!(*x.borrow(), 1.0);
    assert_ne!(x.as_ptr(), live.as_ptr());
}
