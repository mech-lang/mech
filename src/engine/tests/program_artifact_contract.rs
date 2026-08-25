#![cfg(feature = "compiler")]

use mech_engine::*;

use mech_core::{
    AccessMode, AliasPolicy, ApplicationRequirement, ApplicationRequirementId, BytecodeProgram,
    CanonicalNominalPath, ChangeDetectionPolicy, ConstantHandle, ConstantStoreBuilder,
    DeliveryMode, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, EffectContract, EffectDeliveryPolicy,
    ExecutionResourceRequest, ExternalInteraction, FloatWidth, IdempotencyRequirement,
    InputPortLayout, InputPortPolicy, IntegerWidth, KindExpr, LegacyOpaqueOperationContract,
    LegacyValue, NominalKey, NominalKind, ObservationContract, ObservationReplayPolicy,
    OperationContractDeclaration, OperationContractId, OperationContractTable,
    OperationContractTableBuilder, OutputConstruction, OutputPortPolicy, RegionPolicy,
    ResolvedInputPort, ResolvedOperationContract, ResolvedOutputPort, ResourceDelivery,
    ResourceIntent, SchemaBody, SchemaDraft, SchemaField, SchemaHandle, SchemaTableBuilder,
    ShapeContractReference, ShapeRule, Value, ValueDataDraft, ValueDraft,
    snapshot::{
        Complex32Bits, Complex64Bits, ConstantStoreBuild, EnumDraft, F32Bits, F64Bits,
        MapEntryDraft, NamedValueDraft, OptionDraft, ReifiedTypeDraft, SnapshotValidationContext,
        TableColumnDraft,
    },
    write_bytecode_with_artifact,
};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy)]
struct Schemas {
    bool_: SchemaId,
    f64_: SchemaId,
    vector2: SchemaId,
    vector3: SchemaId,
    matrix2: SchemaId,
    matrix3: SchemaId,
    matrix2x3: SchemaId,
    matrix3x2: SchemaId,
    record: SchemaId,
}

#[derive(Clone, Copy)]
struct Constants {
    false_: ConstantId,
    one: ConstantId,
    two: ConstantId,
    vector3: ConstantId,
    matrix3: ConstantId,
    matrix2: ConstantId,
}

struct FixtureData {
    schemas: SchemaTable,
    constants: ConstantStore,
    schema: Schemas,
    constant: Constants,
}

fn schema(body: SchemaBody) -> mech_core::Schema {
    SchemaDraft {
        dimension_parameters: Box::new([]),
        body,
    }
    .finalize()
    .unwrap()
}

fn matrix_schema(rows: u64, columns: u64) -> mech_core::Schema {
    schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    })
}

fn resolve_schema(build: &mech_core::SchemaTableBuild, handle: SchemaHandle) -> SchemaId {
    build.resolve(handle).unwrap()
}

fn scalar_value(schemas: &SchemaTable, schema: SchemaId, value: f64) -> Value {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::F64(F64Bits::from_f64(value)),
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}

fn bool_value(schemas: &SchemaTable, schema: SchemaId, value: bool) -> Value {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Bool(value),
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}

