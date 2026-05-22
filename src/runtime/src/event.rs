//! Runtime events.
//!
//! Runtime events are typed records emitted by the runtime as it performs
//! lifecycle, module, capability, task, actor, object, and transaction work.
//!
//! The event enum captures semantic meaning. The event envelope carries durable
//! event identity and ordering metadata.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::id::{
  ActorId, CapabilityId, EventId, ModuleVersionId, ObjectId, RuntimeId, TaskId,
  TransactionId,
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeEventKind {
  RuntimeCreated { runtime_id: RuntimeId },
  RuntimeShutdown { runtime_id: RuntimeId },
  SourceResolved { canonical_uri: String },
  ModuleCompiled { module_version: ModuleVersionId },
  ModuleActivated { module_version: ModuleVersionId },
  CapabilityGranted { capability_id: CapabilityId },
  CapabilityRevoked { capability_id: CapabilityId },
  ProgramStarted { task_id: TaskId },
  ProgramCompleted { task_id: TaskId },
  ProgramFailed { task_id: Option<TaskId>, message: String },
  TaskCreated { task_id: TaskId },
  TaskStarted { task_id: TaskId },
  TaskCompleted { task_id: TaskId },
  TaskFailed { task_id: TaskId, message: String },
  ActorCreated { actor_id: ActorId },
  ActorMessageSent { actor_id: ActorId },
  ActorTurnCompleted { actor_id: ActorId },
  ObjectCreated { object_id: ObjectId },
  ObjectUpdated { object_id: ObjectId },
  TransactionCommitted { transaction_id: TransactionId },
  RuntimeError { message: String },
}

impl RuntimeEventKind {
  pub fn name(&self) -> &'static str {
    match self {
      RuntimeEventKind::RuntimeCreated { .. } => "runtime.created",
      RuntimeEventKind::RuntimeShutdown { .. } => "runtime.shutdown",
      RuntimeEventKind::SourceResolved { .. } => "source.resolved",
      RuntimeEventKind::ModuleCompiled { .. } => "module.compiled",
      RuntimeEventKind::ModuleActivated { .. } => "module.activated",
      RuntimeEventKind::CapabilityGranted { .. } => "capability.granted",
      RuntimeEventKind::CapabilityRevoked { .. } => "capability.revoked",
      RuntimeEventKind::ProgramStarted { .. } => "program.started",
      RuntimeEventKind::ProgramCompleted { .. } => "program.completed",
      RuntimeEventKind::ProgramFailed { .. } => "program.failed",
      RuntimeEventKind::TaskCreated { .. } => "task.created",
      RuntimeEventKind::TaskStarted { .. } => "task.started",
      RuntimeEventKind::TaskCompleted { .. } => "task.completed",
      RuntimeEventKind::TaskFailed { .. } => "task.failed",
      RuntimeEventKind::ActorCreated { .. } => "actor.created",
      RuntimeEventKind::ActorMessageSent { .. } => "actor.message.sent",
      RuntimeEventKind::ActorTurnCompleted { .. } => "actor.turn.completed",
      RuntimeEventKind::ObjectCreated { .. } => "object.created",
      RuntimeEventKind::ObjectUpdated { .. } => "object.updated",
      RuntimeEventKind::TransactionCommitted { .. } => "transaction.committed",
      RuntimeEventKind::RuntimeError { .. } => "runtime.error",
    }
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeEvent {
  pub id: EventId,
  pub sequence: u64,
  pub timestamp_ms: Option<u64>,
  pub kind: RuntimeEventKind,
}

impl RuntimeEvent {
  pub fn new(id: EventId, sequence: u64, kind: RuntimeEventKind) -> Self {
    Self {
      id,
      sequence,
      timestamp_ms: None,
      kind,
    }
  }

  pub fn with_timestamp_ms(mut self, timestamp_ms: u64) -> Self {
    self.timestamp_ms = Some(timestamp_ms);
    self
  }

  pub fn name(&self) -> &'static str {
    self.kind.name()
  }

  pub fn validate(&self) -> MResult<()> {
    if self.id.is_zero() {
      return Err(MechError::new(
        InvalidRuntimeEventError {
          field: "id",
          reason: "must not be zero",
        },
        None,
      ));
    }

    Ok(())
  }
}

pub trait EventSink: std::fmt::Debug + Send {
  fn emit(&mut self, event: RuntimeEvent) -> MResult<EventId>;
}

#[derive(Debug, Clone)]
pub struct InvalidRuntimeEventError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidRuntimeEventError {
  fn name(&self) -> &str {
    "InvalidRuntimeEvent"
  }

  fn message(&self) -> String {
    format!("Invalid runtime event field `{}`: {}", self.field, self.reason)
  }
}