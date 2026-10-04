#![cfg(feature = "full")]

use mech_core::{
    AccessMode, AliasPolicy, BoundCall, BoundCallOrigin, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, ExecutionTarget, ExternalInteraction,
    InputPortLayout, InputPortPolicy, IntegerInterval, IntegerWidth, KindExpr,
    OperationContractDeclaration, OperationId, OutputConstruction, OutputPortPolicy,
    ResolvedOperationDescriptor, ResolvedOutputSchemaRule, ResolvedType, ResolvedValueDescriptor,
    RuntimeFunctionId, SchemaBody, SchemaDraft, SchemaField, ShapeRule, ValueCell, ValueDataDraft,
    materialize_resolved_output, shape_for_value_data,
};
use nalgebra::{DMatrix, DVector, RowDVector};

fn dimensions(descriptor: &ResolvedValueDescriptor) -> &[DimensionExpr] {
    match descriptor.schema().body() {
        SchemaBody::Matrix { dimensions, .. } => dimensions,
        other => panic!("expected matrix schema, found {other:?}"),
    }
}

fn test_operation_descriptor(name: &str) -> ResolvedOperationDescriptor {
    ResolvedOperationDescriptor::from_name(
        name,
        OperationContractDeclaration {
            inputs: InputPortLayout::Fixed(
                vec![InputPortPolicy {
                    access: AccessMode::Read,
                    delivery: DeliveryMode::Signal,
                }]
                .into_boxed_slice(),
            ),
            outputs: vec![OutputPortPolicy {
                access: AccessMode::Write,
                delivery: DeliveryMode::Signal,
                construction: OutputConstruction::FullWrite {
                    shape: ShapeRule::Declared,
                },
                alias: AliasPolicy::NoAlias,
                change_detection: ChangeDetectionPolicy::AlwaysChanged,
            }]
            .into_boxed_slice(),
            interaction: ExternalInteraction::Pure,
        },
    )
    .unwrap()
}
use mech_core::Ref;

#[test]
fn descriptors_exact_check_semantics_before_storage() {
    let expected = ValueCell::from_exact(1.0_f64)
        .unwrap()
        .resolved_descriptor()
        .unwrap();
    let compatible = ValueCell::from_exact(2.0_f64).unwrap();
    compatible.validate_descriptor(&expected).unwrap();

    let incompatible = ValueCell::from_exact(2_u64).unwrap();
    assert!(incompatible.validate_descriptor(&expected).is_err());
}

#[test]
fn row_column_and_dynamic_matrix_orientation_remain_semantic() {
    let row = ValueCell::from_exact_matrix_ref(Ref::new(RowDVector::<f64>::zeros(3)), 1, 3)
        .unwrap()
        .resolved_descriptor()
        .unwrap();
    let column = ValueCell::from_exact_matrix_ref(Ref::new(DVector::<f64>::zeros(3)), 3, 1)
        .unwrap()
        .resolved_descriptor()
        .unwrap();
    let matrix = ValueCell::from_exact_matrix_ref(Ref::new(DMatrix::<f64>::zeros(1, 3)), 1, 3)
        .unwrap()
        .resolved_descriptor()
        .unwrap();

    assert!(matches!(
        dimensions(&row),
        [DimensionExpr::Constant(1), DimensionExpr::Parameter(_)]
    ));
    assert!(matches!(
        dimensions(&column),
        [DimensionExpr::Parameter(_), DimensionExpr::Constant(1)]
    ));
    assert!(matches!(
        dimensions(&matrix),
        [DimensionExpr::Parameter(_), DimensionExpr::Parameter(_)]
    ));
}

#[test]
fn bound_call_retains_the_selected_semantic_and_physical_identities() {
    let descriptor = ValueCell::from_exact(1.0_f64)
        .unwrap()
        .resolved_descriptor()
        .unwrap();
    let operation = OperationId::from_name("test/r4-operation");
    let runtime = RuntimeFunctionId::from_name("TestR4Runtime");
    let binding = BoundCall::syntax_directed(
        test_operation_descriptor("test/r4-operation"),
        vec![descriptor.clone()].into_boxed_slice(),
        vec![descriptor].into_boxed_slice(),
        runtime,
        ExecutionTarget::DirectRuntime,
    )
    .unwrap();

    assert_eq!(binding.operation(), operation);
    assert_eq!(binding.origin(), &BoundCallOrigin::SyntaxDirected);
    assert_eq!(binding.runtime_function(), Some(runtime));
    assert_eq!(binding.target(), ExecutionTarget::DirectRuntime);
}

