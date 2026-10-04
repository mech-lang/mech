use mech_core::{
    BuiltinKindPredicate, BuiltinScalarKind, CardinalitySpec, DimensionExpr, FloatWidth,
    InputKindScheme, IntegerInterval, IntegerWidth, KindConstraint, KindExpr, KindField,
    KindParameter, KindParameterId, KindScheme, ResolvedType, SchemaBody, SchemaDraft, SchemaField,
    TypeConstraintEnvironment, TypeConstraintOrigin, builtin_scalar_named_kind,
    predicate_unary_same,
};

fn scalar_satisfies(kind: BuiltinScalarKind, predicate: BuiltinKindPredicate) -> bool {
    ResolvedType::new(kind.kind_expr(), Box::new([]))
        .unwrap()
        .satisfies(predicate)
}

#[test]
fn existing_builtin_ordinals_are_unchanged() {
    let expected = [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U16,
        BuiltinScalarKind::U32,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::U128,
        BuiltinScalarKind::I8,
        BuiltinScalarKind::I16,
        BuiltinScalarKind::I32,
        BuiltinScalarKind::I64,
        BuiltinScalarKind::I128,
        BuiltinScalarKind::F32,
        BuiltinScalarKind::F64,
        BuiltinScalarKind::C64,
        BuiltinScalarKind::R64,
        BuiltinScalarKind::String,
        BuiltinScalarKind::Bool,
    ];
    for (ordinal, kind) in expected.into_iter().enumerate() {
        assert_eq!(kind.kind_id().get(), ordinal as u32);
        assert_eq!(BuiltinScalarKind::from_kind_id(kind.kind_id()), Some(kind));
    }
    assert_eq!(BuiltinScalarKind::C32.kind_id().get(), 16);
}

#[test]
fn c32_and_c64_are_distinct() {
    let c32 = BuiltinScalarKind::from_schema_body(&SchemaBody::Complex(FloatWidth::W32)).unwrap();
    let c64 = BuiltinScalarKind::from_schema_body(&SchemaBody::Complex(FloatWidth::W64)).unwrap();
    assert_ne!(c32, c64);
    assert_ne!(c32.kind_expr(), c64.kind_expr());
}

#[test]
fn schema_scalar_round_trip_uses_one_registry() {
    for kind in BuiltinScalarKind::ALL {
        assert_eq!(
            BuiltinScalarKind::from_schema_body(&kind.schema_body()),
            Some(kind)
        );
        let (id, path) =
            builtin_scalar_named_kind(mech_core::hash_str(kind.canonical_name())).unwrap();
        assert_eq!(id, kind.kind_id());
        assert_eq!(path, kind.canonical_path().unwrap());
    }
}

#[test]
fn scalar_predicate_membership_matches_the_closed_table() {
    assert!(scalar_satisfies(
        BuiltinScalarKind::U64,
        BuiltinKindPredicate::Number
    ));
    assert!(scalar_satisfies(
        BuiltinScalarKind::F64,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::Bool,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::String,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C64,
        BuiltinKindPredicate::Ordered
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::U64,
        BuiltinKindPredicate::Negatable
    ));
    assert!(scalar_satisfies(
        BuiltinScalarKind::I64,
        BuiltinKindPredicate::Negatable
    ));
}

#[test]
fn builtin_kind_expression_is_named_by_the_registry() {
    assert_eq!(
        BuiltinScalarKind::F32.kind_expr(),
        KindExpr::Named(BuiltinScalarKind::F32.kind_id())
    );
}

#[test]
fn number_contains_every_numeric_scalar() {
    for kind in BuiltinScalarKind::ALL {
        assert_eq!(
            scalar_satisfies(kind, BuiltinKindPredicate::Number),
            !matches!(kind, BuiltinScalarKind::String | BuiltinScalarKind::Bool),
            "{kind:?}",
        );
    }
}

#[test]
fn number_rejects_bool_and_string() {
    assert!(!scalar_satisfies(
        BuiltinScalarKind::Bool,
        BuiltinKindPredicate::Number
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::String,
        BuiltinKindPredicate::Number
    ));
}

