use mech_core::{
    DimensionExpr as D, DimensionLifetime, DimensionOriginPolicy, DimensionParameterDeclaration,
    DimensionParameterId as Id, DimensionParameterOrigin, FloatWidth, IntegerWidth,
    SchemaBody as B, SchemaDimensionRelation as Relation, SchemaDraft, SchemaField,
    generalize_computed_matrix_axes, schema_dimension_parameter_mapping,
};

fn mixed(left: u32, right: u32) -> SchemaDraft {
    let buffer = |width, dimensions: Vec<D>| B::Matrix {
        element: Box::new(B::UnsignedInteger(width)),
        dimensions: dimensions.into(),
    };
    SchemaDraft {
        dimension_parameters: (0..2)
            .map(|id| DimensionParameterDeclaration {
                id: Id::new(id),
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: D::Constant(0),
                upper_bound: None,
            })
            .collect(),
        body: B::Record(
            vec![SchemaField {
                name: "state".into(),
                schema: B::Tuple(
                    vec![
                        buffer(
                            IntegerWidth::W8,
                            vec![D::Constant(1), D::Parameter(Id::new(left))],
                        ),
                        B::SignedInteger(IntegerWidth::W64),
                        B::Tuple(
                            vec![
                                B::FloatingPoint(FloatWidth::W64),
                                buffer(
                                    IntegerWidth::W16,
                                    vec![
                                        D::Parameter(Id::new(right)),
                                        D::Parameter(Id::new(right)),
                                    ],
                                ),
                            ]
                            .into(),
                        ),
                    ]
                    .into(),
                ),
            }]
            .into(),
        ),
    }
}

#[test]
fn renaming_preserves_nested_mixed_scalars_buffers_and_repeated_axes() {
    let actual = mixed(1, 0).finalize().unwrap();
    let expected = mixed(0, 1).finalize().unwrap();
    assert!(schema_dimension_parameter_mapping(&actual, &expected, Relation::Exact).is_none());
    let mapping =
        schema_dimension_parameter_mapping(&actual, &expected, Relation::FreeTurnUnsignedAxes)
            .unwrap();
    assert_eq!(mapping.target(Id::new(0)), Some(Id::new(1)));
    assert_eq!(mapping.target(Id::new(1)), Some(Id::new(0)));
    let mut wrong = mixed(0, 1);
    let B::Record(fields) = &mut wrong.body else {
        panic!()
    };
    let B::Tuple(fields) = &mut fields[0].schema else {
        panic!()
    };
    fields[1] = B::SignedInteger(IntegerWidth::W32);
    assert!(
        schema_dimension_parameter_mapping(
            &actual,
            &wrong.finalize().unwrap(),
            Relation::FreeTurnUnsignedAxes
        )
        .is_none()
    );
    let mut wrong = mixed(0, 1);
    let B::Record(fields) = &mut wrong.body else {
        panic!()
    };
    let B::Tuple(fields) = &mut fields[0].schema else {
        panic!()
    };
    let B::Tuple(fields) = &mut fields[2] else {
        panic!()
    };
    let B::Matrix { dimensions, .. } = &mut fields[1] else {
        panic!()
    };
    dimensions[0] = D::Parameter(Id::new(0));
    assert!(
        schema_dimension_parameter_mapping(
            &actual,
            &wrong.finalize().unwrap(),
            Relation::FreeTurnUnsignedAxes
        )
        .is_none()
    );
}

#[test]
fn bounds_and_persistent_axes_keep_their_declared_origin_and_identity() {
    for change in 0..3 {
        let mut actual = mixed(1, 0);
        let mut expected = mixed(0, 1);
        for draft in [&mut actual, &mut expected] {
            match change {
                0 => draft.dimension_parameters[0].lower_bound = D::Constant(1),
                1 => draft.dimension_parameters[0].upper_bound = Some(D::Constant(32)),
                _ => draft.dimension_parameters[0].lifetime = DimensionLifetime::Activation,
            }
        }
        assert_eq!(
            actual
                .clone()
                .with_dimension_origin_policy(DimensionOriginPolicy::InferFreeTurnAxes),
            actual
        );
        assert!(
            schema_dimension_parameter_mapping(
                &actual.finalize().unwrap(),
                &expected.finalize().unwrap(),
                Relation::FreeTurnUnsignedAxes
            )
            .is_none()
        );
    }
    let original = mixed(1, 0);
    assert_eq!(
        original
            .clone()
            .with_dimension_origin_policy(DimensionOriginPolicy::Preserve),
        original
    );
    let inferred = original.with_dimension_origin_policy(DimensionOriginPolicy::InferFreeTurnAxes);
    assert!(
        inferred
            .dimension_parameters
            .iter()
            .all(|p| p.origin == DimensionParameterOrigin::Inferred)
    );
}

#[test]
fn generalization_is_distinct_from_renaming_and_keeps_expression_relationships() {
    let mut source = mixed(0, 1);
    let expression = D::Add(vec![D::Parameter(Id::new(0)), D::Constant(1)].into());
    source.body = B::Matrix {
        element: Box::new(B::UnsignedInteger(IntegerWidth::W64)),
        dimensions: vec![
            D::Constant(1),
            expression.clone(),
            expression,
            D::Parameter(Id::new(1)),
        ]
        .into(),
    };
    let generalized = generalize_computed_matrix_axes(&source).unwrap();
    let B::Matrix { dimensions, .. } = &generalized.body else {
        panic!()
    };
    assert_eq!(dimensions[0], D::Constant(1));
    assert_eq!(dimensions[1], dimensions[2]);
    assert_eq!(dimensions[3], D::Parameter(Id::new(1)));
    assert_eq!(generalized.dimension_parameters.len(), 3);
    assert!(
        schema_dimension_parameter_mapping(
            &source.finalize().unwrap(),
            &generalized.finalize().unwrap(),
            Relation::FreeTurnUnsignedAxes
        )
        .is_none()
    );
}
