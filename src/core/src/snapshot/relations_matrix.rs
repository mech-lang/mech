//! Borrowed Matrix key ordering and in-place normalization under the SAME relation.
use super::{
    Ordering, SchemaBody, SequenceStorage, SnapshotCanonicalizationBudget, SnapshotValueError,
    ValueData, compare_key_data_with_budget, normalize_f32, normalize_f64, normalized_key_data,
    string_key_order, string_key_work,
};

/// A canonical Matrix has one declared shape shared by all keys at this site.
/// Its payload order is lexicographic; outer Value::key_cmp already orders
/// distinct complete shape tapes before entering this data-only relation.
pub(super) fn compare(
    element: &SchemaBody,
    left: &SequenceStorage,
    right: &SequenceStorage,
    budget: Option<&SnapshotCanonicalizationBudget>,
) -> Result<Ordering, SnapshotValueError> {
    macro_rules! packed {
        ($left:expr, $right:expr, $variant:ident) => {{
            for (left, right) in $left.iter().zip($right.iter()) {
                let order = compare_key_data_with_budget(
                    element,
                    &ValueData::$variant(*left),
                    &ValueData::$variant(*right),
                    budget,
                )?;
                if order != Ordering::Equal {
                    return Ok(order);
                }
            }
            $left.len().cmp(&$right.len())
        }};
    }
    let order = match (left, right) {
        (SequenceStorage::U8(left), SequenceStorage::U8(right)) => packed!(left, right, U8),
        (SequenceStorage::U16(left), SequenceStorage::U16(right)) => packed!(left, right, U16),
        (SequenceStorage::U32(left), SequenceStorage::U32(right)) => packed!(left, right, U32),
        (SequenceStorage::U64(left), SequenceStorage::U64(right)) => packed!(left, right, U64),
        (SequenceStorage::U128(left), SequenceStorage::U128(right)) => packed!(left, right, U128),
        (SequenceStorage::I8(left), SequenceStorage::I8(right)) => packed!(left, right, I8),
        (SequenceStorage::I16(left), SequenceStorage::I16(right)) => packed!(left, right, I16),
        (SequenceStorage::I32(left), SequenceStorage::I32(right)) => packed!(left, right, I32),
        (SequenceStorage::I64(left), SequenceStorage::I64(right)) => packed!(left, right, I64),
        (SequenceStorage::I128(left), SequenceStorage::I128(right)) => packed!(left, right, I128),
        (SequenceStorage::F32(left), SequenceStorage::F32(right)) => packed!(left, right, F32),
        (SequenceStorage::F64(left), SequenceStorage::F64(right)) => packed!(left, right, F64),
        (SequenceStorage::Rational64(left), SequenceStorage::Rational64(right)) => {
            for (left, right) in left.iter().zip(right.iter()) {
                let order = compare_key_data_with_budget(
                    element,
                    &ValueData::Rational64(left.clone()),
                    &ValueData::Rational64(right.clone()),
                    budget,
                )?;
                if order != Ordering::Equal {
                    return Ok(order);
                }
            }
            left.len().cmp(&right.len())
        }
        (SequenceStorage::Bool(left), SequenceStorage::Bool(right)) => packed!(left, right, Bool),
        (SequenceStorage::Id(left), SequenceStorage::Id(right)) => packed!(left, right, Id),
        (SequenceStorage::Index(left), SequenceStorage::Index(right)) => {
            packed!(left, right, Index)
        }
        (SequenceStorage::String(left), SequenceStorage::String(right)) => {
            for (left, right) in left.iter().zip(right.iter()) {
                if let Some(budget) = budget {
                    budget.charge(string_key_work(left, right))?;
                }
                let order = string_key_order(left, right);
                if order != Ordering::Equal {
                    return Ok(order);
                }
            }
            left.len().cmp(&right.len())
        }
        (SequenceStorage::Unit(left), SequenceStorage::Unit(right)) => {
            for _ in 0..(*left).min(*right) {
                compare_key_data_with_budget(element, &ValueData::Atom, &ValueData::Atom, budget)?;
            }
            left.cmp(right)
        }
        (SequenceStorage::Values(left), SequenceStorage::Values(right)) => {
            for (left, right) in left.iter().zip(right.iter()) {
                let order = compare_key_data_with_budget(element, left, right, budget)?;
                if order != Ordering::Equal {
                    return Ok(order);
                }
            }
            left.len().cmp(&right.len())
        }
        // Complex/Dynamic and differing physical variants cannot be a
        // validated keyable Matrix under one shared element schema.
        _ => return Err(SnapshotValueError::SchemaNotKeyableV1),
    };
    Ok(order)
}

/// Normalize owned scalar bits and nested keys without unpacking packed
/// storage or cloning any String/composite payload.
pub(super) fn normalize(
    element: &SchemaBody,
    mut values: SequenceStorage,
) -> Result<SequenceStorage, SnapshotValueError> {
    match &mut values {
        SequenceStorage::F32(values) => {
            for value in values.iter_mut() {
                *value = super::super::F32Bits::from_bits(normalize_f32(value.bits()));
            }
        }
        SequenceStorage::F64(values) => {
            for value in values.iter_mut() {
                *value = super::super::F64Bits::from_bits(normalize_f64(value.bits()));
            }
        }
        SequenceStorage::Values(values) => {
            for value in values.iter_mut() {
                let owned = core::mem::replace(value, ValueData::Atom);
                *value = normalized_key_data(element, owned)?;
            }
        }
        _ => {}
    }
    Ok(values)
}
