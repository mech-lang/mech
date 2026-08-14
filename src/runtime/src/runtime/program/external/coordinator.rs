use mech_core::{
    ExternalInteraction, InstanceEpoch, MResult, MechError, MechErrorKind, ProgramRevision,
    ReactiveInstanceId, ResidentValueKind, TransactionalEffectProtocol, Value, ValueHash,
};
use mech_engine::{
    ProgramArtifact,
    resident::{
        CapturedValueInput, PreparedResidentTurn, ReactiveInstance, ResidentExecutionError,
        ResidentExternalPublicationAuthority, ResidentTurnSummary,
    },
};
use std::sync::Arc;

use crate::{
    PreparedRuntimeEffect, ResidentDurabilityPolicy, RuntimeCapabilityOperation,
    RuntimeEffectFailure, RuntimeEffectId, RuntimeEffectProtocol,
    RuntimeResidentResourceWriteRequest, RuntimeResourceReadRequest, RuntimeResourceRegistry,
    RuntimeResourceWriteIntent, TransactionId,
    ledger::{LedgerPermit, PreparedLedgerAppend, RecordEstimate, RetainedTurnLedger, TurnLedger},
    outbox::{
        OutboxDeliveryPolicy, OutboxEffectId, OutboxPermit, OwnedEffectIntent, PreparedOutboxBatch,
        RetainedEffectOutbox,
    },
    turn_record::{
        InputSequence, InputSequenceRange, LedgerSequence, OwnedTurnRecord, TurnFailurePhase,
        TurnFailureRecord, TurnId, TurnRecordHeader, TurnRecordStatus,
    },
};

use super::{
    BoundResidentEffect, BoundResidentExternalPlan, CapturedInputBatch, CapturedInputFact,
    ResidentExternalAuthority, ResidentOutboxPayload, ResidentTurnReceiptV1, ResidentTurnRecord,
    bind_external_requirements, bind_replay_requirements, captured_value_from_legacy,
    provider_value_from_canonical, resident_effect_id, resident_effect_ids_hash,
    resident_idempotency_key, resident_idempotency_keys_hash, resident_outbox_policy,
    resident_transaction_id,
};
use crate::runtime::effect_journal::{
    RuntimeEffectJournal, deliver_prepared_after_commit, validate_prepared_after_commit,
};

const MAX_FAILURE_MESSAGE_BYTES: usize = 256;
const MAX_FAILURE_KIND_BYTES: usize = 64;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ResidentExternalLimits {
    pub input_batches: usize,
    pub input_bytes: usize,
    pub receipts: usize,
    pub receipt_bytes: usize,
    pub outbox_effects: usize,
    pub outbox_bytes: usize,
}