fn matrix_value(
    schemas: &SchemaTable,
    schema: SchemaId,
    rows: usize,
    columns: usize,
    value: f64,
) -> Value {
    ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: ValueDataDraft::Matrix(
            vec![ValueDataDraft::F64(F64Bits::from_f64(value)); rows * columns].into_boxed_slice(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(schemas))
    .unwrap()
}

fn fixture_data() -> FixtureData {
    let mut schemas = SchemaTableBuilder::new();
    let bool_ = schemas.insert(schema(SchemaBody::Bool)).unwrap();
    let f64_ = schemas
        .insert(schema(SchemaBody::FloatingPoint(FloatWidth::W64)))
        .unwrap();
    let vector2 = schemas.insert(matrix_schema(2, 1)).unwrap();
    let vector3 = schemas.insert(matrix_schema(3, 1)).unwrap();
    let matrix2 = schemas.insert(matrix_schema(2, 2)).unwrap();
    let matrix3 = schemas.insert(matrix_schema(3, 3)).unwrap();
    let matrix2x3 = schemas.insert(matrix_schema(2, 3)).unwrap();
    let matrix3x2 = schemas.insert(matrix_schema(3, 2)).unwrap();
    let record = schemas
        .insert(schema(SchemaBody::Record(
            vec![
                SchemaField {
                    name: "value".to_owned(),
                    schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                },
                SchemaField {
                    name: "valid".to_owned(),
                    schema: SchemaBody::Bool,
                },
            ]
            .into_boxed_slice(),
        )))
        .unwrap();
    let build = schemas.finish().unwrap();
    let schema = Schemas {
        bool_: resolve_schema(&build, bool_),
        f64_: resolve_schema(&build, f64_),
        vector2: resolve_schema(&build, vector2),
        vector3: resolve_schema(&build, vector3),
        matrix2: resolve_schema(&build, matrix2),
        matrix3: resolve_schema(&build, matrix3),
        matrix2x3: resolve_schema(&build, matrix2x3),
        matrix3x2: resolve_schema(&build, matrix3x2),
        record: resolve_schema(&build, record),
    };
    let (schemas, _) = build.into_parts();

    let mut constants = ConstantStoreBuilder::new(&schemas);
    let false_ = constants
        .insert(bool_value(&schemas, schema.bool_, false))
        .unwrap();
    let one = constants
        .insert(scalar_value(&schemas, schema.f64_, 1.0))
        .unwrap();
    let two = constants
        .insert(scalar_value(&schemas, schema.f64_, 2.0))
        .unwrap();
    let vector3 = constants
        .insert(matrix_value(&schemas, schema.vector3, 3, 1, 0.0))
        .unwrap();
    let matrix3 = constants
        .insert(matrix_value(&schemas, schema.matrix3, 3, 3, 0.0))
        .unwrap();
    let matrix2 = constants
        .insert(matrix_value(&schemas, schema.matrix2, 2, 2, 1.0))
        .unwrap();
    let build = constants.finish().unwrap();
    let constant = Constants {
        false_: resolve_constant(&build, false_),
        one: resolve_constant(&build, one),
        two: resolve_constant(&build, two),
        vector3: resolve_constant(&build, vector3),
        matrix3: resolve_constant(&build, matrix3),
        matrix2: resolve_constant(&build, matrix2),
    };
    let (constants, _) = build.into_parts();
    FixtureData {
        schemas,
        constants,
        schema,
        constant,
    }
}

fn resolve_constant(build: &ConstantStoreBuild, handle: ConstantHandle) -> ConstantId {
    build.resolve(handle).unwrap()
}

fn operation(module: &str, name: &str) -> OperationReference {
    OperationReference {
        module_path: vec![module.to_owned()].into_boxed_slice(),
        operation_name: name.to_owned(),
    }
}

fn node(
    operation: OperationReference,
    inputs: Vec<SourceValue>,
    outputs: Vec<SourceNodeOutput>,
) -> SourceNode {
    SourceNode {
        operation,
        requirement: None,
        inputs: inputs.into_boxed_slice(),
        outputs: outputs.into_boxed_slice(),
    }
}

fn single_node_fixture(
    operation: OperationReference,
    inputs: Vec<SourceValue>,
    schema: SchemaId,
) -> SourceProgram {
    SourceProgram {
        nodes: vec![node(
            operation,
            inputs,
            vec![SourceNodeOutput::Derived { schema }],
        )]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "result".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    }
}

fn constant_scalar(data: &FixtureData) -> SourceProgram {
    single_node_fixture(
        operation("core", "constant"),
        vec![SourceValue::Constant(data.constant.one)],
        data.schema.f64_,
    )
}

fn scalar_add(data: &FixtureData) -> SourceProgram {
    single_node_fixture(
        operation("math", "add"),
        vec![
            SourceValue::Constant(data.constant.one),
            SourceValue::Constant(data.constant.two),
        ],
        data.schema.f64_,
    )
}

fn fixed_matrix_add(data: &FixtureData) -> SourceProgram {
    single_node_fixture(
        operation("math", "add"),
        vec![
            SourceValue::Constant(data.constant.matrix2),
            SourceValue::Constant(data.constant.matrix2),
        ],
        data.schema.matrix2,
    )
}

fn stateful_register(data: &FixtureData) -> SourceProgram {
    SourceProgram {
        inputs: vec![SourceInput {
            name: "next".to_owned(),
            schema: data.schema.f64_,
        }]
        .into_boxed_slice(),
        states: vec![SourceState {
            schema: data.schema.f64_,
            initializer: Some(data.constant.one),
            producer_node: 0,
            producer_output_ordinal: 0,
        }]
        .into_boxed_slice(),
        nodes: vec![node(
            operation("state", "register"),
            vec![SourceValue::Input(0), SourceValue::State(0)],
            vec![SourceNodeOutput::State(0)],
        )]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "state".to_owned(),
            interactive_symbol: None,
            source: SourceValue::State(0),
            schema: data.schema.f64_,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    }
}

fn matrix_selection(data: &FixtureData, all: bool) -> SourceProgram {
    single_node_fixture(
        operation("matrix", if all { "select-all" } else { "select-element" }),
        vec![SourceValue::Constant(data.constant.matrix2)],
        if all {
            data.schema.matrix2
        } else {
            data.schema.f64_
        },
    )
}

fn record_construction(data: &FixtureData) -> SourceProgram {
    single_node_fixture(
        operation("record", "construct"),
        vec![
            SourceValue::Constant(data.constant.one),
            SourceValue::Constant(data.constant.false_),
        ],
        data.schema.record,
    )
}

fn ekf(data: &FixtureData) -> SourceProgram {
    use SourceNodeOutput::{Derived, State};
    use SourceValue::{Input, NodeOutput, State as StateSource};
    let output = |node, output_ordinal| NodeOutput {
        node,
        output_ordinal,
    };
    let derived = |schema| vec![Derived { schema }];
    let nodes = vec![
        node(
            operation("ekf", "trigonometric-state"),
            vec![StateSource(0)],
            derived(data.schema.vector2),
        ),
        node(
            operation("ekf", "motion-jacobian"),
            vec![StateSource(0), Input(0), output(0, 0)],
            derived(data.schema.matrix3),
        ),
        node(
            operation("ekf", "control-jacobian"),
            vec![output(0, 0)],
            derived(data.schema.matrix3x2),
        ),
        node(
            operation("ekf", "predicted-state"),
            vec![StateSource(0), Input(0), output(0, 0)],
            derived(data.schema.vector3),
        ),
        node(
            operation("ekf", "predicted-covariance"),
            vec![StateSource(1), output(1, 0), output(2, 0)],
            derived(data.schema.matrix3),
        ),
        node(
            operation("ekf", "landmark-delta-and-range"),
            vec![output(3, 0)],
            derived(data.schema.vector3),
        ),
        node(
            operation("ekf", "predicted-measurement"),
            vec![output(3, 0), output(5, 0)],
            derived(data.schema.vector2),
        ),
        node(
            operation("ekf", "measurement-jacobian"),
            vec![output(5, 0)],
            derived(data.schema.matrix2x3),
        ),
        node(
            operation("ekf", "innovation-covariance"),
            vec![output(4, 0), output(7, 0)],
            derived(data.schema.matrix2),
        ),
        node(
            operation("ekf", "solve-2x2"),
            vec![output(8, 0)],
            derived(data.schema.matrix2),
        ),
        node(
            operation("ekf", "kalman-gain"),
            vec![output(4, 0), output(7, 0), output(9, 0)],
            derived(data.schema.matrix3x2),
        ),
        node(
            operation("ekf", "innovation"),
            vec![Input(0), output(6, 0)],
            derived(data.schema.vector2),
        ),
        node(
            operation("ekf", "corrected-state"),
            vec![output(3, 0), output(10, 0), output(11, 0)],
            derived(data.schema.vector3),
        ),
        node(
            operation("ekf", "joseph-covariance-update"),
            vec![output(4, 0), output(7, 0), output(10, 0)],
            derived(data.schema.matrix3),
        ),
        node(
            operation("ekf", "covariance-symmetrization"),
            vec![output(12, 0), output(13, 0)],
            vec![State(0), State(1)],
        ),
    ];
    SourceProgram {
        inputs: vec![SourceInput {
            name: "measurement".to_owned(),
            schema: data.schema.matrix2,
        }]
        .into_boxed_slice(),
        states: vec![
            SourceState {
                schema: data.schema.vector3,
                initializer: Some(data.constant.vector3),
                producer_node: 14,
                producer_output_ordinal: 0,
            },
            SourceState {
                schema: data.schema.matrix3,
                initializer: Some(data.constant.matrix3),
                producer_node: 14,
                producer_output_ordinal: 1,
            },
        ]
        .into_boxed_slice(),
        nodes: nodes.into_boxed_slice(),
        outputs: vec![
            SourceOutput {
                name: "state".to_owned(),
                interactive_symbol: None,
                source: SourceValue::State(0),
                schema: data.schema.vector3,
            },
            SourceOutput {
                name: "covariance".to_owned(),
                interactive_symbol: None,
                source: SourceValue::State(1),
                schema: data.schema.matrix3,
            },
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    }
}

fn build_both(data: &FixtureData, graph: SourceProgram) -> (ProgramArtifact, ProgramArtifact) {
    let mut source_context = ArtifactBuildContext::new(&data.schemas, &data.constants);
    let source = compile_source_program(&graph, &mut source_context).unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&source).unwrap();
    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    assert!(!parsed.artifact.is_empty());
    let bytecode = decode_program_artifact_sections(&parsed.artifact).unwrap();
    (source, bytecode)
}

#[test]
fn output_aliases_share_one_materialized_source_slot() {
    let data = fixture_data();
    let graph = SourceProgram {
        outputs: vec![
            SourceOutput {
                name: "first".to_owned(),
                interactive_symbol: None,
                source: SourceValue::Constant(data.constant.one),
                schema: data.schema.f64_,
            },
            SourceOutput {
                name: "second".to_owned(),
                interactive_symbol: None,
                source: SourceValue::Constant(data.constant.one),
                schema: data.schema.f64_,
            },
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };

    let (source, bytecode) = build_both(&data, graph);
    for artifact in [&source, &bytecode] {
        assert_eq!(artifact.outputs()[0].source, artifact.outputs()[1].source);
        assert_eq!(
            artifact
                .slots()
                .iter()
                .filter(|slot| slot.role == SlotRole::Output)
                .count(),
            1,
            "one semantic source must materialize through one output slot"
        );
    }
}

#[test]
fn interactive_symbol_interface_encoding_is_canonical_and_reversible() {
    for lexical in ["odd/name", r"odd\name", "café", "mech-repl-symbol-61"] {
        let encoded = encode_interactive_symbol_output_name(lexical);
        assert!(!encoded.contains(['/', '\\']));
        assert_eq!(
            decode_interactive_symbol_output_name(&encoded).as_deref(),
            Some(lexical)
        );
    }
}

#[test]
fn interactive_symbol_identity_is_explicit_and_survives_bytecode() {
    let data = fixture_data();
    let lexical = "odd/name";
    let graph = SourceProgram {
        outputs: vec![
            SourceOutput {
                name: "query-1".to_owned(),
                interactive_symbol: Some(lexical.to_owned()),
                source: SourceValue::Constant(data.constant.one),
                schema: data.schema.f64_,
            },
            SourceOutput {
                name: "query-2".to_owned(),
                interactive_symbol: Some(r"odd\name".to_owned()),
                source: SourceValue::Constant(data.constant.one),
                schema: data.schema.f64_,
            },
            SourceOutput {
                name: "mech-repl-symbol-61".to_owned(),
                interactive_symbol: None,
                source: SourceValue::Constant(data.constant.two),
                schema: data.schema.f64_,
            },
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };

    let (source, bytecode) = build_both(&data, graph);
    for artifact in [&source, &bytecode] {
        assert_eq!(
            artifact.outputs()[0]
                .interactive_binding
                .as_ref()
                .map(|binding| binding.lexical_name.as_str()),
            Some(lexical)
        );
        assert_eq!(artifact.outputs()[2].name, "mech-repl-symbol-61");
        assert_eq!(artifact.outputs()[2].interactive_binding, None);
        let bindings = artifact.interactive_symbol_bindings().collect::<Vec<_>>();
        assert_eq!(bindings.len(), 2);
        assert_eq!(bindings[0].output, artifact.outputs()[0].output);
        assert_eq!(bindings[0].storage, artifact.outputs()[0].source);
        assert_eq!(
            bindings[0].artifact_source,
            ArtifactSource::Constant(data.constant.one)
        );
        assert_eq!(bindings[1].lexical_name, r"odd\name");
        assert_eq!(bindings[1].artifact_source, bindings[0].artifact_source);
        assert_eq!(bindings[1].storage, bindings[0].storage);
    }
}

fn pure_full_write_contract(
    input_count: usize,
    output_count: usize,
) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                input_count
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![
            OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::KernelReported,
            };
            output_count
        ]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn pure_state_rmw_contract(base_input: u16) -> OperationContractDeclaration {
    OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![
                InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                };
                2
            ]
            .into_boxed_slice(),
        ),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::ReadWrite,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::ReadModifyWrite {
                base_input,
                regions: RegionPolicy::WholeValue,
            },
            alias: AliasPolicy::MayAlias { input: base_input },
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Pure,
    }
}

