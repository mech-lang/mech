use std::collections::HashSet;

use mech_core::{
    ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, ReactiveTurnOutcome,
    Ref, Value,
};

use super::super::{MechRuntime, RuntimePersistentSendSchedule};
use super::persistent_send::{publish, runtime_with_console, snapshot};
use crate::runtime::execution::ACTIVATION_EFFECT_BARRIER_NAME;
use crate::runtime::test_support::{
    capabilities::{grant_read, grant_write},
    providers::{
        RecordingTestOutput, TEST_OUTPUT_BASE_URI, TestResourceProvider, sleep_host,
        test_provider_with, test_runtime, test_runtime_with_output, test_runtime_with_output_host,
    },
    values::{
        combinational_node_for_output_and_inputs, f64_value, register_node_for_symbol, source_cell,
        source_value, symbol_cell, symbol_value,
    },
};
use crate::{
    RuntimeHostInput, RuntimeHostInputOutcome, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue,
};

const TEST_CLOCK_BASE_URI: &str = "test://clock/ticks";
const TEST_SIGNALS_BASE_URI: &str = "test://signals/inputs";

#[test]
fn runtime_reactive_host_input_batches_bound_updates_into_one_turn() {
    let provider = TestResourceProvider::new()
        .with_value(TEST_CLOCK_BASE_URI, "a", Value::F64(Ref::new(1.0)))
        .with_value(TEST_CLOCK_BASE_URI, "b", Value::F64(Ref::new(2.0)));
    let mut runtime = test_runtime(provider);
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "a");
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "b");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(a), :read(b)}\nsum := @pulse/a + @pulse/b",
        )
        .unwrap();
    let a_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "a").unwrap();
    let b_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "b").unwrap();
    let sum_node = combinational_node_for_output_and_inputs(
        &runtime,
        symbol_cell(&runtime, "sum"),
        &[
            source_cell(&runtime, &a_source),
            source_cell(&runtime, &b_source),
        ],
    );
    let root_interpreter_id = runtime.program.interpreter().id;
    let outcome = runtime
        .apply_host_input(
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: a_source.clone(),
                    value: RuntimeHostInputValue::F64(10.0),
                },
                RuntimeHostInputUpdate {
                    source: b_source.clone(),
                    value: RuntimeHostInputValue::F64(20.0),
                },
            ])
            .unwrap(),
        )
        .unwrap();
    let program_turn = outcome.turn.as_ref().unwrap();
    let interpreter_turn = &program_turn.interpreter_turns[0];
    let reactive_turn = &interpreter_turn.turn;
    assert_eq!(outcome.update_count, 2);
    assert_eq!(outcome.ignored_update_count, 0);
    assert_eq!(outcome.binding_count, 2);
    assert_eq!(program_turn.updated_count, 2);
    assert_eq!(program_turn.interpreter_turns.len(), 1);
    assert_eq!(interpreter_turn.interpreter_id, root_interpreter_id);
    assert!(!interpreter_turn.dirty_cells.is_empty());
    assert_eq!(
        reactive_turn
            .before_commit
            .executed_nodes
            .iter()
            .filter(|node| **node == sum_node)
            .count(),
        1
    );
    assert!(reactive_turn.register_commit.staged_nodes.is_empty());
    assert!(reactive_turn.register_commit.committed_nodes.is_empty());
    assert!(reactive_turn.register_commit.dirty_cells.is_empty());
    assert_eq!(f64_value(&source_value(&runtime, &a_source)), 10.0);
    assert_eq!(f64_value(&source_value(&runtime, &b_source)), 20.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "sum")), 30.0);
}

