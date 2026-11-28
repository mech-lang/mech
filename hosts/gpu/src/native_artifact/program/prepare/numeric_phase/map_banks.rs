//! Exact output banks for complete maps without a scalar first-change owner.
//! This source metadata grants no view, owner, memory permit or publication.
use super::super::{
    numeric_plan::Operation,
    readonly_numeric::{Array, ArrayStorage},
};

#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(in crate::native_artifact) struct Bank {
    pub array: usize,
    pub elements: u64,
    pub extents: [u64; 2],
}
impl Bank {
    /// Full fresh-owner/shape/schema/publication planning envelope, admitted
    /// before allocating cells. This is not logical source output cardinality.
    pub fn admission(&self, schema_bytes: u64) -> Option<(u64, u64)> {
        let bytes = self
            .elements
            .checked_add(8)?
            .checked_mul(
                (std::mem::size_of::<mech_core::ValueDataDraft>() as u64)
                    .checked_mul(16)?
                    .checked_add(512)?,
            )?
            .checked_add(schema_bytes.checked_mul(32)?)?
            .checked_add(65536)?;
        let nodes = self
            .elements
            .checked_mul(8)?
            .checked_add(schema_bytes)?
            .checked_add(256)?;
        Some((bytes, nodes))
    }
}
pub(in crate::native_artifact) fn derive(
    actions: &[(usize, Operation)],
    arrays: &[Array],
) -> Result<Vec<Bank>, &'static str> {
    let mut banks: Vec<Bank> = Vec::new();
    for (_, action) in actions {
        let Operation::Map { replacement } = action else {
            continue;
        };
        let array = replacement.target;
        let has_scalar_producer = actions.iter().any(|(_, action)| matches!(action,
            Operation::Write { array: at, .. } | Operation::CellWrite { array: at, .. } if *at == array));
        if has_scalar_producer {
            if matches!(arrays[array].storage, ArrayStorage::PhaseLocal { .. }) {
                return Err("local map has an unsupported scalar ownership producer");
            }
            continue; // Preserve established selected first-change factories.
        }
        if arrays[array].appends || !arrays[array].writable {
            return Err("owned map output has a growing or read-only destination");
        }
        let source = &replacement.source;
        let bank = Bank {
            array,
            elements: source.elements,
            extents: source.extents,
        };
        if let Some(previous) = banks.iter().find(|b| b.array == array) {
            if previous != &bank {
                return Err("owned maps disagree on exact output geometry");
            }
        } else {
            banks.push(bank);
        }
    }
    for (array, data) in arrays.iter().enumerate() {
        if matches!(data.storage, ArrayStorage::PhaseLocal { .. })
            && !banks.iter().any(|b| b.array == array)
        {
            return Err("local map output has no complete ownership producer");
        }
    }
    banks.sort_by_key(|bank| bank.array);
    Ok(banks)
}
