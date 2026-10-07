use mech_core::*;

fn named(ordinal: u32) -> KindExpr {
    KindExpr::Named(KindId::new(ordinal))
}

fn closed(kind: KindExpr) -> ResolvedType {
    ResolvedType::new(kind, Vec::new().into_boxed_slice()).unwrap()
}

fn dimension_parameter(
    ordinal: u32,
    lifetime: DimensionLifetime,
    lower: u64,
    upper: Option<u64>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(ordinal),
        origin: DimensionParameterOrigin::Inferred,
        lifetime,
        lower_bound: DimensionExpr::Constant(lower),
        upper_bound: upper.map(DimensionExpr::Constant),
    }
}

fn scheme(
    kind_parameters: Vec<KindParameter>,
    dimensions: Vec<DimensionParameterDeclaration>,
    inputs: Vec<KindExpr>,
    outputs: Vec<KindExpr>,
    constraints: Vec<KindConstraint>,
) -> KindScheme {
    KindScheme::new(
        kind_parameters.into_boxed_slice(),
        dimensions.into_boxed_slice(),
        InputKindScheme::Fixed(inputs.into_boxed_slice()),
        outputs.into_boxed_slice(),
        constraints.into_boxed_slice(),
    )
    .unwrap()
}

fn solve(
    semantic_name: &str,
    scheme: &KindScheme,
    inputs: &[ResolvedType],
) -> Result<SchemeResolution, TypeResolutionError> {
    TypeConstraintEnvironment::new(TypeConstraintOrigin::new(semantic_name, None))
        .solve_scheme(scheme, inputs, None)
}

fn resolve_maintained_operation(
    operation: &str,
    inputs: &[ResolvedType],
) -> Result<ResolvedOverload, TypeResolutionError> {
    let schemes = if let Some(template) = maintained_source_scheme_template(operation) {
        instantiate_source_scheme_template(template, inputs).unwrap()
    } else {
        maintained_source_schemes(operation).unwrap().unwrap()
    };
    let candidates = schemes
        .iter()
        .enumerate()
        .map(|(id, scheme)| TypeOverloadCandidate {
            id: id as u64,
            scheme,
        })
        .collect::<Vec<_>>();
    resolve_type_overloads(
        TypeConstraintOrigin::new(operation, None),
        &candidates,
        inputs,
        None,
    )
}

