#![cfg(feature = "source")]

use mech_core::{BindingId, CellSlotId, ConstantId, InputId, NodeId, OutputId, SchemaBody};
use mech_engine::*;
use mech_syntax::document::parser::{canonical::parse_canonical_phase_2i_rule_for_test, rules};
use mech_syntax::document::{
    AstNode, DocumentId, ExpressionSyntax, ParseConfig, Revision, SyntaxKind, SyntaxNode,
    TextSnapshot,
};

fn find(node: SyntaxNode) -> Option<ExpressionSyntax> {
    if node.kind() == SyntaxKind::Expression {
        return ExpressionSyntax::cast(node);
    }
    node.children().find_map(find)
}

fn compile(source: &str) -> CanonicalSourceProgram {
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(0x4d41544348), Revision(1), source).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}; consumed {:?}",
        parsed.diagnostics,
        parsed.consumed
    );
    assert_eq!(parsed.consumed.end.0 as usize, source.len());
    CanonicalSourceFrontend
        .compile_expression(&find(parsed.syntax()).unwrap())
        .unwrap()
}

fn fixture() -> ProgramArtifactDraft {
    let base = compile("(flag<bool>, math/neg(11),22,true,false)")
        .compile_artifact()
        .unwrap();
    let boolean = base.inputs()[0].schema;
    let scalar = (0..base.schemas().len())
        .map(|id| mech_core::SchemaId::new(id as u32))
        .find(|id| {
            matches!(
                base.schemas().get(*id).unwrap().body(),
                SchemaBody::FloatingPoint(mech_core::FloatWidth::W64)
            )
        })
        .unwrap();
    let constants = (0..base.constants().len())
        .map(|id| ConstantId::new(id as u32))
        .filter(|id| base.constants().get(*id).unwrap().schema() == scalar)
        .collect::<Vec<_>>();
    assert_eq!(constants.len(), 2);
    let literal = |flag| {
        (0..base.constants().len()).map(|id| ConstantId::new(id as u32))
        .find(|id| matches!(base.constants().get(*id).unwrap().data(), mech_core::ValueData::Bool(value) if *value == flag)).unwrap()
    };
    let true_ = literal(true);
    let false_ = literal(false);
    let block = |id, constant| ControlBlock {
        id: ControlBlockId(id),
        parameters: Box::new([]),
        operations: Box::new([]),
        yield_value: ControlValue::Constant(constant),
    };
    ProgramArtifactDraft {
        schemas: base.schemas().clone(),
        constants: base.constants().clone(),
        contracts: base.contracts().clone(),
        requirements: ApplicationRequirementTable::default(),
        inputs: vec![InputDeclaration {
            input: InputId(0),
            name: "flag".into(),
            slot: CellSlotId(0),
            schema: boolean,
        }]
        .into_boxed_slice(),
        slots: vec![
            SlotDeclaration {
                slot: CellSlotId(0),
                schema: boolean,
                role: SlotRole::Input,
                producer: ProducerReference::Input(InputId(0)),
                initializer: None,
            },
            SlotDeclaration {
                slot: CellSlotId(1),
                schema: scalar,
                role: SlotRole::Derived,
                producer: ProducerReference::NodeOutput {
                    node: NodeId(0),
                    output_ordinal: 0,
                },
                initializer: None,
            },
            SlotDeclaration {
                slot: CellSlotId(2),
                schema: scalar,
                role: SlotRole::Output,
                producer: ProducerReference::Output {
                    output: OutputId(0),
                    source: ArtifactSource::Slot(CellSlotId(1)),
                },
                initializer: None,
            },
        ]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId(0),
            body: ExecutableNodeBody::Match(MatchDeclaration {
                scrutinee: 0,
                partial: false,
                captures: Box::new([]),
                arms: vec![
                    ControlMatchArm {
                        pattern: MatchPattern::Literal(true_),
                        guard: None,
                        body: block(0, constants[0]),
                    },
                    ControlMatchArm {
                        pattern: MatchPattern::Literal(false_),
                        guard: None,
                        body: block(1, constants[1]),
                    },
                ]
                .into_boxed_slice(),
            }),
            input_bindings: 0..1,
            output_bindings: 1..2,
        }]
        .into_boxed_slice(),
        bindings: vec![
            BindingDeclaration::Input {
                id: BindingId(0),
                node: NodeId(0),
                port_ordinal: 0,
                source: ArtifactSource::Slot(CellSlotId(0)),
            },
            BindingDeclaration::Output {
                id: BindingId(1),
                node: NodeId(0),
                port_ordinal: 0,
                target: CellSlotId(1),
            },
        ]
        .into_boxed_slice(),
        outputs: vec![OutputDeclaration {
            output: OutputId(0),
            name: "result".into(),
            interactive_binding: None,
            source: CellSlotId(2),
            schema: scalar,
        }]
        .into_boxed_slice(),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
}

fn control(draft: &mut ProgramArtifactDraft) -> &mut MatchDeclaration {
    let ExecutableNodeBody::Match(control) = &mut draft.nodes[0].body else {
        panic!()
    };
    control
}

fn draft_from(artifact: &ProgramArtifact) -> ProgramArtifactDraft {
    ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: artifact.requirements().clone(),
        inputs: artifact.inputs().into(),
        slots: artifact.slots().into(),
        nodes: artifact.nodes().into(),
        bindings: artifact.bindings().into(),
        outputs: artifact.outputs().into(),
        constraints: artifact.constraints().into(),
        compute_regions: artifact.compute_regions().into(),
    }
}

#[test]
fn typed_match_roundtrip_and_revision_own_all_branch_semantics() {
    let draft = fixture();
    let artifact = draft.clone().finalize().unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(artifact.revision(), decoded.revision());
    assert_eq!(artifact.nodes(), decoded.nodes());
    for change in 0..3 {
        let mut changed = draft.clone();
        let matched = control(&mut changed);
        match change {
            0 => {
                let first = matched.arms[0].pattern.clone();
                matched.arms[0].pattern = matched.arms[1].pattern.clone();
                matched.arms[1].pattern = first;
            }
            1 => {
                let value = matched.arms[0].body.yield_value;
                matched.arms[0].body.yield_value = matched.arms[1].body.yield_value;
                matched.arms[1].body.yield_value = value;
            }
            _ => matched.arms[1].pattern = MatchPattern::Wildcard,
        }
        assert_ne!(artifact.revision(), changed.finalize().unwrap().revision());
    }
}

#[test]
fn typed_match_rejects_invalid_scope_coverage_schema_and_writer() {
    for mutation in 0..10 {
        let mut draft = fixture();
        let boolean = draft.inputs[0].schema;
        match mutation {
            0 => {
                let first = control(&mut draft).arms[0].pattern.clone();
                control(&mut draft).arms[1].pattern = first;
            }
            1 => control(&mut draft).arms[0].body.id = ControlBlockId(9),
            2 => {
                control(&mut draft).arms[0].body.yield_value = ControlValue::Local {
                    block: ControlBlockId(1),
                    node: 0,
                }
            }
            3 => {
                control(&mut draft).arms[0].body.yield_value = ControlValue::Parameter {
                    block: ControlBlockId(0),
                    ordinal: 0,
                }
            }
            4 => control(&mut draft).scrutinee = 9,
            5 => draft.slots[1].schema = boolean,
            6 => draft.slots[1].role = SlotRole::State,
            8 => {
                control(&mut draft).arms[0].pattern =
                    MatchPattern::Literal(ConstantId::new(u32::MAX))
            }
            9 => {
                let value = control(&mut draft).arms[0].body.yield_value;
                let ControlValue::Constant(constant) = value else {
                    panic!()
                };
                control(&mut draft).arms[0].pattern = MatchPattern::Literal(constant);
            }
            _ => {
                let guard = control(&mut draft).arms[0].body.clone();
                control(&mut draft).arms[0].guard = Some(guard);
                control(&mut draft).arms[0].body.id = ControlBlockId(1);
                control(&mut draft).arms[1].body.id = ControlBlockId(2);
            }
        }
        assert!(draft.finalize().is_err(), "mutation {mutation}");
    }
}

