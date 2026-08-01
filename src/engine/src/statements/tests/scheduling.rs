use crate::{Interpreter, ReactiveCellId, ReactiveNodeId, ReactiveNodeKind, Value, hash_str};

fn symbol(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}
fn root_cell(interpreter: &Interpreter, name: &str) -> ReactiveCellId {
    let cells = symbol(interpreter, name).reactive_root_cell_ids();
    assert_eq!(cells.len(), 1);
    cells[0]
}
fn set_f64(interpreter: &Interpreter, name: &str, value: f64) {
    *symbol(interpreter, name).as_f64().unwrap().borrow_mut() = value;
}
fn output_nodes(interpreter: &Interpreter, cell: ReactiveCellId) -> Vec<ReactiveNodeId> {
    let plan = interpreter.plan();
    plan.borrow()
        .nodes
        .iter()
        .filter(|node| node.outputs.contains(&cell))
        .map(|node| node.id)
        .collect()
}
fn executed_outputs(
    interpreter: &Interpreter,
    executed: &[ReactiveNodeId],
    cell: ReactiveCellId,
) -> bool {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    executed
        .iter()
        .any(|id| plan.node(*id).unwrap().outputs.contains(&cell))
}
fn value(interpreter: &Interpreter, name: &str) -> f64 {
    *symbol(interpreter, name).as_f64().unwrap().borrow()
}

#[test]
fn dirty_scheduler_updates_only_reachable_combinational_nodes() {
    let tree = mech_syntax::parser::parse(
        "a := 1.0\nb := a + 1.0\nc := b + 1.0\n\nu := 100.0\nv := u + 1.0",
    )
    .unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    interpreter.interpret(&tree).unwrap();
    assert_eq!(
        (
            value(&interpreter, "b"),
            value(&interpreter, "c"),
            value(&interpreter, "v")
        ),
        (2.0, 3.0, 101.0)
    );
    let a = root_cell(&interpreter, "a");
    let b = root_cell(&interpreter, "b");
    let c = root_cell(&interpreter, "c");
    let v = root_cell(&interpreter, "v");
    set_f64(&interpreter, "a", 10.0);
    let outcome = interpreter.plan().solve_dirty_cells(&[a]).unwrap();
    assert_eq!(
        (
            value(&interpreter, "b"),
            value(&interpreter, "c"),
            value(&interpreter, "v")
        ),
        (11.0, 12.0, 101.0)
    );
    assert!(executed_outputs(&interpreter, &outcome.executed_nodes, b));
    assert!(executed_outputs(&interpreter, &outcome.executed_nodes, c));
    assert!(!executed_outputs(&interpreter, &outcome.executed_nodes, v));
    assert!(outcome.pending_register_nodes.is_empty());
    let plan = interpreter.plan();
    let plan = plan.borrow();
    assert!(outcome.executed_nodes.windows(2).all(
        |pair| plan.node(pair[0]).unwrap().plan_index < plan.node(pair[1]).unwrap().plan_index
    ));
    let unique = outcome
        .executed_nodes
        .iter()
        .collect::<std::collections::HashSet<_>>();
    assert_eq!(unique.len(), outcome.executed_nodes.len());
}

#[test]
fn dirty_scheduler_stops_at_register_boundary() {
    let tree = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx += y\nz := x + 1.0").unwrap();
    let mut interpreter = Interpreter::new_with_full_stdlib(0);
    interpreter.interpret(&tree).unwrap();
    assert_eq!(
        (value(&interpreter, "x"), value(&interpreter, "z")),
        (3.0, 4.0)
    );
    let x = root_cell(&interpreter, "x");
    let y = root_cell(&interpreter, "y");
    let z = root_cell(&interpreter, "z");
    let registers = {
        let plan = interpreter.plan();
        let plan = plan.borrow();
        plan.nodes
            .iter()
            .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&x))
            .map(|n| n.id)
            .collect::<Vec<_>>()
    };
    assert_eq!(registers.len(), 1);
    let register = registers[0];
    assert!(output_nodes(&interpreter, x).contains(&register));
    set_f64(&interpreter, "y", 10.0);
    let outcome = interpreter.plan().solve_dirty_cells(&[y]).unwrap();
    assert_eq!(outcome.pending_register_nodes, vec![register]);
    assert_eq!(
        (value(&interpreter, "x"), value(&interpreter, "z")),
        (3.0, 4.0)
    );
    assert!(!outcome.executed_nodes.contains(&register));
    assert!(!executed_outputs(&interpreter, &outcome.executed_nodes, z));
}
