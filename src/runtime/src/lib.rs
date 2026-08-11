#![cfg_attr(all(feature = "no_std", not(feature = "std")), no_std)]
#![forbid(unsafe_code)]

pub mod config;
pub mod effect;
mod extension;
pub mod id;
#[cfg(feature = "runtime")]
pub mod input;
#[cfg(feature = "runtime")]
mod ledger;
pub mod operation;
#[cfg(feature = "runtime")]
mod outbox;
#[cfg(all(feature = "runtime", feature = "runtime_bench_gate_b"))]
mod resident_recording;
mod resource;
mod resource_contract;
#[cfg(feature = "runtime")]
mod snapshot;

#[cfg(feature = "runtime")]
pub mod actor;
#[cfg(feature = "runtime")]
pub mod actor_behavior;
#[cfg(feature = "runtime")]
pub mod capability;
#[cfg(feature = "runtime")]
pub mod context;
#[cfg(feature = "runtime")]
mod context_events;
#[cfg(feature = "runtime")]
pub mod event;
pub mod host;
#[cfg(feature = "runtime")]
pub mod module;
#[cfg(feature = "runtime")]
pub mod resolver;
#[cfg(feature = "runtime")]
pub mod runtime;
#[cfg(feature = "runtime")]
pub mod scheduler;
#[cfg(feature = "runtime")]
pub mod service;
#[cfg(feature = "runtime")]
pub mod store;
#[cfg(feature = "runtime")]
pub mod transaction;
#[cfg(feature = "runtime")]
mod turn_record;
#[cfg(all(feature = "watcher", feature = "source"))]
mod workspace;

pub use self::config::*;
pub use self::effect::*;
pub use self::extension::{RuntimeExtensionPanicked, RuntimeStoreCommitIndeterminate};
pub use self::id::*;
#[cfg(feature = "runtime")]
pub use self::input::*;
pub use self::operation::*;
pub use self::resource::*;
pub use self::resource_contract::*;
#[cfg(feature = "runtime")]
pub use self::snapshot::*;

#[cfg(feature = "runtime")]
pub use self::actor::*;
#[cfg(feature = "runtime")]
pub use self::actor_behavior::*;
#[cfg(feature = "runtime")]
pub use self::capability::*;
#[cfg(feature = "runtime")]
pub use self::context::*;
#[cfg(feature = "runtime")]
pub use self::event::*;
pub use self::host::*;
#[cfg(feature = "runtime")]
pub use self::module::*;
#[cfg(feature = "runtime")]
pub use self::resolver::*;
#[cfg(feature = "runtime")]
pub use self::runtime::*;
#[cfg(feature = "runtime")]
pub use self::scheduler::*;
#[cfg(feature = "runtime")]
pub use self::service::*;
#[cfg(feature = "runtime")]
pub use self::store::*;
#[cfg(feature = "runtime")]
pub use self::transaction::*;
#[cfg(all(feature = "watcher", feature = "source"))]
pub use self::workspace::*;

/// Provisional Gate A recording primitives for controlled benchmarks only.
///
/// This facade is intentionally hidden behind a non-default probe feature. The
/// normal runtime API does not expose these types while the canonical receipt,
/// value, slot, schema, and epoch model is still under design.
#[doc(hidden)]
#[cfg(all(feature = "runtime", feature = "runtime_bench_probes"))]
pub mod __gate_a_recording {
    use mech_core::MResult;

    pub use crate::ledger::{
        LedgerPermit, OwnedTurnRecordQueue, RecordBufferPool, RecordEstimate, RetainedTurnLedger,
    };
    pub use crate::outbox::{
        OutboxDeliveryPolicy, OutboxEffectId, OutboxPermit, OwnedEffectIntent, RetainedEffectOutbox,
    };
    pub use crate::turn_record::{AccountedRecord, LedgerSequence, TurnId};

    use crate::{
        ledger::{PreparedLedgerAppend, TurnLedger},
        outbox::PreparedOutboxBatch,
    };

