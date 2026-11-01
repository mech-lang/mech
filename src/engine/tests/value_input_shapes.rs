//! Public canonical Value input shape authority, without unchecked signal snapshots.
#![cfg(all(feature = "full_compiler", feature = "resident-artifact"))]

use mech_core::{
    ConstantStoreBuilder, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, FloatWidth, FunctionCatalogBuilder,
    IntegerWidth, ManagedMemoryBudget, ReactiveInstanceId, ResidentValueKind, Schema, SchemaBody,
    SchemaDraft, SchemaTableBuilder, Value, ValueDataDraft,
    snapshot::{F64Bits, SnapshotValidationContext, ValueDraft},
};
use mech_engine::{
    ArtifactBuildContext, CanonicalSourceFrontend, ProgramArtifact, SourceInput, SourceOutput,
    SourceProgram, SourceValue, compile_source_program_with_contracts,
    resident::{
        ActivationFacts, CapturedValueInput, ReactiveInstance, ResidentActivationOptions,
        ResidentExecutionError, activate_with_options,
    },
};
use mech_syntax::document::{
    AstNode, DocumentId, DocumentSyntax, ParseConfig, Revision, TextSnapshot,
    parse_canonical_document,
};

fn both(original: ProgramArtifact) -> [ProgramArtifact; 2] {
    let bytes = mech_engine::encode_program_artifact_bytecode_v1(&original).unwrap();
    let decoded = mech_engine::decode_program_artifact_bytecode_v1(&bytes).unwrap();
    assert_eq!(original.revision(), decoded.revision());
    assert_eq!(
        bytes,
        mech_engine::encode_program_artifact_bytecode_v1(&decoded).unwrap()
    );
    [original, decoded]
}

fn source() -> [ProgramArtifact; 2] {
    let parsed = parse_canonical_document(
        TextSnapshot::new(DocumentId(0x56494e50), Revision(1), "signal<[u64]>\n").unwrap(),
        ParseConfig::default(),
    );
    assert!(parsed.is_strictly_clean(), "{:?}", parsed.diagnostics);
    let document = DocumentSyntax::cast(parsed.syntax()).unwrap();
    both(
        CanonicalSourceFrontend
            .compile_document(&document)
            .unwrap()
            .compile_artifact()
            .unwrap(),
    )
}

fn matrix_schema(element: SchemaBody, mixed: bool) -> Schema {
    SchemaDraft {
        dimension_parameters: [
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: if mixed {
                    DimensionLifetime::Activation
                } else {
                    DimensionLifetime::Turn
                },
                lower_bound: DimensionExpr::Constant(if mixed { 1 } else { 0 }),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(1),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: mixed.then_some(DimensionExpr::Constant(4)),
            },
        ]
        .into(),
        body: SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: Box::new([
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
            ]),
        },
    }
    .finalize()
    .unwrap()
}

// The maintained typed graph builder expresses mixed lifetimes explicitly;
// these are distinct from the actual canonical-source case above.
fn typed(schema: Schema, count: usize) -> [ProgramArtifact; 2] {
    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas.insert(schema).unwrap();
    let built = schemas.finish().unwrap();
    let schema = built.resolve(handle).unwrap();
    let schemas = built.into_parts().0;
    let constants = ConstantStoreBuilder::new(&schemas)
        .finish()
        .unwrap()
        .into_parts()
        .0;
    let graph = SourceProgram {
        inputs: (0..count)
            .map(|i| SourceInput {
                name: format!("input{i}"),
                schema,
            })
            .collect(),
        outputs: (0..count)
            .map(|i| SourceOutput {
                name: format!("result{i}"),
                interactive_symbol: None,
                source: SourceValue::Input(i as u32),
                schema,
            })
            .collect(),
        ..SourceProgram::default()
    };
    both(
        compile_source_program_with_contracts(
            &graph,
            &mut ArtifactBuildContext::new(&schemas, &constants),
            &[],
        )
        .unwrap(),
    )
}

