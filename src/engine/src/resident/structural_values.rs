//! Symbolic compatibility for independently shaped aggregate components.
use mech_core::SchemaBody;

/// A shallow unsigned buffer. Its dimensions are resolved by the maintained
/// constructor against the current child's own shape, never another schema's
/// parameter indices.
pub(crate) fn word_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if dimensions.len() == 2
            && matches!(element.as_ref(), SchemaBody::UnsignedInteger(_)))
}

/// Canonical argument matrices also admit fixed record/tuple elements. Keep
/// this separate from `word_matrix`: aggregate cells need the budgeted snapshot
/// constructor, not the packed unsigned-buffer copy path.
pub(crate) fn carrier_matrix(body: &SchemaBody) -> bool {
    word_matrix(body)
        || matches!(body, SchemaBody::Matrix {element, dimensions}
        if dimensions.len() == 2 && crate::is_control_fixed_aggregate_element(element))
}

/// Symbolic component compatibility for tuple/record construction. Matrix
/// dimensions can belong to different parameter arenas; the selected live
/// binder must close both schemas and validate the actual extents. Constants
/// still agree statically. Other component families keep their exact witness.
pub(crate) fn carrier_component_matches(expected: &SchemaBody, actual: &SchemaBody) -> bool {
    if expected == actual {
        return true;
    }
    match (expected, actual) {
        (
            SchemaBody::Matrix {
                element: a,
                dimensions: ad,
            },
            SchemaBody::Matrix {
                element: b,
                dimensions: bd,
            },
        ) if carrier_matrix(expected) && carrier_matrix(actual) && a == b => {
            ad.iter().zip(bd).all(|(a, b)| match (a, b) {
                (mech_core::DimensionExpr::Constant(a), mech_core::DimensionExpr::Constant(b)) => {
                    a == b
                }
                _ => true,
            })
        }
        (SchemaBody::Tuple(a), SchemaBody::Tuple(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(a, b)| carrier_component_matches(a, b))
        }
        (SchemaBody::Record(a), SchemaBody::Record(b)) => {
            a.len() == b.len()
                && a.iter().zip(b).all(|(a, b)| {
                    a.name == b.name && carrier_component_matches(&a.schema, &b.schema)
                })
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mech_core::{DimensionExpr as D, DimensionParameterId, IntegerWidth, SchemaField};

    #[test]
    fn record_matrix_component_arenas_keep_constant_extents_and_field_kinds() {
        let record = |kind| {
            SchemaBody::Record(
                vec![SchemaField {
                    name: "length".into(),
                    schema: kind,
                }]
                .into_boxed_slice(),
            )
        };
        let matrix = |element, columns| SchemaBody::Matrix {
            element: Box::new(element),
            dimensions: vec![D::Constant(1), columns].into_boxed_slice(),
        };
        let element = record(SchemaBody::UnsignedInteger(IntegerWidth::W32));
        let a = matrix(element.clone(), D::Parameter(DimensionParameterId::new(0)));
        let b = matrix(element.clone(), D::Parameter(DimensionParameterId::new(2)));
        assert!(carrier_component_matches(&a, &b));
        assert!(!word_matrix(&a));
        assert!(!carrier_component_matches(
            &matrix(element.clone(), D::Constant(2)),
            &matrix(element.clone(), D::Constant(3))
        ));
        assert!(!carrier_component_matches(
            &a,
            &matrix(
                record(SchemaBody::UnsignedInteger(IntegerWidth::W16)),
                D::Parameter(DimensionParameterId::new(0))
            )
        ));
    }
}
