//! Ordinary annotation lowering, original/decoded closed folding and the SAME
//! maintained resident conversion owner. No separate set/sort implementation.
use super::shape_fact_tests::closed_collection_source;
use super::*;

fn variants(source: &str) -> [ProgramArtifact; 2] {
    let original = closed_collection_source(source);
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}

fn target(artifact: &ProgramArtifact) -> (NodeId, CellSlotId) {
    let source =
        forwarded_output_source(artifact, ArtifactSource::Slot(artifact.outputs()[0].source))
            .unwrap();
    let ArtifactSource::Slot(slot) = source else {
        panic!("ordinary annotation must remain a real convert/set producer");
    };
    let ProducerReference::NodeOutput { node, .. } = artifact.slots()[slot.get() as usize].producer
    else {
        panic!("conversion node output");
    };
    let reference = artifact.nodes()[node.get() as usize]
        .as_operation()
        .unwrap()
        .operation;
    assert_eq!(reference.canonical_name(), "convert/set");
    assert_eq!(
        closed_producer_disposition(reference),
        Some(ClosedProducerDisposition::Value)
    );
    assert!(closed_iterative_operand_producer(reference));
    assert!(closed_activation_producer_supported(reference));
    (node, slot)
}

fn catalog() -> FunctionCatalog {
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    builder.build().unwrap()
}

#[test]
fn closed_matrix_to_set_annotations_share_resident_conversion_schema_and_canonical_keys() {
    for source in [
        "converted<{f64}> := [3 1;3 2]\nconverted\n",
        "converted<{u64}> := [3u64 1u64 3u64]\nconverted\n",
        "converted<{string}> := [\"é\" \"é\" \"é\" \"🧑‍🚀\"]\nconverted\n",
        "converted<{string}> := [2u64 1u64 2u64]\nconverted\n",
        "converted<{u64}:2> := [3u64 1u64 3u64]\nconverted\n",
    ] {
        for artifact in variants(source) {
            let (node, slot) = target(&artifact);
            let inputs = node_inputs(&artifact, node).unwrap();
            let genuine_source = constant_comparison_operand(
                &artifact,
                node,
                inputs[0],
                &ActivationFacts::default(),
                &ClosedActivationAnalysisContext::default(),
                true,
            )
            .unwrap()
            .unwrap();
            let source_entry = artifact
                .schemas()
                .entry(genuine_source.value.schema())
                .unwrap();
            let SchemaBody::Matrix { dimensions, .. } = source_entry.schema().body() else {
                panic!("ordinary conversion must retain a genuine Matrix source");
            };
            assert_eq!(dimensions.len(), 2);
            assert!(
                dimensions
                    .iter()
                    .all(|extent| matches!(extent, mech_core::DimensionExpr::Constant(_)))
            );
            assert!(source_entry.schema().dimension_parameters().is_empty());
            assert!(genuine_source.value.shape().parameter_values().is_empty());
            assert_eq!(genuine_source.value.schema_key(), source_entry.key());
            assert!(
                genuine_source
                    .value
                    .validate_against(artifact.schemas())
                    .is_ok()
            );
            let target_schema = artifact
                .schemas()
                .get(artifact.slots()[slot.get() as usize].schema)
                .unwrap();
            match target_schema.body() {
                SchemaBody::Set {
                    cardinality: CardinalitySpec::Exact(mech_core::DimensionExpr::Parameter(id)),
                    ..
                } => {
                    assert_eq!(id.get(), 0);
                    assert_eq!(target_schema.dimension_parameters().len(), 1);
                    assert_eq!(
                        target_schema.dimension_parameters()[0].lower_bound(),
                        &mech_core::DimensionExpr::Constant(0)
                    );
                    assert!(
                        target_schema.dimension_parameters()[0]
                            .upper_bound()
                            .is_none()
                    );
                    assert_eq!(
                        slot_shape(&artifact, slot, &ActivationFacts::default())
                            .unwrap()
                            .parameter_values()
                            .len(),
                        1
                    );
                }
                SchemaBody::Set {
                    cardinality: CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
                    ..
                } => {
                    assert!(target_schema.dimension_parameters().is_empty());
                }
                other => panic!("unexpected actual ordinary conversion target: {other:?}"),
            }
            let analysis = ClosedActivationAnalysisContext::default();
            super::super::set_conversion::SOURCE_DRAFTS.with(|count| count.set(0));
            let inferred = constant_comparison_operand(
                &artifact,
                node,
                ArtifactSource::Slot(slot),
                &ActivationFacts::default(),
                &analysis,
                true,
            )
            .unwrap()
            .unwrap_or_else(|| panic!("closed ordinary annotation declined: {source}"));
            assert_eq!(
                super::super::set_conversion::SOURCE_DRAFTS.with(core::cell::Cell::get),
                1
            );
            assert!(analysis.construction.compute_work.get() > 0);
            assert!(analysis.construction.comparison_work.get() > 0);
            assert!(analysis.construction.construction_bytes.get() > 0);
            assert!(matches!(
                inferred.value.data(),
                mech_core::ValueData::Set(_)
            ));
            assert_eq!(
                inferred.value.schema(),
                artifact.slots()[slot.get() as usize].schema
            );
            assert!(inferred.value.validate_against(artifact.schemas()).is_ok());
            let catalog = catalog();
            let factory = catalog
                .resident_factory(&["convert".to_owned()], "set")
                .unwrap();
            assert_eq!(
                factory.implementation_memory(),
                ImplementationMemoryClass::CanonicalSortUnique
            );
            let mut instance = activate(
                ReactiveInstanceId::new(514, 1),
                &artifact,
                &catalog,
                &ActivationFacts::default(),
            )
            .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            instance
                .turn(&[])
                .unwrap_or_else(|error| panic!("{source}: {error:?}"));
            let actual = instance.copied_output(0).unwrap();
            assert_eq!(inferred.value.schema_key(), actual.schema_key());
            assert_eq!(inferred.value.shape(), actual.shape());
            assert!(
                inferred
                    .value
                    .snapshot_eq(artifact.schemas(), &actual, artifact.schemas())
                    .unwrap(),
                "{source}"
            );
        }
    }
}

