use super::support::{
    cell, distinct_assignment_graph_shape, expected_distinct_assignment_shape, register,
    register_node_id_for_output, set_value, symbol, value,
};
use crate::{
    Interpreter, LegacyValue, ReactiveDependencyKind, ReactiveNodeKind, ReactiveTurnState,
};

#[test]
fn whole_variable_assignment_registers_state_node() {
    let source = "~x := 1.0; y := 2.0; x = y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_eq!(
        distinct_assignment_graph_shape(&interpreter, "x", "y"),
        expected_distinct_assignment_shape()
    );
}

#[cfg(all(feature = "matrix", feature = "row_vectord"))]
#[test]
fn whole_matrix_assignment_uses_root_cells() {
    let source = "~x := [1.0 2.0]; y := [3.0 4.0]; x = y; x";
    let tree = mech_syntax::parser::parse(source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap();
    let x = symbol(&interpreter, "x");
    let y = symbol(&interpreter, "y");
    let x_root_cells = x.reactive_root_cell_ids();
    let y_root_cells = y.reactive_root_cell_ids();
    assert_eq!(x_root_cells.len(), 1);
    assert_eq!(y_root_cells.len(), 1);
    let x_cell = x_root_cells[0];
    let y_cell = y_root_cells[0];
    let node_id = register_node_id_for_output(&interpreter, x_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![x_cell]);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, x_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_eq!(node.inputs[1].cell, y_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    assert!(plan.sampled_consumers_for(x_cell).contains(&node_id));
    assert!(!plan.reactive_consumers_for(x_cell).contains(&node_id));
    assert!(plan.reactive_consumers_for(y_cell).contains(&node_id));
    assert!(!plan.sampled_consumers_for(y_cell).contains(&node_id));
    let resolved_output = match output {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
        other => other,
    };
    assert_eq!(resolved_output, y);
}

#[cfg(all(
    feature = "assign",
    feature = "bool",
    feature = "f64",
    feature = "logical_indexing",
    feature = "matrix",
    feature = "matrixd",
    feature = "range_inclusive",
    feature = "subscript_formula",
    feature = "subscript_range",
    feature = "subscript_slice",
    feature = "variable_assign"
))]
fn matrix_after_indexed_assignment(selector: &str, value: &str) -> Vec<f64> {
    let source = format!("~x := [1.0 2.0 3.0; 4.0 5.0 6.0; 7.0 8.0 9.0]; x{selector} = {value}; x");
    let tree = mech_syntax::parser::parse(&source).unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter
        .interpret(&tree)
        .unwrap_or_else(|error| panic!("{selector}: {error:?}"));
    let output = match output {
        LegacyValue::MutableReference(value) => value.borrow().clone(),
        value => value,
    };
    let LegacyValue::MatrixF64(matrix) = output else {
        panic!("expected an f64 matrix assignment result");
    };
    matrix.as_vec()
}

#[cfg(all(
    feature = "assign",
    feature = "bool",
    feature = "f64",
    feature = "logical_indexing",
    feature = "matrix",
    feature = "matrixd",
    feature = "range_inclusive",
    feature = "subscript_formula",
    feature = "subscript_range",
    feature = "subscript_slice",
    feature = "variable_assign"
))]
#[test]
fn explicit_all_selector_preserves_plain_matrix_assignment_layouts() {
    for (selector, value, expected) in [
        ("[:]", "0.0", vec![0.0; 9]),
        (
            "[:,2]",
            "0.0",
            vec![1.0, 4.0, 7.0, 0.0, 0.0, 0.0, 3.0, 6.0, 9.0],
        ),
        (
            "[2,:]",
            "0.0",
            vec![1.0, 0.0, 7.0, 2.0, 0.0, 8.0, 3.0, 0.0, 9.0],
        ),
        (
            "[:,1..=2]",
            "0.0",
            vec![0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 6.0, 9.0],
        ),
        (
            "[1..=2,:]",
            "0.0",
            vec![0.0, 0.0, 7.0, 0.0, 0.0, 8.0, 0.0, 0.0, 9.0],
        ),
        (
            "[:,[1 3]]",
            "0.0",
            vec![0.0, 0.0, 0.0, 2.0, 5.0, 8.0, 0.0, 0.0, 0.0],
        ),
        (
            "[[1 3],:]",
            "0.0",
            vec![0.0, 4.0, 0.0, 0.0, 5.0, 0.0, 0.0, 6.0, 0.0],
        ),
        (
            "[:,[true false true]]",
            "0.0",
            vec![0.0, 0.0, 0.0, 2.0, 5.0, 8.0, 0.0, 0.0, 0.0],
        ),
        (
            "[[true false true],:]",
            "[0.0 0.0 0.0; 0.0 0.0 0.0; 0.0 0.0 0.0]",
            vec![0.0, 4.0, 0.0, 0.0, 5.0, 0.0, 0.0, 6.0, 0.0],
        ),
    ] {
        assert_eq!(
            matrix_after_indexed_assignment(selector, value),
            expected,
            "{selector}"
        );
    }
}

