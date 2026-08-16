// ---------------------------------------------------------------------------
// Actor methods
// ---------------------------------------------------------------------------

// Actors are the primary entities in the Mech runtime that encapsulate state and behavior. They can receive messages, execute turns, and interact with other actors. The methods in this section allow you to create, retrieve, update, and manage actors, as well as send messages to them and run their turns.

use crate::runtime::{MechRuntime, RuntimeInvalidOperationError, RuntimeRecordNotFoundError};
use crate::{
    ActorId, ActorRecord, ActorTurn, CapabilityId, MessageId, MessageRecord, ModuleVersionId,
    ObjectId, ResourceBudgetExceededError, RuntimeContext, RuntimeEventKind,
    RuntimeTransactionNotFoundError, TransactionId,
};
use mech_core::{MResult, MechError};
use std::collections::HashMap;

enum VisibleTransactionMessage {
    Durable(MessageRecord),
    Staged(MessageRecord),
}

impl MechRuntime {
    fn first_visible_transaction_message(
        &self,
        transaction_id: TransactionId,
        actor: ActorId,
    ) -> MResult<Option<VisibleTransactionMessage>> {
        let transaction = self
            .active_transactions
            .get(&transaction_id)
            .ok_or_else(|| {
                MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None)
            })?;

        let mut skipped_occurrences: HashMap<MessageId, usize> = HashMap::new();

        for message in self.store.list_mailbox(actor)? {
            let acknowledged = transaction
                .store
                .staged_message_ack_occurrences(actor, message.id);
            let skipped = skipped_occurrences.entry(message.id).or_insert(0);

            if *skipped < acknowledged {
                *skipped += 1;
                continue;
            }

            return Ok(Some(VisibleTransactionMessage::Durable(message)));
        }