#[test]
fn typed_match_rejects_scalar_schema_for_array_rest_binding() {
    let artifact = compile("[1 2 3] ? | [head | rest], flag<bool> => rest[1] + rest[2] | * => 0")
        .compile_artifact()
        .unwrap();
    let boolean = artifact.inputs()[0].schema;
    let mut draft = draft_from(&artifact);
    let matched = draft
        .nodes
        .iter_mut()
        .find_map(|node| match &mut node.body {
            ExecutableNodeBody::Match(matched) => Some(matched),
            _ => None,
        })
        .unwrap();
    let MatchPattern::Structural(CollectionPattern::Array {
        rest: Some(rest), ..
    }) = &mut matched.arms[0].pattern
    else {
        panic!("expected an array rest pattern")
    };
    let CollectionPattern::Bind { schema, .. } = rest.as_mut() else {
        panic!("expected the rest to bind")
    };
    *schema = boolean;
    assert!(draft.finalize().is_err());
}

#[test]
fn typed_match_codec_admits_exact_bounds_and_rejects_unknown_tags() {
    let artifact = fixture().finalize().unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    let exact = ArtifactDecodeLimits {
        max_nodes: 1,
        max_control_arms: 2,
        max_control_blocks: 2,
        max_control_operations: 0,
        max_control_operands: 0,
        ..ArtifactDecodeLimits::default()
    };
    assert_eq!(
        decode_program_artifact_sections_with_limits(&sections, exact)
            .unwrap()
            .revision(),
        artifact.revision()
    );
    for limits in [
        ArtifactDecodeLimits {
            max_control_arms: 1,
            ..exact
        },
        ArtifactDecodeLimits {
            max_control_blocks: 1,
            ..exact
        },
        ArtifactDecodeLimits {
            max_nodes: 0,
            ..exact
        },
    ] {
        assert!(decode_program_artifact_sections_with_limits(&sections, limits).is_err());
    }
    for key in ["revision", "pattern"] {
        let mut sections = sections.clone();
        let mut graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
        if key == "revision" {
            graph["revision"] = serde_json::json!(0);
        } else {
            let pattern = graph
                .pointer_mut("/nodes/0/body/Match/arms/0/pattern")
                .and_then(serde_json::Value::as_object_mut)
                .expect("fixture must encode the first match pattern as an object");
            let literal = pattern
                .remove("Literal")
                .expect("fixture must encode the first match pattern as Literal");
            assert!(pattern.insert("Unknown".to_owned(), literal).is_none());
        }
        sections.nodes = serde_json::to_vec(&graph).unwrap();
        assert!(
            decode_program_artifact_sections(&sections).is_err(),
            "{key}"
        );
    }
}

#[test]
fn every_local_operation_requires_its_exact_ordinary_contract() {
    let base = compile("math/neg(11)").compile_artifact().unwrap();
    let operation = base.nodes()[0].as_operation().unwrap().operation.clone();
    for mutation in 0..5 {
        let mut draft = fixture();
        let scalar = draft.slots[1].schema;
        let contract = draft
            .contracts
            .iter()
            .find(|contract| {
                matches!(contract,
            mech_core::ResolvedOperationContract::Declared(contract)
                if contract.inputs.len() == 1 && contract.inputs[0].schema == scalar
                    && contract.outputs.len() == 1 && contract.outputs[0].schema == scalar)
            })
            .unwrap()
            .clone();
        let mut contracts = mech_core::OperationContractTableBuilder::new();
        let handle = contracts.insert(contract).unwrap();
        let built = contracts.finish().unwrap();
        let id = built.resolve(handle).unwrap();
        draft.contracts = built.into_parts().0;
        let block = &mut control(&mut draft).arms[0].body;
        block.operations = vec![ControlOperation {
            node: 0,
            body: ControlOperationBody::Operation {
                operation: operation.clone(),
                contract: id,
            },
            inputs: vec![block.yield_value].into_boxed_slice(),
            schema: scalar,
        }]
        .into_boxed_slice();
        block.yield_value = ControlValue::Local {
            block: block.id,
            node: 0,
        };
        match mutation {
            0 => {}
            1 => {
                let ControlOperationBody::Operation { contract, .. } =
                    &mut block.operations[0].body
                else {
                    unreachable!()
                };
                *contract = mech_core::OperationContractId::new(u32::MAX);
            }
            2 => block.operations[0].node = 1,
            3 => block.operations[0].inputs = Box::new([]),
            _ => block.operations[0].inputs[0] = block.yield_value,
        }
        let result = draft.finalize();
        if mutation == 0 {
            result.unwrap();
        } else {
            assert!(result.is_err(), "mutation {mutation}");
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn resident_typed_match_switches_true_false_true_without_eager_blocks() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let artifact = fixture().finalize().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d, 1),
        &decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap(),
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    assert!(!instance.plan.has_only_kernel_steps());
    let matched = match &artifact.nodes()[0].body {
        ExecutableNodeBody::Match(matched) => matched,
        _ => panic!(),
    };
    let expected = matched
        .arms
        .iter()
        .map(|arm| match arm.body.yield_value {
            ControlValue::Constant(id) => match artifact.constants().get(id).unwrap().data() {
                ValueData::F64(value) => value.to_f64(),
                _ => panic!(),
            },
            _ => panic!(),
        })
        .collect::<Vec<_>>();
    let input = instance.plan.inputs[0].clone();
    for (index, flag) in [1u8, 0, 1].into_iter().enumerate() {
        instance
            .turn(&[CapturedSignalInput {
                slot: input.slot,
                value: ResidentValueRef::Bool(&[flag]),
            }])
            .unwrap();
        let output = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = output.data() else {
            panic!()
        };
        assert_eq!(value.to_f64(), expected[usize::from(index == 1)]);
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn structural_match_falls_through_for_foreign_dynamic_payloads() {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_core::{
        DimensionExpr, FloatWidth, FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef,
        SchemaDraft, SchemaTableBuilder, ValueData,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};

    let artifact = compile("signal<[*]:1,1> ? | [true] => 1 | * => 0")
        .compile_artifact()
        .unwrap();
    let mut foreign = SchemaTableBuilder::new();
    let tuple = foreign
        .insert(
            SchemaDraft {
                body: SchemaBody::Tuple(
                    vec![SchemaBody::FloatingPoint(FloatWidth::W64), SchemaBody::Bool]
                        .into_boxed_slice(),
                ),
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let matrix = foreign
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Dynamic),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                        .into_boxed_slice(),
                },
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let foreign = foreign.finish().unwrap();
    let tuple = foreign.resolve(tuple).unwrap();
    let matrix = foreign.resolve(matrix).unwrap();
    let (foreign, _) = foreign.into_parts();
    assert!(
        artifact
            .schemas()
            .find_by_key(foreign.entry(tuple).unwrap().key())
            .is_none()
    );
    let input = ValueDraft {
        schema: matrix,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                schema: tuple,
                shape_values: Box::new([]),
                data: ValueDataDraft::Tuple(
                    vec![
                        ValueDataDraft::F64(F64Bits::from_f64(7.0)),
                        ValueDataDraft::Bool(false),
                    ]
                    .into_boxed_slice(),
                ),
            })))]
            .into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&foreign))
    .unwrap();

    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(0x4d, 3),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    instance
        .turn(&[CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::Snapshot(&[Some(input)]),
        }])
        .unwrap();
    let output = instance.copied_output(0).unwrap();
    assert!(matches!(output.data(), ValueData::F64(value) if value.to_f64() == 0.0));
}

