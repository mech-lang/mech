use super::{
    MechRuntime, RuntimeInvalidOperationError, RuntimeTransactionalLiveRegistrationUnsupported,
};
use crate::{
    ActorId, MessageRecord, ModuleVersionId, ObjectId, ResourceBudget, RuntimeAuthorityScope,
    RuntimeContext, RuntimeContextBinding, RuntimeId, TaskId,
};
use mech_core::{MResult, MechError, ValRef};
use mech_engine::ProgramInputId;
use std::collections::HashMap;

#[derive(Clone, Debug)]
pub(in crate::runtime) struct RuntimeLiveContextTemplate {
    pub(in crate::runtime) runtime: RuntimeId,
    pub(in crate::runtime) subject: String,
    pub(in crate::runtime) task: Option<TaskId>,
    pub(in crate::runtime) actor: Option<ActorId>,
    pub(in crate::runtime) module_version: Option<ModuleVersionId>,
    pub(in crate::runtime) authority: RuntimeAuthorityScope,
    pub(in crate::runtime) budget_limits: ResourceBudget,
    pub(in crate::runtime) actor_message: Option<MessageRecord>,
    pub(in crate::runtime) actor_state: Option<ObjectId>,
}

impl RuntimeLiveContextTemplate {
    fn from_context(context: &RuntimeContext) -> Self {
        Self {
            runtime: context.runtime,
            subject: context.subject.clone(),
            task: context.task,
            actor: context.actor,
            module_version: context.module_version,
            authority: context.authority.clone(),
            budget_limits: ResourceBudget {
                max_steps: context.budget.max_steps,
                used_steps: 0,
                max_bytes: context.budget.max_bytes,
                used_bytes: 0,
                max_items: context.budget.max_items,
                used_items: 0,
                max_messages: context.budget.max_messages,
                used_messages: 0,
            },
            actor_message: context.actor_message.clone(),
            actor_state: context.actor_state,
        }
    }

    fn fresh_context(&self) -> RuntimeContext {
        RuntimeContext {
            runtime: self.runtime,
            subject: self.subject.clone(),
            task: self.task,
            actor: self.actor,
            access: Default::default(),
            module_version: self.module_version,
            transaction: None,
            authority: self.authority.clone(),
            budget: self.budget_limits.clone(),
            events: Vec::new(),
            actor_message: self.actor_message.clone(),
            actor_state: self.actor_state,
        }
    }

    fn matches_context(&self, context: &RuntimeContext) -> bool {
        self.runtime == context.runtime
            && self.subject == context.subject
            && self.task == context.task
            && self.actor == context.actor
            && self.module_version == context.module_version
            && self.actor_message == context.actor_message
            && self.actor_state == context.actor_state
            && self.authority == context.authority
            && self.budget_limits.max_steps == context.budget.max_steps
            && self.budget_limits.max_bytes == context.budget.max_bytes
            && self.budget_limits.max_items == context.budget.max_items
            && self.budget_limits.max_messages == context.budget.max_messages
    }
}

#[derive(Clone)]
pub(in crate::runtime) struct RuntimeLiveStateSnapshot {
    pub(in crate::runtime) context_template: Option<RuntimeLiveContextTemplate>,
    pub(in crate::runtime) input_bindings:
        HashMap<crate::RuntimeHostInputSource, Vec<ProgramInputId>>,
    pub(in crate::runtime) persistent_sends: Vec<RuntimePersistentSend>,
    pub(in crate::runtime) registration_mode: LiveRegistrationMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveRegistrationMode {
    RetainedRoot,
    IsolatedSnapshot,
}

#[derive(Clone, Debug)]
pub(in crate::runtime) struct RuntimePersistentSend {
    pub(in crate::runtime) binding: RuntimeContextBinding,
    pub(in crate::runtime) path: String,
    pub(in crate::runtime) value: ValRef,
    pub(in crate::runtime) schedule: RuntimePersistentSendSchedule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimePersistentSendSchedule {
    EveryAcceptedTurn,
    Activation {
        interpreter_id: u64,
        barrier_node_id: mech_core::ReactiveNodeId,
    },
}

impl MechRuntime {
    pub(in crate::runtime) fn validate_live_context_candidate(
        &self,
        context: &RuntimeContext,
    ) -> MResult<()> {
        if let Some(transaction_id) = context.transaction {
            let active_operation = self.active_program_operation.get();
            if self.program_transaction_owner != Some(transaction_id)
                || active_operation.map(|active| active.transaction_id) != Some(transaction_id)
            {
                return Err(MechError::new(
                    RuntimeTransactionalLiveRegistrationUnsupported {
                        transaction_id,
                        owner: self.program_transaction_owner,
                        active_operation: active_operation.map(|active| active.operation),
                    },
                    None,
                ));
            }
        }
        match &self.live_context_template {
      Some(template) if template.matches_context(context) => Ok(()),
      Some(_) => Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "RuntimeLiveContextMismatch",
          reason:
            "source load attempted to change the live program execution identity or budget maxima"
              .to_string(),
        },
        None,
      )),
      None => Ok(()),
    }
    }

    pub(in crate::runtime) fn commit_live_context_candidate(&mut self, context: &RuntimeContext) {
        if self.live_context_template.is_none() {
            self.live_context_template = Some(RuntimeLiveContextTemplate::from_context(context));
        }
    }

    pub(in crate::runtime) fn live_state_snapshot(&self) -> RuntimeLiveStateSnapshot {
        RuntimeLiveStateSnapshot {
            context_template: self.live_context_template.clone(),
            input_bindings: self.live_input_bindings.clone(),
            persistent_sends: self.persistent_sends.clone(),
            registration_mode: self.live_registration_mode,
        }
    }

    pub(in crate::runtime) fn restore_live_state(&mut self, snapshot: RuntimeLiveStateSnapshot) {
        self.live_context_template = snapshot.context_template;
        self.live_input_bindings = snapshot.input_bindings;
        self.persistent_sends = snapshot.persistent_sends;
        self.live_registration_mode = snapshot.registration_mode;
    }

    pub(in crate::runtime) fn live_turn_context(&self) -> MResult<RuntimeContext> {
        self.live_context_template
            .as_ref()
            .map(RuntimeLiveContextTemplate::fresh_context)
            .ok_or_else(|| {
                MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "RuntimeLiveContextMissing",
                        reason: "host input turn requires a stored live program context"
                            .to_string(),
                    },
                    None,
                )
            })
    }
}