fn build_both_with_contracts(
    data: &FixtureData,
    graph: SourceProgram,
    declarations: &[Option<&'static OperationContractDeclaration>],
) -> (ProgramArtifact, ProgramArtifact) {
    let mut source_context = ArtifactBuildContext::new(&data.schemas, &data.constants);
    let source =
        compile_source_program_with_contracts(&graph, &mut source_context, declarations).unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&source).unwrap();
    let parsed = ParsedProgram::from_bytes(&bytes).unwrap();
    let bytecode = decode_program_artifact_sections(&parsed.artifact).unwrap();
    (source, bytecode)
}

#[test]
fn representative_source_and_bytecode_routes_produce_identical_artifacts() {
    let data = fixture_data();
    let fixtures = [
        constant_scalar(&data),
        scalar_add(&data),
        fixed_matrix_add(&data),
        stateful_register(&data),
        matrix_selection(&data, false),
        matrix_selection(&data, true),
        record_construction(&data),
        ekf(&data),
    ];
    for graph in fixtures {
        let (source, bytecode) = build_both(&data, graph);
        assert_eq!(source.revision(), bytecode.revision());
        assert_eq!(source.contracts(), bytecode.contracts());
        assert_eq!(source.inputs(), bytecode.inputs());
        assert_eq!(source.slots(), bytecode.slots());
        assert_eq!(source.nodes(), bytecode.nodes());
        assert_eq!(source.bindings(), bytecode.bindings());
        assert_eq!(source.outputs(), bytecode.outputs());
        assert_eq!(source.constraints(), bytecode.constraints());
        assert!(
            source
                .slots()
                .iter()
                .all(|slot| slot.slot.get() < source.slots().len() as u32)
        );
        assert!(
            source
                .bindings()
                .iter()
                .enumerate()
                .all(|(index, binding)| {
                    binding.id() == BindingId(u32::try_from(index).unwrap())
                })
        );
    }
}

#[test]
fn synthetic_ekf_contract_fixture_is_fully_declared_and_round_trips_contract_ids() {
    // This synthetic graph intentionally injects a declaration for every node.
    // It proves canonical contract-table construction, zero-opaque behavior when
    // metadata is complete, and bytecode contract-ID round-tripping. It does not
    // claim that ordinary source compilation already supplies complete EKF
    // operation-contract coverage.
    let data = fixture_data();
    let graph = ekf(&data);
    let declarations = graph
        .nodes
        .iter()
        .map(|node| {
            Some(Box::leak(Box::new(pure_full_write_contract(
                node.inputs.len(),
                node.outputs.len(),
            ))) as &'static OperationContractDeclaration)
        })
        .collect::<Vec<_>>();
    let (source, bytecode) = build_both_with_contracts(&data, graph, &declarations);

    assert!(
        source
            .contracts()
            .iter()
            .all(|contract| matches!(contract, ResolvedOperationContract::Declared(_)))
    );
    assert_eq!(
        source
            .contracts()
            .iter()
            .filter(|contract| matches!(contract, ResolvedOperationContract::LegacyOpaque(_)))
            .count(),
        0
    );
    assert_eq!(source.revision(), bytecode.revision());
    assert_eq!(source.contracts(), bytecode.contracts());
    assert_eq!(
        source
            .nodes()
            .iter()
            .map(|node| node.contract)
            .collect::<Vec<_>>(),
        bytecode
            .nodes()
            .iter()
            .map(|node| node.contract)
            .collect::<Vec<_>>()
    );
}

#[test]
fn state_slots_are_initialized_and_break_feedback_cycles() {
    let data = fixture_data();
    let (register, _) = build_both(&data, stateful_register(&data));
    let state = register
        .slots()
        .iter()
        .find(|slot| slot.role == SlotRole::State)
        .unwrap();
    assert!(matches!(
        state.initializer,
        Some(InitializerReference::Constant(_))
    ));
    assert!(matches!(
        state.producer,
        ProducerReference::NodeOutput { .. }
    ));

    let (ekf, _) = build_both(&data, ekf(&data));
    assert_eq!(ekf.nodes().len(), 15);
    assert_eq!(ekf.slots().len(), 17);
    assert_eq!(
        ekf.slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
            .count(),
        2
    );
    assert!(
        ekf.slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::State)
            .all(|slot| slot.initializer.is_some())
    );
}

#[test]
fn constants_remain_sources_and_outputs_receive_only_publication_slots() {
    let data = fixture_data();
    let (artifact, _) = build_both(&data, scalar_add(&data));
    assert_eq!(artifact.slots().len(), 2);
    assert_eq!(
        artifact
            .slots()
            .iter()
            .filter(|slot| slot.role == SlotRole::Output)
            .count(),
        1
    );
    assert_eq!(artifact.constants().len(), data.constants.len());
    assert!(artifact.bindings().iter().any(|binding| matches!(
        binding,
        BindingDeclaration::Input {
            source: ArtifactSource::Constant(_),
            ..
        }
    )));
}