fn instance(
    artifact: &ProgramArtifact,
    facts: &ActivationFacts,
    budget: Option<ManagedMemoryBudget>,
) -> ReactiveInstance {
    let mut catalog = FunctionCatalogBuilder::new();
    mech_engine::install_intrinsic_resident(&mut catalog).unwrap();
    activate_with_options(
        ReactiveInstanceId::new(0x56494e50, 0),
        artifact,
        &catalog.build().unwrap(),
        facts,
        ResidentActivationOptions {
            memory_budget: budget,
            ..Default::default()
        },
    )
    .unwrap()
}

fn value(artifact: &ProgramArtifact, ordinal: usize, extents: [u64; 2]) -> Value {
    let schema = artifact.inputs()[ordinal].schema;
    let definition = artifact.schemas().get(schema).unwrap();
    let shape = mech_core::shape_for_resolved_extents(definition, &extents).unwrap();
    let SchemaBody::Matrix { element, .. } = definition.body() else {
        panic!("Matrix input");
    };
    ValueDraft {
        schema,
        shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
        data: ValueDataDraft::Matrix(
            (0..extents[0] * extents[1])
                .map(|n| match element.as_ref() {
                    SchemaBody::UnsignedInteger(IntegerWidth::W64) => ValueDataDraft::U64(n),
                    SchemaBody::FloatingPoint(FloatWidth::W64) => {
                        ValueDataDraft::F64(F64Bits::from_f64(n as f64))
                    }
                    _ => panic!("test input kind"),
                })
                .collect(),
        ),
    }
    .finalize(&SnapshotValidationContext::new(artifact.schemas()))
    .unwrap()
}

fn turn(
    instance: &mut ReactiveInstance,
    artifact: &ProgramArtifact,
    values: &[Value],
) -> Result<(), ResidentExecutionError> {
    let inputs = instance
        .plan
        .inputs
        .iter()
        .map(|input| {
            let ordinal = artifact
                .inputs()
                .iter()
                .position(|value| value.slot == input.artifact_slot)
                .unwrap();
            CapturedValueInput {
                slot: input.slot,
                value: &values[ordinal],
            }
        })
        .collect::<Vec<_>>();
    instance
        .prepare_turn_values(&inputs)
        .and_then(|prepared| prepared.publish())
        .map(|_| ())
}

fn shape(value: &Value) -> Vec<u64> {
    let owner = value.schemas().unwrap();
    let SchemaBody::Matrix { dimensions, .. } = owner.get(value.schema()).unwrap().body() else {
        panic!("Matrix output");
    };
    dimensions
        .iter()
        .map(|dimension| value.shape().resolve_dimension(dimension).unwrap())
        .collect()
}

fn prior(instance: &ReactiveInstance, count: usize) -> Vec<ValueDataDraft> {
    (0..count)
        .map(|i| {
            instance
                .copied_output(i)
                .unwrap()
                .canonical_data_draft()
                .unwrap()
        })
        .collect()
}

#[test]
fn canonical_source_host_values_keep_actual_turn_axes_original_and_decoded() {
    for artifact in source() {
        let mut instance = instance(&artifact, &ActivationFacts::default(), None);
        assert_eq!(
            instance.plan.inputs[0].region.kind,
            ResidentValueKind::Snapshot
        );
        for extents in [[1, 3], [3, 1], [2, 2], [0, 4], [4, 0], [0, 0], [1, 1]] {
            let input = value(&artifact, 0, extents);
            turn(&mut instance, &artifact, &[input.clone()]).unwrap();
            let output = instance.copied_output(0).unwrap();
            assert_eq!(shape(&output), extents);
            assert_eq!(
                output.canonical_data_draft().unwrap(),
                input.canonical_data_draft().unwrap()
            );
        }
    }
}

