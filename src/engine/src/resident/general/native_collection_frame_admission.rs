//! A successful-path recipe for ONE private flat frame. No retained arguments,
//! extents, debit, permit, owner pin or previous-visit result is cached here.
use super::NativePreparedLibraryCall;
use crate::resident::budget::{PreparedNativeFrameFootprint, ResidentBudgetMeter};
use mech_core::{ManagedMemoryBudget, ResidentKernelError, SchemaId, SchemaTable, Value};

#[derive(Debug)]
pub(super) struct FrameAdmissionRecipe {
    fields: Box<[Field]>,
}

#[derive(Debug)]
struct Field {
    schema: SchemaId,
    key: mech_core::SchemaKey,
    footprint: PreparedNativeFrameFootprint,
}

impl FrameAdmissionRecipe {
    pub(super) fn derive(inputs: &[SchemaId], schemas: &SchemaTable) -> Option<Self> {
        Some(Self {
            fields: inputs
                .iter()
                .map(|id| {
                    Some(Field {
                        schema: *id,
                        key: schemas.get(*id)?.key(),
                        footprint: PreparedNativeFrameFootprint::derive(schemas.get(*id)?)?,
                    })
                })
                .collect::<Option<Box<[_]>>>()?,
        })
    }

    pub(super) fn try_admit(
        &self,
        call: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        words: Option<&[u64]>,
        memory: &ManagedMemoryBudget,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<Option<u64>, ResidentKernelError> {
        // The enclosing selected call has already paid its metadata preflight
        // and reserved its original constructor envelope. This preview reads
        // a bounded number of headers per prepared field. Current shape/output
        // validation remains after these exact footprint debits.
        if inputs.len() != self.fields.len() {
            return Ok(None);
        }
        self.try_admit_current(
            &call.schemas,
            |index| {
                Some((
                    inputs[index].as_ref(),
                    if inputs[index].is_some() {
                        words.map_or(0, |w| w[index])
                    } else {
                        words?[index]
                    },
                ))
            },
            memory,
            meter,
        )
    }

    pub(super) fn try_admit_current<'a>(
        &self,
        schemas: &SchemaTable,
        current: impl Fn(usize) -> Option<(Option<&'a Value>, u64)>,
        memory: &ManagedMemoryBudget,
        meter: &mut ResidentBudgetMeter,
    ) -> Result<Option<u64>, ResidentKernelError> {
        let Some((logical, work, nodes)) = self.measure(schemas, current, memory) else {
            return Ok(None);
        };
        if !meter.try_charge_borrowed_footprint(work, nodes)? {
            return Ok(None);
        }
        Ok(Some(logical))
    }

    fn measure<'a>(
        &self,
        schemas: &SchemaTable,
        current: impl Fn(usize) -> Option<(Option<&'a Value>, u64)>,
        memory: &ManagedMemoryBudget,
    ) -> Option<(u64, u64, u64)> {
        let (mut logical, mut work, mut nodes) = (0u64, 0u64, 0u64);
        for (index, field) in self.fields.iter().enumerate() {
            let (input, word) = current(index)?;
            let (footprint, field_work) = if let Some(value) = input {
                if word != 0
                    || value.schema() != field.schema
                    || value.schema_key() != field.key
                    || value.memory_budget_retained_bytes(memory).is_none()
                    || value.validate_against(schemas).is_err()
                {
                    return None;
                }
                field.footprint.value(value, schemas)?
            } else {
                field.footprint.word(word)?
            };
            logical = logical.checked_add(footprint.retained_bytes)?;
            work = work.checked_add(field_work)?;
            nodes = nodes.checked_add(footprint.node_count)?;
        }
        Some((logical, work, nodes))
    }
}
