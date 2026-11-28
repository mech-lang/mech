//! Confined temporary map owners during one native phase reconciliation.
use super::*;
struct Entry {
    value: u32,
    schema: mech_core::SchemaId,
    key: mech_core::SchemaKey,
    owner: Option<Value>,
}
pub(super) struct Locals {
    entries: Vec<Entry>,
    _memory: mech_core::ManagedMemoryReservation,
    _nodes: mech_core::ManagedNodeReservation,
}
impl Locals {
    pub(in crate::native_artifact::program::runtime) fn metadata_nodes(&self) -> u64 {
        self._nodes.nodes()
    }
}
impl BufferFrames {
    pub(in crate::native_artifact::program::runtime) fn prepare_map_locals(
        &mut self,
        region: usize,
        bindings: &[(u32, mech_core::SchemaId, mech_core::SchemaKey)],
        memory: &ManagedMemoryBudget,
    ) -> Result<bool, NativeProgramExecutionError> {
        if bindings.is_empty() {
            return Ok(true);
        }
        let private = self
            .inner
            .as_mut()
            .and_then(|i| i.private.as_mut())
            .filter(|p| p.region == region)
            .ok_or_else(|| invalid("local map metadata has no private phase"))?;
        if private.locals.is_some() {
            return Err(invalid("duplicate local map metadata"));
        }
        let bytes = amount(
            bindings.len(),
            core::mem::size_of::<Entry>() as u64,
            core::mem::size_of::<Locals>() as u64,
        )?;
        let Ok(reservation) = memory.reserve_capacity(bytes) else {
            return Ok(false);
        };
        let Ok(nodes) = memory.reserve_nodes(bindings.len() as u64 + 1) else {
            return Ok(false);
        };
        let mut entries = Vec::new();
        if entries.try_reserve_exact(bindings.len()).is_err() {
            return Ok(false);
        }
        for &(value, schema, key) in bindings {
            if entries.iter().any(|e: &Entry| e.value == value) {
                return Err(invalid("overlapping local map identity"));
            }
            entries.push(Entry {
                value,
                schema,
                key,
                owner: None,
            });
        }
        private.locals = Some(Box::new(Locals {
            entries,
            _memory: reservation,
            _nodes: nodes,
        }));
        Ok(true)
    }
    pub(in crate::native_artifact::program::runtime) fn private_map_bank(
        &self,
        region: usize,
        array: usize,
    ) -> Result<&mech_core::snapshot::PrivateNumericMatrix, NativeProgramExecutionError> {
        match self
            .inner
            .as_ref()
            .and_then(|i| i.private.as_ref())
            .filter(|p| p.region == region)
            .and_then(|p| p.arrays.get(array))
            .and_then(Option::as_ref)
        {
            Some(PrivateNumericArray::Scalar(bank)) => Ok(bank),
            _ => Err(invalid("owned map lost exact private numeric bank")),
        }
    }
    pub(in crate::native_artifact::program::runtime) fn set_map_local(
        &mut self,
        region: usize,
        value: u32,
        owner: Value,
    ) -> Result<(), NativeProgramExecutionError> {
        let locals = self
            .inner
            .as_mut()
            .and_then(|i| i.private.as_mut())
            .filter(|p| p.region == region)
            .and_then(|p| p.locals.as_mut())
            .ok_or_else(|| invalid("local map publication lacks metadata"))?;
        let entry = locals
            .entries
            .iter_mut()
            .find(|e| e.value == value)
            .ok_or_else(|| invalid("local map publication has foreign source value"))?;
        if entry.owner.is_some()
            || owner.schema() != entry.schema
            || owner.schema_key() != entry.key
            || owner
                .memory_budget_retained_bytes(&locals._memory.budget())
                .is_none()
        {
            return Err(invalid(
                "local map publication has a foreign type, account or lifetime",
            ));
        }
        entry.owner = Some(owner);
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn map_local_owner(
        &self,
        value: u32,
        schema: mech_core::SchemaId,
    ) -> Option<&Value> {
        self.inner
            .as_ref()?
            .private
            .as_ref()?
            .locals
            .as_ref()?
            .entries
            .iter()
            .find(|e| e.value == value && e.schema == schema)?
            .owner
            .as_ref()
    }
    pub(in crate::native_artifact::program::runtime) fn set_private_publication(
        &mut self,
        region: usize,
        ticket: PutTicket,
    ) -> Result<(), NativeProgramExecutionError> {
        let private = self
            .inner
            .as_mut()
            .and_then(|i| i.private.as_mut())
            .filter(|p| p.region == region)
            .ok_or_else(|| invalid("map publication has no private phase"))?;
        match &mut private.publication {
            Some(PrivatePublication::Cold(cold)) if cold.direct.is_none() => {
                cold.direct = Some(ticket)
            }
            None => private.publication = Some(PrivatePublication::Direct(ticket)),
            _ => return Err(invalid("duplicate map publication ticket")),
        }
        Ok(())
    }
    /// Entry-only rollback: no source action or existing owner has changed.
    pub(in crate::native_artifact::program::runtime) fn discard_prepared_map_banks(
        &mut self,
        region: usize,
    ) -> Result<(), NativeProgramExecutionError> {
        let state = self
            .inner
            .as_mut()
            .and_then(|i| i.private.take())
            .ok_or_else(|| invalid("map rollback has no private preparation"))?;
        if state.region != region {
            return Err(invalid("map rollback has a foreign phase"));
        }
        drop(state);
        Ok(())
    }
    pub(in crate::native_artifact::program::runtime) fn clear_map_locals(
        &mut self,
        region: usize,
    ) -> Result<(), NativeProgramExecutionError> {
        let state = self
            .inner
            .as_mut()
            .and_then(|i| i.private.take())
            .ok_or_else(|| invalid("local map reconciliation already ended"))?;
        if state.region != region
            || state.locals.is_none()
            || state.publication.is_some()
            || state.arrays.iter().any(Option::is_some)
        {
            return Err(invalid("local map reconciliation is still active"));
        }
        drop(state);
        Ok(())
    }
}
