//! Admitted private String storage between complete-use-proved operations.
//! This has no frozen Value identity, arena slot, shape, or visit authority.
use super::{InvokeError, NativePreparedLibraryCall};
use crate::resident::budget::NativeLibraryWorkBudget;
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation, SchemaBody, SchemaKey};

/// Unique, initialized UTF-8 backing and its exact same-account capacity claim.
/// Only the actual prepared String-producing factory can construct it. The
/// caller separately retains all original operation admissions and boundaries.
pub struct NativePrivateStringBuffer {
    // Retire the backing before releasing its admitted capacity on every drop.
    text: String,
    reservation: ManagedMemoryReservation,
    key: SchemaKey,
}

impl NativePrivateStringBuffer {
    pub fn as_bytes(&self) -> &[u8] {
        self.text.as_bytes()
    }
    pub const fn schema_key(&self) -> SchemaKey {
        self.key
    }
    pub fn memory_budget(&self) -> ManagedMemoryBudget {
        self.reservation.budget()
    }
    pub fn retained_capacity_bytes(&self) -> u64 {
        self.reservation.capacity_bytes()
    }
    /// Transfers the original initialized allocation, with no copy or scan.
    /// Retain its charge until that backing retires or reaches the final sink.
    pub fn into_parts(self) -> (String, ManagedMemoryReservation) {
        (self.text, self.reservation)
    }
}

impl NativePreparedLibraryCall {
    /// Storage handoff after the original TextJoin's complete work, capacity,
    /// output and node admissions. It neither executes nor authorizes a call.
    /// A compiler use proof, not this owner, permits omitting its frozen Value.
    pub fn finish_private_string_buffer(
        &self,
        text: String,
        reservation: ManagedMemoryReservation,
        work: &NativeLibraryWorkBudget,
    ) -> Result<NativePrivateStringBuffer, InvokeError> {
        use crate::native_artifact::NativeCollectionPrimitive as Primitive;
        let owner = NativePrivateStringBuffer {
            text,
            reservation,
            key: self.output_key,
        };
        if self.template.primitive() != Primitive::TextJoin
            || self.template.output().body() != &SchemaBody::String
            || !self.template.output().dimension_parameters().is_empty()
            || self.output_parameters != 0
            || self.output_key != self.template.output().key()
        {
            return Err(InvokeError::InvalidInput);
        }
        if owner.memory_budget() != work.memory_budget()
            || owner.retained_capacity_bytes() != owner.text.capacity() as u64
        {
            return Err(InvokeError::UnadmittedValue);
        }
        Ok(owner)
    }
}
