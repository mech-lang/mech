use mech_core::structures::Matrix as ValueMatrix;
use mech_core::{
    FunctionCatalogBuilder, MResult, ParsedProgram, Plan, ReactiveCellId, ReactiveDependencyKind,
    ReactiveNodeId, ReactiveNodeKind, ReactiveTurnState, Value, hash_str,
};
use mech_engine::Interpreter;
use mech_engine::{MechProgram, MechProgramConfig};
use std::sync::Arc;

fn source_program() -> MechProgram {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        Arc::new(builder.build().unwrap()),
    )
}

fn runtime_program() -> MechProgram {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        Arc::new(builder.build().unwrap()),
    )
}

fn symbol(interpreter: &Interpreter, name: &str) -> Value {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

fn root_cell(value: &Value) -> ReactiveCellId {
    let cells = value.reactive_root_cell_ids();
    assert_eq!(cells.len(), 1);
    cells[0]
}

fn alias_node(plan: &Plan, name: &str) -> ReactiveNodeId {
    let plan = plan.borrow();
    (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            node.function.to_string().contains(name).then_some(node_id)
        })
        .unwrap_or_else(|| panic!("missing {name} node"))
}

fn assert_alias_node(plan: &Plan, name: &str, output: &Value) {
    let node_id = alias_node(plan, name);
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert!(node.inputs.is_empty());
    assert_eq!(node.outputs.as_slice(), &output.reactive_root_cell_ids());
}

