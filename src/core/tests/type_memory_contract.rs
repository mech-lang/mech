use mech_core::*;

fn nominal_key(byte: u8) -> NominalKey {
    NominalKey::from_bytes([byte; 32])
}

fn schema(body: SchemaBody) -> Schema {
    SchemaDraft {
        dimension_parameters: Vec::new().into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap()
}

fn parameter(
    ordinal: u32,
    lifetime: DimensionLifetime,
    lower: u64,
    upper: Option<u64>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(ordinal),
        origin: DimensionParameterOrigin::Explicit,
        lifetime,
        lower_bound: DimensionExpr::Constant(lower),
        upper_bound: upper.map(DimensionExpr::Constant),
    }
}

fn parameterized_schema(
    parameters: Vec<DimensionParameterDeclaration>,
    body: SchemaBody,
) -> Schema {
    SchemaDraft {
        dimension_parameters: parameters.into_boxed_slice(),
        body,
    }
    .finalize()
    .unwrap()
}

fn addressing(
    positional_rank: Option<u64>,
    named_members: bool,
    keyed_members: bool,
) -> AddressingContract {
    AddressingContract {
        positional_rank,
        named_members,
        keyed_members,
    }
}

fn canonicalization(
    self_describing: bool,
    recursive: bool,
    tagged: bool,
    ordered_keys: bool,
    unique_keys: bool,
) -> CanonicalizationContract {
    CanonicalizationContract {
        self_describing,
        recursive,
        tagged,
        ordered_keys,
        unique_keys,
    }
}

fn accounting(
    payload: PayloadAccounting,
    population: PopulationAccounting,
    tag: bool,
    ordered_index: bool,
    column_directory: bool,
) -> AccountingContract {
    AccountingContract {
        payload,
        population,
        auxiliary: AuxiliaryAccounting {
            tag,
            ordered_index,
            column_directory,
        },
    }
}

fn expected(
    topology: MemoryTopology,
    extent: MemoryExtent,
    extent_evolution: ExtentEvolution,
    addressing: AddressingContract,
    canonicalization: CanonicalizationContract,
    accounting: AccountingContract,
) -> TypeMemoryContract {
    TypeMemoryContract {
        topology,
        extent,
        extent_evolution,
        addressing,
        canonicalization,
        accounting,
    }
}

fn scalar_expected(kind: ScalarMemoryKind) -> TypeMemoryContract {
    expected(
        MemoryTopology::Scalar(kind),
        MemoryExtent::Single,
        ExtentEvolution::Fixed,
        addressing(None, false, false),
        canonicalization(false, false, false, false, false),
        accounting(
            PayloadAccounting::FixedWidth,
            PopulationAccounting::Single,
            false,
            false,
            false,
        ),
    )
}

