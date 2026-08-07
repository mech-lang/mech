use crate::{
    Interpreter, ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, hash_str,
};
use std::collections::HashSet;

const PURE: &str = "tick := 0.0\nx := 10.0\nradius := 2.0\n~> tick {\n left := x - radius\n doubled := left * 2.0\n}";
const REGISTER: &str = "tick := 0.0\n~x := 10.0\n\n~> tick {\n  next-x := x + 1.0\n  x = next-x\n}";
const TWO_REGISTERS: &str = "tick := 0.0\n\n~x := 0.0\n~y := 0.0\n\n~> tick {\n  next-x := x + 1.0\n  next-y := y + 2.0\n\n  x = next-x\n  y = next-y\n}";
fn interpret(source: &str) -> Interpreter {
    let t = mech_syntax::parser::parse(source).unwrap();
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    i.interpret(&t).unwrap();
    i
}
fn load() -> Interpreter {
    interpret(PURE)
}
fn cell(i: &Interpreter, n: &str) -> ReactiveCellId {
    i.symbols()
        .borrow()
        .get(hash_str(n))
        .unwrap()
        .borrow()
        .reactive_root_cell_ids()[0]
}
fn value(i: &Interpreter, n: &str) -> f64 {
    *i.symbols()
        .borrow()
        .get(hash_str(n))
        .unwrap()
        .borrow()
        .as_f64()
        .unwrap()
        .borrow()
}
fn nodes_for_output(i: &Interpreter, name: &str, kind: ReactiveNodeKind) -> Vec<ReactiveNodeId> {
    let output = cell(i, name);
    let p = i.plan();
    p.borrow()
        .nodes
        .iter()
        .filter(|n| n.kind == kind && n.outputs.contains(&output))
        .map(|n| n.id)
        .collect()
}
fn unique_register_for(i: &Interpreter, name: &str) -> ReactiveNodeId {
    let found = nodes_for_output(i, name, ReactiveNodeKind::Register);
    assert_eq!(found.len(), 1, "expected one register node for {name}");
    found[0]
}
fn activation_nodes(
    i: &Interpreter,
    trigger_name: &str,
    kind: ReactiveNodeKind,
) -> Vec<ReactiveNodeId> {
    let trigger = cell(i, trigger_name);
    let p = i.plan();
    p.borrow()
        .nodes
        .iter()
        .filter(|node| {
            node.kind == kind
                && node.inputs.iter().any(|dependency| {
                    dependency.cell == trigger
                        && dependency.kind == ReactiveDependencyKind::Reactive
                })
        })
        .map(|node| node.id)
        .collect()
}
fn combinational_node_for_output_and_dependency(
    i: &Interpreter,
    output_name: &str,
    input_name: &str,
    input_kind: ReactiveDependencyKind,
) -> ReactiveNodeId {
    let output = cell(i, output_name);
    let input = cell(i, input_name);
    let p = i.plan();
    let found = p
        .borrow()
        .nodes
        .iter()
        .filter(|node| {
            node.kind == ReactiveNodeKind::Combinational
                && node.outputs.contains(&output)
                && node
                    .inputs
                    .iter()
                    .any(|dependency| dependency.cell == input && dependency.kind == input_kind)
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(
        found.len(),
        1,
        "expected one combinational node for {output_name} consuming {input_name}"
    );
    found[0]
}
fn nodes(i: &Interpreter) -> Vec<ReactiveNodeId> {
    activation_nodes(i, "tick", ReactiveNodeKind::Combinational)
}
fn two_register_nodes(i: &Interpreter) -> Vec<ReactiveNodeId> {
    let registers = activation_nodes(i, "tick", ReactiveNodeKind::Register);
    assert_eq!(registers.len(), 2, "exactly two activation registers");
    let p = i.plan();
    let p = p.borrow();
    let outputs = registers
        .iter()
        .flat_map(|id| p.node(*id).unwrap().outputs.iter())
        .copied()
        .collect::<HashSet<_>>();
    assert_eq!(outputs, [cell(i, "x"), cell(i, "y")].into_iter().collect());
    registers
}
fn snapshot(
    i: &Interpreter,
) -> (
    usize,
    Vec<(
        usize,
        usize,
        ReactiveNodeKind,
        Vec<u64>,
        Vec<(u64, ReactiveDependencyKind)>,
    )>,
    Vec<(u64, Vec<usize>)>,
    Vec<(u64, Vec<usize>)>,
) {
    let p = i.plan();
    let p = p.borrow();
    let nodes = p
        .nodes
        .iter()
        .map(|n| {
            (
                n.id,
                n.plan_index,
                n.kind,
                n.outputs.iter().map(|c| c.get()).collect(),
                n.inputs.iter().map(|d| (d.cell.get(), d.kind)).collect(),
            )
        })
        .collect();
    let mut reactive = p
        .reactive_consumers
        .iter()
        .map(|(c, n)| (c.get(), n.clone()))
        .collect::<Vec<_>>();
    let mut sampled = p
        .sampled_consumers
        .iter()
        .map(|(c, n)| (c.get(), n.clone()))
        .collect::<Vec<_>>();
    reactive.sort_by_key(|(c, _)| *c);
    sampled.sort_by_key(|(c, _)| *c);
    (p.len(), nodes, reactive, sampled)
}
#[test]
fn activation_scope_does_not_execute_during_load() {
    let i = interpret(REGISTER);
    let (next_x, register) = (
        nodes_for_output(&i, "next-x", ReactiveNodeKind::Combinational),
        unique_register_for(&i, "x"),
    );
    assert_eq!(value(&i, "x"), 10.);
    assert!(!next_x.is_empty());
    assert_eq!(
        i.plan()
            .borrow()
            .nodes
            .iter()
            .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&cell(&i, "x")))
            .map(|n| n.id)
            .collect::<Vec<_>>(),
        vec![register]
    );
    assert!(!i.has_pending_reactive_registers());
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_trigger_is_reactive() {
    let i = load();
    let t = cell(&i, "tick");
    let p = i.plan();
    assert!(nodes(&i).iter().all(|n| {
        p.borrow()
            .node(*n)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == t && d.kind == ReactiveDependencyKind::Reactive)
    }));
}
#[test]
fn activation_scope_external_inputs_are_sampled() {
    let i = load();
    let left = combinational_node_for_output_and_dependency(
        &i,
        "left",
        "x",
        ReactiveDependencyKind::Sampled,
    );
    let p = i.plan();
    let p = p.borrow();
    for input in ["x", "radius"] {
        let c = cell(&i, input);
        assert!(
            p.node(left)
                .unwrap()
                .inputs
                .iter()
                .any(|d| d.cell == c && d.kind == ReactiveDependencyKind::Sampled)
        );
        assert!(p.sampled_consumers_for(c).contains(&left));
        assert!(!p.reactive_consumers_for(c).contains(&left));
    }
}
#[test]
fn activation_scope_local_outputs_are_reactive() {
    let i = load();
    let p = i.plan();
    assert!(
        p.borrow()
            .node(combinational_node_for_output_and_dependency(
                &i,
                "doubled",
                "left",
                ReactiveDependencyKind::Reactive
            ))
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "left") && d.kind == ReactiveDependencyKind::Reactive)
    );
}
#[test]
fn activation_scope_runs_once_on_trigger() {
    let mut i = load();
    let body = nodes(&i);
    let t = cell(&i, "tick");
    let o = i.advance_reactive_turn(&[t]).unwrap();
    let executed = o
        .before_commit
        .executed_nodes
        .iter()
        .chain(o.after_commit.executed_nodes.iter())
        .copied()
        .collect::<Vec<_>>();
    let unique = executed.iter().copied().collect::<HashSet<_>>();
    assert_eq!(unique.len(), executed.len(), "no node runs twice");
    for node in body {
        assert_eq!(
            executed.iter().filter(|id| **id == node).count(),
            1,
            "body node {node} runs exactly once"
        );
    }
    assert_eq!((value(&i, "left"), value(&i, "doubled")), (8., 16.));
}
#[test]
fn activation_scope_ignores_external_value_change() {
    let mut i = load();
    let x = cell(&i, "x");
    let o = i.advance_reactive_turn(&[x]).unwrap();
    assert!(
        nodes(&i)
            .iter()
            .all(|n| !o.before_commit.executed_nodes.contains(n))
    );
}
#[test]
fn activation_scope_samples_latest_external_value() {
    let mut i = load();
    let x = i
        .symbols()
        .borrow()
        .get(hash_str("x"))
        .unwrap()
        .borrow()
        .clone();
    *x.as_f64().unwrap().borrow_mut() = 20.;
    let t = cell(&i, "tick");
    i.advance_reactive_turn(&[t]).unwrap();
    assert_eq!(
        *i.symbols()
            .borrow()
            .get(hash_str("left"))
            .unwrap()
            .borrow()
            .as_f64()
            .unwrap()
            .borrow(),
        18.
    );
}
#[test]
fn activation_scope_registers_commit_atomically() {
    let mut i = interpret(TWO_REGISTERS);
    let registers = two_register_nodes(&i);
    assert_eq!((value(&i, "x"), value(&i, "y")), (0., 0.));
    let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
    assert_eq!(o.before_commit.pending_register_nodes, registers);
    assert_eq!(o.register_commit.staged_nodes, registers);
    assert_eq!(o.register_commit.committed_nodes, registers);
    assert_eq!(
        o.register_commit.dirty_cells,
        vec![cell(&i, "x"), cell(&i, "y")]
    );
    assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
}
#[test]
fn activation_scope_register_commit_does_not_reactivate_body() {
    let mut i = interpret(TWO_REGISTERS);
    let combinational = activation_nodes(&i, "tick", ReactiveNodeKind::Combinational);
    let registers = two_register_nodes(&i);
    assert!(!combinational.is_empty());
    let o = i.advance_reactive_turn(&[cell(&i, "tick")]).unwrap();
    assert!(combinational.iter().all(|id| {
        o.before_commit
            .executed_nodes
            .iter()
            .filter(|node| **node == *id)
            .count()
            == 1
    }));
    assert_eq!(
        o.before_commit
            .pending_register_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(
        o.before_commit.pending_register_nodes.len(),
        registers.len()
    );
    assert_eq!(
        o.register_commit
            .staged_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(o.register_commit.staged_nodes.len(), registers.len());
    assert_eq!(
        o.register_commit
            .committed_nodes
            .iter()
            .copied()
            .collect::<HashSet<_>>(),
        registers.iter().copied().collect()
    );
    assert_eq!(o.register_commit.committed_nodes.len(), registers.len());
    assert_eq!((value(&i, "x"), value(&i, "y")), (1., 2.));
    assert!(
        combinational
            .iter()
            .all(|id| !o.after_commit.executed_nodes.contains(id))
    );
}
#[test]
fn activation_scope_failed_elaboration_clears_registration_state() {
    let mut i = interpret("tick := 0.0\nx := 1.0");
    let before = snapshot(&i);
    let failing=mech_syntax::parser::parse("~> tick {\n  registered-first := x + 1.0\n  fails-later := function-that-does-not-exist(registered-first)\n}").unwrap();
    let error = i.interpret(&failing).unwrap_err();
    assert!(format!("{error:?}").contains("Function"));
    assert_eq!(snapshot(&i), before);
    assert!(!i.symbols().borrow().contains(hash_str("registered-first")));
    assert!(!i.symbols().borrow().contains(hash_str("fails-later")));
    assert!(!i.plan().activation_registration_active());
    let ordinary = mech_syntax::parser::parse("ordinary := x + 2.0").unwrap();
    i.interpret(&ordinary).unwrap();
    let ordinary_nodes = nodes_for_output(&i, "ordinary", ReactiveNodeKind::Combinational);
    assert!(!ordinary_nodes.is_empty());
    let p = i.plan();
    let p = p.borrow();
    assert!(ordinary_nodes.iter().all(|node| {
        !p.node(*node)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "tick"))
    }));
    assert!(ordinary_nodes.iter().any(|node| {
        p.node(*node)
            .unwrap()
            .inputs
            .iter()
            .any(|d| d.cell == cell(&i, "x") && d.kind == ReactiveDependencyKind::Reactive)
    }));
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_rejects_whole_assignment_to_trigger() {
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
    i.interpret(&setup).unwrap();
    let before = snapshot(&i);
    let t = mech_syntax::parser::parse("~> tick {\n tick = tick + 1.0\n}").unwrap();
    assert!(
        format!("{:?}", i.interpret(&t).unwrap_err())
            .contains("ActivationScopeTriggerWriteUnsupported")
    );
    assert_eq!(snapshot(&i), before);
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_rejects_operator_assignment_to_trigger() {
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let setup = mech_syntax::parser::parse("~tick := 0.0").unwrap();
    i.interpret(&setup).unwrap();
    let before = snapshot(&i);
    let t = mech_syntax::parser::parse("~> tick {\n tick += 1.0\n}").unwrap();
    assert!(
        format!("{:?}", i.interpret(&t).unwrap_err())
            .contains("ActivationScopeTriggerWriteUnsupported")
    );
    assert_eq!(snapshot(&i), before);
    assert!(!i.plan().activation_registration_active());
}
#[test]
fn activation_scope_plan_is_stable_across_triggers() {
    let mut i = load();
    let before = snapshot(&i);
    let t = cell(&i, "tick");
    for _ in 0..3 {
        i.advance_reactive_turn(&[t]).unwrap();
    }
    assert_eq!(snapshot(&i), before);
}
