use super::super::{Plan, ReactiveNodeKind, ReactivePlan};
use super::support::TestFunction;
#[cfg(all(feature = "set", feature = "f64"))]
use super::support::set_output;
use crate::{LegacyValue, ReactiveCellId, Ref};
use std::{cell::RefCell, rc::Rc};

#[test]
fn reactive_plan_push_creates_one_node() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].id, 0);
    assert_eq!(plan.nodes[0].plan_index, 0);
}

#[test]
fn reactive_plan_preserves_insertion_order() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.push(Box::new(TestFunction::new("second")));

    let names = plan
        .iter()
        .map(|function| function.to_string())
        .collect::<Vec<_>>();
    assert_eq!(names, vec!["first".to_string(), "second".to_string()]);
    assert_eq!(plan[0].to_string(), "first");
    assert_eq!(plan[1].to_string(), "second");
}

#[test]
fn reactive_plan_node_is_only_function_owner() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.push(Box::new(TestFunction::new("second")));

    assert_eq!(plan.len(), plan.nodes.len());
}

#[test]
fn successful_registration_does_not_render_function_description() {
    let description_calls = Rc::new(RefCell::new(0));
    let mut plan = ReactivePlan::new();

    plan.register(
        Box::new(
            TestFunction::new("expensive-description")
                .with_description_counter(description_calls.clone()),
        ),
        &[],
    )
    .unwrap();

    assert_eq!(*description_calls.borrow(), 0);
    assert_eq!(plan[0].to_string(), "expensive-description");
    assert_eq!(*description_calls.borrow(), 1);
}

#[cfg(all(feature = "set", feature = "f64"))]
#[test]
fn reactive_plan_push_records_root_output_cells() {
    let (output, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::with_output("set", output)));

    assert_eq!(plan.nodes.len(), 1);
    assert_eq!(plan.nodes[0].outputs, vec![outer]);
    assert!(!plan.nodes[0].outputs.contains(&first));
    assert!(!plan.nodes[0].outputs.contains(&second));
}

#[cfg(all(feature = "set", feature = "f64"))]
#[test]
fn reactive_plan_register_records_root_output_cells() {
    let (output, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(Box::new(TestFunction::with_output("set", output)), &[])
        .unwrap();
    let node = plan.node(node_id).unwrap();

    assert_eq!(node.outputs, vec![outer]);
    assert!(!node.outputs.contains(&first));
    assert!(!node.outputs.contains(&second));
}

#[cfg(feature = "f64")]
#[test]
fn reactive_plan_records_output_cells() {
    let output = Ref::new(42.0);
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::with_output(
        "output",
        LegacyValue::F64(output.clone()),
    )));

    assert!(
        plan.nodes[0]
            .outputs
            .contains(&ReactiveCellId::new(output.id()))
    );
}

#[test]
fn reactive_plan_clone_shares_storage() {
    let plan = Plan::new();
    let clone = plan.clone();

    plan.add_function(Box::new(TestFunction::new("shared")));

    assert_eq!(plan.len(), 1);
    assert_eq!(clone.len(), 1);
}

#[test]
fn reactive_plan_clear_removes_nodes_and_indexes() {
    let mut plan = ReactivePlan::new();
    plan.push(Box::new(TestFunction::new("first")));
    plan.reactive_consumers
        .insert(ReactiveCellId::new(1), vec![0]);
    plan.sampled_consumers
        .insert(ReactiveCellId::new(2), vec![0]);

    plan.clear();

    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_records_outputs_and_kind() {
    let output = Ref::new(42.0);
    let output_cell = ReactiveCellId::new(output.id());
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", LegacyValue::F64(output))
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[],
        )
        .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert!(node.outputs.contains(&output_cell));
}