fn register_node_id_for_output(
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
struct RegisterGraphShape {
    output_count: usize,
    input_kinds: Vec<ReactiveDependencyKind>,
    output_is_first_input: bool,
    source_is_second_input: bool,
    output_is_sampled_consumer: bool,
    output_is_reactive_consumer: bool,
    source_is_reactive_consumer: bool,
    source_is_sampled_consumer: bool,
}

fn distinct_assignment_graph_shape(
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

fn decoded_assignment_graph_shape(interpreter: &Interpreter, output: &Value) -> RegisterGraphShape {
    let resolved_output = match output {
        Value::MutableReference(reference) => reference.borrow().clone(),
        other => other.clone(),
    };
    let output_cell = root_cell(&resolved_output);
    let node_id = register_node_id_for_output(interpreter, output_cell);
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node = plan.node(node_id).unwrap();
    assert_eq!(node.kind, ReactiveNodeKind::Register);
    assert_eq!(node.outputs, vec![output_cell]);
    assert_eq!(node.outputs.len(), 1);
    assert_eq!(node.inputs.len(), 2);
    assert_eq!(node.inputs[0].cell, output_cell);
    assert_eq!(node.inputs[0].kind, ReactiveDependencyKind::Sampled);
    assert_ne!(node.inputs[1].cell, output_cell);
    assert_eq!(node.inputs[1].kind, ReactiveDependencyKind::Reactive);
    let source_cell = node.inputs[1].cell;
    RegisterGraphShape {
        output_count: node.outputs.len(),
        input_kinds: node.inputs.iter().map(|input| input.kind).collect(),
        output_is_first_input: node.inputs[0].cell == output_cell,
        source_is_second_input: node.inputs[1].cell == source_cell,
        output_is_sampled_consumer: plan.sampled_consumers_for(output_cell).contains(&node_id),
        output_is_reactive_consumer: plan.reactive_consumers_for(output_cell).contains(&node_id),
        source_is_reactive_consumer: plan.reactive_consumers_for(source_cell).contains(&node_id),
        source_is_sampled_consumer: plan.sampled_consumers_for(source_cell).contains(&node_id),
    }
}

fn expected_distinct_assignment_shape() -> RegisterGraphShape {
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

fn register(interpreter: &Interpreter, output_cell: ReactiveCellId) -> ReactiveNodeId {
    let plan = interpreter.plan();
    let plan = plan.borrow();
    let node_ids = plan
        .nodes
        .iter()
        .filter(|node| {
            node.kind == ReactiveNodeKind::Register && node.outputs.contains(&output_cell)
        })
        .map(|node| node.id)
        .collect::<Vec<_>>();
    assert_eq!(node_ids.len(), 1);
    node_ids[0]
}

fn assert_matrix_literal_chain(plan: &Plan) {
    let plan = plan.borrow();
    assert_eq!(plan.len(), 3);

    let first_row = plan.node(0).unwrap();
    let second_row = plan.node(1).unwrap();
    let vertical = plan.node(2).unwrap();
    assert!(!first_row.inputs.is_empty());
    assert!(!second_row.inputs.is_empty());
    assert!(!vertical.inputs.is_empty());
    assert!(
        first_row
            .outputs
            .iter()
            .all(|output| !vertical.outputs.contains(output))
    );
    assert!(
        second_row
            .outputs
            .iter()
            .all(|output| !vertical.outputs.contains(output))
    );
    assert!(
        vertical
            .inputs
            .iter()
            .all(|dependency| { dependency.kind == ReactiveDependencyKind::Reactive })
    );
    assert!(!vertical.outputs.is_empty());
}

fn set_members(value: &Value) -> Vec<ReactiveCellId> {
    match value {
        Value::Set(set) => set
            .borrow()
            .set
            .iter()
            .flat_map(Value::reactive_root_cell_ids)
            .collect(),
        other => panic!("expected set, found {other:?}"),
    }
}

fn assert_structural_set_node(plan: &Plan, output: &Value) {
    let output_cell = output.reactive_root_cell_ids()[0];
    let member_cells = set_members(output);
    let plan = plan.borrow();
    let (node_id, node) = (0..plan.len())
        .find_map(|node_id| {
            let node = plan.node(node_id).unwrap();
            node.outputs
                .contains(&output_cell)
                .then_some((node_id, node))
        })
        .expect("set structural node should be registered");
    assert!(node.inputs.is_empty());
    assert_eq!(node.outputs.as_slice(), &[output_cell]);
    for member_cell in member_cells {
        assert!(!node.outputs.contains(&member_cell));
        assert!(!plan.reactive_consumers_for(member_cell).contains(&node_id));
        assert!(!plan.sampled_consumers_for(member_cell).contains(&node_id));
    }
}

#[test]
fn decoded_variable_definition_symbol_metadata_round_trips() -> MResult<()> {
    let code = "input := 1.0\n~state := 2.0";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(decoded_output, source_output);
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let input_id = hash_str("input");
    let state_id = hash_str("state");
    assert!(parsed.symbols.contains_key(&input_id));
    assert!(parsed.symbols.contains_key(&state_id));
    assert_eq!(parsed.dictionary.get(&input_id).unwrap(), "input");
    assert_eq!(parsed.dictionary.get(&state_id).unwrap(), "state");
    assert!(!parsed.mutable_symbols.contains(&input_id));
    assert!(parsed.mutable_symbols.contains(&state_id));
    for (name, expected) in [("input", 1.0), ("state", 2.0)] {
        let value = symbol(decoded.interpreter(), name);
        assert_eq!(*value.as_f64().unwrap().borrow(), expected);
    }
    let state = decoded.interpreter().state.borrow();
    assert!(state.get_mutable_symbol(input_id).is_none());
    assert!(state.get_mutable_symbol(state_id).is_some());
    Ok(())
}

#[test]
fn decoded_structural_alias_access_matches_source() -> MResult<()> {
    let code = "tuple := (1, 2); tuple.2";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(decoded_output, source_output);
    assert_alias_node(
        &source.interpreter().plan(),
        "TupleAccessElement",
        &source_output,
    );
    assert_alias_node(
        &decoded.interpreter().plan(),
        "TupleAccessElement",
        &decoded_output,
    );
    Ok(())
}

#[test]
fn decoded_whole_variable_assignment_matches_source_graph() -> MResult<()> {
    let code = "~x := 1.0; y := 2.0; x = y; x";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 2.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 2.0);
    let source_shape = distinct_assignment_graph_shape(source.interpreter(), "x", "y");
    let decoded_shape = decoded_assignment_graph_shape(decoded.interpreter(), &decoded_output);
    assert_eq!(source_shape, expected_distinct_assignment_shape());
    assert_eq!(decoded_shape, expected_distinct_assignment_shape());
    assert_eq!(source_shape, decoded_shape);
    Ok(())
}

#[test]
fn decoded_register_commit_assignment_uses_staging() -> MResult<()> {
    let code = "~x := 1.0\ny := 2.0\nx = y\nx";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 2.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 2.0);
    let output_cell = root_cell(&decoded_output);
    let register_node = register(decoded.interpreter(), output_cell);
    let (source_cell, source_ref) = {
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        let node = plan.node(register_node).unwrap();
        let dependencies = node
            .inputs
            .iter()
            .filter(|dependency| {
                dependency.kind == ReactiveDependencyKind::Reactive
                    && dependency.cell != output_cell
            })
            .collect::<Vec<_>>();
        assert_eq!(
            dependencies.len(),
            1,
            "decoded register must have exactly one distinct reactive source",
        );
        (
            dependencies[0].cell,
            symbol(decoded.interpreter(), "y").as_f64()?,
        )
    };
    *source_ref.borrow_mut() = 5.0;
    let scheduling = decoded
        .interpreter()
        .plan()
        .solve_dirty_cells(&[source_cell])?;
    assert_eq!(scheduling.pending_register_nodes, vec![register_node]);
    let commit = decoded
        .interpreter()
        .plan()
        .commit_pending_registers(&scheduling.pending_register_nodes)?;
    assert_eq!(commit.staged_nodes, vec![register_node]);
    assert_eq!(commit.committed_nodes, vec![register_node]);
    assert_eq!(commit.dirty_cells, vec![output_cell]);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 5.0);
    Ok(())
}