#[test]
fn every_schema_body_variant_has_the_complete_declared_mapping() {
    let scalar_cases = [
        (SchemaBody::Bool, ScalarMemoryKind::Bool),
        (
            SchemaBody::UnsignedInteger(IntegerWidth::W16),
            ScalarMemoryKind::Unsigned(IntegerWidth::W16),
        ),
        (
            SchemaBody::SignedInteger(IntegerWidth::W32),
            ScalarMemoryKind::Signed(IntegerWidth::W32),
        ),
        (
            SchemaBody::FloatingPoint(FloatWidth::W32),
            ScalarMemoryKind::Floating(FloatWidth::W32),
        ),
        (
            SchemaBody::Complex(FloatWidth::W64),
            ScalarMemoryKind::Complex(FloatWidth::W64),
        ),
        (SchemaBody::Rational64, ScalarMemoryKind::Rational64),
        (SchemaBody::Id, ScalarMemoryKind::Id),
        (SchemaBody::Index, ScalarMemoryKind::Index),
        (SchemaBody::Atom(nominal_key(1)), ScalarMemoryKind::Atom),
    ];
    for (body, kind) in scalar_cases {
        assert_eq!(
            schema(body).type_memory_contract().unwrap(),
            scalar_expected(kind)
        );
    }

    assert_eq!(
        schema(SchemaBody::Dynamic).type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Dynamic,
            MemoryExtent::Single,
            ExtentEvolution::TurnUnbounded,
            addressing(None, false, false),
            canonicalization(true, true, false, false, false),
            accounting(
                PayloadAccounting::SelfDescribing,
                PopulationAccounting::Single,
                false,
                false,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::String).type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Scalar(ScalarMemoryKind::String),
            MemoryExtent::Single,
            ExtentEvolution::Fixed,
            addressing(Some(1), false, false),
            canonicalization(false, false, false, false, false),
            accounting(
                PayloadAccounting::VariableWidth,
                PopulationAccounting::Single,
                false,
                false,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::Enum {
            key: nominal_key(2),
            variants: vec![
                EnumVariantSchema {
                    name: "none".to_owned(),
                    payload: None,
                },
                EnumVariantSchema {
                    name: "some".to_owned(),
                    payload: Some(SchemaBody::Bool),
                },
            ]
            .into_boxed_slice(),
        })
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::Tagged { variants: 2 },
            MemoryExtent::Single,
            ExtentEvolution::Fixed,
            addressing(None, false, false),
            canonicalization(false, true, true, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                true,
                false,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::Option(Box::new(SchemaBody::String)))
            .type_memory_contract()
            .unwrap(),
        expected(
            MemoryTopology::Tagged { variants: 2 },
            MemoryExtent::Single,
            ExtentEvolution::Fixed,
            addressing(None, false, false),
            canonicalization(false, true, true, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                true,
                false,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::Tuple(
            vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
        ))
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::Product {
                members: 2,
                named: false,
            },
            MemoryExtent::FixedArity(2),
            ExtentEvolution::Fixed,
            addressing(Some(1), false, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::FixedArity,
                false,
                false,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::Record(
            vec![
                SchemaField {
                    name: "left".to_owned(),
                    schema: SchemaBody::Bool,
                },
                SchemaField {
                    name: "right".to_owned(),
                    schema: SchemaBody::String,
                },
            ]
            .into_boxed_slice(),
        ))
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::Product {
                members: 2,
                named: true,
            },
            MemoryExtent::FixedArity(2),
            ExtentEvolution::Fixed,
            addressing(None, true, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::FixedArity,
                false,
                false,
                false,
            ),
        )
    );

    let dimensions =
        vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)].into_boxed_slice();
    assert_eq!(
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: dimensions.clone(),
        })
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::DenseSequence { rank: 2 },
            MemoryExtent::Dimensions(dimensions),
            ExtentEvolution::Fixed,
            addressing(Some(2), false, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ShapeResolved,
                false,
                false,
                false,
            ),
        )
    );

    let exact_rows = CardinalitySpec::Exact(DimensionExpr::Constant(5));
    assert_eq!(
        schema(SchemaBody::Table {
            columns: vec![SchemaField {
                name: "value".to_owned(),
                schema: SchemaBody::Bool,
            }]
            .into_boxed_slice(),
            rows: exact_rows.clone(),
        })
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::Columnar { columns: 1 },
            MemoryExtent::Cardinality(exact_rows),
            ExtentEvolution::Fixed,
            addressing(Some(2), true, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ExactCardinality,
                false,
                false,
                true,
            ),
        )
    );

    let exact_set = CardinalitySpec::Exact(DimensionExpr::Constant(2));
    assert_eq!(
        schema(SchemaBody::Set {
            element: Box::new(SchemaBody::String),
            cardinality: exact_set.clone(),
        })
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::OrderedSet,
            MemoryExtent::Cardinality(exact_set),
            ExtentEvolution::Fixed,
            addressing(None, false, true),
            canonicalization(false, true, false, true, true),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ExactCardinality,
                false,
                true,
                false,
            ),
        )
    );

    let dynamic_map = CardinalitySpec::Dynamic { upper_bound: None };
    assert_eq!(
        schema(SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(SchemaBody::String),
            cardinality: dynamic_map.clone(),
        })
        .type_memory_contract()
        .unwrap(),
        expected(
            MemoryTopology::OrderedMap,
            MemoryExtent::Cardinality(dynamic_map),
            ExtentEvolution::TurnUnbounded,
            addressing(None, false, true),
            canonicalization(false, true, false, true, true),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ValueCardinality,
                false,
                true,
                false,
            ),
        )
    );

    assert_eq!(
        schema(SchemaBody::ReifiedType)
            .type_memory_contract()
            .unwrap(),
        expected(
            MemoryTopology::ReifiedType,
            MemoryExtent::Single,
            ExtentEvolution::Fixed,
            addressing(None, false, false),
            canonicalization(true, false, false, false, false),
            accounting(
                PayloadAccounting::SelfDescribing,
                PopulationAccounting::Single,
                false,
                false,
                false,
            ),
        )
    );
}

