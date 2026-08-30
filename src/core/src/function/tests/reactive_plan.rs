use super::super::{FunctionInstance, FunctionInvocation, Plan, ReactiveNodeKind, ReactivePlan};
use super::support::{TestFunction, index};
use crate::{CanonicalCellId, ValueCell};
use std::{cell::RefCell, rc::Rc};

fn instance(function: TestFunction, output: ValueCell, inputs: Vec<ValueCell>) -> FunctionInstance {
    FunctionInstance::new(
        Box::new(function),
        FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
    )
}

#[test]
fn reactive_plan_push_preserves_order_and_single_ownership() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.push(Box::new(TestFunction::new("second")));

    assert_eq!(plan.nodes.len(), 2);
    assert_eq!(plan.nodes[0].id, 0);
    assert_eq!(plan.nodes[1].plan_index, 1);
    assert_eq!(
        plan.iter()
            .map(|function| function.to_string())
            .collect::<Vec<_>>(),
        vec!["first".to_string(), "second".to_string()]
    );
    assert_eq!(plan.len(), plan.nodes.len());
}

#[test]
fn canonical_instance_registration_records_output_identity_and_kind() {
    let (output, identity) = index(42);
    let function = TestFunction::with_output("register", output.clone())
        .with_node_kind(ReactiveNodeKind::Register);
    let mut plan = ReactivePlan::new();
    let node = plan
        .register_instance_with_activation(instance(function, output, Vec::new()), None)
        .unwrap();

    assert_eq!(plan.nodes[node].outputs, vec![identity]);
    assert_eq!(plan.nodes[node].kind, ReactiveNodeKind::Register);
}

#[test]
fn successful_registration_does_not_render_function_description() {
    let description_calls = Rc::new(RefCell::new(0));
    let output = ValueCell::unit();
    let function = TestFunction::with_output("description", output.clone())
        .with_description_counter(description_calls.clone());
    let mut plan = ReactivePlan::new();

    plan.register_instance_with_activation(instance(function, output, Vec::new()), None)
        .unwrap();

    assert_eq!(*description_calls.borrow(), 0);
    assert_eq!(plan[0].to_string(), "description");
    assert_eq!(*description_calls.borrow(), 1);
}

#[test]
fn cloned_plan_shares_storage_and_clear_removes_all_indexes() {
    let plan = Plan::new();
    let clone = plan.clone();
    plan.add_function(Box::new(TestFunction::new("shared")));
    assert_eq!((plan.len(), clone.len()), (1, 1));

    {
        let mut plan = plan.borrow_mut();
        plan.reactive_consumers
            .insert(CanonicalCellId::new(1), vec![0]);
        plan.sampled_consumers
            .insert(CanonicalCellId::new(2), vec![0]);
        plan.clear();
        assert!(plan.nodes.is_empty());
        assert!(plan.reactive_consumers.is_empty());
        assert!(plan.sampled_consumers.is_empty());
    }
    assert_eq!(clone.len(), 0);
}
