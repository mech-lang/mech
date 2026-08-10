use mech_bytecode::{
    CompileCtx, CompiledBytecode, CompiledInstructionRole, CompiledIntegrityConstraint,
    CompiledNodeKind, CompiledSymbolDefinition,
};
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction,
    BytecodeProgram, ChangeDetectionPolicy, DeliveryMode, DimensionExpr, EncodedConstant,
    ExecutionResourceRequest, ExternalInteraction, FloatWidth, FunctionArgs, FunctionArgumentRole,
    FunctionCatalog, FunctionCatalogBuilder, FunctionRuntimeType, FunctionSpecializer,
    GuardFunctionSafety, IncorrectNumberOfArguments, InputPortLayout, InputPortPolicy, LegacyValue,
    MResult, MechError, MechFunction, MechFunctionCompiler, MechFunctionFactory, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ParsedProgram, Plan,
    ReactiveCellId, ReactiveDependencyKind, ReactiveNodeId, ReactiveNodeKind, ReactiveTurnState,
    Ref, Register, ResolvedOperationContract, ResourceDelivery, ResourceIntent,
    RuntimeFunctionContract, RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType,
    SchemaBody, ShapeRule, ValueData, ValueKind, compile_value_register, hash_str,
};
use mech_engine::Interpreter;
use mech_engine::{
    ArtifactSource, BindingDeclaration, InitializerReference, MechProgram, MechProgramConfig,
    ProducerReference, ProgramArtifact, ProgramInputId, ProgramInputUpdate, SlotRole,
    decode_program_artifact_sections,
};
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

const TEST_LESS_RUNTIME: &str = "TestLessF64";

#[derive(Debug)]
struct TestLessFunction {
    lhs: Ref<f64>,
    rhs: Ref<f64>,
    out: Ref<bool>,
}

impl MechFunctionImpl for TestLessFunction {
    fn solve_result(&self) -> MResult<()> {
        *self.out.borrow_mut() = *self.lhs.borrow() < *self.rhs.borrow();
        Ok(())
    }

    fn out(&self) -> LegacyValue {
        LegacyValue::Bool(self.out.clone())
    }

    fn transaction_state_values(&self) -> MResult<Vec<LegacyValue>> {
        Ok(self.reactive_output_values())
    }

    fn to_string(&self) -> String {
        TEST_LESS_RUNTIME.to_owned()
    }
}

impl MechFunctionCompiler for TestLessFunction {
    fn compile(&self, context: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        let output = LegacyValue::Bool(self.out.clone());
        let lhs = LegacyValue::F64(self.lhs.clone());
        let rhs = LegacyValue::F64(self.rhs.clone());
        let destination = compile_value_register(&output, self.out.addr(), context)?;
        let lhs = compile_value_register(&lhs, self.lhs.addr(), context)?;
        let rhs = compile_value_register(&rhs, self.rhs.addr(), context)?;
        context.emit_binop(hash_str(TEST_LESS_RUNTIME), destination, lhs, rhs);
        Ok(destination)
    }
}

struct TestLessFactory;

impl MechFunctionFactory for TestLessFactory {
    const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
        <bool as FunctionRuntimeType>::REPRESENTATION,
        <f64 as FunctionRuntimeType>::REPRESENTATION,
        <f64 as FunctionRuntimeType>::REPRESENTATION,
    );

    fn new(arguments: FunctionArgs) -> MResult<Box<dyn MechFunction>> {
        let FunctionArgs::Binary(output, lhs, rhs) = arguments else {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 3,
                    found: 0,
                },
                None,
            )
            .with_compiler_loc());
        };
        Ok(Box::new(TestLessFunction {
            lhs: lhs.try_function_ref(FunctionArgumentRole::Input(0))?,
            rhs: rhs.try_function_ref(FunctionArgumentRole::Input(1))?,
            out: output.try_function_ref(FunctionArgumentRole::Output)?,
        }))
    }
}

struct TestLessSpecializer;

impl FunctionSpecializer for TestLessSpecializer {
    fn specialize(&self, arguments: &[LegacyValue]) -> MResult<Box<dyn MechFunction>> {
        let [lhs, rhs] = arguments else {
            return Err(MechError::new(
                IncorrectNumberOfArguments {
                    expected: 2,
                    found: arguments.len(),
                },
                None,
            )
            .with_compiler_loc());
        };
        Ok(Box::new(TestLessFunction {
            lhs: lhs.as_f64()?,
            rhs: rhs.as_f64()?,
            out: Ref::new(false),
        }))
    }