#[test]
fn combinational_cycles_are_rejected_but_state_feedback_is_valid() {
    let data = fixture_data();
    let graph = SourceProgram {
        nodes: vec![
            node(
                operation("test", "first"),
                vec![SourceValue::NodeOutput {
                    node: 1,
                    output_ordinal: 0,
                }],
                vec![SourceNodeOutput::Derived {
                    schema: data.schema.f64_,
                }],
            ),
            node(
                operation("test", "second"),
                vec![SourceValue::NodeOutput {
                    node: 0,
                    output_ordinal: 0,
                }],
                vec![SourceNodeOutput::Derived {
                    schema: data.schema.f64_,
                }],
            ),
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    let mut context = ArtifactBuildContext::new(&data.schemas, &data.constants);
    assert!(matches!(
        compile_source_program(&graph, &mut context),
        Err(ArtifactBuildError::CombinationalCycle)
    ));
    assert!(build_both(&data, stateful_register(&data)).0.nodes().len() == 1);
}

#[test]
fn state_reads_depend_on_the_latest_preceding_writer() {
    let data = fixture_data();
    let graph = SourceProgram {
        states: vec![SourceState {
            schema: data.schema.f64_,
            initializer: Some(data.constant.one),
            producer_node: 0,
            producer_output_ordinal: 0,
        }]
        .into_boxed_slice(),
        nodes: vec![
            node(
                operation("state", "write"),
                vec![
                    SourceValue::NodeOutput {
                        node: 1,
                        output_ordinal: 0,
                    },
                    SourceValue::State(0),
                ],
                vec![SourceNodeOutput::State(0)],
            ),
            node(
                operation("state", "read-after-write"),
                vec![SourceValue::State(0)],
                vec![SourceNodeOutput::Derived {
                    schema: data.schema.f64_,
                }],
            ),
        ]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    let write = Box::leak(Box::new(pure_state_rmw_contract(1)));
    let read = Box::leak(Box::new(pure_full_write_contract(1, 1)));
    let mut context = ArtifactBuildContext::new(&data.schemas, &data.constants);
    assert!(matches!(
        compile_source_program_with_contracts(&graph, &mut context, &[Some(write), Some(read)],),
        Err(ArtifactBuildError::CombinationalCycle)
    ));
}

#[test]
fn compiler_pseudo_values_never_enter_snapshot_constants() {
    assert_eq!(
        compiler_ir_from_legacy_pseudo_value(&LegacyValue::Empty).unwrap(),
        ExpressionIR::Empty
    );
    assert_eq!(
        compiler_ir_from_legacy_pseudo_value(&LegacyValue::IndexAll).unwrap(),
        ExpressionIR::Selection(SelectionIR::All)
    );
}

#[test]
fn matrix_literal_resolution_is_homogeneous_and_structured() {
    let data = fixture_data();
    let literal = MatrixLiteralIR {
        rows: 2,
        columns: 2,
        elements: vec![ExpressionIR::Constant(data.constant.one); 4].into_boxed_slice(),
    };
    let value = literal
        .resolve_constant(data.schema.matrix2, &data.schemas, &data.constants)
        .unwrap();
    assert_eq!(value.schema(), data.schema.matrix2);

    let heterogeneous = MatrixLiteralIR {
        rows: 2,
        columns: 2,
        elements: vec![
            ExpressionIR::Constant(data.constant.one),
            ExpressionIR::Constant(data.constant.one),
            ExpressionIR::Constant(data.constant.one),
            ExpressionIR::Constant(data.constant.false_),
        ]
        .into_boxed_slice(),
    };
    assert!(matches!(
        heterogeneous.resolve_constant(data.schema.matrix2, &data.schemas, &data.constants),
        Err(CompilerIrError::HeterogeneousMatrixLiteral { index: 3 })
    ));

    let unresolved = MatrixLiteralIR {
        rows: 2,
        columns: 2,
        elements: vec![ExpressionIR::Slot(CellSlotId(0)); 4].into_boxed_slice(),
    };
    assert!(matches!(
        unresolved.resolve_constant(data.schema.matrix2, &data.schemas, &data.constants),
        Err(CompilerIrError::MatrixLiteralElementNotConstant { index: 0 })
    ));
}

#[test]
fn matrix_literal_resolution_accepts_homogeneous_aggregate_elements() {
    let mut schemas = SchemaTableBuilder::new();
    let tuple = schemas
        .insert(schema(SchemaBody::Tuple(
            vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
        )))
        .unwrap();
    let matrix = schemas
        .insert(schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Tuple(
                vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
            )),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)]
                .into_boxed_slice(),
        }))
        .unwrap();
    let build = schemas.finish().unwrap();
    let tuple = build.resolve(tuple).unwrap();
    let matrix = build.resolve(matrix).unwrap();
    let (schemas, _) = build.into_parts();
    let validation = SnapshotValidationContext::new(&schemas);
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let mut handles = Vec::new();
    for (flag, text) in [(true, "left"), (false, "right")] {
        handles.push(
            constants
                .insert(
                    ValueDraft {
                        schema: tuple,
                        shape_values: Box::new([]),
                        data: ValueDataDraft::Tuple(
                            vec![
                                ValueDataDraft::Bool(flag),
                                ValueDataDraft::String(text.to_owned()),
                            ]
                            .into_boxed_slice(),
                        ),
                    }
                    .finalize(&validation)
                    .unwrap(),
                )
                .unwrap(),
        );
    }
    let build = constants.finish().unwrap();
    let constants_ids = handles
        .into_iter()
        .map(|handle| build.resolve(handle).unwrap())
        .collect::<Vec<_>>();
    let (constants, _) = build.into_parts();
    let literal = MatrixLiteralIR {
        rows: 1,
        columns: 2,
        elements: constants_ids
            .into_iter()
            .map(ExpressionIR::Constant)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    };
    let value = literal
        .resolve_constant(matrix, &schemas, &constants)
        .unwrap();
    assert_eq!(value.schema(), matrix);
}

#[test]
fn malformed_artifacts_reject_reviewed_validation_gaps() {
    let data = fixture_data();
    let mismatched_initializer = SourceProgram {
        states: vec![SourceState {
            schema: data.schema.f64_,
            initializer: Some(data.constant.false_),
            producer_node: 0,
            producer_output_ordinal: 0,
        }]
        .into_boxed_slice(),
        nodes: vec![node(
            operation("state", "register"),
            vec![SourceValue::Constant(data.constant.one)],
            vec![SourceNodeOutput::State(0)],
        )]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    assert!(matches!(
        compile_source_program(
            &mismatched_initializer,
            &mut ArtifactBuildContext::new(&data.schemas, &data.constants)
        ),
        Err(ArtifactBuildError::InitializerSchemaMismatch { .. })
    ));

    let mut contract_builder = OperationContractTableBuilder::new();
    contract_builder
        .insert(ResolvedOperationContract::LegacyOpaque(
            LegacyOpaqueOperationContract {
                input_schemas: Box::new([]),
                output_schemas: Box::new([]),
            },
        ))
        .unwrap();
    let contracts = contract_builder.finish().unwrap().table;
    let missing_binding = ProgramArtifactDraft {
        schemas: data.schemas.clone(),
        constants: data.constants.clone(),
        contracts,
        requirements: ApplicationRequirementTable::empty(),
        inputs: Box::new([]),
        slots: vec![SlotDeclaration {
            slot: CellSlotId(0),
            schema: data.schema.f64_,
            role: SlotRole::Derived,
            producer: ProducerReference::NodeOutput {
                node: NodeId(0),
                output_ordinal: 0,
            },
            initializer: None,
        }]
        .into_boxed_slice(),
        nodes: vec![NodeDeclaration {
            node: NodeId(0),
            operation: operation("test", "producer"),
            contract: OperationContractId::new(0),
            requirement: None,
            input_bindings: 0..0,
            output_bindings: 0..0,
        }]
        .into_boxed_slice(),
        bindings: Box::new([]),
        outputs: Box::new([]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    };
    assert!(matches!(
        missing_binding.finalize(),
        Err(ArtifactBuildError::MissingProducerBinding { .. })
    ));

    let empty_module = SourceProgram {
        nodes: vec![node(
            OperationReference {
                module_path: Box::new([]),
                operation_name: "invalid".to_owned(),
            },
            Vec::new(),
            vec![SourceNodeOutput::Derived {
                schema: data.schema.f64_,
            }],
        )]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    assert!(matches!(
        compile_source_program(
            &empty_module,
            &mut ArtifactBuildContext::new(&data.schemas, &data.constants)
        ),
        Err(ArtifactBuildError::InvalidOperationReference { .. })
    ));

    let too_many_ports = SourceProgram {
        nodes: vec![node(
            operation("test", "wide"),
            vec![SourceValue::Constant(data.constant.one); u16::MAX as usize + 2],
            vec![SourceNodeOutput::Derived {
                schema: data.schema.f64_,
            }],
        )]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    assert!(matches!(
        compile_source_program(
            &too_many_ports,
            &mut ArtifactBuildContext::new(&data.schemas, &data.constants)
        ),
        Err(ArtifactBuildError::ArtifactIdentityExhausted {
            identity: "input port ordinal"
        })
    ));
}

fn one_output_bytecode_contract(
    alias: AliasPolicy,
    construction: OutputConstruction,
    interaction: ExternalInteraction,
) -> ResolvedOperationContract {
    ResolvedOperationContract::Declared(mech_core::DeclaredOperationContract {
        inputs: vec![ResolvedInputPort {
            schema: SchemaId::new(3),
            access: AccessMode::Read,
            delivery: DeliveryMode::Signal,
        }]
        .into_boxed_slice(),
        outputs: vec![ResolvedOutputPort {
            schema: SchemaId::new(3),
            access: match &construction {
                OutputConstruction::ReadModifyWrite { .. } => AccessMode::ReadWrite,
                _ => AccessMode::Write,
            },
            delivery: DeliveryMode::Signal,
            construction,
            alias,
            change_detection: ChangeDetectionPolicy::KernelReported,
        }]
        .into_boxed_slice(),
        interaction,
    })
}

fn one_entry_operation_contract_table(contract: &[u8]) -> Vec<u8> {
    let mut table = Vec::with_capacity(8 + contract.len());
    table.extend_from_slice(&1_u32.to_le_bytes());
    table.extend_from_slice(&u32::try_from(contract.len()).unwrap().to_le_bytes());
    table.extend_from_slice(contract);
    table
}

fn decode_with_mutated_operation_contract(
    data: &FixtureData,
    contract: ResolvedOperationContract,
    mutate: impl FnOnce(&mut Vec<u8>),
) -> Result<ProgramArtifact, ArtifactBytecodeError> {
    let artifact = build_both(data, scalar_add(data)).0;
    let mut sections = encode_program_artifact_sections(&artifact).unwrap();
    let mut contract = contract.canonical_bytes().unwrap().into_vec();
    mutate(&mut contract);
    sections.operation_contracts = one_entry_operation_contract_table(&contract);
    decode_program_artifact_sections(&sections)
}

#[test]
fn artifact_bytecode_rejects_malformed_operation_contract_semantics_first() {
    let data = fixture_data();
    for alias in [
        AliasPolicy::MayAlias { input: 0 },
        AliasPolicy::InPlaceRequired { input: 0 },
    ] {
        let result = decode_with_mutated_operation_contract(
            &data,
            one_output_bytecode_contract(
                alias,
                OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                ExternalInteraction::Pure,
            ),
            |bytes| bytes[16..20].copy_from_slice(&9_u32.to_le_bytes()),
        );
        assert!(matches!(
            result,
            Err(ArtifactBytecodeError::Artifact(
                ArtifactBuildError::OperationContract(
                    mech_core::OperationContractError::AliasSchemaMismatch {
                        output: 0,
                        input: 0,
                        input_schema,
                        output_schema,
                    }
                )
            )) if input_schema == SchemaId::new(3) && output_schema == SchemaId::new(9)
        ));
    }

    let effect = decode_with_mutated_operation_contract(
        &data,
        one_output_bytecode_contract(
            AliasPolicy::NoAlias,
            OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            ExternalInteraction::Pure,
        ),
        |bytes| {
            assert_eq!(bytes.pop(), Some(0));
            bytes.extend_from_slice(&[2, 0, 0]);
        },
    );
    assert!(matches!(
        effect,
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::OperationContract(
                mech_core::OperationContractError::EffectOutputUnsupported { outputs: 1 }
            )
        ))
    ));

    let build = one_output_bytecode_contract(
        AliasPolicy::NoAlias,
        OutputConstruction::Build {
            postcondition: ShapeContractReference {
                module_path: vec!["matrixx".to_owned()].into_boxed_slice(),
                contract_name: "logical-mask-output".to_owned(),
            },
        },
        ExternalInteraction::Pure,
    );
    let invalid_reference = decode_with_mutated_operation_contract(&data, build, |bytes| {
        let start = bytes
            .windows(b"matrixx".len())
            .position(|window| window == b"matrixx")
            .unwrap();
        bytes[start..start + b"mat/ixx".len()].copy_from_slice(b"mat/ixx");
    });
    assert!(matches!(
        invalid_reference,
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::OperationContract(
                mech_core::OperationContractError::InvalidShapeContractReference { .. }
            )
        ))
    ));
}

