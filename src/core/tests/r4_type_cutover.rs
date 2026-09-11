#![cfg(feature = "full")]

use mech_core::{
    AccessMode, AliasPolicy, BoundCall, BoundCallOrigin, CardinalitySpec, ChangeDetectionPolicy,
    DeliveryMode, DimensionExpr, DimensionLifetime, DimensionParameterDeclaration,
    DimensionParameterId, DimensionParameterOrigin, ExecutionTarget, ExternalInteraction,
    InputPortLayout, InputPortPolicy, KindExpr, OperationContractDeclaration, OperationId,
    OutputConstruction, OutputPortPolicy, ResolvedOperationDescriptor, ResolvedOutputSchemaRule,
    ResolvedType, ResolvedValueDescriptor, RuntimeFunctionId, SchemaBody, SchemaDraft, ShapeRule,
    ValueCell, ValueDataDraft, materialize_resolved_output, shape_for_value_data,
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