#[cfg(feature = "resident-artifact")]
#[test]
fn nested_dynamic_rest_binding_retains_inherited_shape_after_roundtrip() {
    use mech_core::snapshot::{F64Bits, SnapshotValidationContext, ValueDataDraft, ValueDraft};
    use mech_core::{
        DimensionExpr, FloatWidth, FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef,
        SchemaDraft, SchemaTableBuilder, ValueData,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};

    let artifact = compile("signal<[*]:1,3> ? | [* | [x, *]] => 1 | * => 0")
        .compile_artifact()
        .unwrap();
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut foreign = SchemaTableBuilder::new();
    let scalar = foreign
        .insert(
            SchemaDraft {
                body: SchemaBody::FloatingPoint(FloatWidth::W64),
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let matrix = foreign
        .insert(
            SchemaDraft {
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Dynamic),
                    dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                        .into_boxed_slice(),
                },
                dimension_parameters: Box::new([]),
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let foreign = foreign.finish().unwrap();
    let scalar = foreign.resolve(scalar).unwrap();
    let matrix = foreign.resolve(matrix).unwrap();
    let (foreign, _) = foreign.into_parts();
    let input = ValueDraft {
        schema: matrix,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            [7.0, 8.0, 9.0]
                .map(|value| {
                    ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
                        schema: scalar,
                        shape_values: Box::new([]),
                        data: ValueDataDraft::F64(F64Bits::from_f64(value)),
                    })))
                })
                .into(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(&foreign))
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [&artifact, &decoded] {
        let mut instance = activate(
            ReactiveInstanceId::new(0x4d, 4),
            artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for _ in 0..2 {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::Snapshot(&[Some(input.clone())]),
                }])
                .expect("the nested Dynamic binding inherits the selected rest shape");
            let output = instance.copied_output(0).unwrap();
            assert!(matches!(output.data(), ValueData::F64(value) if value.to_f64() == 1.0));
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn canonical_match_blocks_roundtrip_and_execute_captures_binding_and_guards() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for source in [
        "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2",
        "flag<bool> ? | x, x => signal<f64> + 1 | * => signal<f64> + 2",
    ] {
        let compiled = compile(source);
        assert_eq!(compiled.program().inputs.len(), 2);
        assert_eq!(
            compiled.program().nodes.len(),
            1,
            "arm operations must be block-owned"
        );
        assert!(matches!(
            compiled.program().nodes[0].body,
            SourceNodeBody::Match(_)
        ));
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for artifact in [&artifact, &decoded] {
            let mut instance = activate(
                ReactiveInstanceId::new(0x4d, 2),
                artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let input_slots = instance
                .plan
                .inputs
                .iter()
                .map(|input| input.slot)
                .collect::<Vec<_>>();
            for (flag, signal, expected) in [(1u8, 10.0, 11.0), (0, 20.0, 22.0), (1, 30.0, 31.0)] {
                instance
                    .turn(&[
                        CapturedSignalInput {
                            slot: input_slots[0],
                            value: ResidentValueRef::Bool(&[flag]),
                        },
                        CapturedSignalInput {
                            slot: input_slots[1],
                            value: ResidentValueRef::F64(&[signal]),
                        },
                    ])
                    .unwrap();
                let output = instance.copied_output(0).unwrap();
                let ValueData::F64(value) = output.data() else {
                    panic!()
                };
                assert_eq!(value.to_f64(), expected, "{source}");
            }
        }
    }
}

#[cfg(feature = "resident-artifact")]
mod lazy_execution {
    use super::*;
    use mech_core::{
        BoundResidentKernel, FunctionCatalogBuilder, ManagedMemoryBudget, ReactiveInstanceId,
        ResidentKernelError, ResidentKernelInputs, ResidentValueMut, ResidentValueRef, ValueData,
    };
    use mech_engine::resident::{
        ActivatedTurnStep, ActivationFacts, CapturedSignalInput, ResidentActivationOptions,
        activate, activate_with_options,
    };
    use std::sync::atomic::{AtomicUsize, Ordering};
    static FAILING_CALLS: AtomicUsize = AtomicUsize::new(0);

    fn partial_failure(
        _: &BoundResidentKernel,
        _: &dyn ResidentKernelInputs,
        output: ResidentValueMut<'_>,
    ) -> Result<bool, ResidentKernelError> {
        FAILING_CALLS.fetch_add(1, Ordering::SeqCst);
        let ResidentValueMut::F64(output) = output else {
            return Err(ResidentKernelError::InvalidOutput);
        };
        output[0] = 999.0;
        Err(ResidentKernelError::Arithmetic)
    }