#[test]
fn ordered_rejects_complex() {
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C32,
        BuiltinKindPredicate::Ordered
    ));
    assert!(!scalar_satisfies(
        BuiltinScalarKind::C64,
        BuiltinKindPredicate::Ordered
    ));
}

#[test]
fn negatable_rejects_unsigned() {
    for kind in [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U16,
        BuiltinScalarKind::U32,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::U128,
    ] {
        assert!(!scalar_satisfies(kind, BuiltinKindPredicate::Negatable));
    }
}

#[test]
fn range_endpoint_accepts_index_integer_and_float() {
    assert!(
        ResolvedType::new(KindExpr::Index, Box::new([]))
            .unwrap()
            .satisfies(BuiltinKindPredicate::RangeEndpoint)
    );
    for kind in [
        BuiltinScalarKind::U8,
        BuiltinScalarKind::U64,
        BuiltinScalarKind::I32,
        BuiltinScalarKind::F32,
        BuiltinScalarKind::F64,
    ] {
        assert!(scalar_satisfies(kind, BuiltinKindPredicate::RangeEndpoint));
    }
}

#[test]
fn nested_schema_keyability_becomes_predicate_evidence() {
    let kind = ResolvedType::from_schema_body(
        &SchemaBody::Tuple(
            vec![
                SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W32),
                SchemaBody::Option(Box::new(SchemaBody::Set {
                    element: Box::new(SchemaBody::String),
                    cardinality: CardinalitySpec::Exact(mech_core::DimensionExpr::Constant(2)),
                })),
            ]
            .into_boxed_slice(),
        ),
        &[],
    )
    .unwrap();
    assert!(kind.satisfies(BuiltinKindPredicate::Keyable));
}

#[test]
fn equal_types_intersect_conflicting_predicate_evidence() {
    let left = ResolvedType::from_schema_body(&SchemaBody::String, &[]).unwrap();
    let right = ResolvedType::new(BuiltinScalarKind::String.kind_expr(), Box::new([])).unwrap();
    assert_eq!(left, right);
    for predicate in BuiltinKindPredicate::ALL {
        assert_eq!(
            left.satisfies(predicate),
            right.satisfies(predicate),
            "{predicate:?}"
        );
    }
}

#[test]
fn schema_and_direct_kind_predicates_agree_for_structural_products() {
    let cases = [
        (
            KindExpr::Tuple(Box::new([])),
            SchemaBody::Tuple(Box::new([])),
        ),
        (
            KindExpr::Record(Box::new([])),
            SchemaBody::Record(Box::new([])),
        ),
        (
            KindExpr::Tuple(
                vec![BuiltinScalarKind::String.kind_expr(), KindExpr::Index].into_boxed_slice(),
            ),
            SchemaBody::Tuple(vec![SchemaBody::String, SchemaBody::Index].into_boxed_slice()),
        ),
    ];
    for (kind, schema) in cases {
        let direct = ResolvedType::new(kind, Box::new([])).unwrap();
        let derived = ResolvedType::from_schema_body(&schema, &[]).unwrap();
        assert_eq!(direct, derived);
        for predicate in BuiltinKindPredicate::ALL {
            assert_eq!(
                direct.satisfies(predicate),
                derived.satisfies(predicate),
                "{} {predicate:?}",
                direct.semantic_name(),
            );
        }
        assert!(direct.satisfies(BuiltinKindPredicate::Equatable));
        assert!(direct.satisfies(BuiltinKindPredicate::Keyable));
    }
}

