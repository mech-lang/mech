use crate::stdlib::define::VarDefine;
use crate::{NativeFunctionCompiler, Plan, ReactiveCellId, Ref, Value};

#[test]
fn var_define_registration_has_no_reactive_inputs() {
    let plan = Plan::new();
    let value = Ref::new(1.0);
    let value_cell = ReactiveCellId::new(value.id());
    let arguments = vec![
        Value::F64(value),
        Value::String(Ref::new("defined value".to_string())),
        Value::Bool(Ref::new(false)),
    ];
    let function = VarDefine {}.compile(&arguments).unwrap();

    plan.register_function(function, &[]).unwrap();

    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(0).unwrap();
    assert_eq!(plan_borrow.len(), 1);
    assert!(node.inputs.is_empty());
    assert!(plan_borrow.reactive_consumers.is_empty());
    assert!(plan_borrow.sampled_consumers.is_empty());
    assert!(node.outputs.contains(&value_cell));
}
