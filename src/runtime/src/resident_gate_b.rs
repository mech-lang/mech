//! Private complete-turn coordinator for the retained Gate B efficacy proof.

use mech_core::{MResult, MechError, MechErrorKind};
use mech_engine::__gate_b_resident::{
    PreparedResidentFullWrite, PreparedResidentTurn, ResidentExecutionError, ResidentTurnSummary,
};

use crate::{
    TransactionId,
    turn_record::{
        GateBFixedReceipt, InputSequence, InputSequenceRange, LedgerSequence, OwnedTurnRecord,
        TurnFailurePhase, TurnFailureRecord, TurnId, TurnRecordHeader, TurnRecordStatus,
    },
};

pub(crate) type ResidentTurnRecord = OwnedTurnRecord<GateBFixedReceipt>;

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResidentRecorderFailure(&'static str);

impl MechErrorKind for ResidentRecorderFailure {
    fn name(&self) -> &str {
        "ResidentRecorderFailure"
    }

    fn message(&self) -> String {
        self.0.to_string()
    }
}

fn error(message: &'static str) -> MechError {
    MechError::new(ResidentRecorderFailure(message), None)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentRecordInspection<'a> {
    pub sequence: u64,
    pub turn_id: u64,
    pub transaction_id: u128,
    pub input_first: u64,
    pub input_last: u64,
    pub accepted: bool,
    pub failure_phase: Option<TurnFailurePhase>,
    pub failure_kind: Option<&'a str>,
    pub body: GateBFixedReceipt,
}

struct PreparedResidentAppend<'ledger> {
    recorder: &'ledger mut ResidentTurnRecorder,
    sequence: LedgerSequence,
    record: ResidentTurnRecord,
}

struct PreparedResidentInPlaceAppend<'ledger> {
    recorder: &'ledger mut ResidentTurnRecorder,
    sequence: LedgerSequence,
}

impl PreparedResidentAppend<'_> {
    #[inline]
    fn append(self) -> LedgerSequence {
        debug_assert!(self.recorder.recorded_len < self.recorder.records.len());
        self.recorder.records[self.recorder.recorded_len] = (self.sequence, self.record);
        self.recorder.recorded_len += 1;
        self.sequence
    }
}

impl PreparedResidentInPlaceAppend<'_> {
    #[inline]
    fn append(self) -> LedgerSequence {
        debug_assert!(self.recorder.recorded_len < self.recorder.records.len());
        self.recorder.recorded_len += 1;
        self.sequence
    }
}

/// An admission token backed by storage reserved when the recorder is created.
#[must_use = "admitted resident turns must be prepared or explicitly abandoned"]
pub struct ResidentAdmissionPermit {
    _private: (),
}

pub struct ResidentTurnRecorder {
    // Gate B has one turn owner and a fixed admission window. Reserving the
    // complete window here makes successful append an infallible slot write.
    records: Box<[(LedgerSequence, ResidentTurnRecord)]>,
    recorded_len: usize,
    permits: Box<[Option<ResidentAdmissionPermit>]>,
    next_turn: Option<u64>,
    records_inspected: usize,
    fail_next_preparation: bool,
}

impl ResidentTurnRecorder {
    pub fn new(episode_turns: usize, retained_history: usize) -> MResult<Self> {
        let capacity = retained_history
            .checked_add(episode_turns)
            .ok_or_else(|| error("ledger capacity overflow"))?;
        let mut records = Vec::new();
        records
            .try_reserve_exact(capacity)
            .map_err(|_| error("resident receipt storage allocation failed"))?;
        for ordinal in 1..=capacity {
            let identity = if ordinal <= retained_history {
                u64::try_from(ordinal).map_err(|_| error("history identity overflow"))?
            } else {
                1
            };
            records.push((
                LedgerSequence::new(identity).expect("non-zero reserved identity"),
                accepted_record(
                    identity,
                    ResidentTurnSummary {
                        before_epoch: identity.saturating_sub(1),
                        after_epoch: identity,
                        state_hash: 0,
                        touched_slots: 0,
                        changed_slots: 0,
                        dirty_nodes: 0,
                    },
                ),
            ));
        }
        let records = records.into_boxed_slice();
        debug_assert_eq!(records.len(), capacity);
        let permits = (0..episode_turns)
            .map(|_| Some(ResidentAdmissionPermit { _private: () }))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        let next_turn = u64::try_from(retained_history)
            .ok()
            .and_then(|history| history.checked_add(1))
            .ok_or_else(|| error("turn identity overflow"))?;
        Ok(Self {
            records,
            recorded_len: retained_history,
            permits,
            next_turn: Some(next_turn),
            records_inspected: 0,
            fail_next_preparation: false,
        })
    }

    #[inline]
    pub fn take_admission_permit(&mut self, turn: usize) -> MResult<ResidentAdmissionPermit> {
        self.permits
            .get_mut(turn)
            .and_then(Option::take)
            .ok_or_else(|| error("resident turn has no unused admission permit"))
    }

