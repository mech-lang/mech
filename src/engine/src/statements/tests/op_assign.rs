use super::support::{
    cell, distinct_assignment_graph_shape, expected_distinct_assignment_shape, register,
    register_node_id_for_output, root_cell, set_value, symbol, value,
};
use crate::{Interpreter, ReactiveDependencyKind, ReactiveNodeKind, ReactiveTurnState, ValueData};

fn scalar(cell: &crate::ValueCell) -> f64 {
    let snapshot = cell.snapshot().unwrap();
    let ValueData::F64(value) = snapshot.data() else {
        panic!("expected f64 output, got {snapshot:?}")
    };
    value.to_f64()
}

#[cfg(feature = "math_add_assign")]
#[test]
fn whole_add_assignment_registers_state_node() {
    let source = "~x := 1.0; y := 2.0; x += y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(scalar(&output), 3.0);
    assert_eq!(
        distinct_assignment_graph_shape(&interpreter, "x", "y"),
        expected_distinct_assignment_shape()
    );
}

#[cfg(feature = "math_add_assign")]
#[test]
fn whole_add_assignment_alias_is_sampled_once() {
    let source = "~x := 2.0; x += x; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(scalar(&output), 4.0);
    let x_cell = root_cell(&symbol(&interpreter, "x"));
    let node_id = register_node_id_for_output(&interpreter, x_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![x_cell]);
    assert_eq!(node.inputs.len(), 1);
    assert_eq!(node.inputs[0].cell, x_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert!(plan.sampled_consumers_for(x_cell).contains(&node_id));
    assert!(!plan.reactive_consumers_for(x_cell).contains(&node_id));
}

#[cfg(all(feature = "math_add", feature = "math_add_assign"))]
#[test]
fn register_commit_add_assignment_updates_register_only() {
    let t = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    i.interpret(&t).unwrap();
    assert_eq!((value(&i, "x"), value(&i, "z")), (3., 4.));
    let (x, y) = (cell(&i, "x"), cell(&i, "y"));
    set_value(&i, "y", 10.);
    let s = i.plan().solve_dirty_cells(&[y]).unwrap();
    let c = i
        .plan()
        .commit_pending_registers(&s.pending_register_nodes)
        .unwrap();
    assert_eq!(c.dirty_cells, vec![x]);
    assert_eq!((value(&i, "x"), value(&i, "z")), (13., 4.));
}

#[cfg(all(feature = "math_add", feature = "math_add_assign"))]
#[test]
fn register_commit_simultaneous_assignments_use_precommit_state() {
    let t = mech_syntax::parser::parse("~x := 1.0\n~y := 2.0\nx += y\ny += x").unwrap();
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    i.interpret(&t).unwrap();
    assert_eq!((value(&i, "x"), value(&i, "y")), (3., 5.));
    let (x, y) = (cell(&i, "x"), cell(&i, "y"));
    let (rx, ry) = (register(&i, x), register(&i, y));
    let s = i.plan().solve_dirty_cells(&[x, y]).unwrap();
    assert_eq!(s.pending_register_nodes, vec![rx, ry]);
    let c = i.plan().commit_pending_registers(&[ry, rx]).unwrap();
    assert_eq!(c.staged_nodes, vec![rx, ry]);
    assert_eq!(c.committed_nodes, vec![rx, ry]);
    assert_eq!(c.dirty_cells, vec![x, y]);
    assert_eq!((value(&i, "x"), value(&i, "y")), (8., 8.));
}

#[cfg(all(feature = "math_add", feature = "math_add_assign"))]
#[test]
fn reactive_turn_updates_downstream_after_register_commit() {
    let tree = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    interpreter.interpret(&tree).unwrap();
    let (x_cell, y_cell, z_cell) = (
        cell(&interpreter, "x"),
        cell(&interpreter, "y"),
        cell(&interpreter, "z"),
    );
    let x_register = register(&interpreter, x_cell);
    let x_consumers = {
        let plan_handle = interpreter.plan();
        let plan = plan_handle.borrow();
        let consumers = plan.reactive_consumers_for(x_cell).to_vec();
        assert!(!consumers.is_empty());
        for node_id in &consumers {
            assert_eq!(
                plan.node(*node_id).unwrap().kind,
                ReactiveNodeKind::Combinational
            );
        }
        consumers
    };
    set_value(&interpreter, "y", 10.0);
    let mut turn_state = ReactiveTurnState::default();
    let outcome = interpreter
        .plan()
        .advance_reactive_turn(&mut turn_state, &[y_cell])
        .unwrap();
    assert_eq!(
        (value(&interpreter, "x"), value(&interpreter, "z")),
        (13.0, 14.0)
    );
    assert_eq!(
        outcome.before_commit.pending_register_nodes,
        vec![x_register]
    );
    assert_eq!(outcome.register_commit.staged_nodes, vec![x_register]);
    assert_eq!(outcome.register_commit.committed_nodes, vec![x_register]);
    assert_eq!(outcome.register_commit.dirty_cells, vec![x_cell]);
    for node_id in &x_consumers {
        assert!(outcome.after_commit.executed_nodes.contains(node_id));
    }
    let executed_z_nodes = {
        let plan_handle = interpreter.plan();
        let plan = plan_handle.borrow();
        outcome
            .after_commit
            .executed_nodes
            .iter()
            .copied()
            .filter(|node_id| plan.node(*node_id).unwrap().outputs.contains(&z_cell))
            .collect::<Vec<_>>()
    };
    assert!(!executed_z_nodes.is_empty());
    assert!(turn_state.pending_register_nodes.is_empty());
}
