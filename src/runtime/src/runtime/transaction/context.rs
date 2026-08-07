use crate::runtime::MechRuntime;
use crate::{
    AccessSet, ActorId, ActorTurn, MessageRecord, ModuleVersionId, ObjectId, ResourceBudget,
    RuntimeAuthorityScope, RuntimeContext, RuntimeEvent, RuntimeId, RuntimeInvalidOperationError,
    TaskId, TransactionId,
};
use mech_core::{MResult, MechError};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(in crate::runtime) struct RuntimeTransactionContextIdentity {
    runtime: RuntimeId,
    subject: String,
    task: Option<TaskId>,
    actor: Option<ActorId>,
    actor_message: Option<MessageRecord>,
    actor_state: Option<ObjectId>,
}

impl RuntimeTransactionContextIdentity {
    pub(in crate::runtime) fn capture(context: &RuntimeContext) -> Self {
        Self {
            runtime: context.runtime,
            subject: context.subject.clone(),
            task: context.task,
            actor: context.actor,
            actor_message: context.actor_message.clone(),
            actor_state: context.actor_state,
        }
    }

    pub(in crate::runtime) fn mismatch_reason(&self, context: &RuntimeContext) -> Option<String> {
        if self.runtime != context.runtime {
            return Some(format!(
                "runtime changed from {} to {}",
                self.runtime, context.runtime,
            ));
        }
        if self.subject != context.subject {
            return Some(format!(
                "subject changed from `{}` to `{}`",
                self.subject, context.subject,
            ));
        }
        if self.task != context.task {
            return Some(format!(
                "task changed from {:?} to {:?}",
                self.task, context.task,
            ));
        }
        if self.actor != context.actor {
            return Some(format!(
                "actor changed from {:?} to {:?}",
                self.actor, context.actor,
            ));
        }
        if self.actor_message != context.actor_message {
            return Some("actor message changed".to_string());
        }
        if self.actor_state != context.actor_state {
            return Some(format!(
                "actor state changed from {:?} to {:?}",
                self.actor_state, context.actor_state,
            ));
        }
        None
    }

    pub(in crate::runtime) fn set_actor_state(&mut self, state: ObjectId) {
        self.actor_state = Some(state);
    }

    pub(in crate::runtime) fn bind_actor_turn(&mut self, turn: &ActorTurn) {
        self.subject = turn.subject.clone();
        self.actor = Some(turn.actor);
        self.actor_message = Some(turn.message.clone());
        self.actor_state = turn.state;
    }
}

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeContextCheckpoint {
    runtime: RuntimeId,
    subject: String,
    task: Option<TaskId>,
    actor: Option<ActorId>,
    access: AccessSet,
    module_version: Option<ModuleVersionId>,
    transaction: Option<TransactionId>,
    authority: RuntimeAuthorityScope,
    budget: ResourceBudget,
    events: Vec<RuntimeEvent>,
    actor_message: Option<MessageRecord>,
    actor_state: Option<ObjectId>,
}

impl RuntimeContextCheckpoint {
    pub(in crate::runtime) fn capture(context: &RuntimeContext) -> Self {
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_context_event_snapshot(context.events.len());
        Self {
            runtime: context.runtime,
            subject: context.subject.clone(),
            task: context.task,
            actor: context.actor,
            access: context.access.clone(),
            module_version: context.module_version,
            transaction: context.transaction,
            authority: context.authority.clone(),
            budget: context.budget.clone(),
            events: context.events.clone(),
            actor_message: context.actor_message.clone(),
            actor_state: context.actor_state,
        }
    }

    pub(in crate::runtime) fn restore_preserving_consumption(&self, context: &mut RuntimeContext) {
        let used_steps = context.budget.used_steps.max(self.budget.used_steps);
        let used_bytes = context.budget.used_bytes.max(self.budget.used_bytes);
        let used_items = context.budget.used_items.max(self.budget.used_items);
        let used_messages = context.budget.used_messages.max(self.budget.used_messages);

        context.runtime = self.runtime;
        context.subject = self.subject.clone();
        context.task = self.task;
        context.actor = self.actor;
        context.access = self.access.clone();
        context.module_version = self.module_version;
        context.transaction = self.transaction;
        context.authority = self.authority.clone();
        context.budget = ResourceBudget {
            max_steps: self.budget.max_steps,
            used_steps,
            max_bytes: self.budget.max_bytes,
            used_bytes,
            max_items: self.budget.max_items,
            used_items,
            max_messages: self.budget.max_messages,
            used_messages,
        };
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_context_event_snapshot(self.events.len());
        context.events = self.events.clone();
        context.actor_message = self.actor_message.clone();
        context.actor_state = self.actor_state;
    }

    pub(in crate::runtime) fn access_delta(&self, context: &RuntimeContext) -> AccessSet {
        AccessSet {
            reads: context
                .access
                .reads
                .iter()
                .copied()
                .filter(|object| !self.access.reads.contains(object))
                .collect(),
            writes: context
                .access
                .writes
                .iter()
                .copied()
                .filter(|object| !self.access.writes.contains(object))
                .collect(),
        }
    }
}

impl MechRuntime {
    pub(in crate::runtime) fn context_transaction_id(
        context: &RuntimeContext,
    ) -> MResult<TransactionId> {
        context.transaction.ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "context_transaction_id",
                    reason: "context has no active transaction".to_string(),
                },
                None,
            )
        })
    }
}
