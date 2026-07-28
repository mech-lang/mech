use super::super::ValueMatrixComprehension;
use crate::{MechFunctionImpl, Ref, Value};

#[test]
fn transaction_state_retains_matrix_comprehension_outer_output_ref() {
    let out = Ref::new(Value::Empty);
    let function = ValueMatrixComprehension {
        arguments: Vec::new(),
        out: out.clone(),
    };

    let values = function.transaction_state_values().unwrap();
    assert_eq!(values.len(), 1);
    match &values[0] {
        Value::MutableReference(root) => assert_eq!(root.addr(), out.addr()),
        other => panic!("expected mutable-reference transaction root, got {other:?}"),
    }
}