#[test]
fn maintained_addition_preserves_only_identical_scalar_interval_kinds() {
    let interval_kind = |width, signed, lower: i128, upper: i128, upper_inclusive| {
        KindExpr::IntegerInterval(if signed {
            IntegerInterval::Signed {
                width,
                lower,
                upper,
                upper_inclusive,
            }
        } else {
            IntegerInterval::Unsigned {
                width,
                lower: lower as u128,
                upper: upper as u128,
                upper_inclusive,
            }
        })
    };
    for (width, unsigned_kind, signed_kind) in [
        (
            IntegerWidth::W8,
            BuiltinScalarKind::U8,
            BuiltinScalarKind::I8,
        ),
        (
            IntegerWidth::W16,
            BuiltinScalarKind::U16,
            BuiltinScalarKind::I16,
        ),
        (
            IntegerWidth::W32,
            BuiltinScalarKind::U32,
            BuiltinScalarKind::I32,
        ),
        (
            IntegerWidth::W64,
            BuiltinScalarKind::U64,
            BuiltinScalarKind::I64,
        ),
        (
            IntegerWidth::W128,
            BuiltinScalarKind::U128,
            BuiltinScalarKind::I128,
        ),
    ] {
        for (signed, primitive) in [(false, unsigned_kind), (true, signed_kind)] {
            for inclusive in [false, true] {
                let kind = interval_kind(width, signed, 1, 10, inclusive);
                let input = closed(kind.clone());
                let inputs = [input.clone(), input.clone()];
                let resolution = resolve_maintained_operation("math/add", &inputs).unwrap();
                assert_eq!(resolution.outputs.as_ref(), &[input.clone()]);
                assert!(!input.satisfies(BuiltinKindPredicate::Number));
                assert_eq!(numeric_promotion(&input, &input).unwrap(), None);
                assert!(resolution.conversions.iter().all(|conversion| {
                    conversion.source == input
                        && conversion.target == input
                        && conversion.step == ConversionStep::Identity
                        && conversion.cost == 0
                }));

                let other_width = if width == IntegerWidth::W128 {
                    IntegerWidth::W64
                } else {
                    IntegerWidth::W128
                };
                for other in [
                    closed(primitive.kind_expr()),
                    closed(interval_kind(width, signed, 2, 10, inclusive)),
                    closed(interval_kind(width, signed, 1, 11, inclusive)),
                    closed(interval_kind(width, signed, 1, 10, !inclusive)),
                    closed(interval_kind(width, !signed, 1, 10, inclusive)),
                    closed(interval_kind(other_width, signed, 1, 10, inclusive)),
                ] {
                    for inputs in [
                        [input.clone(), other.clone()],
                        [other.clone(), input.clone()],
                    ] {
                        assert!(
                            resolve_maintained_operation("math/add", &inputs).is_err(),
                            "interval addition must not widen or convert {inputs:?}",
                        );
                    }
                }
                for dimensions in [[1, 1], [2, 3]] {
                    let matrix = closed(KindExpr::Matrix {
                        element: Box::new(kind.clone()),
                        dimensions: dimensions.map(DimensionExpr::Constant).into(),
                    });
                    for inputs in [
                        [input.clone(), matrix.clone()],
                        [matrix.clone(), input.clone()],
                        [matrix.clone(), matrix.clone()],
                    ] {
                        assert!(
                            resolve_maintained_operation("math/add", &inputs).is_err(),
                            "interval addition is scalar-only: {inputs:?}",
                        );
                    }
                }
                for operation in ["math/sub", "math/mul", "math/div", "math/mod", "math/pow"] {
                    assert!(resolve_maintained_operation(operation, &inputs).is_err());
                }
                for operation in ["math/neg", "math/abs"] {
                    assert!(resolve_maintained_operation(operation, &[input.clone()]).is_err());
                }
            }
        }
    }
}

#[test]
fn addition_template_preserves_existing_numeric_schemes() {
    let template = maintained_source_scheme_template("math/add").unwrap();
    let u8_type = closed(BuiltinScalarKind::U8.kind_expr());
    let u16_type = closed(BuiltinScalarKind::U16.kind_expr());
    let matrix = |scalar: &ResolvedType| {
        closed(KindExpr::Matrix {
            element: Box::new(scalar.kind().clone()),
            dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into(),
        })
    };
    for (inputs, output) in [
        ([u8_type.clone(), u16_type.clone()], u16_type.clone()),
        ([matrix(&u8_type), matrix(&u16_type)], matrix(&u16_type)),
        ([u8_type.clone(), matrix(&u16_type)], matrix(&u16_type)),
        ([matrix(&u8_type), u16_type.clone()], matrix(&u16_type)),
    ] {
        assert_eq!(
            instantiate_source_scheme_template(template, &inputs).unwrap(),
            maintained_source_schemes("math/add").unwrap().unwrap(),
        );
        let resolution = resolve_maintained_operation("math/add", &inputs).unwrap();
        assert_eq!(resolution.outputs.as_ref(), &[output]);
    }
}

#[test]
fn exact_conversion_promotion_and_cast_are_independent_relations() {
    let u8_type = closed(named(0));
    let u16_type = closed(named(1));
    let i8_type = closed(named(5));
    let i16_type = closed(named(6));
    let f32_type = closed(named(10));
    let f64_type = closed(named(11));
    let string_type = closed(named(14));
    let bool_type = closed(named(15));

    assert!(exact_type_equal(&u8_type, &u8_type));
    assert!(!exact_type_equal(&u8_type, &u16_type));
    assert!(permitted_conversion(&u8_type, &u16_type));
    assert!(!permitted_conversion(&u16_type, &u8_type));
    assert!(permitted_conversion(&f32_type, &f64_type));
    assert!(!permitted_conversion(&u8_type, &string_type));
    assert!(explicit_cast_allowed(&u8_type, &string_type));
    assert!(explicit_cast_allowed(&f64_type, &u8_type));
    assert!(explicit_cast_allowed(&bool_type, &string_type));
    assert!(explicit_cast_allowed(&closed(named(12)), &f64_type));

    assert_eq!(
        numeric_promotion(&u8_type, &i8_type).unwrap(),
        Some(i16_type)
    );
    assert_eq!(numeric_promotion(&u8_type, &string_type).unwrap(), None);
}

