use mech_core::*;

fn schema(body: SchemaBody) -> Schema {
    SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap()
}

fn nominal_key(byte: u8) -> NominalKey {
    NominalKey::from_bytes([byte; 32])
}

#[test]
fn duplicate_names_and_non_keyable_collection_members_fail() {
    let duplicate = SchemaBody::Record(
        vec![
            SchemaField {
                name: "x".to_owned(),
                schema: SchemaBody::Bool,
            },
            SchemaField {
                name: "x".to_owned(),
                schema: SchemaBody::String,
            },
        ]
        .into_boxed_slice(),
    );
    assert!(matches!(
        SchemaDraft {
            dimension_parameters: Vec::new().into_boxed_slice(),
            body: duplicate,
        }
        .finalize(),
        Err(SemanticModelError::DuplicateSchemaNameV1 {
            category: SchemaNameCategory::RecordField,
            ..
        })
    ));

    for body in [
        SchemaBody::Table {
            columns: vec![
                SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::Bool,
                },
                SchemaField {
                    name: "x".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
            rows: DimensionExpr::Constant(0).into(),
        },
        SchemaBody::Enum {
            key: nominal_key(1),
            variants: vec![
                EnumVariantSchema {
                    name: "x".to_owned(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "x".to_owned(),
                    payload: Some(SchemaBody::Bool),
                },
            ]
            .into_boxed_slice(),
        },
    ] {
        assert!(matches!(
            SchemaDraft {
                dimension_parameters: Vec::new().into_boxed_slice(),
                body,
            }
            .finalize(),
            Err(SemanticModelError::DuplicateSchemaNameV1 { .. })
        ));
    }

    for body in [
        SchemaBody::Set {
            element: Box::new(SchemaBody::Complex(FloatWidth::W64)),
            cardinality: DimensionExpr::Constant(1).into(),
        },
        SchemaBody::Map {
            key: Box::new(SchemaBody::Complex(FloatWidth::W32)),
            value: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(1).into(),
        },
    ] {
        assert!(matches!(
            SchemaDraft {
                dimension_parameters: Vec::new().into_boxed_slice(),
                body,
            }
            .finalize(),
            Err(SemanticModelError::SchemaNotKeyableV1)
        ));
    }
}

#[test]
fn every_scalar_width_has_its_exact_frozen_u16_payload() {
    for (body, tag, width) in [
        (SchemaBody::UnsignedInteger(IntegerWidth::W8), 0x02, 8),
        (SchemaBody::UnsignedInteger(IntegerWidth::W16), 0x02, 16),
        (SchemaBody::UnsignedInteger(IntegerWidth::W32), 0x02, 32),
        (SchemaBody::UnsignedInteger(IntegerWidth::W64), 0x02, 64),
        (SchemaBody::UnsignedInteger(IntegerWidth::W128), 0x02, 128),
        (SchemaBody::SignedInteger(IntegerWidth::W8), 0x03, 8),
        (SchemaBody::SignedInteger(IntegerWidth::W16), 0x03, 16),
        (SchemaBody::SignedInteger(IntegerWidth::W32), 0x03, 32),
        (SchemaBody::SignedInteger(IntegerWidth::W64), 0x03, 64),
        (SchemaBody::SignedInteger(IntegerWidth::W128), 0x03, 128),
        (SchemaBody::FloatingPoint(FloatWidth::W32), 0x04, 32),
        (SchemaBody::FloatingPoint(FloatWidth::W64), 0x04, 64),
        (SchemaBody::Complex(FloatWidth::W32), 0x05, 32),
        (SchemaBody::Complex(FloatWidth::W64), 0x05, 64),
    ] {
        let bytes = schema(body).canonical_bytes();
        assert_eq!(&bytes[13..], &[tag, width as u8, 0]);
    }
}

#[test]
fn enum_record_and_table_order_is_semantic_and_preserved() {
    let left = schema(SchemaBody::Record(
        vec![
            SchemaField {
                name: "a".to_owned(),
                schema: SchemaBody::Bool,
            },
            SchemaField {
                name: "b".to_owned(),
                schema: SchemaBody::String,
            },
        ]
        .into_boxed_slice(),
    ));
    let right = schema(SchemaBody::Record(
        vec![
            SchemaField {
                name: "b".to_owned(),
                schema: SchemaBody::String,
            },
            SchemaField {
                name: "a".to_owned(),
                schema: SchemaBody::Bool,
            },
        ]
        .into_boxed_slice(),
    ));
    assert_ne!(left.canonical_bytes(), right.canonical_bytes());

    let enum_left = schema(SchemaBody::Enum {
        key: nominal_key(3),
        variants: vec![
            EnumVariantSchema {
                name: "A".to_owned(),
                payload: None,
            },
            EnumVariantSchema {
                name: "B".to_owned(),
                payload: Some(SchemaBody::Bool),
            },
        ]
        .into_boxed_slice(),
    });
    let enum_right = schema(SchemaBody::Enum {
        key: nominal_key(3),
        variants: vec![
            EnumVariantSchema {
                name: "B".to_owned(),
                payload: Some(SchemaBody::Bool),
            },
            EnumVariantSchema {
                name: "A".to_owned(),
                payload: None,
            },
        ]
        .into_boxed_slice(),
    });
    assert_ne!(enum_left.key(), enum_right.key());

    let table_left = schema(SchemaBody::Table {
        columns: vec![
            SchemaField {
                name: "a".to_owned(),
                schema: SchemaBody::Bool,
            },
            SchemaField {
                name: "b".to_owned(),
                schema: SchemaBody::String,
            },
        ]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(1).into(),
    });
    let table_right = schema(SchemaBody::Table {
        columns: vec![
            SchemaField {
                name: "b".to_owned(),
                schema: SchemaBody::String,
            },
            SchemaField {
                name: "a".to_owned(),
                schema: SchemaBody::Bool,
            },
        ]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(1).into(),
    });
    assert_ne!(table_left.canonical_bytes(), table_right.canonical_bytes());
}

fn bounded_matrix(lifetime: DimensionLifetime, upper: Option<u64>) -> Schema {
    SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: upper.map(DimensionExpr::Constant),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap()
}

#[test]
fn shape_counts_bounds_bytes_and_extent_evolution_are_exact() {
    let bounded = bounded_matrix(DimensionLifetime::Activation, Some(8));
    assert_eq!(bounded.extent_evolution(), ExtentEvolution::ActivationFixed);
    assert!(matches!(
        bounded.instantiate_shape(Vec::new().into_boxed_slice()),
        Err(SemanticModelError::ShapeParameterCountMismatchV1 {
            expected: 1,
            actual: 0
        })
    ));
    for value in [0, 9] {
        assert!(matches!(
            bounded.instantiate_shape(vec![value].into_boxed_slice()),
            Err(SemanticModelError::ShapeBoundViolationV1 { .. })
        ));
    }
    let shape = bounded
        .instantiate_shape(vec![7].into_boxed_slice())
        .unwrap();
    assert_eq!(shape.parameter_values(), &[7]);
    assert_eq!(
        shape.canonical_bytes().as_ref(),
        &[1, 1, 0, 0, 0, 7, 0, 0, 0, 0, 0, 0, 0]
    );

    assert_eq!(
        schema(SchemaBody::Bool).extent_evolution(),
        ExtentEvolution::Fixed
    );
    assert_eq!(
        bounded_matrix(DimensionLifetime::Turn, Some(8)).extent_evolution(),
        ExtentEvolution::TurnBounded
    );
    assert_eq!(
        bounded_matrix(DimensionLifetime::Turn, None).extent_evolution(),
        ExtentEvolution::TurnUnbounded
    );
}

#[test]
fn finalized_schemas_remove_unused_parameters_and_reject_holes() {
    let unused = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Bool,
    }
    .finalize()
    .unwrap();
    assert!(unused.dimension_parameters().is_empty());

    assert!(matches!(
        SchemaDraft {
            dimension_parameters: Vec::new().into_boxed_slice(),
            body: SchemaBody::Table {
                columns: Vec::new().into_boxed_slice(),
                rows: DimensionExpr::Hole.into(),
            },
        }
        .finalize(),
        Err(SemanticModelError::UnresolvedDimensionHole)
    ));
}

#[test]
fn finalization_normalizes_before_parameter_reachability() {
    let parameter = DimensionParameterId::new(0);
    let reduced = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: parameter,
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Multiply(
                vec![
                    DimensionExpr::Constant(0),
                    DimensionExpr::Parameter(parameter),
                ]
                .into_boxed_slice(),
            )]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let constant = schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![DimensionExpr::Constant(0)].into_boxed_slice(),
    });
    assert!(reduced.dimension_parameters().is_empty());
    assert_eq!(reduced.key(), constant.key());
    reduced
        .instantiate_shape(Vec::new().into_boxed_slice())
        .unwrap();

    let dependent = DimensionParameterId::new(1);
    let bounded = SchemaDraft {
        dimension_parameters: vec![
            DimensionParameterDeclaration {
                id: parameter,
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Multiply(
                    vec![
                        DimensionExpr::Constant(0),
                        DimensionExpr::Parameter(dependent),
                    ]
                    .into_boxed_slice(),
                ),
                upper_bound: None,
            },
            DimensionParameterDeclaration {
                id: dependent,
                origin: DimensionParameterOrigin::Explicit,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            },
        ]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(parameter)].into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    assert_eq!(bounded.dimension_parameters().len(), 1);
    assert_eq!(
        bounded.dimension_parameters()[0].lower_bound(),
        &DimensionExpr::Constant(0)
    );
}

