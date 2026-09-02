//! Canonical positional-selector semantics shared by every execution target.

use crate::{SchemaBody, ValueData, snapshot::SequenceView};

/// Largest one-based selector value that is portable across supported hosts.
pub const PORTABLE_SELECTOR_INDEX_MAX: u64 = u32::MAX as u64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CanonicalSelectorError {
    UnsupportedSchema,
    NonFinite,
    NonPositive,
    OutOfRange,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CanonicalSelectorVisitError<E> {
    Selector(CanonicalSelectorError),
    Visitor(E),
}

pub fn is_positional_selector_schema(schema: &SchemaBody) -> bool {
    matches!(
        schema,
        SchemaBody::Index
            | SchemaBody::UnsignedInteger(_)
            | SchemaBody::SignedInteger(_)
            | SchemaBody::FloatingPoint(_)
    )
}

/// Converts one scalar selector to its canonical one-based portable value.
///
/// Floating-point selectors preserve the established source behavior: they
/// must be finite and positive and are truncated toward zero.
pub fn canonical_positional_ordinal(value: &ValueData) -> Result<u64, CanonicalSelectorError> {
    let value = match value {
        ValueData::Index(value) => u128::from(*value),
        ValueData::U8(value) => u128::from(*value),
        ValueData::U16(value) => u128::from(*value),
        ValueData::U32(value) => u128::from(*value),
        ValueData::U64(value) => u128::from(*value),
        ValueData::U128(value) => *value,
        ValueData::I8(value) => signed_ordinal(i128::from(*value))?,
        ValueData::I16(value) => signed_ordinal(i128::from(*value))?,
        ValueData::I32(value) => signed_ordinal(i128::from(*value))?,
        ValueData::I64(value) => signed_ordinal(i128::from(*value))?,
        ValueData::I128(value) => signed_ordinal(*value)?,
        ValueData::F32(value) => return float_ordinal(f64::from(value.to_f32())),
        ValueData::F64(value) => return float_ordinal(value.to_f64()),
        _ => return Err(CanonicalSelectorError::UnsupportedSchema),
    };
    checked_ordinal(value)
}

fn signed_ordinal(value: i128) -> Result<u128, CanonicalSelectorError> {
    if value <= 0 {
        return Err(CanonicalSelectorError::NonPositive);
    }
    Ok(value as u128)
}

fn checked_ordinal(value: u128) -> Result<u64, CanonicalSelectorError> {
    if value == 0 {
        return Err(CanonicalSelectorError::NonPositive);
    }
    if value > u128::from(PORTABLE_SELECTOR_INDEX_MAX) {
        return Err(CanonicalSelectorError::OutOfRange);
    }
    Ok(value as u64)
}

fn float_ordinal(value: f64) -> Result<u64, CanonicalSelectorError> {
    if !value.is_finite() {
        return Err(CanonicalSelectorError::NonFinite);
    }
    if value <= 0.0 {
        return Err(CanonicalSelectorError::NonPositive);
    }
    if value > PORTABLE_SELECTOR_INDEX_MAX as f64 {
        return Err(CanonicalSelectorError::OutOfRange);
    }
    checked_ordinal(value.trunc() as u128)
}

/// Converts a one-based scalar selector to a checked zero-based position.
pub fn canonical_positional_index(
    value: &ValueData,
    upper: usize,
) -> Result<usize, CanonicalSelectorError> {
    let ordinal = canonical_positional_ordinal(value)?;
    positional_ordinal_to_index(ordinal, upper)
}

fn positional_ordinal_to_index(
    ordinal: u64,
    upper: usize,
) -> Result<usize, CanonicalSelectorError> {
    let position = usize::try_from(ordinal - 1).map_err(|_| CanonicalSelectorError::OutOfRange)?;
    if position >= upper {
        return Err(CanonicalSelectorError::OutOfRange);
    }
    Ok(position)
}

/// Converts one element of a canonical numeric sequence without allocating an
/// intermediate selector vector.
pub fn canonical_positional_ordinal_at(
    sequence: SequenceView<'_>,
    ordinal: usize,
) -> Result<u64, CanonicalSelectorError> {
    macro_rules! at {
        ($values:expr, $variant:ident) => {
            $values
                .get(ordinal)
                .cloned()
                .map(ValueData::$variant)
                .ok_or(CanonicalSelectorError::OutOfRange)
                .and_then(|value| canonical_positional_ordinal(&value))
        };
    }
    match sequence {
        SequenceView::U8(values) => at!(values, U8),
        SequenceView::U16(values) => at!(values, U16),
        SequenceView::U32(values) => at!(values, U32),
        SequenceView::U64(values) => at!(values, U64),
        SequenceView::U128(values) => at!(values, U128),
        SequenceView::I8(values) => at!(values, I8),
        SequenceView::I16(values) => at!(values, I16),
        SequenceView::I32(values) => at!(values, I32),
        SequenceView::I64(values) => at!(values, I64),
        SequenceView::I128(values) => at!(values, I128),
        SequenceView::F32(values) => at!(values, F32),
        SequenceView::F64(values) => at!(values, F64),
        SequenceView::Index(values) => at!(values, Index),
        SequenceView::Values(values) => values
            .get(ordinal)
            .ok_or(CanonicalSelectorError::OutOfRange)
            .and_then(canonical_positional_ordinal),
        _ => Err(CanonicalSelectorError::UnsupportedSchema),
    }
}

/// Visits a canonical numeric sequence using the same scalar conversion and
/// portable bounds as every source and target path.
pub fn visit_canonical_positional_sequence<E>(
    sequence: SequenceView<'_>,
    upper: usize,
    mut visit: impl FnMut(usize) -> Result<(), E>,
) -> Result<(), CanonicalSelectorVisitError<E>> {
    for ordinal in 0..sequence.len() {
        let one_based = canonical_positional_ordinal_at(sequence, ordinal)
            .map_err(CanonicalSelectorVisitError::Selector)?;
        let index = positional_ordinal_to_index(one_based, upper)
            .map_err(CanonicalSelectorVisitError::Selector)?;
        visit(index).map_err(CanonicalSelectorVisitError::Visitor)?;
    }
    Ok(())
}

/// Visits a scalar or matrix of positional selectors without materializing a
/// converted index vector.
pub fn visit_canonical_positional_indices<E>(
    schema: &SchemaBody,
    data: &ValueData,
    upper: usize,
    mut visit: impl FnMut(usize) -> Result<(), E>,
) -> Result<(), CanonicalSelectorVisitError<E>> {
    match (schema, data) {
        (schema, data) if is_positional_selector_schema(schema) => {
            if !scalar_matches_schema(schema, data) {
                return Err(CanonicalSelectorVisitError::Selector(
                    CanonicalSelectorError::UnsupportedSchema,
                ));
            }
            let index = canonical_positional_index(data, upper)
                .map_err(CanonicalSelectorVisitError::Selector)?;
            visit(index).map_err(CanonicalSelectorVisitError::Visitor)
        }
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix))
            if is_positional_selector_schema(element) =>
        {
            visit_sequence(element, matrix.elements(), upper, &mut visit)
        }
        _ => Err(CanonicalSelectorVisitError::Selector(
            CanonicalSelectorError::UnsupportedSchema,
        )),
    }
}