    fn guard_safety(&self) -> GuardFunctionSafety {
        GuardFunctionSafety::PureStatic
    }
}

fn source_catalog() -> Arc<FunctionCatalog> {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    mech_engine::install_intrinsic_source(&mut builder).unwrap();
    builder
        .insert_runtime_factory::<TestLessFactory>(
            TEST_LESS_RUNTIME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
        )
        .unwrap();
    builder
        .insert_intrinsic_specializer("compare/lt", Arc::new(TestLessSpecializer))
        .unwrap();
    Arc::new(builder.build().unwrap())
}

fn source_program() -> MechProgram {
    MechProgram::with_function_catalog(MechProgramConfig::default(), source_catalog())
}

fn runtime_program() -> MechProgram {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    MechProgram::with_function_catalog(
        MechProgramConfig::default(),
        Arc::new(builder.build().unwrap()),
    )
}

fn symbol(interpreter: &Interpreter, name: &str) -> LegacyValue {
    interpreter
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

fn root_cell(value: &LegacyValue) -> ReactiveCellId {
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

fn assert_alias_node(plan: &Plan, name: &str, output: &LegacyValue) {
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

fn decoded_assignment_graph_shape(
    interpreter: &Interpreter,
    output: &LegacyValue,
) -> RegisterGraphShape {
    let resolved_output = match output {
        LegacyValue::MutableReference(reference) => reference.borrow().clone(),
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

fn set_members(value: &LegacyValue) -> Vec<ReactiveCellId> {
    match value {
        LegacyValue::Set(set) => set
            .borrow()
            .set
            .iter()
            .flat_map(LegacyValue::reactive_root_cell_ids)
            .collect(),
        other => panic!("expected set, found {other:?}"),
    }
}

fn assert_structural_set_node(plan: &Plan, output: &LegacyValue) {
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
fn ordinary_mech_sources_emit_equivalent_program_artifacts_in_bytecode_v1() -> MResult<()> {
    for source in [
        include_str!("fixtures/program-artifact/scalar-alias.mec"),
        include_str!("fixtures/program-artifact/state-register.mec"),
        include_str!("fixtures/program-artifact/matrix-literal.mec"),
        include_str!("fixtures/program-artifact/comparison-output.mec"),
        include_str!("fixtures/program-artifact/integrity-constraint.mec"),
    ] {
        let mut program = source_program();
        program.run_string(source)?;
        let product = program.compile_program_product()?;
        let artifact_a = product.artifact();
        let parsed = ParsedProgram::from_bytes(product.bytecode())?;

        assert!(!parsed.artifact.is_empty());
        assert!(
            [
                &parsed.artifact.schemas,
                &parsed.artifact.constants,
                &parsed.artifact.inputs,
                &parsed.artifact.slots,
                &parsed.artifact.producers,
                &parsed.artifact.nodes,
                &parsed.artifact.bindings,
                &parsed.artifact.outputs,
                &parsed.artifact.integrity_constraints,
                &parsed.artifact.operations,
            ]
            .iter()
            .all(|section| !section.is_empty())
        );
        let artifact_b = decode_program_artifact_sections(&parsed.artifact)
            .expect("normal bytecode-v1 artifact sections must decode");
        assert_eq!(artifact_a.revision(), artifact_b.revision());
        assert_eq!(artifact_a.inputs(), artifact_b.inputs());
        assert_eq!(artifact_a.slots(), artifact_b.slots());
        assert_eq!(artifact_a.nodes(), artifact_b.nodes());
        assert_eq!(artifact_a.bindings(), artifact_b.bindings());
        assert_eq!(artifact_a.outputs(), artifact_b.outputs());
        assert!(!artifact_a.schemas().is_empty());
        assert!(!artifact_a.constants().is_empty());
    }
    Ok(())
}

fn compile_artifact_fixture(source: &str) -> MResult<(ProgramArtifact, ProgramArtifact)> {
    let mut program = source_program();
    program.run_string(source)?;
    let (source_artifact, bytecode) = program.compile_program_product()?.into_parts();
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let bytecode_artifact = decode_program_artifact_sections(&parsed.artifact)
        .expect("fixture bytecode artifact sections must decode");
    assert_eq!(source_artifact.revision(), bytecode_artifact.revision());
    assert_eq!(source_artifact.inputs(), bytecode_artifact.inputs());
    assert_eq!(source_artifact.slots(), bytecode_artifact.slots());
    assert_eq!(source_artifact.nodes(), bytecode_artifact.nodes());
    assert_eq!(source_artifact.bindings(), bytecode_artifact.bindings());
    assert_eq!(source_artifact.outputs(), bytecode_artifact.outputs());
    assert_eq!(
        source_artifact.constraints(),
        bytecode_artifact.constraints()
    );
    Ok((source_artifact, bytecode_artifact))
}

fn assert_f64_schema(artifact: &ProgramArtifact, schema: mech_core::SchemaId) {
    let body = artifact.schemas().get(schema).unwrap().body();
    assert!(
        matches!(body, SchemaBody::FloatingPoint(FloatWidth::W64)),
        "expected f64 schema, got {body:?}"
    );
}

fn artifact_source_schema(
    artifact: &ProgramArtifact,
    source: ArtifactSource,
) -> mech_core::SchemaId {
    match source {
        ArtifactSource::Constant(constant) => artifact.constants().get(constant).unwrap().schema(),
        ArtifactSource::Slot(slot) => artifact.slots()[slot.get() as usize].schema,
    }
}

#[test]
fn ordinary_source_artifacts_preserve_exact_semantics() -> MResult<()> {
    let (scalar, _) =
        compile_artifact_fixture(include_str!("fixtures/program-artifact/scalar-alias.mec"))?;
    assert_eq!(
        scalar
            .inputs()
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        vec!["input"]
    );
    assert_eq!(
        scalar
            .outputs()
            .iter()
            .map(|output| output.name.as_str())
            .collect::<Vec<_>>(),
        vec!["output"]
    );
    assert_f64_schema(&scalar, scalar.outputs()[0].schema);
    assert!(
        scalar
            .inputs()
            .iter()
            .all(|input| !input.name.starts_with("input-"))
    );

    let (state, _) =
        compile_artifact_fixture(include_str!("fixtures/program-artifact/state-register.mec"))?;
    assert_eq!(
        state
            .inputs()
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        vec!["input"]
    );
    let state_slots = state
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .collect::<Vec<_>>();
    assert_eq!(state_slots.len(), 1);
    let InitializerReference::Constant(initializer) = state_slots[0]
        .initializer
        .expect("state source initializer must be retained");
    let ValueData::F64(initializer) = state.constants().get(initializer).unwrap().data() else {
        panic!("state initializer must be an f64 snapshot")
    };
    assert_eq!(initializer.to_f64(), 2.0);
    assert!(state.inputs().iter().all(|input| input.name != "state"));
    assert_eq!(state.outputs()[0].name, "output");
    assert_eq!(state.outputs()[0].source, state_slots[0].slot);

    let (matrix, _) =
        compile_artifact_fixture(include_str!("fixtures/program-artifact/matrix-literal.mec"))?;
    assert!(matrix.inputs().is_empty());
    assert_eq!(matrix.outputs()[0].name, "matrix");
    assert!(matches!(
        matrix.schemas().get(matrix.outputs()[0].schema).unwrap().body(),
        SchemaBody::Matrix {
            element,
            dimensions,
        } if matches!(element.as_ref(), SchemaBody::FloatingPoint(FloatWidth::W64))
            && dimensions.as_ref()
                == [mech_core::DimensionExpr::Constant(2), mech_core::DimensionExpr::Constant(2)]
    ));
    assert!(
        matrix
            .constants()
            .get(mech_core::ConstantId::new(0))
            .is_some()
            || !matrix.nodes().is_empty()
    );

    let (comparison, _) = compile_artifact_fixture(include_str!(
        "fixtures/program-artifact/comparison-output.mec"
    ))?;
    assert_eq!(comparison.outputs()[0].name, "less");
    assert!(matches!(
        comparison
            .schemas()
            .get(comparison.outputs()[0].schema)
            .unwrap()
            .body(),
        SchemaBody::Bool
    ));
    let ProducerReference::NodeOutput { node, .. } = comparison
        .slots()
        .iter()
        .find(|slot| slot.slot == comparison.outputs()[0].source)
        .unwrap()
        .producer
    else {
        panic!("comparison output must be produced by a node")
    };
    let comparison_node = &comparison.nodes()[node.get() as usize];
    let comparison_input_schemas = comparison_node
        .input_bindings
        .clone()
        .map(|binding| match comparison.bindings()[binding as usize] {
            BindingDeclaration::Input { source, .. } => artifact_source_schema(&comparison, source),
            _ => panic!("comparison input range contained an output binding"),
        })
        .collect::<Vec<_>>();
    assert_eq!(comparison_input_schemas.len(), 2);
    for schema in comparison_input_schemas {
        assert_f64_schema(&comparison, schema);
    }
    let catalog = source_catalog();
    for node in comparison.nodes() {
        let exact_name = if node.operation.module_path.as_ref() == ["runtime"] {
            node.operation.operation_name.clone()
        } else {
            node.operation
                .module_path
                .iter()
                .chain(std::iter::once(&node.operation.operation_name))
                .cloned()
                .collect::<Vec<_>>()
                .join("/")
        };
        assert!(
            catalog
                .runtime_entries()
                .any(|entry| entry.name == exact_name),
            "artifact operation {exact_name:?} must preserve an installed runtime entry name"
        );
        assert!(!exact_name.starts_with("runtime-"));
        assert!(!exact_name.starts_with("host-"));
        assert!(!exact_name.starts_with("resource-"));
    }

    let (integrity, decoded_integrity) = compile_artifact_fixture(include_str!(
        "fixtures/program-artifact/integrity-constraint.mec"
    ))?;
    assert_eq!(integrity.constraints().len(), 1);
    assert_eq!(integrity.constraints(), decoded_integrity.constraints());
    assert_eq!(
        integrity.constraints()[0].operation.module_path.as_ref(),
        ["integrity"]
    );
    assert_eq!(
        integrity.constraints()[0].operation.operation_name,
        "assert"
    );
    assert_eq!(integrity.constraints()[0].inputs.len(), 1);
    assert!(matches!(
        integrity
            .schemas()
            .get(artifact_source_schema(
                &integrity,
                integrity.constraints()[0].inputs[0]
            ))
            .unwrap()
            .body(),
        SchemaBody::Bool
    ));
    assert!(integrity.nodes().iter().all(|node| {
        node.operation.module_path.as_ref() != ["integrity"]
            || node.operation.operation_name != "constraint"
    }));
    Ok(())
}

#[test]
fn equal_interned_constants_keep_distinct_register_roles() -> MResult<()> {
    let (artifact, decoded) =
        compile_artifact_fixture("input := 1.0\n~state := 1.0\nstate = input\noutput := state")?;
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(artifact.inputs().len(), 1);
    assert_eq!(artifact.inputs()[0].name, "input");
    let state = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .expect("equal state initializer must retain its state role");
    assert!(state.initializer.is_some());
    Ok(())
}

#[test]
fn composite_register_helpers_do_not_become_state_and_keep_the_initializer() -> MResult<()> {
    let tuple = LegacyValue::Tuple(Ref::new(mech_core::MechTuple::from_vec(vec![
        LegacyValue::from(1.0_f64),
        LegacyValue::from(2.0_f64),
    ])));
    let mut encoder = CompileCtx::new();
    let tuple_register = encoder.resolve_value_register(&tuple)?;
    let encoded = encoder.finish_program(tuple_register)?;
    let template = encoded
        .program
        .constants
        .into_iter()
        .find(|constant| matches!(constant.runtime_type, RuntimeType::Tuple(_)))
        .expect("tuple compiler must emit a composite template");
    let catalog = source_catalog();
    let function = catalog
        .runtime_entries()
        .find(|entry| entry.name == TEST_LESS_RUNTIME)
        .unwrap()
        .id
        .raw();
    let mut compiled = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::CompositePack {
                dst: 2,
                template: 2,
                children: vec![0, 1],
            },
            BytecodeInstruction::RuntimeUnary {
                function,
                dst: 2,
                src: 2,
            },
            BytecodeInstruction::Return { src: 2 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )),
            Some(CompiledInstructionRole::Node(CompiledNodeKind::Register)),
            None,
        ],
        vec![
            Some(ValueKind::F64),
            Some(ValueKind::F64),
            Some(tuple.kind()),
        ],
        vec![f64_encoded(1.0), f64_encoded(2.0), template],
        Vec::new(),
        2,
    );
    compiled.symbol_definitions.push(CompiledSymbolDefinition {
        id: hash_str("state"),
        name: "state".to_owned(),
        register: 2,
        mutable: true,
        ordinal: 0,
    });
    let artifact = mech_engine::compile_executable_program_artifact(&compiled, &catalog).unwrap();
    let sections = mech_engine::encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&sections).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    let states = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .collect::<Vec<_>>();
    assert_eq!(states.len(), 1, "only the temporal assignment is stateful");
    let InitializerReference::Constant(initializer) = states[0]
        .initializer
        .expect("composite state must retain a snapshot initializer");
    let initializer = artifact.constants().get(initializer).unwrap();
    assert_eq!(initializer.schema(), states[0].schema);
    assert!(matches!(initializer.data(), ValueData::Tuple(_)));
    Ok(())
}

#[test]
fn collection_schemas_use_actual_element_cardinality() -> MResult<()> {
    let (set, _) = compile_artifact_fixture("values := {1.0, 2.0, 3.0}\nvalues")?;
    assert!(matches!(
        set.schemas().get(set.outputs()[0].schema).unwrap().body(),
        SchemaBody::Set {
            cardinality: DimensionExpr::Constant(3),
            ..
        }
    ));

    let (map, _) = compile_artifact_fixture("values := {\"first\": 1.0, \"second\": 2.0}\nvalues")?;
    assert!(matches!(
        map.schemas().get(map.outputs()[0].schema).unwrap().body(),
        SchemaBody::Map {
            cardinality: DimensionExpr::Constant(2),
            ..
        }
    ));
    Ok(())
}

fn f64_encoded(value: f64) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::F64,
        alignment: 8,
        bytes: value.to_bits().to_le_bytes().to_vec(),
    }
}