#[test]
fn runtime_reactive_host_input_unbound_packet_does_not_advance_pending_registers() {
    let (mut runtime, output) =
        test_runtime_with_output(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0));
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\n~a := 0.0\n~b := 0.0\na = @pulse/value\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0\n@out/line <- output").unwrap();
    assert_eq!(output.lines().len(), 1);
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    runtime
        .apply_host_input(RuntimeHostInput::single(
            source,
            RuntimeHostInputValue::F64(10.0),
        ))
        .unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "b")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 3.0);
    assert!(
        runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    assert_eq!(output.lines().len(), 2);
    let b_before = f64_value(&symbol_value(&runtime, "b"));
    let output_before = f64_value(&symbol_value(&runtime, "output"));
    let lines_before = output.lines();
    let outcome = runtime
        .apply_host_input(
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing-a").unwrap(),
                    value: RuntimeHostInputValue::F64(5.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing-b").unwrap(),
                    value: RuntimeHostInputValue::F64(9.0),
                },
            ])
            .unwrap(),
        )
        .unwrap();
    assert_eq!(outcome.update_count, 2);
    assert_eq!(outcome.ignored_update_count, 2);
    assert_eq!(outcome.binding_count, 0);
    assert!(outcome.turn.is_none());
    assert_eq!(f64_value(&symbol_value(&runtime, "b")), b_before);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), output_before);
    assert!(
        runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    assert_eq!(output.lines(), lines_before);
}