#[test]
fn mixed_lifetimes_allow_turn_changes_but_keep_activation_axes_and_bounds() {
    for artifact in typed(
        matrix_schema(SchemaBody::UnsignedInteger(IntegerWidth::W64), true),
        1,
    ) {
        let mut instance = instance(&artifact, &ActivationFacts::default(), None);
        for columns in [3, 0, 4, 1] {
            turn(
                &mut instance,
                &artifact,
                &[value(&artifact, 0, [1, columns])],
            )
            .unwrap();
            assert_eq!(shape(&instance.copied_output(0).unwrap()), [1, columns]);
        }
        let epoch = instance.published_epoch();
        let before = prior(&instance, 1);
        let error = turn(&mut instance, &artifact, &[value(&artifact, 0, [2, 1])]).unwrap_err();
        assert!(
            matches!(error, ResidentExecutionError::InputLayout { .. }),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(prior(&instance, 1), before);
        // Sealed Values cannot be constructed outside the schema's Turn bound.
        let schema = artifact.schemas().get(artifact.inputs()[0].schema).unwrap();
        assert!(mech_core::shape_for_resolved_extents(schema, &[1, 5]).is_err());
        turn(&mut instance, &artifact, &[value(&artifact, 0, [1, 2])]).unwrap();
        assert_eq!(shape(&instance.copied_output(0).unwrap()), [1, 2]);
    }
}

#[test]
fn caller_fixed_facts_keep_snapshot_and_dense_physical_layout_exact() {
    for element in [
        SchemaBody::UnsignedInteger(IntegerWidth::W64),
        SchemaBody::FloatingPoint(FloatWidth::W64),
    ] {
        for artifact in typed(matrix_schema(element.clone(), false), 1) {
            let mut facts = ActivationFacts::default();
            facts.slot_shapes.insert(
                artifact.inputs()[0].slot,
                value(&artifact, 0, [2, 2]).shape().clone(),
            );
            let mut instance = instance(&artifact, &facts, None);
            let input = &instance.plan.inputs[0];
            assert!(instance.plan.slots[input.artifact_slot.get() as usize].activation_fixed_shape);
            assert_eq!(
                input.region.kind,
                if matches!(element, SchemaBody::FloatingPoint(_)) {
                    ResidentValueKind::F64
                } else {
                    ResidentValueKind::Snapshot
                }
            );
            turn(&mut instance, &artifact, &[value(&artifact, 0, [2, 2])]).unwrap();
            let epoch = instance.published_epoch();
            let before = prior(&instance, 1);
            for extents in [[1, 4], [2, 1], [0, 2]] {
                let error =
                    turn(&mut instance, &artifact, &[value(&artifact, 0, extents)]).unwrap_err();
                assert!(
                    matches!(error, ResidentExecutionError::InputLayout { .. }),
                    "{error:?}"
                );
                assert_eq!(instance.published_epoch(), epoch);
                assert_eq!(prior(&instance, 1), before);
            }
            turn(&mut instance, &artifact, &[value(&artifact, 0, [2, 2])]).unwrap();
        }
    }
}

#[test]
fn wrong_schema_key_id_or_duplicate_later_input_rejects_before_any_batch_write() {
    for artifact in typed(
        matrix_schema(SchemaBody::UnsignedInteger(IntegerWidth::W64), false),
        2,
    ) {
        let mut instance = instance(&artifact, &ActivationFacts::default(), None);
        turn(
            &mut instance,
            &artifact,
            &[value(&artifact, 0, [1, 2]), value(&artifact, 1, [1, 2])],
        )
        .unwrap();
        let epoch = instance.published_epoch();
        let before = prior(&instance, 2);
        let source = value(&artifact, 1, [1, 3]);
        let mut owner = SchemaTableBuilder::new();
        owner
            .insert(
                SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::Bool,
                }
                .finalize()
                .unwrap(),
            )
            .unwrap();
        let retained = owner
            .insert(artifact.schemas().get(source.schema()).unwrap().clone())
            .unwrap();
        let owner = owner.finish().unwrap();
        let shifted = owner.resolve(retained).unwrap();
        let wrong_id = source
            .rebind(shifted, source.shape(), &owner.table)
            .unwrap();
        assert_ne!(wrong_id.schema(), artifact.inputs()[1].schema);
        assert_eq!(wrong_id.schema_key(), source.schema_key());
        let mut foreign = SchemaTableBuilder::new();
        let retained = foreign
            .insert(matrix_schema(
                SchemaBody::FloatingPoint(FloatWidth::W64),
                false,
            ))
            .unwrap();
        let foreign = foreign.finish().unwrap();
        let wrong_key = ValueDraft {
            schema: foreign.resolve(retained).unwrap(),
            shape_values: vec![1, 3].into_boxed_slice(),
            data: ValueDataDraft::Matrix(
                vec![ValueDataDraft::F64(F64Bits::from_f64(1.0)); 3].into_boxed_slice(),
            ),
        }
        .finalize(&SnapshotValidationContext::new(&foreign.table))
        .unwrap();
        assert_eq!(wrong_key.schema(), artifact.inputs()[1].schema);
        assert_ne!(wrong_key.schema_key(), source.schema_key());
        for invalid in [wrong_id, wrong_key] {
            let error = turn(
                &mut instance,
                &artifact,
                &[value(&artifact, 0, [1, 4]), invalid],
            )
            .unwrap_err();
            assert!(
                matches!(error, ResidentExecutionError::InputLayout { .. }),
                "{error:?}"
            );
            assert_eq!(instance.published_epoch(), epoch);
            assert_eq!(prior(&instance, 2), before);
        }
        let first = value(&artifact, 0, [1, 4]);
        let error = instance
            .prepare_turn_values(&[
                CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &first,
                },
                CapturedValueInput {
                    slot: instance.plan.inputs[0].slot,
                    value: &first,
                },
            ])
            .err()
            .unwrap();
        assert!(
            matches!(error, ResidentExecutionError::DuplicateInput { .. }),
            "{error:?}"
        );
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(prior(&instance, 2), before);
        turn(
            &mut instance,
            &artifact,
            &[value(&artifact, 0, [1, 1]), value(&artifact, 1, [1, 3])],
        )
        .unwrap();
        assert_eq!(shape(&instance.copied_output(0).unwrap()), [1, 1]);
        assert_eq!(shape(&instance.copied_output(1).unwrap()), [1, 3]);
    }
}