    fn stateful_artifact(guarded: bool) -> ProgramArtifact {
        let compiled = compile(if guarded {
            "flag<bool> ? | x, x => signal<f64> + 1 | * => signal<f64> + 2"
        } else {
            "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2"
        });
        let mut graph = compiled.program().clone();
        let schema = graph.outputs[0].schema;
        let initial = (0..compiled.constants().len())
            .map(|id| ConstantId::new(id as u32))
            .find(|id| compiled.constants().get(*id).unwrap().schema() == schema)
            .unwrap();
        graph.states = vec![SourceState {
            schema,
            initializer: Some(SourceValue::Constant(initial)),
            producer_node: 1,
            producer_output_ordinal: 0,
        }]
        .into_boxed_slice();
        let mut nodes = graph.nodes.into_vec();
        nodes.push(SourceNode {
            body: SourceNodeBody::Operation {
                operation: OperationReference {
                    module_path: vec!["core".to_owned()].into_boxed_slice(),
                    operation_name: "assign".to_owned(),
                },
                requirement: None,
            },
            inputs: vec![SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            }]
            .into_boxed_slice(),
            outputs: vec![SourceNodeOutput::State(0)].into_boxed_slice(),
        });
        graph.nodes = nodes.into_boxed_slice();
        graph.outputs[0].source = SourceValue::State(0);
        let assign = mech_core::maintained_operation_contract("core/assign", 1, false).unwrap();
        compile_source_program_with_control_contracts(
            &graph,
            &mut ArtifactBuildContext::new(compiled.schemas(), compiled.constants()),
            &[None, Some(&assign)],
        )
        .unwrap()
    }

    fn output(instance: &mech_engine::resident::ReactiveInstance) -> f64 {
        let value = instance.copied_output(0).unwrap();
        let ValueData::F64(value) = value.data() else {
            panic!()
        };
        value.to_f64()
    }

    fn state(instance: &mech_engine::resident::ReactiveInstance, slot: CellSlotId) -> f64 {
        let mech_engine::resident::ResidentValueBorrow::F64 {
            values: [value],
            shape,
        } = instance.state_borrow(slot).unwrap()
        else {
            panic!()
        };
        assert_eq!(shape, mech_core::ResidentShape::SCALAR);
        *value
    }

    #[test]
    fn inactive_failure_guard_laziness_selected_failure_and_abort_preserve_state() {
        for guarded in [false, true] {
            FAILING_CALLS.store(0, Ordering::SeqCst);
            let mut catalog = FunctionCatalogBuilder::new();
            install_intrinsic_resident(&mut catalog).unwrap();
            let catalog = catalog.build().unwrap();
            let artifact = stateful_artifact(guarded);
            let artifact = decode_program_artifact_bytecode_v1(
                &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
            )
            .unwrap();
            let mut instance = activate(
                ReactiveInstanceId::new(0x4d, 3),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap();
            let slots = instance
                .plan
                .inputs
                .iter()
                .map(|input| input.slot)
                .collect::<Vec<_>>();
            let selected = instance
                .plan
                .steps
                .iter()
                .find_map(|step| match step {
                    ActivatedTurnStep::Match(matched) => {
                        Some(matched.arms[0].body.steps[0].node.get() as usize)
                    }
                    _ => None,
                })
                .unwrap();
            let original = instance.plan.replace_kernel_for_test(
                selected,
                BoundResidentKernel::new(partial_failure, Box::new([])),
            );
            let signal = [10.0];
            let capture = |flag| {
                [
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::Bool(flag),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::F64(&signal),
                    },
                ]
            };
            instance.turn(&capture(&[0])).unwrap();
            assert_eq!(
                FAILING_CALLS.load(Ordering::SeqCst),
                0,
                "a false guard must not execute its body"
            );
            assert_eq!(output(&instance), 12.0);
            let epoch = instance.published_epoch();
            let state_slot = instance
                .plan
                .slots
                .iter()
                .find(|slot| slot.role == SlotRole::State)
                .unwrap()
                .artifact_id;
            let prior_state = state(&instance, state_slot);
            assert!(instance.turn(&capture(&[1])).is_err());
            assert_eq!(FAILING_CALLS.load(Ordering::SeqCst), 1);
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(output(&instance), 12.0);
            assert_eq!(state(&instance, state_slot), prior_state);
            instance.plan.replace_kernel_for_test(selected, original);
            instance.prepare_turn(&capture(&[1])).unwrap().abort();
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(output(&instance), 12.0);
            assert_eq!(state(&instance, state_slot), prior_state);
            instance.turn(&capture(&[1])).unwrap();
            assert_eq!(output(&instance), 11.0);
            instance.turn_without_summary(&capture(&[0])).unwrap();
            assert_eq!(output(&instance), 12.0);
        }
    }

    #[test]
    fn exact_backing_budget_admits_both_arms_and_one_below_rejects() {
        let mut catalog = FunctionCatalogBuilder::new();
        install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let artifact = stateful_artifact(true);
        let activate_budget = |budget: &ManagedMemoryBudget| {
            activate_with_options(
                ReactiveInstanceId::new(0x4d, 4),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
                ResidentActivationOptions {
                    memory_budget: Some(budget.clone()),
                    ..ResidentActivationOptions::default()
                },
            )
        };
        let mut lower = 0;
        let mut upper = 1u64;
        while activate_budget(&ManagedMemoryBudget::new(upper)).is_err() {
            lower = upper;
            upper = upper.checked_mul(2).unwrap();
        }
        while upper - lower > 1 {
            let middle = lower + (upper - lower) / 2;
            if activate_budget(&ManagedMemoryBudget::new(middle)).is_ok() {
                upper = middle;
            } else {
                lower = middle;
            }
        }
        let under = ManagedMemoryBudget::new(upper - 1);
        assert!(activate_budget(&under).is_err());
        assert_eq!(under.used_bytes(), 0);
        let exact = ManagedMemoryBudget::new(upper);
        let mut instance = activate_budget(&exact).unwrap();
        let retained = exact.used_bytes();
        let slots = instance
            .plan
            .inputs
            .iter()
            .map(|input| input.slot)
            .collect::<Vec<_>>();
        for turn in 0..100 {
            instance
                .turn(&[
                    CapturedSignalInput {
                        slot: slots[0],
                        value: ResidentValueRef::Bool(&[(turn % 2) as u8]),
                    },
                    CapturedSignalInput {
                        slot: slots[1],
                        value: ResidentValueRef::F64(&[10.0]),
                    },
                ])
                .unwrap();
            assert_eq!(exact.used_bytes(), retained);
            let Some(mech_engine::resident::ResidentValueBorrow::F64 {
                values: [value],
                shape,
            }) = instance.output_borrow(0)
            else {
                panic!("native scalar output");
            };
            assert_eq!(shape, mech_core::ResidentShape::SCALAR);
            assert_eq!(*value, if turn % 2 == 0 { 12.0 } else { 11.0 });
        }
        drop(instance);
        assert_eq!(exact.used_bytes(), 0);
    }
}

#[test]
fn source_only_match_has_typed_capture_and_binding_authority() {
    for source in [
        "flag<bool> ? | true => signal<f64> + 1 | false => signal<f64> + 2",
        "flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1",
        "flag<bool> ? | x => x",
    ] {
        let compiled = compile(source);
        assert_eq!(compiled.program().nodes.len(), 1);
        assert_eq!(compiled.contracts(), &[None]);
        let SourceNodeBody::Match(control) = &compiled.program().nodes[0].body else {
            panic!()
        };
        assert!(!control.arms.is_empty());
        assert!(
            compiled
                .source_map()
                .nodes
                .iter()
                .all(|node| node.operation != "source/match" && node.operation != "source/bind")
        );
        let artifact = compiled.compile_artifact().unwrap();
        let decoded = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        assert_eq!(artifact.revision(), decoded.revision());
        assert_eq!(artifact.nodes(), decoded.nodes());
    }
}

