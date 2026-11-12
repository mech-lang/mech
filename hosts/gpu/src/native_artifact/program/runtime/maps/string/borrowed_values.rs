//! Immutable canonical keys plus independent current U64 cells. This is only
//! storage: caller proofs and original source-call admission govern mutation.
use super::*;
use mech_core::{ManagedMemoryBudget, SchemaTable, Value, ValueData};

pub(in crate::native_artifact) struct BorrowedStringValues {
    // Drop actual backing and its snapshot pin before their private claims.
    values: Box<[u64]>,
    snapshot: Value,
    key_bytes: u64,
    work: NativeLibraryWorkBudget,
    metadata: ManagedMemoryReservation,
    buffer: ManagedMemoryReservation,
    nodes: ManagedNodeReservation,
}
impl BorrowedStringValues {
    #[cfg(test)]
    pub(in crate::native_artifact) fn new(
        source: &Value,
        schemas: &SchemaTable,
        work: NativeLibraryWorkBudget,
        memory: ManagedMemoryBudget,
        limits: Limits,
    ) -> Result<Self, Error> {
        Self::new_with_keys(source, source, schemas, work, memory, limits)
    }
    pub(in crate::native_artifact) fn new_with_keys(
        source: &Value,
        key_snapshot: &Value,
        schemas: &SchemaTable,
        work: NativeLibraryWorkBudget,
        memory: ManagedMemoryBudget,
        limits: Limits,
    ) -> Result<Self, Error> {
        if memory != work.memory_budget() {
            return Err(Error::Binding("foreign borrowed dictionary memory"));
        }
        let retained = source
            .memory_budget_retained_bytes(&memory)
            .ok_or(Error::Binding("unadmitted borrowed dictionary seed"))?;
        let key_retained = key_snapshot
            .memory_budget_retained_bytes(&memory)
            .ok_or(Error::Binding("unadmitted borrowed dictionary keys"))?;
        let same = source.shares_frozen_storage(key_snapshot);
        work.charge(
            add(
                mul(
                    if same {
                        retained
                    } else {
                        add(retained, key_retained)?
                    },
                    64,
                )?,
                512,
            )?,
            0,
        )?;
        let ValueData::Map(map) = source.data() else {
            return Err(Error::Binding("borrowed dictionary seed is not a Map"));
        };
        let count = map.entries().len();
        if schemas.entry(source.schema()).map(|entry| entry.key()) != Some(source.schema_key()) {
            return Err(Error::Binding("borrowed dictionary schema key"));
        }
        string_u64_schema(schemas, source.schema(), count as u64)?;
        if schemas
            .entry(key_snapshot.schema())
            .map(|entry| entry.key())
            != Some(key_snapshot.schema_key())
        {
            return Err(Error::Binding("borrowed dictionary key-snapshot schema"));
        }
        let ValueData::Map(keys) = key_snapshot.data() else {
            return Err(Error::Binding(
                "borrowed dictionary key snapshot is not a Map",
            ));
        };
        string_u64_schema(schemas, key_snapshot.schema(), keys.entries().len() as u64)?;
        if keys.entries().len() != count {
            return Err(Error::Binding("borrowed dictionary key sets differ"));
        }
        check(count as u64, limits.entries, "entries")?;
        let mut key_bytes = 0;
        for (entry, key_entry) in map.entries().iter().zip(keys.entries()) {
            let (ValueData::String(key), ValueData::U64(_)) = (entry.key().data(), entry.value())
            else {
                return Err(Error::Binding("borrowed dictionary key/value kind"));
            };
            if !same && !matches!(key_entry.key().data(),ValueData::String(text) if text==key) {
                return Err(Error::Binding(
                    "borrowed dictionary ordered key sets differ",
                ));
            }
            key_bytes = add(key_bytes, key.len() as u64)?;
        }
        let bytes = mul(count as u64, size_of::<u64>() as u64)?;
        let metadata_bytes = size_of::<Self>() as u64;
        check(
            add(mul(bytes, 2)?, metadata_bytes)?,
            limits.retained_bytes,
            "retained bytes",
        )?;
        check(
            add(mul(bytes, 2)?, metadata_bytes)?,
            limits.temporary_bytes,
            "temporary bytes",
        )?;
        work.charge(add(mul(add(bytes, metadata_bytes)?, 32)?, 256)?, 0)?;
        let metadata = memory.reserve_capacity(metadata_bytes)?;
        let buffer = memory.reserve_capacity(mul(bytes, 2)?)?;
        let nodes = memory.reserve_nodes(2)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| Error::Allocation)?;
        if values.capacity() != count {
            return Err(Error::Allocation);
        }
        for entry in map.entries() {
            let ValueData::U64(value) = entry.value() else {
                return Err(Error::Binding("borrowed dictionary cell kind"));
            };
            values.push(*value);
        }
        Ok(Self {
            values: values.into_boxed_slice(),
            snapshot: key_snapshot.clone(),
            key_bytes,
            work,
            metadata,
            buffer,
            nodes,
        })
    }
    pub(in crate::native_artifact) fn len(&self) -> usize {
        self.values.len()
    }
    pub(in crate::native_artifact) fn key_bytes(&self) -> u64 {
        self.key_bytes
    }
    pub(in crate::native_artifact) fn retained_bytes(&self) -> Result<u64, Error> {
        add(self.metadata.capacity_bytes(), self.buffer.capacity_bytes())
    }
    pub(in crate::native_artifact) fn retained_nodes(&self) -> u64 {
        self.nodes.nodes()
    }
    #[cfg(test)]
    pub(in crate::native_artifact) fn snapshot(&self) -> &Value {
        &self.snapshot
    }
    fn key(&self, index: usize) -> Result<&str, Error> {
        let ValueData::Map(map) = self.snapshot.data() else {
            return Err(Error::Binding("borrowed dictionary lost Map"));
        };
        let Some(entry) = map.entries().get(index) else {
            return Err(Error::Binding("borrowed dictionary ordinal"));
        };
        let ValueData::String(text) = entry.key().data() else {
            return Err(Error::Binding("borrowed dictionary lost key"));
        };
        Ok(text)
    }
    /// Same immutable key-set identity is required. Equal strings in another
    /// snapshot do not authorize indexing this independent current-value bank.
    pub(in crate::native_artifact) fn replace(
        &mut self,
        source: &Value,
        index: usize,
        text: &str,
        incoming: u64,
    ) -> Result<u64, Error> {
        if !source.shares_frozen_storage(&self.snapshot)
            || source
                .memory_budget_retained_bytes(&self.work.memory_budget())
                .is_none()
        {
            return Err(Error::Binding("foreign borrowed dictionary snapshot"));
        }
        let actual = self.key(index)?;
        if actual.as_ptr() != text.as_ptr() || actual.len() != text.len() {
            return Err(Error::Binding("borrowed dictionary ordinal/key differs"));
        }
        self.work.charge(128, 0)?;
        let cell = self
            .values
            .get_mut(index)
            .ok_or(Error::Binding("borrowed dictionary cell"))?;
        Ok(core::mem::replace(cell, incoming))
    }
    /// Descriptive exclusive backing for an uninterrupted generated owner.
    /// The caller pins this bank and the matching genuine key loan until exit.
    pub(in crate::native_artifact) fn native_values(
        &mut self,
        source: &Value,
    ) -> Result<(u64, u64), Error> {
        if !source.shares_frozen_storage(&self.snapshot)
            || source
                .memory_budget_retained_bytes(&self.work.memory_budget())
                .is_none()
        {
            return Err(Error::Binding("foreign fixed-key native source"));
        }
        Ok((self.values.as_mut_ptr() as u64, self.values.len() as u64))
    }
    pub(in crate::native_artifact) fn entries(
        &self,
    ) -> Result<impl ExactSizeIterator<Item = (&str, u64)>, Error> {
        self.work
            .charge(add(mul(self.values.len() as u64, 32)?, 64)?, 0)?;
        let ValueData::Map(map) = self.snapshot.data() else {
            return Err(Error::Binding("borrowed dictionary lost snapshot"));
        };
        Ok(map
            .entries()
            .iter()
            .zip(self.values.iter())
            .map(|(entry, value)| {
                let ValueData::String(key) = entry.key().data() else {
                    unreachable!("validated immutable String key");
                };
                (key.as_ref(), *value)
            }))
    }
}

#[cfg(test)]
#[path = "borrowed_values_tests.rs"]
mod tests;
