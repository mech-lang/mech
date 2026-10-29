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
        let Some((logical, work, nodes)) = self.measure(call, inputs, words, memory) else {
            return Ok(None);
        };
        if !meter.try_charge_borrowed_footprint(work, nodes)? {
            return Ok(None);
        }
        Ok(Some(logical))
    }

    fn measure(
        &self,
        call: &NativePreparedLibraryCall,
        inputs: &[Option<Value>],
        words: Option<&[u64]>,
        memory: &ManagedMemoryBudget,
    ) -> Option<(u64, u64, u64)> {
        if inputs.len() != self.fields.len() {
            return None;
        }
        let (mut logical, mut work, mut nodes) = (0u64, 0u64, 0u64);
        for (index, (field, input)) in self.fields.iter().zip(inputs).enumerate() {
            let (footprint, field_work) = if let Some(value) = input {
                if words.is_some_and(|words| words[index] != 0)
                    || value.schema() != field.schema
                    || value.schema_key() != call.input_keys[index]
                    || value.memory_budget_retained_bytes(memory).is_none()
                    || value.validate_against(&call.schemas).is_err()
                {
                    return None;
                }
                field.footprint.value(value, &call.schemas)?
            } else {
                field.footprint.word(words?[index])?
            };
            logical = logical.checked_add(footprint.retained_bytes)?;
            work = work.checked_add(field_work)?;
            nodes = nodes.checked_add(footprint.node_count)?;
        }
        Some((logical, work, nodes))
    }
}