fn bool_encoded(value: bool) -> EncodedConstant {
    EncodedConstant {
        runtime_type: RuntimeType::Bool,
        alignment: 1,
        bytes: vec![u8::from(value)],
    }
}

fn compiled_fixture(
    instructions: Vec<BytecodeInstruction>,
    instruction_roles: Vec<Option<CompiledInstructionRole>>,
    register_kinds: Vec<Option<ValueKind>>,
    constants: Vec<EncodedConstant>,
    requirements: Vec<ApplicationRequirement>,
    return_register: Register,
) -> CompiledBytecode {
    CompiledBytecode {
        program: BytecodeProgram {
            register_count: register_kinds.len() as u32,
            constants,
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions,
            dictionary: BTreeMap::new(),
            requirements,
        },
        instruction_contracts: vec![None; instruction_roles.len()],
        instruction_source_nodes: vec![None; instruction_roles.len()],
        instruction_roles,
        register_collection_cardinalities: vec![None; register_kinds.len()],
        register_kinds,
        symbol_definitions: Vec::new(),
        return_register,
        integrity_constraints: Vec::new(),
    }
}

fn valid_compiled_fixture() -> CompiledBytecode {
    let function = source_catalog()
        .runtime_entries()
        .find(|entry| entry.name == TEST_LESS_RUNTIME)
        .unwrap()
        .id
        .raw();
    compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::RuntimeUnary {
                function,
                dst: 1,
                src: 0,
            },
            BytecodeInstruction::Return { src: 1 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )),
            None,
        ],
        vec![Some(ValueKind::F64), Some(ValueKind::F64)],
        vec![f64_encoded(1.0), f64_encoded(0.0)],
        Vec::new(),
        1,
    )
}

