//! Consuming ownership transfer, never mutation of an immutable snapshot.
use super::*;

/// A genuine detached String and its original same-account capacity claim.
/// No canonical Value or old frozen identity represents its mutable contents.
#[derive(Debug)]
pub struct OwnedSnapshotString {
    // Backing/schema owners retire before their transferred charge.
    text: String,
    schemas: Arc<SchemaTable>,
    reservation: crate::ManagedMemoryReservation,
    schema: SchemaId,
    key: SchemaKey,
}

/// Mutable backing consumed from a uniquely owned String. Its metadata is
/// SOURCE provenance, not a claim that the current bytes are a canonical String.
/// Revalidate UTF8 before constructing a subsequent String/Value.
#[derive(Debug)]
pub struct OwnedSnapshotBytes {
    bytes: Vec<u8>,
    schemas: Arc<SchemaTable>,
    reservation: crate::ManagedMemoryReservation,
    schema: SchemaId,
    key: SchemaKey,
}

impl OwnedSnapshotBytes {
    pub fn as_bytes(&self) -> &[u8] {
        &self.bytes
    }
    pub fn as_mut_bytes(&mut self) -> &mut [u8] {
        &mut self.bytes
    }
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.reservation.budget()
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    pub fn into_parts(self) -> (Vec<u8>, crate::ManagedMemoryReservation) {
        let Self {
            bytes,
            schemas,
            reservation,
            ..
        } = self;
        drop(schemas);
        (bytes, reservation)
    }
}

#[derive(Debug)]
pub enum StringOwnershipTransfer {
    /// Unsupported/shared values remain completely unchanged, including their
    /// frozen allocation identity, admission and budget-import registration.
    Unavailable(Value),
    Owned(OwnedSnapshotString),
}

impl OwnedSnapshotString {
    pub fn as_str(&self) -> &str {
        &self.text
    }
    pub fn into_bytes(self) -> OwnedSnapshotBytes {
        let Self {
            text,
            schemas,
            reservation,
            schema,
            key,
        } = self;
        OwnedSnapshotBytes {
            bytes: text.into_bytes(),
            schemas,
            reservation,
            schema,
            key,
        }
    }
    pub const fn schema(&self) -> SchemaId {
        self.schema
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn memory_budget(&self) -> crate::ManagedMemoryBudget {
        self.reservation.budget()
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    /// The caller must retain the charge until the returned backing retires or
    /// is handed to another admitted owner. The tuple drops backing first.
    pub fn into_parts(self) -> (String, crate::ManagedMemoryReservation) {
        let Self {
            text,
            schemas,
            reservation,
            ..
        } = self;
        drop(schemas);
        (text, reservation)
    }
}

impl Value {
    /// Consume a uniquely owned, plain canonical String without copying its
    /// payload. This is a storage operation, not permission to retire a Mech
    /// value: compiled callers must separately prove the value's last use.
    ///
    /// Both data and wrapper must be exclusive. Cross-budget imports, ordinary
    /// clones and physical retained-payload tickets conservatively stay frozen.
    /// Weak frozen identities are allowed, but can never identify a subsequently
    /// frozen result: try_unwrap retires the OLD data Arc control block.
    pub fn try_into_owned_string(
        self,
        budget: &crate::ManagedMemoryBudget,
    ) -> crate::MemoryRuntimeResult<StringOwnershipTransfer> {
        if !matches!(self.data(), ValueData::String(_))
            || !self.shape.parameter_values().is_empty()
            || self.memory_budget_retained_bytes(budget).is_none()
            || self.root.data.ownership.get().is_some()
            || Arc::strong_count(&self.root) != 1
            || Arc::strong_count(&self.root.data) != 1
        {
            return Ok(StringOwnershipTransfer::Unavailable(self));
        }
        let Self {
            schema,
            schema_key,
            shape,
            root,
            schemas,
        } = self;
        let mut storage = match Arc::try_unwrap(root) {
            Ok(storage) => storage,
            Err(root) => {
                return Ok(StringOwnershipTransfer::Unavailable(Self {
                    schema,
                    schema_key,
                    shape,
                    root,
                    schemas,
                }));
            }
        };
        // The data Arc is private to this module. Public weak identities cannot
        // upgrade it, and any live Value capable of importing/cloning it would
        // have failed the exclusive wrapper/data checks above. No callback or
        // user code runs between those checks and consuming both owners.
        storage.unlink_budget_registration();
        let ownership = storage
            .memory_budget
            .take()
            .ok_or_else(|| transfer_invariant("exclusive String lost its admission"))?;
        let data = storage.data.clone();
        drop(storage); // The now-unlinked wrapper no longer owns the charge.
        let frozen = Arc::try_unwrap(data)
            .map_err(|_| transfer_invariant("exclusive String gained a frozen data owner"))?;
        let FrozenSnapshotData {
            data,
            resident_token,
            shape: data_shape,
            ownership: physical,
            budget_imports,
        } = frozen;
        let ValueData::String(text) = data else {
            return Err(transfer_invariant("exclusive String changed data kind"));
        };
        // Retire stale import/shape/token metadata before allowing any capacity
        // credit. The original weak data shell may remain cache-owned, but has
        // zero strong owners and cannot match a new frozen allocation.
        drop((resident_token, data_shape, physical, budget_imports, shape));
        let SnapshotBudgetOwnership {
            registration,
            schemas: admitted_schemas,
            required_bytes: _,
            retained_nodes: _,
            reservation,
        } = ownership;
        drop(registration);
        drop(schemas);
        Ok(StringOwnershipTransfer::Owned(OwnedSnapshotString {
            text: text.into_string(),
            schemas: admitted_schemas,
            reservation,
            schema,
            key: schema_key,
        }))
    }
}

fn transfer_invariant(reason: &str) -> crate::MemoryRuntimeError {
    crate::MemoryRuntimeError::CandidateValidationFailed {
        object: None,
        reason: reason.into(),
    }
}

#[cfg(test)]
#[path = "owned_string_tests.rs"]
mod tests;