#[test]
fn decoded_reactive_turn_reuses_compiled_assignment_plan() -> MResult<()> {
    let code = "~x := 1.0\ny := 2.0\nx = y\nx";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(*source_output.as_f64().unwrap().borrow(), 2.0);
    assert_eq!(*decoded_output.as_f64().unwrap().borrow(), 2.0);
    let (x_register, x_ref, x_cell, source_cell, plan_length, node_ids, output_cells) = {
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        let registers = plan
            .nodes
            .iter()
            .filter(|node| node.kind == ReactiveNodeKind::Register)
            .collect::<Vec<_>>();
        assert_eq!(registers.len(), 1);
        let x_register = registers[0].id;
        let x_output = plan.node(x_register).unwrap().function.out();
        let x_ref = x_output.as_f64().unwrap().clone();
        let x_cell = root_cell(&x_output);
        let source_dependencies = plan
            .node(x_register)
            .unwrap()
            .inputs
            .iter()
            .filter(|dependency| {
                dependency.kind == ReactiveDependencyKind::Reactive && dependency.cell != x_cell
            })
            .collect::<Vec<_>>();
        assert_eq!(source_dependencies.len(), 1);
        (
            x_register,
            x_ref,
            x_cell,
            source_dependencies[0].cell,
            plan.len(),
            plan.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            plan.nodes
                .iter()
                .map(|node| node.outputs.clone())
                .collect::<Vec<_>>(),
        )
    };
    assert_eq!(*x_ref.borrow(), 2.0);
    let source_ref = symbol(decoded.interpreter(), "y").as_f64()?;
    let mut turn_state = ReactiveTurnState::default();
    for expected in [5.0, 7.0] {
        *source_ref.borrow_mut() = expected;
        let outcome = decoded
            .interpreter()
            .plan()
            .advance_reactive_turn(&mut turn_state, &[source_cell])?;
        assert_eq!(
            outcome.before_commit.pending_register_nodes,
            vec![x_register]
        );
        assert_eq!(outcome.register_commit.staged_nodes, vec![x_register]);
        assert_eq!(outcome.register_commit.committed_nodes, vec![x_register]);
        assert_eq!(outcome.register_commit.dirty_cells, vec![x_cell]);
        assert!(outcome.after_commit.executed_nodes.is_empty());
        assert_eq!(*x_ref.borrow(), expected);
        assert_eq!(*decoded_output.as_f64().unwrap().borrow(), expected);
        assert!(turn_state.pending_register_nodes.is_empty());
        let plan = decoded.interpreter().plan();
        let plan = plan.borrow();
        assert_eq!(plan.len(), plan_length);
        assert_eq!(
            plan.nodes.iter().map(|node| node.id).collect::<Vec<_>>(),
            node_ids,
        );
        assert_eq!(
            plan.nodes
                .iter()
                .map(|node| node.outputs.clone())
                .collect::<Vec<_>>(),
            output_cells,
        );
    }
    Ok(())
}

#[test]
fn decoded_matrix_literal_preserves_dependency_chain() -> MResult<()> {
    let code = "[1.0 2.0; 3.0 4.0]";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    let expected = Value::MatrixF64(ValueMatrix::from_vec(vec![1.0, 3.0, 2.0, 4.0], 2, 2));
    assert_eq!(source_output, expected);
    assert_eq!(decoded_output, expected);
    assert_matrix_literal_chain(&source.interpreter().plan());
    assert_matrix_literal_chain(&decoded.interpreter().plan());
    Ok(())
}

#[test]
fn decoded_set_literal_registers_structural_node() -> MResult<()> {
    let code = "{1.0, 2.0}";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    assert_eq!(decoded_output, source_output);
    assert_structural_set_node(&source.interpreter().plan(), &source_output);
    assert_structural_set_node(&decoded.interpreter().plan(), &decoded_output);
    Ok(())
}