fn scalar_matches_schema(schema: &SchemaBody, data: &ValueData) -> bool {
    matches!(
        (schema, data),
        (SchemaBody::Index, ValueData::Index(_))
            | (
                SchemaBody::UnsignedInteger(crate::IntegerWidth::W8),
                ValueData::U8(_)
            )
            | (
                SchemaBody::UnsignedInteger(crate::IntegerWidth::W16),
                ValueData::U16(_)
            )
            | (
                SchemaBody::UnsignedInteger(crate::IntegerWidth::W32),
                ValueData::U32(_)
            )
            | (
                SchemaBody::UnsignedInteger(crate::IntegerWidth::W64),
                ValueData::U64(_)
            )
            | (
                SchemaBody::UnsignedInteger(crate::IntegerWidth::W128),
                ValueData::U128(_)
            )
            | (
                SchemaBody::SignedInteger(crate::IntegerWidth::W8),
                ValueData::I8(_)
            )
            | (
                SchemaBody::SignedInteger(crate::IntegerWidth::W16),
                ValueData::I16(_)
            )
            | (
                SchemaBody::SignedInteger(crate::IntegerWidth::W32),
                ValueData::I32(_)
            )
            | (
                SchemaBody::SignedInteger(crate::IntegerWidth::W64),
                ValueData::I64(_)
            )
            | (
                SchemaBody::SignedInteger(crate::IntegerWidth::W128),
                ValueData::I128(_)
            )
            | (
                SchemaBody::FloatingPoint(crate::FloatWidth::W32),
                ValueData::F32(_)
            )
            | (
                SchemaBody::FloatingPoint(crate::FloatWidth::W64),
                ValueData::F64(_)
            )
    )
}