fn resource_requirement() -> ApplicationRequirement {
    ApplicationRequirement::Resource(ExecutionResourceRequest {
        base_uri: "test://fixture".to_owned(),
        path: "value".to_owned(),
        context_name: "ctx".to_owned(),
        operation: "read".to_owned(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Snapshot,
    })
}

fn unary_declared_contract() -> &'static OperationContractDeclaration {
    Box::leak(Box::new(OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::ExactScalar,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }))
}

#[test]
fn compiler_sidecar_resolves_declared_contracts_into_the_artifact() {
    let catalog = source_catalog();
    let mut compiled = valid_compiled_fixture();
    compiled.instruction_contracts[2] = Some(unary_declared_contract());
    compiled.instruction_source_nodes[2] = Some(0);

    let artifact = mech_engine::compile_executable_program_artifact(&compiled, &catalog).unwrap();
    let node = &artifact.nodes()[0];
    let ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(node.contract).unwrap()
    else {
        panic!("declared compiler metadata became opaque");
    };
    assert_eq!(contract.inputs.len(), 1);
    assert_eq!(contract.outputs.len(), 1);
    assert_eq!(contract.interaction, ExternalInteraction::Pure);
}

#[test]
fn catalog_contract_fills_an_empty_specialized_function_sidecar() {
    let mut builder = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_runtime(&mut builder).unwrap();
    builder
        .insert_runtime_factory_with_semantic_contract::<TestLessFactory>(
            TEST_LESS_RUNTIME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            unary_declared_contract(),
        )
        .unwrap();
    let catalog = builder.build().unwrap();
    let compiled = valid_compiled_fixture();
    assert!(compiled.instruction_contracts[2].is_none());

    let artifact = mech_engine::compile_executable_program_artifact(&compiled, &catalog).unwrap();
    let node = &artifact.nodes()[0];
    let ResolvedOperationContract::Declared(contract) =
        artifact.contracts().get(node.contract).unwrap()
    else {
        panic!("catalog contract became opaque");
    };
    assert_eq!(contract.inputs.len(), 1);
    assert_eq!(contract.outputs.len(), 1);
    assert_eq!(contract.interaction, ExternalInteraction::Pure);
}

