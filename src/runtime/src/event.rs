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
  TransactionId, MessageId
};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeEventKind {
  RuntimeCreated { runtime_id: RuntimeId },
  RuntimeShutdown { runtime_id: RuntimeId },
  RuntimeTickStarted,
  RuntimeTickCompleted { work_count: u64 },
  RuntimeError { message: String },

  SourceResolved { canonical_uri: String },
  SourceChanged { canonical_uri: String },
  SourceReloaded { canonical_uri: String },
  SourceResolveFailed { specifier: String, message: String },

  ModuleCompiled { module_version: ModuleVersionId },
  ModuleCompileFailed { canonical_uri: String, message: String },
  ModuleActivated { module_version: ModuleVersionId },
  ModuleActivationFailed { module_version: ModuleVersionId, message: String },
  ModuleExecutionStarted { module_version: ModuleVersionId },
  ModuleExecutionCompleted { module_version: ModuleVersionId },
  ModuleExecutionFailed { module_version: ModuleVersionId, message: String },
  ModuleImportLinked {
    importer: ModuleVersionId,
    dependency: ModuleVersionId,
    specifier: String,
  },

  CapabilityGranted { capability_id: CapabilityId },
  CapabilityRevoked { capability_id: CapabilityId },
  CapabilityDenied {
    subject: String,
    operation: String,
    resource: String,
  },

  ProgramStarted { task_id: Option<TaskId> },
  ProgramCompleted { task_id: Option<TaskId> },
  ProgramFailed { task_id: Option<TaskId>, message: String },
  ProgramProfiled {
    task_id: Option<TaskId>,
    duration_ns: u128,
  },

  TaskCreated { task_id: TaskId },
  TaskStarted { task_id: TaskId },
  TaskCompleted { task_id: TaskId },
  TaskFailed { task_id: TaskId, message: String },

  ActorCreated { actor_id: ActorId },
  ActorMessageSent { actor_id: ActorId, message_id: MessageId },
  ActorTurnStarted { actor_id: ActorId },
  ActorTurnCompleted { actor_id: ActorId },
  ActorTurnFailed { actor_id: ActorId, message: String },

  ObjectCreated { object_id: ObjectId },
  ObjectUpdated { object_id: ObjectId },

  TransactionStarted { transaction_id: TransactionId },
  TransactionCommitted { transaction_id: TransactionId },
  TransactionAborted { transaction_id: TransactionId, message: String },

  SchedulerWorkQueued { work: String },
  SchedulerWorkStarted { work: String },
  SchedulerWorkCompleted { work: String },
  SchedulerWorkFailed { work: String, message: String },

  HostCallStarted { name: String },
  HostCallCompleted { name: String },
  HostCallFailed { name: String, message: String },
}

impl RuntimeEventKind {
  pub fn name(&self) -> &'static str {
    match self {
      RuntimeEventKind::RuntimeCreated { .. } => ":runtime/created",
      RuntimeEventKind::RuntimeShutdown { .. } => ":runtime/shutdown",
      RuntimeEventKind::RuntimeTickStarted => ":runtime/tick/started",
      RuntimeEventKind::RuntimeTickCompleted { .. } => ":runtime/tick/completed",
      RuntimeEventKind::SourceResolved { .. } => ":source/resolved",
      RuntimeEventKind::SourceChanged { .. } => ":source/changed",
      RuntimeEventKind::SourceReloaded { .. } => ":source/reloaded",
      RuntimeEventKind::SourceResolveFailed { .. } => ":source/resolve/failed",
      RuntimeEventKind::ModuleCompiled { .. } => ":module/compiled",
      RuntimeEventKind::ModuleCompileFailed { .. } => ":module/compile/failed",
      RuntimeEventKind::ModuleActivated { .. } => ":module/activated",
      RuntimeEventKind::ModuleActivationFailed { .. } => ":module/activation/failed",
      RuntimeEventKind::ModuleExecutionStarted { .. } => ":module/execution/started",
      RuntimeEventKind::ModuleExecutionCompleted { .. } => ":module/execution/completed",
      RuntimeEventKind::ModuleExecutionFailed { .. } => ":module/execution/failed",
      RuntimeEventKind::ModuleImportLinked { .. } => ":module/import/linked",
      RuntimeEventKind::CapabilityGranted { .. } => ":capability/granted",
      RuntimeEventKind::CapabilityRevoked { .. } => ":capability/revoked",
      RuntimeEventKind::CapabilityDenied { .. } => ":capability/denied",
      RuntimeEventKind::ProgramStarted { .. } => ":program/started",
      RuntimeEventKind::ProgramCompleted { .. } => ":program/completed",
      RuntimeEventKind::ProgramFailed { .. } => ":program/failed",
      RuntimeEventKind::ProgramProfiled { .. } => ":program/profiled",
      RuntimeEventKind::TaskCreated { .. } => ":task/created",
      RuntimeEventKind::TaskStarted { .. } => ":task/started",
      RuntimeEventKind::TaskCompleted { .. } => ":task/completed",
      RuntimeEventKind::TaskFailed { .. } => ":task/failed",
      RuntimeEventKind::ActorCreated { .. } => ":actor/created",
      RuntimeEventKind::ActorMessageSent { .. } => ":actor/message/sent",
      RuntimeEventKind::ActorTurnStarted { .. } => ":actor/turn/started",
      RuntimeEventKind::ActorTurnCompleted { .. } => ":actor/turn/completed",
      RuntimeEventKind::ActorTurnFailed { .. } => ":actor/turn/failed",
      RuntimeEventKind::ObjectCreated { .. } => ":object/created",
      RuntimeEventKind::ObjectUpdated { .. } => ":object/updated",
      RuntimeEventKind::TransactionStarted { .. } => ":transaction/started",
      RuntimeEventKind::TransactionCommitted { .. } => ":transaction/committed",
      RuntimeEventKind::TransactionAborted { .. } => ":transaction/aborted",
      RuntimeEventKind::SchedulerWorkQueued { .. } => ":scheduler/work/queued",
      RuntimeEventKind::SchedulerWorkStarted { .. } => ":scheduler/work/started",
      RuntimeEventKind::SchedulerWorkCompleted { .. } => ":scheduler/work/completed",
      RuntimeEventKind::SchedulerWorkFailed { .. } => ":scheduler/work/failed",
      RuntimeEventKind::RuntimeError { .. } => ":runtime/error",
      RuntimeEventKind::HostCallStarted { .. } => ":host/call/started",
      RuntimeEventKind::HostCallCompleted { .. } => ":host/call/completed",
      RuntimeEventKind::HostCallFailed { .. } => ":host/call/failed",
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
