//! Owned turn-record identities and status metadata.

use core::{fmt, num::NonZeroU64};

use mech_core::{MResult, MechError, MechErrorKind};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg(test)]
use crate::RuntimeEventKind;
use crate::TransactionId;

macro_rules! sequence_id {
    ($name:ident) => {
        #[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
        #[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
        #[repr(transparent)]
        pub struct $name(NonZeroU64);

        impl $name {
            pub const fn new(value: u64) -> Option<Self> {
                match NonZeroU64::new(value) {
                    Some(value) => Some(Self(value)),
                    None => None,
                }
            }

            pub const fn get(self) -> u64 {
                self.0.get()
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                self.0.fmt(formatter)
            }
        }

        impl fmt::Debug for $name {
            fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
                formatter
                    .debug_tuple(stringify!($name))
                    .field(&self.get())
                    .finish()
            }
        }

        impl TryFrom<u64> for $name {
            type Error = InvalidTurnRecord;

            fn try_from(value: u64) -> Result<Self, Self::Error> {
                Self::new(value).ok_or(InvalidTurnRecord {
                    field: stringify!($name),
                    reason: "sequence identifiers must be non-zero".to_string(),
                })
            }
        }

        impl From<$name> for u64 {
            fn from(value: $name) -> Self {
                value.get()
            }
        }
    };
}

sequence_id!(TurnId);
sequence_id!(InputSequence);
sequence_id!(LedgerSequence);

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct InputSequenceRange {
    pub first: InputSequence,
    pub last: InputSequence,
}

impl InputSequenceRange {
    pub fn new(first: InputSequence, last: InputSequence) -> MResult<Self> {
        if first > last {
            return invalid_turn_record("input_range", "first input follows last input");
        }
        Ok(Self { first, last })
    }

    pub const fn first(self) -> InputSequence {
        self.first
    }

