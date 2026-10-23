//! Frontend annotations retain real inner axes without equating Set size.
use super::*;
use mech_core::{ConversionStep, KindScheme, TypeConstraintEnvironment};

fn compile(source: &str) -> Result<CanonicalSourceProgram, SourceSemanticError> {
    let parsed = mech_syntax::document::parse_canonical_document(
        mech_syntax::document::TextSnapshot::new(DocumentId(0x41584953), Revision(1), source)
            .unwrap(),
        mech_syntax::document::ParseConfig::default(),
    );
    assert!(
        parsed.is_strictly_clean(),
        "{source}: {:?}",
        parsed.diagnostics
    );
    let mut builder = mech_core::FunctionCatalogBuilder::new();
    crate::install_intrinsic_resident(&mut builder).unwrap();
    CanonicalSourceFrontend.compile_document_with_catalog(
        &DocumentSyntax::cast(parsed.syntax()).unwrap(),
        Arc::new(builder.build().unwrap()),
    )
}
fn artifacts(source: &str) -> [ProgramArtifact; 2] {
    let original = compile(source).unwrap().compile_artifact().unwrap();
    let decoded = crate::decode_program_artifact_bytecode_v1(
        &crate::encode_program_artifact_bytecode_v1(&original).unwrap(),
    )
    .unwrap();
    [original, decoded]
}
fn matrix(element: SchemaBody, rows: u64, columns: u64) -> SchemaBody {
    SchemaBody::Matrix {
        element: Box::new(element),
        dimensions: vec![
            DimensionExpr::Constant(rows),
            DimensionExpr::Constant(columns),
        ]
        .into_boxed_slice(),
    }
}