        Ok(transaction
            .store
            .peek_staged_enqueued_message(actor)
            .map(VisibleTransactionMessage::Staged))
    }

    pub fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        self.ensure_runtime_mutation_allowed("put_actor")?;
        let mut context = self.context_for_actor(&actor)?;
        self.put_actor_with_context(&mut context, actor)
    }

    pub fn put_actor_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor: ActorRecord,
    ) -> MResult<ActorId> {
        self.ensure_runtime_mutation_allowed("put_actor_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;

        if self.store.get_actor(actor.id)?.is_none() {
            if let Some(max) = self.config.limits.max_actors {
                let used = self.store.actor_count()?;
                let next = used.checked_add(1).ok_or_else(|| {
                    MechError::new(
                        ResourceBudgetExceededError {
                            resource: "actors",
                            used,
                            requested: 1,
                            max: None,
                        },
                        None,
                    )
                })?;
                if next > max {
                    return Err(MechError::new(
                        ResourceBudgetExceededError {
                            resource: "actors",
                            used,
                            requested: 1,
                            max: Some(max),
                        },
                        None,
                    ));
                }
            }
        }

        let id = self.store.put_actor(actor)?;

        self.emit_event_to_context(context, RuntimeEventKind::ActorCreated { actor_id: id })?;

        Ok(id)
    }

    pub fn create_actor(
        &mut self,
        subject: impl Into<String>,
        behavior: Option<ModuleVersionId>,
        state: Option<ObjectId>,
        capabilities: Vec<CapabilityId>,
    ) -> MResult<ActorId> {
        self.ensure_runtime_mutation_allowed("create_actor")?;
        let id = self.next_actor_id();

        let mut actor = ActorRecord::new(id, subject).with_capabilities(capabilities);

        if let Some(behavior) = behavior {
            actor = actor.with_behavior(behavior);
        }

        if let Some(state) = state {
            actor = actor.with_state(state);
        }

        self.put_actor(actor)
    }

    pub fn get_actor(&self, id: ActorId) -> MResult<Option<ActorRecord>> {
        self.store.get_actor(id)
    }

    pub fn get_actor_with_context(
        &mut self,
        context: &mut RuntimeContext,
        id: ActorId,
    ) -> MResult<Option<ActorRecord>> {
        self.validate_context_for_runtime(context)?;

        if let Some(transaction_id) = context.transaction {
            if let Some(transaction) = self.active_transactions.get(&transaction_id) {
                if let Some(actor) = transaction.store.get_staged_actor(id) {
                    return Ok(Some(actor));
                }
            }
        }

        self.store.get_actor(id)
    }

    pub fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        self.ensure_runtime_mutation_allowed("update_actor")?;
        self.store.update_actor(actor)
    }

    pub fn update_actor_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor: ActorRecord,
    ) -> MResult<ActorId> {
        self.ensure_runtime_mutation_allowed("update_actor_with_context")?;
        self.validate_context_for_runtime(context)?;

        if let Some(transaction_id) = context.transaction {
            let id = actor.id;

            self.active_transaction_mut(transaction_id)?
                .stage_actor_update(actor)?;

            return Ok(id);
        }

        self.store.update_actor(actor)
    }

    pub fn send_message(
        &mut self,
        actor: ActorId,
        kind: impl Into<String>,
        payload: Vec<u8>,
    ) -> MResult<MessageId> {
        self.ensure_runtime_mutation_allowed("send_message")?;
        let Some(actor_record) = self.store.get_actor(actor)? else {
            return Err(MechError::new(
                RuntimeRecordNotFoundError {
                    record_type: "actor",
                    id: actor.to_string(),
                },
                None,
            ));
        };

        let mut context = self.context_for_actor(&actor_record)?;
        self.send_message_with_context(&mut context, actor, kind, payload)
    }

    pub fn send_message_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor: ActorId,
        kind: impl Into<String>,
        payload: Vec<u8>,
    ) -> MResult<MessageId> {
        self.ensure_runtime_mutation_allowed("send_message_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_messages(1)?;
        context.charge_bytes(payload.len() as u64)?;

        self.enforce_actor_mailbox_limit(context, actor)?;

        let id = self.next_message_id();
        let message = MessageRecord::new(id, actor, kind, payload);

        if let Some(transaction_id) = context.transaction {
            self.active_transaction_mut(transaction_id)?
                .stage_message_enqueue(actor, message)?;

            self.emit_event_to_context(
                context,
                RuntimeEventKind::ActorMessageSent {
                    actor_id: actor,
                    message_id: id,
                },
            )?;

            return Ok(id);
        }

        self.store.enqueue_message(actor, message)?;

        self.emit_event_to_context(
            context,
            RuntimeEventKind::ActorMessageSent {
                actor_id: actor,
                message_id: id,
            },
        )?;

        Ok(id)
    }

    fn enforce_actor_mailbox_limit(&self, context: &RuntimeContext, actor: ActorId) -> MResult<()> {
        let Some(max) = self.config.limits.max_actor_mailbox_len else {
            return Ok(());
        };

        let durable_len = self.store.mailbox_len(actor)?;
        let mut effective_len = durable_len;

        if let Some(transaction_id) = context.transaction {
            if let Some(transaction) = self.active_transactions.get(&transaction_id) {
                let ack_count = transaction.store.staged_message_ack_count(actor)?;
                effective_len = effective_len.checked_sub(ack_count).ok_or_else(|| {
                    MechError::new(
                        RuntimeInvalidOperationError {
                            operation: "send_message",
                            reason: "staged message acknowledgements exceed durable mailbox length"
                                .to_string(),
                        },
                        None,
                    )
                })?;
                effective_len = effective_len
                    .checked_add(transaction.store.staged_message_enqueue_count(actor)?)
                    .ok_or_else(|| {
                        MechError::new(
                            ResourceBudgetExceededError {
                                resource: "actor_mailbox",
                                used: effective_len,
                                requested: 1,
                                max: None,
                            },
                            None,
                        )
                    })?;
            }
        }

        let next_len = effective_len.checked_add(1).ok_or_else(|| {
            MechError::new(
                ResourceBudgetExceededError {
                    resource: "actor_mailbox",
                    used: effective_len,
                    requested: 1,
                    max: None,
                },
                None,
            )
        })?;

        if next_len > max {
            return Err(MechError::new(
                ResourceBudgetExceededError {
                    resource: "actor_mailbox",
                    used: effective_len,
                    requested: 1,
                    max: Some(max),
                },
                None,
            ));
        }

        Ok(())
    }

    pub fn pop_message(&mut self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        self.ensure_runtime_mutation_allowed("pop_message")?;
        self.store.pop_message(actor)
    }

    pub fn pop_message_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor: ActorId,
    ) -> MResult<Option<MessageRecord>> {
        self.ensure_runtime_mutation_allowed("pop_message_with_context")?;
        self.validate_context_for_runtime(context)?;

        if let Some(transaction_id) = context.transaction {
            return match self.first_visible_transaction_message(transaction_id, actor)? {
                Some(VisibleTransactionMessage::Durable(message)) => {
                    self.active_transaction_mut(transaction_id)?
                        .stage_message_ack(actor, message.id)?;

                    Ok(Some(message))
                }
                Some(VisibleTransactionMessage::Staged(_)) => Ok(self
                    .active_transaction_mut(transaction_id)?
                    .pop_staged_enqueued_message(actor)),
                None => Ok(None),
            };
        }

        self.store.pop_message(actor)
    }

    pub fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        self.store.peek_message(actor)
    }

    pub fn peek_message_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor: ActorId,
    ) -> MResult<Option<MessageRecord>> {
        self.validate_context_for_runtime(context)?;

        if let Some(transaction_id) = context.transaction {
            return match self.first_visible_transaction_message(transaction_id, actor)? {
                Some(VisibleTransactionMessage::Durable(message))
                | Some(VisibleTransactionMessage::Staged(message)) => Ok(Some(message)),
                None => Ok(None),
            };
        }

        self.store.peek_message(actor)
    }

    pub fn next_actor_turn_with_context(
        &mut self,
        context: &mut RuntimeContext,
        actor_id: ActorId,
    ) -> MResult<Option<ActorTurn>> {
        self.ensure_runtime_mutation_allowed("next_actor_turn_with_context")?;
        self.validate_context_for_runtime(context)?;

        let Some(actor) = self.get_actor_with_context(context, actor_id)? else {
            return Err(MechError::new(
                RuntimeRecordNotFoundError {
                    record_type: "actor",
                    id: actor_id.to_string(),
                },
                None,
            ));
        };

        let Some(message) = self.pop_message_with_context(context, actor_id)? else {
            return Ok(None);
        };

        Ok(Some(ActorTurn::new(actor, message)?))
    }
}

#[cfg(test)]
#[path = "tests/mod.rs"]
mod tests;
