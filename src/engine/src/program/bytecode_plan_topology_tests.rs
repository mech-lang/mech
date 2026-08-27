#![cfg(feature = "compiler")]

use crate::{
    ArtifactSource, BindingDeclaration, CompilerPlanningConfig, CompilerPlanningProgram,
    InitializerReference, ProducerReference, ProgramArtifact, SlotRole,
    decode_program_artifact_sections,
};
use crate::{
    CompileCtx, CompiledBytecode, CompiledInstructionRole, CompiledIntegrityConstraint,
    CompiledNodeKind, CompiledSymbolDefinition,
};
#[cfg(feature = "native-plan")]
use mech_core::snapshot::SequenceView;
use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction,
    BytecodeProgram, ChangeDetectionPolicy, DeliveryMode, DimensionExpr, EncodedConstant,
    ExecutionResourceRequest, ExternalInteraction, FloatWidth, FunctionArgs, FunctionArgumentRole,
    FunctionCatalog, FunctionCatalogBuilder, FunctionRuntimeType, FunctionSpecializer,
    GuardFunctionSafety, IncorrectNumberOfArguments, InputPortLayout, InputPortPolicy, LegacyValue,
    MResult, MechError, MechFunction, MechFunctionCompiler, MechFunctionFactory, MechFunctionImpl,
    OperationContractDeclaration, OutputConstruction, OutputPortPolicy, ParsedProgram, Ref,
    Register, ResolvedOperationContract, ResourceDelivery, ResourceIntent, RuntimeFunctionContract,
    RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType, SchemaBody, SchemaId,
    ShapeRule, ValueData, ValueKind, compile_value_register, hash_str,
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
    crate::install_intrinsic_runtime(&mut builder).unwrap();
    crate::install_intrinsic_source(&mut builder).unwrap();
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

fn source_program() -> CompilerPlanningProgram {
    CompilerPlanningProgram::with_function_catalog(
        CompilerPlanningConfig::default(),
        source_catalog(),
    )
}

fn assert_frozen_v1_topology_parity(artifact_a: &ProgramArtifact, artifact_b: &ProgramArtifact) {
    let artifact = artifact_a;
    assert_eq!(artifact.contracts(), artifact_b.contracts());
    assert_eq!(artifact.inputs(), artifact_b.inputs());
    assert_eq!(artifact.slots(), artifact_b.slots());
    assert_eq!(artifact.bindings(), artifact_b.bindings());
    assert_eq!(artifact.outputs(), artifact_b.outputs());
    assert_eq!(artifact.constraints(), artifact_b.constraints());
    assert_eq!(artifact.schemas().len(), artifact_b.schemas().len());
    for index in 0..artifact.schemas().len() {
        let schema = SchemaId::new(index as u32);
        assert_eq!(
            artifact.schemas().entry(schema).unwrap().canonical_bytes(),
            artifact_b
                .schemas()
                .entry(schema)
                .unwrap()
                .canonical_bytes(),
        );
    }
    assert_eq!(artifact.constants().len(), artifact_b.constants().len());
    assert_eq!(artifact.nodes().len(), artifact_b.nodes().len());

    let mut projected_implementation = false;
    for (source, bytecode) in artifact.nodes().iter().zip(artifact_b.nodes()) {
        assert_eq!(source.node, bytecode.node);
        assert_eq!(source.contract, bytecode.contract);
        assert_eq!(source.requirement, bytecode.requirement);
        assert_eq!(source.input_bindings, bytecode.input_bindings);
        assert_eq!(source.output_bindings, bytecode.output_bindings);
        if source.operation != bytecode.operation {
            projected_implementation = true;
            assert_ne!(source.operation.module_path.as_ref(), ["runtime"]);
            assert_eq!(bytecode.operation.module_path.as_ref(), ["runtime"]);
        }
    }

    if projected_implementation {
        assert_ne!(artifact_a.revision(), artifact_b.revision());
    } else {
        assert_eq!(artifact_a.revision(), artifact_b.revision());
    }
}