#[test]
fn malformed_compiled_sidecars_fail_closed() {
    let catalog = source_catalog();

    let mut missing_role = valid_compiled_fixture();
    missing_role.instruction_roles[2] = None;
    let missing_role_error =
        mech_engine::compile_executable_program_artifact(&missing_role, &catalog).unwrap_err();
    assert!(
        matches!(
            &missing_role_error,
            mech_engine::ArtifactBuildError::MissingInstructionRole { instruction: 2 }
        ),
        "unexpected missing-role error: {missing_role_error:?}"
    );

    let mut role_length = valid_compiled_fixture();
    role_length.instruction_roles.pop();
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&role_length, &catalog),
        Err(
            mech_engine::ArtifactBuildError::CompiledMetadataLengthMismatch {
                table: "instruction_roles",
                ..
            }
        )
    ));

    let mut kind_length = valid_compiled_fixture();
    kind_length.register_kinds.pop();
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&kind_length, &catalog),
        Err(
            mech_engine::ArtifactBuildError::CompiledMetadataLengthMismatch {
                table: "register_kinds",
                ..
            }
        )
    ));

    let mut cardinality_length = valid_compiled_fixture();
    cardinality_length.register_collection_cardinalities.pop();
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&cardinality_length, &catalog),
        Err(
            mech_engine::ArtifactBuildError::CompiledMetadataLengthMismatch {
                table: "register_collection_cardinalities",
                ..
            }
        )
    ));

    let mut mismatched_return = valid_compiled_fixture();
    let last = mismatched_return.program.instructions.len() - 1;
    mismatched_return.program.instructions[last] = BytecodeInstruction::Return { src: 0 };
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&mismatched_return, &catalog),
        Err(
            mech_engine::ArtifactBuildError::CompiledReturnRegisterMismatch {
                expected: 1,
                found: 0,
                ..
            }
        )
    ));

    let mut nonterminal_return = valid_compiled_fixture();
    nonterminal_return.program.instructions.swap(2, 3);
    nonterminal_return.instruction_roles.swap(2, 3);
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&nonterminal_return, &catalog),
        Err(mech_engine::ArtifactBuildError::NonTerminalCompiledReturn { .. })
    ));

    let mut missing_destination_kind = valid_compiled_fixture();
    missing_destination_kind.register_kinds[1] = None;
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&missing_destination_kind, &catalog),
        Err(mech_engine::ArtifactBuildError::MissingRegisterKind {
            instruction: 2,
            register: 1,
        })
    ));

    let function = catalog
        .runtime_entries()
        .find(|entry| entry.name == TEST_LESS_RUNTIME)
        .unwrap()
        .id
        .raw();
    let missing_source = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::RuntimeUnary {
                function,
                dst: 1,
                src: 0,
            },
            BytecodeInstruction::Return { src: 1 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )),
            None,
        ],
        vec![Some(ValueKind::Empty), Some(ValueKind::F64)],
        vec![
            EncodedConstant {
                runtime_type: RuntimeType::Empty,
                alignment: 1,
                bytes: Vec::new(),
            },
            f64_encoded(0.0),
        ],
        Vec::new(),
        1,
    );
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&missing_source, &catalog),
        Err(mech_engine::ArtifactBuildError::MissingRegisterSource {
            instruction: 2,
            register: 0,
            role: "input",
        })
    ));

    let mut unknown_runtime = valid_compiled_fixture();
    let BytecodeInstruction::RuntimeUnary { function, .. } =
        &mut unknown_runtime.program.instructions[2]
    else {
        unreachable!()
    };
    *function = u64::MAX;
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&unknown_runtime, &catalog),
        Err(mech_engine::ArtifactBuildError::UnknownRuntimeFunction { function: u64::MAX })
    ));

    let requirement_mismatch = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::HostCall {
                requirement: 0,
                dst: 1,
                arguments: vec![0],
            },
            BytecodeInstruction::Return { src: 1 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )),
            None,
        ],
        vec![Some(ValueKind::F64), Some(ValueKind::F64)],
        vec![f64_encoded(1.0), f64_encoded(0.0)],
        vec![resource_requirement()],
        1,
    );
    let mismatch_error =
        mech_engine::compile_executable_program_artifact(&requirement_mismatch, &catalog)
            .unwrap_err();
    assert!(
        matches!(
            &mismatch_error,
            mech_engine::ArtifactBuildError::ApplicationRequirementKindMismatch {
                requirement: 0,
                expected: "host function",
            }
        ),
        "unexpected requirement mismatch error: {mismatch_error:?}"
    );

    let mut missing_requirement = requirement_mismatch.clone();
    missing_requirement.program.requirements.clear();
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&missing_requirement, &catalog),
        Err(mech_engine::ArtifactBuildError::UnknownApplicationRequirement { requirement: 0 })
    ));

    let mut ambiguous_role = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        vec![None, None],
        vec![Some(ValueKind::Reference(Box::new(ValueKind::F64)))],
        vec![f64_encoded(1.0)],
        Vec::new(),
        0,
    );
    ambiguous_role.symbol_definitions = vec![
        CompiledSymbolDefinition {
            id: hash_str("external"),
            name: "external".to_owned(),
            register: 0,
            mutable: false,
            ordinal: 0,
        },
        CompiledSymbolDefinition {
            id: hash_str("state"),
            name: "state".to_owned(),
            register: 0,
            mutable: true,
            ordinal: 1,
        },
    ];
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&ambiguous_role, &catalog),
        Err(mech_engine::ArtifactBuildError::AmbiguousRegisterRole { register: 0 })
    ));

    let mut non_boolean_integrity = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::RuntimeVariadic {
                function: hash_str("integrity/constraint"),
                dst: 1,
                arguments: vec![0],
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::IntegrityMarker),
            None,
        ],
        vec![Some(ValueKind::F64), Some(ValueKind::Bool)],
        vec![f64_encoded(1.0), bool_encoded(false)],
        Vec::new(),
        0,
    );
    non_boolean_integrity.integrity_constraints =
        vec![CompiledIntegrityConstraint { result_register: 0 }];
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&non_boolean_integrity, &catalog),
        Err(
            mech_engine::ArtifactBuildError::IntegrityConstraintSchemaMismatch {
                constraint: 0,
                ..
            }
        )
    ));

    let mut missing_integrity_declaration = non_boolean_integrity.clone();
    missing_integrity_declaration.integrity_constraints.clear();
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&missing_integrity_declaration, &catalog),
        Err(
            mech_engine::ArtifactBuildError::IntegrityConstraintMetadataMismatch {
                marker_register: Some(0),
                declared_register: None,
                ..
            }
        )
    ));

    let mut wrong_integrity_register = non_boolean_integrity.clone();
    wrong_integrity_register.integrity_constraints =
        vec![CompiledIntegrityConstraint { result_register: 1 }];
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&wrong_integrity_register, &catalog),
        Err(
            mech_engine::ArtifactBuildError::IntegrityConstraintMetadataMismatch {
                marker_register: Some(0),
                declared_register: Some(1),
                ..
            }
        )
    ));

    let nominal_name = "source-atom";
    let nominal_id = hash_str(nominal_name);
    let unresolved_nominal = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        vec![None, None],
        vec![Some(ValueKind::Atom(nominal_id, nominal_name.to_owned()))],
        vec![EncodedConstant {
            runtime_type: RuntimeType::Atom {
                id: nominal_id,
                name: nominal_name.to_owned(),
            },
            alignment: 1,
            bytes: Vec::new(),
        }],
        Vec::new(),
        0,
    );
    assert!(matches!(
        mech_engine::compile_executable_program_artifact(&unresolved_nominal, &catalog),
        Err(mech_engine::ArtifactBuildError::Semantic(
            mech_core::SemanticModelError::LegacyNominalUnresolved {
                kind: mech_core::NominalKind::Atom,
                legacy_id,
                legacy_name,
            }
        )) if legacy_id == nominal_id && legacy_name == nominal_name
    ));
}

