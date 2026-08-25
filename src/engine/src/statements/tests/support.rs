use crate::{
    Interpreter, LegacyValue, ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId,
    ReactiveNodeKind, hash_str,
};

pub(super) fn symbol(interpreter: &Interpreter, name: &str) -> LegacyValue {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

pub(super) fn root_cell(value: &LegacyValue) -> ReactiveCellId {
    let cells = value.reactive_root_cell_ids();
    assert_eq!(cells.len(), 1);
    cells[0]
}

pub(super) fn register_node_id_for_output(
    interpreter: &Interpreter,
    output_cell: ReactiveCellId,
) -> ReactiveNodeId {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node_ids = plan
        .nodes
        .iter()
        .filter(|node| node.kind == ReactiveNodeKind::Register && node.outputs == vec![output_cell])
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(node_ids.len(), 1);
    node_ids[0]
}

#[derive(Debug, PartialEq, Eq)]
pub(super) struct RegisterGraphShape {
    output_count: usize,
    input_kinds: Vec<ReactiveDependencyKind>,
    output_is_first_input: bool,
    source_is_second_input: bool,
    output_is_sampled_consumer: bool,
    output_is_reactive_consumer: bool,
    source_is_reactive_consumer: bool,
    source_is_sampled_consumer: bool,
}

pub(super) fn distinct_assignment_graph_shape(
    interpreter: &Interpreter,
    target_name: &str,
    source_name: &str,
) -> RegisterGraphShape {
    let target_cell = root_cell(&symbol(interpreter, target_name));
    let source_cell = root_cell(&symbol(interpreter, source_name));
    assert_ne!(target_cell, source_cell);
    let node_id = register_node_id_for_output(interpreter, target_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![target_cell]);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, target_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_eq!(node.inputs[1].cell, source_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    RegisterGraphShape {
        output_count: node.outputs.len(),
        input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
        output_is_first_input: node.inputs[0].cell == target_cell,
        source_is_second_input: node.inputs[1].cell == source_cell,
        output_is_sampled_consumer: plan.sampled_consumers_for(target_cell).contains(&node_id),
        output_is_reactive_consumer: plan.reactive_consumers_for(target_cell).contains(&node_id),
        source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
        source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
}

pub(super) fn expected_distinct_assignment_shape() -> RegisterGraphShape {
    RegisterGraphShape {
        output_count: 1,
        input_kinds: vec![
            ReactiveDependencyKind::Sampled,
            ReactiveDependencyKind::Reactive,
        ],
        output_is_first_input: true,
        source_is_second_input: true,
        output_is_sampled_consumer: true,
        output_is_reactive_consumer: false,
        source_is_reactive_consumer: true,
        source_is_sampled_consumer: false,
    }
}

pub(super) fn cell(i: &Interpreter, n: &str) -> ReactiveCellId {
    let c = symbol(i, n).reactive_root_cell_ids();
    assert_eq!(c.len(), 1);
    c[0]
}
pub(super) fn value(i: &Interpreter, n: &str) -> f64 {
    *symbol(i, n).as_f64().unwrap().borrow()
}
pub(super) fn set_value(i: &Interpreter, n: &str, v: f64) {
    *symbol(i, n).as_f64().unwrap().borrow_mut() = v;
}
pub(super) fn register(i: &Interpreter, c: ReactiveCellId) -> ReactiveNodeId {
    let p = i.plan();
    let p = p.borrow();
    let v = p
        .nodes
        .iter()
        .filter(|n| n.kind == ReactiveNodeKind::Register && n.outputs.contains(&c))
        .map(|n| n.id)
        .collect::<Vec<_>>();
    assert_eq!(v.len(), 1);
    v[0]
}