#[test]
fn closed_matrix_to_set_shared_sort_admission_denies_before_draft_and_fresh_analysis_recovers() {
    for artifact in variants("converted<{string}> := [\"z\" \"a\" \"z\"]\nconverted\n") {
        let (node, slot) = target(&artifact);
        for dimension in 0..3 {
            let denied = ClosedActivationAnalysisContext::default();
            match dimension {
                0 => denied
                    .construction
                    .construction_bytes
                    .set(mech_core::RESIDENT_MAX_BYTES),
                1 => denied
                    .construction
                    .compute_work
                    .set(mech_core::RESIDENT_MAX_COMPUTE_WORK),
                _ => denied
                    .construction
                    .comparison_work
                    .set(mech_core::RESIDENT_MAX_COMPARISON_WORK),
            }
            super::super::set_conversion::SOURCE_DRAFTS.with(|count| count.set(0));
            assert!(
                constant_comparison_operand(
                    &artifact,
                    node,
                    ArtifactSource::Slot(slot),
                    &ActivationFacts::default(),
                    &denied,
                    true,
                )
                .unwrap()
                .is_none(),
                "dimension {dimension}"
            );
            assert_eq!(
                super::super::set_conversion::SOURCE_DRAFTS.with(core::cell::Cell::get),
                0,
                "shared conversion must not copy/sort a source before denied admission"
            );
            assert!(
                constant_comparison_operand(
                    &artifact,
                    node,
                    ArtifactSource::Slot(slot),
                    &ActivationFacts::default(),
                    &ClosedActivationAnalysisContext::default(),
                    true,
                )
                .unwrap()
                .is_some()
            );
        }
    }
}