#[test]
fn canonical_environment_orders_explicit_then_inferred_and_includes_bounds() {
    let declarations = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(1),
            upper_bound: None,
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(0)),
            upper_bound: Some(DimensionExpr::Constant(9)),
        },
    ];
    let finalized = SchemaDraft {
        dimension_parameters: declarations.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(1))]
                .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    assert_eq!(finalized.dimension_parameters().len(), 2);
    assert_eq!(
        finalized.dimension_parameters()[1].lower_bound(),
        &DimensionExpr::Parameter(DimensionParameterId::new(0))
    );

    let inferred = vec![
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
            upper_bound: None,
        },
    ];
    let inferred = SchemaDraft {
        dimension_parameters: inferred.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![
                DimensionExpr::Parameter(DimensionParameterId::new(1)),
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    let SchemaBody::Matrix { dimensions, .. } = inferred.body() else {
        panic!("expected matrix")
    };
    assert_eq!(
        dimensions.as_ref(),
        &[
            DimensionExpr::Parameter(DimensionParameterId::new(0)),
            DimensionExpr::Parameter(DimensionParameterId::new(1)),
        ]
    );
}

#[test]
fn canonical_environment_rejects_forward_references_and_cycles() {
    let forward = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(1)),
            upper_bound: None,
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        },
    ];
    let draft = |parameters: Vec<DimensionParameterDeclaration>| SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(DimensionParameterId::new(0))]
                .into_boxed_slice(),
        },
    };
    assert!(matches!(
        draft(forward).finalize(),
        Err(SemanticModelError::ForwardDimensionParameterReferenceV1 { .. })
    ));

    let cycle = vec![
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(1)),
            upper_bound: None,
        },
        DimensionParameterDeclaration {
            id: DimensionParameterId::new(1),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Activation,
            lower_bound: DimensionExpr::Parameter(DimensionParameterId::new(0)),
            upper_bound: None,
        },
    ];
    assert!(matches!(
        draft(cycle).finalize(),
        Err(SemanticModelError::CyclicDimensionParameterBoundsV1)
    ));
}

