//! Canonical child trees covered by the native projection's draft witness.
//! Closed word matrices additionally prepay packed-to-draft expansion.
//! Field-label drafts and foreign Dynamic roots still require their own witnesses.
use mech_core::SchemaBody;

/// A shallow unsigned buffer. Its dimensions are resolved by the maintained
/// constructor against the current child's own shape, never another schema's
/// parameter indices.
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
pub(crate) fn word_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if dimensions.len() == 2
            && matches!(element.as_ref(), SchemaBody::UnsignedInteger(_)))
}

/// Symbolic component compatibility for tuple/record construction. Matrix
/// dimensions can belong to different parameter arenas; the selected live
/// binder must close both schemas and validate the actual extents. Constants
/// still agree statically. Other component families keep their exact witness.
#[cfg(any(feature = "pure_text", feature = "map_ops"))]
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

/// Only closed, shallow unsigned buffers are covered by the packed expansion
/// witness. Dynamic dimensions and nested element owners remain unsupported.
pub(crate) fn closed_word_matrix(body: &SchemaBody) -> bool {
    matches!(body, SchemaBody::Matrix { element, dimensions }
        if dimensions.len() == 2
            && dimensions.iter().all(|d| matches!(d, mech_core::DimensionExpr::Constant(_)))
            && matches!(element.as_ref(), SchemaBody::UnsignedInteger(_)))
}

pub(crate) fn supported(body: &SchemaBody) -> bool {
    fn visit(body: &SchemaBody, depth: usize) -> bool {
        if depth >= 32 {
            return false;
        }
        match body {
            SchemaBody::Bool
            | SchemaBody::Id
            | SchemaBody::Index
            | SchemaBody::Atom(_)
            | SchemaBody::String
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
            | SchemaBody::Complex(_)
            | SchemaBody::Rational64 => true,
            SchemaBody::Matrix { .. } => closed_word_matrix(body),
            SchemaBody::Tuple(items) => items.iter().all(|item| visit(item, depth + 1)),
            SchemaBody::Option(item) | SchemaBody::Set { element: item, .. } => {
                visit(item, depth + 1)
            }
            SchemaBody::Map { key, value, .. } => visit(key, depth + 1) && visit(value, depth + 1),
            SchemaBody::Enum { variants, .. } => variants.iter().all(|variant| {
                variant
                    .payload
                    .as_ref()
                    .is_none_or(|item| visit(item, depth + 1))
            }),
            _ => false,
        }
    }
    visit(body, 0)
}