#[test]
fn managed_payload_denial_keeps_prior_publication_then_recovers() {
    for artifact in typed(
        matrix_schema(SchemaBody::UnsignedInteger(IntegerWidth::W64), false),
        1,
    ) {
        let budget = ManagedMemoryBudget::new(1 << 24);
        let mut instance = instance(&artifact, &ActivationFacts::default(), Some(budget.clone()));
        turn(&mut instance, &artifact, &[value(&artifact, 0, [1, 2])]).unwrap();
        let epoch = instance.published_epoch();
        let before = prior(&instance, 1);
        let occupied = budget
            .reserve_capacity(budget.limit_bytes() - budget.used_bytes() - 1024)
            .unwrap();
        let error = turn(&mut instance, &artifact, &[value(&artifact, 0, [1, 32768])]).unwrap_err();
        assert!(
            matches!(error, ResidentExecutionError::MemoryRuntime { .. }),
            "{error:?}"
        );
        drop(occupied);
        assert_eq!(instance.published_epoch(), epoch);
        assert_eq!(prior(&instance, 1), before);
        turn(&mut instance, &artifact, &[value(&artifact, 0, [1, 3])]).unwrap();
        assert_eq!(shape(&instance.copied_output(0).unwrap()), [1, 3]);
        drop(instance);
        assert_eq!(budget.used_bytes(), 0);
    }
}