#[test]
fn captured_source_identity_and_guard_changes_change_revision() {
    let original = compile("flag<bool> ? | x, x => left<f64> + 1 | * => right<f64> + 2")
        .compile_artifact()
        .unwrap();
    for source in [
        "flag<bool> ? | x, !x => left<f64> + 1 | * => right<f64> + 2",
        "flag<bool> ? | x, x => right<f64> + 1 | * => left<f64> + 2",
    ] {
        assert_ne!(
            original.revision(),
            compile(source).compile_artifact().unwrap().revision()
        );
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn portable_scalar_match_target_executes_snapshot_backed_literals() {
    let compiled = compile("flag<u8> ? | 1u8 => 1u8 | * => 2u8");
    let artifact = compiled.compile_artifact().unwrap();
    let encoded = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    assert_eq!(
        artifact
            .schemas()
            .get(artifact.outputs()[0].schema)
            .unwrap()
            .body(),
        &SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8)
    );
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for artifact in [
        artifact,
        decode_program_artifact_bytecode_v1(&encoded).unwrap(),
    ] {
        mech_engine::resident::preflight_resident_target(
            &artifact,
            &catalog,
            &mech_engine::resident::ActivationFacts::default(),
            mech_engine::resident::ResidentActivationOptions::default(),
        )
        .unwrap();
        let schema = artifact.inputs()[0].schema;
        let mut instance = mech_engine::resident::activate(
            mech_core::ReactiveInstanceId::new(0x4d415443, 8),
            &artifact,
            &catalog,
            &mech_engine::resident::ActivationFacts::default(),
        )
        .unwrap();
        for (input, expected) in [(1, 1), (2, 2), (1, 1)] {
            let input = mech_core::ValueDraft {
                schema,
                shape_values: Box::new([]),
                data: mech_core::ValueDataDraft::U8(input),
            }
            .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
                artifact.schemas(),
            ))
            .unwrap();
            instance
                .turn(&[mech_engine::resident::CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: mech_core::ResidentValueRef::Snapshot(&[Some(input)]),
                }])
                .unwrap();
            assert!(
                matches!(instance.copied_output(0).unwrap().data(), mech_core::ValueData::U8(actual) if *actual == expected)
            );
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn local_operations_have_capability_witnesses_and_report_the_enclosing_owner() {
    use mech_engine::resident::{
        ActivationFacts, ResidentActivationOptions, preflight_resident_target,
    };
    let artifact = compile("flag<bool> ? | x, !x => signal<f64> + 2 | * => signal<f64> + 1")
        .compile_artifact()
        .unwrap();
    let empty = mech_core::FunctionCatalogBuilder::new().build().unwrap();
    let error = preflight_resident_target(
        &artifact,
        &empty,
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .err()
    .expect("expected rejection");
    assert_eq!(
        error.node,
        Some(NodeId(0)),
        "local physical IDs must not escape as source owners"
    );
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let witness = preflight_resident_target(
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .unwrap();
    assert_eq!(witness.concrete_cases.len(), 3);
    assert!(
        witness
            .concrete_cases
            .iter()
            .all(|case| case.node == NodeId(0))
    );
    assert_eq!(
        witness
            .concrete_cases
            .iter()
            .filter(|case| case.operation.canonical_name() == "math/add")
            .count(),
        2
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn scalar_layouts_and_guard_operations_match_fresh_instances() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentActivationOptions, ResidentIntegrityMode,
        ResidentValueBorrow, activate_with_options,
    };
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (kind, source) in [
        (0, "flag<bool> ? | x, !x => false | * => true"),
        (1, "flag<bool> ? | x, !x => left<index> | * => right<index>"),
        (2, "flag<bool> ? | x, !x => 2.0 | * => 1.0"),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        for integrity in [
            ResidentIntegrityMode::Checked,
            ResidentIntegrityMode::Unchecked,
        ] {
            let activate = || {
                activate_with_options(
                    mech_core::ReactiveInstanceId::new(822, 5),
                    &artifact,
                    &catalog,
                    &ActivationFacts::default(),
                    ResidentActivationOptions {
                        integrity,
                        ..Default::default()
                    },
                )
                .unwrap()
            };
            let mut instance = activate();
            for flag in [1u8, 0, 1] {
                let mut fresh = activate();
                for instance in [&mut instance, &mut fresh] {
                    let flag_values = [flag];
                    let inputs = instance
                        .plan
                        .inputs
                        .iter()
                        .enumerate()
                        .map(|(ordinal, input)| CapturedSignalInput {
                            slot: input.slot,
                            value: match ordinal {
                                0 => mech_core::ResidentValueRef::Bool(&flag_values),
                                1 => mech_core::ResidentValueRef::Index(&[2]),
                                2 => mech_core::ResidentValueRef::Index(&[1]),
                                _ => panic!("input layout"),
                            },
                        })
                        .collect::<Vec<_>>();
                    instance.turn(&inputs).unwrap();
                    match (kind, instance.output_borrow(0).unwrap()) {
                        (
                            0,
                            ResidentValueBorrow::Bool {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, flag),
                        (
                            1,
                            ResidentValueBorrow::Index {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, if flag == 0 { 2 } else { 1 }),
                        (
                            2,
                            ResidentValueBorrow::F64 {
                                values: [actual], ..
                            },
                        ) => assert_eq!(*actual, if flag == 0 { 2.0 } else { 1.0 }),
                        _ => panic!("exact native scalar layout"),
                    }
                }
            }
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn match_integrity_failure_preserves_state_and_published_epoch() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentValueBorrow, activate,
    };
    let compiled = compile("flag<bool> ? | true => true | false => false");
    let mut graph = compiled.program().clone();
    let schema = graph.outputs[0].schema;
    let initial = (0..compiled.constants().len())
        .map(|id| ConstantId::new(id as u32))
        .find(|id| {
            matches!(
                compiled.constants().get(*id).unwrap().data(),
                mech_core::ValueData::Bool(true)
            )
        })
        .unwrap();
    graph.states = vec![SourceState {
        schema,
        initializer: Some(SourceValue::Constant(initial)),
        producer_node: 1,
        producer_output_ordinal: 0,
    }]
    .into_boxed_slice();
    let mut nodes = graph.nodes.into_vec();
    nodes.push(SourceNode {
        body: SourceNodeBody::Operation {
            operation: OperationReference {
                module_path: vec!["core".to_owned()].into_boxed_slice(),
                operation_name: "assign".to_owned(),
            },
            requirement: None,
        },
        inputs: vec![SourceValue::NodeOutput {
            node: 0,
            output_ordinal: 0,
        }]
        .into_boxed_slice(),
        outputs: vec![SourceNodeOutput::State(0)].into_boxed_slice(),
    });
    graph.nodes = nodes.into_boxed_slice();
    graph.outputs[0].source = SourceValue::State(0);
    graph.constraints = vec![SourceIntegrityConstraint {
        name: "selected result must hold".to_owned(),
        operation: OperationReference {
            module_path: vec!["integrity".to_owned()].into_boxed_slice(),
            operation_name: "assert".to_owned(),
        },
        inputs: vec![SourceValue::State(0)].into_boxed_slice(),
    }]
    .into_boxed_slice();
    let assign = mech_core::maintained_operation_contract("core/assign", 1, false).unwrap();
    let artifact = compile_source_program_with_control_contracts(
        &graph,
        &mut ArtifactBuildContext::new(compiled.schemas(), compiled.constants()),
        &[None, Some(&assign)],
    )
    .unwrap();
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(822, 6),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let input = instance.plan.inputs[0].slot;
    let state = instance
        .plan
        .slots
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .unwrap()
        .artifact_id;
    instance
        .turn(&[CapturedSignalInput {
            slot: input,
            value: mech_core::ResidentValueRef::Bool(&[1]),
        }])
        .unwrap();
    let epoch = instance.published_epoch();
    assert!(
        instance
            .turn(&[CapturedSignalInput {
                slot: input,
                value: mech_core::ResidentValueRef::Bool(&[0])
            }])
            .is_err()
    );
    assert_eq!(instance.published_epoch(), epoch);
    assert!(matches!(
        instance.output_borrow(0),
        Some(ResidentValueBorrow::Bool { values: [1], .. })
    ));
    assert!(matches!(
        instance.state_borrow(state),
        Some(ResidentValueBorrow::Bool { values: [1], .. })
    ));
    instance
        .turn(&[CapturedSignalInput {
            slot: input,
            value: mech_core::ResidentValueRef::Bool(&[1]),
        }])
        .unwrap();
}

#[test]
fn unfinished_control_forms_are_explicit_source_errors() {
    for source in ["x<*> ? | y => y | * => 0"] {
        let parsed = parse_canonical_phase_2i_rule_for_test(
            TextSnapshot::new(DocumentId(822), Revision(1), source).unwrap(),
            rules::EXPRESSION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean());
        assert_eq!(parsed.consumed.end.0 as usize, source.len());
        let error = CanonicalSourceFrontend
            .compile_expression(&find(parsed.syntax()).unwrap())
            .err()
            .expect("expected rejection");
        assert_eq!(error.code, "source-semantics/unsupported-match");
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn long_blocks_keep_local_slots_outside_the_artifact_schedule() {
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentValueBorrow, activate,
    };
    let source = format!(
        "flag<bool> ? | true => signal<f64>{} | false => 0",
        " + 1".repeat(80)
    );
    let compiled = compile(&source);
    assert_eq!(compiled.program().nodes.len(), 1);
    let artifact = compiled.compile_artifact().unwrap();
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        mech_core::ReactiveInstanceId::new(822, 7),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    assert_eq!(instance.plan.steps.len(), 81);
    let slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| input.slot)
        .collect::<Vec<_>>();
    for flag in [1u8, 0, 1] {
        instance
            .turn(&[
                CapturedSignalInput {
                    slot: slots[0],
                    value: mech_core::ResidentValueRef::Bool(&[flag]),
                },
                CapturedSignalInput {
                    slot: slots[1],
                    value: mech_core::ResidentValueRef::F64(&[5.0]),
                },
            ])
            .unwrap();
        let Some(ResidentValueBorrow::F64 {
            values: [actual], ..
        }) = instance.output_borrow(0)
        else {
            panic!("scalar result");
        };
        assert_eq!(*actual, if flag == 1 { 85.0 } else { 0.0 });
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn numeric_match_roundtrips_and_uses_literal_binding_guard_and_wildcard_on_each_turn() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let compiled = compile(
        "signal<f64> ? | -1 => 20 | -(3) => 22 | -1.0e3 => 30 | -128<f64> => 40 | 0 => 10 | item, item > 0 => item + 1 | * => -1",
    );
    let artifact = compiled.compile_artifact().unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(822, 44),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    for (input, expected) in [
        (0.0, 10.0),
        (3.0, 4.0),
        (-4.0, -1.0),
        (-2.0, -1.0),
        (-3.0, 22.0),
        (-1.0, 20.0),
        (-1000.0, 30.0),
        (-128.0, 40.0),
        (-0.0, 10.0),
        (8.0, 9.0),
    ] {
        instance
            .turn(&[CapturedSignalInput {
                slot: instance.plan.inputs[0].slot,
                value: ResidentValueRef::F64(&[input]),
            }])
            .unwrap();
        let value = instance.copied_output(0).unwrap();
        let ValueData::F64(actual) = value.data() else {
            panic!("{value:?}")
        };
        assert_eq!(actual.to_f64(), expected, "input {input}");
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn compound_match_results_reuse_managed_construction_across_branch_switches() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let f = |value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value));
    for (source, live) in [
        ("signal<f64> ? | 0 => (1, true) | * => (2, false)", false),
        (
            "signal<f64> ? | 0 => (signal + 1, true) | * => (signal + 2, false)",
            true,
        ),
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        install_intrinsic_resident(&mut catalog).unwrap();
        let catalog = catalog.build().unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 80),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for input in [0.0, 4.0, 0.0] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[input]),
                }])
                .unwrap();
            let selected = input == 0.0;
            let number = if selected { 1.0 } else { 2.0 } + if live { input } else { 0.0 };
            assert_eq!(
                instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                ValueDataDraft::Tuple(
                    vec![f(number), ValueDataDraft::Bool(selected)].into_boxed_slice()
                ),
                "{source}: {input}",
            );
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn compound_match_keeps_failing_siblings_lazy_and_publication_atomic() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let artifact =
        compile("(values<[f64]:1,2>, flag<bool> ? | true => (1, true) | false => (values[index<f64>], false))")
            .compile_artifact()
            .unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(822, 81),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let values = instance.plan.inputs[0].slot;
    let flag = instance.plan.inputs[1].slot;
    let index = instance.plan.inputs[2].slot;
    let inputs = |flag_value, index_value| {
        [
            CapturedSignalInput {
                slot: values,
                value: ResidentValueRef::F64(&[1.0, 2.0]),
            },
            CapturedSignalInput {
                slot: flag,
                value: ResidentValueRef::Bool(flag_value),
            },
            CapturedSignalInput {
                slot: index,
                value: ResidentValueRef::F64(index_value),
            },
        ]
    };
    let expected = |value, flag| {
        let f = |value| ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value));
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::Matrix(vec![f(1.0), f(2.0)].into_boxed_slice()),
                ValueDataDraft::Tuple(
                    vec![f(value), ValueDataDraft::Bool(flag)].into_boxed_slice(),
                ),
            ]
            .into_boxed_slice(),
        )
    };
    // An out-of-bounds access in the unselected branch must not execute.
    instance.turn(&inputs(&[1], &[3.0])).unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        expected(1.0, true)
    );
    instance.turn(&inputs(&[0], &[1.0])).unwrap();
    let epoch = instance.published_epoch();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        expected(1.0, false)
    );
    assert!(instance.turn(&inputs(&[0], &[3.0])).is_err());
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        expected(1.0, false)
    );
    instance
        .prepare_turn(&inputs(&[1], &[3.0]))
        .unwrap()
        .abort();
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        expected(1.0, false)
    );
    instance.turn(&inputs(&[0], &[2.0])).unwrap();
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        expected(2.0, false)
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn closed_match_values_use_existing_matrix_record_string_and_snapshot_layouts() {
    use mech_core::snapshot::NamedValueDraft;
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let f = |value| Data::F64(mech_core::snapshot::F64Bits::from_f64(value));
    let record = |value, selected| {
        Data::Record(
            vec![
                NamedValueDraft {
                    name: "a".to_owned(),
                    value: f(value),
                },
                NamedValueDraft {
                    name: "b".to_owned(),
                    value: Data::Bool(selected),
                },
            ]
            .into_boxed_slice(),
        )
    };
    let cases = [
        (
            "signal<f64> ? | 0 => [1 2] | * => [3 4]",
            Data::Matrix(vec![f(1.0), f(2.0)].into_boxed_slice()),
            Data::Matrix(vec![f(3.0), f(4.0)].into_boxed_slice()),
        ),
        (
            "signal<f64> ? | 0 => {a: 1, b: true} | * => {a: signal, b: false}",
            record(1.0, true),
            record(4.0, false),
        ),
        (
            "signal<f64> ? | 0 => 1u8 | * => 2u8",
            Data::U8(1),
            Data::U8(2),
        ),
        (
            "(signal<f64>, true) ? | item => item",
            Data::Tuple(vec![f(0.0), Data::Bool(true)].into_boxed_slice()),
            Data::Tuple(vec![f(4.0), Data::Bool(true)].into_boxed_slice()),
        ),
        (
            "signal<f64> ? | 0 => \"first\" | * => \"second\"",
            Data::String("first".to_owned()),
            Data::String("second".to_owned()),
        ),
    ];
    for (source, first, second) in cases {
        let artifact = compile(source)
            .compile_artifact()
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
        let artifact = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut catalog = FunctionCatalogBuilder::new();
        install_intrinsic_resident(&mut catalog).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 82),
            &artifact,
            &catalog.build().unwrap(),
            &ActivationFacts::default(),
        )
        .unwrap();
        for (input, expected) in [(0.0, &first), (4.0, &second), (0.0, &first)] {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(&[input]),
                }])
                .unwrap();
            assert_eq!(
                &instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn nested_matches_compose_captures_guards_and_compound_results() {
    use mech_core::{
        FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueDataDraft as Data,
    };
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let f = |value| Data::F64(mech_core::snapshot::F64Bits::from_f64(value));
    let tuple = |value, flag| Data::Tuple(vec![f(value), Data::Bool(flag)].into_boxed_slice());
    let cases = [
        (
            "signal<f64> ? | item => ((item + 1) ? | inner => (inner + item, true))",
            tuple(1.0, true),
            tuple(9.0, true),
        ),
        (
            "signal<f64> ? | item, (item ? | 0 => true | * => false) => (1, true) | * => (2, false)",
            tuple(1.0, true),
            tuple(2.0, false),
        ),
        (
            "signal<f64> ? | 0 => (signal ? | 0 => (1, true) | * => (2, false)) | * => (signal ? | 4 => (3, true) | * => (4, false))",
            tuple(1.0, true),
            tuple(3.0, true),
        ),
    ];
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (source, zero, four) in cases {
        let artifact = compile(source).compile_artifact().unwrap();
        let artifact = decode_program_artifact_bytecode_v1(
            &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
        )
        .unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 91),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        let slot = instance.plan.inputs[0].slot;
        for (input, expected) in [(0.0, &zero), (4.0, &four), (0.0, &zero)] {
            instance
                .turn(&[CapturedSignalInput {
                    slot,
                    value: ResidentValueRef::F64(&[input]),
                }])
                .unwrap();
            assert_eq!(
                &instance
                    .copied_output(0)
                    .unwrap()
                    .canonical_data_draft()
                    .unwrap(),
                expected,
                "{source}"
            );
        }
    }
}

