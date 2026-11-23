//! Canonical call owners have a source-derived span, independent of raw words.
use super::*;

pub(super) struct CallInputs {
    backing: NativeTypedStorage<Option<Value>>,
    funded: usize,
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
            // The same full claim and setup price remain in force; only the
            // canonical-owner prefix is physically initialized. Raw numeric
            // arguments retain their separate original typed word backing.
            backing: NativeTypedStorage::new_values_compact(funded, values, limit, memory)?,
            funded,
        })
    }

    pub(super) fn values(&self) -> &[Option<Value>] {
        self.backing.values()
    }

    pub(super) fn values_mut(&mut self) -> &mut [Option<Value>] {
        self.backing.values_mut()
    }

    /// Conservative original header price, even for unexposed empty slots.
    pub(super) fn funded_len(&self) -> usize {
        self.funded
    }

    #[cfg(test)]
    pub(super) fn physical_len(&self) -> usize {
        self.backing.values().len()
    }
}