    fn prepare_accepted_append(
        &mut self,
        _permit: ResidentAdmissionPermit,
        summary: ResidentTurnSummary,
    ) -> MResult<PreparedResidentAppend<'_>> {
        let identity = self.take_turn_identity()?;
        if core::mem::take(&mut self.fail_next_preparation) {
            return Err(error("forced resident ledger preparation failure"));
        }
        let sequence = LedgerSequence::new(identity)
            .ok_or_else(|| error("resident ledger sequence must be non-zero"))?;
        let record = accepted_record(identity, summary);
        Ok(PreparedResidentAppend {
            recorder: self,
            sequence,
            record,
        })
    }

    fn prepare_accepted_append_in_place(
        &mut self,
        _permit: ResidentAdmissionPermit,
        summary: ResidentTurnSummary,
    ) -> MResult<PreparedResidentInPlaceAppend<'_>> {
        let identity = self.take_turn_identity()?;
        if core::mem::take(&mut self.fail_next_preparation) {
            return Err(error("forced resident ledger preparation failure"));
        }
        let sequence = LedgerSequence::new(identity)
            .ok_or_else(|| error("resident ledger sequence must be non-zero"))?;
        debug_assert!(self.recorded_len < self.records.len());
        self.records[self.recorded_len] = (sequence, accepted_record(identity, summary));
        Ok(PreparedResidentInPlaceAppend {
            recorder: self,
            sequence,
        })
    }

    pub fn prepare_commit<'instance>(
        &mut self,
        permit: ResidentAdmissionPermit,
        turn: PreparedResidentTurn<'instance>,
    ) -> MResult<PreparedResidentCommit<'instance, '_>> {
        let summary = turn.summary();
        match self.prepare_accepted_append(permit, summary) {
            Ok(append) => Ok(PreparedResidentCommit {
                turn: PreparedResidentPublication::Ekf(turn),
                append,
            }),
            Err(error) => {
                turn.abort();
                Err(error)
            }
        }
    }

    #[doc(hidden)]
    pub fn prepare_commit_in_place<'instance>(
        &mut self,
        permit: ResidentAdmissionPermit,
        turn: PreparedResidentTurn<'instance>,
    ) -> MResult<PreparedResidentInPlaceCommit<'instance, '_>> {
        let summary = turn.summary();
        match self.prepare_accepted_append_in_place(permit, summary) {
            Ok(append) => Ok(PreparedResidentInPlaceCommit { turn, append }),
            Err(error) => {
                turn.abort();
                Err(error)
            }
        }
    }

    pub fn prepare_full_write_commit<'instance>(
        &mut self,
        permit: ResidentAdmissionPermit,
        turn: PreparedResidentFullWrite<'instance>,
    ) -> MResult<PreparedResidentCommit<'instance, '_>> {
        let summary = turn.summary();
        match self.prepare_accepted_append(permit, summary) {
            Ok(append) => Ok(PreparedResidentCommit {
                turn: PreparedResidentPublication::FullWrite(turn),
                append,
            }),
            Err(error) => {
                turn.abort();
                Err(error)
            }
        }
    }

    pub fn prepare_rejected(
        &mut self,
        _permit: ResidentAdmissionPermit,
        before_epoch: u64,
        failure: ResidentExecutionError,
    ) -> MResult<PreparedRejectedAppend<'_>> {
        let identity = self.take_turn_identity()?;
        let sequence = LedgerSequence::new(identity)
            .ok_or_else(|| error("resident ledger sequence must be non-zero"))?;
        let record = rejected_record(identity, before_epoch, failure)?;
        Ok(PreparedRejectedAppend(PreparedResidentAppend {
            recorder: self,
            sequence,
            record,
        }))
    }

    pub fn recorded_ledger_len(&self) -> usize {
        self.recorded_len
    }

    pub fn records_inspected(&self) -> usize {
        self.records_inspected
    }

    pub fn inspect_last(&mut self) -> Option<ResidentRecordInspection<'_>> {
        self.records_inspected += 1;
        let (sequence, record) = self.records.get(self.recorded_len.checked_sub(1)?)?;
        let range = record.header.input_range?;
        Some(ResidentRecordInspection {
            sequence: sequence.get(),
            turn_id: record.header.turn_id.get(),
            transaction_id: record.header.transaction_id.as_u128(),
            input_first: range.first().get(),
            input_last: range.last().get(),
            accepted: record.header.status == TurnRecordStatus::Accepted,
            failure_phase: record.header.failure.as_ref().map(|failure| failure.phase),
            failure_kind: record
                .header
                .failure
                .as_ref()
                .map(|failure| failure.kind.as_str()),
            body: record.body,
        })
    }

    #[doc(hidden)]
    pub fn fail_next_preparation_for_test(&mut self) {
        self.fail_next_preparation = true;
    }

    #[doc(hidden)]
    pub fn set_next_turn_identity_for_test(&mut self, next_turn: u64) {
        assert_ne!(next_turn, 0, "resident turn identities are non-zero");
        self.next_turn = Some(next_turn);
    }

    #[doc(hidden)]
    pub fn reserve_additional_permit_for_test(&self) -> MResult<ResidentAdmissionPermit> {
        if self.recorded_len >= self.records.len() {
            return Err(error("resident receipt capacity exhausted"));
        }
        Ok(ResidentAdmissionPermit { _private: () })
    }

    fn take_turn_identity(&mut self) -> MResult<u64> {
        let identity = self
            .next_turn
            .take()
            .ok_or_else(|| error("turn identity exhausted"))?;
        self.next_turn = identity.checked_add(1);
        Ok(identity)
    }
}

