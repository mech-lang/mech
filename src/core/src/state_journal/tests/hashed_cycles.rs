use crate::{LegacyValue, MechTuple, Ref, ValueData, ValueSnapshotCycleUnsupported};

#[test]
fn canonical_legacy_ingress_rejects_self_cycles() {
    let reference = Ref::new(LegacyValue::Empty);
    *reference.borrow_mut() = LegacyValue::MutableReference(reference.clone());

    let error = LegacyValue::MutableReference(reference)
        .to_canonical_value()
        .expect_err("self-referential legacy data must be rejected");
    assert!(error.kind_as::<ValueSnapshotCycleUnsupported>().is_some());
}

#[test]
fn canonical_legacy_ingress_rejects_multi_node_cycles() {
    let first = Ref::new(LegacyValue::Empty);
    let second = Ref::new(LegacyValue::MutableReference(first.clone()));
    *first.borrow_mut() = LegacyValue::MutableReference(second.clone());

    let error = LegacyValue::MutableReference(first)
        .to_canonical_value()
        .expect_err("multi-node legacy cycles must be rejected");
    assert!(error.kind_as::<ValueSnapshotCycleUnsupported>().is_some());
}

#[test]
fn canonical_legacy_ingress_duplicates_shared_data_but_cells_keep_aliases() {
    let shared = Ref::new(LegacyValue::F64(Ref::new(3.0)));
    let legacy = LegacyValue::Tuple(Ref::new(MechTuple::from_vec(vec![
        LegacyValue::MutableReference(shared.clone()),
        LegacyValue::MutableReference(shared.clone()),
    ])));

    let canonical = legacy.to_canonical_value().unwrap();
    let ValueData::Tuple(elements) = canonical.data() else {
        panic!("shared tuple changed canonical representation");
    };
    assert!(matches!(
        elements.as_ref(),
        [ValueData::F64(first), ValueData::F64(second)] if first == second
    ));
    assert!(!core::ptr::eq(&elements[0], &elements[1]));

    let cell = crate::ValueCell::from_legacy_ref(shared);
    assert!(cell.same_cell(&cell.clone()));

    // The canonical encoding contains owned schema/shape/data only, so no
    // reference edge exists from which an encoded data cycle could form.
    let schemas = canonical.schemas().unwrap();
    assert!(
        !canonical
            .canonical_snapshot_bytes(&schemas)
            .unwrap()
            .is_empty()
    );
}
