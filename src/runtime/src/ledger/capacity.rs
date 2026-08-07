use core::{fmt, num::NonZeroU64};
use std::sync::{
    Arc, Mutex, MutexGuard,
    atomic::{AtomicU64, Ordering},
};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::turn_record::{CheckedSequenceAllocator, LedgerSequence};

static NEXT_LEDGER_IDENTITY: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RecordEstimate {
    pub records: usize,
    pub bytes: usize,
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct CapacityReservation {
    pub(crate) sequence: LedgerSequence,
    pub(crate) records: usize,
    pub(crate) bytes: usize,
}

#[derive(Debug)]
struct CapacityState {
    generation: NonZeroU64,
    max_records: usize,
    max_bytes: usize,
    retained_records: usize,
    retained_bytes: usize,
    reserved_records: usize,
    reserved_bytes: usize,
    prepared_sequence: Option<LedgerSequence>,
    last_appended_sequence: Option<LedgerSequence>,
    healthy: bool,
    sequences: CheckedSequenceAllocator<LedgerSequence>,
}

#[derive(Clone)]
pub(crate) struct CapacityController {
    identity: u64,
    state: Arc<Mutex<CapacityState>>,
}

impl fmt::Debug for CapacityController {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("CapacityController")
            .field("identity", &self.identity)
            .field("generation", &self.generation())
            .finish_non_exhaustive()
    }
}

impl CapacityController {
    pub(crate) fn new(max_records: usize, max_bytes: usize) -> MResult<Self> {
        if max_records == 0 {
            return Err(MechError::new(
                LedgerCapacityExceeded {
                    resource: "records",
                    maximum: max_records,
                    retained: 0,
                    reserved: 0,
                    requested: 1,
                },
                None,
            ));
        }
        let identity = NEXT_LEDGER_IDENTITY
            .fetch_update(Ordering::Relaxed, Ordering::Relaxed, |identity| {
                identity.checked_add(1)
            })
            .map_err(|_| {
                MechError::new(
                    LedgerAllocationFailed {
                        resource: "ledger identity",
                    },
                    None,
                )
            })?;
        Ok(Self {
            identity,
            state: Arc::new(Mutex::new(CapacityState {
                generation: NonZeroU64::MIN,
                max_records,
                max_bytes,
                retained_records: 0,
                retained_bytes: 0,
                reserved_records: 0,
                reserved_bytes: 0,
                prepared_sequence: None,
                last_appended_sequence: None,
                healthy: true,
                sequences: CheckedSequenceAllocator::new(),
            })),
        })
    }

    pub(crate) const fn identity(&self) -> u64 {
        self.identity
    }

    pub(crate) fn generation(&self) -> u64 {
        self.lock().generation.get()
    }

    pub(crate) fn reserve(&self, estimate: RecordEstimate) -> MResult<CapacityReservation> {
        if estimate.records == 0 {
            return Err(invalid_permit(
                "record estimate must reserve at least one record",
            ));
        }
        let mut state = self.lock();
        if !state.healthy {
            return Err(invalid_permit("ledger writer is unhealthy"));
        }
        ensure_capacity(
            "records",
            state.max_records,
            state.retained_records,
            state.reserved_records,
            estimate.records,
        )?;
        ensure_capacity(
            "bytes",
            state.max_bytes,
            state.retained_bytes,
            state.reserved_bytes,
            estimate.bytes,
        )?;
        let sequence = state.sequences.allocate()?;
        state.reserved_records += estimate.records;
        state.reserved_bytes += estimate.bytes;
        Ok(CapacityReservation {
            sequence,
            records: estimate.records,
            bytes: estimate.bytes,
        })
    }

    pub(crate) fn bind(
        &self,
        permit_controller: &Self,
        ledger_identity: u64,
        ledger_generation: u64,
        consumed: bool,
        mut reservation: CapacityReservation,
        actual_bytes: usize,
    ) -> MResult<CapacityReservation> {
        if consumed {
            return Err(invalid_permit("ledger permit has already been consumed"));
        }
        if ledger_identity != self.identity || !Arc::ptr_eq(&permit_controller.state, &self.state) {
            return Err(invalid_permit(
                "ledger permit belongs to a different ledger",
            ));
        }
        let mut state = self.lock();
        if ledger_generation != state.generation.get() {
            return Err(invalid_permit("ledger permit generation is stale"));
        }
        if reservation.records < 1 {
            return Err(invalid_permit("ledger permit does not reserve a record"));
        }
        if state.prepared_sequence.is_some() {
            return Err(invalid_permit(
                "another prepared append is already active for this ledger",
            ));
        }
        if state
            .last_appended_sequence
            .is_some_and(|last| reservation.sequence <= last)
        {
            return Err(invalid_permit(
                "ledger permit sequence does not follow the append watermark",
            ));
        }
        if actual_bytes > reservation.bytes {
            return Err(MechError::new(
                LedgerCapacityExceeded {
                    resource: "prepared record bytes",
                    maximum: reservation.bytes,
                    retained: 0,
                    reserved: 0,
                    requested: actual_bytes,
                },
                None,
            ));
        }
        let unused_records = reservation.records - 1;
        let unused_bytes = reservation.bytes - actual_bytes;
        state.reserved_records -= unused_records;
        state.reserved_bytes -= unused_bytes;
        reservation.records = 1;
        reservation.bytes = actual_bytes;
        state.prepared_sequence = Some(reservation.sequence);
        Ok(reservation)
    }