#[test]
fn semantic_output_materialization_preserves_dynamic_collection_schema() {
    let input = ValueCell::from_schema_data(
        SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
        },
        ValueDataDraft::Set(
            vec![ValueDataDraft::Index(1), ValueDataDraft::Index(2)].into_boxed_slice(),
        ),
    )
    .unwrap()
    .resolved_descriptor()
    .unwrap();
    let output = materialize_resolved_output(
        input.resolved_type(),
        &ResolvedOutputSchemaRule::FromInput(0),
        &[input.clone()],
        Box::new([]),
    )
    .unwrap();

    assert_eq!(output.resolved_type(), input.resolved_type());
    assert!(matches!(
        output.schema().body(),
        SchemaBody::Set {
            cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            ..
        }
    ));
}

#[test]
fn turn_varying_collection_shapes_retain_one_type_contract() {
    let schema = bounded_dynamic_set_schema();
    let empty = ResolvedValueDescriptor::from_schema(
        schema.clone(),
        schema
            .instantiate_shape(vec![0].into_boxed_slice())
            .unwrap(),
    )
    .unwrap();
    let populated = ResolvedValueDescriptor::from_schema(
        schema.clone(),
        schema
            .instantiate_shape(vec![3].into_boxed_slice())
            .unwrap(),
    )
    .unwrap();

    assert_ne!(empty.shape(), populated.shape());
    assert!(empty.has_same_type_contract(&populated));
}

#[test]
fn published_collection_shape_is_derived_from_its_materialized_data() {
    let schema = bounded_dynamic_set_schema();
    let data = ValueDataDraft::Set(
        vec![
            ValueDataDraft::Index(1),
            ValueDataDraft::Index(2),
            ValueDataDraft::Index(3),
        ]
        .into_boxed_slice(),
    );

    let shape = shape_for_value_data(&schema, &data, &[], None).unwrap();
    assert_eq!(shape.parameter_values(), &[3]);
}

fn bounded_dynamic_set_schema() -> mech_core::Schema {
    SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
        body: SchemaBody::Set {
            element: Box::new(SchemaBody::Index),
            cardinality: CardinalitySpec::Dynamic {
                upper_bound: Some(DimensionExpr::Parameter(DimensionParameterId::new(0))),
            },
        },
    }
    .finalize()
    .unwrap()
}

#[test]
fn cartesian_product_and_powerset_materialize_dynamic_schemas_semantically() {
    fn set(element: SchemaBody) -> ResolvedValueDescriptor {
        ValueCell::from_schema_data(
            SchemaBody::Set {
                element: Box::new(element),
                cardinality: CardinalitySpec::Dynamic { upper_bound: None },
            },
            ValueDataDraft::Set(Box::new([])),
        )
        .unwrap()
        .resolved_descriptor()
        .unwrap()
    }

    let left_element = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W8);
    let right_element = SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W16);
    let left = set(left_element.clone());
    let right = set(right_element.clone());
    let product_template = set(SchemaBody::Tuple(
        vec![left_element, right_element].into_boxed_slice(),
    ));
    let product = materialize_resolved_output(
        product_template.resolved_type(),
        &ResolvedOutputSchemaRule::DynamicSetCartesianProduct,
        &[left.clone(), right],
        Box::new([]),
    )
    .unwrap();
    assert_eq!(product.resolved_type(), product_template.resolved_type());

    let powerset_template = set(left.schema().body().clone());
    let powerset = materialize_resolved_output(
        powerset_template.resolved_type(),
        &ResolvedOutputSchemaRule::DynamicSetPowerset,
        &[left],
        Box::new([]),
    )
    .unwrap();
    assert_eq!(powerset.resolved_type(), powerset_template.resolved_type());
}