#[test]
fn cardinality_and_extent_evaluation_are_checked() {
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Table {
            columns: Vec::new().into_boxed_slice(),
            rows: DimensionExpr::Multiply(
                vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(2),
                ]
                .into_boxed_slice(),
            )
            .into(),
        },
    }
    .finalize()
    .unwrap();
    assert!(matches!(
        schema.instantiate_shape(vec![u64::MAX].into_boxed_slice()),
        Err(SemanticModelError::DimensionOverflowV1)
    ));
}

#[test]
fn matrix_shape_validation_checks_the_product_of_separate_dimensions() {
    let constant_overflow = schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![
            DimensionExpr::Constant(u64::MAX),
            DimensionExpr::Constant(2),
        ]
        .into_boxed_slice(),
    });
    assert!(matches!(
        constant_overflow.instantiate_shape(Vec::new().into_boxed_slice()),
        Err(SemanticModelError::DimensionOverflowV1)
    ));

    let parameterized_overflow = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    assert!(matches!(
        parameterized_overflow.instantiate_shape(vec![u64::MAX].into_boxed_slice()),
        Err(SemanticModelError::DimensionOverflowV1)
    ));

    let large_valid = schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::Bool),
        dimensions: vec![
            DimensionExpr::Constant(u64::MAX / 2),
            DimensionExpr::Constant(2),
        ]
        .into_boxed_slice(),
    });
    large_valid
        .instantiate_shape(Vec::new().into_boxed_slice())
        .unwrap();

    let nested_overflow = schema(SchemaBody::Tuple(
        vec![SchemaBody::Option(Box::new(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![
                DimensionExpr::Constant(u64::MAX),
                DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        }))]
        .into_boxed_slice(),
    ));
    assert!(matches!(
        nested_overflow.instantiate_shape(Vec::new().into_boxed_slice()),
        Err(SemanticModelError::DimensionOverflowV1)
    ));
}

#[test]
fn schema_table_ids_follow_canonical_bytes_not_insertion_order() {
    let bool_schema = schema(SchemaBody::Bool);
    let string_schema = schema(SchemaBody::String);

    let mut first = SchemaTableBuilder::new();
    let first_string = first.insert(string_schema.clone()).unwrap();
    let first_bool = first.insert(bool_schema.clone()).unwrap();
    let first_duplicate = first.insert(bool_schema.clone()).unwrap();
    let first = first.finish().unwrap();

    let mut second = SchemaTableBuilder::new();
    let second_bool = second.insert(bool_schema.clone()).unwrap();
    let second_string = second.insert(string_schema.clone()).unwrap();
    let second = second.finish().unwrap();

    assert_eq!(first.table.len(), 2);
    assert_eq!(
        first.resolve(first_bool).unwrap(),
        first.resolve(first_duplicate).unwrap()
    );
    assert_eq!(
        first.resolve(first_bool).unwrap(),
        second.resolve(second_bool).unwrap(),
        "Bool must retain the same canonical-byte ID"
    );
    assert_eq!(
        first.resolve(first_string).unwrap(),
        second.resolve(second_string).unwrap(),
        "String must retain the same canonical-byte ID"
    );
    assert_eq!(
        first.table.find_by_key(bool_schema.key()),
        Some(first.resolve(first_bool).unwrap())
    );
    assert_eq!(
        first.resolve(first_bool).unwrap().get() < first.resolve(first_string).unwrap().get(),
        bool_schema.canonical_bytes() < string_schema.canonical_bytes()
    );
    assert!(matches!(
        first.resolve(second_string),
        Err(SemanticModelError::InvalidSchemaHandleV1)
    ));
}
