#[cfg(feature = "matrix_comprehensions")]
use super::super::ValueMatrixComprehension;
#[cfg(feature = "set_comprehensions")]
use super::super::ValueSetComprehension;
use crate::{FunctionArgs, MechFunctionFactory, MechFunctionImpl, MechSet, Ref, Value};

#[cfg(feature = "matrix_comprehensions")]
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

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_preserves_checked_set_output() {
    let output = Ref::new(MechSet::from_vec(Vec::new()));
    let function =
        ValueSetComprehension::new(FunctionArgs::Nullary(Value::Set(output.clone()))).unwrap();
    let Value::Set(actual) = function.out() else {
        panic!("expected set comprehension output")
    };
    assert_eq!(actual.addr(), output.addr());
}

#[cfg(feature = "set_comprehensions")]
#[test]
fn set_comprehension_factory_rejects_non_set_output() {
    let result = ValueSetComprehension::new(FunctionArgs::Nullary(Value::Empty));
    let error = match result {
        Ok(_) => panic!("non-set bytecode output should be rejected"),
        Err(error) => error,
    };
    assert_eq!(
        error.kind_name(),
        "SetComprehensionOutputKindMismatch".to_string()
    );
}