#[test]
fn decoded_artifact_sections_revalidate_structure_and_limits() {
    let data = fixture_data();
    let mut context = ArtifactBuildContext::new(&data.schemas, &data.constants);
    let artifact = compile_source_program(&stateful_register(&data), &mut context).unwrap();
    let sections = encode_program_artifact_sections(&artifact).unwrap();

    let mut missing_binding = sections.clone();
    missing_binding.bindings = b"[]".to_vec();
    let mut nodes: serde_json::Value = serde_json::from_slice(&missing_binding.nodes).unwrap();
    for node in nodes["nodes"].as_array_mut().unwrap() {
        node["input_start"] = serde_json::Value::from(0);
        node["input_end"] = serde_json::Value::from(0);
        node["output_start"] = serde_json::Value::from(0);
        node["output_end"] = serde_json::Value::from(0);
    }
    missing_binding.nodes = serde_json::to_vec(&nodes).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&missing_binding),
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::OperationContract(
                mech_core::OperationContractError::PortCountMismatch { .. }
            )
        ))
    ));

    let mut mismatch = sections.clone();
    let mut slots: serde_json::Value = serde_json::from_slice(&mismatch.slots).unwrap();
    let state = slots
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|slot| slot["role"] == 2)
        .unwrap();
    state["initializer"] = serde_json::Value::from(data.constant.false_.get());
    mismatch.slots = serde_json::to_vec(&slots).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&mismatch),
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::InitializerSchemaMismatch { .. }
        ))
    ));

    let mut empty_module = sections.clone();
    let mut operations: serde_json::Value =
        serde_json::from_slice(&empty_module.operations).unwrap();
    operations.as_array_mut().unwrap()[0]["module_path"] = serde_json::json!([]);
    empty_module.operations = serde_json::to_vec(&operations).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&empty_module),
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::InvalidOperationReference { .. }
        ))
    ));

    let mut unused_operation = sections.clone();
    let mut operations: serde_json::Value =
        serde_json::from_slice(&unused_operation.operations).unwrap();
    operations.as_array_mut().unwrap().push(serde_json::json!({
        "module_path": ["unused"],
        "operation_name": "operation"
    }));
    unused_operation.operations = serde_json::to_vec(&operations).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&unused_operation),
        Err(ArtifactBytecodeError::NonCanonicalOperationTable)
    ));

    let mut duplicate_operation = sections.clone();
    let mut operations: serde_json::Value =
        serde_json::from_slice(&duplicate_operation.operations).unwrap();
    let duplicate = operations.as_array().unwrap()[0].clone();
    operations.as_array_mut().unwrap().push(duplicate);
    duplicate_operation.operations = serde_json::to_vec(&operations).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&duplicate_operation),
        Err(ArtifactBytecodeError::NonCanonicalOperationTable)
    ));

    let mut unknown_contract = sections.clone();
    let mut nodes: serde_json::Value = serde_json::from_slice(&unknown_contract.nodes).unwrap();
    nodes["nodes"].as_array_mut().unwrap()[0]["contract"] = serde_json::Value::from(u32::MAX);
    unknown_contract.nodes = serde_json::to_vec(&nodes).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&unknown_contract),
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::UnknownOperationContract { .. }
        ))
    ));

    let mut malformed_contract = sections.clone();
    malformed_contract.operation_contracts[9] = u8::MAX;
    assert!(matches!(
        decode_program_artifact_sections(&malformed_contract),
        Err(ArtifactBytecodeError::Artifact(
            ArtifactBuildError::OperationContract(
                mech_core::OperationContractError::InvalidCanonicalEncoding { .. }
            )
        ))
    ));

    let (multi_operation, _) = build_both(&data, ekf(&data));
    let mut reordered_operation = encode_program_artifact_sections(&multi_operation).unwrap();
    let mut operations: serde_json::Value =
        serde_json::from_slice(&reordered_operation.operations).unwrap();
    operations.as_array_mut().unwrap().swap(0, 1);
    reordered_operation.operations = serde_json::to_vec(&operations).unwrap();
    let mut nodes: serde_json::Value = serde_json::from_slice(&reordered_operation.nodes).unwrap();
    for node in nodes["nodes"].as_array_mut().unwrap() {
        node["operation"] = match node["operation"].as_u64().unwrap() {
            0 => serde_json::Value::from(1),
            1 => serde_json::Value::from(0),
            operation => serde_json::Value::from(operation),
        };
    }
    reordered_operation.nodes = serde_json::to_vec(&nodes).unwrap();
    assert!(matches!(
        decode_program_artifact_sections(&reordered_operation),
        Err(ArtifactBytecodeError::NonCanonicalOperationTable)
    ));

    assert!(matches!(
        decode_program_artifact_sections_with_limits(
            &sections,
            ArtifactDecodeLimits {
                max_constants: 0,
                ..ArtifactDecodeLimits::default()
            }
        ),
        Err(ArtifactBytecodeError::SectionItemLimit {
            section: "constants",
            ..
        })
    ));
    assert!(matches!(
        decode_program_artifact_sections_with_limits(
            &sections,
            ArtifactDecodeLimits {
                max_contracts: 0,
                ..ArtifactDecodeLimits::default()
            }
        ),
        Err(ArtifactBytecodeError::SectionItemLimit {
            section: "operation contracts",
            ..
        })
    ));
    assert!(matches!(
        decode_program_artifact_sections_with_limits(
            &sections,
            ArtifactDecodeLimits {
                max_section_bytes: 1,
                ..ArtifactDecodeLimits::default()
            }
        ),
        Err(ArtifactBytecodeError::SectionByteLimit { .. })
    ));
}

