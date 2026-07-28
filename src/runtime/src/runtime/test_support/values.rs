use mech_core::{
    ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, Value, hash_str,
};

use super::super::MechRuntime;
use crate::{HostArgumentValue, RuntimeHostInputSource};

pub(crate) fn f64_value(value: &Value) -> f64 {
    match value {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected f64, got {other:?}"),
    }
}

pub(crate) fn bool_value(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value.borrow(),
        other => panic!("expected bool, got {other:?}"),
    }
}

pub(crate) fn string_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.borrow().clone(),
        other => panic!("expected string, got {other:?}"),
    }
}

pub(crate) fn host_f64_argument(value: &impl HostArgumentValue) -> f64 {
    match value.host_argument_value() {
        Value::F64(value) => *value.borrow(),
        Value::MutableReference(value) => match &*value.borrow() {
            Value::F64(value) => *value.borrow(),
            other => panic!("expected f64 mutable reference, got {other:?}",),
        },
        other => panic!("expected f64 host argument, got {other:?}"),
    }
}

pub(crate) fn symbol_value(runtime: &MechRuntime, name: &str) -> Value {
    runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

pub(crate) fn source_value(runtime: &MechRuntime, source: &RuntimeHostInputSource) -> Value {
    let input = runtime
        .live_input_bindings
        .get(source)
        .and_then(|inputs| inputs.first())
        .unwrap_or_else(|| {
            panic!(
                "missing binding for {} / {}",
                source.base_uri(),
                source.path(),
            )
        });
    runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(input.symbol_id)
        .unwrap_or_else(|| panic!("missing symbol {}", input.symbol_id))
        .borrow()
        .clone()
}

pub(crate) fn symbol_cell(runtime: &MechRuntime, name: &str) -> ReactiveCellId {
    let cells = symbol_value(runtime, name).reactive_root_cell_ids();
    assert_eq!(cells.len(), 1, "symbol {name} must have one root cell");
    cells[0]
}

pub(crate) fn source_cell(
    runtime: &MechRuntime,
    source: &RuntimeHostInputSource,
) -> ReactiveCellId {
    let cells = source_value(runtime, source).reactive_root_cell_ids();
    assert_eq!(cells.len(), 1, "source must have one root cell");
    cells[0]
}

pub(crate) fn register_node_for_symbol(runtime: &MechRuntime, name: &str) -> ReactiveNodeId {
    let output = symbol_cell(runtime, name);
    let plan = runtime.program.interpreter().plan();
    let plan = plan.borrow();
    let nodes = plan
        .nodes
        .iter()
        .filter(|node| node.kind == ReactiveNodeKind::Register && node.outputs.contains(&output))
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 1, "symbol {name} must have one register node");
    nodes[0]
}

pub(crate) fn combinational_node_for_output_and_inputs(
    runtime: &MechRuntime,
    output: ReactiveCellId,
    required_inputs: &[ReactiveCellId],
) -> ReactiveNodeId {
    let plan = runtime.program.interpreter().plan();
    let plan = plan.borrow();
    let nodes = plan
        .nodes
        .iter()
        .filter(|node| {
            node.kind == ReactiveNodeKind::Combinational
                && node.outputs.contains(&output)
                && required_inputs.iter().all(|required| {
                    node.inputs.iter().any(|dependency| {
                        dependency.cell == *required
                            && dependency.kind == ReactiveDependencyKind::Reactive
                    })
                })
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(nodes.len(), 1, "expected one matching combinational node");
    nodes[0]
}

pub(crate) fn plan_snapshot(
    runtime: &MechRuntime,
) -> (usize, Vec<ReactiveNodeId>, Vec<Vec<ReactiveCellId>>) {
    let plan = runtime.program.interpreter().plan();
    let plan = plan.borrow();
    (
        plan.len(),
        plan.nodes.iter().map(|node| node.id).collect(),
        plan.nodes.iter().map(|node| node.outputs.clone()).collect(),
    )
}
