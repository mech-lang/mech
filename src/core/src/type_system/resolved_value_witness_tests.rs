//! Shared canonical witness regressions, not a native span shape evaluator.
use super::{
    DimensionExpr, DimensionParameterDeclaration, DimensionParameterId, DimensionParameterOrigin,
    Schema, SchemaBody, SchemaDraft, SemanticModelError, TypeConstraintFailure,
    TypeResolutionError, assign_unique_dimension_witness, dimension_witness_mismatch, invalid_rule,
    shape_for_resolved_extents,
};
use crate::{DimensionLifetime, IntegerWidth};

#[cfg(feature = "no_std")]
use alloc::{boxed::Box, format, vec, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, format, vec, vec::Vec};

// Test-only exact reference for the pre-optimization search. It is never a
// runtime alternative, and deliberately retains the original error verifier.
fn searched_witness(
    expression: &DimensionExpr,
    target: u64,
    parameter: DimensionParameterId,
    values: &mut [u64],
) -> Result<(), TypeResolutionError> {
    let index = parameter.get() as usize;
    if index >= values.len() {
        return Err(invalid_rule(format!("unknown dimension {parameter:?}")));
    }
    let mut lower = 0u64;
    let mut upper = target;
    while lower < upper {
        let candidate = lower + (upper - lower) / 2;
        values[index] = candidate;
        match crate::evaluate_dimension(expression, values) {
            Ok(actual) if actual < target => lower = candidate + 1,
            Ok(_) | Err(SemanticModelError::DimensionOverflowV1) => upper = candidate,
            Err(error) => return Err(TypeResolutionError::semantic(error)),
        }
    }
    values[index] = lower;
    dimension_witness_mismatch(expression, target, values)
}

fn assert_reference(
    expression: &DimensionExpr,
    target: u64,
    parameter: DimensionParameterId,
    initial: &[u64],
) {
    let mut expected_values = initial.to_vec();
    let mut actual_values = initial.to_vec();
    let expected = searched_witness(expression, target, parameter, &mut expected_values);
    let actual = assign_unique_dimension_witness(expression, target, parameter, &mut actual_values);
    assert_eq!(actual, expected, "{expression:?}, target={target}");
    assert_eq!(
        actual_values, expected_values,
        "{expression:?}, target={target}"
    );
}

fn parameter(index: u32) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(index))
}

fn declaration(
    index: u32,
    lower_bound: DimensionExpr,
    upper_bound: Option<DimensionExpr>,
) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(index),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Turn,
        lower_bound,
        upper_bound,
    }
}

fn matrix(
    dimensions: Vec<DimensionExpr>,
    declarations: Vec<DimensionParameterDeclaration>,
) -> Schema {
    SchemaDraft {
        dimension_parameters: declarations.into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W64)),
            dimensions: dimensions.into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap()
}

#[test]
fn direct_parameter_matches_old_search_for_zero_max_and_binary_boundaries() {
    let mut targets = vec![0, 1, 2, 3, u64::MAX];
    for shift in 0..u64::BITS {
        let power = 1u64 << shift;
        targets.extend([power.saturating_sub(1), power, power.saturating_add(1)]);
    }
    targets.sort_unstable();
    targets.dedup();
    for target in targets {
        assert_reference(
            &parameter(0),
            target,
            DimensionParameterId::new(0),
            &[19, 73],
        );
        assert_reference(
            &parameter(1),
            target,
            DimensionParameterId::new(1),
            &[19, 73],
        );
    }
}

#[test]
fn direct_parameter_keeps_index_errors_and_nonmatching_parameter_fallback() {
    for target in [0, 3, 4, u64::MAX] {
        assert_reference(&parameter(0), target, DimensionParameterId::new(0), &[]);
        assert_reference(&parameter(1), target, DimensionParameterId::new(1), &[3]);
        assert_reference(
            &parameter(0),
            target,
            DimensionParameterId::new(1),
            &[3, 73],
        );
    }
}

