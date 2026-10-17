//! Canonical control results retain their selected current Matrix geometry.
#![cfg(feature = "full_compiler")]

use mech_core::{
    ReactiveInstanceId, SchemaBody, Value, ValueDataDraft,
    snapshot::{NamedValueDraft, SnapshotValidationContext, ValueDraft},
};
use mech_engine::resident::{ActivationFacts, CapturedValueInput, ReactiveInstance, activate};
use mech_engine::{CanonicalSourceFrontend, ProgramArtifact, SourceDocumentOutputKind};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn compile(source: &str) -> ([ProgramArtifact; 2], usize) {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x43525348), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let program = CanonicalSourceFrontend
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    ([original, decoded], output)
}

fn instance(artifact: &ProgramArtifact, ordinal: usize) -> ReactiveInstance {
    activate(
        ReactiveInstanceId::new(0x43525348, ordinal as u32),
        artifact,
        &mech_stdlib::source_catalog(),
        &ActivationFacts::default(),
    )
    .unwrap()
}

fn input(
    artifact: &ProgramArtifact,
    name: &str,
    extents: Option<[u64; 2]>,
    data: ValueDataDraft,
) -> Value {
    let input = artifact
        .inputs()
        .iter()
        .find(|input| mech_engine::decode_source_input_name(&input.name).as_deref() == Some(name))
        .unwrap();
    let schema = artifact.schemas().get(input.schema).unwrap();
    let shape_values = extents
        .map(|extents| {
            mech_core::shape_for_resolved_extents(schema, &extents)
                .unwrap()
                .parameter_values()
                .to_vec()
                .into_boxed_slice()
        })
        .unwrap_or_default();
    ValueDraft {
        schema: input.schema,
        shape_values,
        data,
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap()
}

fn data(extents: [u64; 2], base: u64) -> ValueDataDraft {
    ValueDataDraft::Matrix(
        (0..extents[0] * extents[1])
            .map(|index| ValueDataDraft::U64(base + index))
            .collect(),
    )
}

fn matrix(artifact: &ProgramArtifact, name: &str, extents: [u64; 2], base: u64) -> Value {
    input(artifact, name, Some(extents), data(extents, base))
}

fn boolean(artifact: &ProgramArtifact, name: &str, value: bool) -> Value {
    input(artifact, name, None, ValueDataDraft::Bool(value))
}

fn turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    named: &[(&str, Value)],
) -> Result<(), String> {
    let captured = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let declaration = artifact
                .inputs()
                .iter()
                .find(|declaration| declaration.slot == input.artifact_slot)
                .unwrap();
            let name = mech_engine::decode_source_input_name(&declaration.name).unwrap();
            let value = &named.iter().find(|(key, _)| *key == name).unwrap().1;
            CapturedValueInput {
                slot: input.slot,
                value,
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&captured)
        .and_then(|prepared| prepared.publish())
        .map(|_| ())
        .map_err(|error| format!("{error:?}"))
}

fn assert_matrix_geometry(value: &Value, body: &SchemaBody, extents: [u64; 2]) {
    let SchemaBody::Matrix { dimensions, .. } = body else {
        panic!("expected Matrix schema: {body:?}")
    };
    assert_eq!(
        dimensions
            .iter()
            .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
            .collect::<Vec<_>>(),
        extents
    );
}

fn assert_matrix(value: &Value, extents: [u64; 2], base: u64) {
    let schemas = value.schemas().unwrap();
    assert_matrix_geometry(value, schemas.get(value.schema()).unwrap().body(), extents);
    assert_eq!(value.canonical_data_draft().unwrap(), data(extents, base));
}

