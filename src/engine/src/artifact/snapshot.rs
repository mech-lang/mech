//! Lossless projection of finalized snapshot data back into validated drafts.
//!
//! Bytecode-v1 and compiler literal folding share this one conversion so every
//! C2 snapshot family has the same total semantic representation.

use mech_core::snapshot::{
    EnumDraft, MapEntryDraft, NamedValueDraft, OptionDraft, ReifiedType, ReifiedTypeDraft,
    SequenceView, TableColumnDraft,
};
use mech_core::{SchemaBody, ValueData, ValueDataDraft};

pub(super) fn data_draft(data: &ValueData, schema: &SchemaBody) -> Option<ValueDataDraft> {
    Some(match (data, schema) {
        (ValueData::U8(value), _) => ValueDataDraft::U8(*value),
        (ValueData::U16(value), _) => ValueDataDraft::U16(*value),
        (ValueData::U32(value), _) => ValueDataDraft::U32(*value),
        (ValueData::U64(value), _) => ValueDataDraft::U64(*value),
        (ValueData::U128(value), _) => ValueDataDraft::U128(*value),
        (ValueData::I8(value), _) => ValueDataDraft::I8(*value),
        (ValueData::I16(value), _) => ValueDataDraft::I16(*value),
        (ValueData::I32(value), _) => ValueDataDraft::I32(*value),
        (ValueData::I64(value), _) => ValueDataDraft::I64(*value),
        (ValueData::I128(value), _) => ValueDataDraft::I128(*value),
        (ValueData::F32(value), _) => ValueDataDraft::F32(*value),
        (ValueData::F64(value), _) => ValueDataDraft::F64(*value),
        (ValueData::Complex32(value), _) => ValueDataDraft::Complex32(*value),
        (ValueData::Complex64(value), _) => ValueDataDraft::Complex64(*value),
        (ValueData::Rational64(value), _) => ValueDataDraft::Rational64 {
            numerator: value.numerator(),
            denominator: value.denominator(),
        },
        (ValueData::Bool(value), _) => ValueDataDraft::Bool(*value),
        (ValueData::String(value), _) => ValueDataDraft::String(value.to_string()),
        (ValueData::Id(value), _) => ValueDataDraft::Id(*value),
        (ValueData::Index(value), _) => ValueDataDraft::Index(*value),
        (ValueData::Atom, _) => ValueDataDraft::Atom,
        (ValueData::Enum(value), SchemaBody::Enum { variants, .. }) => {
            let variant = variants.get(value.ordinal() as usize)?;
            ValueDataDraft::Enum(EnumDraft {
                ordinal: value.ordinal(),
                payload: match (value.payload(), variant.payload.as_ref()) {
                    (Some(data), Some(schema)) => Some(Box::new(data_draft(data, schema)?)),
                    (None, None) => None,
                    _ => return None,
                },
            })
        }
        (ValueData::Option(value), SchemaBody::Option(element)) => {
            ValueDataDraft::Option(OptionDraft {
                present: value.is_some(),
                value: match value.as_deref() {
                    Some(data) => Some(Box::new(data_draft(data, element)?)),
                    None => None,
                },
            })
        }
        (ValueData::Tuple(values), SchemaBody::Tuple(elements)) => ValueDataDraft::Tuple(
            values
                .iter()
                .zip(elements)
                .map(|(data, schema)| data_draft(data, schema))
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        (ValueData::Record(value), SchemaBody::Record(fields)) => ValueDataDraft::Record(
            value
                .fields()
                .iter()
                .zip(fields)
                .map(|(data, field)| {
                    Some(NamedValueDraft {
                        name: field.name.clone(),
                        value: data_draft(data, &field.schema)?,
                    })
                })
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        (ValueData::Matrix(value), SchemaBody::Matrix { element, .. }) => {
            ValueDataDraft::Matrix(sequence_drafts(value.elements(), element)?.into_boxed_slice())
        }
        (ValueData::Table(value), SchemaBody::Table { columns, .. }) => ValueDataDraft::Table(
            columns
                .iter()
                .enumerate()
                .map(|(index, column)| {
                    Some(TableColumnDraft {
                        name: column.name.clone(),
                        values: sequence_drafts(value.column(index)?, &column.schema)?
                            .into_boxed_slice(),
                    })
                })
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        (ValueData::Set(value), SchemaBody::Set { element, .. }) => ValueDataDraft::Set(
            value
                .elements()
                .iter()
                .map(|value| data_draft(value.data(), element))
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        (
            ValueData::Map(value),
            SchemaBody::Map {
                key,
                value: value_schema,
                ..
            },
        ) => ValueDataDraft::Map(
            value
                .entries()
                .iter()
                .map(|entry| {
                    Some(MapEntryDraft {
                        items: vec![
                            data_draft(entry.key().data(), key)?,
                            data_draft(entry.value(), value_schema)?,
                        ]
                        .into_boxed_slice(),
                    })
                })
                .collect::<Option<Vec<_>>>()?
                .into_boxed_slice(),
        ),
        (ValueData::Type(ReifiedType::Schema(schema)), SchemaBody::ReifiedType) => {
            ValueDataDraft::Type(ReifiedTypeDraft::Schema(*schema))
        }
        (ValueData::Type(ReifiedType::Kind(kind)), SchemaBody::ReifiedType) => {
            ValueDataDraft::Type(ReifiedTypeDraft::CanonicalKind(
                kind.canonical_bytes().to_vec().into_boxed_slice(),
            ))
        }
        _ => return None,
    })
}

fn sequence_drafts(values: SequenceView<'_>, schema: &SchemaBody) -> Option<Vec<ValueDataDraft>> {
    macro_rules! primitive {
        ($values:expr, $variant:ident) => {
            $values
                .iter()
                .copied()
                .map(ValueDataDraft::$variant)
                .collect()
        };
    }
    Some(match values {
        SequenceView::U8(values) => primitive!(values, U8),
        SequenceView::U16(values) => primitive!(values, U16),
        SequenceView::U32(values) => primitive!(values, U32),
        SequenceView::U64(values) => primitive!(values, U64),
        SequenceView::U128(values) => primitive!(values, U128),
        SequenceView::I8(values) => primitive!(values, I8),
        SequenceView::I16(values) => primitive!(values, I16),
        SequenceView::I32(values) => primitive!(values, I32),
        SequenceView::I64(values) => primitive!(values, I64),
        SequenceView::I128(values) => primitive!(values, I128),
        SequenceView::F32(values) => primitive!(values, F32),
        SequenceView::F64(values) => primitive!(values, F64),
        SequenceView::Complex32(values) => primitive!(values, Complex32),
        SequenceView::Complex64(values) => primitive!(values, Complex64),
        SequenceView::Rational64(values) => values
            .iter()
            .map(|value| ValueDataDraft::Rational64 {
                numerator: value.numerator(),
                denominator: value.denominator(),
            })
            .collect(),
        SequenceView::Bool(values) => primitive!(values, Bool),
        SequenceView::String(values) => values
            .iter()
            .map(|value| ValueDataDraft::String(value.to_string()))
            .collect(),
        SequenceView::Id(values) => primitive!(values, Id),
        SequenceView::Index(values) => primitive!(values, Index),
        SequenceView::Unit(count) => (0..count).map(|_| ValueDataDraft::Atom).collect(),
        SequenceView::Values(values) => values
            .iter()
            .map(|value| data_draft(value, schema))
            .collect::<Option<Vec<_>>>()?,
    })
}
