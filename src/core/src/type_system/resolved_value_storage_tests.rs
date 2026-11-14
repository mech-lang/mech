//! Canonical shape storage changes retain the complete witness/error contract.
use super::*;
use crate::{DimensionLifetime, IntegerWidth};

fn parameter(index: usize) -> DimensionExpr {
    DimensionExpr::Parameter(DimensionParameterId::new(index as u32))
}
fn declaration(index: usize, lower: DimensionExpr) -> DimensionParameterDeclaration {
    DimensionParameterDeclaration {
        id: DimensionParameterId::new(index as u32),
        origin: DimensionParameterOrigin::Explicit,
        lifetime: DimensionLifetime::Turn,
        lower_bound: lower,
        upper_bound: None,
    }
}
fn matrix(dimensions: Vec<DimensionExpr>, parameters: usize) -> Schema {
    SchemaDraft {
        dimension_parameters: (0..parameters)
            .map(|index| declaration(index, DimensionExpr::Constant(0)))
            .collect::<Vec<_>>()
            .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            dimensions: dimensions.into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap()
}

#[test]
fn storage_keeps_zero_inline_and_heap_shapes_current_across_visits() {
    for count in [0, 1, 16, 17, 65] {
        let dimensions = if count == 0 {
            vec![DimensionExpr::Constant(1)]
        } else {
            (0..count).map(parameter).collect()
        };
        let schema = matrix(dimensions, count);
        let saved_schema = schema.clone();
        for last in [0, 1, u64::MAX, 7] {
            let mut extents = vec![1; count.max(1)];
            if count > 0 {
                *extents.last_mut().unwrap() = last;
            }
            let shape = shape_for_resolved_extents(&schema, &extents).unwrap();
            let expected = if count == 0 { vec![] } else { extents.clone() };
            assert_eq!(shape.parameter_values(), expected);
            assert_eq!(schema, saved_schema);
        }
        assert!(shape_for_resolved_extents(&schema, &[]).is_err());
    }
}

#[test]
fn storage_keeps_compound_shared_witnesses_and_retry_on_both_sides_of_cutoff() {
    for count in [1, 16, 17, 65] {
        let mut dimensions = vec![
            DimensionExpr::Min(vec![parameter(0), DimensionExpr::Constant(5)].into()),
            DimensionExpr::Max(vec![parameter(0), DimensionExpr::Constant(2)].into()),
            parameter(0),
            DimensionExpr::Multiply(vec![parameter(0), DimensionExpr::Constant(2)].into()),
        ];
        dimensions.extend((1..count).map(parameter));
        let schema = matrix(dimensions, count);
        let mut extents = vec![5, 6, 6, 12];
        extents.extend(core::iter::repeat_n(1, count - 1));
        for invalid in [false, true, false] {
            extents[0] = if invalid { 4 } else { 5 };
            let result = shape_for_resolved_extents(&schema, &extents);
            if invalid {
                assert!(matches!(
                    result,
                    Err(TypeResolutionError::Incompatible { .. })
                ));
            } else {
                let mut expected = vec![1; count];
                expected[0] = 6;
                assert_eq!(result.unwrap().parameter_values(), expected);
            }
        }
    }
}

#[test]
fn storage_reuses_unpublished_parameters_and_keeps_validation_error_order() {
    let schema = matrix(vec![parameter(0)], 1);
    let seed = schema.instantiate_shape(vec![3].into()).unwrap();
    let alias = seed.clone();
    let pointer = seed.parameter_values().as_ptr();
    let mut values = seed.into_parameter_values();
    assert_eq!(values.as_ptr(), pointer);
    values[0] = 5;
    let changed = schema.instantiate_shape(values).unwrap();
    assert_eq!(changed.parameter_values().as_ptr(), pointer);
    assert_eq!(alias.parameter_values(), &[3]);
    assert_eq!(changed.parameter_values(), &[5]);

    let overflow = SchemaDraft {
        dimension_parameters: vec![
            declaration(0, DimensionExpr::Constant(u64::MAX)),
            declaration(
                1,
                DimensionExpr::Add(vec![parameter(0), DimensionExpr::Constant(1)].into()),
            ),
        ]
        .into_boxed_slice(),
        body: SchemaBody::Matrix {
            element: Box::new(SchemaBody::UnsignedInteger(IntegerWidth::W8)),
            // Referenced declarations survive canonical parameter pruning.
            dimensions: vec![parameter(0), parameter(1)].into_boxed_slice(),
        },
    }
    .finalize()
    .unwrap();
    for extents in [vec![], vec![1], vec![1, 2]] {
        assert_eq!(
            shape_for_resolved_extents(&overflow, &extents),
            Err(TypeResolutionError::semantic(
                SemanticModelError::DimensionOverflowV1
            )),
            "lower-bound failure must precede arity and witness errors"
        );
    }
}