#[test]
fn pseudo_destination_effects_preserve_the_node_and_every_input() {
    let compiled = compiled_fixture(
        vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::ConstLoad {
                dst: 1,
                constant: 1,
            },
            BytecodeInstruction::ResourceSend {
                requirement: 0,
                dst: 0,
                src: 1,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        vec![
            None,
            None,
            Some(CompiledInstructionRole::Node(
                CompiledNodeKind::Combinational,
            )),
            None,
        ],
        vec![Some(ValueKind::Empty), Some(ValueKind::F64)],
        vec![
            EncodedConstant {
                runtime_type: RuntimeType::Empty,
                alignment: 1,
                bytes: Vec::new(),
            },
            f64_encoded(1.0),
        ],
        vec![ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "test://fixture".to_owned(),
            path: "value".to_owned(),
            context_name: "ctx".to_owned(),
            operation: "publish/value".to_owned(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
        })],
        0,
    );

    let artifact =
        mech_engine::compile_executable_program_artifact(&compiled, &source_catalog()).unwrap();
    let effect = artifact
        .nodes()
        .iter()
        .find(|node| {
            node.operation.module_path.as_ref() == ["resource", "send", "publish"]
                && node.operation.operation_name == "value"
        })
        .expect("resource send must remain an artifact node");

    assert_eq!(effect.input_bindings.end - effect.input_bindings.start, 1);
    assert_eq!(effect.output_bindings.end - effect.output_bindings.start, 0);
    assert!(artifact.outputs().is_empty());
    assert!(matches!(
        artifact.bindings()[effect.input_bindings.start as usize],
        BindingDeclaration::Input {
            source: ArtifactSource::Constant(_),
            ..
        }
    ));
}

