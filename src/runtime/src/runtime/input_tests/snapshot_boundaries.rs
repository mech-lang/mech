use crate::{RuntimeHostInputValue, RuntimeValueSnapshot};
use mech_core::ValueData;

#[test]
fn canonical_host_input_snapshot_is_detached_and_schema_aware() {
    let value = RuntimeHostInputValue::F64(41.0).into_value().unwrap();
    let schema_key = value.schema_key();
    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();

    assert_eq!(snapshot.schema_key(), schema_key);
    assert!(matches!(snapshot.value().data(), ValueData::F64(value) if value.to_f64() == 41.0));
}

#[test]
fn canonical_matrix_host_input_preserves_logical_order() {
    let value = RuntimeHostInputValue::F64Matrix {
        rows: 2,
        columns: 2,
        values: vec![1.0, 2.0, 3.0, 4.0],
    }
    .into_value()
    .unwrap();
    let snapshot = RuntimeValueSnapshot::from_value(value).unwrap();

    let ValueData::Matrix(matrix) = snapshot.value().data() else {
        panic!("matrix host input must remain a canonical matrix")
    };
    let mech_core::snapshot::SequenceView::F64(values) = matrix.elements() else {
        panic!("matrix host input must retain f64 storage")
    };
    assert_eq!(
        values
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        [1.0, 2.0, 3.0, 4.0]
    );
}

#[test]
fn canonical_unit_is_the_only_empty_runtime_snapshot() {
    assert!(RuntimeValueSnapshot::empty().is_empty());
    let scalar =
        RuntimeValueSnapshot::from_value(RuntimeHostInputValue::Bool(false).into_value().unwrap())
            .unwrap();
    assert!(!scalar.is_empty());
}
