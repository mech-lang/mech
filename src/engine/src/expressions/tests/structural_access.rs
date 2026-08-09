use crate::{Interpreter, LegacyValue, Plan, hash_str};

fn symbol(interpreter: &Interpreter, name: &str) -> LegacyValue {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap()
        .borrow()
        .clone()
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

fn assert_alias_node(plan: &Plan, name: &str, output: &LegacyValue, container: &LegacyValue) {
    let output_cell = output.reactive_root_cell_ids()[0];
    let container_cell = container.reactive_root_cell_ids()[0];
    let node_id = alias_node(plan, name);
    let plan_borrow = plan.borrow();
    let node = plan_borrow.node(node_id).unwrap();
    assert!(node.inputs.is_empty());
    assert_eq!(node.outputs.as_slice(), &[output_cell]);
    assert!(!node.inputs.iter().any(|input| input.cell == container_cell));
    assert!(
        !plan_borrow
            .reactive_consumers_for(container_cell)
            .contains(&node_id)
    );
    assert!(
        !plan_borrow
            .sampled_consumers_for(container_cell)
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
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_alias_node(
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
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 2.0);
    assert_alias_node(
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
    let output = interpreter.interpret(&tree).unwrap();
    assert_eq!(*output.as_f64().unwrap().borrow(), 3.0);
    let record = symbol(&interpreter, "record");
    let record_cell = record.reactive_root_cell_ids()[0];
    let field_cell = {
        let LegacyValue::Record(record) = record else {
            panic!("expected record")
        };
        record
            .borrow()
            .get(&hash_str("field"))
            .unwrap()
            .reactive_root_cell_ids()[0]
    };
    let plan = interpreter.plan();
    let alias_id = alias_node(&plan, "RecordAccessField");
    assert!(plan.borrow().node(alias_id).unwrap().inputs.is_empty());
    let output_cell = output.reactive_root_cell_ids()[0];
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
    assert!(
        !consumer
            .inputs
            .iter()
            .any(|input| input.cell == record_cell)
    );
}
