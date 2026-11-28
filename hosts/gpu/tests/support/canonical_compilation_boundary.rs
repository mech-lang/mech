use super::{artifacts, limits};
use mech_core::{
    BindingId, CellSlotId, DeclaredOperationContract, FunctionCatalog, InputId, IntegerWidth,
    ManagedMemoryBudget, NodeId, OperationContractTableBuilder, OutputId, ResolvedInputPort,
    ResolvedOperationContract, ResolvedOutputPort, SchemaBody, SchemaDraft, SchemaTableBuilder,
    Value, ValueData, ValueDataDraft,
    snapshot::{ConstantStoreBuilder, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{
    ArtifactSource, BindingDeclaration, ExecutableNodeBody, InputDeclaration, NodeDeclaration,
    OperationNodeBody, OperationReference, OutputDeclaration, ProducerReference, ProgramArtifact,
    ProgramArtifactDraft, SlotDeclaration, SlotRole, decode_program_artifact_bytecode_v1,
    decode_source_input_name, encode_program_artifact_bytecode_v1,
    native_artifact::NativeScalarKind,
};
use mech_gpu::native_artifact::{
    JitProgramArtifact, NativeIterationExecutionError, NativeProgramExecutionError,
    emit_program_object,
};

fn direct_artifact(catalog: &FunctionCatalog) -> ProgramArtifact {
    let mut schemas = SchemaTableBuilder::new();
    let schema = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::UnsignedInteger(IntegerWidth::W64),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    let schema = build.resolve(schema).unwrap();
    let (schemas, _) = build.into_parts();
    let (constants, _) = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts();

    let operations = [("math/add", [0, 1]), ("math/div", [3, 2])];
    let mut contracts = OperationContractTableBuilder::new();
    let contract_ids = operations.map(|(name, _)| {
        let declaration = catalog.source_operation_contract(name, 2, false).unwrap();
        contracts
            .insert(ResolvedOperationContract::Declared(
                DeclaredOperationContract {
                    inputs: declaration
                        .inputs
                        .resolve(2)
                        .unwrap()
                        .iter()
                        .map(|port| ResolvedInputPort {
                            schema,
                            access: port.access,
                            delivery: port.delivery,
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    outputs: declaration
                        .outputs
                        .iter()
                        .map(|port| ResolvedOutputPort {
                            schema,
                            access: port.access,
                            delivery: port.delivery,
                            construction: port.construction.clone(),
                            alias: port.alias,
                            change_detection: port.change_detection,
                        })
                        .collect::<Vec<_>>()
                        .into_boxed_slice(),
                    interaction: declaration.interaction.clone(),
                },
            ))
            .unwrap()
    });
    let build = contracts.finish().unwrap();
    let contract_ids = contract_ids.map(|contract| build.resolve(contract).unwrap());
    let (contracts, _) = build.into_parts();

    let inputs = ["left", "right", "divisor"]
        .into_iter()
        .enumerate()
        .map(|(index, name)| InputDeclaration {
            input: InputId::new(index as u32),
            name: name.into(),
            slot: CellSlotId::new(index as u32),
            schema,
        })
        .collect::<Vec<_>>();
    let mut slots = inputs
        .iter()
        .map(|input| SlotDeclaration {
            slot: input.slot,
            schema,
            role: SlotRole::Input,
            producer: ProducerReference::Input(input.input),
            initializer: None,
        })
        .collect::<Vec<_>>();
    let mut nodes = Vec::new();
    let mut bindings = Vec::new();
    for (index, ((name, operands), contract)) in
        operations.into_iter().zip(contract_ids).enumerate()
    {
        let index = index as u32;
        let node = NodeId::new(index);
        let target = CellSlotId::new(3 + index);
        let base = 3 * index;
        let (module, operation) = name.split_once('/').unwrap();
        nodes.push(NodeDeclaration {
            node,
            body: ExecutableNodeBody::Operation(OperationNodeBody {
                operation: OperationReference {
                    module_path: Box::new([module.to_owned()]),
                    operation_name: operation.into(),
                },
                contract,
                requirement: None,
            }),
            input_bindings: base..base + 2,
            output_bindings: base + 2..base + 3,
        });
        slots.push(SlotDeclaration {
            slot: target,
            schema,
            role: SlotRole::Derived,
            producer: ProducerReference::NodeOutput {
                node,
                output_ordinal: 0,
            },
            initializer: None,
        });
        bindings.extend(operands.into_iter().enumerate().map(|(port, input)| {
            BindingDeclaration::Input {
                id: BindingId::new(base + port as u32),
                node,
                port_ordinal: port as u16,
                source: ArtifactSource::Slot(CellSlotId::new(input)),
            }
        }));
        bindings.push(BindingDeclaration::Output {
            id: BindingId::new(base + 2),
            node,
            port_ordinal: 0,
            target,
        });
    }
    let output = OutputId::new(0);
    let source = CellSlotId::new(5);
    slots.push(SlotDeclaration {
        slot: source,
        schema,
        role: SlotRole::Output,
        producer: ProducerReference::Output {
            output,
            source: ArtifactSource::Slot(CellSlotId::new(4)),
        },
        initializer: None,
    });
    ProgramArtifactDraft {
        schemas,
        constants,
        contracts,
        requirements: Default::default(),
        inputs: inputs.into_boxed_slice(),
        slots: slots.into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        bindings: bindings.into_boxed_slice(),
        outputs: Box::new([OutputDeclaration {
            output,
            name: "result".into(),
            interactive_binding: None,
            source,
            schema,
        }]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap()
}

fn scalar(value: &Value) -> u64 {
    let ValueData::U64(word) = value.data() else {
        panic!("expected a U64 output");
    };
    *word
}

fn evaluate(artifact: &ProgramArtifact, catalog: &FunctionCatalog) -> [Result<u64, String>; 4] {
    let jit = JitProgramArtifact::compile(artifact, catalog).unwrap();
    let memory = ManagedMemoryBudget::new(256 * 1024 * 1024);
    let mut output = [None];
    let results = [[20, 22, 2], [u64::MAX, 1, 1], [20, 22, 0], [1, 3, 2]].map(|values| {
        let published = output[0].as_ref().map(scalar);
        let retained = memory.used_bytes();
        let args = artifact
            .inputs()
            .iter()
            .map(|input| {
                let name =
                    decode_source_input_name(&input.name).unwrap_or_else(|| input.name.clone());
                let word = match name.as_str() {
                    "left" => values[0],
                    "right" => values[1],
                    "divisor" => values[2],
                    _ => panic!("unexpected input {name}"),
                };
                let value = ValueDraft {
                    schema: input.schema,
                    shape_values: Box::new([]),
                    data: ValueDataDraft::U64(word),
                }
                .finalize(&SnapshotValidationContext::new(artifact.schemas()))
                .unwrap();
                let mut reservation = memory
                    .reserve_capacity(value.memory_budget_admission_bytes(&memory).unwrap())
                    .unwrap();
                value.into_memory_budget(&mut reservation).unwrap()
            })
            .collect::<Vec<_>>();
        let admitted = memory.used_bytes();
        match jit.evaluate_into(&args, &mut output, limits(), &memory) {
            Ok(_) => Ok(scalar(output[0].as_ref().unwrap())),
            Err(NativeProgramExecutionError::Control(
                NativeIterationExecutionError::Arithmetic { origin },
            )) => {
                assert_eq!(output[0].as_ref().map(scalar), published);
                assert_eq!(memory.used_bytes(), admitted);
                drop(args);
                assert_eq!(memory.used_bytes(), retained);
                let node = artifact
                    .nodes()
                    .iter()
                    .find(|node| node.node == origin.node())
                    .unwrap();
                let ExecutableNodeBody::Operation(operation) = &node.body else {
                    panic!("arithmetic denial did not identify a declared operation");
                };
                assert_eq!(&*operation.operation.module_path, ["math"]);
                Err(operation.operation.operation_name.clone())
            }
            Err(error) => panic!("unexpected native failure: {error:?}"),
        }
    });
    drop(output);
    assert_eq!(memory.used_bytes(), 0);
    results
}

fn check_object(artifact: &ProgramArtifact, catalog: &FunctionCatalog) {
    let object = emit_program_object(artifact, catalog).unwrap();
    let descriptor = object.descriptor();
    let plan = descriptor.plan();
    assert!(!object.bytes().is_empty());
    assert!(!object.native_metadata().is_empty());
    assert_eq!(plan.program_revision(), artifact.revision());
    assert_eq!(plan.transport_input_count(), 3);
    assert_eq!(plan.transport_output_count(), 1);
    assert!(plan.resource_requirements().is_empty());
    assert!(plan.observations().is_empty());
    assert!(plan.effects().is_empty());
    assert_eq!(plan.inputs().len(), artifact.inputs().len());
    for (port, declaration) in plan.inputs().iter().zip(artifact.inputs()) {
        assert_eq!(port.input(), declaration.input);
        assert_eq!(port.name(), declaration.name);
        assert_eq!(port.ty().schema(), declaration.schema);
        assert_eq!(port.ty().scalar_kind(), Some(NativeScalarKind::U64));
    }
    let [port] = plan.outputs() else {
        panic!("expected one native output");
    };
    let [declaration] = artifact.outputs() else {
        panic!("expected one declared output");
    };
    assert_eq!(port.output(), declaration.output);
    assert_eq!(port.name(), declaration.name);
    assert_eq!(port.ty().schema(), declaration.schema);
    assert_eq!(port.ty().scalar_kind(), Some(NativeScalarKind::U64));
}

#[test]
fn source_and_direct_artifacts_use_the_same_checked_boundary() {
    let (source, _) = artifacts("(left<u64> + right<u64>) / divisor<u64>\n");
    let catalog = mech_stdlib::source_catalog();
    let direct = direct_artifact(&catalog);
    let decoded =
        decode_program_artifact_bytecode_v1(&encode_program_artifact_bytecode_v1(&direct).unwrap())
            .unwrap();

    let expected = [Ok(21), Err("add".to_owned()), Err("div".to_owned()), Ok(2)];
    for (source, direct) in source.iter().zip([&direct, &decoded]) {
        let source_results = evaluate(source, &catalog);
        let direct_results = evaluate(direct, &catalog);
        assert_eq!(source_results, direct_results);
        assert_eq!(direct_results, expected);
        check_object(source, &catalog);
        check_object(direct, &catalog);
    }
}
