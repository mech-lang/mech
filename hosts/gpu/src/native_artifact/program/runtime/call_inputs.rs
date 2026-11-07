//! Canonical call owners have a source-derived span, independent of raw words.
use super::*;

pub(super) struct CallInputs {
    backing: NativeTypedStorage<Option<Value>>,
    values: usize,
}

impl CallInputs {
    pub(super) fn new(
        funded: usize,
        values: usize,
        limit: u64,
        memory: &ManagedMemoryBudget,
    ) -> Result<Self, NativeProgramExecutionError> {
        if values > funded {
            return Err(invalid("canonical arguments exceed admitted backing"));
        }
        Ok(Self {
            // Retain the original allocation, initialization and memory floor.
            // The unexposed tail stays None: no installer can access it through
            // this type, and raw numeric words live in different typed backing.
            backing: NativeTypedStorage::new_values(funded, limit, memory)?,
            values,
        })
    }

    pub(super) fn values(&self) -> &[Option<Value>] {
        &self.backing.values()[..self.values]
    }

    pub(super) fn values_mut(&mut self) -> &mut [Option<Value>] {
        &mut self.backing.values_mut()[..self.values]
    }

    /// Conservative original header price, even for unexposed empty slots.
    pub(super) fn funded_len(&self) -> usize {
        self.backing.values().len()
    }
}
