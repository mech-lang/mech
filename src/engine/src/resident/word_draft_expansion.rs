//! Additional draft/finalization workspace for unsigned packed buffers.
use crate::resident::budget::{
    self, ResidentBudgetMeter, checked_cost_product as mul, checked_cost_sum as add,
};
use mech_core::{ResidentKernelError, SchemaBody, ValueData};

/// Packed canonical buffers have constant retained-node counts, but draft
/// reconstruction allocates one draft and one temporary canonical cell per
/// element. Measure that extra space before creating either allocation.
pub(crate) fn unsigned_word_draft_expansion(
    meter: &mut ResidentBudgetMeter,
    body: &SchemaBody,
    data: &ValueData,
) -> Result<u64, ResidentKernelError> {
    meter.charge_compute_work(32)?;
    match (body, data) {
        (SchemaBody::Matrix { element, .. }, ValueData::Matrix(matrix))
            if matches!(element.as_ref(), SchemaBody::UnsignedInteger(_)) =>
        {
            let count = budget::checked_u64(matrix.elements().len())?;
            mul(&[
                count,
                (core::mem::size_of::<mech_core::ValueDataDraft>()
                    + core::mem::size_of::<ValueData>()) as u64,
            ])
        }
        (SchemaBody::Tuple(fields), ValueData::Tuple(values)) if fields.len() == values.len() => {
            fields
                .iter()
                .zip(values.iter())
                .try_fold(0, |total, (body, data)| {
                    add(&[total, unsigned_word_draft_expansion(meter, body, data)?])
                })
        }
        (SchemaBody::Enum { variants, .. }, ValueData::Enum(value)) => {
            let variant = variants
                .get(value.ordinal() as usize)
                .ok_or(ResidentKernelError::InvalidInput)?;
            match (&variant.payload, value.payload()) {
                (Some(body), Some(data)) => unsigned_word_draft_expansion(meter, body, data),
                (None, None) => Ok(0),
                _ => Err(ResidentKernelError::InvalidInput),
            }
        }
        (SchemaBody::Option(body), ValueData::Option(value)) => {
            value.as_deref().map_or(Ok(0), |data| {
                unsigned_word_draft_expansion(meter, body, data)
            })
        }
        (SchemaBody::Set { element, .. }, ValueData::Set(value)) => {
            value.elements().iter().try_fold(0, |total, item| {
                add(&[
                    total,
                    unsigned_word_draft_expansion(meter, element, item.data())?,
                ])
            })
        }
        (
            SchemaBody::Map {
                key, value: body, ..
            },
            ValueData::Map(value),
        ) => value.entries().iter().try_fold(0, |total, entry| {
            add(&[
                total,
                unsigned_word_draft_expansion(meter, key, entry.key().data())?,
                unsigned_word_draft_expansion(meter, body, entry.value())?,
            ])
        }),
        (SchemaBody::Record(fields), ValueData::Record(value))
            if fields.len() == value.fields().len() =>
        {
            fields
                .iter()
                .zip(value.fields())
                .try_fold(0, |total, (field, data)| {
                    add(&[
                        total,
                        unsigned_word_draft_expansion(meter, &field.schema, data)?,
                    ])
                })
        }
        // Other existing payload families keep their current witnesses. All
        // unsigned matrix carriers newly admitted natively occur in the
        // containers handled above, and canonical values were validated first.
        _ => Ok(0),
    }
}