#[test]
fn maintained_ordering_resolves_exact_interval_kinds_without_numeric_promotion() {
    let interval = KindExpr::IntegerInterval(IntegerInterval::Unsigned {
        width: IntegerWidth::W128,
        lower: (1_u128 << 100) + 1,
        upper: (1_u128 << 100) + 4,
        upper_inclusive: false,
    });
    let interval_type = closed(interval.clone());
    assert!(interval_type.satisfies(BuiltinKindPredicate::Ordered));
    assert!(!interval_type.satisfies(BuiltinKindPredicate::Number));
    assert_eq!(
        numeric_promotion(&interval_type, &interval_type).unwrap(),
        None
    );
    let matrix_kind = |element| KindExpr::Matrix {
        element: Box::new(element),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into_boxed_slice(),
    };
    for operation in ["compare/lt", "compare/lte", "compare/gt", "compare/gte"] {
        // The semantic template inventory is available in every core profile;
        // its function-catalog wrapper is optional and adds no policy here.
        let template = maintained_source_scheme_template(operation)
            .expect("maintained ordering is a closed input-aware family");
        let resolve = |inputs: &[ResolvedType]| {
            let schemes = instantiate_source_scheme_template(template, inputs).unwrap();
            let candidates = schemes
                .iter()
                .enumerate()
                .map(|(id, scheme)| TypeOverloadCandidate {
                    id: id as u64,
                    scheme,
                })
                .collect::<Vec<_>>();
            resolve_type_overloads(
                TypeConstraintOrigin::new(operation, None),
                &candidates,
                inputs,
                None,
            )
        };
        for (left, right, output) in [
            (
                interval_type.clone(),
                interval_type.clone(),
                closed(BuiltinScalarKind::Bool.kind_expr()),
            ),
            (
                closed(matrix_kind(interval.clone())),
                closed(matrix_kind(interval.clone())),
                closed(matrix_kind(BuiltinScalarKind::Bool.kind_expr())),
            ),
            (
                interval_type.clone(),
                closed(matrix_kind(interval.clone())),
                closed(matrix_kind(BuiltinScalarKind::Bool.kind_expr())),
            ),
            (
                closed(matrix_kind(interval.clone())),
                interval_type.clone(),
                closed(matrix_kind(BuiltinScalarKind::Bool.kind_expr())),
            ),
        ] {
            let inputs = [left, right];
            let resolution = resolve(&inputs).unwrap();
            assert_eq!(resolution.outputs.as_ref(), &[output]);
            assert!(
                resolution
                    .conversions
                    .iter()
                    .zip(&inputs)
                    .all(|(conversion, input)| {
                        &conversion.source == input
                            && &conversion.target == input
                            && conversion.step == ConversionStep::Identity
                            && conversion.cost == 0
                    }),
                "{operation} must retain interval/schema identity rather than promote it"
            );
        }
        for other in [
            closed(BuiltinScalarKind::U128.kind_expr()),
            closed(KindExpr::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower: (1_u128 << 100) + 2,
                upper: (1_u128 << 100) + 4,
                upper_inclusive: false,
            })),
            closed(KindExpr::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower: (1_u128 << 100) + 1,
                upper: (1_u128 << 100) + 5,
                upper_inclusive: false,
            })),
            closed(KindExpr::IntegerInterval(IntegerInterval::Unsigned {
                width: IntegerWidth::W128,
                lower: (1_u128 << 100) + 1,
                upper: (1_u128 << 100) + 4,
                upper_inclusive: true,
            })),
        ] {
            assert!(resolve(&[interval_type.clone(), other.clone()]).is_err());
            assert!(resolve(&[other, interval_type.clone()]).is_err());
        }
        // Independent input dimensions must not be unified into one symbolic
        // origin merely because an execution can establish equal extents.
        let symbolic_matrix = |row_parameter: bool| {
            ResolvedType::new(
                KindExpr::Matrix {
                    element: Box::new(interval.clone()),
                    dimensions: if row_parameter {
                        vec![
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                            DimensionExpr::Constant(3),
                        ]
                    } else {
                        vec![
                            DimensionExpr::Constant(2),
                            DimensionExpr::Parameter(DimensionParameterId::new(0)),
                        ]
                    }
                    .into_boxed_slice(),
                },
                vec![dimension_parameter(0, DimensionLifetime::Turn, 0, None)].into_boxed_slice(),
            )
            .unwrap()
        };
        let inputs = [symbolic_matrix(true), symbolic_matrix(false)];
        let compatible =
            resolve(&inputs).expect("equal-extent obligations allow distinct input origins");
        assert!(
            matches!(compatible.outputs[0].kind(), KindExpr::Matrix { element, dimensions }
            if element.as_ref() == &BuiltinScalarKind::Bool.kind_expr() && dimensions.len() == 2)
        );
        assert!(
            compatible
                .conversions
                .iter()
                .zip(&inputs)
                .all(|(conversion, input)| &conversion.source == input
                    && &conversion.target == input
                    && conversion.step == ConversionStep::Identity)
        );
        // The template is an extension of the pre-existing non-interval
        // registry, not a generic Ordered admission bypass.
        for inputs in [
            vec![closed(KindExpr::Index), closed(KindExpr::Index)],
            vec![
                closed(matrix_kind(BuiltinScalarKind::String.kind_expr())),
                closed(BuiltinScalarKind::String.kind_expr()),
            ],
            vec![
                closed(BuiltinScalarKind::String.kind_expr()),
                closed(matrix_kind(BuiltinScalarKind::String.kind_expr())),
            ],
            vec![
                closed(BuiltinScalarKind::U8.kind_expr()),
                closed(BuiltinScalarKind::U16.kind_expr()),
            ],
        ] {
            assert_eq!(
                instantiate_source_scheme_template(template, &inputs).unwrap(),
                maintained_source_schemes(operation).unwrap().unwrap(),
            );
        }
        assert!(resolve(&[closed(KindExpr::Index), closed(KindExpr::Index)]).is_err());
        let promoted = resolve(&[
            closed(BuiltinScalarKind::U8.kind_expr()),
            closed(BuiltinScalarKind::U16.kind_expr()),
        ])
        .unwrap();
        assert_eq!(
            promoted.outputs.as_ref(),
            &[closed(BuiltinScalarKind::Bool.kind_expr())]
        );
        assert_eq!(
            promoted.conversions[0].target,
            closed(BuiltinScalarKind::U16.kind_expr())
        );
    }
}

