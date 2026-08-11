//! Private complete-turn coordinator for the retained Gate B efficacy proof.

use mech_core::{MResult, MechError, MechErrorKind};
use mech_engine::__gate_b_resident::{
    PreparedResidentFullWrite, PreparedResidentTurn as PreparedGateBResidentTurn,
    ResidentExecutionError, ResidentTurnSummary as GateBResidentTurnSummary,
};
use mech_engine::__gate_d::{
    PreparedArtifactResidentTurn, ResidentTurnSummary as ArtifactResidentTurnSummary,
};

use crate::{
    TransactionId,
    ledger::{LedgerPermit, PreparedLedgerAppend, RecordEstimate, RetainedTurnLedger, TurnLedger},
    turn_record::{
        GateBFixedReceipt, InputSequence, InputSequenceRange, LedgerSequence, OwnedTurnRecord,
        TurnFailurePhase, TurnFailureRecord, TurnId, TurnRecordHeader, TurnRecordStatus,
    },
};

pub(crate) type ResidentTurnRecord = OwnedTurnRecord<GateBFixedReceipt>;
const RESIDENT_RECORD_RESERVATION_BYTES: usize = 256;
const RESIDENT_RECORD_ESTIMATE: RecordEstimate = RecordEstimate {
    records: 1,
    bytes: RESIDENT_RECORD_RESERVATION_BYTES,
};

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
    ledger: &'ledger mut RetainedTurnLedger<ResidentTurnRecord>,
    prepared: PreparedLedgerAppend<ResidentTurnRecord>,
}

impl PreparedResidentAppend<'_> {
    #[inline]
    fn append(self) -> LedgerSequence {
        TurnLedger::append(self.ledger, self.prepared)
    }
}

pub struct ResidentTurnRecorder {
    ledger: RetainedTurnLedger<ResidentTurnRecord>,
    permits: Box<[Option<LedgerPermit>]>,
    next_turn: Option<u64>,
    records_inspected: usize,
    fail_next_preparation: bool,
}

impl ResidentTurnRecorder {
    pub fn new(episode_turns: usize, retained_history: usize) -> MResult<Self> {
        let capacity = retained_history
            .checked_add(episode_turns)
            .ok_or_else(|| error("ledger capacity overflow"))?;
        let max_bytes = capacity
            .checked_mul(RESIDENT_RECORD_RESERVATION_BYTES)
            .ok_or_else(|| error("ledger byte capacity overflow"))?;
        let mut ledger = RetainedTurnLedger::new(capacity, max_bytes)?;
        for ordinal in 1..=retained_history {
            let identity =
                u64::try_from(ordinal).map_err(|_| error("history identity overflow"))?;
            let record = accepted_record(
                identity,
                GateBResidentTurnSummary {
                    before_epoch: identity.saturating_sub(1),
                    after_epoch: identity,
                    state_hash: 0,
                    touched_slots: 0,
                    changed_slots: 0,
                    dirty_nodes: 0,
                },
            )?;
            let permit = TurnLedger::reserve(&ledger, RESIDENT_RECORD_ESTIMATE)?;
            let prepared = TurnLedger::prepare_append(&ledger, permit, record)?;
            TurnLedger::append(&mut ledger, prepared);
        }

        let permits = (0..episode_turns)
            .map(|_| TurnLedger::reserve(&ledger, RESIDENT_RECORD_ESTIMATE).map(Some))
            .collect::<MResult<Vec<_>>>()?
            .into_boxed_slice();
        let next_turn = u64::try_from(retained_history)
            .ok()
            .and_then(|history| history.checked_add(1))
            .ok_or_else(|| error("turn identity overflow"))?;
        Ok(Self {
            ledger,
            permits,
            next_turn: Some(next_turn),
            records_inspected: 0,
            fail_next_preparation: false,
        })
    }

    pub fn take_admission_permit(&mut self, turn: usize) -> MResult<LedgerPermit> {
        self.permits
            .get_mut(turn)
            .and_then(Option::take)
            .ok_or_else(|| error("resident turn has no unused admission permit"))
    }