#[test]
fn program_revision_changes_with_semantic_graph_order() {
    let data = fixture_data();
    let (forward, _) = build_both(&data, scalar_add(&data));
    let mut reversed = scalar_add(&data);
    reversed.nodes[0].inputs.swap(0, 1);
    let (reversed, _) = build_both(&data, reversed);
    assert_ne!(forward.revision(), reversed.revision());
}

#[test]
fn compute_regions_are_intrinsic_revision_bearing_artifact_declarations() {
    let data = fixture_data();
    let (ordinary, _) = build_both(&data, scalar_add(&data));
    let region = ComputeRegionDeclaration {
        id: mech_core::ComputeRegionId::new(0),
        name: "scalar-add".into(),
        placement: mech_core::ComputePlacement::Compute,
        nodes: ordinary
            .nodes()
            .iter()
            .map(|node| node.node)
            .collect::<Vec<_>>()
            .into_boxed_slice(),
    };
    let with_region = ProgramArtifactDraft {
        schemas: ordinary.schemas().clone(),
        constants: ordinary.constants().clone(),
        contracts: ordinary.contracts().clone(),
        requirements: ordinary.requirements().clone(),
        inputs: ordinary.inputs().to_vec().into_boxed_slice(),
        slots: ordinary.slots().to_vec().into_boxed_slice(),
        nodes: ordinary.nodes().to_vec().into_boxed_slice(),
        bindings: ordinary.bindings().to_vec().into_boxed_slice(),
        outputs: ordinary.outputs().to_vec().into_boxed_slice(),
        constraints: ordinary.constraints().to_vec().into_boxed_slice(),
        compute_regions: vec![region].into_boxed_slice(),
    }
    .finalize()
    .unwrap();

    assert_ne!(ordinary.revision(), with_region.revision());
    assert!(
        encode_program_artifact_sections(&ordinary)
            .unwrap()
            .compute_regions
            .is_empty()
    );
    let sections = encode_program_artifact_sections(&with_region).unwrap();
    assert!(!sections.compute_regions.is_empty());
    let decoded = decode_program_artifact_sections(&sections).unwrap();
    assert_eq!(decoded.revision(), with_region.revision());
    assert_eq!(decoded.compute_regions(), with_region.compute_regions());
}

#[test]
fn external_requirements_are_artifact_authority_and_round_trip_in_bytecode_v1() {
    let data = fixture_data();
    let requirements = ApplicationRequirementTable::from_canonical_entries(vec![
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "gate-d3://input/value".to_owned(),
            path: "sample".to_owned(),
            context_name: "value".to_owned(),
            operation: "read".to_owned(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }),
        ApplicationRequirement::Resource(ExecutionResourceRequest {
            base_uri: "gate-d3://scene/output".to_owned(),
            path: "frame".to_owned(),
            context_name: "output".to_owned(),
            operation: "write".to_owned(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
        }),
    ])
    .unwrap();
    let graph = SourceProgram {
        requirements,
        nodes: vec![
            SourceNode {
                operation: operation("resource", "read"),
                requirement: Some(ApplicationRequirementId::new(0)),
                inputs: Box::new([]),
                outputs: vec![SourceNodeOutput::Derived {
                    schema: data.schema.f64_,
                }]
                .into_boxed_slice(),
            },
            SourceNode {
                operation: operation("resource", "write"),
                requirement: Some(ApplicationRequirementId::new(1)),
                inputs: vec![SourceValue::NodeOutput {
                    node: 0,
                    output_ordinal: 0,
                }]
                .into_boxed_slice(),
                outputs: Box::new([]),
            },
        ]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "output".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: data.schema.f64_,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    let observation = Box::leak(Box::new(OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(Box::new([])),
        outputs: vec![OutputPortPolicy {
            access: AccessMode::Write,
            delivery: DeliveryMode::Signal,
            construction: OutputConstruction::FullWrite {
                shape: ShapeRule::Declared,
            },
            alias: AliasPolicy::NoAlias,
            change_detection: ChangeDetectionPolicy::AlwaysChanged,
        }]
        .into_boxed_slice(),
        interaction: ExternalInteraction::Observation(ObservationContract {
            replay: ObservationReplayPolicy::CaptureAsInputFact,
        }),
    }));
    let effect = Box::leak(Box::new(OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery: EffectDeliveryPolicy::IdempotentRetry,
            idempotency: IdempotencyRequirement::Required,
        }),
    }));
    let artifact = compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&data.schemas, &data.constants),
        &[Some(observation), Some(effect)],
    )
    .unwrap();
    assert_eq!(artifact.requirements(), &graph.requirements);
    assert_eq!(
        artifact.nodes()[0].requirement,
        Some(ApplicationRequirementId::new(0))
    );
    assert_eq!(
        artifact.nodes()[1].requirement,
        Some(ApplicationRequirementId::new(1))
    );
    assert!(artifact.nodes()[1].output_bindings.is_empty());
    assert!(!artifact.slots().iter().any(|slot| matches!(
        slot.producer,
        ProducerReference::NodeOutput { node, .. } if node == NodeId::new(1)
    )));

    let decoded = decode_program_artifact_bytecode_v1(
        &encode_program_artifact_bytecode_v1(&artifact).unwrap(),
    )
    .unwrap();
    assert_eq!(decoded.revision(), artifact.revision());
    assert_eq!(decoded.requirements(), artifact.requirements());
    assert_eq!(decoded.nodes(), artifact.nodes());

    let sections = encode_program_artifact_sections(&artifact).unwrap();
    let missing_outer = write_bytecode_with_artifact(
        &BytecodeProgram {
            register_count: 0,
            constants: Vec::new(),
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: Vec::new(),
            dictionary: BTreeMap::new(),
            requirements: Vec::new(),
        },
        &sections,
    )
    .unwrap();
    assert!(matches!(
        decode_program_artifact_bytecode_v1(&missing_outer),
        Err(ArtifactBytecodeError::RequirementTableMismatch)
    ));
}

#[test]
fn artifact_requirement_table_rejects_noncanonical_resource_syntax() {
    assert!(
        ApplicationRequirementTable::from_canonical_entries(vec![
            ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri: "gate-d3://scene/output/".to_owned(),
                path: "frame/../latest".to_owned(),
                context_name: "output".to_owned(),
                operation: "read".to_owned(),
                intent: ResourceIntent::Send,
                delivery: ResourceDelivery::Live,
            })
        ])
        .is_err()
    );
}

fn artifact_with_declaration(
    data: &FixtureData,
    declaration: OperationContractDeclaration,
) -> ProgramArtifact {
    let mut graph = scalar_add(data);
    if matches!(
        declaration.interaction,
        ExternalInteraction::TransactionalExternal(_)
    ) {
        graph.requirements = ApplicationRequirementTable::from_canonical_entries(vec![
            ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri: "test://transactional".to_owned(),
                path: "value".to_owned(),
                context_name: "transactional".to_owned(),
                operation: "write".to_owned(),
                intent: ResourceIntent::Assign,
                delivery: ResourceDelivery::Snapshot,
            }),
        ])
        .unwrap();
        graph.nodes[0].requirement = Some(ApplicationRequirementId::new(0));
    }
    let declaration = Box::leak(Box::new(declaration));
    compile_source_program_with_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&data.schemas, &data.constants),
        &[Some(declaration)],
    )
    .unwrap()
}

