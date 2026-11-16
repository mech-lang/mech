//! Selected results generalize computed axes, separately from identity renaming.
use super::*;

pub(super) fn selected_matrix_result_schema(source: &SchemaDraft) -> Option<SchemaDraft> {
    mech_core::generalize_computed_matrix_axes(source)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn draft(dimensions: Vec<DimensionExpr>) -> SchemaDraft {
        SchemaDraft {
            body: SchemaBody::Matrix {
                element: Box::new(SchemaBody::UnsignedInteger(mech_core::IntegerWidth::W64)),
                dimensions: dimensions.into_boxed_slice(),
            },
            dimension_parameters: vec![DimensionParameterDeclaration {
                id: DimensionParameterId::new(0),
                origin: DimensionParameterOrigin::Inferred,
                lifetime: DimensionLifetime::Turn,
                lower_bound: DimensionExpr::Constant(0),
                upper_bound: None,
            }]
            .into_boxed_slice(),
        }
    }
    fn sum() -> DimensionExpr {
        DimensionExpr::Add(
            vec![
                DimensionExpr::Parameter(DimensionParameterId::new(0)),
                DimensionExpr::Constant(1),
            ]
            .into_boxed_slice(),
        )
    }
    #[test]
    fn equal_computed_axes_remain_equal_and_constant_axes_remain_fixed() {
        let selected = selected_matrix_result_schema(&draft(vec![sum(), sum()])).unwrap();
        let SchemaBody::Matrix { dimensions, .. } = selected.body else {
            panic!()
        };
        assert_eq!(dimensions[0], dimensions[1]);
        let selected =
            selected_matrix_result_schema(&draft(vec![DimensionExpr::Constant(1), sum()])).unwrap();
        let SchemaBody::Matrix { dimensions, .. } = selected.body else {
            panic!()
        };
        assert_eq!(dimensions[0], DimensionExpr::Constant(1));
        assert!(matches!(dimensions[1], DimensionExpr::Parameter(_)));
    }
    #[test]
    fn fixed_bounded_and_persistent_axes_are_not_generalized() {
        assert!(
            selected_matrix_result_schema(&draft(vec![
                DimensionExpr::Constant(1),
                DimensionExpr::Add(
                    vec![DimensionExpr::Constant(1), DimensionExpr::Constant(2)].into_boxed_slice()
                )
            ]))
            .is_none()
        );
        for variant in 0..3 {
            let mut schema = draft(vec![DimensionExpr::Constant(1), sum()]);
            match variant {
                0 => schema.dimension_parameters[0].lower_bound = DimensionExpr::Constant(1),
                1 => schema.dimension_parameters[0].upper_bound = Some(DimensionExpr::Constant(32)),
                _ => schema.dimension_parameters[0].lifetime = DimensionLifetime::Activation,
            }
            assert!(selected_matrix_result_schema(&schema).is_none());
        }
    }
}