#[test]
fn nested_contracts_propagate_evolution_without_copying_child_structure() {
    let turn = DimensionParameterId::new(0);
    let record = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 1, None)],
        SchemaBody::Record(
            vec![SchemaField {
                name: "samples".to_owned(),
                schema: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::Bool),
                    dimensions: vec![DimensionExpr::Parameter(turn)].into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
        ),
    );
    assert_eq!(
        record.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Product {
                members: 1,
                named: true,
            },
            MemoryExtent::FixedArity(1),
            ExtentEvolution::TurnUnbounded,
            addressing(None, true, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::FixedArity,
                false,
                false,
                false,
            ),
        )
    );

    let matrix_of_string = schema(SchemaBody::Matrix {
        element: Box::new(SchemaBody::String),
        dimensions: vec![DimensionExpr::Constant(2)].into_boxed_slice(),
    });
    assert_eq!(
        matrix_of_string.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::DenseSequence { rank: 1 },
            MemoryExtent::Dimensions(vec![DimensionExpr::Constant(2)].into_boxed_slice(),),
            ExtentEvolution::Fixed,
            addressing(Some(1), false, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ShapeResolved,
                false,
                false,
                false,
            ),
        )
    );

    let table_with_dynamic_column = schema(SchemaBody::Table {
        columns: vec![SchemaField {
            name: "any".to_owned(),
            schema: SchemaBody::Dynamic,
        }]
        .into_boxed_slice(),
        rows: DimensionExpr::Constant(1).into(),
    });
    assert_eq!(
        table_with_dynamic_column.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Columnar { columns: 1 },
            MemoryExtent::Cardinality(CardinalitySpec::Exact(DimensionExpr::Constant(1))),
            ExtentEvolution::TurnUnbounded,
            addressing(Some(2), true, false),
            canonicalization(false, true, false, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ExactCardinality,
                false,
                false,
                true,
            ),
        )
    );

    let set_of_tuple_keys = schema(SchemaBody::Set {
        element: Box::new(SchemaBody::Tuple(
            vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
        )),
        cardinality: DimensionExpr::Constant(3).into(),
    });
    assert_eq!(
        set_of_tuple_keys.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::OrderedSet,
            MemoryExtent::Cardinality(CardinalitySpec::Exact(DimensionExpr::Constant(3))),
            ExtentEvolution::Fixed,
            addressing(None, false, true),
            canonicalization(false, true, false, true, true),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ExactCardinality,
                false,
                true,
                false,
            ),
        )
    );

    let map_with_recursive_values = schema(SchemaBody::Map {
        key: Box::new(SchemaBody::String),
        value: Box::new(SchemaBody::Option(Box::new(SchemaBody::Tuple(
            vec![SchemaBody::Bool, SchemaBody::String].into_boxed_slice(),
        )))),
        cardinality: DimensionExpr::Constant(1).into(),
    });
    assert_eq!(
        map_with_recursive_values.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::OrderedMap,
            MemoryExtent::Cardinality(CardinalitySpec::Exact(DimensionExpr::Constant(1))),
            ExtentEvolution::Fixed,
            addressing(None, false, true),
            canonicalization(false, true, false, true, true),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ExactCardinality,
                false,
                true,
                false,
            ),
        )
    );

    let enum_with_dynamic_collection = schema(SchemaBody::Enum {
        key: nominal_key(3),
        variants: vec![EnumVariantSchema {
            name: "values".to_owned(),
            payload: Some(SchemaBody::Set {
                element: Box::new(SchemaBody::Bool),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            }),
        }]
        .into_boxed_slice(),
    });
    assert_eq!(
        enum_with_dynamic_collection.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Tagged { variants: 1 },
            MemoryExtent::Single,
            ExtentEvolution::TurnUnbounded,
            addressing(None, false, false),
            canonicalization(false, true, true, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                true,
                false,
                false,
            ),
        )
    );

    let bounded = DimensionParameterId::new(0);
    let option_with_bounded_matrix = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 1, Some(8))],
        SchemaBody::Option(Box::new(SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(bounded)].into_boxed_slice(),
        })),
    );
    assert_eq!(
        option_with_bounded_matrix.type_memory_contract().unwrap(),
        expected(
            MemoryTopology::Tagged { variants: 2 },
            MemoryExtent::Single,
            ExtentEvolution::TurnBounded,
            addressing(None, false, false),
            canonicalization(false, true, true, false, false),
            accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::Single,
                true,
                false,
                false,
            ),
        )
    );
}

