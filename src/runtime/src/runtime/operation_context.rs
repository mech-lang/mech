//! Runtime operation-context construction and validation.

use super::{MechRuntime, RuntimeInvalidOperationError, RuntimeTransactionContextMismatch};
use crate::{
    ActorRecord, ActorTurn, RuntimeContext, RuntimeContextBuilder, TaskRecord, TransactionRecord,
};
use mech_core::{MResult, MechError};

impl MechRuntime {
    pub fn runtime_context(&self) -> MResult<RuntimeContext> {
        RuntimeContextBuilder::new(self.id)
            .budget(self.default_budget())
            .build()
    }

    pub fn context_for_task(&self, task: &TaskRecord) -> MResult<RuntimeContext> {
        let mut builder = RuntimeContextBuilder::new(self.id)
            .subject(task.subject.clone())
            .task(task.id)
            .capabilities(task.capabilities.clone())
            .budget(self.default_budget());

        if let Some(module_version) = task.module_version {
            builder = builder.module_version(module_version);
        }

        builder.build()
    }

    pub fn context_for_actor(&self, actor: &ActorRecord) -> MResult<RuntimeContext> {
        let mut builder = RuntimeContextBuilder::new(self.id)
            .subject(actor.subject.clone())
            .actor(actor.id)
            .capabilities(actor.capabilities.clone())
            .budget(self.default_budget());

        if let Some(module_version) = actor.behavior {
            builder = builder.module_version(module_version);
        }
        if let Some(state) = actor.state {
            builder = builder.actor_state(state);
        }

        builder.build()
    }

    pub fn context_for_actor_turn(&self, turn: &ActorTurn) -> MResult<RuntimeContext> {
        turn.validate()?;
        let actor = self.store.get_actor(turn.actor)?.ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "context_for_actor_turn",
                    reason: format!("actor record {} was not found", turn.actor,),
                },
                None,
            )
        })?;
        if actor.subject != turn.subject
            || actor.behavior != turn.behavior
            || actor.state != turn.state
        {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "context_for_actor_turn",
                    reason: format!(
                        "actor turn metadata does not match actor record {}",
                        turn.actor,
                    ),
                },
                None,
            ));
        }

        let mut builder = RuntimeContextBuilder::new(self.id)
            .subject(actor.subject)
            .actor(actor.id)
            .actor_message(turn.message.clone())
            .capabilities(actor.capabilities)
            .budget(self.default_budget());
        if let Some(module_version) = actor.behavior {
            builder = builder.module_version(module_version);
        }
        if let Some(state) = actor.state {
            builder = builder.actor_state(state);
        }
        builder.build()
    }

    /// Build a subject context from a persisted transaction record.
    ///
    /// Transaction records are historical metadata. This context does not reopen,
    /// resume, or attach to the recorded transaction, and `transaction` remains
    /// unset.
    pub fn context_for_transaction(
        &self,
        transaction: &TransactionRecord,
    ) -> MResult<RuntimeContext> {
        RuntimeContextBuilder::new(self.id)
            .subject(transaction.subject.clone())
            .budget(self.default_budget())
            .build()
    }

    pub(in crate::runtime) fn validate_context_for_runtime(
        &self,
        context: &RuntimeContext,
    ) -> MResult<()> {
        context.validate()?;

        if context.runtime != self.id {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "validate_context_for_runtime",
                    reason: format!(
                        "runtime context mismatch: expected runtime {}, supplied runtime {}",
                        self.id, context.runtime,
                    ),
                },
                None,
            ));
        }

        if let Some(transaction_id) = context.transaction {
            let transaction = self.active_execution_transaction(transaction_id)?;

            if let Some(reason) = transaction.context_identity.mismatch_reason(context) {
                return Err(MechError::new(
                    RuntimeTransactionContextMismatch {
                        transaction_id,
                        reason,
                    },
                    None,
                ));
            }
        }

        Ok(())
    }
}
