use super::super::{
    Plan, ReactiveDependency, ReactiveDependencyKind, ReactiveDependencyScope, ReactiveNodeKind,
    ReactivePlan,
};
#[cfg(feature = "set")]
use super::support::set_output;
use super::support::{TestFunction, scalar};
use crate::{LegacyValue, Ref};

#[cfg(feature = "f64")]
#[test]
fn register_node_indexes_output_as_sampled_state() {
    let (output, output_cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", output)
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[],
        )
        .unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![output_cell]);
    assert_eq!(
        node.inputs,
        vec![ReactiveDependency {
            cell: output_cell,
            kind: ReactiveDependencyKind::Sampled
        }]
    );
    assert_eq!(plan.sampled_consumers_for(output_cell), &[node_id]);
    assert!(plan.reactive_consumers_for(output_cell).is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_node_keeps_source_dependency_reactive() {
    let (output, output_cell) = scalar(1.0);
    let (source, source_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", output)
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[source],
        )
        .unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![
            ReactiveDependency {
                cell: output_cell,
                kind: ReactiveDependencyKind::Sampled
            },
            ReactiveDependency {
                cell: source_cell,
                kind: ReactiveDependencyKind::Reactive
            },
        ]
    );
    assert_eq!(plan.sampled_consumers_for(output_cell), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(source_cell), &[node_id]);
}

