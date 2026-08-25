use std::collections::VecDeque;

use mech_core::{MResult, MechError};

use crate::turn_record::{AccountedRecord, LedgerSequence};

use super::{
    CapacityController, LedgerPermit, PreparedLedgerAppend, RecordEstimate, TurnLedger,
    capacity::LedgerAllocationFailed, prepare, reserve,
};

/// A bounded FIFO ledger that retains owned records until explicitly drained.
#[derive(Debug)]
pub struct RetainedTurnLedger<R> {
    records: VecDeque<(LedgerSequence, R)>,
    record_bytes: VecDeque<usize>,
    controller: CapacityController,
}

impl<R> RetainedTurnLedger<R> {
    pub fn new(max_records: usize, max_bytes: usize) -> MResult<Self> {
        let controller = CapacityController::new(max_records, max_bytes)?;
        let mut records = VecDeque::new();
        records.try_reserve_exact(max_records).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "record slots",
                },
                None,
            )
        })?;
        let mut record_bytes = VecDeque::new();
        record_bytes.try_reserve_exact(max_records).map_err(|_| {
            MechError::new(
                LedgerAllocationFailed {
                    resource: "record accounting slots",
                },
                None,
            )
        })?;
        Ok(Self {
            records,
            record_bytes,
            controller,
        })
    }

    #[cfg(any(test, feature = "runtime_bench_probes"))]
    pub fn len(&self) -> usize {
        self.records.len()
    }

    #[cfg(any(test, feature = "runtime_bench_probes"))]
    pub fn is_empty(&self) -> bool {
        self.records.is_empty()
    }

    #[cfg(any(test, feature = "runtime_bench_probes"))]
    pub fn retained_bytes(&self) -> usize {
        self.controller.retained().bytes
    }

    pub fn iter(&self) -> impl Iterator<Item = (LedgerSequence, &R)> {
        self.records
            .iter()
            .map(|(sequence, record)| (*sequence, record))
    }

    #[cfg(feature = "runtime_bench_gate_b")]
    pub fn last(&self) -> Option<(LedgerSequence, &R)> {
        self.records
            .back()
            .map(|(sequence, record)| (*sequence, record))
    }

    pub fn pop_front(&mut self) -> Option<(LedgerSequence, R)> {
        let entry = self.records.pop_front()?;
        let bytes = self
            .record_bytes
            .pop_front()
            .expect("retained record byte accounting entry");
        self.controller.release_retained(1, bytes);
        Some(entry)
    }

    #[cfg(any(test, feature = "runtime_bench_probes"))]
    pub fn drain(&mut self) -> RetainedLedgerDrain<'_, R> {
        RetainedLedgerDrain { ledger: self }
    }

    #[cfg(test)]
    pub(super) fn set_sequence_for_test(&self, next: core::num::NonZeroU64) {
        self.controller.set_sequence_for_test(next);
    }
}

impl<R: AccountedRecord> TurnLedger<R> for RetainedTurnLedger<R> {
    fn reserve(&self, estimate: RecordEstimate) -> MResult<LedgerPermit> {
        reserve(&self.controller, estimate)
    }

    fn prepare_append(&self, permit: LedgerPermit, record: R) -> MResult<PreparedLedgerAppend<R>> {
        prepare(&self.controller, permit, record)
    }

    fn append(&mut self, prepared: PreparedLedgerAppend<R>) -> LedgerSequence {
        let reservation = prepared.reservation();
        let sequence = reservation.sequence;
        self.controller
            .commit_prepared(prepared.controller(), reservation);
        let (_, record) = prepared.into_parts();
        self.records.push_back((sequence, record));
        self.record_bytes.push_back(reservation.bytes);
        sequence
    }
}

#[cfg(any(test, feature = "runtime_bench_probes"))]
pub struct RetainedLedgerDrain<'a, R> {
    ledger: &'a mut RetainedTurnLedger<R>,
}

#[cfg(any(test, feature = "runtime_bench_probes"))]
impl<R> Iterator for RetainedLedgerDrain<'_, R> {
    type Item = (LedgerSequence, R);

    fn next(&mut self) -> Option<Self::Item> {
        self.ledger.pop_front()
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let len = self.ledger.len();
        (len, Some(len))
    }
}

#[cfg(any(test, feature = "runtime_bench_probes"))]
impl<R> ExactSizeIterator for RetainedLedgerDrain<'_, R> {}

#[cfg(any(test, feature = "runtime_bench_probes"))]
impl<R> Drop for RetainedLedgerDrain<'_, R> {
    fn drop(&mut self) {
        while self.ledger.pop_front().is_some() {}
    }
}