#[test]
fn ordinary_mech_sources_emit_equivalent_program_artifacts_in_bytecode_v1() -> MResult<()> {
    for source in [
        include_str!("../../tests/fixtures/program-artifact/scalar-alias.mec"),
        include_str!("../../tests/fixtures/program-artifact/state-register.mec"),
        include_str!("../../tests/fixtures/program-artifact/matrix-literal.mec"),
        include_str!("../../tests/fixtures/program-artifact/comparison-output.mec"),
        include_str!("../../tests/fixtures/program-artifact/integrity-constraint.mec"),
    ] {
        let mut program = source_program();
        program.plan_source_for_test(source)?;
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
                &parsed.artifact.operation_contracts,
            ]
            .iter()
            .all(|section| !section.is_empty())
        );
        let artifact_b = decode_program_artifact_sections(&parsed.artifact)
            .expect("normal bytecode-v1 artifact sections must decode");
        assert_frozen_v1_topology_parity(artifact_a, &artifact_b);
        assert_eq!(artifact_a.revision(), artifact_b.revision());
        assert!(!artifact_a.schemas().is_empty());
    }
    Ok(())
}

fn compile_artifact_fixture(source: &str) -> MResult<(ProgramArtifact, ProgramArtifact)> {
    let mut program = source_program();
    program.plan_source_for_test(source)?;
    let (source_artifact, bytecode) = program.compile_program_product()?.into_parts();
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let bytecode_artifact = decode_program_artifact_sections(&parsed.artifact)
        .expect("fixture bytecode artifact sections must decode");
    assert_frozen_v1_topology_parity(&source_artifact, &bytecode_artifact);
    assert_eq!(source_artifact.revision(), bytecode_artifact.revision());
    Ok((source_artifact, bytecode_artifact))
}

#[test]
fn frozen_v1_compatibility_product_preserves_implementation_operation_ids() -> MResult<()> {
    let mut program = source_program();
    program.plan_source_for_test("lhs := 1.0\nrhs := 2.0\nlhs < rhs")?;
    let (artifact, bytecode) = program.compile_frozen_v1_program_product()?.into_parts();
    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    let decoded = decode_program_artifact_sections(&parsed.artifact)
        .expect("frozen-v1 compatibility artifact sections must decode");

    assert_frozen_v1_topology_parity(&artifact, &decoded);
    assert_ne!(artifact.revision(), decoded.revision());
    assert!(decoded.nodes().iter().any(|node| {
        node.operation.module_path.as_ref() == ["runtime"]
            && node.operation.operation_name == TEST_LESS_RUNTIME
    }));
    Ok(())
}

#[test]
fn composite_return_materialization_has_semantic_node_metadata() -> MResult<()> {
    let (artifact, decoded) = compile_artifact_fixture("(1.0, 2.0)")?;
    let composite = artifact
        .nodes()
        .iter()
        .find(|node| {
            node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack"
        })
        .expect("direct tuple return must retain its composite-pack node");
    assert_eq!(composite.input_bindings.len(), 3);
    assert_eq!(composite.output_bindings.len(), 1);
    assert_eq!(artifact.outputs().len(), 1);
    assert_eq!(artifact.outputs(), decoded.outputs());
    Ok(())
}