#[test]
fn compound_dimension_expressions_materialize_exact_witnesses() {
    for (dimension, extent, witness) in [
        (
            DimensionExpr::Add(
                vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(2),
                ]
                .into_boxed_slice(),
            ),
            5,
            3,
        ),
        (
            DimensionExpr::Multiply(
                vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(2),
                ]
                .into_boxed_slice(),
            ),
            6,
            3,
        ),
        (
            DimensionExpr::Min(
                vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(5),
                ]
                .into_boxed_slice(),
            ),
            4,
            4,
        ),
        (
            DimensionExpr::Max(
                vec![
                    DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    DimensionExpr::Constant(2),
                ]
                .into_boxed_slice(),
            ),
            4,
            4,
        ),
    ] {
        let resolved = ResolvedType::new(
            KindExpr::Matrix {
                element: Box::new(KindExpr::Index),
                dimensions: vec![dimension, DimensionExpr::Constant(1)].into_boxed_slice(),
            },
            vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
        )
        .unwrap();
        let descriptor = materialize_resolved_output(
            &resolved,
            &ResolvedOutputSchemaRule::FromResolvedType,
            &[],
            vec![extent, 1].into_boxed_slice(),
        )
        .unwrap();
        assert_eq!(descriptor.shape().parameter_values(), &[witness]);
    }
}

#[test]
fn shared_dimension_witnesses_reject_inconsistent_extents() {
    let axis = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let resolved = ResolvedType::new(
        KindExpr::Matrix {
            element: Box::new(KindExpr::Index),
            dimensions: vec![axis.clone(), axis].into_boxed_slice(),
        },
        vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Inferred,
            lifetime: DimensionLifetime::Turn,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: None,
        }]
        .into_boxed_slice(),
    )
    .unwrap();

    assert!(
        materialize_resolved_output(
            &resolved,
            &ResolvedOutputSchemaRule::FromResolvedType,
            &[],
            vec![2, 3].into_boxed_slice(),
        )
        .is_err()
    );
}

fn interval_output_cases(interval: IntegerInterval) -> Vec<(&'static str, SchemaBody, Box<[u64]>)> {
    interval_output_cases_with_element(SchemaBody::IntegerInterval(interval))
}

fn interval_output_cases_with_element(
    element: SchemaBody,
) -> Vec<(&'static str, SchemaBody, Box<[u64]>)> {
    let fields = || {
        vec![SchemaField {
            name: "bounded".into(),
            schema: element.clone(),
        }]
        .into_boxed_slice()
    };
    let cardinality = CardinalitySpec::Exact(DimensionExpr::Constant(2));
    vec![
        ("scalar", element.clone(), Box::new([])),
        (
            "option",
            SchemaBody::Option(Box::new(element.clone())),
            Box::new([]),
        ),
        (
            "matrix",
            SchemaBody::Matrix {
                element: Box::new(element.clone()),
                dimensions: vec![DimensionExpr::Constant(2), DimensionExpr::Constant(3)]
                    .into_boxed_slice(),
            },
            vec![2, 3].into_boxed_slice(),
        ),
        (
            "tuple",
            SchemaBody::Tuple(
                vec![
                    element.clone(),
                    SchemaBody::Option(Box::new(element.clone())),
                ]
                .into_boxed_slice(),
            ),
            Box::new([]),
        ),
        ("record", SchemaBody::Record(fields()), Box::new([])),
        (
            "table",
            SchemaBody::Table {
                columns: fields(),
                rows: cardinality.clone(),
            },
            vec![2].into_boxed_slice(),
        ),
        (
            "set",
            SchemaBody::Set {
                element: Box::new(element.clone()),
                cardinality: cardinality.clone(),
            },
            vec![2].into_boxed_slice(),
        ),
        (
            "map",
            SchemaBody::Map {
                key: Box::new(element.clone()),
                value: Box::new(element.clone()),
                cardinality: cardinality.clone(),
            },
            vec![2].into_boxed_slice(),
        ),
        (
            "nested",
            SchemaBody::Tuple(
                vec![
                    SchemaBody::Option(Box::new(SchemaBody::Record(fields()))),
                    SchemaBody::Map {
                        key: Box::new(element.clone()),
                        value: Box::new(SchemaBody::Option(Box::new(SchemaBody::Set {
                            element: Box::new(element),
                            cardinality: CardinalitySpec::Exact(DimensionExpr::Constant(1)),
                        }))),
                        cardinality,
                    },
                ]
                .into_boxed_slice(),
            ),
            Box::new([]),
        ),
    ]
}