#[test]
fn tuple_constant_round_trips_through_bytecode() -> MResult<()> {
    let code = "tuple := (1, 2); tuple.2";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    assert_alias_node(
        &source.interpreter().plan(),
        "TupleAccessElement",
        &source_output,
    );
    let bytecode = source.compile_bytecode()?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    assert!(
        parsed
            .types
            .iter()
            .any(|ty| matches!(ty, RuntimeType::Tuple(_)))
    );
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;
    assert_eq!(decoded_output, source_output);
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
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let matrix_comprehensions = parsed
        .instructions
        .iter()
        .filter_map(|instruction| match instruction {
            BytecodeInstruction::RuntimeVariadic { arguments, .. } => Some(arguments.len()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        matrix_comprehensions,
        vec![2, 2],
        "each row comprehension must encode both child registers",
    );
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;

    for output in [&source_output, &decoded_output] {
        match output {
            LegacyValue::MatrixF64(matrix) => {
                assert_eq!(matrix.shape(), vec![2, 2]);
                assert_eq!(matrix.as_vec(), vec![1.0, 3.0, 2.0, 4.0]);
            }
            other => panic!("expected f64 matrix literal, got {other:?}"),
        }
    }
    assert_matrix_literal_chain(&source.interpreter().plan());
    assert_matrix_literal_chain(&decoded.interpreter().plan());
    Ok(())
}

#[test]
fn decoded_matrix_comprehension_publishes_reactive_results() -> MResult<()> {
    let mut source = source_program();
    source.run_string("x := 1.0\npayload := [x 2.0]\npayload")?;
    let bytecode = source.compile_bytecode()?;
    let mut decoded = runtime_program();
    decoded.run_bytecode(&bytecode)?;

    decoded.update_inputs_and_advance_turn(&[ProgramInputUpdate {
        input: ProgramInputId {
            interpreter_id: decoded.interpreter().id,
            symbol_id: hash_str("x"),
        },
        value: LegacyValue::from(3.0f64),
    }])?;

    let LegacyValue::MatrixF64(payload) = decoded.root_symbol_value("payload")? else {
        panic!("expected decoded matrix payload")
    };
    assert_eq!(payload.as_vec(), vec![3.0, 2.0]);
    Ok(())
}

#[test]
fn set_constant_round_trips_through_bytecode() -> MResult<()> {
    let code = "{1.0, 2.0}";
    let mut source = source_program();
    let source_output = source.run_string(code)?;
    assert_structural_set_node(&source.interpreter().plan(), &source_output);
    let bytecode = source.compile_bytecode()?;
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    assert!(
        parsed
            .types
            .iter()
            .any(|ty| matches!(ty, RuntimeType::Set { .. }))
    );
    let mut decoded = runtime_program();
    let decoded_output = decoded.run_bytecode(&bytecode)?;
    assert_eq!(decoded_output, source_output);
    Ok(())
}