#[test]
fn dimension_operators_resolve_with_independent_axis_evolution() {
    let activation = DimensionParameterId::new(0);
    let bounded = DimensionParameterId::new(1);
    let unbounded = DimensionParameterId::new(2);
    let dimensions = vec![
        DimensionExpr::Constant(2),
        DimensionExpr::Parameter(activation),
        DimensionExpr::Parameter(bounded),
        DimensionExpr::Parameter(unbounded),
        DimensionExpr::Add(
            vec![
                DimensionExpr::Parameter(activation),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        ),
        DimensionExpr::Multiply(
            vec![
                DimensionExpr::Parameter(bounded),
                DimensionExpr::Constant(2),
            ]
            .into_boxed_slice(),
        ),
        DimensionExpr::Min(
            vec![
                DimensionExpr::Parameter(activation),
                DimensionExpr::Parameter(bounded),
            ]
            .into_boxed_slice(),
        ),
        DimensionExpr::Max(
            vec![
                DimensionExpr::Parameter(bounded),
                DimensionExpr::Parameter(unbounded),
            ]
            .into_boxed_slice(),
        ),
    ];
    let matrix = parameterized_schema(
        vec![
            parameter(0, DimensionLifetime::Activation, 1, None),
            parameter(1, DimensionLifetime::Turn, 1, Some(10)),
            parameter(2, DimensionLifetime::Turn, 1, None),
        ],
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: dimensions.into_boxed_slice(),
        },
    );
    let shape = matrix
        .instantiate_shape(vec![3, 4, 5].into_boxed_slice())
        .unwrap();
    let contract = matrix.resolved_type_memory_contract(&shape).unwrap();
    assert_eq!(
        contract,
        ResolvedTypeMemoryContract {
            topology: MemoryTopology::DenseSequence { rank: 8 },
            extent: ResolvedMemoryExtent::Dimensions(
                vec![
                    ResolvedAxisExtent {
                        value: 2,
                        evolution: ExtentEvolution::Fixed,
                    },
                    ResolvedAxisExtent {
                        value: 3,
                        evolution: ExtentEvolution::ActivationFixed,
                    },
                    ResolvedAxisExtent {
                        value: 4,
                        evolution: ExtentEvolution::TurnBounded,
                    },
                    ResolvedAxisExtent {
                        value: 5,
                        evolution: ExtentEvolution::TurnUnbounded,
                    },
                    ResolvedAxisExtent {
                        value: 4,
                        evolution: ExtentEvolution::ActivationFixed,
                    },
                    ResolvedAxisExtent {
                        value: 8,
                        evolution: ExtentEvolution::TurnBounded,
                    },
                    ResolvedAxisExtent {
                        value: 3,
                        evolution: ExtentEvolution::TurnBounded,
                    },
                    ResolvedAxisExtent {
                        value: 5,
                        evolution: ExtentEvolution::TurnUnbounded,
                    },
                ]
                .into_boxed_slice(),
            ),
            extent_evolution: ExtentEvolution::TurnUnbounded,
            addressing: addressing(Some(8), false, false),
            canonicalization: canonicalization(false, true, false, false, false),
            accounting: accounting(
                PayloadAccounting::Recursive,
                PopulationAccounting::ShapeResolved,
                false,
                false,
                false,
            ),
        }
    );
}

#[test]
fn exact_and_dynamic_cardinalities_resolve_without_inventing_population() {
    let activation = DimensionParameterId::new(0);
    let exact = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Activation, 0, Some(10))],
        SchemaBody::Set {
            element: Box::new(SchemaBody::Bool),
            cardinality: CardinalitySpec::Exact(DimensionExpr::Parameter(activation)),
        },
    );
    let exact_shape = exact.instantiate_shape(vec![4].into_boxed_slice()).unwrap();
    let exact_contract = exact.resolved_type_memory_contract(&exact_shape).unwrap();
    assert_eq!(
        exact_contract.extent,
        ResolvedMemoryExtent::ExactCardinality(4)
    );
    assert_eq!(
        exact_contract.extent_evolution,
        ExtentEvolution::ActivationFixed
    );

    let bounded = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Activation, 0, Some(10))],
        SchemaBody::Map {
            key: Box::new(SchemaBody::Bool),
            value: Box::new(SchemaBody::String),
            cardinality: CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Parameter(activation)),
            },
        },
    );
    let bounded_shape = bounded
        .instantiate_shape(vec![7].into_boxed_slice())
        .unwrap();
    let bounded_contract = bounded
        .resolved_type_memory_contract(&bounded_shape)
        .unwrap();
    assert_eq!(
        bounded_contract.extent,
        ResolvedMemoryExtent::DynamicCardinality {
            upper_bound: Some(7),
        }
    );
    assert_eq!(
        bounded_contract.extent_evolution,
        ExtentEvolution::TurnBounded
    );

    let unbounded = schema(SchemaBody::Set {
        element: Box::new(SchemaBody::Bool),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    });
    let unbounded_shape = unbounded
        .instantiate_shape(Vec::new().into_boxed_slice())
        .unwrap();
    assert_eq!(
        unbounded
            .resolved_type_memory_contract(&unbounded_shape)
            .unwrap()
            .extent,
        ResolvedMemoryExtent::DynamicCardinality { upper_bound: None }
    );
}

