//! Empty exact-capacity carriers under the existing call-lane reservation.
use super::{NativeProgramExecutionError as Error, collection, invalid};

pub(super) fn allocate<T>(
    count: usize,
    bytes: u64,
    initialize: impl FnMut(usize) -> T,
) -> Result<Vec<T>, Error> {
    allocate_with(
        count,
        |count| {
            #[cfg(all(test, feature = "canonical-native-program"))]
            let count = observation::requested(count)?;
            let mut values = Vec::new();
            values.try_reserve_exact(count).map_err(|_| {
                mech_core::MemoryRuntimeError::AllocationFailed {
                    object: None,
                    requested: bytes,
                    alignment: core::mem::align_of::<T>() as u32,
                    space: mech_core::MemorySpace::Host,
                }
            })?;
            Ok(values)
        },
        initialize,
    )
}

fn allocate_with<T>(
    count: usize,
    allocate: impl FnOnce(usize) -> Result<Vec<T>, Error>,
    mut initialize: impl FnMut(usize) -> T,
) -> Result<Vec<T>, Error> {
    let reason = "call cache backing exceeds admitted capacity";
    let mut values = match collection::admitted_capacity(allocate(count)?, count, reason) {
        Ok(values) if values.is_empty() => values,
        Ok(values) => {
            drop(values);
            #[cfg(all(test, feature = "canonical-native-program"))]
            observation::rejected();
            return Err(invalid(reason));
        }
        Err(error) => {
            #[cfg(all(test, feature = "canonical-native-program"))]
            observation::rejected();
            return Err(error);
        }
    };
    // No Once or Repeated entry is initialized until the SAME shared guard
    // has checked actual backing. Filling cannot grow, and len==capacity
    // ensures the subsequent boxed conversion does not need shrink backing.
    for ordinal in 0..count {
        #[cfg(all(test, feature = "canonical-native-program"))]
        observation::initialized();
        values.push(initialize(ordinal));
    }
    Ok(values)
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(super) fn with_extra<R>(
    extra: usize,
    memory: &mech_core::ManagedMemoryBudget,
    invoke: impl FnOnce() -> R,
) -> (R, (usize, usize, usize, u64)) {
    observation::with(extra, memory, invoke)
}

#[cfg(all(test, feature = "canonical-native-program"))]
mod observation {
    use super::*;
    use mech_core::ManagedMemoryBudget;
    use std::cell::RefCell;

    struct State {
        extra: usize,
        memory: ManagedMemoryBudget,
        stats: (usize, usize, usize, u64),
    }
    thread_local! {
        static ACTIVE: RefCell<Option<State>> = const { RefCell::new(None) };
    }
    struct Restore(Option<State>);
    impl Drop for Restore {
        fn drop(&mut self) {
            ACTIVE.with(|active| *active.borrow_mut() = self.0.take());
        }
    }
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
        drop(restore);
        (result, stats)
    }
    pub(super) fn requested(count: usize) -> Result<usize, Error> {
        ACTIVE.with(|active| match active.borrow_mut().as_mut() {
            Some(state) => {
                state.stats.0 += 1;
                count
                    .checked_add(state.extra)
                    .ok_or_else(|| invalid("call cache allocation overflow"))
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
#[path = "program_call_cache_backing_tests.rs"]
mod tests;