#[test]
fn runtime_reactive_host_input_preserves_deferred_registers_across_packets() {
    let mut runtime = test_runtime(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0));
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\n~a := 0.0\n~b := 0.0\na = @pulse/value\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0").unwrap();
    let a = register_node_for_symbol(&runtime, "a");
    let b = register_node_for_symbol(&runtime, "b");
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "a")), 1.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "middle")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "b")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 3.0);
    let first = runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(10.0),
        ))
        .unwrap();
    let turn = &first.turn.as_ref().unwrap().interpreter_turns[0].turn;
    assert_eq!(turn.register_commit.committed_nodes, vec![a]);
    assert_eq!(turn.after_commit.pending_register_nodes, vec![b]);
    assert_eq!(
        (
            f64_value(&symbol_value(&runtime, "a")),
            f64_value(&symbol_value(&runtime, "middle")),
            f64_value(&symbol_value(&runtime, "b")),
            f64_value(&symbol_value(&runtime, "output"))
        ),
        (10.0, 11.0, 2.0, 3.0)
    );
    assert!(
        runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    let second = runtime
        .apply_host_input(RuntimeHostInput::single(
            source,
            RuntimeHostInputValue::F64(20.0),
        ))
        .unwrap();
    let turn = &second.turn.as_ref().unwrap().interpreter_turns[0].turn;
    assert_eq!(turn.before_commit.pending_register_nodes, vec![a]);
    assert_eq!(turn.register_commit.committed_nodes, vec![a, b]);
    assert_eq!(turn.after_commit.pending_register_nodes, vec![b]);
    assert_eq!(
        (
            f64_value(&symbol_value(&runtime, "a")),
            f64_value(&symbol_value(&runtime, "middle")),
            f64_value(&symbol_value(&runtime, "b")),
            f64_value(&symbol_value(&runtime, "output"))
        ),
        (20.0, 21.0, 11.0, 12.0)
    );
    assert!(
        runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ActivationPlanSnapshot {
    nodes: Vec<ActivationNodeSnapshot>,
    reactive_consumers: Vec<(ReactiveCellId, Vec<ReactiveNodeId>)>,
    sampled_consumers: Vec<(ReactiveCellId, Vec<ReactiveNodeId>)>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
struct ActivationNodeSnapshot {
    id: ReactiveNodeId,
    plan_index: usize,
    kind: ReactiveNodeKind,
    inputs: Vec<(ReactiveCellId, ReactiveDependencyKind)>,
    outputs: Vec<ReactiveCellId>,
}
pub(super) fn activation_plan_snapshot(runtime: &MechRuntime) -> ActivationPlanSnapshot {
    let plan = runtime.program.interpreter().plan();
    let plan = plan.borrow();
    let nodes = plan
        .nodes
        .iter()
        .map(|n| ActivationNodeSnapshot {
            id: n.id,
            plan_index: n.plan_index,
            kind: n.kind,
            inputs: n.inputs.iter().map(|d| (d.cell, d.kind)).collect(),
            outputs: n.outputs.clone(),
        })
        .collect();
    let mut reactive_consumers = plan
        .reactive_consumers
        .iter()
        .map(|(c, n)| (*c, n.clone()))
        .collect::<Vec<_>>();
    let mut sampled_consumers = plan
        .sampled_consumers
        .iter()
        .map(|(c, n)| (*c, n.clone()))
        .collect::<Vec<_>>();
    reactive_consumers.sort_by_key(|(c, _)| c.get());
    sampled_consumers.sort_by_key(|(c, _)| c.get());
    ActivationPlanSnapshot {
        nodes,
        reactive_consumers,
        sampled_consumers,
    }
}
fn activation_nodes_for_trigger(
    runtime: &MechRuntime,
    trigger_name: &str,
    kind: ReactiveNodeKind,
) -> Vec<ReactiveNodeId> {
    let c = symbol_cell(runtime, trigger_name);
    let p = runtime.program.interpreter().plan();
    p.borrow()
        .nodes
        .iter()
        .filter(|n| {
            n.kind == kind
                && n.inputs
                    .iter()
                    .any(|d| d.cell == c && d.kind == ReactiveDependencyKind::Reactive)
        })
        .map(|n| n.id)
        .collect()
}
fn activation_barrier_for_trigger(runtime: &MechRuntime, trigger_name: &str) -> ReactiveNodeId {
    let c = symbol_cell(runtime, trigger_name);
    let p = runtime.program.interpreter().plan();
    let barriers = p
        .borrow()
        .nodes
        .iter()
        .filter(|n| {
            n.kind == ReactiveNodeKind::Combinational
                && n.function.to_string() == ACTIVATION_EFFECT_BARRIER_NAME
                && n.outputs.is_empty()
                && n.inputs
                    .iter()
                    .any(|d| d.cell == c && d.kind == ReactiveDependencyKind::Reactive)
        })
        .map(|n| n.id)
        .collect::<Vec<_>>();
    assert_eq!(
        barriers.len(),
        1,
        "expected exactly one activation-effect barrier for trigger {trigger_name}"
    );
    barriers[0]
}
pub(super) fn only_reactive_turn(outcome: &RuntimeHostInputOutcome) -> &ReactiveTurnOutcome {
    let p = outcome
        .turn
        .as_ref()
        .expect("expected a program input turn");
    assert_eq!(
        p.interpreter_turns.len(),
        1,
        "expected exactly one affected interpreter"
    );
    &p.interpreter_turns[0].turn
}
fn executed_count(turn: &ReactiveTurnOutcome, id: ReactiveNodeId) -> usize {
    turn.before_commit
        .executed_nodes
        .iter()
        .chain(turn.after_commit.executed_nodes.iter())
        .filter(|x| **x == id)
        .count()
}
pub(super) fn apply_f64_input(
    r: &mut MechRuntime,
    b: &str,
    p: &str,
    v: f64,
) -> RuntimeHostInputOutcome {
    r.apply_host_input(RuntimeHostInput::single(
        RuntimeHostInputSource::new(b, p).unwrap(),
        RuntimeHostInputValue::F64(v),
    ))
    .unwrap()
}
pub(super) fn recorded_f64(o: &RecordingTestOutput, i: usize) -> f64 {
    o.lines()[i].trim().parse().unwrap()
}
pub(super) fn activation_send_count(r: &MechRuntime) -> usize {
    r.persistent_sends
        .iter()
        .filter(|s| matches!(s.schedule, RuntimePersistentSendSchedule::Activation { .. }))
        .count()
}

#[test]
fn activation_send_snapshots_fixed_payloads_before_same_trigger_register_commit() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) =
        test_runtime_with_output_host(provider, sleep_host("demo/activation-duration-sleep"));
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~state := 0.0

~> render-tick {
state = state + 1.0
}

~> render-tick {
@out/line <- state
@out/line <- state + 0.0
}
"#,
        )
        .unwrap();

    let plan = activation_plan_snapshot(&runtime);
    let register = register_node_for_symbol(&runtime, "state");
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 2);

    let first = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert_eq!(
        only_reactive_turn(&first).register_commit.committed_nodes,
        vec![register]
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0", "0"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let equal = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert_eq!(
        only_reactive_turn(&equal).register_commit.committed_nodes,
        vec![register]
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 2.0);
    assert_eq!(output.lines(), vec!["0", "0", "1", "1"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
}

#[test]
fn patterned_activation_sends_only_from_the_selected_arm() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
| 99.0 => {
    @out/line <- 99.0
  }
| selected, selected > 0.0 => {
    @out/line <- selected
  }
| * => {
    @out/line <- -1.0
  }
"#,
        )
        .unwrap();
    assert!(
        output.lines().is_empty(),
        "patterned effects ran during load"
    );
    assert_eq!(activation_send_count(&runtime), 3);
    let barriers = runtime
        .persistent_sends
        .iter()
        .map(|send| match send.schedule {
            RuntimePersistentSendSchedule::Activation {
                barrier_node_id, ..
            } => barrier_node_id,
            RuntimePersistentSendSchedule::EveryAcceptedTurn => {
                panic!("patterned activation send used top-level schedule")
            }
        })
        .collect::<Vec<_>>();
    assert_eq!(
        barriers.iter().copied().collect::<HashSet<_>>().len(),
        3,
        "each effectful arm must own a distinct barrier"
    );
    let plan = activation_plan_snapshot(&runtime);
    let first = apply_f64_input(&mut runtime, "test://render/timer", "tick", 5.0);
    let turn = only_reactive_turn(&first);
    assert_eq!(output.lines(), vec!["5"]);
    assert_eq!(
        (
            executed_count(turn, barriers[0]),
            executed_count(turn, barriers[1]),
            executed_count(turn, barriers[2])
        ),
        (0, 1, 0)
    );
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    let equal = apply_f64_input(&mut runtime, "test://render/timer", "tick", 5.0);
    let turn = only_reactive_turn(&equal);
    assert_eq!(output.lines(), vec!["5", "5"]);
    assert_eq!(
        (
            executed_count(turn, barriers[0]),
            executed_count(turn, barriers[1]),
            executed_count(turn, barriers[2])
        ),
        (0, 1, 0)
    );
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    let fallback = apply_f64_input(&mut runtime, "test://render/timer", "tick", -5.0);
    let turn = only_reactive_turn(&fallback);
    assert_eq!(output.lines(), vec!["5", "5", "-1"]);
    assert_eq!(
        (
            executed_count(turn, barriers[0]),
            executed_count(turn, barriers[1]),
            executed_count(turn, barriers[2])
        ),
        (0, 0, 1)
    );
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 3);
}