#[test]
fn direct_match_selects_the_current_matrix_shape_including_empty_axes() {
    let source = "left := first<[u64]>\nright := second<[u64]>\n(flag<bool>? | true => left | false => right)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (left, right, selected) in [
            ([2, 3], [1, 1], true),
            ([1, 4], [0, 5], false),
            ([4, 0], [2, 2], true),
            ([1, 1], [3, 0], false),
            ([0, 0], [0, 2], true),
            ([2, 1], [1, 3], false),
        ] {
            turn(
                &mut instance,
                artifact,
                &[
                    ("first", matrix(artifact, "first", left, 10)),
                    ("second", matrix(artifact, "second", right, 40)),
                    ("flag", boolean(artifact, "flag", selected)),
                ],
            )
            .unwrap();
            let (expected, base) = if selected { (left, 10) } else { (right, 40) };
            assert_matrix(&instance.copied_output(output).unwrap(), expected, base);
            assert!(!instance.has_ready_continuation());
        }
    }
}

#[test]
fn nested_match_forwards_only_its_selected_current_matrix() {
    let source = "left := first<[u64]>\nright := second<[u64]>\n(outer<bool>? | true => (inner<bool>? | true => left | false => right) | false => right)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (left, right, outer, inner) in [
            ([2, 2], [1, 3], true, true),
            ([3, 1], [0, 4], true, false),
            ([2, 0], [4, 0], false, true),
            ([0, 3], [1, 2], true, true),
            ([1, 1], [2, 3], false, false),
        ] {
            turn(
                &mut instance,
                artifact,
                &[
                    ("first", matrix(artifact, "first", left, 100)),
                    ("second", matrix(artifact, "second", right, 200)),
                    ("outer", boolean(artifact, "outer", outer)),
                    ("inner", boolean(artifact, "inner", inner)),
                ],
            )
            .unwrap();
            let (expected, base) = if outer && inner {
                (left, 100)
            } else {
                (right, 200)
            };
            assert_matrix(&instance.copied_output(output).unwrap(), expected, base);
        }
    }
}

fn assert_composite(value: &Value, record: bool, extents: [u64; 2], mark: u64) {
    let schemas = value.schemas().unwrap();
    let body = schemas.get(value.schema()).unwrap().body();
    let expected = if record {
        let SchemaBody::Record(fields) = body else {
            panic!("record result")
        };
        let items = fields.iter().find(|field| field.name == "items").unwrap();
        assert_matrix_geometry(value, &items.schema, extents);
        ValueDataDraft::Record(
            vec![
                NamedValueDraft {
                    name: "items".to_owned(),
                    value: data(extents, 30),
                },
                NamedValueDraft {
                    name: "mark".to_owned(),
                    value: ValueDataDraft::U64(mark),
                },
            ]
            .into_boxed_slice(),
        )
    } else {
        let SchemaBody::Tuple(fields) = body else {
            panic!("tuple result")
        };
        assert_matrix_geometry(value, &fields[0], extents);
        ValueDataDraft::Tuple(vec![data(extents, 30), ValueDataDraft::U64(mark)].into_boxed_slice())
    };
    assert_eq!(value.canonical_data_draft().unwrap(), expected);
}

#[test]
fn tuple_and_record_match_results_keep_actual_matrix_child_geometry() {
    for (record, source) in [
        (
            false,
            "matrix := values<[u64]>\n(flag<bool>? | true => (matrix, 1u64) | false => (matrix, 2u64))\n",
        ),
        (
            true,
            "matrix := values<[u64]>\n(flag<bool>? | true => {items: matrix, mark: 1u64} | false => {items: matrix, mark: 2u64})\n",
        ),
    ] {
        let (artifacts, output) = compile(source);
        for (ordinal, artifact) in artifacts.iter().enumerate() {
            let mut instance = instance(artifact, ordinal);
            for (extents, selected) in [
                ([2, 3], true),
                ([0, 4], false),
                ([3, 0], true),
                ([1, 1], false),
            ] {
                turn(
                    &mut instance,
                    artifact,
                    &[
                        ("values", matrix(artifact, "values", extents, 30)),
                        ("flag", boolean(artifact, "flag", selected)),
                    ],
                )
                .unwrap();
                assert_composite(
                    &instance.copied_output(output).unwrap(),
                    record,
                    extents,
                    if selected { 1 } else { 2 },
                );
            }
        }
    }
}

