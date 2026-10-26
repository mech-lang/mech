//! Matrix conversion uses the same finalization and canonical key relation as
//! ordinary sets. Callers admit the complete staging/sort work before entry.
use super::error::{SnapshotPath, SnapshotPathSegment};
use super::relations::{compare_key_data, normalized_key_data};
use super::validation::{canonical_snapshot_data_draft, finalize_data};
use super::{SnapshotValidationContext, SnapshotValueError, ValueData, ValueDataDraft};
use crate::{Schema, SchemaBody, SchemaTable, ShapeInstance};
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

pub(crate) fn normalize_set_conversion_drafts(
    schema: &Schema,
    drafts: Box<[ValueDataDraft]>,
    schemas: &SchemaTable,
) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
    normalize_set_conversion_drafts_impl(
        schema.body(),
        drafts,
        ConversionShape::Declared(schema),
        &SnapshotValidationContext::new(schemas),
    )
}

pub(crate) fn normalize_set_conversion_drafts_with_context(
    body: &SchemaBody,
    drafts: Box<[ValueDataDraft]>,
    shape: &ShapeInstance,
    context: &SnapshotValidationContext<'_>,
) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
    normalize_set_conversion_drafts_impl(body, drafts, ConversionShape::Current(shape), context)
}

enum ConversionShape<'a> {
    Declared(&'a Schema),
    Current(&'a ShapeInstance),
}

fn normalize_set_conversion_drafts_impl(
    body: &SchemaBody,
    drafts: Box<[ValueDataDraft]>,
    authority: ConversionShape<'_>,
    context: &SnapshotValidationContext<'_>,
) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
    if !crate::schema::is_schema_body_keyable(body) {
        return Err(SnapshotValueError::SchemaNotKeyableV1);
    }
    let mut values = Vec::with_capacity(drafts.len());
    for (index, draft) in drafts.into_vec().into_iter().enumerate() {
        let declared_shape;
        let shape = match authority {
            ConversionShape::Current(shape) => shape,
            ConversionShape::Declared(schema) => {
                // Constant matrix axes are already declared, not inferred from
                // flat data length. Parameterized axes require the explicit
                // current-shape API; no lower-bound geometry is guessed here.
                declared_shape = if matches!(body, SchemaBody::Matrix { .. }) {
                    schema
                        .instantiate_shape(Box::new([]))
                        .map_err(|_| SnapshotValueError::SchemaNotKeyableV1)?
                } else {
                    crate::shape_for_value_data(schema, &draft, &[], None)
                        .map_err(|_| SnapshotValueError::SchemaNotKeyableV1)?
                };
                &declared_shape
            }
        };
        let path = SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64));
        let value = finalize_data(body, draft, shape, context, &path)?;
        values.push(normalized_key_data(body, value)?);
    }
    // Bounded, allocation-free heapsort; no comparator-owned payload copies
    // and no host-stack recursion. All comparisons use canonical key identity.
    fn sift(
        body: &crate::SchemaBody,
        values: &mut [ValueData],
        mut root: usize,
        end: usize,
    ) -> Result<(), SnapshotValueError> {
        while let Some(left) = root
            .checked_mul(2)
            .and_then(|n| n.checked_add(1))
            .filter(|n| *n < end)
        {
            let mut next = left;
            if left + 1 < end && compare_key_data(body, &values[left], &values[left + 1])?.is_lt() {
                next = left + 1;
            }
            if !compare_key_data(body, &values[root], &values[next])?.is_lt() {
                break;
            }
            values.swap(root, next);
            root = next;
        }
        Ok(())
    }
    for start in (0..values.len() / 2).rev() {
        let end = values.len();
        sift(body, &mut values, start, end)?;
    }
    for end in (1..values.len()).rev() {
        values.swap(0, end);
        sift(body, &mut values, 0, end)?;
    }
    let mut retained = 0;
    for next in 0..values.len() {
        if retained == 0 || !compare_key_data(body, &values[retained - 1], &values[next])?.is_eq() {
            values.swap(retained, next);
            retained += 1;
        }
    }
    values.truncate(retained);
    values
        .iter()
        .map(|value| canonical_snapshot_data_draft(body, value))
        .collect()
}