#[test]
fn patterned_activation_samples_outer_effect_values_only_on_its_trigger() {
    let provider = TestResourceProvider::new()
        .with_value("test://render/timer", "tick", Value::F64(Ref::new(0.0)))
        .with_value(TEST_SIGNALS_BASE_URI, "value", Value::F64(Ref::new(1.0)));
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "value");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@signals := test://signals/inputs{:read(value)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
scene := @signals/value
~> render-tick
| *, scene > 0.0 => {
    @out/line <- scene
  }
| * => {}
"#,
        )
        .unwrap();

    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let barrier = runtime
        .persistent_sends
        .iter()
        .find_map(|send| match send.schedule {
            RuntimePersistentSendSchedule::Activation {
                barrier_node_id, ..
            } => Some(barrier_node_id),
            RuntimePersistentSendSchedule::EveryAcceptedTurn => None,
        })
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);

    let scene_only = apply_f64_input(&mut runtime, TEST_SIGNALS_BASE_URI, "value", -1.0);
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let guard_false = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&guard_false), barrier), 0);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let scene_only = apply_f64_input(&mut runtime, TEST_SIGNALS_BASE_URI, "value", 10.0);
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);

    let render = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert_eq!(output.lines(), vec!["10"]);
    assert_eq!(executed_count(only_reactive_turn(&render), barrier), 1);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let scene_only = apply_f64_input(&mut runtime, TEST_SIGNALS_BASE_URI, "value", 20.0);
    assert_eq!(output.lines(), vec!["10"]);
    assert_eq!(executed_count(only_reactive_turn(&scene_only), barrier), 0);

    let equal_render = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert_eq!(output.lines(), vec!["10", "20"]);
    assert_eq!(
        executed_count(only_reactive_turn(&equal_render), barrier),
        1
    );
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 1);
}