fn visit_sequence<E>(
    schema: &SchemaBody,
    sequence: SequenceView<'_>,
    upper: usize,
    visit: &mut impl FnMut(usize) -> Result<(), E>,
) -> Result<(), CanonicalSelectorVisitError<E>> {
    macro_rules! visit_values {
        ($values:expr, $variant:ident) => {{
            for value in $values {
                let value = ValueData::$variant(value.clone());
                if !scalar_matches_schema(schema, &value) {
                    return Err(CanonicalSelectorVisitError::Selector(
                        CanonicalSelectorError::UnsupportedSchema,
                    ));
                }
                let index = canonical_positional_index(&value, upper)
                    .map_err(CanonicalSelectorVisitError::Selector)?;
                visit(index).map_err(CanonicalSelectorVisitError::Visitor)?;
            }
            Ok(())
        }};
    }
    match sequence {
        SequenceView::U8(values) => visit_values!(values, U8),
        SequenceView::U16(values) => visit_values!(values, U16),
        SequenceView::U32(values) => visit_values!(values, U32),
        SequenceView::U64(values) => visit_values!(values, U64),
        SequenceView::U128(values) => visit_values!(values, U128),
        SequenceView::I8(values) => visit_values!(values, I8),
        SequenceView::I16(values) => visit_values!(values, I16),
        SequenceView::I32(values) => visit_values!(values, I32),
        SequenceView::I64(values) => visit_values!(values, I64),
        SequenceView::I128(values) => visit_values!(values, I128),
        SequenceView::F32(values) => visit_values!(values, F32),
        SequenceView::F64(values) => visit_values!(values, F64),
        SequenceView::Index(values) => visit_values!(values, Index),
        SequenceView::Values(values) => {
            for value in values {
                if !scalar_matches_schema(schema, value) {
                    return Err(CanonicalSelectorVisitError::Selector(
                        CanonicalSelectorError::UnsupportedSchema,
                    ));
                }
                let index = canonical_positional_index(value, upper)
                    .map_err(CanonicalSelectorVisitError::Selector)?;
                visit(index).map_err(CanonicalSelectorVisitError::Visitor)?;
            }
            Ok(())
        }
        _ => Err(CanonicalSelectorVisitError::Selector(
            CanonicalSelectorError::UnsupportedSchema,
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::snapshot::{F32Bits, F64Bits};

    #[test]
    fn canonical_selector_conversion_has_one_portable_boundary() {
        for value in [
            ValueData::Index(1),
            ValueData::U8(1),
            ValueData::U16(1),
            ValueData::U32(1),
            ValueData::U64(1),
            ValueData::U128(1),
            ValueData::I8(1),
            ValueData::I16(1),
            ValueData::I32(1),
            ValueData::I64(1),
            ValueData::I128(1),
            ValueData::F32(F32Bits::from_f32(1.9)),
            ValueData::F64(F64Bits::from_f64(1.9)),
        ] {
            assert_eq!(canonical_positional_ordinal(&value), Ok(1), "{value:?}");
        }
        assert_eq!(
            canonical_positional_index(&ValueData::F64(F64Bits::from_f64(1.9)), 3),
            Ok(0),
        );
        assert_eq!(
            canonical_positional_index(&ValueData::F32(F32Bits::from_f32(2.5)), 3),
            Ok(1),
        );
        assert_eq!(
            canonical_positional_ordinal(&ValueData::U32(u32::MAX)),
            Ok(PORTABLE_SELECTOR_INDEX_MAX),
        );
        for value in [0.0, 0.5, -1.0] {
            assert_eq!(
                canonical_positional_ordinal(&ValueData::F64(F64Bits::from_f64(value))),
                Err(CanonicalSelectorError::NonPositive),
            );
        }
        for value in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(
                canonical_positional_ordinal(&ValueData::F64(F64Bits::from_f64(value))),
                Err(CanonicalSelectorError::NonFinite),
            );
        }
        assert_eq!(
            canonical_positional_ordinal(&ValueData::U64(PORTABLE_SELECTOR_INDEX_MAX + 1)),
            Err(CanonicalSelectorError::OutOfRange),
        );
        assert_eq!(
            canonical_positional_ordinal(&ValueData::I64(-1)),
            Err(CanonicalSelectorError::NonPositive),
        );
        assert_eq!(
            canonical_positional_ordinal(&ValueData::Bool(true)),
            Err(CanonicalSelectorError::UnsupportedSchema),
        );
    }
}
