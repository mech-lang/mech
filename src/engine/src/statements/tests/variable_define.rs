use crate::Interpreter;

#[test]
fn var_define_registration_has_no_reactive_inputs() {
    let tree = mech_syntax::parser::parse("defined-value := 1.0; defined-value").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    let output_cell = output.reactive_cell_id();
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let definition = (0..plan.len())
        .filter_map(|index| plan.node(index))
        .find(|node| node.outputs.contains(&output_cell))
        .expect("canonical variable definition registers its output");
    assert!(definition.inputs.is_empty());
}