#[test]
fn match_block_accepts_turn_shaped_comprehension_local_with_closed_yield() {
    let compiled =
        compile("signal<bool> ? | true => ([item | item <- [1]] ? | * => 1) | false => 0");
    let artifact = compiled.compile_artifact().unwrap();
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.nodes().len(), artifact.nodes().len());
}

#[test]
fn comprehension_match_comprehension_maps_inner_operation_contract() {
    let source = "[(item ? | * => ([x + 1 | x <- [1]] ? | * => 1)) | item <- [1]]";
    let artifact = compile(source).compile_artifact().unwrap();
    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.nodes().len(), artifact.nodes().len());
}

#[cfg(feature = "resident-artifact")]
#[test]
fn nested_capability_witnesses_preserve_constant_and_local_selector_provenance() {
    use mech_engine::resident::{
        ActivationFacts, ResidentActivationOptions, preflight_resident_target,
    };
    let mut catalog = mech_core::FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    for (index, constant) in [("1", true), ("item + 1", false)] {
        let source = format!(
            "(values<[f64]:1,2>, signal<f64> ? | item => (({index}) ? | index => values[index]))"
        );
        let artifact = compile(&source).compile_artifact().unwrap();
        let witness = preflight_resident_target(
            &artifact,
            &catalog,
            &ActivationFacts::default(),
            ResidentActivationOptions::default(),
        )
        .unwrap();
        let access = witness
            .concrete_cases
            .iter()
            .find(|case| {
                case.operation
                    .module_path
                    .iter()
                    .any(|name| name == "access")
            })
            .expect("nested access capability");
        assert_eq!(access.node, NodeId(0));
        assert_eq!(access.input_resolved_selectors.len(), 2);
        assert_eq!(
            access.input_resolved_selectors[1].is_some(),
            constant,
            "{source}"
        );
    }
    let artifact = compile("flag<bool> ? | * => (signal<u8> ? | 1u8 => 1 | * => 2)")
        .compile_artifact()
        .unwrap();
    preflight_resident_target(
        &artifact,
        &catalog,
        &ActivationFacts::default(),
        ResidentActivationOptions::default(),
    )
    .expect("nested snapshot-backed scalar literals have resident capability");
}

