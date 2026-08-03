//! Lossless semantic-kind reconstruction for zero-payload Kind constants.

use crate::{MResult, ValueKind, kind::Kind};

use super::invalid;

pub(super) fn value_kind_from_semantic_kind(kind: &Kind) -> MResult<ValueKind> {
    Ok(match kind {
        Kind::Any => ValueKind::Any,
        Kind::None => ValueKind::None,
        Kind::Empty => ValueKind::Empty,
        Kind::Atom(id, name) => ValueKind::Atom(*id, name.clone()),
        Kind::Enum(id, name) => ValueKind::Enum(*id, name.clone()),
        Kind::Id => ValueKind::Id,
        Kind::Index => ValueKind::Index,
        Kind::Map(key, value) => ValueKind::Map(
            Box::new(value_kind_from_semantic_kind(key)?),
            Box::new(value_kind_from_semantic_kind(value)?),
        ),
        Kind::Matrix(element, dimensions) => ValueKind::Matrix(
            Box::new(value_kind_from_semantic_kind(element)?),
            dimensions.clone(),
        ),
        Kind::Option(inner) => ValueKind::Option(Box::new(value_kind_from_semantic_kind(inner)?)),
        Kind::Record(fields) => ValueKind::Record(
            fields
                .iter()
                .map(|(name, child)| Ok((name.clone(), value_kind_from_semantic_kind(child)?)))
                .collect::<MResult<_>>()?,
        ),
        Kind::Reference(inner) => {
            ValueKind::Reference(Box::new(value_kind_from_semantic_kind(inner)?))
        }
        Kind::Set(element, max_len) => {
            ValueKind::Set(Box::new(value_kind_from_semantic_kind(element)?), *max_len)
        }
        Kind::Table(columns, primary_key) => ValueKind::Table(
            columns
                .iter()
                .map(|(name, child)| Ok((name.clone(), value_kind_from_semantic_kind(child)?)))
                .collect::<MResult<_>>()?,
            *primary_key,
        ),
        Kind::Tuple(types) => ValueKind::Tuple(
            types
                .iter()
                .map(value_kind_from_semantic_kind)
                .collect::<MResult<_>>()?,
        ),
        Kind::Kind(inner) => ValueKind::Kind(Box::new(value_kind_from_semantic_kind(inner)?)),
        Kind::Scalar(id) => scalar_value_kind_from_id(*id)?,
    })
}

fn scalar_value_kind_from_id(id: u64) -> MResult<ValueKind> {
    for (name, kind) in [
        ("u8", ValueKind::U8),
        ("u16", ValueKind::U16),
        ("u32", ValueKind::U32),
        ("u64", ValueKind::U64),
        ("u128", ValueKind::U128),
        ("i8", ValueKind::I8),
        ("i16", ValueKind::I16),
        ("i32", ValueKind::I32),
        ("i64", ValueKind::I64),
        ("i128", ValueKind::I128),
        ("f32", ValueKind::F32),
        ("f64", ValueKind::F64),
        ("c64", ValueKind::C64),
        ("r64", ValueKind::R64),
        ("string", ValueKind::String),
        ("bool", ValueKind::Bool),
    ] {
        if crate::hash_str(name) == id {
            return Ok(kind);
        }
    }
    invalid("Kind scalar ID does not identify a canonical runtime scalar")
}