    fn prepare_accepted_append(
        &mut self,
        permit: LedgerPermit,
        summary: GateBResidentTurnSummary,
    ) -> MResult<PreparedResidentAppend<'_>> {
        let identity = self.take_turn_identity()?;
        if core::mem::take(&mut self.fail_next_preparation) {
            drop(permit);
            return Err(error("forced resident ledger preparation failure"));
        }
        let record = accepted_record(identity, summary)?;
        let prepared = TurnLedger::prepare_append(&self.ledger, permit, record)?;
        Ok(PreparedResidentAppend {
            ledger: &mut self.ledger,
            prepared,
        })
    }

    pub fn prepare_commit<'instance>(
        &mut self,
        permit: LedgerPermit,
        turn: PreparedGateBResidentTurn<'instance>,
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

    pub fn prepare_artifact_commit<'instance>(
        &mut self,
        permit: LedgerPermit,
        turn: PreparedArtifactResidentTurn<'instance>,
    ) -> MResult<PreparedResidentCommit<'instance, '_>> {
        let summary = turn.summary();
        let identity = match self.take_turn_identity() {
            Ok(identity) => identity,
            Err(error) => {
                turn.abort();
                return Err(error);
            }
        };
        if core::mem::take(&mut self.fail_next_preparation) {
            drop(permit);
            turn.abort();
            return Err(error("forced resident ledger preparation failure"));
        }
        let record = match accepted_artifact_record(identity, summary) {
            Ok(record) => record,
            Err(error) => {
                turn.abort();
                return Err(error);
            }
        };
        match TurnLedger::prepare_append(&self.ledger, permit, record) {
            Ok(prepared) => Ok(PreparedResidentCommit {
                turn: PreparedResidentPublication::Artifact(turn),
                append: PreparedResidentAppend {
                    ledger: &mut self.ledger,
                    prepared,
                },
            }),
            Err(error) => {
                turn.abort();
                Err(error)
            }
        }
    }

    pub fn prepare_full_write_commit<'instance>(
        &mut self,
        permit: LedgerPermit,
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
        permit: LedgerPermit,
        before_epoch: u64,
        failure: ResidentExecutionError,
    ) -> MResult<PreparedRejectedAppend<'_>> {
        let identity = self.take_turn_identity()?;
        let record = rejected_record(identity, before_epoch, failure)?;
        let prepared = TurnLedger::prepare_append(&self.ledger, permit, record)?;
        Ok(PreparedRejectedAppend(PreparedResidentAppend {
            ledger: &mut self.ledger,
            prepared,
        }))
    }

    pub fn recorded_ledger_len(&self) -> usize {
        self.ledger.len()
    }

    pub fn records_inspected(&self) -> usize {
        self.records_inspected
    }

    pub fn inspect_last(&mut self) -> Option<ResidentRecordInspection<'_>> {
        self.records_inspected += 1;
        let (sequence, record) = self.ledger.last()?;
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
    pub fn reserve_additional_permit_for_test(&self) -> MResult<LedgerPermit> {
        TurnLedger::reserve(&self.ledger, RESIDENT_RECORD_ESTIMATE)
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
) -> MResult<TurnRecordHeader> {
    let input =
        InputSequence::new(identity).ok_or_else(|| error("input identity must be non-zero"))?;
    Ok(TurnRecordHeader {
        turn_id: TurnId::new(identity).ok_or_else(|| error("turn identity must be non-zero"))?,
        transaction_id: TransactionId::new(u128::from(identity)),
        input_range: Some(InputSequenceRange::new(input, input)?),
        status,
        failure,
    })
}

fn accepted_record(
    identity: u64,
    summary: GateBResidentTurnSummary,
) -> MResult<ResidentTurnRecord> {
    Ok(OwnedTurnRecord {
        header: header(identity, TurnRecordStatus::Accepted, None)?,
        body: GateBFixedReceipt::accepted(
            summary.before_epoch,
            summary.after_epoch,
            summary.state_hash,
            summary.touched_slots,
            summary.changed_slots,
            summary.dirty_nodes,
        ),
    })
}

fn accepted_artifact_record(
    identity: u64,
    summary: ArtifactResidentTurnSummary,
) -> MResult<ResidentTurnRecord> {
    Ok(OwnedTurnRecord {
        header: header(identity, TurnRecordStatus::Accepted, None)?,
        body: GateBFixedReceipt::accepted(
            summary.before_epoch.get(),
            summary.after_epoch.get(),
            summary.state_hash,
            summary.touched_slots,
            summary.changed_slots,
            summary.dirty_nodes,
        ),
    })
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
        ResidentExecutionError::IncompleteCandidate => (
            TurnFailurePhase::Integrity,
            "ResidentIncompleteCandidate",
            "resident candidate did not fully materialize its state",
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
        )?,
        body: GateBFixedReceipt::rejected(before_epoch),
    })
}

pub struct PreparedResidentCommit<'instance, 'ledger> {
    turn: PreparedResidentPublication<'instance>,
    append: PreparedResidentAppend<'ledger>,
}

enum PreparedResidentPublication<'instance> {
    Ekf(PreparedGateBResidentTurn<'instance>),
    Artifact(PreparedArtifactResidentTurn<'instance>),
    FullWrite(PreparedResidentFullWrite<'instance>),
}

impl PreparedResidentPublication<'_> {
    #[inline]
    fn publish(self) {
        match self {
            Self::Ekf(turn) => turn.publish(),
            Self::Artifact(turn) => {
                turn.publish();
            }
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