#[test]
fn program_revision_commits_to_every_operation_contract_semantic() {
    let data = fixture_data();
    let mut baseline_contract = pure_full_write_contract(2, 1);
    baseline_contract.outputs[0].change_detection = ChangeDetectionPolicy::ExactScalar;
    let baseline = artifact_with_declaration(&data, baseline_contract.clone()).revision();

    let mut consume = baseline_contract.clone();
    let InputPortLayout::Fixed(inputs) = &mut consume.inputs else {
        unreachable!()
    };
    inputs[0].access = AccessMode::Consume;
    assert_ne!(
        baseline,
        artifact_with_declaration(&data, consume).revision()
    );

    let mut read_modify_write = baseline_contract.clone();
    read_modify_write.outputs[0].access = AccessMode::ReadWrite;
    read_modify_write.outputs[0].construction = OutputConstruction::ReadModifyWrite {
        base_input: 0,
        regions: RegionPolicy::SingleElement,
    };
    assert_ne!(
        baseline,
        artifact_with_declaration(&data, read_modify_write).revision()
    );

    let mut transactional = baseline_contract.clone();
    transactional.interaction =
        ExternalInteraction::TransactionalExternal(mech_core::TransactionalExternalContract {
            protocol: mech_core::TransactionalEffectProtocol::PrepareCommit,
        });
    assert_ne!(
        baseline,
        artifact_with_declaration(&data, transactional).revision()
    );

    let mut alias = baseline_contract.clone();
    alias.outputs[0].alias = AliasPolicy::MayAlias { input: 0 };
    assert_ne!(baseline, artifact_with_declaration(&data, alias).revision());

    let mut always_changed = baseline_contract.clone();
    always_changed.outputs[0].change_detection = ChangeDetectionPolicy::AlwaysChanged;
    assert_ne!(
        baseline,
        artifact_with_declaration(&data, always_changed).revision()
    );

    let mut effect_node = node(
        operation("resource", "write"),
        vec![SourceValue::Constant(data.constant.one)],
        Vec::new(),
    );
    effect_node.requirement = Some(ApplicationRequirementId::new(0));
    let effect_source = SourceProgram {
        requirements: ApplicationRequirementTable::from_canonical_entries(vec![
            ApplicationRequirement::Resource(ExecutionResourceRequest {
                base_uri: "test://effect".to_owned(),
                path: "value".to_owned(),
                context_name: "effect".to_owned(),
                operation: "write".to_owned(),
                intent: ResourceIntent::Send,
                delivery: ResourceDelivery::Snapshot,
            }),
        ])
        .unwrap(),
        nodes: vec![effect_node].into_boxed_slice(),
        ..SourceProgram::default()
    };
    let effect_contract = |delivery| OperationContractDeclaration {
        inputs: InputPortLayout::Fixed(
            vec![InputPortPolicy {
                access: AccessMode::Read,
                delivery: DeliveryMode::Signal,
            }]
            .into_boxed_slice(),
        ),
        outputs: Box::new([]),
        interaction: ExternalInteraction::Effect(EffectContract {
            delivery,
            idempotency: IdempotencyRequirement::Optional,
        }),
    };
    let provider_defined = Box::leak(Box::new(effect_contract(
        EffectDeliveryPolicy::ProviderDefined,
    )));
    let provider_defined = compile_source_program_with_contracts(
        &effect_source,
        &mut ArtifactBuildContext::new(&data.schemas, &data.constants),
        &[Some(provider_defined)],
    )
    .unwrap()
    .revision();
    let at_most_once = Box::leak(Box::new(effect_contract(EffectDeliveryPolicy::AtMostOnce)));
    let at_most_once = compile_source_program_with_contracts(
        &effect_source,
        &mut ArtifactBuildContext::new(&data.schemas, &data.constants),
        &[Some(at_most_once)],
    )
    .unwrap()
    .revision();
    assert_ne!(provider_defined, at_most_once);

    let mut first_shape = baseline_contract.clone();
    first_shape.outputs[0].construction = OutputConstruction::Build {
        postcondition: ShapeContractReference {
            module_path: vec!["matrix".to_owned(), "selection".to_owned()].into_boxed_slice(),
            contract_name: "logical-mask-output".to_owned(),
        },
    };
    let mut second_shape = first_shape.clone();
    let OutputConstruction::Build { postcondition } = &mut second_shape.outputs[0].construction
    else {
        unreachable!()
    };
    postcondition.contract_name = "index-selection-output".to_owned();
    assert_ne!(
        artifact_with_declaration(&data, first_shape).revision(),
        artifact_with_declaration(&data, second_shape).revision()
    );
}

#[test]
fn contract_insertion_order_does_not_change_program_revision() {
    let data = fixture_data();
    let mut exact = pure_full_write_contract(2, 1);
    exact.outputs[0].change_detection = ChangeDetectionPolicy::ExactScalar;
    let mut always = exact.clone();
    always.outputs[0].change_detection = ChangeDetectionPolicy::AlwaysChanged;
    let exact = artifact_with_declaration(&data, exact)
        .contracts()
        .iter()
        .next()
        .unwrap()
        .clone();
    let always = artifact_with_declaration(&data, always)
        .contracts()
        .iter()
        .next()
        .unwrap()
        .clone();

    let mut first_builder = OperationContractTableBuilder::new();
    let first_exact = first_builder.insert(exact.clone()).unwrap();
    first_builder.insert(always.clone()).unwrap();
    let first = first_builder.finish().unwrap();

    let mut second_builder = OperationContractTableBuilder::new();
    second_builder.insert(always).unwrap();
    let second_exact = second_builder.insert(exact).unwrap();
    let second = second_builder.finish().unwrap();

    let base = build_both(&data, scalar_add(&data)).0;
    let make_draft = |contracts: OperationContractTable, contract: OperationContractId| {
        let mut nodes = base.nodes().to_vec();
        nodes[0].contract = contract;
        ProgramArtifactDraft {
            schemas: base.schemas().clone(),
            constants: base.constants().clone(),
            contracts,
            requirements: base.requirements().clone(),
            inputs: base.inputs().to_vec().into_boxed_slice(),
            slots: base.slots().to_vec().into_boxed_slice(),
            nodes: nodes.into_boxed_slice(),
            bindings: base.bindings().to_vec().into_boxed_slice(),
            outputs: base.outputs().to_vec().into_boxed_slice(),
            constraints: base.constraints().to_vec().into_boxed_slice(),
            compute_regions: base.compute_regions().to_vec().into_boxed_slice(),
        }
    };
    let first_id = first.resolve(first_exact).unwrap();
    let second_id = second.resolve(second_exact).unwrap();
    let first = make_draft(first.table, first_id).finalize().unwrap();
    let second = make_draft(second.table, second_id).finalize().unwrap();
    assert_eq!(first.contracts(), second.contracts());
    assert_eq!(first.revision(), second.revision());
}