#[test]
fn an_unselected_invalid_arm_is_lazy_and_selected_failure_preserves_publication() {
    let source = "matrix := values<[u64]>\n(flag<bool>? | true => (matrix, 7u64) | false => (matrix, 1u64 / divisor<u64>))\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        let named = |extents, flag, divisor| {
            vec![
                ("values", matrix(artifact, "values", extents, 30)),
                ("flag", boolean(artifact, "flag", flag)),
                (
                    "divisor",
                    input(artifact, "divisor", None, ValueDataDraft::U64(divisor)),
                ),
            ]
        };
        for extents in [[2, 2], [0, 3]] {
            turn(&mut instance, artifact, &named(extents, true, 0)).unwrap();
            assert_composite(&instance.copied_output(output).unwrap(), false, extents, 7);
        }
        turn(&mut instance, artifact, &named([1, 2], false, 2)).unwrap();
        assert_composite(&instance.copied_output(output).unwrap(), false, [1, 2], 0);
        let epoch = instance.published_epoch();
        let error = turn(&mut instance, artifact, &named([3, 0], false, 0)).unwrap_err();
        assert!(error.contains("Arithmetic"), "{error}");
        assert_eq!(instance.published_epoch(), epoch);
        assert_composite(&instance.copied_output(output).unwrap(), false, [1, 2], 0);
        turn(&mut instance, artifact, &named([3, 0], true, 0)).unwrap();
        assert_composite(&instance.copied_output(output).unwrap(), false, [3, 0], 7);
        turn(&mut instance, artifact, &named([1, 1], false, 1)).unwrap();
        assert_composite(&instance.copied_output(output).unwrap(), false, [1, 1], 1);
    }
}

#[test]
fn caller_fixed_snapshot_result_fact_rejects_mismatch_atomically_and_recovers() {
    let source = "left := first<[u64]>\nright := second<[u64]>\n(flag<bool>? | true => left | false => right)\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let probe = instance(artifact, ordinal);
        let write = probe
            .plan
            .steps
            .iter()
            .find_map(|step| match step {
                mech_engine::resident::ActivatedTurnStep::Match(control) => Some(control.write),
                _ => None,
            })
            .unwrap();
        let schema = artifact
            .schemas()
            .get(probe.plan.slots[write.slot.get() as usize].schema)
            .unwrap();
        let mut facts = ActivationFacts::default();
        facts.slot_shapes.insert(
            write.slot,
            mech_core::shape_for_resolved_extents(schema, &[1, 2]).unwrap(),
        );
        drop(probe);
        let mut instance = activate(
            ReactiveInstanceId::new(0x43524658, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &facts,
        )
        .unwrap();
        assert!(instance.plan.slots[write.slot.get() as usize].activation_fixed_shape);
        let named = |left, right, selected| {
            vec![
                ("first", matrix(artifact, "first", left, 10)),
                ("second", matrix(artifact, "second", right, 40)),
                ("flag", boolean(artifact, "flag", selected)),
            ]
        };
        turn(&mut instance, artifact, &named([1, 2], [3, 1], true)).unwrap();
        assert_matrix(&instance.copied_output(output).unwrap(), [1, 2], 10);
        let epoch = instance.published_epoch();
        for extents in [[2, 1], [0, 4], [3, 0]] {
            let error = turn(&mut instance, artifact, &named([1, 2], extents, false)).unwrap_err();
            assert!(error.contains("InvalidShape"), "{error}");
            assert_eq!(instance.published_epoch(), epoch);
            assert_matrix(&instance.copied_output(output).unwrap(), [1, 2], 10);
        }
        turn(&mut instance, artifact, &named([3, 3], [1, 2], false)).unwrap();
        assert_matrix(&instance.copied_output(output).unwrap(), [1, 2], 40);
    }
}