    pub(crate) fn release_reserved(&self, reservation: CapacityReservation) {
        let mut state = self.lock();
        state.reserved_records = state
            .reserved_records
            .checked_sub(reservation.records)
            .expect("reservation record accounting underflow");
        state.reserved_bytes = state
            .reserved_bytes
            .checked_sub(reservation.bytes)
            .expect("reservation byte accounting underflow");
    }

    pub(crate) fn release_prepared(&self, reservation: CapacityReservation) {
        let mut state = self.lock();
        assert_eq!(
            state.prepared_sequence,
            Some(reservation.sequence),
            "dropped prepared append must own the active preparation lease"
        );
        state.prepared_sequence = None;
        state.reserved_records = state
            .reserved_records
            .checked_sub(reservation.records)
            .expect("prepared reservation record accounting underflow");
        state.reserved_bytes = state
            .reserved_bytes
            .checked_sub(reservation.bytes)
            .expect("prepared reservation byte accounting underflow");
    }

    pub(crate) fn commit_prepared(
        &self,
        prepared_controller: &Self,
        reservation: CapacityReservation,
    ) {
        assert!(
            self.identity == prepared_controller.identity
                && Arc::ptr_eq(&self.state, &prepared_controller.state),
            "prepared append must retain its originating ledger controller"
        );
        let mut state = self.lock();
        assert_eq!(
            state.prepared_sequence,
            Some(reservation.sequence),
            "prepared append sequence must match the active preparation lease"
        );
        state.prepared_sequence = None;
        state.last_appended_sequence = Some(reservation.sequence);
        state.reserved_records = state
            .reserved_records
            .checked_sub(reservation.records)
            .expect("committed reservation record accounting underflow");
        state.reserved_bytes = state
            .reserved_bytes
            .checked_sub(reservation.bytes)
            .expect("committed reservation byte accounting underflow");
        state.retained_records = state
            .retained_records
            .checked_add(reservation.records)
            .expect("retained record accounting overflow");
        state.retained_bytes = state
            .retained_bytes
            .checked_add(reservation.bytes)
            .expect("retained byte accounting overflow");
    }

    pub(crate) fn release_retained(&self, records: usize, bytes: usize) {
        let mut state = self.lock();
        state.retained_records = state
            .retained_records
            .checked_sub(records)
            .expect("retained record accounting underflow");
        state.retained_bytes = state
            .retained_bytes
            .checked_sub(bytes)
            .expect("retained byte accounting underflow");
    }

    pub(crate) fn retained(&self) -> RecordEstimate {
        let state = self.lock();
        RecordEstimate {
            records: state.retained_records,
            bytes: state.retained_bytes,
        }
    }

    pub(crate) fn reserved(&self) -> RecordEstimate {
        let state = self.lock();
        RecordEstimate {
            records: state.reserved_records,
            bytes: state.reserved_bytes,
        }
    }

    pub(crate) fn mark_unhealthy(&self) {
        self.lock().healthy = false;
    }

    #[cfg(test)]
    pub(super) fn force_generation_for_test(&self, generation: NonZeroU64) {
        self.lock().generation = generation;
    }

    #[cfg(test)]
    pub(super) fn set_sequence_for_test(&self, next: NonZeroU64) {
        self.lock().sequences = CheckedSequenceAllocator::starting_at(next);
    }

    fn lock(&self) -> MutexGuard<'_, CapacityState> {
        self.state
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
    }
}

fn ensure_capacity(
    resource: &'static str,
    maximum: usize,
    retained: usize,
    reserved: usize,
    requested: usize,
) -> MResult<()> {
    let used = retained.checked_add(reserved).ok_or_else(|| {
        MechError::new(
            LedgerCapacityExceeded {
                resource,
                maximum,
                retained,
                reserved,
                requested,
            },
            None,
        )
    })?;
    let total = used.checked_add(requested).ok_or_else(|| {
        MechError::new(
            LedgerCapacityExceeded {
                resource,
                maximum,
                retained,
                reserved,
                requested,
            },
            None,
        )
    })?;
    if total > maximum {
        return Err(MechError::new(
            LedgerCapacityExceeded {
                resource,
                maximum,
                retained,
                reserved,
                requested,
            },
            None,
        ));
    }
    Ok(())
}

pub(crate) fn invalid_permit(reason: &'static str) -> MechError {
    MechError::new(LedgerPermitInvalid { reason }, None)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerCapacityExceeded {
    pub resource: &'static str,
    pub maximum: usize,
    pub retained: usize,
    pub reserved: usize,
    pub requested: usize,
}

impl MechErrorKind for LedgerCapacityExceeded {
    fn name(&self) -> &str {
        "LedgerCapacityExceeded"
    }

    fn message(&self) -> String {
        format!(
            "ledger {} capacity exceeded: retained {}, reserved {}, requested {}, maximum {}",
            self.resource, self.retained, self.reserved, self.requested, self.maximum
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerPermitInvalid {
    pub reason: &'static str,
}

impl MechErrorKind for LedgerPermitInvalid {
    fn name(&self) -> &str {
        "LedgerPermitInvalid"
    }

    fn message(&self) -> String {
        format!("invalid ledger permit: {}", self.reason)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LedgerAllocationFailed {
    pub resource: &'static str,
}

impl MechErrorKind for LedgerAllocationFailed {
    fn name(&self) -> &str {
        "LedgerAllocationFailed"
    }

    fn message(&self) -> String {
        format!("failed to allocate bounded ledger {}", self.resource)
    }
}