#[test]
fn inferred_matrix_set_key_axes_survive_independent_arenas_and_current_updates() {
    for artifact in artifacts("unique<{[u8]}> := values<[[u8]]>\nunique\n") {
        assert_eq!(artifact.inputs().len(), 1);
        let source = artifact.schemas().get(artifact.inputs()[0].schema).unwrap();
        let target_id = artifact.outputs().last().unwrap().schema;
        let target = artifact.schemas().get(target_id).unwrap();
        assert_eq!(source.dimension_parameters().len(), 4);
        assert_eq!(target.dimension_parameters().len(), 3);
        for (rows, columns) in [(1, 4), (2, 2), (4, 1)] {
            let concrete = matrix(
                matrix(SchemaBody::UnsignedInteger(IntegerWidth::W8), rows, columns),
                1,
                1,
            );
            let source_shape =
                mech_core::shape_for_schema_components(source, &[(source.body(), concrete)], None)
                    .unwrap();
            let plan = plan_explicit_cast(
                &ResolvedType::from_schema(source, &source_shape).unwrap(),
                &ResolvedType::from_schema(
                    target,
                    &mech_core::shape_for_declared_lower_bounds(target).unwrap(),
                )
                .unwrap(),
            )
            .unwrap();
            assert!(matches!(&plan.step, ConversionStep::MatrixToSet(key)
                if matches!(&key.step, ConversionStep::Identity)));
            let seed =
                mech_core::conversion_output_shape(source, &source_shape, target, &plan).unwrap();
            let key = ValueDataDraft::Matrix((1u8..=4).map(ValueDataDraft::U8).collect());
            let input = ValueDraft {
                schema: artifact.inputs()[0].schema,
                shape_values: source_shape.parameter_values().to_vec().into_boxed_slice(),
                data: ValueDataDraft::Matrix(vec![key.clone()].into_boxed_slice()),
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let data = mech_core::execute_conversion_draft_with_schema(
                input.canonical_data_draft().unwrap(),
                &plan,
                target,
                &seed,
                &SnapshotValidationContext::new(artifact.schemas()),
            )
            .unwrap();
            let shape = mech_core::conversion_output_shape_for_data(target, &seed, &data).unwrap();
            let output = ValueDraft {
                schema: target_id,
                shape_values: shape.parameter_values().to_vec().into_boxed_slice(),
                data,
            }
            .finalize(&SnapshotValidationContext::new(artifact.schemas()))
            .unwrap();
            let SchemaBody::Set { element, .. } = target.closed_body(output.shape()).unwrap()
            else {
                panic!("Set")
            };
            let SchemaBody::Matrix { dimensions, .. } = *element else {
                panic!("Matrix key")
            };
            assert_eq!(
                dimensions.as_ref(),
                &[
                    DimensionExpr::Constant(rows),
                    DimensionExpr::Constant(columns)
                ]
            );
            assert_eq!(
                output.canonical_data_draft().unwrap(),
                ValueDataDraft::Set(vec![key].into_boxed_slice())
            );
        }
    }
}

#[test]
fn inferred_key_axes_do_not_bind_set_size_or_waive_declared_cast_constraints() {
    for artifact in artifacts("unique<{[u8]}> := values<[[u8]:2,2]:1,7>\nunique\n") {
        let source = artifact.schemas().get(artifact.inputs()[0].schema).unwrap();
        let target = artifact
            .schemas()
            .get(artifact.outputs().last().unwrap().schema)
            .unwrap();
        assert!(source.dimension_parameters().is_empty());
        assert_eq!(target.dimension_parameters().len(), 1);
        let SchemaBody::Set {
            element,
            cardinality,
        } = target.body()
        else {
            panic!("Set")
        };
        assert!(
            matches!(
                cardinality,
                CardinalitySpec::Exact(DimensionExpr::Parameter(_))
            ),
            "Set size stays a fresh existential, not source count seven"
        );
        let SchemaBody::Matrix { dimensions, .. } = element.as_ref() else {
            panic!("Matrix key")
        };
        assert_eq!(
            dimensions.as_ref(),
            &[DimensionExpr::Constant(2), DimensionExpr::Constant(2)]
        );
    }
    for artifact in artifacts("unique<{[u8]}:2> := values<[[u8]:2,2]:1,7>\nunique\n") {
        let target = artifact
            .schemas()
            .get(artifact.outputs().last().unwrap().schema)
            .unwrap();
        assert!(target.dimension_parameters().is_empty());
        assert!(matches!(
            target.body(),
            SchemaBody::Set {
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
                ..
            }
        ));
    }
    assert!(
        compile("unique<{[u8]:2,2}> := values<[[u8]:1,4]>\nunique\n").is_err(),
        "equal products are not equal declared Matrix axes"
    );
    assert!(
        compile("unique<{[:token]}> := values<[[u8]]>\nunique\n").is_err(),
        "shape inference cannot authorize an unsupported nominal scalar cast"
    );
}

#[test]
fn fresh_source_axis_variables_preserve_dependent_target_bounds() {
    let p = |n| DimensionExpr::Parameter(DimensionParameterId::new(n));
    let mut parameters = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(p(1)),
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        },
    ];
    let old = parameters.clone();
    let actual = ResolvedType::from_schema_body(
        &matrix(
            matrix(SchemaBody::UnsignedInteger(IntegerWidth::W8), 2, 2),
            1,
            7,
        ),
        &[],
    )
    .unwrap();
    let output = KindExpr::Set {
        element: Box::new(KindExpr::Matrix {
            element: Box::new(BuiltinScalarKind::U8.kind_expr()),
            dimensions: vec![p(0), DimensionExpr::Constant(2)].into_boxed_slice(),
        }),
        cardinality: p(1),
    };
    let pattern = KindExpr::Set {
        element: Box::new(KindExpr::Matrix {
            element: Box::new(KindExpr::Wildcard),
            dimensions: vec![p(0), DimensionExpr::Constant(2)].into_boxed_slice(),
        }),
        cardinality: p(1),
    };
    let input =
        annotation_conversion_shape::input_shape(actual.kind(), &pattern, &mut parameters).unwrap();
    assert_eq!(&parameters[..2], old.as_slice());
    assert_eq!(parameters.len(), 4);
    let KindExpr::Matrix { dimensions, .. } = &input else {
        panic!("source Matrix")
    };
    assert_eq!(dimensions.as_ref(), &[p(2), p(3)]);
    let scheme = KindScheme::new(
        Box::new([]),
        parameters.into_boxed_slice(),
        InputKindScheme::Fixed(vec![input].into_boxed_slice()),
        vec![output].into_boxed_slice(),
        Box::new([]),
    )
    .unwrap();
    let error =
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("dependent annotation", None))
            .solve_scheme(&scheme, &[actual], None)
            .unwrap_err();
    assert!(
        format!("{error:?}").contains("DimensionBoundNotProven"),
        "{error:?}"
    );
}
