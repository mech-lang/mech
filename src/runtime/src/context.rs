//! Runtime execution context.
//!
//! A RuntimeContext is the per-operation view of the runtime.
//!
//! It should be passed into schedulers, actor turns, host calls, transactions,
//! and capability-checked operations. It should not own the whole runtime.
//!
//! The context tracks:
//!
//! - current runtime
//! - current subject
//! - optional task / actor / module version
//! - optional active transaction
//! - capabilities associated with this execution
//! - resource budget
//! - events accumulated during this operation

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::capability::{
  CapabilityContext, CapabilityRequest, Operation, Resource,
};

use crate::event::{
  RuntimeEvent,
};

use crate::id::{
  ActorId, CapabilityId, EventId, ModuleVersionId, ObjectId, RuntimeId, TaskId,
  TransactionId,
};

use crate::actor::ActorTurn;

use crate::store::MessageRecord;

use crate::{
  SourceContextBase, SourceContextCapability, SourceContextCapabilityScope,
  SourceContextDeclaration, SourceScope,
};

// -----------------------------------------------------------------------------
// Runtime Context Registry
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextCapability {
  pub operation: String,
  pub scope: RuntimeContextCapabilityScope,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeContextCapabilityScope {
  Path(String),
  Wildcard,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RuntimeContextBase {
  ResourceUri(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeContextBinding {
  pub name: String,
  pub base: RuntimeContextBase,
  pub capabilities: Vec<RuntimeContextCapability>,
  pub scope: SourceScope,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct RuntimeContextRegistry {
  bindings: HashMap<String, RuntimeContextBinding>,
}

impl RuntimeContextRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn from_declarations(
    scope: SourceScope,
    declarations: &[SourceContextDeclaration],
  ) -> MResult<Self> {
    let mut registry = Self::new();
    for declaration in declarations {
      let binding = RuntimeContextBinding::from_source(scope.clone(), declaration)?;
      registry.insert(binding)?;
    }
    Ok(registry)
  }

  pub fn insert(&mut self, binding: RuntimeContextBinding) -> MResult<()> {
    if self.bindings.contains_key(&binding.name) {
      return Err(MechError::new(
        RuntimeContextDuplicateBinding { name: binding.name },
        None,
      ));
    }
    self.bindings.insert(binding.name.clone(), binding);
    Ok(())
  }

  pub fn get(&self, name: &str) -> Option<&RuntimeContextBinding> {
    self.bindings.get(name)
  }

  pub fn contains(&self, name: &str) -> bool {
    self.bindings.contains_key(name)
  }

  pub fn len(&self) -> usize {
    self.bindings.len()
  }

  pub fn is_empty(&self) -> bool {
    self.bindings.is_empty()
  }
}

impl RuntimeContextBinding {
  pub fn from_source(
    scope: SourceScope,
    declaration: &SourceContextDeclaration,
  ) -> MResult<Self> {
    if declaration.name.is_empty() {
      return Err(MechError::new(
        RuntimeContextInvalidBinding {
          name: declaration.name.clone(),
          reason: "context name cannot be empty".to_string(),
        },
        None,
      ));
    }

    let base = match &declaration.base {
      SourceContextBase::ResourceUri(uri) => {
        if uri.is_empty() {
          return Err(MechError::new(
            RuntimeContextInvalidBinding {
              name: declaration.name.clone(),
              reason: "resource URI cannot be empty".to_string(),
            },
            None,
          ));
        }
        RuntimeContextBase::ResourceUri(uri.clone())
      }
      SourceContextBase::Context(name) => {
        return Err(MechError::new(
          RuntimeContextDerivedBaseUnsupported {
            name: declaration.name.clone(),
            base: name.clone(),
          },
          None,
        ));
      }
    };

    let mut capabilities = Vec::with_capacity(declaration.capabilities.len());
    for capability in &declaration.capabilities {
      capabilities.push(runtime_context_capability_from_source(
        &declaration.name,
        capability,
      )?);
    }

    Ok(Self {
      name: declaration.name.clone(),
      base,
      capabilities,
      scope,
    })
  }
}

fn runtime_context_capability_from_source(
    context_name: &str,
    capability: &SourceContextCapability,
) -> MResult<RuntimeContextCapability> {
  if capability.operation.is_empty() {
    return Err(MechError::new(
      RuntimeContextInvalidBinding {
        name: context_name.to_string(),
        reason: "capability operation cannot be empty".to_string(),
      },
      None,
    ));
  }

  let scope = match &capability.scope {
    SourceContextCapabilityScope::Path(path) => {
      RuntimeContextCapabilityScope::Path(path.clone())
    }
    SourceContextCapabilityScope::Wildcard => RuntimeContextCapabilityScope::Wildcard,
  };

  Ok(RuntimeContextCapability {
    operation: capability.operation.clone(),
    scope,
  })
}

#[derive(Debug, Clone)]
pub struct RuntimeContextDuplicateBinding {
  pub name: String,
}

impl MechErrorKind for RuntimeContextDuplicateBinding {
  fn name(&self) -> &str {
    "RuntimeContextDuplicateBinding"
  }

  fn message(&self) -> String {
    format!("runtime context `{}` is declared more than once", self.name)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeContextInvalidBinding {
  pub name: String,
  pub reason: String,
}

impl MechErrorKind for RuntimeContextInvalidBinding {
  fn name(&self) -> &str {
    "RuntimeContextInvalidBinding"
  }

  fn message(&self) -> String {
    format!("invalid runtime context `{}`: {}", self.name, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeContextDerivedBaseUnsupported {
  pub name: String,
  pub base: String,
}

impl MechErrorKind for RuntimeContextDerivedBaseUnsupported {
  fn name(&self) -> &str {
    "RuntimeContextDerivedBaseUnsupported"
  }

  fn message(&self) -> String {
    format!(
      "runtime context `{}` derives from `{}`, but derived context bases are not supported yet",
      self.name, self.base,
    )
  }
}



// -----------------------------------------------------------------------------
// Runtime Context
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RuntimeContext {
  pub runtime: RuntimeId,
  pub subject: String,
  pub task: Option<TaskId>,
  pub actor: Option<ActorId>,
  pub access: AccessSet,
  pub module_version: Option<ModuleVersionId>,
  pub transaction: Option<TransactionId>,
  pub capabilities: Vec<CapabilityId>,
  pub budget: ResourceBudget,
  pub events: Vec<RuntimeEvent>,
  pub actor_message: Option<MessageRecord>,
  pub actor_state: Option<ObjectId>,
}

impl RuntimeContext {
  pub fn new(runtime: RuntimeId, subject: impl Into<String>) -> Self {
    Self {
      runtime,
      subject: subject.into(),
      task: None,
      actor: None,
      access: AccessSet::new(),
      module_version: None,
      transaction: None,
      capabilities: Vec::new(),
      budget: ResourceBudget::default(),
      events: Vec::new(),
      actor_message: None,
      actor_state: None,
    }
  }

  pub fn runtime(runtime: RuntimeId) -> Self {
    Self::new(runtime, format!("runtime:{}", runtime))
  }

  pub fn with_subject(mut self, subject: impl Into<String>) -> Self {
    self.subject = subject.into();
    self
  }

  pub fn with_task(mut self, task: TaskId) -> Self {
    self.task = Some(task);
    self
  }

  pub fn with_actor(mut self, actor: ActorId) -> Self {
    self.actor = Some(actor);
    self
  }

  pub fn with_module_version(mut self, module_version: ModuleVersionId) -> Self {
    self.module_version = Some(module_version);
    self
  }

  pub fn with_transaction(mut self, transaction: TransactionId) -> Self {
    self.transaction = Some(transaction);
    self
  }

  pub fn with_capabilities(mut self, capabilities: Vec<CapabilityId>) -> Self {
    self.capabilities = capabilities;
    self
  }

  pub fn with_budget(mut self, budget: ResourceBudget) -> Self {
    self.budget = budget;
    self
  }

  pub fn validate(&self) -> MResult<()> {
    if self.runtime.is_zero() {
      return invalid_context("runtime", "must not be zero");
    }

    if self.subject.trim().is_empty() {
      return invalid_context("subject", "must not be empty");
    }

    Ok(())
  }

  pub fn is_runtime_subject(&self) -> bool {
    self.subject == format!("runtime:{}", self.runtime)
  }

  pub fn is_actor_turn(&self) -> bool {
    self.actor.is_some()
  }

  pub fn is_task_execution(&self) -> bool {
    self.task.is_some()
  }

  pub fn is_transactional(&self) -> bool {
    self.transaction.is_some()
  }

  pub fn capability_context(&self) -> CapabilityContext {
    CapabilityContext {
      local: true,
      bytes: None,
      items: None,
      duration_ms: None,
    }
  }

  pub fn capability_request(
    &self,
    operation: &dyn Operation,
    resource: &dyn Resource,
  ) -> CapabilityRequest {
    CapabilityRequest {
      subject: self.subject.clone(),
      operation: operation.key().to_string(),
      resource: resource.key().to_string(),
      context: self.capability_context(),
    }
  }

  pub fn push_event(&mut self, event: RuntimeEvent) {
    self.events.push(event);
  }

  pub fn drain_events(&mut self) -> Vec<RuntimeEvent> {
    std::mem::take(&mut self.events)
  }

  pub fn has_capability(&self, capability: CapabilityId) -> bool {
    self.capabilities.contains(&capability)
  }

  pub fn add_capability(&mut self, capability: CapabilityId) {
    if !self.capabilities.contains(&capability) {
      self.capabilities.push(capability);
    }
  }

  pub fn remove_capability(&mut self, capability: CapabilityId) {
    self.capabilities.retain(|id| *id != capability);
  }

  pub fn charge_step(&mut self) -> MResult<()> {
    self.budget.charge_steps(1)
  }

  pub fn charge_steps(&mut self, steps: u64) -> MResult<()> {
    self.budget.charge_steps(steps)
  }

  pub fn charge_bytes(&mut self, bytes: u64) -> MResult<()> {
    self.budget.charge_bytes(bytes)
  }

  pub fn charge_items(&mut self, items: u64) -> MResult<()> {
    self.budget.charge_items(items)
  }

  pub fn charge_messages(&mut self, messages: u64) -> MResult<()> {
    self.budget.charge_messages(messages)
  }

  pub fn record_read(&mut self, object: ObjectId) {
    self.access.read(object);
  }

  pub fn record_write(&mut self, object: ObjectId) {
    self.access.write(object);
  }

  pub fn emitted_event_ids(&self) -> Vec<EventId> {
    self.events.iter().map(|event| event.id).collect()
  }

  pub fn bind_actor_turn(&mut self, turn: &ActorTurn) {
    self.actor = Some(turn.actor);
    self.subject = turn.subject.clone();
    self.actor_message = Some(turn.message.clone());
    self.actor_state = turn.state;
  }

  pub fn actor_message(&self) -> Option<&MessageRecord> {
    self.actor_message.as_ref()
  }

  pub fn actor_message_kind(&self) -> Option<&str> {
    self.actor_message.as_ref().map(|message| message.kind.as_str())
  }

  pub fn actor_message_payload(&self) -> Option<&[u8]> {
    self.actor_message.as_ref().map(|message| message.payload.as_slice())
  }

  pub fn actor_state(&self) -> Option<ObjectId> {
    self.actor_state
  }

}

// -----------------------------------------------------------------------------
// Runtime Context Builder
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct RuntimeContextBuilder {
  runtime: RuntimeId,
  subject: Option<String>,
  task: Option<TaskId>,
  actor: Option<ActorId>,
  module_version: Option<ModuleVersionId>,
  transaction: Option<TransactionId>,
  capabilities: Vec<CapabilityId>,
  budget: ResourceBudget,
  access: AccessSet,
  actor_message: Option<MessageRecord>,
  actor_state: Option<ObjectId>,
}

impl RuntimeContextBuilder {
  pub fn new(runtime: RuntimeId) -> Self {
    Self {
      runtime,
      subject: None,
      task: None,
      actor: None,
      module_version: None,
      transaction: None,
      capabilities: Vec::new(),
      budget: ResourceBudget::default(),
      access: AccessSet::new(),
      actor_message: None,
      actor_state: None,
    }
  }

  pub fn actor_message(mut self, message: MessageRecord) -> Self {
    self.actor_message = Some(message);
    self
  }

  pub fn actor_state(mut self, state: ObjectId) -> Self {
    self.actor_state = Some(state);
    self
  }

  pub fn subject(mut self, subject: impl Into<String>) -> Self {
    self.subject = Some(subject.into());
    self
  }

  pub fn task(mut self, task: TaskId) -> Self {
    self.task = Some(task);
    self
  }

  pub fn actor(mut self, actor: ActorId) -> Self {
    self.actor = Some(actor);
    self
  }

  pub fn module_version(mut self, module_version: ModuleVersionId) -> Self {
    self.module_version = Some(module_version);
    self
  }

  pub fn transaction(mut self, transaction: TransactionId) -> Self {
    self.transaction = Some(transaction);
    self
  }

  pub fn capabilities(mut self, capabilities: Vec<CapabilityId>) -> Self {
    self.capabilities = capabilities;
    self
  }

  pub fn budget(mut self, budget: ResourceBudget) -> Self {
    self.budget = budget;
    self
  }

  pub fn access(mut self, access: AccessSet) -> Self {
    self.access = access;
    self
  }

  pub fn build(self) -> MResult<RuntimeContext> {
    let subject = self.subject.unwrap_or_else(|| {
      if let Some(actor) = self.actor {
        format!("actor:{}", actor)
      } else if let Some(task) = self.task {
        format!("task:{}", task)
      } else {
        format!("runtime:{}", self.runtime)
      }
    });

    let context = RuntimeContext {
      runtime: self.runtime,
      subject,
      task: self.task,
      actor: self.actor,
      module_version: self.module_version,
      transaction: self.transaction,
      capabilities: self.capabilities,
      budget: self.budget,
      access: self.access,
      events: Vec::new(),
      actor_message: self.actor_message,
      actor_state: self.actor_state,
    };

    context.validate()?;
    Ok(context)
  }
}

// -----------------------------------------------------------------------------
// Resource Budget
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceBudget {
  pub max_steps: Option<u64>,
  pub used_steps: u64,

  pub max_bytes: Option<u64>,
  pub used_bytes: u64,

  pub max_items: Option<u64>,
  pub used_items: u64,

  pub max_messages: Option<u64>,
  pub used_messages: u64,
}

impl Default for ResourceBudget {
  fn default() -> Self {
    Self {
      max_steps: None,
      used_steps: 0,
      max_bytes: None,
      used_bytes: 0,
      max_items: None,
      used_items: 0,
      max_messages: None,
      used_messages: 0,
    }
  }
}

impl ResourceBudget {
  pub fn unbounded() -> Self {
    Self::default()
  }

  pub fn with_max_steps(mut self, max_steps: u64) -> Self {
    self.max_steps = Some(max_steps);
    self
  }

  pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
    self.max_bytes = Some(max_bytes);
    self
  }

  pub fn with_max_items(mut self, max_items: u64) -> Self {
    self.max_items = Some(max_items);
    self
  }

  pub fn with_max_messages(mut self, max_messages: u64) -> Self {
    self.max_messages = Some(max_messages);
    self
  }

  pub fn remaining_steps(&self) -> Option<u64> {
    self.max_steps
      .map(|max| max.saturating_sub(self.used_steps))
  }

  pub fn remaining_bytes(&self) -> Option<u64> {
    self.max_bytes
      .map(|max| max.saturating_sub(self.used_bytes))
  }

  pub fn remaining_items(&self) -> Option<u64> {
    self.max_items
      .map(|max| max.saturating_sub(self.used_items))
  }

  pub fn remaining_messages(&self) -> Option<u64> {
    self.max_messages
      .map(|max| max.saturating_sub(self.used_messages))
  }

  pub fn charge_steps(&mut self, steps: u64) -> MResult<()> {
    self.used_steps = checked_charge(
      "steps",
      self.used_steps,
      self.max_steps,
      steps,
    )?;

    Ok(())
  }

  pub fn charge_bytes(&mut self, bytes: u64) -> MResult<()> {
    self.used_bytes = checked_charge(
      "bytes",
      self.used_bytes,
      self.max_bytes,
      bytes,
    )?;

    Ok(())
  }

  pub fn charge_items(&mut self, items: u64) -> MResult<()> {
    self.used_items = checked_charge(
      "items",
      self.used_items,
      self.max_items,
      items,
    )?;

    Ok(())
  }

  pub fn charge_messages(&mut self, messages: u64) -> MResult<()> {
    self.used_messages = checked_charge(
      "messages",
      self.used_messages,
      self.max_messages,
      messages,
    )?;

    Ok(())
  }
}

fn checked_charge(
  resource: &'static str,
  used: u64,
  max: Option<u64>,
  amount: u64,
) -> MResult<u64> {
  let Some(next) = used.checked_add(amount) else {
    return Err(MechError::new(
      ResourceBudgetExceededError {
        resource,
        max: None,
        requested: amount,
        used,
      },
      None,
    ));
  };

  if let Some(max) = max {
    if next > max {
      return Err(MechError::new(
        ResourceBudgetExceededError {
          resource,
          max: Some(max),
          requested: amount,
          used,
        },
        None,
      ));
    }
  }

  Ok(next)
}

// -----------------------------------------------------------------------------
// Access Sets
// -----------------------------------------------------------------------------

/// Records object reads and writes performed during an operation.
///
/// This is intentionally small. The transaction layer can later use it to build
/// TransactionRecord read/write sets.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct AccessSet {
  pub reads: Vec<ObjectId>,
  pub writes: Vec<ObjectId>,
}

impl AccessSet {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn read(&mut self, object: ObjectId) {
    if !self.reads.contains(&object) {
      self.reads.push(object);
    }
  }

  pub fn write(&mut self, object: ObjectId) {
    if !self.writes.contains(&object) {
      self.writes.push(object);
    }
  }

  pub fn clear(&mut self) {
    self.reads.clear();
    self.writes.clear();
  }

  pub fn is_empty(&self) -> bool {
    self.reads.is_empty() && self.writes.is_empty()
  }
}

// -----------------------------------------------------------------------------
// Turn Outcome
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeTurnOutcome {
  pub task: Option<TaskId>,
  pub actor: Option<ActorId>,
  pub transaction: Option<TransactionId>,
  pub events: Vec<EventId>,
  pub access: AccessSet,
}

impl RuntimeTurnOutcome {
  pub fn new() -> Self {
    Self {
      task: None,
      actor: None,
      transaction: None,
      events: Vec::new(),
      access: AccessSet::new(),
    }
  }

  pub fn with_task(mut self, task: TaskId) -> Self {
    self.task = Some(task);
    self
  }

  pub fn with_actor(mut self, actor: ActorId) -> Self {
    self.actor = Some(actor);
    self
  }

  pub fn with_transaction(mut self, transaction: TransactionId) -> Self {
    self.transaction = Some(transaction);
    self
  }

  pub fn with_events(mut self, events: Vec<EventId>) -> Self {
    self.events = events;
    self
  }

  pub fn with_access(mut self, access: AccessSet) -> Self {
    self.access = access;
    self
  }
}

impl Default for RuntimeTurnOutcome {
  fn default() -> Self {
    Self::new()
  }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidRuntimeContextError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidRuntimeContextError {
  fn name(&self) -> &str {
    "InvalidRuntimeContext"
  }

  fn message(&self) -> String {
    format!("Invalid runtime context field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_context<T>(
  field: &'static str,
  reason: &'static str,
) -> MResult<T> {
  Err(MechError::new(
    InvalidRuntimeContextError { field, reason },
    None,
  ))
}

#[derive(Debug, Clone)]
pub struct ResourceBudgetExceededError {
  pub resource: &'static str,
  pub max: Option<u64>,
  pub requested: u64,
  pub used: u64,
}

impl MechErrorKind for ResourceBudgetExceededError {
  fn name(&self) -> &str {
    "ResourceBudgetExceeded"
  }

  fn message(&self) -> String {
    match self.max {
      Some(max) => format!(
        "Resource budget exceeded for `{}`: used {}, requested {}, max {}",
        self.resource, self.used, self.requested, max
      ),
      None => format!(
        "Resource budget overflow for `{}`: used {}, requested {}",
        self.resource, self.used, self.requested
      ),
    }
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn context_builder_defaults_to_runtime_subject() {
    let context = RuntimeContextBuilder::new(RuntimeId(1))
      .build()
      .unwrap();

    assert_eq!(context.subject, "runtime:00000000000000000000000000000001");
  }

  #[test]
  fn context_builder_prefers_actor_subject() {
    let context = RuntimeContextBuilder::new(RuntimeId(1))
      .actor(ActorId(2))
      .build()
      .unwrap();

    assert_eq!(context.subject, "actor:00000000000000000000000000000002");
  }

  #[test]
  fn context_builder_prefers_task_subject_when_no_actor() {
    let context = RuntimeContextBuilder::new(RuntimeId(1))
      .task(TaskId(2))
      .build()
      .unwrap();

    assert_eq!(context.subject, "task:00000000000000000000000000000002");
  }

  #[test]
  fn context_tracks_capabilities() {
    let mut context = RuntimeContext::new(RuntimeId(1), "task:1");

    assert!(!context.has_capability(CapabilityId(7)));

    context.add_capability(CapabilityId(7));

    assert!(context.has_capability(CapabilityId(7)));

    context.remove_capability(CapabilityId(7));

    assert!(!context.has_capability(CapabilityId(7)));
  }

  #[test]
  fn budget_charges_until_limit() {
    let mut budget = ResourceBudget::default()
      .with_max_steps(2);

    assert!(budget.charge_steps(1).is_ok());
    assert!(budget.charge_steps(1).is_ok());
    assert!(budget.charge_steps(1).is_err());
  }

  #[test]
  fn access_set_deduplicates_reads_and_writes() {
    let mut access = AccessSet::new();

    access.read(ObjectId(1));
    access.read(ObjectId(1));
    access.write(ObjectId(2));
    access.write(ObjectId(2));

    assert_eq!(access.reads, vec![ObjectId(1)]);
    assert_eq!(access.writes, vec![ObjectId(2)]);
  }

  #[test]
  fn context_drains_events() {
    use crate::event::RuntimeEventKind;
    let mut context = RuntimeContext::new(RuntimeId(1), "task:1");

    context.push_event(RuntimeEvent::new(
      EventId(1),
      0,
      RuntimeEventKind::RuntimeError {
        message: "test".to_string(),
      },
    ));

    let events = context.drain_events();

    assert_eq!(events.len(), 1);
    assert!(context.events.is_empty());
  }
}