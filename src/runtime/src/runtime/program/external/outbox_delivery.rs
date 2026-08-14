use mech_core::{
    ApplicationRequirement, ApplicationRequirementId, EffectDeliveryPolicy, ExternalInteraction,
    MResult, MechError, MechErrorKind, ProgramRevision, ReactiveInstanceId, Value, ValueHash,
    canonical_application_requirement_bytes,
};

use crate::{
    RuntimeEffectId,
    outbox::OutboxDeliveryPolicy,
    turn_record::{AccountedRecord, TurnId, sealed::Sealed},
};

#[derive(Clone, Debug)]
pub struct ResidentOutboxPayload {
    pub requirement: ApplicationRequirementId,
    pub value: Value,
    pub payload_hash: ValueHash,
    pub attempts: u32,
    retained_bytes: usize,
}

impl ResidentOutboxPayload {
    pub fn new(
        requirement: ApplicationRequirementId,
        value: Value,
        payload_hash: ValueHash,
        retained_bytes: usize,
    ) -> Self {
        Self {
            requirement,
            value,
            payload_hash,
            attempts: 0,
            retained_bytes,
        }
    }
}

impl Sealed for ResidentOutboxPayload {}

impl AccountedRecord for ResidentOutboxPayload {
    fn retained_bytes(&self) -> usize {
        self.retained_bytes
    }
}

pub fn resident_outbox_policy(
    interaction: &ExternalInteraction,
) -> MResult<Option<OutboxDeliveryPolicy>> {
    match interaction {
        ExternalInteraction::Effect(effect) => Ok(Some(match effect.delivery {
            EffectDeliveryPolicy::AtMostOnce => OutboxDeliveryPolicy::AtMostOnce,
            EffectDeliveryPolicy::AtLeastOnce => OutboxDeliveryPolicy::AtLeastOnce,
            EffectDeliveryPolicy::IdempotentRetry => OutboxDeliveryPolicy::IdempotentRetry,
            EffectDeliveryPolicy::ProviderDefined => {
                return Err(MechError::new(
                    InvalidResidentEffectDelivery {
                        reason: "ProviderDefined is not resident-admissible".to_owned(),
                    },
                    None,
                ));
            }
        })),
        ExternalInteraction::TransactionalExternal(_) => Ok(None),
        _ => Err(MechError::new(
            InvalidResidentEffectDelivery {
                reason: "resident outbox entry is not an effect".to_owned(),
            },
            None,
        )),
    }
}

pub fn resident_transaction_id(instance: ReactiveInstanceId, turn: TurnId) -> crate::TransactionId {
    let namespace = (u128::from(instance.index()) << 96)
        | (u128::from(instance.generation()) << 64)
        | u128::from(turn.get());
    crate::TransactionId(namespace)
}

pub fn resident_effect_id(
    instance: ReactiveInstanceId,
    turn: TurnId,
    ordinal: u32,
) -> RuntimeEffectId {
    RuntimeEffectId {
        transaction: resident_transaction_id(instance, turn),
        sequence: u64::from(ordinal),
    }
}

pub fn resident_effect_ids_hash(ids: impl IntoIterator<Item = RuntimeEffectId>) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"mech-resident-effect-ids-v1");
    for id in ids {
        hash.update(&id.transaction.0.to_le_bytes());
        hash.update(&id.sequence.to_le_bytes());
    }
    *hash.finalize().as_bytes()
}

pub fn resident_idempotency_keys_hash<'a>(keys: impl IntoIterator<Item = &'a str>) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"mech-resident-idempotency-keys-v1");
    for key in keys {
        hash.update(&(key.len() as u64).to_le_bytes());
        hash.update(key.as_bytes());
    }
    *hash.finalize().as_bytes()
}

pub fn resident_idempotency_key(
    instance: ReactiveInstanceId,
    revision: ProgramRevision,
    turn: TurnId,
    ordinal: u32,
    requirement: &ApplicationRequirement,
    payload_hash: ValueHash,
) -> MResult<String> {
    let mut hash = blake3::Hasher::new();
    hash.update(b"mech-resident-effect-idempotency-v1");
    hash.update(&instance.index().to_le_bytes());
    hash.update(&instance.generation().to_le_bytes());
    hash.update(revision.as_bytes());
    hash.update(&turn.get().to_le_bytes());
    hash.update(&ordinal.to_le_bytes());
    hash.update(&canonical_application_requirement_bytes(requirement)?);
    hash.update(payload_hash.as_bytes());
    Ok(hash.finalize().to_hex().to_string())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InvalidResidentEffectDelivery {
    pub reason: String,
}

impl MechErrorKind for InvalidResidentEffectDelivery {
    fn name(&self) -> &str {
        "InvalidResidentEffectDelivery"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}