#[test]
fn managed_control_turn_denial_preserves_epoch_and_recovers() {
    let source = "matrix := values<[u64]>\n(flag<bool>? | true => (matrix, 1u64) | false => (matrix, 2u64))\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let budget = mech_core::ManagedMemoryBudget::new(1 << 24);
        let mut instance = mech_engine::resident::activate_with_options(
            ReactiveInstanceId::new(0x43524244, ordinal as u32),
            artifact,
            &mech_stdlib::source_catalog(),
            &ActivationFacts::default(),
            mech_engine::resident::ResidentActivationOptions {
                memory_budget: Some(budget.clone()),
                ..Default::default()
            },
        )
        .unwrap();
        let named = |extents, flag| {
            vec![
                ("values", matrix(artifact, "values", extents, 30)),
                ("flag", boolean(artifact, "flag", flag)),
            ]
        };
        turn(&mut instance, artifact, &named([1, 2], true)).unwrap();
        let epoch = instance.published_epoch();
        let occupied = budget
            .reserve_capacity(budget.limit_bytes() - budget.used_bytes() - 1024)
            .unwrap();
        let error = turn(&mut instance, artifact, &named([1, 32768], false)).unwrap_err();
        assert!(error.contains("MemoryRuntime"), "{error}");
        drop(occupied);
        assert_eq!(instance.published_epoch(), epoch);
        assert_composite(&instance.copied_output(output).unwrap(), false, [1, 2], 1);
        turn(&mut instance, artifact, &named([0, 3], false)).unwrap();
        assert_composite(&instance.copied_output(output).unwrap(), false, [0, 3], 2);
        drop(instance);
        assert_eq!(budget.used_bytes(), 0);
    }
}

// Additional ordinary Option/Enum result coverage. The current nominal Enum
// declaration frontend requires closed payload kinds. This case therefore
// proves active fixed Matrix payloads (including an empty axis); it does not
// claim source support for a nominal Enum carrying Turn dimension parameters.

#[test]
fn optional_matrix_results_preserve_current_present_shapes_and_real_absence() {
    let source = "wrap(value<[u64]>) => (result<[u64]?>) := result := value.\nabsent() => (result<[u64]?>) := result := _.\nmatrix := values<[u64]>\n(flag<bool>? | true => wrap(matrix) | false => absent())\n";
    let (artifacts, output) = compile(source);
    for (ordinal, artifact) in artifacts.iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (extents, selected) in [
            ([2, 3], true),
            ([1, 1], false),
            ([0, 4], true),
            ([3, 0], true),
            ([2, 2], false),
            ([1, 2], true),
        ] {
            turn(
                &mut instance,
                artifact,
                &[
                    ("values", matrix(artifact, "values", extents, 50)),
                    ("flag", boolean(artifact, "flag", selected)),
                ],
            )
            .unwrap();
            let value = instance.copied_output(output).unwrap();
            let schemas = value.schemas().unwrap();
            let SchemaBody::Option(payload) = schemas.get(value.schema()).unwrap().body() else {
                panic!("optional Matrix result")
            };
            assert_matrix_geometry(&value, payload, if selected { extents } else { [0, 0] });
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Option(mech_core::snapshot::OptionDraft {
                    present: selected,
                    value: selected.then(|| Box::new(data(extents, 50))),
                })
            );
        }
    }
}

#[test]
fn nominal_enum_matrix_payload_results_select_live_values_and_empty_fixed_geometry() {
    let source = "<event> := :full<[u64]:2,2> | :empty<[u64]:0,3>\nleft<event> := :full(values<[u64]:2,2>)\nright<event> := :empty(empty<[u64]:0,3>)\n(flag<bool>? | true => left | false => right)\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x4352454e), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let origin = mech_core::CanonicalNominalPath::new(vec![
        "control-result-tests".to_owned(),
        "matrix-enum".to_owned(),
    ])
    .unwrap();
    let program = CanonicalSourceFrontend
        .with_nominal_origin(origin)
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        )
        .unwrap_or_else(|error| panic!("{source}: {error:?}"));
    let output = program
        .document_outputs()
        .iter()
        .rev()
        .find(|output| output.kind == SourceDocumentOutputKind::Program)
        .unwrap()
        .output as usize;
    let original = program.compile_artifact().unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    for (ordinal, artifact) in [original, decoded].iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for (base, selected) in [(10, true), (20, false), (30, true), (40, false)] {
            turn(
                &mut instance,
                artifact,
                &[
                    ("values", matrix(artifact, "values", [2, 2], base)),
                    ("empty", matrix(artifact, "empty", [0, 3], 0)),
                    ("flag", boolean(artifact, "flag", selected)),
                ],
            )
            .unwrap();
            let value = instance.copied_output(output).unwrap();
            let schemas = value.schemas().unwrap();
            let SchemaBody::Enum { variants, .. } = schemas.get(value.schema()).unwrap().body()
            else {
                panic!("nominal Enum result")
            };
            assert_eq!(
                variants
                    .iter()
                    .map(|variant| variant.name.as_str())
                    .collect::<Vec<_>>(),
                ["full", "empty"]
            );
            let (variant, extents) = if selected { (0, [2, 2]) } else { (1, [0, 3]) };
            assert_matrix_geometry(&value, variants[variant].payload.as_ref().unwrap(), extents);
            assert_eq!(
                value.canonical_data_draft().unwrap(),
                ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                    ordinal: variant as u32,
                    payload: Some(Box::new(data(extents, if selected { base } else { 0 }))),
                })
            );
        }
    }
}

