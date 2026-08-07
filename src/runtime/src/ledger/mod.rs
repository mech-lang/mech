//! Bounded owned turn-record storage primitives.

mod capacity;
mod queue;
mod retained;
#[cfg(test)]
mod tests;

use mech_core::MResult;

use crate::turn_record::{AccountedRecord, LedgerSequence};

pub(crate) use capacity::{CapacityController, CapacityReservation};
pub use capacity::{
    LedgerAllocationFailed, LedgerCapacityExceeded, LedgerPermitInvalid, RecordEstimate,
};
pub use queue::OwnedTurnRecordQueue;
pub use retained::{RetainedLedgerDrain, RetainedTurnLedger};

/// An unused reservation for one future ledger append.
#[derive(Debug)]
pub struct LedgerPermit {
    controller: CapacityController,
    ledger_identity: u64,
    ledger_generation: u64,
    reservation: Option<CapacityReservation>,
    consumed: bool,
}

impl LedgerPermit {
    pub fn sequence(&self) -> LedgerSequence {
        self.reservation
            .as_ref()
            .expect("live ledger permit reservation")
            .sequence
    }

    pub fn reserved_records(&self) -> usize {
        self.reservation
            .as_ref()
            .map_or(0, |reservation| reservation.records)
    }

    pub fn reserved_bytes(&self) -> usize {
        self.reservation
            .as_ref()
            .map_or(0, |reservation| reservation.bytes)
    }

    pub fn is_consumed(&self) -> bool {
        self.consumed
    }

    fn new(controller: CapacityController, reservation: CapacityReservation) -> Self {
        Self {
            ledger_identity: controller.identity(),
            ledger_generation: controller.generation(),
            controller,
            reservation: Some(reservation),
            consumed: false,
        }
    }
}

impl Drop for LedgerPermit {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            self.controller.release_reserved(reservation);
        }
    }
}

/// An owned record bound to already-reserved ledger capacity.
#[derive(Debug)]
pub struct PreparedLedgerAppend<R> {
    controller: CapacityController,
    reservation: Option<CapacityReservation>,
    record: Option<R>,
}

impl<R> PreparedLedgerAppend<R> {
    pub fn sequence(&self) -> LedgerSequence {
        self.reservation
            .as_ref()
            .expect("live prepared ledger append reservation")
            .sequence
    }

    pub fn retained_bytes(&self) -> usize {
        self.reservation
            .as_ref()
            .map_or(0, |reservation| reservation.bytes)
    }

    pub(crate) fn into_parts(mut self) -> (CapacityReservation, R) {
        let reservation = self
            .reservation
            .take()
            .expect("prepared append reservation consumed once");
        let record = self
            .record
            .take()
            .expect("prepared append record consumed once");
        (reservation, record)
    }

    pub(crate) fn controller(&self) -> &CapacityController {
        &self.controller
    }
}

impl<R> Drop for PreparedLedgerAppend<R> {
    fn drop(&mut self) {
        if let Some(reservation) = self.reservation.take() {
            self.controller.release_prepared(reservation);
        }
    }
}

/// Reserve, prepare, then infallibly append an owned record.
pub trait TurnLedger<R>
where
    R: AccountedRecord,
{
    fn reserve(&self, estimate: RecordEstimate) -> MResult<LedgerPermit>;

    fn prepare_append(&self, permit: LedgerPermit, record: R) -> MResult<PreparedLedgerAppend<R>>;

    fn append(&mut self, prepared: PreparedLedgerAppend<R>) -> LedgerSequence;
}

pub(crate) fn reserve(
    controller: &CapacityController,
    estimate: RecordEstimate,
) -> MResult<LedgerPermit> {
    let reservation = controller.reserve(estimate)?;
    Ok(LedgerPermit::new(controller.clone(), reservation))
}

pub(crate) fn prepare<R: AccountedRecord>(
    controller: &CapacityController,
    mut permit: LedgerPermit,
    record: R,
) -> MResult<PreparedLedgerAppend<R>> {
    let actual_bytes = record.retained_bytes();
    let reservation = permit
        .reservation
        .take()
        .ok_or_else(|| capacity::invalid_permit("permit reservation has already been consumed"))?;
    let bound = controller.bind(
        &permit.controller,
        permit.ledger_identity,
        permit.ledger_generation,
        permit.consumed,
        reservation,
        actual_bytes,
    );
    let reservation = match bound {
        Ok(reservation) => reservation,
        Err(error) => {
            permit.controller.release_reserved(reservation);
            return Err(error);
        }
    };
    permit.consumed = true;
    Ok(PreparedLedgerAppend {
        controller: controller.clone(),
        reservation: Some(reservation),
        record: Some(record),
    })
}
