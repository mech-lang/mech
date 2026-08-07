//! Bounded owned effect intents prepared before turn publication.

mod retained;
#[cfg(test)]
mod tests;

use mech_core::{MResult, MechError, MechErrorKind};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{AccountedRecord, TurnId};

pub use retained::{PreparedOutboxBatch, RetainedEffectOutbox};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct OutboxEffectId {
    pub turn_id: TurnId,
    pub ordinal: u32,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum OutboxDeliveryPolicy {
    #[default]
    AtLeastOnce,
    ProviderTransactional,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OwnedEffectIntent<P> {
    pub id: OutboxEffectId,
    pub operation: String,
    pub target: String,
    pub payload: P,
    pub idempotency_key: String,
    pub delivery: OutboxDeliveryPolicy,
}

impl<P: AccountedRecord> OwnedEffectIntent<P> {
    pub fn validate(&self) -> MResult<()> {
        self.validated_retained_bytes().map(|_| ())
    }

    pub(super) fn validated_retained_bytes(&self) -> MResult<usize> {
        if self.operation.is_empty() {
            return invalid_effect_intent("operation", "operation must not be empty");
        }
        if self.target.is_empty() {
            return invalid_effect_intent("target", "target must not be empty");
        }
        if self.idempotency_key.is_empty() {
            return invalid_effect_intent("idempotency_key", "idempotency key must not be empty");
        }
        self.accounted_bytes().ok_or_else(|| {
            MechError::new(
                InvalidEffectIntent {
                    field: "effect",
                    reason: "owned effect byte accounting overflowed",
                },
                None,
            )
        })
    }

    fn accounted_bytes(&self) -> Option<usize> {
        self.operation
            .capacity()
            .checked_add(self.target.capacity())?
            .checked_add(self.idempotency_key.capacity())?
            .checked_add(self.payload.retained_bytes())
    }
}

impl<P: AccountedRecord> AccountedRecord for OwnedEffectIntent<P> {
    fn retained_bytes(&self) -> usize {
        self.accounted_bytes()
            .expect("validated owned effect byte accounting")
    }
}

#[derive(Debug)]
pub struct OutboxPermit {
    pub(crate) inner: Option<crate::LedgerPermit>,
}

impl OutboxPermit {
    pub fn reserved_effects(&self) -> usize {
        self.inner
            .as_ref()
            .map_or(0, crate::LedgerPermit::reserved_records)
    }

    pub fn reserved_bytes(&self) -> usize {
        self.inner
            .as_ref()
            .map_or(0, crate::LedgerPermit::reserved_bytes)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DuplicateOutboxEffectId {
    pub id: OutboxEffectId,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidOutboxEffectOrder {
    pub previous: OutboxEffectId,
    pub next: OutboxEffectId,
}

impl MechErrorKind for InvalidOutboxEffectOrder {
    fn name(&self) -> &str {
        "InvalidOutboxEffectOrder"
    }

    fn message(&self) -> String {
        format!(
            "outbox effect {:?} must follow retained effect {:?}",
            self.next, self.previous
        )
    }
}

impl MechErrorKind for DuplicateOutboxEffectId {
    fn name(&self) -> &str {
        "DuplicateOutboxEffectId"
    }

    fn message(&self) -> String {
        format!(
            "duplicate outbox effect ID for turn {} ordinal {}",
            self.id.turn_id, self.id.ordinal
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct InvalidEffectIntent {
    pub field: &'static str,
    pub reason: &'static str,
}

impl MechErrorKind for InvalidEffectIntent {
    fn name(&self) -> &str {
        "InvalidEffectIntent"
    }

    fn message(&self) -> String {
        format!(
            "invalid owned effect field `{}`: {}",
            self.field, self.reason
        )
    }
}

fn invalid_effect_intent<T>(field: &'static str, reason: &'static str) -> MResult<T> {
    Err(MechError::new(InvalidEffectIntent { field, reason }, None))
}