#[test]
fn kind_variables_close_options_and_structural_aggregates() {
    let type_parameter = KindParameterId::new(0);
    let identity = scheme(
        vec![KindParameter {
            id: type_parameter,
            upper_bound: None,
        }],
        vec![],
        vec![KindExpr::Option(Box::new(KindExpr::Parameter(
            type_parameter,
        )))],
        vec![KindExpr::Parameter(type_parameter)],
        vec![],
    );
    let record = KindExpr::Record(
        vec![
            KindField {
                name: "enabled".into(),
                kind: named(15),
            },
            KindField {
                name: "count".into(),
                kind: named(3),
            },
        ]
        .into_boxed_slice(),
    );
    let actual = closed(KindExpr::Option(Box::new(record.clone())));
    let resolution = solve("option/default", &identity, &[actual]).unwrap();

    assert_eq!(resolution.outputs.as_ref(), &[closed(record)]);
}

#[test]
fn matrix_dimensions_are_solved_and_conflicts_are_structured() {
    let element = KindParameterId::new(0);
    let rows = DimensionParameterId::new(0);
    let columns = DimensionParameterId::new(1);
    let matrix = KindExpr::Matrix {
        element: Box::new(KindExpr::Parameter(element)),
        dimensions: vec![
            DimensionExpr::Parameter(rows),
            DimensionExpr::Parameter(columns),
        ]
        .into_boxed_slice(),
    };
    let same_shape = scheme(
        vec![KindParameter {
            id: element,
            upper_bound: None,
        }],
        vec![
            dimension_parameter(0, DimensionLifetime::Activation, 0, None),
            dimension_parameter(1, DimensionLifetime::Activation, 0, None),
        ],
        vec![matrix.clone(), matrix.clone()],
        vec![matrix],
        vec![],
    );
    let matrix_2x3 = closed(KindExpr::Matrix {
        element: Box::new(named(11)),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into_boxed_slice(),
    });
    let matrix_2x4 = closed(KindExpr::Matrix {
        element: Box::new(named(11)),
        dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(4)].into_boxed_slice(),
    });

    let resolution = solve(
        "matrix/same-shape",
        &same_shape,
        &[matrix_2x3.clone(), matrix_2x3.clone()],
    )
    .unwrap();
    assert_eq!(resolution.outputs.as_ref(), &[matrix_2x3.clone()]);

    let error = solve("matrix/same-shape", &same_shape, &[matrix_2x3, matrix_2x4]).unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(
                failure,
                TypeConstraintFailure::IncompatibleDimensions { .. }
            ))
    ));
}