#[test]
fn closed_matrix_to_set_rejects_wrong_exact_cardinality_like_resident_publication() {
    for artifact in variants("converted<{u64}:3> := [3u64 1u64 3u64]\nconverted\n") {
        let (node, slot) = target(&artifact);
        assert!(
            constant_comparison_operand(
                &artifact,
                node,
                ArtifactSource::Slot(slot),
                &ActivationFacts::default(),
                &ClosedActivationAnalysisContext::default(),
                true,
            )
            .unwrap()
            .is_none()
        );
        let catalog = catalog();
        match activate(
            ReactiveInstanceId::new(514, 2),
            &artifact,
            &catalog,
            &ActivationFacts::default(),
        ) {
            Ok(mut instance) => assert!(instance.turn(&[]).is_err()),
            Err(_) => {} // Rejection may occur during exact closed shape activation.
        }
    }
}

#[test]
fn closed_matrix_to_set_setup_guard_rejects_recursive_schema_before_admission() {
    use mech_core::{
        CardinalitySpec, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
        DimensionParameterId, DimensionParameterOrigin, SchemaBody, SchemaDraft,
    };
    let schema = |body| {
        SchemaDraft {
            dimension_parameters: Box::new([]),
            body,
        }
        .finalize()
        .unwrap()
    };
    let matrix = |element| {
        schema(SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(3)]
                .into_boxed_slice(),
        })
    };
    let set = |element, cardinality| {
        schema(SchemaBody::Set {
            element: Box::new(element),
            cardinality,
        })
    };
    let source = matrix(SchemaBody::String);
    let target_schema = set(
        SchemaBody::String,
        CardinalitySpec::Dynamic { upper_bound: None },
    );
    let setup =
        super::super::set_conversion::closed_scalar_setup_bytes(&source, &target_schema).unwrap();
    assert!(
        setup
            > source.clone_allocation_bound_bytes().unwrap()
                + target_schema.clone_allocation_bound_bytes().unwrap()
    );
    assert!(setup.checked_mul(8).unwrap() + 256 < mech_core::RESIDENT_MAX_COMPUTE_WORK);
    assert!(
        super::super::set_conversion::closed_scalar_setup_bytes(
            &source,
            &set(
                SchemaBody::String,
                CardinalitySpec::Dynamic {
                    upper_bound: Some(DimensionExpr::Constant(3))
                }
            ),
        )
        .is_some()
    );
    let parameter_target = |upper_bound| {
        SchemaDraft {
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound,
            }]
            .into_boxed_slice(),
            body: SchemaBody::Set {
                element: Box::new(SchemaBody::String),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(
                    DimensionParameterId::new(0),
                )),
            },
        }
        .finalize()
        .unwrap()
    };
    assert_eq!(
        super::super::set_conversion::closed_scalar_setup_bytes(&source, &parameter_target(None)),
        Some(setup)
    );
    assert_eq!(
        super::super::set_conversion::closed_scalar_setup_bytes(
            &source,
            &parameter_target(Some(DimensionExpr::Constant(3)))
        ),
        Some(setup)
    );
    let dependent_target = SchemaDraft {
        dimension_parameters: vec![
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: DimensionParameterId::new(1),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: Some(DimensionExpr::Parameter(DimensionParameterId::new(0))),
            },
        ]
        .into_boxed_slice(),
        body: SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(
                DimensionParameterId::new(1),
            )),
        },
    }
    .finalize()
    .unwrap();
    assert!(
        super::super::set_conversion::closed_scalar_setup_bytes(&source, &dependent_target)
            .is_none(),
        "dependent/multiple parameters remain outside the bounded grammar"
    );
    let mut nested = SchemaBody::String;
    for _ in 0..32 {
        nested = SchemaBody::Option(Box::new(nested));
    }
    assert!(
        super::super::set_conversion::closed_scalar_setup_bytes(
            &matrix(nested.clone()),
            &set(nested, CardinalitySpec::Dynamic { upper_bound: None }),
        )
        .is_none(),
        "ancestor evidence and recursive plan allocations are outside the bounded grammar"
    );
    let tuple = SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Bool].into_boxed_slice());
    assert!(
        super::super::set_conversion::closed_scalar_setup_bytes(
            &matrix(tuple.clone()),
            &set(tuple, CardinalitySpec::Dynamic { upper_bound: None }),
        )
        .is_none()
    );

    // A genuine ordinary source still reaches setup admission before any
    // maintained conversion draft/sort is entered; this is not a mock Value.
    for artifact in variants("converted<{string}> := [\"z\" \"a\" \"z\"]\nconverted\n") {
        let (node, slot) = target(&artifact);
        let inputs = node_inputs(&artifact, node).unwrap();
        let source = constant_comparison_operand(
            &artifact,
            node,
            inputs[0],
            &ActivationFacts::default(),
            &ClosedActivationAnalysisContext::default(),
            true,
        )
        .unwrap()
        .unwrap();
        let actual_source = artifact.schemas().entry(source.value.schema()).unwrap();
        let actual_target = artifact
            .schemas()
            .get(artifact.slots()[slot.get() as usize].schema)
            .unwrap();
        assert_eq!(source.value.schema_key(), actual_source.key());
        assert_eq!(actual_target.dimension_parameters().len(), 1);
        assert_eq!(
            super::super::set_conversion::closed_scalar_setup_bytes(
                actual_source.schema(),
                actual_target
            ),
            Some(setup)
        );
        let mut admissions = 0;
        let mut shape_calls = 0;
        super::super::set_conversion::SOURCE_DRAFTS.with(|count| count.set(0));
        let result = super::super::set_conversion::fold_closed(
            &std::sync::Arc::new(artifact.schemas().clone()),
            &source.value,
            artifact.slots()[slot.get() as usize].schema,
            || {
                shape_calls += 1;
                Ok(slot_shape(&artifact, slot, &ActivationFacts::default()).unwrap())
            },
            super::super::budget::ResidentBudgetMeter::default,
            |cost| {
                admissions += 1;
                assert_eq!(cost.temporary_bytes(), setup);
                Err(ResidentKernelError::InvalidShape)
            },
        );
        assert!(result.is_err());
        assert_eq!(admissions, 1);
        assert_eq!(
            shape_calls, 0,
            "denied setup must precede target-shape allocation"
        );
        assert_eq!(
            super::super::set_conversion::SOURCE_DRAFTS.with(core::cell::Cell::get),
            0
        );

        // A valid witness from a different schema is an invalid oversized
        // activation fact for this real one-parameter conversion target.
        let oversized_schema = SchemaDraft {
            dimension_parameters: (0..128)
                .map(|id| DimensionParameterDeclaration {
                    id: DimensionParameterId::new(id),
                    origin: DimensionParameterOrigin::Inferred,
                    lifetime: DimensionLifetime::Turn,
                    lower_bound: DimensionExpr::Constant(0),
                    upper_bound: None,
                })
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::String),
                dimensions: (0..128)
                    .map(|id| DimensionExpr::Parameter(DimensionParameterId::new(id)))
                    .collect::<Vec<_>>()
                    .into_boxed_slice(),
            },
        }
        .finalize()
        .unwrap();
        let oversized = oversized_schema
            .instantiate_shape(vec![1; 128].into_boxed_slice())
            .unwrap();
        for use_seed in [false, true] {
            let mut facts = ActivationFacts::default();
            if use_seed {
                facts.slot_shape_seeds.insert(slot, oversized.clone());
            } else {
                facts.slot_shapes.insert(slot, oversized.clone());
            }
            SLOT_SHAPE_PARAMETER_COPIES.with(|copies| copies.set(0));
            assert!(matches!(slot_shape(&artifact, slot, &facts),
                Err(ResidentActivationError::UnresolvedShape { slot: failed }) if failed == slot));
            assert_eq!(
                SLOT_SHAPE_PARAMETER_COPIES.with(core::cell::Cell::get),
                0,
                "wrong parameter count must reject before copying supplied shape values"
            );
            super::super::set_conversion::SOURCE_DRAFTS.with(|count| count.set(0));
            assert!(matches!(constant_comparison_operand(
                &artifact, node, ArtifactSource::Slot(slot), &facts,
                &ClosedActivationAnalysisContext::default(), true,
            ), Err(ResidentActivationError::UnresolvedShape { slot: failed }) if failed == slot));
            assert_eq!(
                super::super::set_conversion::SOURCE_DRAFTS.with(core::cell::Cell::get),
                0
            );
        }
    }
}
