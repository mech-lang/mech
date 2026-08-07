use std::{
    collections::VecDeque,
    sync::{Arc, Mutex, MutexGuard},
};

use mech_core::{MResult, MechError};

use crate::turn_record::{AccountedRecord, LedgerSequence};

use super::{
    CapacityController, LedgerPermit, PreparedLedgerAppend, RecordEstimate,
    capacity::LedgerAllocationFailed, prepare, reserve,
};

#[derive(Debug)]
struct QueueState<R> {
    records: VecDeque<(LedgerSequence, R)>,
    record_bytes: VecDeque<usize>,
}

/// A bounded, cloneable producer handle for cross-thread owned records.
#[derive(Debug)]
pub struct OwnedTurnRecordQueue<R> {
    state: Arc<Mutex<QueueState<R>>>,
    controller: CapacityController,
}

impl<R> Clone for OwnedTurnRecordQueue<R> {
    fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
            controller: self.controller.clone(),
        }
    }
}

impl<R> OwnedTurnRecordQueue<R> {
    pub fn new(max_records: usize, max_bytes: usize) -> MResult<Self> {
        let controller = CapacityController::new(max_records, max_bytes)?;
        let mut records = VecDeque::new();
        records.try_reserve_exact(max_records).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "queue record slots",
                },
                None,
            )
        })?;
        let mut record_bytes = VecDeque::new();
        record_bytes.try_reserve_exact(max_records).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "queue accounting slots",
                },
                None,
            )
        })?;
        let queue = Self {
            state: Arc::new(Mutex::new(QueueState {
                records,
                record_bytes,
            })),
            controller,
        };
        // Initialize any platform mutex state while construction is still fallible.
        drop(queue.lock());
        Ok(queue)
    }

    pub fn len(&self) -> usize {
        self.lock().records.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn retained_bytes(&self) -> usize {
        self.controller.retained().bytes
    }

    pub fn writer_is_healthy(&self) -> bool {
        self.controller.is_healthy()
    }

    pub fn mark_writer_unhealthy(&self) {
        self.controller.mark_unhealthy();
    }

    pub(crate) fn reserve(&self, estimate: RecordEstimate) -> MResult<LedgerPermit> {
        reserve(&self.controller, estimate)
    }

    pub(crate) fn prepare_append(
        &self,
        permit: LedgerPermit,
        record: R,
    ) -> MResult<PreparedLedgerAppend<R>>
    where
        R: AccountedRecord + Send + 'static,
    {
        prepare(&self.controller, permit, record)
    }

    /// Appends a valid prepared record without allocation or a recoverable failure branch.
    pub(crate) fn append(&self, prepared: PreparedLedgerAppend<R>) -> LedgerSequence
    where
        R: Send + 'static,
    {
        let reservation = prepared.reservation();
        let sequence = reservation.sequence;
        let mut state = self.lock();
        self.controller
            .commit_prepared(prepared.controller(), reservation);
        let (_, record) = prepared.into_parts();
        state.records.push_back((sequence, record));
        state.record_bytes.push_back(reservation.bytes);
        sequence
    }

    pub fn pop_front(&self) -> Option<(LedgerSequence, R)> {
        let mut state = self.lock();
        let entry = state.records.pop_front()?;
        let bytes = state
            .record_bytes
            .pop_front()
            .expect("queued record byte accounting entry");
        self.controller.release_retained(1, bytes);
        Some(entry)
    }

    pub fn drain(&self) -> Vec<(LedgerSequence, R)> {
        let queued = self.len();
        let mut drained = Vec::with_capacity(queued);
        for _ in 0..queued {
            if let Some(record) = self.pop_front() {
                drained.push(record);
            }
        }
        drained
    }

    #[cfg(test)]
    pub(super) fn poison_mutex_for_test(&self) {
        let state = Arc::clone(&self.state);
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(move || {
            let _guard = state.lock().unwrap();
            panic!("poison queue mutex for recovery test");
        }));
    }

    fn lock(&self) -> MutexGuard<'_, QueueState<R>> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}