#[test]
fn dynamic_extents_cannot_satisfy_a_more_stable_extent_contract() {
    let actual_dimension = DimensionParameterId::new(0);
    let actual = ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(named(11)),
            dimensions: vec![
                DimensionExpr::Parameter(actual_dimension),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        },
        vec![dimension_parameter(0, DimensionLifetime::Turn, 0, Some(64))].into_boxed_slice(),
    )
    .unwrap();
    let expected_dimension = DimensionParameterId::new(0);
    let matrix = KindExpr::Matrix {
        element: Box::new(named(11)),
        dimensions: vec![
            DimensionExpr::Parameter(expected_dimension),
            DimensionExpr::Constant(1),
        ]
        .into_boxed_slice(),
    };
    let activation_fixed = scheme(
        vec![],
        vec![dimension_parameter(
            0,
            DimensionLifetime::Activation,
            0,
            Some(64),
        )],
        vec![matrix.clone()],
        vec![matrix],
        vec![],
    );

    let error = solve("matrix/activation-fixed", &activation_fixed, &[actual]).unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(
                failure,
                TypeConstraintFailure::InvalidDynamicFixedExtent { .. }
            ))
    ));
}

#[test]
fn schema_derived_matrix_types_preserve_extent_lifetimes_and_bounds() {
    let dimension = DimensionParameterId::new(0);
    let schema = SchemaDraft {
        dimension_parameters: vec![dimension_parameter(0, DimensionLifetime::Turn, 1, Some(10))]
            .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::FloatingPoint(FloatWidth::W64)),
            dimensions: vec![
                DimensionExpr::Parameter(dimension),
                DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let first = ResolvedType::from_schema(
        &schema,
        &schema
            .instantiate_shape(vec![3].into_boxed_slice())
            .unwrap(),
    )
    .unwrap();
    let second = ResolvedType::from_schema(
        &schema,
        &schema
            .instantiate_shape(vec![7].into_boxed_slice())
            .unwrap(),
    )
    .unwrap();

    assert_eq!(first, second);
    assert_eq!(first.dimension_parameters().len(), 1);
    assert_eq!(
        first.dimension_parameters()[0].lifetime,
        DimensionLifetime::Turn
    );
    assert_eq!(
        first.dimension_parameters()[0].lower_bound,
        DimensionExpr::Constant(1)
    );
    assert_eq!(
        first.dimension_parameters()[0].upper_bound,
        Some(DimensionExpr::Constant(10))
    );
    assert!(matches!(
        first.kind(),
        KindExpr::Matrix { dimensions, .. }
            if dimensions[0] == DimensionExpr::Parameter(DimensionParameterId::new(0))
    ));
}

#[test]
fn nominal_identity_conflicts_do_not_degrade_to_structural_matching() {
    let atom_a = KindExpr::Atom(NominalKey::from_bytes([1; 32]));
    let atom_b = KindExpr::Atom(NominalKey::from_bytes([2; 32]));
    let exact_atom = scheme(vec![], vec![], vec![atom_a.clone()], vec![atom_a], vec![]);

    let error = solve("atom/use", &exact_atom, &[closed(atom_b)]).unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(
                failure,
                TypeConstraintFailure::ConflictingNominalTypes { .. }
            ))
    ));
}

