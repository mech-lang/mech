use crate::{Interpreter, ReactiveDependencyKind};

#[test]
fn variable_kind_cast_is_indexed() {
    let tree = mech_syntax::parser::parse("value := 1; value<f64>").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 1.0);
    let output_cell = output.reactive_root_cell_ids()[0];
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let (node_id, node) = (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            (node.outputs.contains(&output_cell) && !node.inputs.is_empty())
                .then_some((node_id, node))
        })
        .expect("converted variable read should be registered in the plan");
    assert!(
        node.inputs
            .iter()
            .all(|dependency| dependency.kind == ReactiveDependencyKind::Reactive)
    );
    assert!(node.outputs.contains(&output_cell));
    assert!(
        !node
            .inputs
            .iter()
            .any(|dependency| dependency.cell == output_cell)
    );
    for dependency in &node.inputs {
        assert!(
            plan.reactive_consumers_for(dependency.cell)
                .contains(&node_id)
        );
        assert!(plan.sampled_consumers_for(dependency.cell).is_empty());
    }
}