#[test]
fn resolution_revalidates_foreign_shapes_and_checked_arithmetic() {
    let id = DimensionParameterId::new(0);
    let source = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 0, Some(10))],
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(id)].into_boxed_slice(),
        },
    );
    let foreign_shape = source
        .instantiate_shape(vec![9].into_boxed_slice())
        .unwrap();
    let narrower = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 0, Some(4))],
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(id)].into_boxed_slice(),
        },
    );
    assert!(matches!(
        narrower.resolved_type_memory_contract(&foreign_shape),
        Err(SemanticModelError::ShapeBoundViolationV1 { value: 9, .. })
    ));

    let wide_source = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 0, None)],
        SchemaBody::Matrix {
            element: Box::new(SchemaBody::Bool),
            dimensions: vec![DimensionExpr::Parameter(id)].into_boxed_slice(),
        },
    );
    let maximum_shape = wide_source
        .instantiate_shape(vec![u64::MAX].into_boxed_slice())
        .unwrap();
    let overflow = parameterized_schema(
        vec![parameter(0, DimensionLifetime::Turn, 0, None)],
        SchemaBody::Table {
            columns: Vec::new().into_boxed_slice(),
            rows: CardinalitySpec::Exact(DimensionExpr::Multiply(
                vec![DimensionExpr::Parameter(id), DimensionExpr::Constant(2)].into_boxed_slice(),
            )),
        },
    );
    assert!(matches!(
        overflow.resolved_type_memory_contract(&maximum_shape),
        Err(SemanticModelError::DimensionOverflowV1)
    ));
}

#[test]
fn derivation_preserves_schema_keys_and_canonical_bytes() {
    let representative_schemas = vec![
        schema(SchemaBody::Bool),
        schema(SchemaBody::Matrix {
            element: Box::new(SchemaBody::String),
            dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(1)]
                .into_boxed_slice(),
        }),
        schema(SchemaBody::Record(
            vec![SchemaField {
                name: "value".to_owned(),
                schema: SchemaBody::String,
            }]
            .into_boxed_slice(),
        )),
        schema(SchemaBody::Set {
            element: Box::new(SchemaBody::Bool),
            cardinality: DimensionExpr::Constant(2).into(),
        }),
        schema(SchemaBody::Map {
            key: Box::new(SchemaBody::String),
            value: Box::new(SchemaBody::Bool),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        }),
        schema(SchemaBody::Table {
            columns: vec![SchemaField {
                name: "value".to_owned(),
                schema: SchemaBody::Bool,
            }]
            .into_boxed_slice(),
            rows: DimensionExpr::Constant(1).into(),
        }),
    ];

    for schema in representative_schemas {
        let key_before = schema.key();
        let bytes_before = schema.canonical_bytes();
        let shape = schema
            .instantiate_shape(Vec::new().into_boxed_slice())
            .unwrap();
        let _type_contract = schema.type_memory_contract().unwrap();
        let _resolved_contract = schema.resolved_type_memory_contract(&shape).unwrap();
        assert_eq!(schema.key(), key_before);
        assert_eq!(schema.canonical_bytes(), bytes_before);
    }
}

#[test]
fn memory_contract_types_do_not_acquire_serde_derives() {
    let module_source = include_str!("../src/memory_contract/mod.rs");
    let contract_source = include_str!("../src/memory_contract/type_contract.rs");
    for forbidden in ["Serialize", "Deserialize"] {
        assert!(!module_source.contains(forbidden));
        assert!(!contract_source.contains(forbidden));
    }
}
