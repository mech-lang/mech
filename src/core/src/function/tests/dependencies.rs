use super::super::{
    FunctionInstance, FunctionInvocation, ReactiveDependency, ReactiveDependencyKind,
    ReactiveDependencyScope, ReactiveNodeKind, ReactivePlan,
};
use super::support::{TestFunction, index};
use crate::ValueCell;

fn instance(function: TestFunction, output: ValueCell, inputs: Vec<ValueCell>) -> FunctionInstance {
    FunctionInstance::new(
        Box::new(function),
        FunctionInvocation::variadic(output, inputs.into_boxed_slice()),
    )
}

#[test]
fn register_outputs_are_sampled_while_sources_remain_reactive() {
    let (output, output_id) = index(1);
    let (source, source_id) = index(2);
    let function = TestFunction::with_output("register", output.clone())
        .with_node_kind(ReactiveNodeKind::Register);
    let mut plan = ReactivePlan::new();
    let node = plan
        .register_instance_with_activation(instance(function, output, vec![source]), None)
        .unwrap();

    assert_eq!(
        plan.nodes[node].inputs,
        vec![
            ReactiveDependency {
                cell: output_id,
                kind: ReactiveDependencyKind::Sampled,
            },
            ReactiveDependency {
                cell: source_id,
                kind: ReactiveDependencyKind::Reactive,
            },
        ]
    );
    assert_eq!(plan.sampled_consumers_for(output_id), &[node]);
    assert!(plan.reactive_consumers_for(output_id).is_empty());
    assert_eq!(plan.reactive_consumers_for(source_id), &[node]);
}

#[test]
fn output_input_alias_is_coalesced_to_one_sampled_dependency() {
    let (output, output_id) = index(1);
    let function = TestFunction::with_output("alias", output.clone())
        .with_node_kind(ReactiveNodeKind::Register);
    let mut plan = ReactivePlan::new();
    let node = plan
        .register_instance_with_activation(instance(function, output.clone(), vec![output]), None)
        .unwrap();

    assert_eq!(
        plan.nodes[node].inputs,
        vec![ReactiveDependency {
            cell: output_id,
            kind: ReactiveDependencyKind::Sampled,
        }]
    );
    assert!(plan.reactive_consumers_for(output_id).is_empty());
}

#[test]
fn dependency_order_kinds_and_scopes_are_canonical() {
    let output = ValueCell::unit();
    let (first, first_id) = index(1);
    let (second, second_id) = index(2);
    let function = TestFunction::with_output("ordered", output.clone())
        .with_dependency_kinds(Some(vec![
            ReactiveDependencyKind::Sampled,
            ReactiveDependencyKind::Reactive,
        ]))
        .with_dependency_scopes(Some(vec![
            ReactiveDependencyScope::Root,
            ReactiveDependencyScope::None,
        ]));
    let mut plan = ReactivePlan::new();
    let node = plan
        .register_instance_with_activation(instance(function, output, vec![first, second]), None)
        .unwrap();

    assert_eq!(
        plan.nodes[node].inputs,
        vec![ReactiveDependency {
            cell: first_id,
            kind: ReactiveDependencyKind::Sampled,
        }]
    );
    assert_eq!(plan.sampled_consumers_for(first_id), &[node]);
    assert!(plan.reactive_consumers_for(second_id).is_empty());
}

#[test]
fn duplicate_dependencies_are_deduplicated_and_conflicts_fail_atomically() {
    let output = ValueCell::unit();
    let (value, identity) = index(1);
    let mut plan = ReactivePlan::new();
    let node = plan
        .register_instance_with_activation(
            instance(
                TestFunction::with_output("dedupe", output.clone()),
                output.clone(),
                vec![value.clone(), value.clone()],
            ),
            None,
        )
        .unwrap();
    assert_eq!(
        plan.nodes[node].inputs,
        vec![ReactiveDependency {
            cell: identity,
            kind: ReactiveDependencyKind::Reactive,
        }]
    );

    let before = plan.len();
    let error = plan
        .register_instance_with_activation(
            instance(
                TestFunction::with_output("conflict", output.clone()).with_dependency_kinds(Some(
                    vec![
                        ReactiveDependencyKind::Sampled,
                        ReactiveDependencyKind::Reactive,
                    ],
                )),
                output,
                vec![value.clone(), value],
            ),
            None,
        )
        .unwrap_err();
    assert_eq!(error.kind_name(), "ReactiveDependencyKindConflict");
    assert_eq!(plan.len(), before);
}

#[test]
fn dependency_metadata_arity_errors_do_not_mutate_the_plan() {
    let output = ValueCell::unit();
    let (first, _) = index(1);
    let (second, _) = index(2);
    let mut plan = ReactivePlan::new();
    let error = plan
        .register_instance_with_activation(
            instance(
                TestFunction::with_output("arity", output.clone())
                    .with_dependency_kinds(Some(vec![ReactiveDependencyKind::Reactive])),
                output,
                vec![first, second],
            ),
            None,
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveDependencyArityMismatch");
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}

#[test]
fn dependency_scope_arity_errors_do_not_mutate_the_plan() {
    let output = ValueCell::unit();
    let (first, _) = index(1);
    let (second, _) = index(2);
    let mut plan = ReactivePlan::new();
    let error = plan
        .register_instance_with_activation(
            instance(
                TestFunction::with_output("scope arity", output.clone())
                    .with_dependency_scopes(Some(vec![ReactiveDependencyScope::Root])),
                output,
                vec![first, second],
            ),
            None,
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "ReactiveDependencyScopeArityMismatch");
    assert!(plan.nodes.is_empty());
    assert!(plan.reactive_consumers.is_empty());
    assert!(plan.sampled_consumers.is_empty());
}