#[test]
fn integer_interval_predicates_agree_for_direct_and_schema_derived_types() {
    for width in [
        IntegerWidth::W8,
        IntegerWidth::W16,
        IntegerWidth::W32,
        IntegerWidth::W64,
        IntegerWidth::W128,
    ] {
        for upper_inclusive in [false, true] {
            for interval in [
                IntegerInterval::Unsigned {
                    width,
                    lower: 1,
                    upper: 10,
                    upper_inclusive,
                },
                IntegerInterval::Signed {
                    width,
                    lower: -10,
                    upper: -1,
                    upper_inclusive,
                },
            ] {
                let direct =
                    ResolvedType::new(KindExpr::IntegerInterval(interval), Box::new([])).unwrap();
                let schema = SchemaDraft {
                    dimension_parameters: Box::new([]),
                    body: SchemaBody::IntegerInterval(interval),
                }
                .finalize()
                .unwrap();
                let shape = schema.instantiate_shape(Box::new([])).unwrap();
                let derived = ResolvedType::from_schema(&schema, &shape).unwrap();
                assert_eq!(direct, derived);
                for predicate in BuiltinKindPredicate::ALL {
                    let expected = matches!(
                        predicate,
                        BuiltinKindPredicate::Equatable
                            | BuiltinKindPredicate::Keyable
                            | BuiltinKindPredicate::Ordered
                    );
                    assert_eq!(
                        direct.satisfies(predicate),
                        expected,
                        "{interval:?} {predicate:?}"
                    );
                    assert_eq!(
                        derived.satisfies(predicate),
                        expected,
                        "{interval:?} {predicate:?}"
                    );
                }
            }
        }
    }
}

