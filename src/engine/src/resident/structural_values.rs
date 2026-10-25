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
        ) if word_matrix(expected) && word_matrix(actual) && a == b => {
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