#[test]
fn ordinary_nominal_enum_declarations_keep_their_closed_payload_requirement() {
    let source = "<event> := :items<[u64]>\nvalue<event> := :items(values<[u64]>)\nvalue\n";
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x43524552), Revision(1), source).unwrap(),
        ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let origin = mech_core::CanonicalNominalPath::new(vec![
        "control-result-tests".to_owned(),
        "matrix-enum".to_owned(),
    ])
    .unwrap();
    let result = CanonicalSourceFrontend
        .with_nominal_origin(origin)
        .compile_document_with_catalog(
            &DocumentSyntax::cast(parsed.syntax()).unwrap(),
            mech_stdlib::source_catalog(),
        );
    let error = match result {
        Ok(_) => panic!("a nominal payload with Turn dimensions is not supported"),
        Err(error) => error,
    };
    assert_eq!(
        error.code,
        "source-semantics/enum-payload-requires-closed-kind"
    );
}

// The public typed graph boundary can express a dimensional Enum independently
// of the separate closed-only ordinary nominal declaration frontend.
#[test]
fn typed_source_graph_selects_turn_enum_constants_with_actual_matrix_payload_shapes() {
    use mech_core::{
        ConstantStoreBuilder, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
        DimensionParameterId, DimensionParameterOrigin, EnumVariantSchema, IntegerWidth,
        NominalKey, SchemaDraft, SchemaTableBuilder,
    };
    use mech_engine::{
        ArtifactBuildContext, ControlBlock, ControlBlockId, ControlMatchArm, ControlValue,
        MatchDeclaration, MatchPattern, SourceInput, SourceNode, SourceNodeBody, SourceNodeOutput,
        SourceOutput, SourceProgram, SourceValue, compile_source_program_with_control_contracts,
    };
    let mut schemas = SchemaTableBuilder::new();
    let boolean = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Bool,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let enumeration = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: (0..2)
                    .map(|axis| DimensionParameterDeclaration {
                        id: DimensionParameterId::new(axis),
                        origin: DimensionParameterOrigin::Explicit,
                        lifetime: DimensionLifetime::Turn,
                        lower_bound: DimensionExpr::Constant(0),
                        upper_bound: Some(DimensionExpr::Constant(8)),
                    })
                    .collect(),
                body: SchemaBody::Enum {
                    key: NominalKey::from_bytes([0x45; 32]),
                    variants: vec![EnumVariantSchema {
                        name: "items".to_owned(),
                        payload: Some(SchemaBody::Matrix {
                            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
                            dimensions: (0..2)
                                .map(|axis| {
                                    DimensionExpr::Parameter(DimensionParameterId::new(axis))
                                })
                                .collect(),
                        }),
                    }]
                    .into_boxed_slice(),
                },
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let built = schemas.finish().unwrap();
    let boolean = built.resolve(boolean).unwrap();
    let enumeration = built.resolve(enumeration).unwrap();
    let schemas = built.into_parts().0;
    let context = SnapshotValidationContext::new(&schemas);
    let flag = |value| {
        mech_core::snapshot::ValueDraft {
            schema: boolean,
            shape_values: Box::new([]),
            data: ValueDataDraft::Bool(value),
        }
        .finalize(&context)
        .unwrap()
    };
    let enum_value = |extents: [u64; 2], base| {
        mech_core::snapshot::ValueDraft {
            schema: enumeration,
            shape_values: extents.into(),
            data: ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(data(extents, base))),
            }),
        }
        .finalize(&context)
    };
    // The actual immutable constructor enforces this schema's upper bounds.
    assert!(enum_value([9, 1], 0).is_err());
    let mut constants = ConstantStoreBuilder::new(&schemas);
    let yes = constants.insert(flag(true)).unwrap();
    let no = constants.insert(flag(false)).unwrap();
    let first = constants.insert(enum_value([2, 3], 10).unwrap()).unwrap();
    let second = constants.insert(enum_value([0, 4], 40).unwrap()).unwrap();
    let built = constants.finish().unwrap();
    let yes = built.resolve(yes).unwrap();
    let no = built.resolve(no).unwrap();
    let first = built.resolve(first).unwrap();
    let second = built.resolve(second).unwrap();
    let constants = built.into_parts().0;
    let body = |id, value| ControlBlock {
        id: ControlBlockId(id),
        parameters: Box::new([]),
        operations: Box::new([]),
        yield_value: ControlValue::Constant(value),
    };
    let graph = SourceProgram {
        inputs: vec![SourceInput {
            name: "flag".to_owned(),
            schema: boolean,
        }]
        .into_boxed_slice(),
        nodes: vec![SourceNode {
            body: SourceNodeBody::Match(MatchDeclaration {
                scrutinee: 0,
                partial: false,
                captures: Box::new([]),
                arms: vec![
                    ControlMatchArm {
                        pattern: MatchPattern::Literal(yes),
                        guard: None,
                        body: body(0, first),
                    },
                    ControlMatchArm {
                        pattern: MatchPattern::Literal(no),
                        guard: None,
                        body: body(1, second),
                    },
                ]
                .into_boxed_slice(),
            }),
            inputs: vec![SourceValue::Input(0)].into_boxed_slice(),
            outputs: vec![SourceNodeOutput::Derived {
                schema: enumeration,
            }]
            .into_boxed_slice(),
        }]
        .into_boxed_slice(),
        outputs: vec![SourceOutput {
            name: "selected".to_owned(),
            interactive_symbol: None,
            source: SourceValue::NodeOutput {
                node: 0,
                output_ordinal: 0,
            },
            schema: enumeration,
        }]
        .into_boxed_slice(),
        ..SourceProgram::default()
    };
    let original = compile_source_program_with_control_contracts(
        &graph,
        &mut ArtifactBuildContext::new(&schemas, &constants),
        &[None],
    )
    .unwrap();
    let encoded = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&encoded).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        encoded,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    for (ordinal, artifact) in [original, decoded].iter().enumerate() {
        let mut instance = instance(artifact, ordinal);
        for selected in [true, false, true, false] {
            let input = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: Box::new([]),
                data: ValueDataDraft::Bool(selected),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let slot = instance.plan.inputs[0].slot;
            instance
                .prepare_turn_values(&[CapturedValueInput {
                    slot,
                    value: &input,
                }])
                .unwrap()
                .publish()
                .unwrap();
            let output = instance.copied_output(0).unwrap();
            let owner = output.schemas().unwrap();
            let SchemaBody::Enum { variants, .. } = owner.get(output.schema()).unwrap().body()
            else {
                panic!("Enum result")
            };
            let (extents, base) = if selected { ([2, 3], 10) } else { ([0, 4], 40) };
            assert_matrix_geometry(&output, variants[0].payload.as_ref().unwrap(), extents);
            assert_eq!(
                output.canonical_data_draft().unwrap(),
                ValueDataDraft::Enum(mech_core::snapshot::EnumDraft {
                    ordinal: 0,
                    payload: Some(Box::new(data(extents, base))),
                })
            );
        }
    }
}