#[test]
fn direct_parameter_shape_keeps_full_u64_extents_and_fixed_axis_mismatch() {
    let dynamic = matrix(
        vec![DimensionExpr::Constant(1), parameter(0)],
        vec![declaration(0, DimensionExpr::Constant(0), None)],
    );
    for extent in [0, 1, 2, 3, u64::MAX] {
        assert_eq!(
            shape_for_resolved_extents(&dynamic, &[1, extent])
                .unwrap()
                .parameter_values(),
            &[extent]
        );
    }
    assert!(shape_for_resolved_extents(&dynamic, &[2, 3]).is_err());
    assert!(shape_for_resolved_extents(&dynamic, &[3]).is_err());
    let fixed = matrix(
        vec![DimensionExpr::Constant(1), DimensionExpr::Constant(4)],
        vec![],
    );
    assert!(shape_for_resolved_extents(&fixed, &[1, 4]).is_ok());
    assert_eq!(
        shape_for_resolved_extents(&fixed, &[1, 3]),
        Err(TypeResolutionError::incompatible(
            "resolved output descriptor",
            TypeConstraintFailure::IncompatibleDimensions {
                expected: "4".into(),
                actual: "3".into(),
            },
        ))
    );
}

#[test]
fn direct_parameter_shape_preserves_bounds_shared_witnesses_and_overflow() {
    let bounded = matrix(
        vec![DimensionExpr::Constant(1), parameter(0)],
        vec![declaration(
            0,
            DimensionExpr::Constant(2),
            Some(DimensionExpr::Constant(4)),
        )],
    );
    for extent in [2, 3, 4] {
        assert_eq!(
            shape_for_resolved_extents(&bounded, &[1, extent])
                .unwrap()
                .parameter_values(),
            &[extent]
        );
    }
    for extent in [0, 1, 5, u64::MAX] {
        assert_eq!(
            shape_for_resolved_extents(&bounded, &[1, extent]),
            Err(TypeResolutionError::semantic(
                SemanticModelError::ShapeBoundViolationV1 {
                    parameter: DimensionParameterId::new(0),
                    value: extent,
                    lower: 2,
                    upper: Some(4),
                }
            ))
        );
    }
    let shared = matrix(
        vec![parameter(0), parameter(0)],
        vec![declaration(0, DimensionExpr::Constant(0), None)],
    );
    assert_eq!(
        shape_for_resolved_extents(&shared, &[2, 2])
            .unwrap()
            .parameter_values(),
        &[2]
    );
    assert_eq!(
        shape_for_resolved_extents(&shared, &[2, 3]),
        Err(TypeResolutionError::incompatible(
            "resolved output descriptor",
            TypeConstraintFailure::IncompatibleDimensions {
                expected: "2".into(),
                actual: "3".into(),
            },
        ))
    );
    assert_eq!(
        shape_for_resolved_extents(&shared, &[u64::MAX, u64::MAX]),
        Err(TypeResolutionError::semantic(
            SemanticModelError::DimensionOverflowV1
        ))
    );
    let dependent = matrix(
        vec![parameter(0), parameter(1)],
        vec![
            declaration(0, DimensionExpr::Constant(0), None),
            declaration(
                1,
                DimensionExpr::Add(
                    vec![parameter(0), DimensionExpr::Constant(1)].into_boxed_slice(),
                ),
                None,
            ),
        ],
    );
    assert_eq!(
        shape_for_resolved_extents(&dependent, &[3, 5])
            .unwrap()
            .parameter_values(),
        &[3, 5]
    );
    assert!(shape_for_resolved_extents(&dependent, &[3, 3]).is_err());
}

#[test]
fn direct_parameter_keeps_compound_search_and_ambiguous_witness_regressions() {
    let expressions = [
        DimensionExpr::Add(vec![parameter(0), DimensionExpr::Constant(2)].into_boxed_slice()),
        DimensionExpr::Multiply(vec![parameter(0), DimensionExpr::Constant(2)].into_boxed_slice()),
        DimensionExpr::Min(vec![parameter(0), DimensionExpr::Constant(5)].into_boxed_slice()),
        DimensionExpr::Max(vec![parameter(0), DimensionExpr::Constant(2)].into_boxed_slice()),
    ];
    for expression in &expressions {
        for target in [0, 1, 2, 3, 6, u64::MAX] {
            assert_reference(expression, target, DimensionParameterId::new(0), &[0]);
        }
    }
    for (expression, extent, witness) in [
        (expressions[0].clone(), 5, 3),
        (expressions[1].clone(), 6, 3),
        (expressions[2].clone(), 4, 4),
        (expressions[3].clone(), 4, 4),
    ] {
        let schema = matrix(
            vec![DimensionExpr::Constant(1), expression],
            vec![declaration(0, DimensionExpr::Constant(0), None)],
        );
        assert_eq!(
            shape_for_resolved_extents(&schema, &[1, extent])
                .unwrap()
                .parameter_values(),
            &[witness]
        );
    }
}