#[test]
fn bytecode_v1_round_trips_every_c2_snapshot_family() {
    let atom_path =
        CanonicalNominalPath::new(vec!["test".to_owned(), "atom".to_owned()].into_boxed_slice())
            .unwrap();
    let enum_path =
        CanonicalNominalPath::new(vec!["test".to_owned(), "enum".to_owned()].into_boxed_slice())
            .unwrap();
    let atom_key = NominalKey::from_path(NominalKind::Atom, &atom_path);
    let enum_key = NominalKey::from_path(NominalKind::Enum, &enum_path);

    let mut builder = SchemaTableBuilder::new();
    let mut handles = BTreeMap::<&'static str, SchemaHandle>::new();
    let mut insert = |name, body| {
        handles.insert(name, builder.insert(schema(body)).unwrap());
    };
    insert("u8", SchemaBody::UnsignedInteger(IntegerWidth::W8));
    insert("u16", SchemaBody::UnsignedInteger(IntegerWidth::W16));
    insert("u32", SchemaBody::UnsignedInteger(IntegerWidth::W32));
    insert("u64", SchemaBody::UnsignedInteger(IntegerWidth::W64));
    insert("u128", SchemaBody::UnsignedInteger(IntegerWidth::W128));
    insert("i8", SchemaBody::SignedInteger(IntegerWidth::W8));
    insert("i16", SchemaBody::SignedInteger(IntegerWidth::W16));
    insert("i32", SchemaBody::SignedInteger(IntegerWidth::W32));
    insert("i64", SchemaBody::SignedInteger(IntegerWidth::W64));
    insert("i128", SchemaBody::SignedInteger(IntegerWidth::W128));
    insert("f32", SchemaBody::FloatingPoint(FloatWidth::W32));
    insert("f64", SchemaBody::FloatingPoint(FloatWidth::W64));
    insert("c32", SchemaBody::Complex(FloatWidth::W32));
    insert("c64", SchemaBody::Complex(FloatWidth::W64));
    insert("r64", SchemaBody::Rational64);
    insert("bool", SchemaBody::Bool);
    insert("string", SchemaBody::String);
    insert("id", SchemaBody::Id);
    insert("index", SchemaBody::Index);
    insert("atom", SchemaBody::Atom(atom_key));
    insert(
        "enum",
        SchemaBody::Enum {
            key: enum_key,
            variants: vec![mech_core::EnumVariantSchema {
                name: "payload".to_owned(),
                payload: Some(SchemaBody::FloatingPoint(FloatWidth::W64)),
            }]
            .into_boxed_slice(),
        },
    );
    insert(
        "option",
        SchemaBody::Option(Box::new(SchemaBody::FloatingPoint(FloatWidth::W64))),
    );
    insert(
        "tuple",
        SchemaBody::Tuple(vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice()),
    );
    insert(
        "record",
        SchemaBody::Record(
            vec![
                SchemaField {
                    name: "value".to_owned(),
                    schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                },
                SchemaField {
                    name: "valid".to_owned(),
                    schema: SchemaBody::Bool,
                },
            ]
            .into_boxed_slice(),
        ),
    );
    insert(
        "table",
        SchemaBody::Table {
            columns: vec![
                SchemaField {
                    name: "number".to_owned(),
                    schema: SchemaBody::FloatingPoint(FloatWidth::W64),
                },
                SchemaField {
                    name: "flag".to_owned(),
                    schema: SchemaBody::Bool,
                },
            ]
            .into_boxed_slice(),
            rows: DimensionExpr::Constant(2),
        },
    );
    insert(
        "set",
        SchemaBody::Set {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            cardinality: DimensionExpr::Constant(2),
        },
    );
    insert(
        "map",
        SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::Option(Box::new(SchemaBody::FloatingPoint(
                FloatWidth::W64,
            )))),
            cardinality: DimensionExpr::Constant(1),
        },
    );
    insert("type", SchemaBody::ReifiedType);
    drop(insert);
    let matrix_handle = builder
        .insert(
            SchemaDraft {
                dimension_parameters: vec![DimensionParameterDeclaration {
                    id: DimensionParameterId::new(0),
                    origin: DimensionParameterOrigin::Explicit,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(1),
                    upper_bound: Some(DimensionExpr::Constant(4)),
                }]
                .into_boxed_slice(),
                body: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Tuple(
                        vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
                    )),
                    dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                        .into_boxed_slice(),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    handles.insert("matrix", matrix_handle);

    let build = builder.finish().unwrap();
    let ids = handles
        .into_iter()
        .map(|(name, handle)| (name, build.resolve(handle).unwrap()))
        .collect::<BTreeMap<_, _>>();
    let (schemas, _) = build.into_parts();
    let id = |name| ids[name];
    let f64 = |value| ValueDataDraft::F64(F64Bits::from_f64(value));
    let tuple = |flag, text: &str| {
        ValueDataDraft::Tuple(
            vec![
                ValueDataDraft::Bool(flag),
                ValueDataDraft::String(text.to_owned()),
            ]
            .into_boxed_slice(),
        )
    };
    let drafts: Vec<(&str, Box<[u64]>, ValueDataDraft)> = vec![
        ("u8", Box::new([]), ValueDataDraft::U8(1)),
        ("u16", Box::new([]), ValueDataDraft::U16(2)),
        ("u32", Box::new([]), ValueDataDraft::U32(3)),
        ("u64", Box::new([]), ValueDataDraft::U64(4)),
        ("u128", Box::new([]), ValueDataDraft::U128(5)),
        ("i8", Box::new([]), ValueDataDraft::I8(-1)),
        ("i16", Box::new([]), ValueDataDraft::I16(-2)),
        ("i32", Box::new([]), ValueDataDraft::I32(-3)),
        ("i64", Box::new([]), ValueDataDraft::I64(-4)),
        ("i128", Box::new([]), ValueDataDraft::I128(-5)),
        (
            "f32",
            Box::new([]),
            ValueDataDraft::F32(F32Bits::from_f32(1.25)),
        ),
        ("f64", Box::new([]), f64(2.5)),
        (
            "c32",
            Box::new([]),
            ValueDataDraft::Complex32(Complex32Bits::new(
                F32Bits::from_f32(1.0),
                F32Bits::from_f32(-1.0),
            )),
        ),
        (
            "c64",
            Box::new([]),
            ValueDataDraft::Complex64(Complex64Bits::new(
                F64Bits::from_f64(2.0),
                F64Bits::from_f64(-2.0),
            )),
        ),
        (
            "r64",
            Box::new([]),
            ValueDataDraft::Rational64 {
                numerator: 2,
                denominator: 3,
            },
        ),
        ("bool", Box::new([]), ValueDataDraft::Bool(true)),
        (
            "string",
            Box::new([]),
            ValueDataDraft::String("hello".to_owned()),
        ),
        ("id", Box::new([]), ValueDataDraft::Id(7)),
        ("index", Box::new([]), ValueDataDraft::Index(8)),
        ("atom", Box::new([]), ValueDataDraft::Atom),
        (
            "enum",
            Box::new([]),
            ValueDataDraft::Enum(EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(f64(9.0))),
            }),
        ),
        (
            "option",
            Box::new([]),
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(f64(10.0))),
            }),
        ),
        ("tuple", Box::new([]), tuple(true, "tuple")),
        (
            "record",
            Box::new([]),
            ValueDataDraft::Record(
                vec![
                    NamedValueDraft {
                        name: "value".to_owned(),
                        value: f64(11.0),
                    },
                    NamedValueDraft {
                        name: "valid".to_owned(),
                        value: ValueDataDraft::Bool(true),
                    },
                ]
                .into_boxed_slice(),
            ),
        ),
        (
            "matrix",
            vec![2].into_boxed_slice(),
            ValueDataDraft::Matrix(vec![tuple(true, "a"), tuple(false, "b")].into_boxed_slice()),
        ),
        (
            "table",
            Box::new([]),
            ValueDataDraft::Table(
                vec![
                    TableColumnDraft {
                        name: "number".to_owned(),
                        values: vec![f64(1.0), f64(2.0)].into_boxed_slice(),
                    },
                    TableColumnDraft {
                        name: "flag".to_owned(),
                        values: vec![ValueDataDraft::Bool(true), ValueDataDraft::Bool(false)]
                            .into_boxed_slice(),
                    },
                ]
                .into_boxed_slice(),
            ),
        ),
        (
            "set",
            Box::new([]),
            ValueDataDraft::Set(
                vec![ValueDataDraft::U8(1), ValueDataDraft::U8(2)].into_boxed_slice(),
            ),
        ),
        (
            "map",
            Box::new([]),
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![
                        ValueDataDraft::String("key".to_owned()),
                        ValueDataDraft::Option(OptionDraft {
                            present: true,
                            value: Some(Box::new(f64(12.0))),
                        }),
                    ]
                    .into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "type",
            Box::new([]),
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(
                schemas.entry(id("record")).unwrap().key(),
            )),
        ),
        (
            "type",
            Box::new([]),
            ValueDataDraft::Type(ReifiedTypeDraft::Kind {
                kind: KindExpr::Tuple(vec![KindExpr::Id, KindExpr::Index].into_boxed_slice()),
                dimension_parameters: Box::new([]),
            }),
        ),
    ];

    let validation = SnapshotValidationContext::new(&schemas);
    let mut constants = ConstantStoreBuilder::new(&schemas);
    for (schema, shape_values, data) in drafts {
        let value = ValueDraft {
            schema: id(schema),
            shape_values,
            data,
        }
        .finalize(&validation)
        .unwrap();
        constants.insert(value).unwrap();
    }
    let constants = constants.finish().unwrap().into_parts().0;
    let artifact = ProgramArtifactDraft {
        schemas,
        constants,
        contracts: OperationContractTable::empty(),
        requirements: ApplicationRequirementTable::empty(),
        inputs: Box::new([]),
        slots: Box::new([]),
        nodes: Box::new([]),
        bindings: Box::new([]),
        outputs: Box::new([]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap();
    let bytes = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    let decoded = decode_program_artifact_bytecode_v1(&bytes).unwrap();

    assert_eq!(decoded.revision(), artifact.revision());
    assert_eq!(decoded.constants().len(), artifact.constants().len());
    for raw in 0..artifact.constants().len() {
        let constant = ConstantId::new(raw as u32);
        let left = artifact.constants().get(constant).unwrap();
        let right = decoded.constants().get(constant).unwrap();
        assert_eq!(left.schema_key(), right.schema_key());
        assert_eq!(
            left.value_hash(artifact.schemas()).unwrap(),
            right.value_hash(decoded.schemas()).unwrap()
        );
        assert_eq!(
            left.shape().parameter_values(),
            right.shape().parameter_values()
        );
    }
}