#[test]
fn activation_two_clock_physics_render_acceptance() {
    let provider = TestResourceProvider::new()
        .with_value("test://physics/timer", "tick", Value::F64(Ref::new(0.0)))
        .with_value("test://render/timer", "tick", Value::F64(Ref::new(0.0)));
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://physics/timer", "tick");
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@physics := test://physics/timer{:read(tick)}
@render := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
physics-tick := @physics/tick
render-tick := @render/tick
~x := 0.0
~> physics-tick {
next-x := x + 1.0
x = next-x
}
~> render-tick {
@out/line <- x
}
"#,
        )
        .unwrap();
    let initial_plan = activation_plan_snapshot(&runtime);
    let render_barrier = activation_barrier_for_trigger(&runtime, "render-tick");
    let physics_combinational_nodes =
        activation_nodes_for_trigger(&runtime, "physics-tick", ReactiveNodeKind::Combinational);
    let x_register = register_node_for_symbol(&runtime, "x");
    assert!(!physics_combinational_nodes.is_empty());
    assert_eq!(f64_value(&symbol_value(&runtime, "x")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let a = apply_f64_input(&mut runtime, "test://physics/timer", "tick", 1.0);
    let t = only_reactive_turn(&a);
    assert_eq!(f64_value(&symbol_value(&runtime, "x")), 1.0);
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(t, render_barrier), 0);
    for n in &physics_combinational_nodes {
        assert_eq!(executed_count(t, *n), 1)
    }
    assert_eq!(t.register_commit.committed_nodes, vec![x_register]);
    assert_eq!(activation_plan_snapshot(&runtime), initial_plan);
    assert_eq!(activation_send_count(&runtime), 1);
    let a = apply_f64_input(&mut runtime, "test://physics/timer", "tick", 2.0);
    let t = only_reactive_turn(&a);
    assert_eq!(f64_value(&symbol_value(&runtime, "x")), 2.0);
    assert!(output.lines().is_empty());
    assert_eq!(executed_count(t, render_barrier), 0);
    for n in &physics_combinational_nodes {
        assert_eq!(executed_count(t, *n), 1)
    }
    assert_eq!(t.register_commit.committed_nodes, vec![x_register]);
    assert_eq!(activation_plan_snapshot(&runtime), initial_plan);
    assert_eq!(activation_send_count(&runtime), 1);
    let a = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    let t = only_reactive_turn(&a);
    assert_eq!(f64_value(&symbol_value(&runtime, "x")), 2.0);
    assert_eq!(output.lines().len(), 1);
    assert_eq!(recorded_f64(&output, 0), 2.0);
    assert_eq!(executed_count(t, render_barrier), 1);
    for n in &physics_combinational_nodes {
        assert_eq!(executed_count(t, *n), 0)
    }
    assert!(!t.before_commit.pending_register_nodes.contains(&x_register));
    assert!(!t.register_commit.staged_nodes.contains(&x_register));
    assert!(!t.register_commit.committed_nodes.contains(&x_register));
    assert!(!t.after_commit.pending_register_nodes.contains(&x_register));
    assert_eq!(activation_plan_snapshot(&runtime), initial_plan);
    assert_eq!(activation_send_count(&runtime), 1);
    let a = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "x")), 2.0);
    assert_eq!(output.lines().len(), 2);
    assert_eq!(recorded_f64(&output, 0), 2.0);
    assert_eq!(recorded_f64(&output, 1), 2.0);
    assert_eq!(executed_count(only_reactive_turn(&a), render_barrier), 1);
    assert_eq!(activation_plan_snapshot(&runtime), initial_plan);
    assert_eq!(activation_send_count(&runtime), 1);
}
#[test]
fn activation_send_samples_latest_value_and_ignores_other_updates() {
    let (mut r, o) = test_runtime_with_output(
        TestResourceProvider::new()
            .with_value("test://render/timer", "tick", Value::F64(Ref::new(0.0)))
            .with_value("test://other/timer", "tick", Value::F64(Ref::new(0.0)))
            .with_value(TEST_SIGNALS_BASE_URI, "value", Value::F64(Ref::new(1.0))),
    );
    for (b, p) in [
        ("test://render/timer", "tick"),
        ("test://other/timer", "tick"),
        (TEST_SIGNALS_BASE_URI, "value"),
    ] {
        grant_read(&mut r, b, p)
    }
    grant_write(&mut r, TEST_OUTPUT_BASE_URI, "line");
    r.run_string(
        r#"@render := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@signals := test://signals/inputs{:read(value)}
@out := test://effects/output{:write(line)}
render-tick := @render/tick
other-tick := @other/tick
sampled-value := @signals/value
~> render-tick {
@out/line <- sampled-value
}
~> other-tick {
other-result := sampled-value + 1.0
}
"#,
    )
    .unwrap();
    let b = activation_barrier_for_trigger(&r, "render-tick");
    let ns = activation_nodes_for_trigger(&r, "other-tick", ReactiveNodeKind::Combinational);
    assert!(!ns.is_empty());
    assert!(o.lines().is_empty());
    let q = apply_f64_input(&mut r, TEST_SIGNALS_BASE_URI, "value", 10.0);
    assert!(o.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&q), b), 0);
    let q = apply_f64_input(&mut r, "test://other/timer", "tick", 1.0);
    assert!(o.lines().is_empty());
    assert_eq!(executed_count(only_reactive_turn(&q), b), 0);
    for n in ns {
        assert_eq!(executed_count(only_reactive_turn(&q), n), 1)
    }
    let q = apply_f64_input(&mut r, "test://render/timer", "tick", 1.0);
    assert_eq!(o.lines().len(), 1);
    assert_eq!(recorded_f64(&o, 0), 10.0);
    assert_eq!(executed_count(only_reactive_turn(&q), b), 1);
    let q = apply_f64_input(&mut r, TEST_SIGNALS_BASE_URI, "value", 20.0);
    assert_eq!(o.lines().len(), 1);
    assert_eq!(executed_count(only_reactive_turn(&q), b), 0);
    let q = apply_f64_input(&mut r, "test://render/timer", "tick", 1.0);
    assert_eq!(o.lines().len(), 2);
    assert_eq!(recorded_f64(&o, 0), 10.0);
    assert_eq!(recorded_f64(&o, 1), 20.0);
    assert_eq!(executed_count(only_reactive_turn(&q), b), 1);
}

