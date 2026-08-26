use crate::intrinsics::define::VarDefine;
use crate::{FunctionSpecializer, LegacyValue, Plan, ReactiveCellId, Ref};

#[test]
fn var_define_registration_has_no_reactive_inputs() {
    let plan = Plan::new();
    let value = Ref::new(1.0);
    let value_cell = ReactiveCellId::new(value.id());
    let arguments = vec![
        LegacyValue::F64(value),
        LegacyValue::String(Ref::new("defined value".to_string())),
        LegacyValue::Bool(Ref::new(false)),
        LegacyValue::Bool(Ref::new(true)),
    ];
    let function = VarDefine {}.specialize(&arguments).unwrap();

    plan.register_function(function, &[]).unwrap();

    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(0).unwrap();
    assert_eq!(plan_borrow.len(), 1);
    assert!(node.inputs.is_empty());
    assert!(plan_borrow.reactive_consumers.is_empty());
    assert!(plan_borrow.sampled_consumers.is_empty());
    assert!(node.outputs.contains(&value_cell));
}