#[test]
fn conversion_and_keyability_constraints_are_enforced_by_the_environment() {
    let parameter = KindParameterId::new(0);
    let convert_to_u16 = scheme(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }],
        vec![],
        vec![KindExpr::Parameter(parameter)],
        vec![named(1)],
        vec![KindConstraint::Convertible(
            KindExpr::Parameter(parameter),
            named(1),
        )],
    );
    let converted = solve("convert", &convert_to_u16, &[closed(named(0))]).unwrap();
    assert_eq!(converted.conversion_count, 1);
    assert_eq!(converted.outputs.as_ref(), &[closed(named(1))]);

    let keyable = scheme(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }],
        vec![],
        vec![KindExpr::Parameter(parameter)],
        vec![KindExpr::Parameter(parameter)],
        vec![KindConstraint::Satisfies {
            kind: KindExpr::Parameter(parameter),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    );
    assert!(solve("map/key", &keyable, &[closed(named(0))]).is_ok());
    let error = solve("map/key", &keyable, &[closed(named(12))]).unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(
                failure,
                TypeConstraintFailure::PredicateNotSatisfied {
                    predicate: BuiltinKindPredicate::Keyable,
                    ..
                }
            ))
    ));
    assert!(solve("map/key", &keyable, &[closed(named(99))]).is_err());
}

#[test]
fn schema_keyability_survives_nested_kind_variable_unification() {
    let parameter = KindParameterId::new(0);
    let unwrap_keyable = scheme(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }],
        vec![],
        vec![KindExpr::Option(Box::new(KindExpr::Parameter(parameter)))],
        vec![KindExpr::Parameter(parameter)],
        vec![KindConstraint::Satisfies {
            kind: KindExpr::Parameter(parameter),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    );
    let enum_key = NominalKey::from_bytes([31; 32]);
    let schema = SchemaDraft {
        dimension_parameters: Box::new([]),
        body: SchemaBody::Option(Box::new(SchemaBody::Enum {
            key: enum_key,
            variants: vec![
                EnumVariantSchema {
                    name: "none".into(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "some".into(),
                    payload: Some(SchemaBody::String),
                },
            ]
            .into_boxed_slice(),
        })),
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(Box::new([])).unwrap();
    let actual = ResolvedType::from_schema(&schema, &shape).unwrap();

    let resolution = solve("option/keyable", &unwrap_keyable, &[actual]).unwrap();
    assert_eq!(
        resolution.outputs.as_ref(),
        &[closed(KindExpr::Enum(enum_key))]
    );
    assert!(resolution.outputs[0].satisfies(BuiltinKindPredicate::Keyable));

    let parameter = KindParameterId::new(0);
    let key_identity = scheme(
        vec![KindParameter {
            id: parameter,
            upper_bound: None,
        }],
        vec![],
        vec![KindExpr::Parameter(parameter)],
        vec![KindExpr::Parameter(parameter)],
        vec![KindConstraint::Satisfies {
            kind: KindExpr::Parameter(parameter),
            predicate: BuiltinKindPredicate::Keyable,
        }],
    );
    assert!(solve("map/reuse-key", &key_identity, &resolution.outputs).is_ok());
}

#[test]
fn ambiguity_and_unresolved_variables_have_distinct_structured_results() {
    let to_u8 = scheme(
        vec![],
        vec![],
        vec![KindExpr::Wildcard],
        vec![named(0)],
        vec![],
    );
    let to_u16 = scheme(
        vec![],
        vec![],
        vec![KindExpr::Wildcard],
        vec![named(1)],
        vec![],
    );
    let candidates = [
        TypeOverloadCandidate {
            id: 11,
            scheme: &to_u8,
        },
        TypeOverloadCandidate {
            id: 12,
            scheme: &to_u16,
        },
    ];
    let error = resolve_type_overloads(
        TypeConstraintOrigin::new("ambiguous", None),
        &candidates,
        &[closed(named(15))],
        None,
    )
    .unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Ambiguous { ref alternatives, .. } if alternatives.len() == 2
    ));

    let unresolved_parameter = KindParameterId::new(0);
    let unresolved = scheme(
        vec![KindParameter {
            id: unresolved_parameter,
            upper_bound: None,
        }],
        vec![],
        vec![],
        vec![KindExpr::Parameter(unresolved_parameter)],
        vec![],
    );
    let error = solve("unresolved", &unresolved, &[]).unwrap_err();
    assert!(matches!(
        error,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.iter().any(|failure| matches!(
                failure,
                TypeConstraintFailure::UnresolvedKindVariable { .. }
            ))
    ));

    let hole = ResolvedType::new(KindExpr::Hole, Vec::new().into_boxed_slice()).unwrap_err();
    assert!(matches!(
        hole,
        TypeResolutionError::Incompatible { ref failures, .. }
            if failures.as_ref() == &[TypeConstraintFailure::UnresolvedKindHole]
    ));
}

