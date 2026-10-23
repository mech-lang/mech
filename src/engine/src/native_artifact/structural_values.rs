//! Canonical child trees covered by the native projection's draft witness.
//! Packed collections, field-label drafts and foreign Dynamic roots require
//! their own expansion/ownership witnesses before this route can select them.
use mech_core::SchemaBody;

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