impl Default for ResidentExternalLimits {
    fn default() -> Self {
        Self {
            input_batches: 1_024,
            input_bytes: 16 * 1_024 * 1_024,
            receipts: 1_024,
            receipt_bytes: 4 * 1_024 * 1_024,
            outbox_effects: 4_096,
            outbox_bytes: 32 * 1_024 * 1_024,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentExternalHealth {
    Healthy,
    PoisonedPrepublicationCleanup {
        turn: TurnId,
        failure_hash: [u8; 32],
    },
    PoisonedPostpublicationCommit {
        turn: TurnId,
        failure_hash: [u8; 32],
    },
    PoisonedRetainedEffectCleanup {
        turn: TurnId,
        failure_hash: [u8; 32],
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ResidentExternalTurnOutcome {
    Accepted {
        turn: TurnId,
        receipt_sequence: LedgerSequence,
        delivery_failures: Box<[RuntimeEffectFailure]>,
    },
    Rejected {
        turn: TurnId,
        receipt_sequence: LedgerSequence,
        phase: TurnFailurePhase,
    },
    PublishedIndeterminate {
        turn: TurnId,
        receipt_sequence: LedgerSequence,
        failures: Box<[RuntimeEffectFailure]>,
    },
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct ResidentExternalStructuralProbe {
    pub publication_store_count: usize,
    pub outbox_batch_append_count: usize,
    pub commit_runtime_call_count: usize,
    pub legacy_journal_capture_count: usize,
    pub runtime_execution_transaction_construction_count: usize,
    pub effects_delivered_before_publication: usize,
    pub effects_delivered_for_rejected_turns: usize,
    pub scene_effects_prepared: usize,
    pub scene_effects_delivered: usize,
}

enum RetainedDeliveryFailure {
    /// The provider did not return a delivery handle, so no delivery attempt
    /// occurred and every policy must retain the outbox entry.
    Preparation {
        failure: RuntimeEffectFailure,
        cleanup: Vec<RuntimeEffectFailure>,
    },
    /// A provider delivery handle was invoked and returned an error. Only this
    /// ambiguous attempt is terminal for AtMostOnce.
    Delivery(RuntimeEffectFailure),
}

#[derive(Debug)]
struct RuntimeResidentPublicationAuthority {
    _private: (),
}

// SAFETY: this type never escapes the resident external coordinator. The
// coordinator invokes authority-backed publication only after input capture,
// candidate execution/integrity, effect materialization, provider preparation,
// accepted-receipt and ordinary-outbox preparation, transactional preparation,
// and compensatable application all succeed. Its only post-publication work is
// infallible local append, transactional commit, and after-commit delivery.
#[allow(unsafe_code)]
unsafe impl ResidentExternalPublicationAuthority for RuntimeResidentPublicationAuthority {}

pub struct ResidentExternalCoordinator {
    instance: Option<ReactiveInstance>,
    publication_authority: RuntimeResidentPublicationAuthority,
    instance_id: ReactiveInstanceId,
    program_revision: ProgramRevision,
    plan_generation: mech_core::PlanGeneration,
    layout_generation: mech_core::LayoutGeneration,
    artifact: Arc<ProgramArtifact>,
    live: bool,
    bound: BoundResidentExternalPlan,
    durability: ResidentDurabilityPolicy,
    health: ResidentExternalHealth,
    published_state_hash: u64,
    next_turn: u64,
    next_input: u64,
    input_ledger: RetainedTurnLedger<CapturedInputBatch>,
    receipt_ledger: RetainedTurnLedger<ResidentTurnRecord>,
    outbox: RetainedEffectOutbox<ResidentOutboxPayload>,
    structural_probe: ResidentExternalStructuralProbe,
    published_through_turn: u64,
    last_rejected_turn: Option<TurnId>,
}

/// Capacity and identity reserved before a host packet leaves runtime ingress.
/// Once constructed, ordinary retained-capacity admission cannot fail midway
/// through consumption of that packet.
pub(crate) struct ResidentExternalTurnAdmission {
    input_permit: Option<LedgerPermit>,
    receipt_permit: LedgerPermit,
    outbox_permit: Option<OutboxPermit>,
    turn: TurnId,
    transaction: TransactionId,
    before_epoch: InstanceEpoch,
}

impl ResidentExternalCoordinator {
    pub fn new_live(
        instance: ReactiveInstance,
        artifact: Arc<ProgramArtifact>,
        providers: &RuntimeResourceRegistry,
        authority: &dyn ResidentExternalAuthority,
        durability: ResidentDurabilityPolicy,
        limits: ResidentExternalLimits,
    ) -> MResult<Self> {
        let bound = bind_external_requirements(&instance.plan, &artifact, providers, authority)?;
        Self::from_bound(instance, artifact, true, bound, durability, limits)
    }

    /// Constructs an offline replay coordinator without live providers or
    /// present-day external authorization.
    ///
    /// Replay remains bound to the exact activated artifact and validates all
    /// canonical captured facts, but it performs no provider reads, writes, or
    /// deliveries. Live turns and outbox retries are rejected on this mode.
    pub fn new_replay(
        instance: ReactiveInstance,
        artifact: Arc<ProgramArtifact>,
        durability: ResidentDurabilityPolicy,
        limits: ResidentExternalLimits,
    ) -> MResult<Self> {
        let bound = bind_replay_requirements(&instance.plan, &artifact)?;
        Self::from_bound(instance, artifact, false, bound, durability, limits)
    }

    fn from_bound(
        instance: ReactiveInstance,
        artifact: Arc<ProgramArtifact>,
        live: bool,
        bound: BoundResidentExternalPlan,
        durability: ResidentDurabilityPolicy,
        limits: ResidentExternalLimits,
    ) -> MResult<Self> {
        if !matches!(
            durability,
            ResidentDurabilityPolicy::Volatile | ResidentDurabilityPolicy::Retained
        ) {
            return Err(MechError::new(
                UnsupportedResidentDurability { durability },
                None,
            ));
        }
        if artifact.revision() != instance.plan.program_revision {
            return Err(MechError::new(
                ResidentExternalCoordinatorInvalid {
                    reason: "resident instance and external authority artifact revisions differ"
                        .to_owned(),
                },
                None,
            ));
        }
        if bound.observations().len() != instance.plan.inputs.len() {
            return Err(MechError::new(
                ResidentExternalCoordinatorInvalid {
                    reason: "every D3 resident turn input must be an admitted observation"
                        .to_owned(),
                },
                None,
            ));
        }
        let instance_id = instance.id;
        let published_state_hash = instance.published_state_hash();
        let program_revision = instance.plan.program_revision;
        let plan_generation = instance.plan.plan_generation;
        let layout_generation = instance.plan.layout_generation;
        let publication_authority = RuntimeResidentPublicationAuthority { _private: () };
        Ok(Self {
            instance: Some(instance),
            publication_authority,
            instance_id,
            program_revision,
            plan_generation,
            layout_generation,
            artifact,
            live,
            bound,
            durability,
            health: ResidentExternalHealth::Healthy,
            published_state_hash,
            next_turn: 1,
            next_input: 1,
            input_ledger: RetainedTurnLedger::new(limits.input_batches, limits.input_bytes)?,
            receipt_ledger: RetainedTurnLedger::new(limits.receipts, limits.receipt_bytes)?,
            outbox: RetainedEffectOutbox::new(limits.outbox_effects, limits.outbox_bytes)?,
            structural_probe: ResidentExternalStructuralProbe::default(),
            published_through_turn: 0,
            last_rejected_turn: None,
        })
    }

    pub const fn durability(&self) -> ResidentDurabilityPolicy {
        self.durability
    }

    pub fn health(&self) -> &ResidentExternalHealth {
        &self.health
    }

    pub fn instance(&self) -> &ReactiveInstance {
        self.instance
            .as_ref()
            .expect("resident instance is present")
    }

    #[cfg(feature = "runtime_bench_gate_d3")]
    #[doc(hidden)]
    pub fn set_next_epoch_for_benchmark(&mut self, next: u64) {
        self.instance
            .as_mut()
            .expect("resident instance is present")
            .set_next_epoch_for_test(next);
    }

    pub fn input_facts(&self) -> impl Iterator<Item = (LedgerSequence, &CapturedInputBatch)> {
        self.input_ledger.iter()
    }

    pub fn receipts(&self) -> impl Iterator<Item = (LedgerSequence, &ResidentTurnRecord)> {
        self.receipt_ledger.iter()
    }

    pub fn outbox(&self) -> impl Iterator<Item = &OwnedEffectIntent<ResidentOutboxPayload>> {
        self.outbox.iter()
    }

    pub fn pending_outbox_count(&self) -> usize {
        self.outbox.iter().count()
    }

    pub fn has_active_candidate(&self) -> bool {
        self.instance().has_active_candidate()
    }

    pub fn trigger_sources(&self) -> MResult<Box<[crate::RuntimeHostInputSource]>> {
        let mut sources = std::collections::BTreeSet::new();
        for observation in self.bound.observations() {
            let binding = observation.provider_binding.as_ref().ok_or_else(|| {
                invalid_value("live observation has no provider binding".to_owned())
            })?;
            let request = RuntimeResourceReadRequest {
                base_uri: observation.request.base_uri.clone(),
                path: observation.request.path.clone(),
                context_name: observation.request.context_name.clone(),
            };
            if binding.observation_requires_input_driver(&request)? {
                sources.insert(crate::RuntimeHostInputSource::new(
                    request.base_uri,
                    request.path,
                )?);
            }
        }
        Ok(sources.into_iter().collect::<Vec<_>>().into_boxed_slice())
    }

    pub const fn structural_probe(&self) -> ResidentExternalStructuralProbe {
        self.structural_probe
    }

    /// Releases the oldest retained input batch and transfers ownership to the
    /// caller for checkpointing, replication, or archival.
    pub fn release_next_input_batch(&mut self) -> Option<(LedgerSequence, CapturedInputBatch)> {
        self.input_ledger.pop_front()
    }

    /// Releases the oldest retained receipt and transfers ownership to the
    /// caller. Release order is the ledger's canonical FIFO order.
    pub fn release_next_receipt(&mut self) -> Option<(LedgerSequence, ResidentTurnRecord)> {
        self.receipt_ledger.pop_front()
    }

    pub fn execute_turn(&mut self) -> MResult<ResidentExternalTurnOutcome> {
        self.ensure_live_bindings()?;
        let admission = self.reserve_live_turn()?;
        self.execute_live_turn(None, admission, || Ok(()))
    }

    /// Executes one live turn while using owned ingress values for matching
    /// observations. Values absent from the update set are captured from their
    /// bound providers. This keeps host packets as the authority for the event
    /// they triggered instead of racing a separately updated provider snapshot.
    pub fn execute_turn_with_host_updates(
        &mut self,
        updates: &[crate::RuntimeHostInputUpdate],
    ) -> MResult<ResidentExternalTurnOutcome> {
        let admission = self.admit_host_turn(updates)?;
        self.execute_live_turn(Some(updates), admission, || Ok(()))
    }

    pub(crate) fn admit_host_turn(
        &mut self,
        updates: &[crate::RuntimeHostInputUpdate],
    ) -> MResult<ResidentExternalTurnAdmission> {
        self.ensure_live_bindings()?;
        self.validate_host_updates(updates)?;
        self.reserve_live_turn()
    }

    pub(crate) fn execute_admitted_host_turn<F>(
        &mut self,
        updates: &[crate::RuntimeHostInputUpdate],
        admission: ResidentExternalTurnAdmission,
        prepublication: F,
    ) -> MResult<ResidentExternalTurnOutcome>
    where
        F: FnOnce() -> MResult<()>,
    {
        self.execute_live_turn(Some(updates), admission, prepublication)
    }

    pub(crate) fn execute_admitted_turn<F>(
        &mut self,
        admission: ResidentExternalTurnAdmission,
        prepublication: F,
    ) -> MResult<ResidentExternalTurnOutcome>
    where
        F: FnOnce() -> MResult<()>,
    {
        self.execute_live_turn(None, admission, prepublication)
    }

    pub(crate) fn admit_turn(&mut self) -> MResult<ResidentExternalTurnAdmission> {
        self.ensure_live_bindings()?;
        self.reserve_live_turn()
    }

    fn validate_host_updates(&self, updates: &[crate::RuntimeHostInputUpdate]) -> MResult<()> {
        let mut sources = std::collections::BTreeSet::new();
        for update in updates {
            if !sources.insert(update.source.clone()) {
                return invalid_coordinator("host input repeats an activated source value");
            }
            let matching = self
                .bound
                .observations()
                .iter()
                .filter(|observation| {
                    update.source.base_uri() == observation.request.base_uri
                        && update.source.path() == observation.request.path
                })
                .collect::<Vec<_>>();
            if matching.is_empty() {
                return invalid_coordinator("host input does not match an activated observation");
            }
            let legacy = update.value.clone().into_mech_value()?;
            for observation in matching {
                captured_value_from_legacy(
                    &legacy,
                    observation.input.schema,
                    &observation.input.shape,
                    self.artifact.schemas(),
                )?;
            }
        }
        Ok(())
    }

    /// Replays one canonical recorded turn attempt.
    ///
    /// The receipt is the decision authority. Accepted records execute and
    /// must reproduce the complete receipt exactly; rejected records consume
    /// their optional full or partial input batch without executing or
    /// publishing. Input-only replay is intentionally unsupported because it
    /// cannot preserve rejection decisions.
    pub fn execute_replay_batch(
        &mut self,
        batch: Option<&CapturedInputBatch>,
        record: &ResidentTurnRecord,
    ) -> MResult<ResidentExternalTurnOutcome> {
        self.execute_replay_attempt(batch.cloned(), record.clone())
    }

    fn reserve_live_turn(&mut self) -> MResult<ResidentExternalTurnAdmission> {
        self.ensure_healthy()?;
        let before_epoch = self.instance().published_epoch();
        let input_permit = if self.bound.observations().is_empty() {
            None
        } else {
            Some(self.input_ledger.reserve(self.input_estimate())?)
        };
        let receipt_permit = self.receipt_ledger.reserve(self.receipt_estimate())?;
        let outbox_permit = self.reserve_outbox()?;
        let turn = self.allocate_turn()?;
        let transaction = resident_transaction_id(self.instance_id, turn);
        Ok(ResidentExternalTurnAdmission {
            input_permit,
            receipt_permit,
            outbox_permit,
            turn,
            transaction,
            before_epoch,
        })
    }

    fn execute_live_turn<F>(
        &mut self,
        host_updates: Option<&[crate::RuntimeHostInputUpdate]>,
        admission: ResidentExternalTurnAdmission,
        prepublication: F,
    ) -> MResult<ResidentExternalTurnOutcome>
    where
        F: FnOnce() -> MResult<()>,
    {
        let ResidentExternalTurnAdmission {
            input_permit,
            receipt_permit,
            outbox_permit,
            turn,
            transaction,
            before_epoch,
        } = admission;

        let batch = if let Some(input_permit) = input_permit {
            let batch = match self.capture_with_providers(host_updates) {
                Ok(batch) => batch,
                Err(failure) => {
                    let evidence = if let Some(prefix) = failure.captured_prefix {
                        if let Err(error) = self.append_input_batch(input_permit, prefix.clone()) {
                            drop(outbox_permit);
                            return self.append_rejected(
                                receipt_permit,
                                turn,
                                transaction,
                                RejectedTurnEvidence::default(),
                                before_epoch,
                                TurnFailurePhase::Recording,
                                error,
                            );
                        }
                        self.advance_input_identity(&prefix)?;
                        RejectedTurnEvidence::from_input(&prefix)
                    } else {
                        drop(input_permit);
                        RejectedTurnEvidence::default()
                    };
                    drop(outbox_permit);
                    return self.append_rejected(
                        receipt_permit,
                        turn,
                        transaction,
                        evidence,
                        before_epoch,
                        TurnFailurePhase::InputInstallation,
                        failure.error,
                    );
                }
            };
            if let Err(error) = self.append_input_batch(input_permit, batch.clone()) {
                drop(outbox_permit);
                return self.append_rejected(
                    receipt_permit,
                    turn,
                    transaction,
                    RejectedTurnEvidence::default(),
                    before_epoch,
                    TurnFailurePhase::Recording,
                    error,
                );
            }
            self.advance_input_identity(&batch)?;
            Some(batch)
        } else {
            if host_updates.is_some_and(|updates| !updates.is_empty()) {
                drop(outbox_permit);
                return self.append_rejected(
                    receipt_permit,
                    turn,
                    transaction,
                    RejectedTurnEvidence::default(),
                    before_epoch,
                    TurnFailurePhase::InputInstallation,
                    invalid_value("input-free resident turn received host updates".to_owned()),
                );
            }
            None
        };
        let input_evidence = batch
            .as_ref()
            .map(RejectedTurnEvidence::from_input)
            .unwrap_or_default();

        let inputs = batch
            .iter()
            .flat_map(|batch| batch.facts.iter())
            .map(|fact| CapturedValueInput {
                slot: self
                    .bound
                    .observations()
                    .iter()
                    .find(|observation| {
                        observation.node == fact.node
                            && observation.input.artifact_slot == fact.slot
                    })
                    .expect("validated batch observation")
                    .input
                    .slot,
                value: &fact.value,
            })
            .collect::<Vec<_>>();
        let mut instance = self.instance.take().expect("resident instance is present");
        let result = (|| {
            let prepared_turn = match instance.prepare_turn_values(&inputs) {
                Ok(prepared) => prepared,
                Err(error) => {
                    drop(outbox_permit);
                    let phase = if matches!(error, ResidentExecutionError::Integrity { .. }) {
                        TurnFailurePhase::Integrity
                    } else {
                        TurnFailurePhase::Execution
                    };
                    return self.append_rejected(
                        receipt_permit,
                        turn,
                        transaction,
                        input_evidence,
                        before_epoch,
                        phase,
                        resident_execution_error(error),
                    );
                }
            };

            self.prepare_and_publish(
                prepared_turn,
                turn,
                transaction,
                input_evidence,
                receipt_permit,
                outbox_permit,
                prepublication,
            )
        })();
        self.instance = Some(instance);
        result
    }

    fn execute_replay_attempt(
        &mut self,
        batch: Option<CapturedInputBatch>,
        record: ResidentTurnRecord,
    ) -> MResult<ResidentExternalTurnOutcome> {
        self.ensure_healthy()?;
        if self.live {
            return invalid_coordinator("recorded replay requires an offline coordinator");
        }
        let batch = batch
            .map(|batch| self.validate_replay_batch(batch))
            .transpose()?;
        self.validate_replay_record(batch.as_ref(), &record)?;

        let input_permit = batch
            .as_ref()
            .map(|_| self.input_ledger.reserve(self.input_estimate()))
            .transpose()?;
        let receipt_permit = self.receipt_ledger.reserve(self.receipt_estimate())?;
        let turn = TurnId::new(self.next_turn).ok_or_else(sequence_exhausted)?;
        let next_turn = self
            .next_turn
            .checked_add(1)
            .ok_or_else(sequence_exhausted)?;
        let next_input = batch
            .as_ref()
            .map(|batch| {
                batch
                    .range
                    .last()
                    .get()
                    .checked_add(1)
                    .ok_or_else(sequence_exhausted)
            })
            .transpose()?;
        let transaction = resident_transaction_id(self.instance_id, turn);
        let prepared_input = input_permit
            .zip(batch.as_ref())
            .map(|(permit, batch)| self.input_ledger.prepare_append(permit, batch.clone()))
            .transpose()?;
        let prepared_receipt = self
            .receipt_ledger
            .prepare_append(receipt_permit, record.clone())?;

        match record.header.status {
            TurnRecordStatus::Rejected => {
                let phase = record
                    .header
                    .failure
                    .as_ref()
                    .expect("validated rejected replay receipt")
                    .phase;
                if let Some(prepared) = prepared_input {
                    self.append_prepared_input(prepared);
                }
                if let Some(next_input) = next_input {
                    self.next_input = next_input;
                }
                self.next_turn = next_turn;
                let receipt_sequence = self.append_receipt(prepared_receipt);
                self.last_rejected_turn = Some(turn);
                Ok(ResidentExternalTurnOutcome::Rejected {
                    turn,
                    receipt_sequence,
                    phase,
                })
            }
            TurnRecordStatus::Accepted => self.execute_replay_accepted(
                batch.as_ref(),
                record,
                prepared_input,
                prepared_receipt,
                next_input,
                next_turn,
                turn,
                transaction,
            ),
            TurnRecordStatus::Staged => {
                invalid_coordinator("staged resident turns are not replay decisions")
            }
        }
    }

    fn execute_replay_accepted(
        &mut self,
        batch: Option<&CapturedInputBatch>,
        expected: ResidentTurnRecord,
        prepared_input: Option<PreparedLedgerAppend<CapturedInputBatch>>,
        prepared_receipt: PreparedLedgerAppend<ResidentTurnRecord>,
        next_input: Option<u64>,
        next_turn: u64,
        turn: TurnId,
        transaction: TransactionId,
    ) -> MResult<ResidentExternalTurnOutcome> {
        let inputs = batch
            .iter()
            .flat_map(|batch| batch.facts.iter())
            .map(|fact| CapturedValueInput {
                slot: self
                    .bound
                    .observations()
                    .iter()
                    .find(|observation| {
                        observation.node == fact.node
                            && observation.input.artifact_slot == fact.slot
                    })
                    .expect("validated replay observation")
                    .input
                    .slot,
                value: &fact.value,
            })
            .collect::<Vec<_>>();
        let mut instance = self.instance.take().expect("resident instance is present");
        let result = (|| {
            let prepared_turn = instance
                .prepare_turn_values(&inputs)
                .map_err(resident_execution_error)?;
            let materialized = match materialize_effects(
                &prepared_turn,
                &self.artifact,
                self.bound.effects(),
                self.instance_id,
                turn,
            ) {
                Ok(materialized) => materialized,
                Err(error) => {
                    prepared_turn.abort();
                    return Err(error);
                }
            };
            let summary = prepared_turn.summary();
            let input_range = batch.map(|batch| batch.range);
            let input_hash = batch.map_or([0; 32], |batch| batch.batch_hash);
            let reproduced = self.accepted_record(
                turn,
                transaction,
                input_range,
                input_hash,
                effect_batch_hash(&materialized),
                summary,
                &materialized,
            )?;
            if reproduced != expected {
                prepared_turn.abort();
                return invalid_coordinator(
                    "recorded accepted receipt does not match replayed resident turn",
                );
            }
            self.observe_prepared_turn(prepared_turn.structural_probe());
            self.publish_prepared(turn, prepared_turn)?;
            if let Some(prepared) = prepared_input {
                self.append_prepared_input(prepared);
            }
            if let Some(next_input) = next_input {
                self.next_input = next_input;
            }
            self.next_turn = next_turn;
            let receipt_sequence = self.append_receipt(prepared_receipt);
            Ok(ResidentExternalTurnOutcome::Accepted {
                turn,
                receipt_sequence,
                delivery_failures: Box::new([]),
            })
        })();
        self.instance = Some(instance);
        result
    }

    fn validate_replay_record(
        &self,
        batch: Option<&CapturedInputBatch>,
        record: &ResidentTurnRecord,
    ) -> MResult<()> {
        record.validate()?;
        let expected_turn = TurnId::new(self.next_turn).ok_or_else(sequence_exhausted)?;
        let expected_transaction = resident_transaction_id(self.instance_id, expected_turn);
        if record.header.turn_id != expected_turn
            || record.header.transaction_id != expected_transaction
            || record.body.version != ResidentTurnReceiptV1::VERSION
            || record.body.instance != self.instance_id
            || record.body.program_revision != self.program_revision
            || record.body.plan_generation != self.plan_generation
            || record.body.layout_generation != self.layout_generation
            || record.body.before_epoch != self.instance().published_epoch()
        {
            return invalid_coordinator(
                "recorded replay receipt does not match the next activated turn",
            );
        }
        match batch {
            Some(batch)
                if record.header.input_range == Some(batch.range)
                    && record.body.input_batch_hash == batch.batch_hash => {}
            None if record.header.input_range.is_none()
                && record.body.input_batch_hash == [0; 32] => {}
            _ => {
                return invalid_coordinator(
                    "recorded replay receipt does not identify its captured input",
                );
            }
        }
        match record.header.status {
            TurnRecordStatus::Accepted => {
                let complete_inputs = if self.bound.observations().is_empty() {
                    batch.is_none()
                } else {
                    batch.is_some_and(|batch| batch.facts.len() == self.bound.observations().len())
                };
                if !complete_inputs || record.body.after_epoch.is_none() {
                    return invalid_coordinator(
                        "accepted replay requires the activated complete input boundary",
                    );
                }
            }
            TurnRecordStatus::Rejected => {
                if record.body.after_epoch.is_some()
                    || record.body.state_hash != self.published_state_hash
                {
                    return invalid_coordinator(
                        "rejected replay receipt must preserve the published state",
                    );
                }
            }
            TurnRecordStatus::Staged => {
                return invalid_coordinator("staged resident turns are not replay decisions");
            }
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn prepare_and_publish<F>(
        &mut self,
        prepared_turn: PreparedResidentTurn<'_>,
        turn: TurnId,
        transaction: TransactionId,
        input_evidence: RejectedTurnEvidence,
        receipt_permit: LedgerPermit,
        outbox_permit: Option<OutboxPermit>,
        prepublication: F,
    ) -> MResult<ResidentExternalTurnOutcome>
    where
        F: FnOnce() -> MResult<()>,
    {
        let before_epoch = prepared_turn.summary().before_epoch;
        let materialized = match materialize_effects(
            &prepared_turn,
            &self.artifact,
            self.bound.effects(),
            self.instance_id,
            turn,
        ) {
            Ok(materialized) => materialized,
            Err(error) => {
                prepared_turn.abort();
                drop(outbox_permit);
                return self.append_rejected(
                    receipt_permit,
                    turn,
                    transaction,
                    input_evidence,
                    before_epoch,
                    TurnFailurePhase::EffectMaterialization,
                    error,
                );
            }
        };
        let effect_batch_hash = effect_batch_hash(&materialized);
        let rejected_evidence = input_evidence.with_effects(&materialized)?;
        self.observe_prepared_turn(prepared_turn.structural_probe());
        let summary = prepared_turn.summary();
        let receipt = self.accepted_record(
            turn,
            transaction,
            input_evidence.input_range,
            input_evidence.input_batch_hash,
            effect_batch_hash,
            summary,
            &materialized,
        )?;
        let mut journal = RuntimeEffectJournal::new();
        if let Err(failure) = self.prepare_provider_effects(&materialized, &mut journal) {
            prepared_turn.abort();
            drop(outbox_permit);
            return self.reject_with_cleanup(
                receipt_permit,
                turn,
                transaction,
                rejected_evidence,
                before_epoch,
                TurnFailurePhase::ExternalPrepare,
                failure.error,
                failure.cleanup,
            );
        }

        let prepared_outbox = match self.prepare_outbox(outbox_permit, &materialized) {
            Ok(prepared) => prepared,
            Err(error) => {
                let cleanup = journal.abort_all();
                prepared_turn.abort();
                return self.reject_with_cleanup(
                    receipt_permit,
                    turn,
                    transaction,
                    rejected_evidence,
                    before_epoch,
                    TurnFailurePhase::ExternalPrepare,
                    error,
                    cleanup,
                );
            }
        };

        if let Err(step) = journal.prepare_transactional() {
            let cleanup = journal.abort_all();
            prepared_turn.abort();
            drop(prepared_outbox);
            return self.reject_with_cleanup(
                receipt_permit,
                turn,
                transaction,
                rejected_evidence,
                before_epoch,
                TurnFailurePhase::ExternalPrepare,
                step.error,
                cleanup,
            );
        }
        if let Err(step) = journal.apply_compensatable() {
            let mut cleanup = journal.compensate_applied_reverse();
            cleanup.extend(journal.abort_all());
            prepared_turn.abort();
            drop(prepared_outbox);
            return self.reject_with_cleanup(
                receipt_permit,
                turn,
                transaction,
                rejected_evidence,
                before_epoch,
                TurnFailurePhase::ExternalApply,
                step.error,
                cleanup,
            );
        }

        if let Err(error) = prepublication() {
            let mut cleanup = journal.compensate_applied_reverse();
            cleanup.extend(journal.abort_all());
            prepared_turn.abort();
            drop(prepared_outbox);
            return self.reject_with_cleanup(
                receipt_permit,
                turn,
                transaction,
                rejected_evidence,
                before_epoch,
                TurnFailurePhase::Execution,
                error,
                cleanup,
            );
        }

        let prepared_receipt = match self.receipt_ledger.prepare_append(receipt_permit, receipt) {
            Ok(prepared) => prepared,
            Err(error) => {
                let mut cleanup = journal.compensate_applied_reverse();
                cleanup.extend(journal.abort_all());
                prepared_turn.abort();
                drop(prepared_outbox);
                if cleanup.is_empty() {
                    return Err(error);
                }
                self.health = ResidentExternalHealth::PoisonedPrepublicationCleanup {
                    turn,
                    failure_hash: failures_hash(&cleanup),
                };
                return Err(MechError::new(
                    ResidentExternalCleanupFailed {
                        turn,
                        failures: cleanup,
                    },
                    None,
                ));
            }
        };
        self.publish_prepared(turn, prepared_turn)?;
        let receipt_sequence = self.append_receipt(prepared_receipt);
        if let Some(prepared) = prepared_outbox {
            self.outbox.append(prepared);
            self.structural_probe.outbox_batch_append_count = self
                .structural_probe
                .outbox_batch_append_count
                .saturating_add(1);
        }

        let commit = journal.commit_transactional();
        if !commit.failures.is_empty() {
            let failures = commit
                .failures
                .into_iter()
                .map(|failure| failure.failure)
                .collect::<Vec<_>>();
            self.health = ResidentExternalHealth::PoisonedPostpublicationCommit {
                turn,
                failure_hash: failures_hash(&failures),
            };
            return Ok(ResidentExternalTurnOutcome::PublishedIndeterminate {
                turn,
                receipt_sequence,
                failures: failures.into_boxed_slice(),
            });
        }

        let delivery_failures = self.deliver_fifo(Some((transaction, &mut journal)))?;
        Ok(ResidentExternalTurnOutcome::Accepted {
            turn,
            receipt_sequence,
            delivery_failures: delivery_failures.into_boxed_slice(),
        })
    }

    fn capture_with_providers(
        &self,
        host_updates: Option<&[crate::RuntimeHostInputUpdate]>,
    ) -> Result<CapturedInputBatch, CaptureFailure> {
        let mut facts = Vec::with_capacity(self.bound.observations().len());
        for (ordinal, observation) in self.bound.observations().iter().enumerate() {
            let fact = (|| -> MResult<CapturedInputFact> {
                let sequence_value = self
                    .next_input
                    .checked_add(ordinal as u64)
                    .ok_or_else(sequence_exhausted)?;
                let sequence = InputSequence::new(sequence_value).ok_or_else(sequence_exhausted)?;
                let packet_value = host_updates.and_then(|updates| {
                    updates.iter().rev().find(|update| {
                        update.source.base_uri() == observation.request.base_uri
                            && update.source.path() == observation.request.path
                    })
                });
                let legacy = if let Some(update) = packet_value {
                    update.value.clone().into_mech_value()?
                } else {
                    let provider_binding =
                        observation.provider_binding.as_ref().ok_or_else(|| {
                            invalid_value("live observation has no provider binding".to_owned())
                        })?;
                    provider_binding.read(RuntimeResourceReadRequest {
                        base_uri: observation.request.base_uri.clone(),
                        path: observation.request.path.clone(),
                        context_name: observation.request.context_name.clone(),
                    })?
                };
                let value = captured_value_from_legacy(
                    &legacy,
                    observation.input.schema,
                    &observation.input.shape,
                    self.artifact.schemas(),
                )?;
                CapturedInputFact::new(
                    sequence,
                    observation.requirement,
                    observation.node,
                    observation.input.artifact_slot,
                    observation.input.schema_key,
                    observation.input.shape.clone(),
                    value,
                    self.artifact.schemas(),
                )
            })();
            match fact {
                Ok(fact) => facts.push(fact),
                Err(error) => return Err(CaptureFailure::with_prefix(facts, error)),
            }
        }
        CapturedInputBatch::new(facts).map_err(CaptureFailure::without_prefix)
    }

    fn validate_replay_batch(&self, batch: CapturedInputBatch) -> MResult<CapturedInputBatch> {
        let facts = batch
            .facts
            .iter()
            .map(|fact| {
                CapturedInputFact::new(
                    fact.sequence,
                    fact.requirement,
                    fact.node,
                    fact.slot,
                    fact.schema_key,
                    fact.shape.clone(),
                    fact.value.clone(),
                    self.artifact.schemas(),
                )
            })
            .collect::<MResult<Vec<_>>>()?;
        let canonical = CapturedInputBatch::new(facts)?;
        if batch.facts.len() > self.bound.observations().len()
            || canonical.range.first().get() != self.next_input
            || batch.range != canonical.range
            || batch.batch_hash != canonical.batch_hash
        {
            return invalid_coordinator(
                "replay input identities do not match the next admitted batch",
            );
        }
        for (fact, observation) in canonical.facts.iter().zip(self.bound.observations()) {
            if fact.requirement != observation.requirement
                || fact.node != observation.node
                || fact.slot != observation.input.artifact_slot
                || fact.schema_key != observation.input.schema_key
                || fact.shape != observation.input.shape
                || fact.value.schema() != observation.input.schema
                || fact.value.schema_key() != observation.input.schema_key
                || fact.value.shape() != &observation.input.shape
                || fact.value.value_hash(self.artifact.schemas()).ok() != Some(fact.payload_hash)
            {
                return invalid_coordinator(
                    "replay batch differs from the activated observation plan",
                );
            }
        }
        Ok(canonical)
    }

    fn prepare_provider_effects(
        &mut self,
        effects: &[MaterializedEffect],
        journal: &mut RuntimeEffectJournal,
    ) -> Result<(), ProviderPreparationFailure> {
        for effect in effects {
            let is_scene = effect
                .bound
                .provider_binding
                .as_ref()
                .is_some_and(|binding| binding.scheme() == "scene");
            let result = (|| -> MResult<PreparedRuntimeEffect> {
                let intent = request_write_intent(&effect.bound)?;
                let value = provider_value_from_canonical(&effect.value, self.artifact.schemas())?;
                let provider_binding = effect.bound.provider_binding.as_ref().ok_or_else(|| {
                    invalid_value("live effect has no provider binding".to_owned())
                })?;
                provider_binding.prepare_write(RuntimeResidentResourceWriteRequest {
                    base_uri: effect.bound.request.base_uri.clone(),
                    path: effect.bound.request.path.clone(),
                    context_name: effect.bound.request.context_name.clone(),
                    operation: RuntimeCapabilityOperation::from_name(
                        effect.bound.request.operation.clone(),
                    )?,
                    value,
                    intent,
                    effect_id: effect.id,
                    idempotency_key: effect.idempotency_key.clone(),
                })
            })();
            let result = match result {
                Ok(result) => result,
                Err(error) => return Err(ProviderPreparationFailure::new(error, journal)),
            };
            let actual = result.protocol();
            if let Err(failure) = journal.stage_exact(effect.id, result) {
                let mut cleanup = failure.cleanup;
                cleanup.extend(journal.abort_all());
                return Err(ProviderPreparationFailure {
                    error: failure.error,
                    cleanup,
                });
            }
            if !prepared_protocol_matches(&effect.bound.interaction, actual) {
                return Err(ProviderPreparationFailure::new(
                    MechError::new(
                        ResidentPreparedEffectProtocolMismatch {
                            effect: effect.id,
                            actual,
                        },
                        None,
                    ),
                    journal,
                ));
            }
            if is_scene {
                self.structural_probe.scene_effects_prepared = self
                    .structural_probe
                    .scene_effects_prepared
                    .saturating_add(1);
            }
        }
        Ok(())
    }

    fn prepare_outbox(
        &mut self,
        permit: Option<OutboxPermit>,
        effects: &[MaterializedEffect],
    ) -> MResult<Option<PreparedOutboxBatch<ResidentOutboxPayload>>> {
        let mut ordinary = Vec::with_capacity(self.bound.ordinary_effect_count());
        for effect in effects {
            if let Some(delivery) = resident_outbox_policy(&effect.bound.interaction)? {
                ordinary.push(OwnedEffectIntent {
                    id: OutboxEffectId {
                        turn_id: effect.turn,
                        ordinal: effect.bound.ordinal,
                    },
                    operation: effect.bound.request.operation.clone(),
                    target: format!(
                        "{}#{}",
                        effect.bound.request.base_uri, effect.bound.request.path
                    ),
                    payload: ResidentOutboxPayload::new(
                        effect.bound.requirement,
                        effect.value.clone(),
                        effect.payload_hash,
                        effect.retained_bytes,
                    ),
                    idempotency_key: effect.idempotency_key.clone(),
                    delivery,
                });
            }
        }
        if ordinary.is_empty() {
            return Ok(None);
        }
        let permit = permit.ok_or_else(|| {
            MechError::new(
                ResidentExternalCoordinatorInvalid {
                    reason: "ordinary effects were not pre-reserved in the outbox".to_owned(),
                },
                None,
            )
        })?;
        self.outbox.prepare_batch(permit, ordinary).map(Some)
    }

    fn deliver_fifo(
        &mut self,
        mut current: Option<(TransactionId, &mut RuntimeEffectJournal)>,
    ) -> MResult<Vec<RuntimeEffectFailure>> {
        let mut failures = Vec::new();
        loop {
            let Some(front) = self.outbox.front() else {
                break;
            };
            let front_turn = front.id.turn_id;
            let effect_ordinal = front.id.ordinal;
            let delivery = front.delivery;
            let is_scene = self.bound.effects().iter().any(|effect| {
                effect.ordinal == effect_ordinal
                    && effect
                        .provider_binding
                        .as_ref()
                        .is_some_and(|binding| binding.scheme() == "scene")
            });
            if front_turn.get() > self.published_through_turn {
                self.structural_probe.effects_delivered_before_publication = self
                    .structural_probe
                    .effects_delivered_before_publication
                    .saturating_add(1);
            }
            if self.last_rejected_turn == Some(front_turn) {
                self.structural_probe.effects_delivered_for_rejected_turns = self
                    .structural_probe
                    .effects_delivered_for_rejected_turns
                    .saturating_add(1);
            }
            let id = resident_effect_id(self.instance_id, front_turn, front.id.ordinal);
            let result = match current.as_mut() {
                Some((transaction, journal)) if *transaction == id.transaction => journal
                    .validate_after_commit_exact(id)
                    .map_err(|failure| RetainedDeliveryFailure::Preparation {
                        failure: failure.failure,
                        cleanup: failure.cleanup,
                    })
                    .and_then(|()| {
                        journal
                            .deliver_after_commit_exact(id)
                            .map_err(RetainedDeliveryFailure::Delivery)
                    }),
                _ => self.prepare_retained_delivery(id),
            };
            match result {
                Ok(()) => {
                    if is_scene {
                        self.structural_probe.scene_effects_delivered = self
                            .structural_probe
                            .scene_effects_delivered
                            .saturating_add(1);
                    }
                    self.outbox.acknowledge_front();
                }
                Err(RetainedDeliveryFailure::Preparation { failure, cleanup }) => {
                    failures.push(failure);
                    if !cleanup.is_empty() {
                        self.health = ResidentExternalHealth::PoisonedRetainedEffectCleanup {
                            turn: front_turn,
                            failure_hash: failures_hash(&cleanup),
                        };
                        return Err(MechError::new(
                            ResidentExternalCleanupFailed {
                                turn: front_turn,
                                failures: cleanup,
                            },
                            None,
                        ));
                    }
                    break;
                }
                Err(RetainedDeliveryFailure::Delivery(failure)) => {
                    failures.push(failure);
                    match delivery {
                        OutboxDeliveryPolicy::AtMostOnce => {
                            self.outbox.acknowledge_front();
                        }
                        OutboxDeliveryPolicy::AtLeastOnce
                        | OutboxDeliveryPolicy::IdempotentRetry
                        | OutboxDeliveryPolicy::ProviderTransactional => {
                            if let Some(front) = self.outbox.front_mut() {
                                front.payload.attempts = front.payload.attempts.saturating_add(1);
                            }
                            break;
                        }
                    }
                }
            }
        }
        Ok(failures)
    }

    fn prepare_retained_delivery(
        &mut self,
        id: RuntimeEffectId,
    ) -> Result<(), RetainedDeliveryFailure> {
        let front = self.outbox.front().expect("retained outbox front");
        let bound = self
            .bound
            .effects()
            .iter()
            .find(|effect| effect.ordinal == front.id.ordinal)
            .expect("bound resident outbox effect");
        let prepared = (|| -> MResult<PreparedRuntimeEffect> {
            let intent = request_write_intent(bound)?;
            let provider_binding = bound.provider_binding.as_ref().ok_or_else(|| {
                invalid_value("live retained effect has no provider binding".to_owned())
            })?;
            provider_binding.prepare_write(RuntimeResidentResourceWriteRequest {
                base_uri: bound.request.base_uri.clone(),
                path: bound.request.path.clone(),
                context_name: bound.request.context_name.clone(),
                operation: RuntimeCapabilityOperation::from_name(bound.request.operation.clone())?,
                value: provider_value_from_canonical(
                    &front.payload.value,
                    self.artifact.schemas(),
                )?,
                intent,
                effect_id: id,
                idempotency_key: front.idempotency_key.clone(),
            })
        })();
        let mut prepared = prepared.map_err(|error| RetainedDeliveryFailure::Preparation {
            failure: RuntimeEffectFailure {
                effect_id: id,
                phase: crate::RuntimeEffectFailurePhase::Prepare,
                message: format!("{error:?}"),
            },
            cleanup: Vec::new(),
        })?;
        if bound
            .provider_binding
            .as_ref()
            .is_some_and(|binding| binding.scheme() == "scene")
        {
            self.structural_probe.scene_effects_prepared = self
                .structural_probe
                .scene_effects_prepared
                .saturating_add(1);
        }
        validate_prepared_after_commit(id, &mut prepared).map_err(|failure| {
            RetainedDeliveryFailure::Preparation {
                failure: failure.failure,
                cleanup: failure.cleanup,
            }
        })?;
        deliver_prepared_after_commit(id, &mut prepared).map_err(RetainedDeliveryFailure::Delivery)
    }

    pub fn retry_outbox(&mut self) -> MResult<Box<[RuntimeEffectFailure]>> {
        self.ensure_healthy()?;
        self.ensure_live_bindings()?;
        Ok(self.deliver_fifo(None)?.into_boxed_slice())
    }

    fn reserve_outbox(&self) -> MResult<Option<OutboxPermit>> {
        let count = self.bound.ordinary_effect_count();
        if count == 0 {
            return Ok(None);
        }
        self.outbox
            .reserve(RecordEstimate {
                records: count,
                bytes: self.outbox_estimate_bytes(),
            })
            .map(Some)
    }

    fn input_estimate(&self) -> RecordEstimate {
        let bytes = core::mem::size_of::<CapturedInputBatch>()
            + self
                .bound
                .observations()
                .iter()
                .map(|observation| {
                    core::mem::size_of::<CapturedInputFact>()
                        + observation.input.region.len
                            * resident_scalar_bytes(observation.input.region.kind)
                        + observation.input.shape.parameter_values().len() * 8
                })
                .sum::<usize>();
        RecordEstimate { records: 1, bytes }
    }

    fn receipt_estimate(&self) -> RecordEstimate {
        RecordEstimate {
            records: 1,
            bytes: core::mem::size_of::<ResidentTurnReceiptV1>()
                + MAX_FAILURE_KIND_BYTES
                + MAX_FAILURE_MESSAGE_BYTES,
        }
    }

    fn outbox_estimate_bytes(&self) -> usize {
        self.bound
            .effects()
            .iter()
            .filter(|effect| matches!(effect.interaction, ExternalInteraction::Effect(_)))
            .map(|effect| {
                let payload = self
                    .instance()
                    .plan
                    .steps
                    .iter()
                    .find_map(|step| match step {
                        mech_engine::resident::ActivatedTurnStep::External(external)
                            if external.effect_ordinal == effect.ordinal =>
                        {
                            Some(external)
                        }
                        _ => None,
                    })
                    .expect("bound external step");
                core::mem::size_of::<OwnedEffectIntent<ResidentOutboxPayload>>()
                    + payload.payload.region().len
                        * resident_scalar_bytes(payload.payload.region().kind)
                    + effect.request.base_uri.len()
                    + effect.request.path.len()
                    + effect.request.operation.len()
                    + 128
            })
            .sum()
    }

    fn accepted_record(
        &self,
        turn: TurnId,
        transaction: TransactionId,
        input_range: Option<InputSequenceRange>,
        input_batch_hash: [u8; 32],
        effect_batch_hash: [u8; 32],
        summary: ResidentTurnSummary,
        effects: &[MaterializedEffect],
    ) -> MResult<ResidentTurnRecord> {
        let effect_count = effects.len();
        let outbox_effect_count = effects
            .iter()
            .filter(|effect| matches!(effect.bound.interaction, ExternalInteraction::Effect(_)))
            .count();
        let transactional_effect_count = effects
            .iter()
            .filter(|effect| {
                matches!(
                    effect.bound.interaction,
                    ExternalInteraction::TransactionalExternal(_)
                )
            })
            .count();
        Ok(OwnedTurnRecord {
            header: TurnRecordHeader {
                turn_id: turn,
                transaction_id: transaction,
                input_range,
                status: TurnRecordStatus::Accepted,
                failure: None,
            },
            body: ResidentTurnReceiptV1 {
                version: ResidentTurnReceiptV1::VERSION,
                instance: summary.instance,
                program_revision: summary.program_revision,
                plan_generation: self.plan_generation,
                layout_generation: self.layout_generation,
                input_batch_hash,
                before_epoch: summary.before_epoch,
                after_epoch: Some(summary.after_epoch),
                state_hash: summary.state_hash,
                touched_slots: u32::from(summary.touched_slots),
                changed_slots: u32::from(summary.changed_slots),
                executed_nodes: u32::from(summary.dirty_nodes),
                effect_count: u32::try_from(effect_count).map_err(|_| count_overflow())?,
                outbox_effect_count: u32::try_from(outbox_effect_count)
                    .map_err(|_| count_overflow())?,
                transactional_effect_count: u32::try_from(transactional_effect_count)
                    .map_err(|_| count_overflow())?,
                effect_batch_hash,
                effect_ids_hash: resident_effect_ids_hash(effects.iter().map(|effect| effect.id)),
                idempotency_keys_hash: resident_idempotency_keys_hash(
                    effects.iter().map(|effect| effect.idempotency_key.as_str()),
                ),
            },
        })
    }

    #[cfg(test)]
    pub(super) fn accepted_record_without_materialized_effects_for_test(
        &self,
        summary: ResidentTurnSummary,
    ) -> MResult<ResidentTurnRecord> {
        self.accepted_record(
            TurnId::new(1).expect("non-zero test turn"),
            TransactionId(1),
            None,
            [0; 32],
            [0; 32],
            summary,
            &[],
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn append_rejected(
        &mut self,
        permit: LedgerPermit,
        turn: TurnId,
        transaction: TransactionId,
        evidence: RejectedTurnEvidence,
        before_epoch: InstanceEpoch,
        phase: TurnFailurePhase,
        error: MechError,
    ) -> MResult<ResidentExternalTurnOutcome> {
        let message = bounded_failure_message(&error);
        let record = OwnedTurnRecord {
            header: TurnRecordHeader {
                turn_id: turn,
                transaction_id: transaction,
                input_range: evidence.input_range,
                status: TurnRecordStatus::Rejected,
                failure: Some(TurnFailureRecord {
                    phase,
                    kind: "ResidentExternalTurnRejected".to_owned(),
                    message,
                }),
            },
            body: ResidentTurnReceiptV1 {
                version: ResidentTurnReceiptV1::VERSION,
                instance: self.instance_id,
                program_revision: self.program_revision,
                plan_generation: self.plan_generation,
                layout_generation: self.layout_generation,
                input_batch_hash: evidence.input_batch_hash,
                before_epoch,
                after_epoch: None,
                state_hash: self.published_state_hash,
                touched_slots: 0,
                changed_slots: 0,
                executed_nodes: 0,
                effect_count: evidence.effect_count,
                outbox_effect_count: evidence.outbox_effect_count,
                transactional_effect_count: evidence.transactional_effect_count,
                effect_batch_hash: evidence.effect_batch_hash,
                effect_ids_hash: evidence.effect_ids_hash,
                idempotency_keys_hash: evidence.idempotency_keys_hash,
            },
        };
        let prepared = self.receipt_ledger.prepare_append(permit, record)?;
        let receipt_sequence = self.append_receipt(prepared);
        self.last_rejected_turn = Some(turn);
        Ok(ResidentExternalTurnOutcome::Rejected {
            turn,
            receipt_sequence,
            phase,
        })
    }

    #[allow(clippy::too_many_arguments)]
    fn reject_with_cleanup(
        &mut self,
        permit: LedgerPermit,
        turn: TurnId,
        transaction: TransactionId,
        evidence: RejectedTurnEvidence,
        before_epoch: InstanceEpoch,
        phase: TurnFailurePhase,
        error: MechError,
        cleanup: Vec<RuntimeEffectFailure>,
    ) -> MResult<ResidentExternalTurnOutcome> {
        let outcome = self.append_rejected(
            permit,
            turn,
            transaction,
            evidence,
            before_epoch,
            phase,
            error,
        )?;
        if cleanup.is_empty() {
            return Ok(outcome);
        }
        self.health = ResidentExternalHealth::PoisonedPrepublicationCleanup {
            turn,
            failure_hash: failures_hash(&cleanup),
        };
        Err(MechError::new(
            ResidentExternalCleanupFailed {
                turn,
                failures: cleanup,
            },
            None,
        ))
    }

    fn append_input_batch(
        &mut self,
        permit: LedgerPermit,
        batch: CapturedInputBatch,
    ) -> MResult<()> {
        let prepared = self.input_ledger.prepare_append(permit, batch)?;
        self.append_prepared_input(prepared);
        Ok(())
    }

    fn append_prepared_input(&mut self, prepared: PreparedLedgerAppend<CapturedInputBatch>) {
        let sequence = self.input_ledger.append(prepared);
        if self.durability == ResidentDurabilityPolicy::Volatile {
            let (discarded, _) = self
                .input_ledger
                .pop_front()
                .expect("volatile input append is immediately releasable");
            debug_assert_eq!(discarded, sequence);
        }
    }

    fn append_receipt(
        &mut self,
        prepared: PreparedLedgerAppend<ResidentTurnRecord>,
    ) -> LedgerSequence {
        let sequence = self.receipt_ledger.append(prepared);
        if self.durability == ResidentDurabilityPolicy::Volatile {
            let (discarded, _) = self
                .receipt_ledger
                .pop_front()
                .expect("volatile receipt append is immediately releasable");
            debug_assert_eq!(discarded, sequence);
        }
        sequence
    }

    fn observe_prepared_turn(&mut self, probe: mech_engine::resident::ResidentStructuralProbe) {
        self.structural_probe.commit_runtime_call_count = self
            .structural_probe
            .commit_runtime_call_count
            .saturating_add(probe.commit_runtime_call_count);
        self.structural_probe.legacy_journal_capture_count = self
            .structural_probe
            .legacy_journal_capture_count
            .saturating_add(probe.legacy_journal_capture_count);
        self.structural_probe
            .runtime_execution_transaction_construction_count = self
            .structural_probe
            .runtime_execution_transaction_construction_count
            .saturating_add(probe.runtime_execution_transaction_construction_count);
    }

    fn ensure_healthy(&self) -> MResult<()> {
        if self.health == ResidentExternalHealth::Healthy {
            Ok(())
        } else {
            Err(MechError::new(
                ResidentExternalCoordinatorPoisoned {
                    health: self.health.clone(),
                },
                None,
            ))
        }
    }

    fn ensure_live_bindings(&self) -> MResult<()> {
        if self.live
            && self
                .bound
                .observations()
                .iter()
                .all(|observation| observation.provider_binding.is_some())
            && self
                .bound
                .effects()
                .iter()
                .all(|effect| effect.provider_binding.is_some())
        {
            Ok(())
        } else {
            invalid_coordinator(
                "offline replay coordinator cannot execute live provider operations",
            )
        }
    }

    fn publish_prepared(
        &mut self,
        turn: TurnId,
        prepared_turn: PreparedResidentTurn<'_>,
    ) -> MResult<()> {
        let summary = prepared_turn
            .publish_external(&self.publication_authority)
            .map_err(resident_execution_error)?;
        self.published_state_hash = summary.state_hash;
        self.published_through_turn = turn.get();
        self.structural_probe.publication_store_count = self
            .structural_probe
            .publication_store_count
            .saturating_add(1);
        Ok(())
    }

    fn allocate_turn(&mut self) -> MResult<TurnId> {
        let turn = TurnId::new(self.next_turn).ok_or_else(sequence_exhausted)?;
        self.next_turn = self
            .next_turn
            .checked_add(1)
            .ok_or_else(sequence_exhausted)?;
        Ok(turn)
    }

    fn advance_input_identity(&mut self, batch: &CapturedInputBatch) -> MResult<()> {
        self.next_input = batch
            .range
            .last()
            .get()
            .checked_add(1)
            .ok_or_else(sequence_exhausted)?;
        Ok(())
    }
}

#[derive(Clone, Debug)]
struct MaterializedEffect {
    turn: TurnId,
    id: RuntimeEffectId,
    bound: BoundResidentEffect,
    value: Value,
    payload_hash: ValueHash,
    idempotency_key: String,
    retained_bytes: usize,
}

#[derive(Clone, Copy, Debug, Default)]
struct RejectedTurnEvidence {
    input_range: Option<InputSequenceRange>,
    input_batch_hash: [u8; 32],
    effect_count: u32,
    outbox_effect_count: u32,
    transactional_effect_count: u32,
    effect_batch_hash: [u8; 32],
    effect_ids_hash: [u8; 32],
    idempotency_keys_hash: [u8; 32],
}

impl RejectedTurnEvidence {
    fn from_input(batch: &CapturedInputBatch) -> Self {
        Self {
            input_range: Some(batch.range),
            input_batch_hash: batch.batch_hash,
            ..Self::default()
        }
    }

    fn with_effects(mut self, effects: &[MaterializedEffect]) -> MResult<Self> {
        self.effect_count = u32::try_from(effects.len()).map_err(|_| count_overflow())?;
        self.outbox_effect_count = u32::try_from(
            effects
                .iter()
                .filter(|effect| matches!(effect.bound.interaction, ExternalInteraction::Effect(_)))
                .count(),
        )
        .map_err(|_| count_overflow())?;
        self.transactional_effect_count = u32::try_from(
            effects
                .iter()
                .filter(|effect| {
                    matches!(
                        effect.bound.interaction,
                        ExternalInteraction::TransactionalExternal(_)
                    )
                })
                .count(),
        )
        .map_err(|_| count_overflow())?;
        self.effect_batch_hash = effect_batch_hash(effects);
        self.effect_ids_hash = resident_effect_ids_hash(effects.iter().map(|effect| effect.id));
        self.idempotency_keys_hash = resident_idempotency_keys_hash(
            effects.iter().map(|effect| effect.idempotency_key.as_str()),
        );
        Ok(self)
    }
}

struct CaptureFailure {
    captured_prefix: Option<CapturedInputBatch>,
    error: MechError,
}

impl CaptureFailure {
    fn without_prefix(error: MechError) -> Self {
        Self {
            captured_prefix: None,
            error,
        }
    }

    fn with_prefix(facts: Vec<CapturedInputFact>, error: MechError) -> Self {
        if facts.is_empty() {
            return Self::without_prefix(error);
        }
        match CapturedInputBatch::new(facts) {
            Ok(batch) => Self {
                captured_prefix: Some(batch),
                error,
            },
            Err(error) => Self::without_prefix(error),
        }
    }
}

struct ProviderPreparationFailure {
    error: MechError,
    cleanup: Vec<RuntimeEffectFailure>,
}

impl ProviderPreparationFailure {
    fn new(error: MechError, journal: &mut RuntimeEffectJournal) -> Self {
        Self {
            error,
            cleanup: journal.abort_all(),
        }
    }
}

fn materialize_effects(
    prepared: &PreparedResidentTurn<'_>,
    artifact: &ProgramArtifact,
    bound: &[BoundResidentEffect],
    instance: ReactiveInstanceId,
    turn: TurnId,
) -> MResult<Vec<MaterializedEffect>> {
    let mut effects = Vec::with_capacity(bound.len());
    for intent in prepared.effect_intents() {
        let binding = bound
            .iter()
            .find(|binding| binding.ordinal == intent.ordinal)
            .ok_or_else(|| {
                MechError::new(
                    ResidentExternalCoordinatorInvalid {
                        reason: "staged effect has no frozen provider binding".to_owned(),
                    },
                    None,
                )
            })?;
        let value = prepared.materialize_effect_payload(intent.ordinal)?;
        let payload_hash = value
            .value_hash(artifact.schemas())
            .map_err(|error| invalid_value(format!("effect payload hash failed: {error:?}")))?;
        let retained_bytes = value
            .canonical_payload_bytes(artifact.schemas())
            .map_err(|error| invalid_value(format!("effect payload encoding failed: {error:?}")))?
            .len()
            + core::mem::size_of::<ResidentOutboxPayload>();
        let requirement = artifact
            .requirements()
            .get(binding.requirement)
            .ok_or_else(|| {
                invalid_value("effect requirement disappeared from artifact".to_owned())
            })?;
        let idempotency_key = resident_idempotency_key(
            instance,
            artifact.revision(),
            turn,
            intent.ordinal,
            requirement,
            payload_hash,
        )?;
        effects.push(MaterializedEffect {
            turn,
            id: resident_effect_id(instance, turn, intent.ordinal),
            bound: binding.clone(),
            value,
            payload_hash,
            idempotency_key,
            retained_bytes,
        });
    }
    Ok(effects)
}

fn effect_batch_hash(effects: &[MaterializedEffect]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"mech-resident-effect-batch-v1");
    for effect in effects {
        hash.update(&effect.id.transaction.as_u128().to_le_bytes());
        hash.update(&effect.id.sequence.to_le_bytes());
        hash.update(effect.payload_hash.as_bytes());
        hash.update(effect.idempotency_key.as_bytes());
    }
    *hash.finalize().as_bytes()
}

fn request_write_intent(effect: &BoundResidentEffect) -> MResult<RuntimeResourceWriteIntent> {
    match effect.request.intent {
        mech_core::ResourceIntent::Assign => Ok(RuntimeResourceWriteIntent::Assign),
        mech_core::ResourceIntent::Send => Ok(RuntimeResourceWriteIntent::Send),
        mech_core::ResourceIntent::Read => invalid_coordinator("effect requirement is not a write"),
    }
}

fn prepared_protocol_matches(
    interaction: &ExternalInteraction,
    protocol: RuntimeEffectProtocol,
) -> bool {
    match interaction {
        ExternalInteraction::Effect(_) => protocol == RuntimeEffectProtocol::AfterCommit,
        ExternalInteraction::TransactionalExternal(contract) => match contract.protocol {
            TransactionalEffectProtocol::PrepareCommit => {
                protocol == RuntimeEffectProtocol::Transactional
            }
            TransactionalEffectProtocol::PrepareCommitCompensate => {
                protocol == RuntimeEffectProtocol::Compensatable
            }
        },
        _ => false,
    }
}

fn resident_scalar_bytes(kind: ResidentValueKind) -> usize {
    match kind {
        ResidentValueKind::Bool => 1,
        ResidentValueKind::Index | ResidentValueKind::F64 => 8,
        // String payloads are dynamic. Reserve a conservative bounded lane
        // before provider capture/effect materialization; numeric estimates
        // remain exact and structurally unchanged.
        ResidentValueKind::String => 64 * 1_024,
    }
}

fn failures_hash(failures: &[RuntimeEffectFailure]) -> [u8; 32] {
    let mut hash = blake3::Hasher::new();
    hash.update(b"mech-resident-external-failure-v1");
    for failure in failures {
        hash.update(&failure.effect_id.transaction.as_u128().to_le_bytes());
        hash.update(&failure.effect_id.sequence.to_le_bytes());
        hash.update(format!("{:?}", failure.phase).as_bytes());
        hash.update(failure.message.as_bytes());
    }
    *hash.finalize().as_bytes()
}

fn resident_execution_error(error: ResidentExecutionError) -> MechError {
    invalid_value(format!("resident candidate failed: {error:?}"))
}

fn invalid_value(reason: String) -> MechError {
    MechError::new(ResidentExternalCoordinatorInvalid { reason }, None)
}

fn invalid_coordinator<T>(reason: &'static str) -> MResult<T> {
    Err(invalid_value(reason.to_owned()))
}

fn sequence_exhausted() -> MechError {
    invalid_value("resident external identity sequence exhausted".to_owned())
}

fn count_overflow() -> MechError {
    invalid_value("resident external receipt count exceeds u32".to_owned())
}

fn bounded_failure_message(error: &MechError) -> String {
    let rendered = format!("{error:?}");
    if rendered.len() <= MAX_FAILURE_MESSAGE_BYTES {
        return rendered;
    }
    let mut end = MAX_FAILURE_MESSAGE_BYTES;
    while !rendered.is_char_boundary(end) {
        end -= 1;
    }
    let mut bounded = rendered[..end].to_owned();
    bounded.shrink_to_fit();
    bounded
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UnsupportedResidentDurability {
    pub durability: ResidentDurabilityPolicy,
}

impl MechErrorKind for UnsupportedResidentDurability {
    fn name(&self) -> &str {
        "UnsupportedResidentDurability"
    }
    fn message(&self) -> String {
        format!(
            "resident durability {:?} is not implemented in D3",
            self.durability
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentExternalCoordinatorInvalid {
    pub reason: String,
}

impl MechErrorKind for ResidentExternalCoordinatorInvalid {
    fn name(&self) -> &str {
        "ResidentExternalCoordinatorInvalid"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentPreparedEffectProtocolMismatch {
    pub effect: RuntimeEffectId,
    pub actual: RuntimeEffectProtocol,
}

impl MechErrorKind for ResidentPreparedEffectProtocolMismatch {
    fn name(&self) -> &str {
        "ResidentPreparedEffectProtocolMismatch"
    }
    fn message(&self) -> String {
        format!(
            "resident effect {} returned unexpected protocol {:?}",
            self.effect, self.actual
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentExternalCoordinatorPoisoned {
    pub health: ResidentExternalHealth,
}

impl MechErrorKind for ResidentExternalCoordinatorPoisoned {
    fn name(&self) -> &str {
        "ResidentExternalCoordinatorPoisoned"
    }
    fn message(&self) -> String {
        format!(
            "resident external coordinator is poisoned: {:?}",
            self.health
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResidentExternalCleanupFailed {
    pub turn: TurnId,
    pub failures: Vec<RuntimeEffectFailure>,
}

impl MechErrorKind for ResidentExternalCleanupFailed {
    fn name(&self) -> &str {
        "ResidentExternalCleanupFailed"
    }
    fn message(&self) -> String {
        format!("resident external cleanup failed for turn {}", self.turn)
    }
}