#[test]
fn the_most_specific_scheme_wins_before_execution_binding() {
    let exact = scheme(vec![], vec![], vec![named(0)], vec![named(0)], vec![]);
    let wildcard = scheme(
        vec![],
        vec![],
        vec![KindExpr::Wildcard],
        vec![named(1)],
        vec![],
    );
    let candidates = [
        TypeOverloadCandidate {
            id: 4,
            scheme: &wildcard,
        },
        TypeOverloadCandidate {
            id: 9,
            scheme: &exact,
        },
    ];
    let result = resolve_type_overloads(
        TypeConstraintOrigin::new("specific", None),
        &candidates,
        &[closed(named(0))],
        None,
    )
    .unwrap();

    assert_eq!(result.candidate_ids.as_ref(), &[9]);
    assert_eq!(result.outputs.as_ref(), &[closed(named(0))]);
}

#[test]
fn source_ranges_and_semantic_names_survive_diagnostic_conversion() {
    let range = SourceRange {
        start: SourceLocation { row: 4, col: 7 },
        end: SourceLocation { row: 4, col: 16 },
    };
    let exact = scheme(vec![], vec![], vec![named(0)], vec![named(0)], vec![]);
    let error =
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("math/add", Some(range.clone())))
            .solve_scheme(&exact, &[closed(named(15))], None)
            .unwrap_err();
    let error = MechError::from(error);

    assert_eq!(error.kind_name(), "TypeIncompatibility");
    assert!(error.kind_message().contains("math/add"));
    assert_eq!(error.annotations, vec![range]);
    assert!(!error.kind_message().contains("FunctionValueRepresentation"));
}

#[test]
fn dimension_normalization_is_idempotent_and_commutative_over_generated_inputs() {
    let mut state = 0x2d35_8dcc_aa6c_78a5_u64;
    for _ in 0..512 {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        let first = DimensionExpr::Parameter(DimensionParameterId::new((state % 3) as u32));
        let second = DimensionExpr::Constant((state >> 8) % 97);
        let third = DimensionExpr::Parameter(DimensionParameterId::new(((state >> 16) % 3) as u32));
        let left = DimensionExpr::Add(
            vec![
                first.clone(),
                DimensionExpr::Add(vec![second.clone(), third.clone()].into_boxed_slice()),
                DimensionExpr::Constant(0),
            ]
            .into_boxed_slice(),
        );
        let right = DimensionExpr::Add(
            vec![third, second, first, DimensionExpr::Constant(0)].into_boxed_slice(),
        );
        let normalized = normalize_dimension(&left, 3).unwrap();
        assert_eq!(normalized, normalize_dimension(&normalized, 3).unwrap());
        assert_eq!(normalized, normalize_dimension(&right, 3).unwrap());
    }
}