pub struct PreparedRejectedAppend<'ledger>(PreparedResidentAppend<'ledger>);

impl PreparedRejectedAppend<'_> {
    pub fn append(self) -> LedgerSequence {
        self.0.append()
    }
}

fn header(
    identity: u64,
    status: TurnRecordStatus,
    failure: Option<TurnFailureRecord>,
) -> TurnRecordHeader {
    let input = InputSequence::new(identity).expect("resident input identity is non-zero");
    TurnRecordHeader {
        turn_id: TurnId::new(identity).expect("resident turn identity is non-zero"),
        transaction_id: TransactionId::new(u128::from(identity)),
        input_range: Some(InputSequenceRange {
            first: input,
            last: input,
        }),
        status,
        failure,
    }
}

fn accepted_record(identity: u64, summary: ResidentTurnSummary) -> ResidentTurnRecord {
    OwnedTurnRecord {
        header: header(identity, TurnRecordStatus::Accepted, None),
        body: GateBFixedReceipt::accepted(
            summary.before_epoch,
            summary.after_epoch,
            summary.state_hash,
            summary.touched_slots,
            summary.changed_slots,
            summary.dirty_nodes,
        ),
    }
}

fn rejected_record(
    identity: u64,
    before_epoch: u64,
    failure: ResidentExecutionError,
) -> MResult<ResidentTurnRecord> {
    let (phase, kind, message) = match failure {
        ResidentExecutionError::EpochExhausted => (
            TurnFailurePhase::Execution,
            "ResidentEpochExhausted",
            "resident candidate epoch exhausted",
        ),
        ResidentExecutionError::LandmarkDistance => (
            TurnFailurePhase::Integrity,
            "ResidentLandmarkDistance",
            "landmark distance integrity failed",
        ),
        ResidentExecutionError::InnovationDeterminant => (
            TurnFailurePhase::Integrity,
            "ResidentInnovationDeterminant",
            "innovation determinant integrity failed",
        ),
        ResidentExecutionError::NonFiniteState => (
            TurnFailurePhase::Integrity,
            "ResidentNonFiniteState",
            "resident state contains a non-finite value",
        ),
        ResidentExecutionError::CovarianceDiagonal => (
            TurnFailurePhase::Integrity,
            "ResidentCovarianceDiagonal",
            "resident covariance diagonal is invalid",
        ),
        ResidentExecutionError::CovarianceSymmetry => (
            TurnFailurePhase::Integrity,
            "ResidentCovarianceSymmetry",
            "resident covariance is not symmetric",
        ),
    };
    Ok(OwnedTurnRecord {
        header: header(
            identity,
            TurnRecordStatus::Rejected,
            Some(TurnFailureRecord {
                phase,
                kind: kind.to_string(),
                message: message.to_string(),
            }),
        ),
        body: GateBFixedReceipt::rejected(before_epoch),
    })
}

pub struct PreparedResidentCommit<'instance, 'ledger> {
    turn: PreparedResidentPublication<'instance>,
    append: PreparedResidentAppend<'ledger>,
}

enum PreparedResidentPublication<'instance> {
    Ekf(PreparedResidentTurn<'instance>),
    FullWrite(PreparedResidentFullWrite<'instance>),
}

impl PreparedResidentPublication<'_> {
    #[inline]
    fn publish(self) {
        match self {
            Self::Ekf(turn) => turn.publish(),
            Self::FullWrite(turn) => turn.publish(),
        }
    }
}

impl PreparedResidentCommit<'_, '_> {
    #[inline]
    pub fn commit(self) -> LedgerSequence {
        self.turn.publish();
        self.append.append()
    }
}

#[must_use = "prepared resident commits must be committed"]
pub struct PreparedResidentInPlaceCommit<'instance, 'ledger> {
    turn: PreparedResidentTurn<'instance>,
    append: PreparedResidentInPlaceAppend<'ledger>,
}

impl PreparedResidentInPlaceCommit<'_, '_> {
    #[doc(hidden)]
    pub fn recorded_ledger_len(&self) -> usize {
        self.append.recorder.recorded_len
    }

    #[doc(hidden)]
    pub fn published_epoch(&self) -> u64 {
        self.turn.published_epoch()
    }

    #[doc(hidden)]
    pub fn published_state(
        &self,
        index: usize,
    ) -> mech_engine::__gate_b_resident::ResidentEkfState {
        self.turn.published_state(index)
    }

    #[inline]
    pub fn commit(self) -> LedgerSequence {
        self.turn.publish();
        self.append.append()
    }
}
