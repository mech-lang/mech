use mech_core::*;

fn p(index: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(index))
}

fn declaration(
    index: u32,
    lower: DimensionExpr,
    upper: Option<DimensionExpr>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(index),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Turn,
        lower_bound: lower,
        upper_bound: upper,
    }
}

fn carrier(values: &[u64]) -> ShapeInstance {
    SchemaDraft {
        dimension_parameters: (0..values.len())
            .map(|i| declaration(i as u32, DimensionExpr::Constant(0), None))
            .collect(),
        body: SchemaBody::Tuple(
            (0..values.len())
                .map(|i| SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
                    dimensions: vec![DimensionExpr::Constant(1), p(i as u32)].into(),
                })
                .collect(),
        ),
    }
    .finalize()
    .unwrap()
    .instantiate_shape(values.into())
    .unwrap()
}

#[test]
fn current_descriptor_shapes_match_fresh_types_without_mutating_the_prototype() {
    for width in [
        IntegerWidth::W8,
        IntegerWidth::W16,
        IntegerWidth::W32,
        IntegerWidth::W64,
    ] {
        let matrix = SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(width)),
            dimensions: vec![p(0), p(1)].into(),
        };
        let schema = SchemaDraft {
            dimension_parameters: vec![
                declaration(0, DimensionExpr::Constant(0), None),
                declaration(1, DimensionExpr::Constant(0), None),
            ]
            .into(),
            body: SchemaBody::Record(
                vec![
                    SchemaField {
                        name: "buffer".into(),
                        schema: matrix.clone(),
                    },
                    SchemaField {
                        name: "config".into(),
                        schema: SchemaBody::Tuple(
                            vec![
                                SchemaBody::Bool,
                                SchemaBody::SignedInteger(IntegerWidth::W64),
                                SchemaBody::FloatingPoint(FloatWidth::W64),
                                SchemaBody::String,
                            ]
                            .into(),
                        ),
                    },
                    SchemaField {
                        name: "optional".into(),
                        schema: SchemaBody::Option(Box::new(matrix)),
                    },
                ]
                .into(),
            ),
        }
        .finalize()
        .unwrap();
        let prototype = ResolvedValueDescriptor::from_schema(
            schema.clone(),
            schema.instantiate_shape(vec![2, 3].into()).unwrap(),
        )
        .unwrap();
        let frozen = prototype.clone();
        for values in [[0, 7], [2, 3], [3, 2], [3, 0], [1, u64::MAX]] {
            let shape = carrier(&values);
            let fresh =
                ResolvedValueDescriptor::from_schema(schema.clone(), shape.clone()).unwrap();
            let current = prototype.with_current_shape(shape).unwrap();
            assert_eq!(current, fresh);
            assert_eq!(current.shape().parameter_values(), &values);
            assert_eq!(prototype, frozen);
        }
    }
}

#[test]
fn current_descriptor_shapes_keep_dependent_bounds_and_first_shape_errors() {
    let schema = SchemaDraft {
        dimension_parameters: vec![
            declaration(
                0,
                DimensionExpr::Constant(2),
                Some(DimensionExpr::Constant(7)),
            ),
            declaration(
                1,
                p(0),
                Some(DimensionExpr::Multiply(
                    vec![DimensionExpr::Constant(2), p(0)].into(),
                )),
            ),
        ]
        .into(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W16)),
            dimensions: vec![p(0), p(1)].into(),
        },
    }
    .finalize()
    .unwrap();
    let prototype = ResolvedValueDescriptor::from_schema(
        schema.clone(),
        schema.instantiate_shape(vec![2, 3].into()).unwrap(),
    )
    .unwrap();
    for values in [
        vec![],
        vec![3],
        vec![3, 4, 5],
        vec![1, 0],
        vec![8, 9],
        vec![3, 2],
        vec![3, 7],
        vec![u64::MAX, 0],
    ] {
        let shape = carrier(&values);
        let expected = ResolvedValueDescriptor::from_schema(schema.clone(), shape.clone());
        assert!(expected.is_err());
        assert_eq!(prototype.with_current_shape(shape), expected, "{values:?}");
    }
    for values in [[2, 2], [3, 6], [7, 14]] {
        let shape = carrier(&values);
        assert_eq!(
            prototype.with_current_shape(shape.clone()),
            ResolvedValueDescriptor::from_schema(schema.clone(), shape)
        );
    }
}

#[test]
fn current_descriptor_shapes_recheck_extent_and_bound_overflow() {
    for bound in [false, true] {
        let schema = SchemaDraft {
            dimension_parameters: if bound {
                vec![
                    declaration(0, DimensionExpr::Constant(0), None),
                    declaration(
                        1,
                        DimensionExpr::Constant(0),
                        Some(DimensionExpr::Multiply(
                            vec![DimensionExpr::Constant(2), p(0)].into(),
                        )),
                    ),
                ]
                .into()
            } else {
                vec![declaration(0, DimensionExpr::Constant(0), None)].into()
            },
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W32)),
                dimensions: if bound {
                    vec![p(0), p(1)].into()
                } else {
                    vec![
                        DimensionExpr::Multiply(vec![p(0), p(0)].into()),
                        DimensionExpr::Constant(1),
                    ]
                    .into()
                },
            },
        }
        .finalize()
        .unwrap();
        let seed = if bound { vec![1, 1] } else { vec![1] };
        let prototype = ResolvedValueDescriptor::from_schema(
            schema.clone(),
            schema.instantiate_shape(seed.into()).unwrap(),
        )
        .unwrap();
        let shape = carrier(if bound { &[u64::MAX, 0] } else { &[u64::MAX] });
        let expected = ResolvedValueDescriptor::from_schema(schema.clone(), shape.clone());
        assert!(expected.is_err());
        assert_eq!(prototype.with_current_shape(shape), expected);
        assert_eq!(
            prototype
                .with_current_shape(carrier(if bound { &[1, 1] } else { &[1] }))
                .unwrap(),
            prototype
        );
    }
}