    pub const fn last(self) -> InputSequence {
        self.last
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TurnRecordStatus {
    Accepted,
    Rejected,
    Staged,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum TurnFailurePhase {
    Admission,
    InputInstallation,
    Execution,
    Integrity,
    EffectMaterialization,
    ExternalPrepare,
    ExternalApply,
    Publication,
    ExternalCommit,
    EffectDelivery,
    Finalization,
    Recording,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TurnFailureRecord {
    pub phase: TurnFailurePhase,
    pub kind: String,
    pub message: String,
}

impl TurnFailureRecord {
    pub fn validate(&self) -> MResult<()> {
        if self.kind.is_empty() {
            return invalid_turn_record("failure.kind", "failure kind must not be empty");
        }
        if self.message.is_empty() {
            return invalid_turn_record("failure.message", "failure message must not be empty");
        }
        self.kind
            .capacity()
            .checked_add(self.message.capacity())
            .ok_or_else(|| {
                MechError::new(
                    InvalidTurnRecord {
                        field: "failure",
                        reason: "failure text byte accounting overflowed".to_string(),
                    },
                    None,
                )
            })?;
        Ok(())
    }

    fn retained_bytes(&self) -> usize {
        self.kind
            .capacity()
            .checked_add(self.message.capacity())
            .expect("validated turn failure byte accounting")
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct TurnRecordHeader {
    pub turn_id: TurnId,
    pub transaction_id: TransactionId,
    pub input_range: Option<InputSequenceRange>,
    pub status: TurnRecordStatus,
    pub failure: Option<TurnFailureRecord>,
}

/// Projects only the existing standalone transaction lifecycle for compatibility tests.
#[cfg(test)]
pub(crate) fn project_transaction_lifecycle_events(
    header: &TurnRecordHeader,
) -> MResult<impl Iterator<Item = RuntimeEventKind>> {
    header.validate()?;
    let started = RuntimeEventKind::TransactionStarted {
        transaction_id: header.transaction_id,
    };
    let completed = match (&header.status, &header.failure) {
        (TurnRecordStatus::Accepted, None) => Some(RuntimeEventKind::TransactionCommitted {
            transaction_id: header.transaction_id,
        }),
        (TurnRecordStatus::Rejected, Some(failure)) => Some(RuntimeEventKind::TransactionAborted {
            transaction_id: header.transaction_id,
            message: failure.message.clone(),
        }),
        (TurnRecordStatus::Staged, None) => None,
        _ => unreachable!("TurnRecordHeader::validate accepted an invalid status/failure pair"),
    };
    Ok([Some(started), completed].into_iter().flatten())
}

impl TurnRecordHeader {
    pub fn validate(&self) -> MResult<()> {
        if self.transaction_id.is_zero() {
            return invalid_turn_record("transaction_id", "transaction ID must be non-zero");
        }
        if let Some(range) = self.input_range {
            InputSequenceRange::new(range.first, range.last)?;
        }
        match (&self.status, &self.failure) {
            (TurnRecordStatus::Accepted, Some(_)) => {
                return invalid_turn_record("failure", "accepted turns may not contain a failure");
            }
            (TurnRecordStatus::Rejected, None) => {
                return invalid_turn_record("failure", "rejected turns require a failure");
            }
            (TurnRecordStatus::Staged, Some(_)) => {
                return invalid_turn_record("failure", "staged turns may not contain a failure");
            }
            _ => {}
        }
        if let Some(failure) = &self.failure {
            failure.validate()?;
        }
        Ok(())
    }

    fn retained_bytes(&self) -> usize {
        self.failure
            .as_ref()
            .map_or(0, TurnFailureRecord::retained_bytes)
    }
}

pub(crate) mod sealed {
    pub trait Sealed {}
}

/// Reports the owned heap bytes retained by a trusted record representation.
///
/// The private supertrait prevents callers of the benchmark facade from
/// supplying unaccounted payload implementations.
pub trait AccountedRecord: sealed::Sealed {
    /// Validates trusted semantic structure before byte accounting is bound.
    fn validate_for_recording(&self) -> MResult<()> {
        Ok(())
    }

    fn retained_bytes(&self) -> usize;
}

impl sealed::Sealed for Vec<u8> {}
impl AccountedRecord for Vec<u8> {
    fn retained_bytes(&self) -> usize {
        self.capacity()
    }
}

impl sealed::Sealed for String {}
impl AccountedRecord for String {
    fn retained_bytes(&self) -> usize {
        self.capacity()
    }
}

impl sealed::Sealed for Box<[u8]> {}
impl AccountedRecord for Box<[u8]> {
    fn retained_bytes(&self) -> usize {
        self.len()
    }
}

/// Fixed private receipt used only by the Gate B efficacy benchmark.
///
/// The payload is inline so binding and retained append require no turn-time
/// heap allocation. It is not the canonical resident receipt model.
#[doc(hidden)]
#[cfg(feature = "runtime_bench_gate_b")]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GateBFixedReceipt {
    words: [u64; 8],
}

#[cfg(feature = "runtime_bench_gate_b")]
impl GateBFixedReceipt {
    pub const RETAINED_BYTES: usize = core::mem::size_of::<Self>();

    pub fn accepted(
        before_epoch: u64,
        after_epoch: u64,
        state_hash: u64,
        touched: u16,
        changed: u16,
        dirty_nodes: u16,
    ) -> Self {
        Self {
            words: [
                before_epoch,
                after_epoch,
                state_hash,
                u64::from(touched),
                u64::from(changed),
                u64::from(dirty_nodes),
                1,
                0,
            ],
        }
    }

    pub fn rejected(before_epoch: u64) -> Self {
        Self {
            words: [before_epoch, 0, 0, 0, 0, 0, 2, 0],
        }
    }

    pub fn before_epoch(&self) -> u64 {
        self.words[0]
    }

    pub fn after_epoch(&self) -> u64 {
        self.words[1]
    }

    pub fn state_hash(&self) -> u64 {
        self.words[2]
    }

    pub fn touched_slots(&self) -> u16 {
        self.words[3] as u16
    }

    pub fn changed_slots(&self) -> u16 {
        self.words[4] as u16
    }

    pub fn dirty_nodes(&self) -> u16 {
        self.words[5] as u16
    }

    pub fn is_accepted(&self) -> bool {
        self.words[6] == 1
    }

    pub fn version(&self) -> u64 {
        self.words[7]
    }
}

#[cfg(feature = "runtime_bench_gate_b")]
impl sealed::Sealed for GateBFixedReceipt {}

#[cfg(feature = "runtime_bench_gate_b")]
impl AccountedRecord for GateBFixedReceipt {
    fn retained_bytes(&self) -> usize {
        Self::RETAINED_BYTES
    }
}

impl sealed::Sealed for crate::ledger::PooledRecordBuffer {}

impl<P: AccountedRecord> sealed::Sealed for crate::outbox::OwnedEffectIntent<P> {}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedTurnRecord<B> {
    pub header: TurnRecordHeader,
    pub body: B,
}

impl<B: AccountedRecord> OwnedTurnRecord<B> {
    pub fn validate(&self) -> MResult<()> {
        self.header.validate()?;
        self.header
            .retained_bytes()
            .checked_add(self.body.retained_bytes())
            .ok_or_else(|| {
                MechError::new(
                    InvalidTurnRecord {
                        field: "record",
                        reason: "record byte accounting overflowed".to_string(),
                    },
                    None,
                )
            })?;
        Ok(())
    }
}

impl<B: AccountedRecord> sealed::Sealed for OwnedTurnRecord<B> {}

impl<B: AccountedRecord> AccountedRecord for OwnedTurnRecord<B> {
    fn validate_for_recording(&self) -> MResult<()> {
        self.validate()
    }

    fn retained_bytes(&self) -> usize {
        self.header
            .retained_bytes()
            .checked_add(self.body.retained_bytes())
            .expect("validated owned turn record byte accounting")
    }
}

pub(crate) trait CheckedSequence: Copy {
    const NAME: &'static str;
    fn from_non_zero(value: NonZeroU64) -> Self;
}

impl CheckedSequence for TurnId {
    const NAME: &'static str = "TurnId";

    fn from_non_zero(value: NonZeroU64) -> Self {
        Self(value)
    }
}

impl CheckedSequence for InputSequence {
    const NAME: &'static str = "InputSequence";

    fn from_non_zero(value: NonZeroU64) -> Self {
        Self(value)
    }
}

impl CheckedSequence for LedgerSequence {
    const NAME: &'static str = "LedgerSequence";

    fn from_non_zero(value: NonZeroU64) -> Self {
        Self(value)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct CheckedSequenceAllocator<S> {
    next: Option<NonZeroU64>,
    marker: core::marker::PhantomData<S>,
}

impl<S: CheckedSequence> CheckedSequenceAllocator<S> {
    pub(crate) fn new() -> Self {
        Self::starting_at(NonZeroU64::MIN)
    }

    #[cfg(test)]
    pub(crate) fn starting_at(next: NonZeroU64) -> Self {
        Self {
            next: Some(next),
            marker: core::marker::PhantomData,
        }
    }

    #[cfg(not(test))]
    fn starting_at(next: NonZeroU64) -> Self {
        Self {
            next: Some(next),
            marker: core::marker::PhantomData,
        }
    }

    pub(crate) fn allocate(&mut self) -> MResult<S> {
        let next = self
            .next
            .ok_or_else(|| MechError::new(SequenceExhausted { sequence: S::NAME }, None))?;
        self.next = next.get().checked_add(1).and_then(NonZeroU64::new);
        Ok(S::from_non_zero(next))
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InvalidTurnRecord {
    pub field: &'static str,
    pub reason: String,
}

impl MechErrorKind for InvalidTurnRecord {
    fn name(&self) -> &str {
        "InvalidTurnRecord"
    }

    fn message(&self) -> String {
        format!(
            "invalid turn record field `{}`: {}",
            self.field, self.reason
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SequenceExhausted {
    pub sequence: &'static str,
}

impl MechErrorKind for SequenceExhausted {
    fn name(&self) -> &str {
        "SequenceExhausted"
    }

    fn message(&self) -> String {
        format!("{} sequence is exhausted", self.sequence)
    }
}

fn invalid_turn_record<T>(field: &'static str, reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        InvalidTurnRecord {
            field,
            reason: reason.into(),
        },
        None,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checked_sequence_allocation_never_wraps() {
        let mut allocator = CheckedSequenceAllocator::<TurnId>::starting_at(NonZeroU64::MAX);
        assert_eq!(allocator.allocate().unwrap().get(), u64::MAX);
        let error = allocator.allocate().unwrap_err();
        assert_eq!(error.kind_name(), "SequenceExhausted");
    }

    #[test]
    fn header_status_and_input_range_are_validated() {
        let header = TurnRecordHeader {
            turn_id: TurnId::new(1).unwrap(),
            transaction_id: TransactionId::new(1),
            input_range: Some(InputSequenceRange {
                first: InputSequence::new(2).unwrap(),
                last: InputSequence::new(1).unwrap(),
            }),
            status: TurnRecordStatus::Accepted,
            failure: None,
        };
        assert_eq!(
            header.validate().unwrap_err().kind_name(),
            "InvalidTurnRecord"
        );
    }

    fn assert_owned_record<T: Send + 'static>() {}

    fn build_owned_record() -> OwnedTurnRecord<String> {
        let builder_text = String::from("owned after builder drop");
        OwnedTurnRecord {
            header: TurnRecordHeader {
                turn_id: TurnId::new(1).unwrap(),
                transaction_id: TransactionId::new(1),
                input_range: None,
                status: TurnRecordStatus::Accepted,
                failure: None,
            },
            body: builder_text,
        }
    }

    #[test]
    fn owned_record_outlives_builder_and_workspace_reuse() {
        assert_owned_record::<OwnedTurnRecord<String>>();
        let record = build_owned_record();
        assert_eq!(record.body, "owned after builder drop");

        let mut workspace = vec![1_u8, 2, 3, 4];
        let record = OwnedTurnRecord {
            header: record.header,
            body: workspace.clone().into_boxed_slice(),
        };
        workspace.fill(0);
        drop(workspace);
        assert_eq!(&*record.body, &[1, 2, 3, 4]);
    }

    #[test]
    fn staged_projection_has_no_final_lifecycle_event() {
        let header = TurnRecordHeader {
            turn_id: TurnId::new(1).unwrap(),
            transaction_id: TransactionId::new(1),
            input_range: None,
            status: TurnRecordStatus::Staged,
            failure: None,
        };
        assert_eq!(
            project_transaction_lifecycle_events(&header)
                .unwrap()
                .count(),
            1
        );
    }

    #[test]
    fn header_rejects_zero_transaction_and_staged_failure() {
        let zero_transaction = TurnRecordHeader {
            turn_id: TurnId::new(1).unwrap(),
            transaction_id: TransactionId::ZERO,
            input_range: None,
            status: TurnRecordStatus::Accepted,
            failure: None,
        };
        assert_eq!(
            zero_transaction.validate().unwrap_err().kind_name(),
            "InvalidTurnRecord"
        );

        let staged_failure = TurnRecordHeader {
            turn_id: TurnId::new(1).unwrap(),
            transaction_id: TransactionId::new(1),
            input_range: None,
            status: TurnRecordStatus::Staged,
            failure: Some(TurnFailureRecord {
                phase: TurnFailurePhase::Execution,
                kind: "Rejected".to_string(),
                message: "not valid for staged".to_string(),
            }),
        };
        assert_eq!(
            staged_failure.validate().unwrap_err().kind_name(),
            "InvalidTurnRecord"
        );
    }

    #[test]
    fn failure_accounting_includes_retained_string_capacity() {
        let mut kind = String::with_capacity(128);
        kind.push('k');
        let mut message = String::with_capacity(256);
        message.push('m');
        let failure = TurnFailureRecord {
            phase: TurnFailurePhase::Execution,
            kind,
            message,
        };
        failure.validate().unwrap();
        assert!(failure.retained_bytes() >= 384);
    }
}