#[test]
fn integer_interval_predicates_survive_structural_and_generic_constraints() {
    let interval = IntegerInterval::Unsigned {
        width: IntegerWidth::W8,
        lower: 1,
        upper: 10,
        upper_inclusive: false,
    };
    let element = KindExpr::IntegerInterval(interval);
    let element_body = SchemaBody::IntegerInterval(interval);
    let cases = [
        (element.clone(), element_body.clone()),
        (
            KindExpr::Option(Box::new(element.clone())),
            SchemaBody::Option(Box::new(element_body.clone())),
        ),
        (
            KindExpr::Matrix {
                element: Box::new(element.clone()),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            SchemaBody::Matrix {
                element: Box::new(element_body.clone()),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
        ),
        (
            KindExpr::Set {
                element: Box::new(element.clone()),
                cardinality: DimensionExpr::Constant(2),
            },
            SchemaBody::Set {
                element: Box::new(element_body.clone()),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
        ),
        (
            KindExpr::Tuple(
                vec![element.clone(), KindExpr::Option(Box::new(element.clone()))]
                    .into_boxed_slice(),
            ),
            SchemaBody::Tuple(
                vec![
                    element_body.clone(),
                    SchemaBody::Option(Box::new(element_body.clone())),
                ]
                .into_boxed_slice(),
            ),
        ),
        (
            KindExpr::Map {
                key: Box::new(element.clone()),
                value: Box::new(element.clone()),
                cardinality: DimensionExpr::Constant(2),
            },
            SchemaBody::Map {
                key: Box::new(element_body.clone()),
                value: Box::new(element_body.clone()),
                cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
        ),
        (
            KindExpr::Record(
                vec![KindField {
                    name: "value".into(),
                    kind: element.clone(),
                }]
                .into_boxed_slice(),
            ),
            SchemaBody::Record(
                vec![SchemaField {
                    name: "value".into(),
                    schema: element_body.clone(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            KindExpr::Table {
                columns: vec![KindField {
                    name: "value".into(),
                    kind: element.clone(),
                }]
                .into_boxed_slice(),
                rows: DimensionExpr::Constant(2),
            },
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "value".into(),
                    schema: element_body.clone(),
                }]
                .into_boxed_slice(),
                rows: CardinalitySpec::Exact(DimensionExpr::Constant(2)),
            },
        ),
    ];
    for (kind, body) in cases {
        let direct = ResolvedType::new(kind, Box::new([])).unwrap();
        let derived = ResolvedType::from_schema_body(&body, &[]).unwrap();
        assert_eq!(direct, derived);
        for predicate in BuiltinKindPredicate::ALL {
            assert_eq!(
                direct.satisfies(predicate),
                derived.satisfies(predicate),
                "{body:?} {predicate:?}"
            );
        }
        assert!(direct.satisfies(BuiltinKindPredicate::Equatable));
        assert_eq!(
            direct.satisfies(BuiltinKindPredicate::Keyable),
            !matches!(body, SchemaBody::Map { .. } | SchemaBody::Table { .. }),
        );
        assert_eq!(
            direct.satisfies(BuiltinKindPredicate::Ordered),
            matches!(body, SchemaBody::IntegerInterval(_)),
        );
        assert!(!direct.satisfies(BuiltinKindPredicate::Number));
        for predicate in [
            BuiltinKindPredicate::Equatable,
            BuiltinKindPredicate::Keyable,
            BuiltinKindPredicate::Ordered,
            BuiltinKindPredicate::Number,
            BuiltinKindPredicate::Integer,
            BuiltinKindPredicate::RangeEndpoint,
        ] {
            let scheme = &predicate_unary_same(predicate).unwrap()[0];
            for actual in [&direct, &derived] {
                let result = TypeConstraintEnvironment::new(TypeConstraintOrigin::new(
                    "interval/predicate-identity",
                    None,
                ))
                .solve_scheme(scheme, &[actual.clone()], None);
                if actual.satisfies(predicate) {
                    let result = result.unwrap();
                    assert_eq!(result.outputs.as_ref(), &[actual.clone()]);
                    assert!(result.outputs[0].satisfies(predicate));
                } else {
                    assert!(result.is_err(), "{body:?} {predicate:?}");
                }
            }
        }
    }
}

#[test]
fn invalid_integer_intervals_do_not_construct_resolved_types() {
    for interval in [
        IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 10,
            upper: 1,
            upper_inclusive: false,
        },
        IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 256,
            upper_inclusive: true,
        },
        IntegerInterval::Signed {
            width: IntegerWidth::W8,
            lower: -129,
            upper: 1,
            upper_inclusive: false,
        },
        IntegerInterval::Signed {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 1,
            upper_inclusive: false,
        },
    ] {
        assert!(!interval.is_valid());
        assert!(ResolvedType::new(KindExpr::IntegerInterval(interval), Box::new([])).is_err());
        assert!(
            ResolvedType::from_schema_body(&SchemaBody::IntegerInterval(interval), &[]).is_err()
        );
    }
}

#[test]
fn mixed_interval_type_origins_preserve_binary_constraint_evidence() {
    let interval = IntegerInterval::Signed {
        width: IntegerWidth::W32,
        lower: -10,
        upper: 10,
        upper_inclusive: true,
    };
    let direct = ResolvedType::new(KindExpr::IntegerInterval(interval), Box::new([])).unwrap();
    let derived =
        ResolvedType::from_schema_body(&SchemaBody::IntegerInterval(interval), &[]).unwrap();
    let parameter = KindParameterId::new(0);
    for predicate in [
        BuiltinKindPredicate::Equatable,
        BuiltinKindPredicate::Keyable,
        BuiltinKindPredicate::Ordered,
    ] {
        let scheme = KindScheme::new(
            vec![KindParameter {
                id: parameter,
                upper_bound: None,
            }]
            .into_boxed_slice(),
            Box::new([]),
            InputKindScheme::Fixed(vec![KindExpr::Parameter(parameter); 2].into_boxed_slice()),
            vec![KindExpr::Parameter(parameter)].into_boxed_slice(),
            vec![KindConstraint::Satisfies {
                kind: KindExpr::Parameter(parameter),
                predicate,
            }]
            .into_boxed_slice(),
        )
        .unwrap();
        for inputs in [
            [direct.clone(), derived.clone()],
            [derived.clone(), direct.clone()],
        ] {
            let result = TypeConstraintEnvironment::new(TypeConstraintOrigin::new(
                "interval/mixed-origin-binary",
                None,
            ))
            .solve_scheme(&scheme, &inputs, None)
            .unwrap();
            assert_eq!(result.outputs.as_ref(), &[direct.clone()]);
            for required in [
                BuiltinKindPredicate::Equatable,
                BuiltinKindPredicate::Keyable,
                BuiltinKindPredicate::Ordered,
            ] {
                assert!(result.outputs[0].satisfies(required));
            }
            assert!(!result.outputs[0].satisfies(BuiltinKindPredicate::Number));
        }
    }
}