#[test]
fn nested_control_admission_checks_descendant_scopes_and_counts() {
    let artifact = compile("signal<f64> ? | item => ((item + 1) ? | inner => inner + item)")
        .compile_artifact()
        .unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    for mutation in 0..5 {
        let mut sections = sections.clone();
        let mut graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
        let outer = &mut graph["nodes"][0]["body"]["Match"]["arms"][0]["body"];
        let nested = &mut outer["operations"][1]["body"]["Match"];
        let body = &mut nested["arms"][0]["body"];
        match mutation {
            0 => body["id"] = serde_json::json!(0),
            1 => body["yield_value"] = serde_json::json!({"Local":{"block":0,"node":0}}),
            2 => {
                body["operations"][0]["inputs"][0] =
                    serde_json::json!({"Local":{"block":1,"node":0}})
            }
            3 => {
                body["operations"][0]["body"]["Operation"]["contract"] = serde_json::json!(u32::MAX)
            }
            _ => body["operations"][0]["contract"] = serde_json::json!(0),
        }
        sections.nodes = serde_json::to_vec(&graph).unwrap();
        assert!(
            decode_program_artifact_sections(&sections).is_err(),
            "mutation {mutation}"
        );
    }
    let exact = ArtifactDecodeLimits {
        max_control_operations: 3,
        max_control_blocks: 2,
        ..ArtifactDecodeLimits::default()
    };
    decode_program_artifact_sections_with_limits(&sections, exact).unwrap();
    assert!(
        decode_program_artifact_sections_with_limits(
            &sections,
            ArtifactDecodeLimits {
                max_control_operations: 2,
                ..exact
            }
        )
        .is_err()
    );
    assert!(
        decode_program_artifact_sections_with_limits(
            &sections,
            ArtifactDecodeLimits {
                max_control_blocks: 1,
                ..exact
            }
        )
        .is_err()
    );
}

#[test]
fn nested_control_depth_is_bounded_before_artifact_mapping_and_wire_allocation() {
    let mut source = "1".to_owned();
    for _ in 0..MAX_CONTROL_DEPTH {
        source = format!("signal<f64> ? | * => ({source})");
    }
    let program = compile(&source);
    let artifact = program.compile_artifact().unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    decode_program_artifact_sections(&sections).unwrap();
    let mut graph = program.program().clone();
    let schema = graph.outputs[0].schema;
    let SourceNodeBody::Match(root) = &mut graph.nodes[0].body else {
        panic!()
    };
    let repeated = root.clone();
    let mut deepest = root;
    for _ in 1..MAX_CONTROL_DEPTH {
        let ControlOperationBody::Match(nested) = &mut deepest.arms[0].body.operations[0].body
        else {
            panic!()
        };
        deepest = nested;
    }
    deepest.arms[0].body.operations = vec![ControlOperation {
        node: 0,
        body: ControlOperationBody::Match(repeated),
        inputs: Box::new([]),
        schema,
    }]
    .into_boxed_slice();
    assert!(matches!(
        compile_source_program_with_control_contracts(
            &graph,
            &mut ArtifactBuildContext::new(program.schemas(), program.constants()),
            &[None]
        ),
        Err(ArtifactBuildError::InvalidControl {
            reason: "control graph nesting limit",
            ..
        })
    ));
    let source = format!("signal<f64> ? | * => ({source})");
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(822), Revision(1), source.as_str()).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean());
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&find(parsed.syntax()).unwrap())
            .err()
            .unwrap()
            .code,
        "source-semantics/control-depth-limit"
    );
    let mut sections = sections;
    let mut graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
    let repeated = graph["nodes"][0]["body"].clone();
    let mut deepest = &mut graph["nodes"][0]["body"];
    for _ in 1..MAX_CONTROL_DEPTH {
        deepest = &mut deepest["Match"]["arms"][0]["body"]["operations"][0]["body"];
    }
    deepest["Match"]["arms"][0]["body"]["operations"] =
        serde_json::json!([{"node":0,"body":repeated,"inputs":[],"schema":0}]);
    sections.nodes = serde_json::to_vec(&graph).unwrap();
    let error = decode_program_artifact_sections(&sections).unwrap_err();
    assert!(
        matches!(error, ArtifactBytecodeError::Json(_)),
        "preflight must reject before typed construction: {error:?}"
    );
}