fn interval_descriptor(
    body: SchemaBody,
    declarations: Box<[DimensionParameterDeclaration]>,
    witnesses: Box<[u64]>,
) -> ResolvedValueDescriptor {
    let schema = SchemaDraft {
        dimension_parameters: declarations,
        body,
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape(witnesses).unwrap();
    ResolvedValueDescriptor::from_schema(schema, shape).unwrap()
}

#[test]
fn interval_output_materialization_preserves_exact_identity_for_every_authority() {
    for width in [
        IntegerWidth::W8,
        IntegerWidth::W16,
        IntegerWidth::W32,
        IntegerWidth::W64,
        IntegerWidth::W128,
    ] {
        for upper_inclusive in [false, true] {
            for interval in [
                IntegerInterval::Signed {
                    width,
                    lower: 1,
                    upper: 9,
                    upper_inclusive,
                },
                IntegerInterval::Unsigned {
                    width,
                    lower: 1,
                    upper: 9,
                    upper_inclusive,
                },
            ] {
                for (family, body, extents) in interval_output_cases(interval) {
                    let input = interval_descriptor(body.clone(), Box::new([]), Box::new([]));
                    for rule in [
                        ResolvedOutputSchemaRule::FromResolvedType,
                        ResolvedOutputSchemaRule::Declared(body.clone()),
                        ResolvedOutputSchemaRule::FromInput(0),
                    ] {
                        let output = materialize_resolved_output(
                            input.resolved_type(),
                            &rule,
                            core::slice::from_ref(&input),
                            extents.clone(),
                        )
                        .unwrap_or_else(|error| {
                            panic!("{interval:?} {family} {rule:?}: {error:?}")
                        });
                        assert_eq!(output, input, "{interval:?} {family} {rule:?}");
                    }
                }
            }
        }
    }
}

#[test]
fn interval_output_materialization_preserves_parameterized_shape_witnesses() {
    let interval = SchemaBody::IntegerInterval(IntegerInterval::Signed {
        width: IntegerWidth::W128,
        lower: -9,
        upper: -1,
        upper_inclusive: true,
    });
    let parameter = DimensionExpr::Parameter(DimensionParameterId::new(0));
    let matrix = SchemaBody::Matrix {
        element: Box::new(interval),
        dimensions: vec![
            parameter.clone(),
            DimensionExpr::Add(vec![parameter, DimensionExpr::Constant(1)].into_boxed_slice()),
        ]
        .into_boxed_slice(),
    };
    for (body, extents, witness) in [
        (matrix.clone(), vec![3, 4].into_boxed_slice(), 3),
        // Embedded matrices have no top-level extent authority. Their existing
        // contract reconstructs the declared lower-bound witness, not any
        // arbitrary witness belonging to an input value.
        (
            SchemaBody::Option(Box::new(matrix)),
            Box::<[u64]>::default(),
            2,
        ),
    ] {
        let input = interval_descriptor(
            body.clone(),
            vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Activation,
                lower_bound: DimensionExpr::Constant(2),
                upper_bound: Some(DimensionExpr::Constant(5)),
            }]
            .into_boxed_slice(),
            vec![witness].into_boxed_slice(),
        );
        for rule in [
            ResolvedOutputSchemaRule::FromResolvedType,
            ResolvedOutputSchemaRule::Declared(body.clone()),
            ResolvedOutputSchemaRule::FromInput(0),
        ] {
            let output = materialize_resolved_output(
                input.resolved_type(),
                &rule,
                core::slice::from_ref(&input),
                extents.clone(),
            )
            .unwrap();
            assert_eq!(output.schema(), input.schema());
            assert_eq!(output.shape(), input.shape());
            assert_eq!(output.resolved_type(), input.resolved_type());
            assert_eq!(output.shape().parameter_values(), &[witness]);
        }
    }
}

