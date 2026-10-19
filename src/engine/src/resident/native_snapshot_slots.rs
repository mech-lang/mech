//! Exact prepaid Slot carriers; no cache, measurement or admission authority.
use mech_core::ResidentKernelError;

pub(super) fn allocate<T>(
    count: usize,
    initialize: impl FnMut(usize) -> T,
) -> Result<Vec<T>, ResidentKernelError> {
    allocate_with(
        count,
        |count| {
            // A test-only response models an allocator reporting extra capacity;
            // production always requests the already-admitted count exactly.
            #[cfg(test)]
            let count = observation::requested(count)?;
            let mut values = Vec::new();
            values
                .try_reserve_exact(count)
                .map_err(|_| ResidentKernelError::InvalidShape)?;
            Ok(values)
        },
        initialize,
    )
}

fn allocate_with<T>(
    count: usize,
    allocate: impl FnOnce(usize) -> Result<Vec<T>, ResidentKernelError>,
    mut initialize: impl FnMut(usize) -> T,
) -> Result<Vec<T>, ResidentKernelError> {
    let mut values = allocate(count)?;
    if values.capacity() != count || !values.is_empty() {
        // No Slot, weak identity, or measured DATA may be initialized/installed.
        // The enclosing cache constructor still owns its prepaid reservation.
        drop(values);
        #[cfg(test)]
        observation::rejected();
        return Err(ResidentKernelError::InvalidShape);
    }
    for ordinal in 0..count {
        #[cfg(test)]
        observation::initialized();
        values.push(initialize(ordinal));
    }
    Ok(values)
}

#[cfg(all(test, any(feature = "full_source", feature = "full_compiler")))]
impl super::super::NativeTurnPlanCache {
    pub(crate) fn with_data_slot_allocation_extra<R>(
        extra: usize,
        memory: &mech_core::ManagedMemoryBudget,
        invoke: impl FnOnce() -> R,
    ) -> (R, (usize, usize, usize, u64)) {
        observation::with(extra, memory, invoke)
    }
}

#[cfg(test)]
mod observation {
    use mech_core::{ManagedMemoryBudget, ResidentKernelError};
    use std::cell::RefCell;

    struct State {
        extra: usize,
        memory: ManagedMemoryBudget,
        stats: (usize, usize, usize, u64), // requests, initializations, rejected drops, claim after drop
    }
    thread_local! {
        static ACTIVE: RefCell<Option<State>> = const { RefCell::new(None) };
    }
    #[cfg(any(feature = "full_source", feature = "full_compiler"))]
    struct Restore(Option<State>);
    #[cfg(any(feature = "full_source", feature = "full_compiler"))]
    impl Drop for Restore {
        fn drop(&mut self) {
            ACTIVE.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
    #[cfg(any(feature = "full_source", feature = "full_compiler"))]
    pub(super) fn with<R>(
        extra: usize,
        memory: &ManagedMemoryBudget,
        invoke: impl FnOnce() -> R,
    ) -> (R, (usize, usize, usize, u64)) {
        let previous = ACTIVE.with(|active| {
            active.replace(Some(State {
                extra,
                memory: memory.clone(),
                stats: (0, 0, 0, 0),
            }))
        });
        let restore = Restore(previous);
        let result = invoke();
        let stats = ACTIVE.with(|active| active.borrow().as_ref().unwrap().stats);
        drop(restore); // Also restores the prior hook on unwind.
        (result, stats)
    }
    pub(super) fn requested(count: usize) -> Result<usize, ResidentKernelError> {
        ACTIVE.with(|active| match active.borrow_mut().as_mut() {
            Some(state) => {
                state.stats.0 += 1;
                count
                    .checked_add(state.extra)
                    .ok_or(ResidentKernelError::InvalidShape)
            }
            None => Ok(count),
        })
    }
    pub(super) fn initialized() {
        ACTIVE.with(|active| {
            if let Some(state) = active.borrow_mut().as_mut() {
                state.stats.1 += 1;
            }
        });
    }
    pub(super) fn rejected() {
        ACTIVE.with(|active| {
            if let Some(state) = active.borrow_mut().as_mut() {
                state.stats.2 += 1;
                state.stats.3 = state.memory.used_bytes();
            }
        });
    }
}

#[cfg(test)]
#[path = "native_snapshot_slots_tests.rs"]
mod tests;
