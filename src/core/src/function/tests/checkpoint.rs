#[cfg(feature = "compiler")]
use super::super::MechFunctionCompiler;
use super::super::{
    ActivationRegistrationScope, MechFunctionImpl, PatternActivationRegistration, Plan,
    ReactiveDependency, ReactiveDependencyKind, ReactiveNodeKind, ReactivePlan,
    ReactivePlanFunction, ReactiveTurnState, reactive_function_identity,
};
use super::support::TestFunction;
#[cfg(feature = "f64")]
use super::support::reg;
#[cfg(feature = "compiler")]
use crate::{BytecodeCompilerContext, Register};
use crate::{LegacyValue, MResult, ReactiveCellId, Ref, ToValue};

struct RetainedZstFunction;

impl MechFunctionImpl for RetainedZstFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }
    fn out(&self) -> LegacyValue {
        LegacyValue::Empty
    }
    fn to_string(&self) -> String {
        "retained-zst".into()
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }
}

#[cfg(feature = "compiler")]
impl MechFunctionCompiler for RetainedZstFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

#[test]
fn reactive_plan_rollback_truncates_nodes_and_rebuilds_consumers() {
    let reactive_input = LegacyValue::Index(Ref::new(1));
    let sampled_input = LegacyValue::Index(Ref::new(2));
    let reactive_cell = reactive_input.reactive_root_cell_ids()[0];
    let sampled_cell = sampled_input.reactive_root_cell_ids()[0];
    let mut plan = ReactivePlan::new();
    let base = plan
        .register(Box::new(TestFunction::new("base")), &[reactive_input])
        .unwrap();
    let checkpoint = plan.checkpoint();
    let tail = plan
        .register(
            Box::new(
                TestFunction::new("tail")
                    .with_dependency_kinds(Some(vec![ReactiveDependencyKind::Sampled])),
            ),
            &[sampled_input],
        )
        .unwrap();
    plan.rollback(checkpoint).unwrap();
    assert_eq!(plan.len(), 1);
    assert_eq!(plan.nodes[0].id, base);
    assert_eq!(plan.nodes[0].plan_index, 0);
    assert_eq!(
        plan.nodes[0].inputs,
        vec![ReactiveDependency {
            cell: reactive_cell,
            kind: ReactiveDependencyKind::Reactive
        }]
    );
    assert_eq!(plan.reactive_consumers_for(reactive_cell), &[base]);
    assert!(plan.sampled_consumers_for(sampled_cell).is_empty());
    assert!(
        plan.reactive_consumers
            .values()
            .all(|nodes| !nodes.contains(&tail))
    );
    assert!(
        plan.sampled_consumers
            .values()
            .all(|nodes| !nodes.contains(&tail))
    );
}

#[test]
fn plan_rollback_restores_full_structure_and_rebuilds_consumers() {
    let input = LegacyValue::Index(Ref::new(1));
    let original_cell = input.reactive_root_cell_ids()[0];
    let replacement_cell = ReactiveCellId::new(99);
    let sampled_cell = ReactiveCellId::new(100);
    let plan = Plan::new();
    let base = plan
        .borrow_mut()
        .register(Box::new(TestFunction::new("base")), &[input])
        .unwrap();
    plan.push_activation_registration_scope_with_sampled_cells(
        vec![original_cell],
        vec![sampled_cell],
    );
    {
        let mut reactive = plan.borrow_mut();
        reactive.register_pattern_activation(PatternActivationRegistration {
            scope_pulse_node: base,
            selector_node: base,
            arms: Vec::new(),
        });
    }
    let checkpoint = plan.checkpoint();
    let function_identity = {
        let reactive = plan.borrow();
        reactive_function_identity(&reactive.nodes[base].function)
    };

    {
        let mut reactive = plan.borrow_mut();
        let node = &mut reactive.nodes[base];
        node.id = 42;
        node.plan_index = 77;
        node.inputs = vec![ReactiveDependency {
            cell: replacement_cell,
            kind: ReactiveDependencyKind::Sampled,
        }];
        node.outputs = vec![replacement_cell];
        node.kind = ReactiveNodeKind::Register;
        reactive.pattern_activation_registrations.clear();
        reactive.activation_sampled_cells = vec![vec![replacement_cell]];
        reactive.push(Box::new(TestFunction::new("tail")));
    }
    {
        let mut scopes = plan.1.borrow_mut();
        scopes[0].trigger_cells = vec![replacement_cell];
        scopes[0].local_combinational_cells = vec![replacement_cell];
        scopes.push(ActivationRegistrationScope {
            trigger_cells: vec![replacement_cell],
            local_combinational_cells: Vec::new(),
        });
    }

    plan.preflight_rollback(&checkpoint).unwrap();
    plan.apply_rollback(&checkpoint);

    assert_eq!(plan.checkpoint(), checkpoint);
    let reactive = plan.borrow();
    assert_eq!(reactive.len(), 1);
    assert_eq!(
        reactive_function_identity(&reactive.nodes[base].function),
        function_identity,
    );
    assert_eq!(reactive.reactive_consumers_for(original_cell), &[base]);
    assert!(reactive.sampled_consumers_for(replacement_cell).is_empty());
    assert_eq!(reactive.activation_sampled_cells, vec![vec![sampled_cell]],);
}

