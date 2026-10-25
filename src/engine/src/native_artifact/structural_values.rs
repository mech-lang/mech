//! Canonical child trees covered by the native projection's draft witness.
//! Closed word matrices additionally prepay packed-to-draft expansion.
//! Field-label drafts and foreign Dynamic roots still require their own witnesses.
use mech_core::SchemaBody;

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