#[test]
fn nested_comprehension_depth_is_bounded_before_contract_mapping() {
    let program = compile("[item | item <- [1]]");
    let mut graph = program.program().clone();
    let schema = graph.outputs[0].schema;
    let SourceNodeBody::Comprehension(root) = &mut graph.nodes[0].body else {
        panic!()
    };
    let mut nested = root.clone();
    for depth in 1..=MAX_CONTROL_DEPTH {
        nested = ComprehensionDeclaration {
            id: ControlBlockId(depth as u32),
            kind: ComprehensionKind::Matrix,
            steps: vec![ComprehensionStep::Operation(ComprehensionOperation {
                local: 0,
                body: ControlOperationBody::Comprehension(nested),
                inputs: Box::new([]),
                schema,
            })]
            .into_boxed_slice(),
            yield_value: ComprehensionValue::Local(0),
        };
    }
    *root = nested;
    assert!(matches!(
        compile_source_program_with_control_contracts(
            &graph,
            &mut ArtifactBuildContext::new(program.schemas(), program.constants()),
            &[None]
        ),
        Err(ArtifactBuildError::InvalidControl {
            reason: "control graph nesting limit",
            ..
        })
    ));

    let mut source = "1".to_owned();
    for _ in 0..MAX_CONTROL_DEPTH {
        source = format!("[{source} | item <- [1]]");
    }
    compile(&source);
    source = format!("[{source} | item <- [1]]");
    let parsed = parse_canonical_phase_2i_rule_for_test(
        TextSnapshot::new(DocumentId(823), Revision(1), source.as_str()).unwrap(),
        rules::EXPRESSION,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean());
    assert_eq!(
        CanonicalSourceFrontend
            .compile_expression(&find(parsed.syntax()).unwrap())
            .err()
            .unwrap()
            .code,
        "source-semantics/control-depth-limit"
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn nested_match_failure_is_lazy_and_cannot_publish_a_partial_outer_value() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    let artifact = compile("(values<[f64]:1,2>, outer<bool> ? | true => (inner<bool> ? | true => (1, true) | false => (values[index<f64>], false)) | false => (2, false))").compile_artifact().unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(822, 92),
        &artifact,
        &catalog.build().unwrap(),
        &ActivationFacts::default(),
    )
    .unwrap();
    let slots = instance
        .plan
        .inputs
        .iter()
        .map(|input| input.slot)
        .collect::<Vec<_>>();
    let inputs = |outer, inner, index| {
        [
            CapturedSignalInput {
                slot: slots[0],
                value: ResidentValueRef::F64(&[1.0, 2.0]),
            },
            CapturedSignalInput {
                slot: slots[1],
                value: ResidentValueRef::Bool(outer),
            },
            CapturedSignalInput {
                slot: slots[2],
                value: ResidentValueRef::Bool(inner),
            },
            CapturedSignalInput {
                slot: slots[3],
                value: ResidentValueRef::F64(index),
            },
        ]
    };
    instance.turn(&inputs(&[0], &[0], &[3.0])).unwrap();
    instance.turn(&inputs(&[1], &[1], &[3.0])).unwrap();
    let epoch = instance.published_epoch();
    let output = instance
        .copied_output(0)
        .unwrap()
        .canonical_data_draft()
        .unwrap();
    assert!(instance.turn(&inputs(&[1], &[0], &[3.0])).is_err());
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        output
    );
    instance
        .prepare_turn(&inputs(&[1], &[0], &[2.0]))
        .unwrap()
        .abort();
    assert_eq!(instance.published_epoch(), epoch);
    assert_eq!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        output
    );
    instance.turn(&inputs(&[1], &[0], &[2.0])).unwrap();
    assert_ne!(
        instance
            .copied_output(0)
            .unwrap()
            .canonical_data_draft()
            .unwrap(),
        output
    );
}

#[test]
fn exact_scalar_pattern_schemas_are_artifact_semantics() {
    for source in [
        "x<u8> ? | 1u8 => 2u8 | y => y",
        "x<ix> ? | 1<ix> => 2<ix> | y => y",
        "x<i8> ? | -128<i8> => 0<i8> | y => y",
    ] {
        let artifact = compile(source).compile_artifact().unwrap();
        let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
        let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();
        assert_eq!(artifact.revision(), decoded.revision());
    }
}

#[cfg(feature = "resident-artifact")]
#[test]
fn wildcard_matches_do_not_impose_a_scalar_type_on_unused_scrutinees() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{ActivationFacts, CapturedSignalInput, activate};
    compile("x ? | *, true => 1 | * => 2")
        .compile_artifact()
        .unwrap();
    let artifact = compile("signal<[f64]:1,2> ? | *, true => 1 | * => 2")
        .compile_artifact()
        .unwrap();
    let artifact = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    let mut catalog = FunctionCatalogBuilder::new();
    install_intrinsic_resident(&mut catalog).unwrap();
    let catalog = catalog.build().unwrap();
    let mut instance = activate(
        ReactiveInstanceId::new(822, 45),
        &artifact,
        &catalog,
        &ActivationFacts::default(),
    )
    .unwrap();
    instance
        .turn(&[CapturedSignalInput {
            slot: instance.plan.inputs[0].slot,
            value: ResidentValueRef::F64(&[8.0, 9.0]),
        }])
        .unwrap();
    assert!(
        matches!(instance.copied_output(0).unwrap().data(), ValueData::F64(value) if value.to_f64() == 1.0)
    );
}

#[cfg(feature = "resident-artifact")]
#[test]
fn root_structural_bind_and_equality_materialize_dense_matrix_scrutinees() {
    use mech_core::{FunctionCatalogBuilder, ReactiveInstanceId, ResidentValueRef, ValueData};
    use mech_engine::resident::{
        ActivationFacts, CapturedSignalInput, ResidentValueBorrow, activate,
    };

    let compiled = compile("signal<[f64]:1,2> ? | [1 2] => template<[f64]:1,2> | * => [30 40]");
    let template_schema = compiled.program().inputs[1].schema;
    let template = mech_core::ValueDraft {
        schema: template_schema,
        shape_values: Box::new([]),
        data: mech_core::ValueDataDraft::Matrix(
            [10.0, 20.0]
                .map(|value| {
                    mech_core::ValueDataDraft::F64(mech_core::snapshot::F64Bits::from_f64(value))
                })
                .into(),
        ),
    }
    .finalize(&mech_core::snapshot::SnapshotValidationContext::new(
        compiled.schemas(),
    ))
    .unwrap();
    let artifact = compiled
        .bind_input_constants(&[(1, template)])
        .unwrap()
        .compile_artifact()
        .unwrap();
    let matrix_schema = artifact.inputs()[0].schema;
    let matrix_literal = (0..artifact.constants().len())
        .map(|id| ConstantId::new(id as u32))
        .find(|id| {
            let value = artifact.constants().get(*id).unwrap();
            value.schema() == matrix_schema && matches!(value.data(), ValueData::Matrix(_))
        })
        .expect("the selected-arm result contributes a matrix constant");
    let sections = encode_program_artifact_sections(&artifact).unwrap();
    let catalog = {
        let mut catalog = FunctionCatalogBuilder::new();
        install_intrinsic_resident(&mut catalog).unwrap();
        catalog.build().unwrap()
    };

    for (case, (pattern, turns)) in [
        (
            serde_json::json!({"Structural":{"Equal":{"Literal":matrix_literal.get()}}}),
            vec![
                (&[10.0, 20.0][..], &[10.0, 20.0][..]),
                (&[9.0, 8.0][..], &[30.0, 40.0][..]),
            ],
        ),
        (
            serde_json::json!({"Structural":{"Bind":{"local":0,"schema":matrix_schema.get()}}}),
            vec![(&[9.0, 8.0][..], &[10.0, 20.0][..])],
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let mut sections = sections.clone();
        let mut graph: serde_json::Value = serde_json::from_slice(&sections.nodes).unwrap();
        graph["nodes"][0]["body"]["Match"]["arms"][0]["pattern"] = pattern;
        sections.nodes = serde_json::to_vec(&graph).unwrap();
        let artifact = decode_program_artifact_sections(&sections).unwrap();
        let mut instance = activate(
            ReactiveInstanceId::new(822, 46),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        )
        .unwrap();
        for (input, expected) in turns {
            instance
                .turn(&[CapturedSignalInput {
                    slot: instance.plan.inputs[0].slot,
                    value: ResidentValueRef::F64(input),
                }])
                .unwrap_or_else(|error| panic!("case {case}, input {input:?}: {error:?}"));
            assert!(matches!(
                instance.output_borrow(0),
                Some(ResidentValueBorrow::F64 { values, .. }) if values == expected
            ));
        }
    }
}