    /// A prepared retained-ledger append bound to its originating destination.
    #[must_use = "prepared records must be appended or dropped"]
    pub struct PreparedRetainedAppend<'a, R: AccountedRecord> {
        ledger: &'a mut RetainedTurnLedger<R>,
        prepared: PreparedLedgerAppend<R>,
    }

    impl<R: AccountedRecord> PreparedRetainedAppend<'_, R> {
        pub fn append(self) -> LedgerSequence {
            TurnLedger::append(self.ledger, self.prepared)
        }
    }

    pub fn reserve_retained<R: AccountedRecord>(
        ledger: &RetainedTurnLedger<R>,
        estimate: RecordEstimate,
    ) -> MResult<LedgerPermit> {
        TurnLedger::reserve(ledger, estimate)
    }

    pub fn prepare_retained<'a, R: AccountedRecord>(
        ledger: &'a mut RetainedTurnLedger<R>,
        permit: LedgerPermit,
        record: R,
    ) -> MResult<PreparedRetainedAppend<'a, R>> {
        let prepared = TurnLedger::prepare_append(ledger, permit, record)?;
        Ok(PreparedRetainedAppend { ledger, prepared })
    }

    /// A prepared queue append that publishes only to its originating queue.
    #[must_use = "prepared records must be appended or dropped"]
    pub struct PreparedQueueAppend<R> {
        queue: OwnedTurnRecordQueue<R>,
        prepared: PreparedLedgerAppend<R>,
    }

    impl<R: Send + 'static> PreparedQueueAppend<R> {
        pub fn append(self) -> LedgerSequence {
            self.queue.append(self.prepared)
        }
    }

    pub fn reserve_queue<R>(
        queue: &OwnedTurnRecordQueue<R>,
        estimate: RecordEstimate,
    ) -> MResult<LedgerPermit> {
        queue.reserve(estimate)
    }

    pub fn prepare_queue<R: AccountedRecord + Send + 'static>(
        queue: &OwnedTurnRecordQueue<R>,
        permit: LedgerPermit,
        record: R,
    ) -> MResult<PreparedQueueAppend<R>> {
        let prepared = queue.prepare_append(permit, record)?;
        Ok(PreparedQueueAppend {
            queue: queue.clone(),
            prepared,
        })
    }

    /// A prepared effect batch bound to its originating outbox.
    #[must_use = "prepared effect batches must be appended or dropped"]
    pub struct PreparedEffectBatch<'a, P> {
        outbox: &'a mut RetainedEffectOutbox<P>,
        prepared: PreparedOutboxBatch<P>,
    }

    impl<P> PreparedEffectBatch<'_, P> {
        pub fn append(self) {
            self.outbox.append(self.prepared);
        }
    }

    pub fn reserve_outbox<P>(
        outbox: &RetainedEffectOutbox<P>,
        estimate: RecordEstimate,
    ) -> MResult<OutboxPermit> {
        outbox.reserve(estimate)
    }

    pub fn prepare_outbox<'a, P: AccountedRecord + Send + 'static>(
        outbox: &'a mut RetainedEffectOutbox<P>,
        permit: OutboxPermit,
        effects: Vec<OwnedEffectIntent<P>>,
    ) -> MResult<PreparedEffectBatch<'a, P>> {
        let prepared = outbox.prepare_batch(permit, effects)?;
        Ok(PreparedEffectBatch { outbox, prepared })
    }
}

/// Fixed-receipt wrapper over the Gate A ledger for resident efficacy controls.
///
/// Normal runtime builds do not expose this provisional benchmark surface.
#[doc(hidden)]
#[cfg(all(feature = "runtime", feature = "runtime_bench_gate_b"))]
pub mod __resident_recording {
    use mech_core::MResult;

    pub use crate::ledger::{LedgerPermit, RecordEstimate, RetainedTurnLedger};
    pub use crate::resident_recording::{
        PreparedResidentCommit, ResidentRecordInspection, ResidentTurnRecorder,
    };
    pub use crate::turn_record::{
        AccountedRecord, GateBFixedReceipt, InputSequence, InputSequenceRange, LedgerSequence,
        OwnedTurnRecord, TurnFailurePhase, TurnId, TurnRecordHeader, TurnRecordStatus,
    };

    use crate::ledger::{PreparedLedgerAppend, TurnLedger};

    /// A Gate B prepared append bound to its originating retained ledger.
    #[must_use = "prepared records must be appended or dropped"]
    pub struct PreparedRetainedAppend<'a, R: AccountedRecord> {
        ledger: &'a mut RetainedTurnLedger<R>,
        prepared: PreparedLedgerAppend<R>,
    }

    impl<R: AccountedRecord> PreparedRetainedAppend<'_, R> {
        pub fn append(self) -> LedgerSequence {
            TurnLedger::append(self.ledger, self.prepared)
        }
    }

    pub fn reserve_retained<R: AccountedRecord>(
        ledger: &RetainedTurnLedger<R>,
        estimate: RecordEstimate,
    ) -> MResult<LedgerPermit> {
        TurnLedger::reserve(ledger, estimate)
    }

    pub fn prepare_retained<'a, R: AccountedRecord>(
        ledger: &'a mut RetainedTurnLedger<R>,
        permit: LedgerPermit,
        record: R,
    ) -> MResult<PreparedRetainedAppend<'a, R>> {
        let prepared = TurnLedger::prepare_append(ledger, permit, record)?;
        Ok(PreparedRetainedAppend { ledger, prepared })
    }
}

#[doc(hidden)]
#[cfg(feature = "native-link")]
pub mod __mech_native {
    use std::sync::Arc;

    use mech_core::MResult;

    use crate::{
        ActorMessageKindHostFunction, ActorMessagePayloadHostFunction, ActorStateGetHostFunction,
        ActorStateIdHostFunction, ActorStatePutHostFunction, RegisteredHostFunction,
        RuntimeBuilder,
    };

    pub fn install_actor_message_kind(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorMessageKindHostFunction::new(),
        )))
    }

    pub fn install_actor_message_payload(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorMessagePayloadHostFunction::new(),
        )))
    }

    pub fn install_actor_state_id(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::Pure(Arc::new(
            ActorStateIdHostFunction::new(),
        )))
    }

    pub fn install_actor_state_get(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
            ActorStateGetHostFunction::new(),
        )))
    }

    pub fn install_actor_state_put(builder: RuntimeBuilder) -> MResult<RuntimeBuilder> {
        builder.host_function(RegisteredHostFunction::RuntimeManaged(Arc::new(
            ActorStatePutHostFunction::new(),
        )))
    }
}
