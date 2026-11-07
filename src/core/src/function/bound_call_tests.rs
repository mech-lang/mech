//! Current extents stay visit-local even when immutable call ports are retained.
use crate::{
    AccessMode, AliasPolicy, BoundCall, ChangeDetectionPolicy, DeliveryMode, DimensionExpr,
    DimensionLifetime, DimensionParameterDeclaration, DimensionParameterId,
    DimensionParameterOrigin, ExecutionTarget, ExternalInteraction, InputPortLayout,
    InputPortPolicy, IntegerWidth, OperationContractDeclaration, OutputConstruction,
    OutputPortPolicy, ResolvedOperationDescriptor, ResolvedValueDescriptor, RuntimeFunctionId,
    SchemaBody, SchemaDraft, SchemaField, ShapeRule,
};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec};

fn descriptor(lifetime: DimensionLifetime, extent: u64) -> ResolvedValueDescriptor {
    let schema = SchemaDraft {
        dimension_parameters: vec![DimensionParameterDeclaration {
            id: DimensionParameterId::new(0),
            origin: DimensionParameterOrigin::Explicit,
            lifetime,
            lower_bound: DimensionExpr::Constant(0),
            upper_bound: Some(DimensionExpr::Constant(8)),
        }]
        .into_boxed_slice(),
        body: SchemaBody::Record(
            vec![SchemaField {
                name: "payload".into(),
                schema: SchemaBody::Matrix {
                    element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W32)),
                    dimensions: vec![
                        DimensionExpr::Constant(1),
                        DimensionExpr::Parameter(DimensionParameterId::new(0)),
                    ]
                    .into_boxed_slice(),
                },
            }]
            .into_boxed_slice(),
        ),
    }
    .finalize()
    .unwrap();
    let shape = schema.instantiate_shape([extent].into()).unwrap();
    ResolvedValueDescriptor::from_schema(schema, shape).unwrap()
}

fn call(lifetime: DimensionLifetime) -> BoundCall {
    let descriptor = descriptor(lifetime, 2);
    BoundCall::syntax_directed(
        ResolvedOperationDescriptor::from_name(
            "test/descriptor-identity",
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
                        shape: ShapeRule::SameAsInput { input: 0 },
                    },
                    alias: AliasPolicy::NoAlias,
                    change_detection: ChangeDetectionPolicy::SemanticHash,
                }]
                .into_boxed_slice(),
                interaction: ExternalInteraction::Pure,
            },
        )
        .unwrap(),
        [descriptor.clone()].into(),
        [descriptor].into(),
        RuntimeFunctionId::from_name("test/descriptor-identity"),
        ExecutionTarget::DirectRuntime,
    )
    .unwrap()
}

#[test]
fn bound_call_refresh_keeps_retained_shapes_and_independent_operation_contracts() {
    let original = call(DimensionLifetime::Turn);
    let retained = original.clone();
    assert!(core::ptr::eq(
        original.operation_descriptor(),
        retained.operation_descriptor()
    ));
    assert!(core::ptr::eq(original.inputs(), retained.inputs()));
    assert!(core::ptr::eq(original.outputs(), retained.outputs()));
    for extent in [0, 3, 8, 2] {
        let current = descriptor(DimensionLifetime::Turn, extent);
        let refreshed = original
            .with_current_descriptors([current.clone()].into(), [current].into())
            .unwrap();
        assert_eq!(refreshed.inputs()[0].shape().parameter_values(), &[extent]);
        assert_eq!(refreshed.outputs()[0].shape().parameter_values(), &[extent]);
        assert_eq!(
            refreshed.operation_descriptor(),
            original.operation_descriptor()
        );
        assert_eq!(refreshed.implementation(), original.implementation());
        assert_eq!(refreshed.origin(), original.origin());
        assert_eq!(refreshed.target(), original.target());
        assert!(core::ptr::eq(
            refreshed.operation_descriptor(),
            original.operation_descriptor()
        ));
        assert!(!core::ptr::eq(refreshed.inputs(), original.inputs()));
        assert!(!core::ptr::eq(refreshed.outputs(), original.outputs()));
        assert_eq!(original, retained);
    }
    let mut altered = original.clone();
    let mut contract = altered.operation_descriptor().contract.clone();
    contract.outputs[0].change_detection = ChangeDetectionPolicy::AlwaysChanged;
    altered.resolve_operation_contract(&contract).unwrap();
    assert!(!core::ptr::eq(
        altered.operation_descriptor(),
        original.operation_descriptor()
    ));
    assert!(core::ptr::eq(altered.inputs(), original.inputs()));
    assert!(core::ptr::eq(altered.outputs(), original.outputs()));
    assert_ne!(
        altered.operation_descriptor(),
        original.operation_descriptor()
    );
    assert_eq!(original, retained);
    drop(original);
    assert_eq!(retained.inputs()[0].shape().parameter_values(), &[2]);
}

#[test]
fn bound_call_refresh_rejects_arity_type_and_fixed_lifetime_changes_then_retries() {
    for lifetime in [DimensionLifetime::Turn, DimensionLifetime::Activation] {
        let original = call(lifetime);
        let retained = original.clone();
        let current = descriptor(lifetime, 2);
        assert!(
            original
                .with_current_descriptors(Box::new([]), [current.clone()].into())
                .is_err()
        );
        let boolean = SchemaDraft {
            dimension_parameters: Box::new([]),
            body: SchemaBody::Bool,
        }
        .finalize()
        .unwrap();
        let boolean_shape = boolean.instantiate_shape(Box::new([])).unwrap();
        let boolean = ResolvedValueDescriptor::from_schema(boolean, boolean_shape).unwrap();
        assert!(
            original
                .with_current_descriptors([boolean].into(), [current.clone()].into())
                .is_err()
        );
        let changed = descriptor(lifetime, 3);
        assert_eq!(
            original
                .with_current_descriptors([changed.clone()].into(), [changed].into())
                .is_ok(),
            lifetime == DimensionLifetime::Turn
        );
        assert_eq!(original, retained);
        let retried = original
            .with_current_descriptors([current.clone()].into(), [current].into())
            .unwrap();
        assert_eq!(retried, original);
        assert_eq!(original, retained);
    }
}
