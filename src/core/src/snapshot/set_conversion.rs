//! Matrix conversion uses the same finalization and canonical key relation as
//! ordinary sets. Callers admit the complete staging/sort work before entry.
use super::error::{SnapshotPath, SnapshotPathSegment};
use super::relations::{compare_key_data, normalized_key_data};
use super::validation::{canonical_snapshot_data_draft, finalize_data};
use super::{SnapshotValidationContext, SnapshotValueError, ValueData, ValueDataDraft};
use crate::{Schema, SchemaTable};
#[cfg(feature = "no_std")]
use alloc::{boxed::Box, vec::Vec};
#[cfg(not(feature = "no_std"))]
use std::{boxed::Box, vec::Vec};

pub(crate) fn normalize_set_conversion_drafts(
    schema: &Schema,
    drafts: Box<[ValueDataDraft]>,
    schemas: &SchemaTable,
) -> Result<Box<[ValueDataDraft]>, SnapshotValueError> {
    if !schema.is_keyable() {
        return Err(SnapshotValueError::SchemaNotKeyableV1);
    }
    let context = SnapshotValidationContext::new(schemas);
    let body = schema.body();
    let mut values = Vec::with_capacity(drafts.len());
    for (index, draft) in drafts.into_vec().into_iter().enumerate() {
        let shape = crate::shape_for_value_data(schema, &draft, &[], None)
            .map_err(|_| SnapshotValueError::SchemaNotKeyableV1)?;
        let path = SnapshotPath::root().child(SnapshotPathSegment::SetElement(index as u64));
        let value = finalize_data(body, draft, &shape, &context, &path)?;
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