#[test]
fn plan_rollback_restores_activation_registration_depth() {
    let plan = Plan::new();
    let checkpoint = plan.checkpoint();
    plan.push_activation_registration_scope(vec![ReactiveCellId::new(1)]);
    plan.push_activation_registration_scope(vec![ReactiveCellId::new(2)]);
    assert_eq!(plan.activation_registration_depth(), 2);
    plan.rollback(checkpoint).unwrap();
    assert_eq!(plan.activation_registration_depth(), 0);
}

#[test]
fn plan_rollback_rejects_removed_node_without_partial_restore() {
    let plan = Plan::new();
    plan.add_function(Box::new(TestFunction::new("retained")));
    let checkpoint = plan.checkpoint();
    plan.borrow_mut().nodes.pop();
    plan.push_activation_registration_scope(vec![ReactiveCellId::new(1)]);

    let error = plan.rollback(checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "ReactivePlanRollbackInvariant");
    assert_eq!(plan.len(), 0);
    assert_eq!(plan.activation_registration_depth(), 1);
}

#[test]
fn plan_rollback_rejects_replaced_function_without_partial_restore() {
    let plan = Plan::new();
    let node_id = plan.add_function(Box::new(TestFunction::new("retained")));
    let checkpoint = plan.checkpoint();
    {
        let mut reactive = plan.borrow_mut();
        reactive.nodes[node_id].plan_index = 123;
        reactive.nodes[node_id].function =
            ReactivePlanFunction::new(Box::new(TestFunction::new("replacement")));
    }
    plan.push_activation_registration_scope(vec![ReactiveCellId::new(1)]);

    let error = plan.rollback(checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "ReactivePlanFunctionIdentity");
    assert_eq!(plan.borrow().nodes[node_id].plan_index, 123);
    assert_eq!(plan.activation_registration_depth(), 1);
}

#[test]
fn plan_rollback_rejects_replaced_zero_sized_function_box() {
    let plan = Plan::new();
    let node_id = plan.add_function(Box::new(RetainedZstFunction));
    let checkpoint = plan.checkpoint();
    let saved_identity = {
        let reactive = plan.borrow();
        reactive_function_identity(&reactive.nodes[node_id].function)
    };
    {
        let mut reactive = plan.borrow_mut();
        reactive.nodes[node_id].function = ReactivePlanFunction::new(Box::new(RetainedZstFunction));
    }
    let replacement_identity = {
        let reactive = plan.borrow();
        reactive_function_identity(&reactive.nodes[node_id].function)
    };

    assert_ne!(saved_identity, replacement_identity);
    let error = plan.rollback(checkpoint).unwrap_err();

    assert_eq!(error.kind_name(), "ReactivePlanFunctionIdentity");
}

#[cfg(feature = "f64")]
#[test]
fn checkpoint_validation_accepts_duplicate_registers_from_retried_failed_turn() {
    let plan = Plan::new();
    let input = Ref::new(1.);
    let sink = Ref::new(1.);
    let (register, _, stage, _) = {
        let mut reactive = plan.borrow_mut();
        reg(&mut reactive, input.clone(), sink, true)
    };
    let dirty = input.to_value().reactive_root_cell_ids();
    let mut state = ReactiveTurnState::default();

    assert!(plan.advance_reactive_turn(&mut state, &dirty).is_err());
    assert_eq!(state.pending_register_nodes, vec![register]);
    assert!(plan.advance_reactive_turn(&mut state, &dirty).is_err());
    assert_eq!(state.pending_register_nodes, vec![register, register]);
    assert_eq!(*stage.borrow(), 2);

    plan.validate_checkpoint_turn_state(&state).unwrap();
}