#[test]
fn resolved_type_normalization_is_deterministic_across_dimension_permutations() {
    let declarations = vec![
        dimension_parameter(0, DimensionLifetime::Activation, 0, Some(64)),
        dimension_parameter(1, DimensionLifetime::Activation, 0, Some(64)),
    ];
    let operands = [
        DimensionExpr::Parameter(DimensionParameterId::new(0)),
        DimensionExpr::Parameter(DimensionParameterId::new(1)),
        DimensionExpr::Constant(3),
    ];
    let permutations = [
        [0, 1, 2],
        [0, 2, 1],
        [1, 0, 2],
        [1, 2, 0],
        [2, 0, 1],
        [2, 1, 0],
    ];
    let mut baseline = None;
    for permutation in permutations {
        let dimension = DimensionExpr::Add(
            permutation
                .iter()
                .map(|index| operands[*index].clone())
                .collect::<Vec<_>>()
                .into_boxed_slice(),
        );
        let resolved = ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(named(11)),
                dimensions: vec![dimension, DimensionExpr::Constant(1)].into_boxed_slice(),
            },
            declarations.clone().into_boxed_slice(),
        )
        .unwrap();
        let repeated = ResolvedType::new(
            resolved.kind().clone(),
            resolved.dimension_parameters().to_vec().into_boxed_slice(),
        )
        .unwrap();
        assert_eq!(resolved, repeated);
        if let Some(baseline) = &baseline {
            assert_eq!(baseline, &resolved);
        } else {
            baseline = Some(resolved);
        }
    }
}

#[test]
fn dynamic_extent_lifetimes_participate_in_exact_and_conversion_relations() {
    let kind = KindExpr::Set {
        element: Box::new(named(0)),
        cardinality: DimensionExpr::Parameter(DimensionParameterId::new(0)),
    };
    let activation = ResolvedType::new(
        kind.clone(),
        vec![dimension_parameter(
            0,
            DimensionLifetime::Activation,
            0,
            Some(8),
        )]
        .into_boxed_slice(),
    )
    .unwrap();
    let turn = ResolvedType::new(
        kind,
        vec![dimension_parameter(0, DimensionLifetime::Turn, 0, Some(8))].into_boxed_slice(),
    )
    .unwrap();

    assert!(!exact_type_equal(&activation, &turn));
    assert!(!permitted_conversion(&activation, &turn));
    assert!(!explicit_cast_allowed(&activation, &turn));
    assert_eq!(numeric_promotion(&activation, &turn).unwrap(), None);
}

#[test]
fn alpha_equivalent_dynamic_outputs_close_in_one_constraint_environment() {
    let parameter = DimensionParameterId::new(0);
    let dynamic_set = |lifetime| {
        ResolvedType::new(
            KindExpr::Set {
                element: Box::new(named(0)),
                cardinality: DimensionExpr::Parameter(parameter),
            },
            vec![dimension_parameter(0, lifetime, 0, Some(32))].into_boxed_slice(),
        )
        .unwrap()
    };
    let scheme_dimension = DimensionParameterId::new(0);
    let set_scheme = KindExpr::Set {
        element: Box::new(named(0)),
        cardinality: DimensionExpr::Parameter(scheme_dimension),
    };
    let identity = scheme(
        vec![],
        vec![dimension_parameter(0, DimensionLifetime::Turn, 0, Some(32))],
        vec![set_scheme.clone()],
        vec![set_scheme],
        vec![],
    );
    let input = dynamic_set(DimensionLifetime::Turn);
    let expected = dynamic_set(DimensionLifetime::Turn);
    let resolution =
        TypeConstraintEnvironment::new(TypeConstraintOrigin::new("set/identity", None))
            .solve_scheme(&identity, &[input], Some(&[expected.clone()]))
            .unwrap();

    assert_eq!(resolution.outputs.as_ref(), &[expected]);
}