#[test]
fn immutable_composite_definitions_remain_reactive_packs() -> MResult<()> {
    let (artifact, _) =
        compile_artifact_fixture("first := 1.0\nsecond := 2.0\npair := (first, second)\npair")?;
    let composite = artifact
        .nodes()
        .iter()
        .find(|node| {
            node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack"
        })
        .expect("an immutable composite definition must not be frozen as a startup constant");
    assert!(composite.input_bindings.len() >= 2);
    assert_eq!(composite.output_bindings.len(), 1);
    assert_eq!(artifact.outputs().len(), 1);
    Ok(())
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
    let (scalar, _) = compile_artifact_fixture(include_str!(
        "../../tests/fixtures/program-artifact/scalar-alias.mec"
    ))?;
    assert_eq!(
        scalar
            .inputs()
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "a local constant binding is not a host observation input"
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

    let (state, _) = compile_artifact_fixture(include_str!(
        "../../tests/fixtures/program-artifact/state-register.mec"
    ))?;
    assert_eq!(
        state
            .inputs()
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>(),
        Vec::<&str>::new(),
        "the state source is a local constant, not a host observation input"
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

    let (matrix, _) = compile_artifact_fixture(include_str!(
        "../../tests/fixtures/program-artifact/matrix-literal.mec"
    ))?;
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
        "../../tests/fixtures/program-artifact/comparison-output.mec"
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
    let comparison_output = comparison
        .slots()
        .iter()
        .find(|slot| slot.slot == comparison.outputs()[0].source)
        .expect("comparison output must have a publication slot");
    assert_eq!(comparison_output.role, SlotRole::Output);
    let ProducerReference::Output {
        source: ArtifactSource::Slot(comparison_source),
        ..
    } = comparison_output.producer
    else {
        panic!("comparison output must publish a node-derived slot")
    };
    let ProducerReference::NodeOutput { node, .. } = comparison
        .slots()
        .iter()
        .find(|slot| slot.slot == comparison_source)
        .expect("comparison source slot must exist")
        .producer
    else {
        panic!("comparison source must be produced by a node")
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
    assert!(comparison.nodes().iter().any(|node| {
        node.operation.module_path.as_ref() == ["compare"] && node.operation.operation_name == "lt"
    }));
    assert!(
        comparison
            .nodes()
            .iter()
            .all(|node| node.operation.module_path.as_ref() != ["runtime"]),
        "artifact nodes must name semantic operations, not runtime implementations"
    );

    let (integrity, decoded_integrity) = compile_artifact_fixture(include_str!(
        "../../tests/fixtures/program-artifact/integrity-constraint.mec"
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

#[cfg(feature = "native-plan")]
#[test]
fn mutable_matrix_state_retains_its_declaration_time_initializer() -> MResult<()> {
    let mut program = source_program();
    program.plan_source_for_test(
        "~state := [1.0 2.0; 3.0 4.0]\nreplacement := [0.0 0.0; 0.0 0.0]\nstate = replacement\nstate",
    )?;
    let product = program.compile_program_product()?;
    let artifact = product.artifact();
    let state = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .expect("mutable matrix must produce a state slot");
    let InitializerReference::Constant(initializer) = state
        .initializer
        .expect("mutable matrix state must retain an initializer");
    let ValueData::Matrix(initializer) = artifact.constants().get(initializer).unwrap().data()
    else {
        panic!("matrix state initializer must remain a matrix")
    };
    let SequenceView::F64(values) = initializer.elements() else {
        panic!("matrix state initializer must retain f64 elements")
    };
    assert_eq!(
        values
            .iter()
            .map(|value| value.to_f64())
            .collect::<Vec<_>>(),
        vec![1.0, 2.0, 3.0, 4.0]
    );
    Ok(())
}

#[test]
fn equal_interned_constants_keep_distinct_register_roles() -> MResult<()> {
    let (artifact, decoded) =
        compile_artifact_fixture("input := 1.0\n~state := 1.0\nstate = input\noutput := state")?;
    assert_frozen_v1_topology_parity(&artifact, &decoded);
    assert!(
        artifact.inputs().is_empty(),
        "the immutable local constant must not become a host observation input"
    );
    let state = artifact
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .expect("equal state initializer must retain its state role");
    assert!(state.initializer.is_some());
    Ok(())
}

#[test]
fn multiple_full_state_writers_fail_closed() -> MResult<()> {
    let mut program = source_program();
    program.plan_source_for_test(
        "~state := 1.0\nlimit := 2.0\nbefore := state < limit\nstate = limit\nstate = 3.0\nstate",
    )?;

    let error = program
        .compile_program_product()
        .expect_err("multiple full writers must not produce an ambiguous state artifact");
    assert_eq!(error.kind_name(), "ProgramArtifactCompilationError");
    assert!(error.kind_message().contains("InvalidStateWriterChain"));
    Ok(())
}

#[test]
fn composite_helpers_and_mutable_metadata_without_a_declaration_do_not_become_state() -> MResult<()>
{
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
        root_visible: true,
        ordinal: 0,
    });
    let artifact = crate::compile_executable_program_artifact(&compiled, &catalog).unwrap();
    let sections = crate::encode_program_artifact_sections(&artifact).unwrap();
    let decoded = decode_program_artifact_sections(&sections).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    let states = artifact
        .slots()
        .iter()
        .filter(|slot| slot.role == SlotRole::State)
        .collect::<Vec<_>>();
    assert!(
        states.is_empty(),
        "a composite helper and bare mutable symbol metadata must not manufacture temporal state without a declaration marker"
    );
    let composite = artifact
        .nodes()
        .iter()
        .find(|node| {
            node.operation.module_path.as_ref() == ["core"]
                && node.operation.operation_name == "composite-pack"
        })
        .expect("composite helper remains an artifact node");
    assert_eq!(
        composite.input_bindings.end - composite.input_bindings.start,
        3
    );
    assert!(matches!(
        artifact.bindings()[composite.input_bindings.start as usize],
        BindingDeclaration::Input {
            source: ArtifactSource::Constant(_),
            ..
        }
    ));
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
    let instruction_operations = instructions
        .iter()
        .zip(&instruction_roles)
        .map(|(instruction, role)| {
            (matches!(role, Some(CompiledInstructionRole::Node(_)))
                && matches!(
                    instruction,
                    BytecodeInstruction::RuntimeNullary { .. }
                        | BytecodeInstruction::RuntimeUnary { .. }
                        | BytecodeInstruction::RuntimeBinary { .. }
                        | BytecodeInstruction::RuntimeTernary { .. }
                        | BytecodeInstruction::RuntimeQuaternary { .. }
                        | BytecodeInstruction::RuntimeVariadic { .. }
                ))
            .then(|| "compare/lt".to_owned())
        })
        .collect();
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
        runtime_function_names: BTreeMap::new(),
        instruction_contracts: vec![None; instruction_roles.len()],
        instruction_operations,
        instruction_source_nodes: vec![None; instruction_roles.len()],
        instruction_roles,
        register_collection_cardinalities: vec![None; register_kinds.len()],
        register_state_initializers: vec![None; register_kinds.len()],
        matrix_literals: BTreeMap::new(),
        register_kinds,
        symbol_definitions: Vec::new(),
        return_register,
        integrity_constraints: Vec::new(),
        compute_regions: Vec::new(),
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

    let artifact = crate::compile_executable_program_artifact(&compiled, &catalog).unwrap();
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
    crate::install_intrinsic_runtime(&mut builder).unwrap();
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

    let artifact = crate::compile_executable_program_artifact(&compiled, &catalog).unwrap();
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

    let mut missing_operation = valid_compiled_fixture();
    missing_operation.instruction_operations[2] = None;
    assert!(matches!(
        crate::compile_executable_program_artifact(&missing_operation, &catalog),
        Err(crate::ArtifactBuildError::MissingSemanticOperation {
            instruction: 2,
            implementation,
        }) if implementation == TEST_LESS_RUNTIME
    ));

    let mut missing_role = valid_compiled_fixture();
    missing_role.instruction_roles[2] = None;
    let missing_role_error =
        crate::compile_executable_program_artifact(&missing_role, &catalog).unwrap_err();
    assert!(
        matches!(
            &missing_role_error,
            crate::ArtifactBuildError::MissingInstructionRole { instruction: 2 }
        ),
        "unexpected missing-role error: {missing_role_error:?}"
    );

    let mut role_length = valid_compiled_fixture();
    role_length.instruction_roles.pop();
    assert!(matches!(
        crate::compile_executable_program_artifact(&role_length, &catalog),
        Err(crate::ArtifactBuildError::CompiledMetadataLengthMismatch {
            table: "instruction_roles",
            ..
        })
    ));

    let mut kind_length = valid_compiled_fixture();
    kind_length.register_kinds.pop();
    assert!(matches!(
        crate::compile_executable_program_artifact(&kind_length, &catalog),
        Err(crate::ArtifactBuildError::CompiledMetadataLengthMismatch {
            table: "register_kinds",
            ..
        })
    ));

    let mut cardinality_length = valid_compiled_fixture();
    cardinality_length.register_collection_cardinalities.pop();
    assert!(matches!(
        crate::compile_executable_program_artifact(&cardinality_length, &catalog),
        Err(crate::ArtifactBuildError::CompiledMetadataLengthMismatch {
            table: "register_collection_cardinalities",
            ..
        })
    ));

    let mut mismatched_return = valid_compiled_fixture();
    let last = mismatched_return.program.instructions.len() - 1;
    mismatched_return.program.instructions[last] = BytecodeInstruction::Return { src: 0 };
    assert!(matches!(
        crate::compile_executable_program_artifact(&mismatched_return, &catalog),
        Err(crate::ArtifactBuildError::CompiledReturnRegisterMismatch {
            expected: 1,
            found: 0,
            ..
        })
    ));

    let mut nonterminal_return = valid_compiled_fixture();
    nonterminal_return.program.instructions.swap(2, 3);
    nonterminal_return.instruction_roles.swap(2, 3);
    assert!(matches!(
        crate::compile_executable_program_artifact(&nonterminal_return, &catalog),
        Err(crate::ArtifactBuildError::NonTerminalCompiledReturn { .. })
    ));

    let mut missing_destination_kind = valid_compiled_fixture();
    missing_destination_kind.register_kinds[1] = None;
    assert!(matches!(
        crate::compile_executable_program_artifact(&missing_destination_kind, &catalog),
        Err(crate::ArtifactBuildError::MissingRegisterKind {
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
        crate::compile_executable_program_artifact(&missing_source, &catalog),
        Err(crate::ArtifactBuildError::MissingRegisterSource {
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
        crate::compile_executable_program_artifact(&unknown_runtime, &catalog),
        Err(crate::ArtifactBuildError::UnknownRuntimeFunction { function: u64::MAX })
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
        crate::compile_executable_program_artifact(&requirement_mismatch, &catalog).unwrap_err();
    assert!(
        matches!(
            &mismatch_error,
            crate::ArtifactBuildError::ApplicationRequirementKindMismatch {
                requirement: 0,
                expected: "host function",
            }
        ),
        "unexpected requirement mismatch error: {mismatch_error:?}"
    );

    let mut missing_requirement = requirement_mismatch.clone();
    missing_requirement.program.requirements.clear();
    assert!(matches!(
        crate::compile_executable_program_artifact(&missing_requirement, &catalog),
        Err(crate::ArtifactBuildError::UnknownApplicationRequirement { requirement: 0 })
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
            root_visible: true,
            ordinal: 0,
        },
        CompiledSymbolDefinition {
            id: hash_str("state"),
            name: "state".to_owned(),
            register: 0,
            mutable: true,
            root_visible: true,
            ordinal: 1,
        },
    ];
    assert!(matches!(
        crate::compile_executable_program_artifact(&ambiguous_role, &catalog),
        Err(crate::ArtifactBuildError::AmbiguousRegisterRole { register: 0 })
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
    non_boolean_integrity.integrity_constraints = vec![CompiledIntegrityConstraint {
        name: "constraint-0".to_owned(),
        result_register: 0,
    }];
    assert!(matches!(
        crate::compile_executable_program_artifact(&non_boolean_integrity, &catalog),
        Err(crate::ArtifactBuildError::IntegrityConstraintSchemaMismatch { constraint: 0, .. })
    ));

    let mut missing_integrity_declaration = non_boolean_integrity.clone();
    missing_integrity_declaration.integrity_constraints.clear();
    assert!(matches!(
        crate::compile_executable_program_artifact(&missing_integrity_declaration, &catalog),
        Err(
            crate::ArtifactBuildError::IntegrityConstraintMetadataMismatch {
                marker_register: Some(0),
                declared_register: None,
                ..
            }
        )
    ));

    let mut wrong_integrity_register = non_boolean_integrity.clone();
    wrong_integrity_register.integrity_constraints = vec![CompiledIntegrityConstraint {
        name: "constraint-1".to_owned(),
        result_register: 1,
    }];
    assert!(matches!(
        crate::compile_executable_program_artifact(&wrong_integrity_register, &catalog),
        Err(
            crate::ArtifactBuildError::IntegrityConstraintMetadataMismatch {
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
        crate::compile_executable_program_artifact(&unresolved_nominal, &catalog),
        Err(crate::ArtifactBuildError::Semantic(
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
        crate::compile_executable_program_artifact(&compiled, &source_catalog()).unwrap();
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