#[cfg(feature = "f64")]
#[test]
fn register_node_coalesces_output_operand_alias_to_sampled() {
    let (output, output_cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", output.clone())
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[output],
        )
        .unwrap();
    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![ReactiveDependency {
            cell: output_cell,
            kind: ReactiveDependencyKind::Sampled
        }]
    );
    assert!(plan.reactive_consumers_for(output_cell).is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_node_has_no_reactive_self_consumer() {
    let (output, output_cell) = scalar(1.0);
    let (source, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", output)
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[source],
        )
        .unwrap();
    assert!(!plan.reactive_consumers_for(output_cell).contains(&node_id));
    assert!(plan.sampled_consumers_for(output_cell).contains(&node_id));
}

#[cfg(feature = "f64")]
#[test]
fn register_node_preserves_dependency_order() {
    let (output, output_cell) = scalar(1.0);
    let (first, first_cell) = scalar(2.0);
    let (second, second_cell) = scalar(3.0);
    let mut plan = ReactivePlan::new();
    let node_id = plan
        .register(
            Box::new(
                TestFunction::with_output("register", output)
                    .with_node_kind(ReactiveNodeKind::Register),
            ),
            &[first, second],
        )
        .unwrap();
    assert_eq!(
        plan.node(node_id).unwrap().inputs,
        vec![
            ReactiveDependency {
                cell: output_cell,
                kind: ReactiveDependencyKind::Sampled
            },
            ReactiveDependency {
                cell: first_cell,
                kind: ReactiveDependencyKind::Reactive
            },
            ReactiveDependency {
                cell: second_cell,
                kind: ReactiveDependencyKind::Reactive
            },
        ]
    );
}

#[cfg(feature = "f64")]
#[test]
fn register_node_validation_failure_does_not_mutate_plan() {
    let (output, _) = scalar(1.0);
    let (source, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();
    assert!(
        plan.register(
            Box::new(
                TestFunction::with_output("register", output)
                    .with_node_kind(ReactiveNodeKind::Register)
                    .with_dependency_kinds(Some(vec![]))
            ),
            &[source]
        )
        .is_err()
    );
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_defaults_arguments_to_reactive() {
    let (first, first_cell) = scalar(1.0);
    let (second, second_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(Box::new(TestFunction::new("default")), &[first, second])
        .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![
            ReactiveDependency {
                cell: first_cell,
                kind: ReactiveDependencyKind::Reactive
            },
            ReactiveDependency {
                cell: second_cell,
                kind: ReactiveDependencyKind::Reactive
            },
        ],
    );
    assert_eq!(plan.reactive_consumers_for(first_cell), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(second_cell), &[node_id]);
    assert!(plan.sampled_consumers_for(first_cell).is_empty());
    assert!(plan.sampled_consumers_for(second_cell).is_empty());
}

#[cfg(all(feature = "set", feature = "f64"))]
#[test]
fn register_defaults_dependency_scope_to_recursive() {
    let (set, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(Box::new(TestFunction::new("recursive")), &[set])
        .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![
            ReactiveDependency {
                cell: outer,
                kind: ReactiveDependencyKind::Reactive
            },
            ReactiveDependency {
                cell: first,
                kind: ReactiveDependencyKind::Reactive
            },
            ReactiveDependency {
                cell: second,
                kind: ReactiveDependencyKind::Reactive
            },
        ],
    );
    assert_eq!(plan.reactive_consumers_for(outer), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(first), &[node_id]);
    assert_eq!(plan.reactive_consumers_for(second), &[node_id]);
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(all(feature = "set", feature = "f64"))]
#[test]
fn register_root_scope_uses_only_root_cell() {
    let (set, outer, first, second) = set_output();
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(
            Box::new(
                TestFunction::new("root")
                    .with_dependency_scopes(Some(vec![ReactiveDependencyScope::Root])),
            ),
            &[set],
        )
        .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![ReactiveDependency {
            cell: outer,
            kind: ReactiveDependencyKind::Reactive
        }],
    );
    assert_eq!(plan.reactive_consumers_for(outer), &[node_id]);
    assert!(plan.reactive_consumers_for(first).is_empty());
    assert!(plan.reactive_consumers_for(second).is_empty());
    assert_eq!(plan.reactive_consumers.len(), 1);
}

#[cfg(feature = "f64")]
#[test]
fn register_none_scope_ignores_argument_cells() {
    let (value, _) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(
            Box::new(
                TestFunction::new("none")
                    .with_dependency_scopes(Some(vec![ReactiveDependencyScope::None])),
            ),
            &[value],
        )
        .unwrap();

    assert!(plan.node(node_id).unwrap().inputs.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_records_sampled_dependencies_separately() {
    let (first, first_cell) = scalar(1.0);
    let (second, second_cell) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(
            Box::new(
                TestFunction::new("sampled").with_dependency_kinds(Some(vec![
                    ReactiveDependencyKind::Sampled,
                    ReactiveDependencyKind::Reactive,
                ])),
            ),
            &[first, second],
        )
        .unwrap();

    assert_eq!(plan.sampled_consumers_for(first_cell), &[node_id]);
    assert!(plan.reactive_consumers_for(first_cell).is_empty());
    assert_eq!(plan.reactive_consumers_for(second_cell), &[node_id]);
    assert!(plan.sampled_consumers_for(second_cell).is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_deduplicates_same_cell_same_kind() {
    let (value, cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let node_id = plan
        .register(
            Box::new(TestFunction::new("dedupe")),
            &[value.clone(), value],
        )
        .unwrap();

    let node = plan.node(node_id).unwrap();
    assert_eq!(
        node.inputs,
        vec![ReactiveDependency {
            cell,
            kind: ReactiveDependencyKind::Reactive
        }]
    );
    assert_eq!(plan.reactive_consumers_for(cell), &[node_id]);
}

#[cfg(feature = "f64")]
#[test]
fn register_rejects_same_cell_with_conflicting_kinds() {
    let (value, _cell) = scalar(1.0);
    let mut plan = ReactivePlan::new();

    let error = plan
        .register(
            Box::new(
                TestFunction::new("conflict").with_dependency_kinds(Some(vec![
                    ReactiveDependencyKind::Sampled,
                    ReactiveDependencyKind::Reactive,
                ])),
            ),
            &[value.clone(), value],
        )
        .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyKindConflict"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_rejects_dependency_arity_mismatch() {
    let (first, _) = scalar(1.0);
    let (second, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let error = plan
        .register(
            Box::new(
                TestFunction::new("arity")
                    .with_dependency_kinds(Some(vec![ReactiveDependencyKind::Reactive])),
            ),
            &[first, second],
        )
        .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyArityMismatch"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn register_rejects_dependency_scope_arity_mismatch() {
    let (first, _) = scalar(1.0);
    let (second, _) = scalar(2.0);
    let mut plan = ReactivePlan::new();

    let error = plan
        .register(
            Box::new(
                TestFunction::new("scope arity")
                    .with_dependency_scopes(Some(vec![ReactiveDependencyScope::Recursive])),
            ),
            &[first, second],
        )
        .unwrap_err();

    assert!(format!("{:?}", error).contains("ReactiveDependencyScopeArityMismatch"));
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[cfg(feature = "f64")]
#[test]
fn activation_registration_does_not_promote_preexisting_alias_cells() {
    let trigger = LegacyValue::F64(Ref::new(0.0));
    let sampled = LegacyValue::F64(Ref::new(1.0));
    let local = LegacyValue::F64(Ref::new(2.0));
    let trigger_cell = trigger.reactive_root_cell_ids()[0];
    let sampled_cell = sampled.reactive_root_cell_ids()[0];
    let local_cell = local.reactive_root_cell_ids()[0];
    let plan = Plan::new();

    plan.push_activation_registration_scope_with_sampled_cells(
        vec![trigger_cell],
        sampled.reactive_cell_ids(),
    );
    plan.register_function(
        Box::new(TestFunction::with_output("sampled alias", sampled.clone())),
        &[],
    )
    .unwrap();
    let sampled_consumer = plan
        .register_function(Box::new(TestFunction::new("sampled consumer")), &[sampled])
        .unwrap();
    plan.register_function(
        Box::new(TestFunction::with_output("local producer", local.clone())),
        &[],
    )
    .unwrap();
    let local_consumer = plan
        .register_function(Box::new(TestFunction::new("local consumer")), &[local])
        .unwrap();
    plan.pop_activation_registration_scope();

    let plan = plan.borrow();
    assert_eq!(
        plan.node(sampled_consumer)
            .unwrap()
            .inputs
            .iter()
            .find(|dependency| dependency.cell == sampled_cell)
            .unwrap()
            .kind,
        ReactiveDependencyKind::Sampled,
    );
    assert_eq!(
        plan.node(local_consumer)
            .unwrap()
            .inputs
            .iter()
            .find(|dependency| dependency.cell == local_cell)
            .unwrap()
            .kind,
        ReactiveDependencyKind::Reactive,
    );
}
