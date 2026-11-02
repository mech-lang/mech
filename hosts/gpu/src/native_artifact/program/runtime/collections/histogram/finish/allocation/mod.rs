//! Exact empty buffers under the caller's already admitted finish envelope.
use crate::native_artifact::program::runtime::collections::allocation as admitted;
use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::native_artifact::program::runtime::collections::histogram::finish) enum HistogramAllocationSite
{
    Entries,
    Items,
    Key,
}

pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn vector<T>(
    count: usize,
    kind: HistogramAllocationSite,
    reason: &'static str,
) -> Result<Vec<T>, Error> {
    vector_with(count, kind, reason, |count| {
        #[cfg(all(test, feature = "canonical-native-program"))]
        let count = observation::requested(kind, count)?;
        let mut values = Vec::new();
        values
            .try_reserve_exact(count)
            .map_err(|_| invalid(reason))?;
        Ok(values)
    })
}

pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn vector_with<
    T,
>(
    count: usize,
    _kind: HistogramAllocationSite,
    reason: &'static str,
    allocate: impl FnOnce(usize) -> Result<Vec<T>, Error>,
) -> Result<Vec<T>, Error> {
    let values = allocate(count)?;
    // Reject before a caller can initialize or box even one draft. The shared
    // guard destroys an oversized Vec before returning; no retry or shrink.
    let values = match admitted::exact_capacity(values, count, reason) {
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
    #[cfg(all(test, feature = "canonical-native-program"))]
    observation::accepted(_kind);
    Ok(values)
}

pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn key(
    source: &str,
) -> Result<String, Error> {
    let _kind = HistogramAllocationSite::Key;
    key_with(source, |count| {
        #[cfg(all(test, feature = "canonical-native-program"))]
        let count = observation::requested(_kind, count)?;
        let mut text = String::new();
        text.try_reserve_exact(count)
            .map_err(|_| invalid("Histogram result key allocation"))?;
        Ok(text)
    })
}

pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn key_with(
    source: &str,
    allocate: impl FnOnce(usize) -> Result<String, Error>,
) -> Result<String, Error> {
    let mut text = allocate(source.len())?;
    if text.capacity() != source.len() || !text.is_empty() {
        drop(text); // Complete payload retires before the enclosing claim.
        #[cfg(all(test, feature = "canonical-native-program"))]
        observation::rejected();
        return Err(invalid("Histogram result key allocation"));
    }
    #[cfg(all(test, feature = "canonical-native-program"))]
    observation::accepted(HistogramAllocationSite::Key);
    text.push_str(source); // Exact byte capacity; this cannot grow the buffer.
    Ok(text)
}

#[cfg(all(test, feature = "canonical-native-program"))]
pub(in crate::native_artifact::program::runtime::collections::histogram::finish) mod observation {
    use crate::native_artifact::program::runtime::collections::histogram::finish::allocation::HistogramAllocationSite;
    use crate::native_artifact::program::runtime::{NativeProgramExecutionError as Error, invalid};
    use mech_engine::native_artifact::NativeLibraryWorkBudget;
    use std::cell::RefCell;

    #[derive(Clone, Copy, Debug, Default)]
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish) struct Record {
        pub attempts: [u64; 3],
        pub accepted: [u64; 3],
        pub rejected: u64,
        pub memory_on_reject: u64,
        pub envelope: u64,
        pub work_at_admission: (u64, u64),
        pub work_after: (u64, u64),
    }
    struct State {
        target: Option<(HistogramAllocationSite, u64)>,
        fired: bool,
        work: Option<NativeLibraryWorkBudget>,
        record: Record,
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
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn with<R>(
        target: Option<(HistogramAllocationSite, u64)>,
        invoke: impl FnOnce() -> R,
    ) -> (R, Record) {
        let previous = ACTIVE.with(|active| {
            active.replace(Some(State {
                target,
                fired: false,
                work: None,
                record: Record::default(),
            }))
        });
        let restore = Restore(previous);
        let result = invoke();
        let record = ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            let state = active.as_mut().unwrap();
            if let Some(work) = &state.work {
                state.record.work_after = work.used();
            }
            state.record
        });
        drop(restore); // Retire the observation's account clone before return.
        (result, record)
    }
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish) fn admitted(
        work: &NativeLibraryWorkBudget,
        envelope: u64,
    ) {
        ACTIVE.with(|active| {
            if let Some(state) = active.borrow_mut().as_mut() {
                state.work = Some(work.clone());
                state.record.envelope = envelope;
                state.record.work_at_admission = work.used();
            }
        });
    }
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish::allocation) fn requested(
        kind: HistogramAllocationSite,
        count: usize,
    ) -> Result<usize, Error> {
        ACTIVE.with(|active| {
            let mut active = active.borrow_mut();
            let Some(state) = active.as_mut() else {
                return Ok(count);
            };
            state.record.attempts[kind as usize] += 1;
            if state.target == Some((kind, state.record.attempts[kind as usize])) && !state.fired {
                state.fired = true;
                return count
                    .checked_add(1)
                    .ok_or_else(|| invalid("Histogram test capacity"));
            }
            Ok(count)
        })
    }
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish::allocation) fn accepted(
        kind: HistogramAllocationSite,
    ) {
        ACTIVE.with(|active| {
            if let Some(state) = active.borrow_mut().as_mut() {
                state.record.accepted[kind as usize] += 1;
            }
        });
    }
    pub(in crate::native_artifact::program::runtime::collections::histogram::finish::allocation) fn rejected()
     {
        ACTIVE.with(|active| {
            if let Some(state) = active.borrow_mut().as_mut() {
                state.record.rejected += 1;
                state.record.memory_on_reject = state
                    .work
                    .as_ref()
                    .map_or(0, |work| work.memory_budget().used_bytes());
            }
        });
    }
}
