use mech_core::{
    InstanceEpoch, LayoutGeneration, PlanGeneration, ProgramRevision, ReactiveInstanceId,
};

use crate::turn_record::{AccountedRecord, OwnedTurnRecord, sealed::Sealed};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentTurnReceiptV1 {
    pub version: u16,
    pub instance: ReactiveInstanceId,
    pub program_revision: ProgramRevision,
    pub plan_generation: PlanGeneration,
    pub layout_generation: LayoutGeneration,
    pub input_batch_hash: [u8; 32],
    pub before_epoch: InstanceEpoch,
    pub after_epoch: Option<InstanceEpoch>,
    pub state_hash: u64,
    pub touched_slots: u32,
    pub changed_slots: u32,
    pub executed_nodes: u32,
    pub effect_count: u32,
    pub outbox_effect_count: u32,
    pub transactional_effect_count: u32,
    pub effect_batch_hash: [u8; 32],
    pub effect_ids_hash: [u8; 32],
    pub idempotency_keys_hash: [u8; 32],
}

impl ResidentTurnReceiptV1 {
    pub const VERSION: u16 = 1;
}

impl Sealed for ResidentTurnReceiptV1 {}

impl AccountedRecord for ResidentTurnReceiptV1 {
    fn retained_bytes(&self) -> usize {
        core::mem::size_of::<Self>()
    }
}

pub type ResidentTurnRecord = OwnedTurnRecord<ResidentTurnReceiptV1>;