#[test]
fn interval_output_materialization_rejects_different_interval_templates() {
    let interval = IntegerInterval::Signed {
        width: IntegerWidth::W8,
        lower: 1,
        upper: 9,
        upper_inclusive: false,
    };
    for mismatched in [
        IntegerInterval::Signed {
            width: IntegerWidth::W8,
            lower: 2,
            upper: 9,
            upper_inclusive: false,
        },
        IntegerInterval::Signed {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 8,
            upper_inclusive: false,
        },
        IntegerInterval::Signed {
            width: IntegerWidth::W16,
            lower: 1,
            upper: 9,
            upper_inclusive: false,
        },
        IntegerInterval::Unsigned {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 9,
            upper_inclusive: false,
        },
        IntegerInterval::Signed {
            width: IntegerWidth::W8,
            lower: 1,
            upper: 9,
            upper_inclusive: true,
        },
    ] {
        for ((family, body, extents), (other_family, other_body, _)) in
            interval_output_cases(interval)
                .into_iter()
                .zip(interval_output_cases(mismatched))
        {
            assert_eq!(family, other_family);
            let expected = interval_descriptor(body, Box::new([]), Box::new([]));
            let input = interval_descriptor(other_body.clone(), Box::new([]), Box::new([]));
            for rule in [
                ResolvedOutputSchemaRule::Declared(other_body.clone()),
                ResolvedOutputSchemaRule::FromInput(0),
            ] {
                assert!(
                    materialize_resolved_output(
                        expected.resolved_type(),
                        &rule,
                        core::slice::from_ref(&input),
                        extents.clone(),
                    )
                    .is_err(),
                    "{family} {mismatched:?} {rule:?} must not replace {interval:?}"
                );
            }
        }
    }
}

#[test]
fn interval_output_materialization_rejects_erased_base_integer_templates() {
    let interval = IntegerInterval::Signed {
        width: IntegerWidth::W8,
        lower: 1,
        upper: 9,
        upper_inclusive: false,
    };
    for ((family, body, extents), (other_family, other_body, _)) in interval_output_cases(interval)
        .into_iter()
        .zip(interval_output_cases_with_element(
            SchemaBody::SignedInteger(IntegerWidth::W8),
        ))
    {
        assert_eq!(family, other_family);
        let expected = interval_descriptor(body, Box::new([]), Box::new([]));
        let input = interval_descriptor(other_body.clone(), Box::new([]), Box::new([]));
        for rule in [
            ResolvedOutputSchemaRule::Declared(other_body),
            ResolvedOutputSchemaRule::FromInput(0),
        ] {
            assert!(
                materialize_resolved_output(
                    expected.resolved_type(),
                    &rule,
                    core::slice::from_ref(&input),
                    extents.clone(),
                )
                .is_err(),
                "{family} {rule:?} must not erase the interval into its base storage kind"
            );
        }
    }
}

#[test]
fn interval_output_materialization_preserves_dynamic_set_rule_identity() {
    let left_element = SchemaBody::IntegerInterval(IntegerInterval::Signed {
        width: IntegerWidth::W8,
        lower: -9,
        upper: -1,
        upper_inclusive: false,
    });
    let right_element = SchemaBody::IntegerInterval(IntegerInterval::Unsigned {
        width: IntegerWidth::W128,
        lower: 1,
        upper: 9,
        upper_inclusive: true,
    });
    let dynamic_set = |element| SchemaBody::Set {
        element: Box::new(element),
        cardinality: CardinalitySpec::Dynamic { upper_bound: None },
    };
    let descriptor = |body| interval_descriptor(body, Box::new([]), Box::new([]));
    let left = descriptor(dynamic_set(left_element.clone()));
    let right = descriptor(dynamic_set(right_element.clone()));
    let product_template = descriptor(dynamic_set(SchemaBody::Tuple(
        vec![left_element, right_element].into_boxed_slice(),
    )));
    let product = materialize_resolved_output(
        product_template.resolved_type(),
        &ResolvedOutputSchemaRule::DynamicSetCartesianProduct,
        &[left.clone(), right],
        Box::new([]),
    )
    .unwrap();
    assert_eq!(product, product_template);

    let powerset_template = descriptor(dynamic_set(left.schema().body().clone()));
    let powerset = materialize_resolved_output(
        powerset_template.resolved_type(),
        &ResolvedOutputSchemaRule::DynamicSetPowerset,
        &[left],
        Box::new([]),
    )
    .unwrap();
    assert_eq!(powerset, powerset_template);
}
