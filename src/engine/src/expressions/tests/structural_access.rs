use crate::{Interpreter, Plan, ValueCell, ValueData, hash_str};

fn symbol(interpreter: &Interpreter, name: &str) -> ValueCell {
    interpreter.symbols().borrow().get(hash_str(name)).unwrap()
}

fn f64_value(cell: &ValueCell) -> f64 {
    let snapshot = cell.snapshot().unwrap();
    match snapshot.data() {
        ValueData::F64(value) => value.to_f64(),
        other => panic!("expected f64, got {other:?}"),
    }
}

fn alias_node(plan: &Plan, name: &str) -> usize {
    let plan = plan.borrow();
    (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            node.function.to_string().contains(name).then_some(node_id)
        })
        .unwrap_or_else(|| panic!("missing {name} node"))
}

fn assert_access_node(plan: &Plan, name: &str, output: &ValueCell, container: &ValueCell) {
    let output_cell = output.reactive_cell_id();
    let container_cell = container.reactive_cell_id();
    let node_id = alias_node(plan, name);
    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(node_id).unwrap();
    assert!(node.inputs.iter().any(|input| input.cell == container_cell));
    assert_eq!(node.outputs.as_slice(), &[output_cell]);
    assert!(
        plan_borrow
            .reactive_consumers_for(container_cell)
            .contains(&node_id)
    );
}

#[test]
fn record_field_access_registers_structural_node() {
    let tree = mech_syntax::parser::parse("record := {field: 2}; record.field").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(f64_value(&output), 2.0);
    assert_access_node(
        &interpreter.plan(),
        "RecordAccessField",
        &output,
        &symbol(&interpreter, "record"),
    );
}

#[test]
fn tuple_element_access_registers_structural_node() {
    let tree = mech_syntax::parser::parse("tuple := (1, 2); tuple.2").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(f64_value(&output), 2.0);
    assert_access_node(
        &interpreter.plan(),
        "TupleAccessElement",
        &output,
        &symbol(&interpreter, "tuple"),
    );
}

#[test]
fn record_field_consumer_depends_on_member_cell() {
    let tree = mech_syntax::parser::parse("record := {field: 2}; record.field + 1").unwrap();
    let mut interpreter = Interpreter::with_function_catalog(
        0,
        10_000,
        crate::test_support::catalog::function_catalog(),
    );
    let output = interpreter.interpret(&tree).unwrap().unwrap();
    assert_eq!(f64_value(&output), 3.0);
    let record = symbol(&interpreter, "record");
    let record_cell = record.reactive_cell_id();
    let plan = interpreter.plan();
    let alias_id = alias_node(&plan, "RecordAccessField");
    assert!(
        plan.borrow()
            .node(alias_id)
            .unwrap()
            .inputs
            .iter()
            .any(|input| input.cell == record_cell)
    );
    let field_cell = plan.borrow().node(alias_id).unwrap().outputs[0];
    let output_cell = output.reactive_cell_id();
    let plan = plan.borrow();
    let (consumer_id, consumer) = (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            (node_id != alias_id && node.outputs.contains(&output_cell)).then_some((node_id, node))
        })
        .expect("missing computed field consumer");
    assert!(consumer.inputs.iter().any(|input| input.cell == field_cell));
    assert!(
        plan.reactive_consumers_for(field_cell)
            .contains(&consumer_id)
    );
}