#[test]
fn activation_send_registers_one_barrier_per_scope_and_replays_equal_triggers() {
    let (mut runtime, driver, console) = runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    runtime
        .run_string(
            r#"@out := console://console/output{:write(line)}
@clock := time://clock/clock{:read(hour)}
render-tick := @clock/hour
~> render-tick {
@out/line <- "first"
@out/line <- "second"
@out/line <- "third"
}
"#,
        )
        .unwrap();

    // Activation effects are registered, rather than evaluated, during load.
    assert!(console.lines().is_empty());
    assert_eq!(runtime.persistent_send_count(), 3);
    let schedules: Vec<_> = runtime
        .persistent_sends
        .iter()
        .map(|send| match send.schedule {
            RuntimePersistentSendSchedule::Activation {
                barrier_node_id, ..
            } => barrier_node_id,
            RuntimePersistentSendSchedule::EveryAcceptedTurn => {
                panic!("activation send used top-level schedule")
            }
        })
        .collect();
    assert!(schedules.windows(2).all(|ids| ids[0] == ids[1]));
    let barriers = runtime
        .program
        .interpreter()
        .plan()
        .borrow()
        .nodes
        .iter()
        .filter(|node| {
            node.kind == mech_core::ReactiveNodeKind::Combinational
                && node.function.to_string() == ACTIVATION_EFFECT_BARRIER_NAME
                && node.outputs.is_empty()
        })
        .count();
    assert_eq!(barriers, 1);

    // Equal admitted values still execute the barrier and replay every send.
    publish(&mut runtime, &driver, snapshot(5.0, 2.0, 3.0, 4.0));
    publish(&mut runtime, &driver, snapshot(5.0, 2.0, 3.0, 4.0));
    assert_eq!(
        console.lines(),
        vec![
            "\"first\"",
            "\"second\"",
            "\"third\"",
            "\"first\"",
            "\"second\"",
            "\"third\""
        ]
    );
}

#[test]
fn activation_internal_barrier_is_not_user_callable() {
    let (mut runtime, _driver, _console) =
        runtime_with_console(snapshot(1.0, 2.0, 3.0, 4.0), false);
    let error = runtime
        .run_string("mech/runtime/activation-effect-barrier()")
        .unwrap_err();
    assert!(format!("{error:?}").contains("MissingFunction"));
}