#[cfg(all(
    feature = "assign",
    feature = "f64",
    feature = "matrix",
    feature = "matrixd",
    feature = "subscript_range",
    feature = "subscript_slice",
    feature = "variable_assign"
))]
#[test]
fn all_all_matrix_assignment_remains_rejected() {
    let result = std::panic::catch_unwind(|| {
        let tree = mech_syntax::parser::parse("~x := [1.0 2.0; 3.0 4.0]; x[:,:] = 0.0; x").unwrap();
        let mut interpreter = Interpreter::with_function_catalog(
            0,
            10_000,
            crate::test_support::catalog::function_catalog(),
        );
        interpreter.interpret(&tree)
    });
    assert!(match result {
        Err(_) => true,
        Ok(result) => result.is_err(),
    });
}

#[cfg(all(feature = "math_add", feature = "math_add_assign"))]
#[test]
fn register_commit_plain_assignment_updates_register_only() {
    let t = mech_syntax::parser::parse("~x := 1.0\ny := 2.0\nx = y\nz := x + 1.0").unwrap();
    let mut i = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    i.interpret(&t).unwrap();
    assert_eq!((value(&i, "x"), value(&i, "z")), (2., 3.));
    let (x, y) = (cell(&i, "x"), cell(&i, "y"));
    let r = register(&i, x);
    set_value(&i, "y", 10.);
    let s = i.plan().solve_dirty_cells(&[y]).unwrap();
    assert_eq!(s.pending_register_nodes, vec![r]);
    let c = i
        .plan()
        .commit_pending_registers(&s.pending_register_nodes)
        .unwrap();
    assert_eq!(c.staged_nodes, vec![r]);
    assert_eq!(c.committed_nodes, vec![r]);
    assert_eq!(c.dirty_cells, vec![x]);
    assert_eq!((value(&i, "x"), value(&i, "z")), (10., 3.));
}

#[cfg(all(feature = "math_add", feature = "math_add_assign"))]
#[test]
fn reactive_turn_defers_second_register_layer() {
    let tree = mech_syntax::parser::parse("input := 1.0\n~a := 0.0\n~b := 0.0\na = input\nmiddle := a + 1.0\nb = middle\noutput := b + 1.0").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    interpreter.interpret(&tree).unwrap();
    assert_eq!(
        (
            value(&interpreter, "input"),
            value(&interpreter, "a"),
            value(&interpreter, "middle"),
            value(&interpreter, "b"),
            value(&interpreter, "output")
        ),
        (1., 1., 2., 2., 3.)
    );
    let (input, a, b) = (
        cell(&interpreter, "input"),
        cell(&interpreter, "a"),
        cell(&interpreter, "b"),
    );
    let (a_register, b_register) = (register(&interpreter, a), register(&interpreter, b));
    set_value(&interpreter, "input", 10.0);
    let mut turn_state = ReactiveTurnState::default();
    let first = interpreter
        .plan()
        .advance_reactive_turn(&mut turn_state, &[input])
        .unwrap();
    assert_eq!(
        (
            value(&interpreter, "a"),
            value(&interpreter, "middle"),
            value(&interpreter, "b"),
            value(&interpreter, "output")
        ),
        (10., 11., 2., 3.)
    );
    assert_eq!(first.register_commit.committed_nodes, vec![a_register]);
    assert_eq!(first.after_commit.pending_register_nodes, vec![b_register]);
    assert_eq!(turn_state.pending_register_nodes, vec![b_register]);
    let second = interpreter
        .plan()
        .advance_reactive_turn(&mut turn_state, &[])
        .unwrap();
    assert_eq!(
        (
            value(&interpreter, "a"),
            value(&interpreter, "middle"),
            value(&interpreter, "b"),
            value(&interpreter, "output")
        ),
        (10., 11., 11., 12.)
    );
    assert_eq!(second.register_commit.committed_nodes, vec![b_register]);
    assert!(!second.register_commit.committed_nodes.contains(&a_register));
    assert!(turn_state.pending_register_nodes.is_empty());
}
