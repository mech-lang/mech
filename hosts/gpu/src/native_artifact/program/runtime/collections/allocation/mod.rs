//! Allocator-reported capacity must match each already prepaid construction.
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
use mech_core::{ManagedMemoryBudget, ManagedMemoryReservation};

pub(in crate::native_artifact::program::runtime::collections) fn exact_capacity<T>(
    values: Vec<T>,
    count: usize,
    reason: &'static str,
) -> Result<Vec<T>, Error> {
    if values.capacity() != count {
        // Owned input drops here, before the caller's still-live reservation.
        return Err(invalid(reason));
    }
    Ok(values)
}

pub(in crate::native_artifact::program::runtime::collections) fn metadata<T>(
    count: usize,
    reason: &'static str,
) -> Result<Vec<Option<T>>, Error> {
    metadata_with(count, reason, |count| {
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| invalid(reason))?;
        Ok(values)
    })
}

fn metadata_with<T>(
    count: usize,
    reason: &'static str,
    allocate: impl FnOnce(usize) -> Result<Vec<Option<T>>, Error>,
) -> Result<Vec<Option<T>>, Error> {
    let mut values = exact_capacity(allocate(count)?, count, reason)?;
    // try_reserve_exact promises a minimum. Reject rounding before filling or
    // retaining this buffer; the caller prepaid only count element headers.
    if !values.is_empty() {
        return Err(invalid(reason));
    }
    values.resize_with(count, || None);
    Ok(values)
}

pub(in crate::native_artifact::program::runtime::collections) fn text(
    capacity: u64,
    memory: &ManagedMemoryBudget,
) -> Result<(String, ManagedMemoryReservation), Error> {
    text_with(capacity, memory, |capacity| {
        let mut text = String::new();
        text.try_reserve_exact(capacity)
            .map_err(|_| invalid("TextJoin backing allocation"))?;
        Ok(text)
    })
}

fn text_with(
    capacity: u64,
    memory: &ManagedMemoryBudget,
    allocate: impl FnOnce(usize) -> Result<String, Error>,
) -> Result<(String, ManagedMemoryReservation), Error> {
    let host_capacity =
        usize::try_from(capacity).map_err(|_| invalid("TextJoin capacity exceeds host"))?;
    let reservation = memory.reserve_capacity(capacity)?;
    let text = allocate(host_capacity)?;
    // Keep the old backing and its claim untouched on rejection. This local
    // text drops before reservation on error, just as in the caller's swap.
    if text.capacity() != host_capacity || !text.is_empty() {
        return Err(invalid("TextJoin backing exceeds admitted capacity"));
    }
    Ok((text, reservation))
}

#[cfg(test)]
#[path = "tests.rs"]
pub(in crate::native_artifact::program::runtime::collections::allocation) mod tests;
