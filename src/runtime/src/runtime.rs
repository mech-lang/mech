//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It wraps the current
//! program/interpreter layer and owns the system-level components:
//!
//! - ID generator
//! - store
//! - capability kernel
//! - module resolver
//! - runtime config
//!
//! This file is intentionally conservative. It does not try to replace the
//! interpreter. It creates the system boundary that v0.4 can grow into.

use std::collections::HashMap;
use std::sync::Arc;

use mech_core::{
  MResult, MechError, MechErrorKind, MechSourceCode, Value,
};

use mech_program::{
  MechProgram, MechProgramConfig, MechProgramEnvironment,
};

use crate::capability::{
  Capability, CapabilityGrant, CapabilityKernel, CapabilityRequest,
  CapabilityRevocation, BasicCapabilityKernel,
};

use crate::config::RuntimeConfig;

use crate::id::{
  module_id, module_version_id, ActorId, CapabilityId, DefaultIdGenerator,
  EventId, IdGenerator, ModuleId, ModuleVersionId, ObjectId, RuntimeId,
  TaskId, TransactionId,
};

use crate::store::{
  ActorRecord, InMemoryStore, MechStore, MessageId, MessageRecord,
  ModuleRecord, ModuleVersionRecord, ObjectRecord, RuntimeEvent, TaskRecord,
  TaskStatus, TransactionRecord,
};

// -----------------------------------------------------------------------------
// Module Resolution
// -----------------------------------------------------------------------------

/// Runtime module resolver.
///
/// This is trait-based because module loading is host policy:
///
/// - filesystem
/// - database
/// - memory
/// - embedded resources
/// - network bundle
/// - editor buffer
/// - host application callback
pub trait ModuleResolver: std::fmt::Debug + Send {
  fn resolve(&self, name: &str) -> MResult<Option<ResolvedModule>>;
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResolvedModule {
  pub name: String,
  pub source: String,
}

impl ResolvedModule {
  pub fn new(name: impl Into<String>, source: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      source: source.into(),
    }
  }

  pub fn validate(&self) -> MResult<()> {
    if self.name.trim().is_empty() {
      return invalid_runtime("module.name", "must not be empty");
    }

    Ok(())
  }
}

/// Simple in-memory resolver for tests, examples, and the first runtime shell.
#[derive(Clone, Debug, Default)]
pub struct InMemoryModuleResolver {
  modules: HashMap<String, String>,
}

impl InMemoryModuleResolver {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn insert(&mut self, name: impl Into<String>, source: impl Into<String>) {
    self.modules.insert(name.into(), source.into());
  }
}

impl ModuleResolver for InMemoryModuleResolver {
  fn resolve(&self, name: &str) -> MResult<Option<ResolvedModule>> {
    Ok(
      self
        .modules
        .get(name)
        .map(|source| ResolvedModule::new(name, source.clone())),
    )
  }
}

// -----------------------------------------------------------------------------
// Runtime Builder
// -----------------------------------------------------------------------------

/// Builder for MechRuntime.
///
/// Concrete implementation choices live here, not in RuntimeConfig.
pub struct RuntimeBuilder {
  config: RuntimeConfig,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  module_resolver: Box<dyn ModuleResolver>,
}

impl std::fmt::Debug for RuntimeBuilder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RuntimeBuilder")
      .field("config", &self.config)
      .field("id_generator", &"<dyn IdGenerator>")
      .field("store", &"<dyn MechStore>")
      .field("capability_kernel", &"<dyn CapabilityKernel>")
      .field("module_resolver", &"<dyn ModuleResolver>")
      .finish()
  }
}

impl Default for RuntimeBuilder {
  fn default() -> Self {
    Self {
      config: RuntimeConfig::default(),
      id_generator: Box::new(DefaultIdGenerator::new()),
      store: Box::new(InMemoryStore::new()),
      capability_kernel: Box::new(BasicCapabilityKernel::new()),
      module_resolver: Box::new(InMemoryModuleResolver::new()),
    }
  }
}

impl RuntimeBuilder {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn config(mut self, config: RuntimeConfig) -> Self {
    self.config = config;
    self
  }

  pub fn id_generator(mut self, id_generator: impl IdGenerator + 'static) -> Self {
    self.id_generator = Box::new(id_generator);
    self
  }

  pub fn store(mut self, store: impl MechStore + 'static) -> Self {
    self.store = Box::new(store);
    self
  }

  pub fn capability_kernel(
    mut self,
    capability_kernel: impl CapabilityKernel + 'static,
  ) -> Self {
    self.capability_kernel = Box::new(capability_kernel);
    self
  }

  pub fn module_resolver(
    mut self,
    module_resolver: impl ModuleResolver + 'static,
  ) -> Self {
    self.module_resolver = Box::new(module_resolver);
    self
  }

  pub fn build(mut self) -> MResult<MechRuntime> {
    self.config.validate()?;

    let runtime_id = self.id_generator.runtime_id();

    let program_config = MechProgramConfig {
      name: self.config.name.clone(),
      environment: MechProgramEnvironment {
        trace_enabled: self.config.diagnostics.trace_enabled,
        debug_enabled: self.config.diagnostics.debug_enabled,
        profile_enabled: self.config.diagnostics.profile_enabled,
        rounds_per_step: self
          .config
          .limits
          .max_steps_per_turn
          .unwrap_or(10_000) as usize,
      },
    };

    let mut runtime = MechRuntime {
      id: runtime_id,
      config: self.config,
      program: MechProgram::new(program_config),
      id_generator: self.id_generator,
      store: self.store,
      capability_kernel: self.capability_kernel,
      module_resolver: self.module_resolver,
    };

    let event_id = runtime.next_event_id();
    runtime.append_event(RuntimeEvent::new(event_id, "runtime.created"))?;

    Ok(runtime)
  }
}

// -----------------------------------------------------------------------------
// MechRuntime
// -----------------------------------------------------------------------------

pub struct MechRuntime {
  id: RuntimeId,
  config: RuntimeConfig,
  program: MechProgram,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  module_resolver: Box<dyn ModuleResolver>,
}

impl std::fmt::Debug for MechRuntime {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MechRuntime")
      .field("id", &self.id)
      .field("config", &self.config)
      .field("program", &"<MechProgram>")
      .field("id_generator", &"<dyn IdGenerator>")
      .field("store", &"<dyn MechStore>")
      .field("capability_kernel", &"<dyn CapabilityKernel>")
      .field("module_resolver", &"<dyn ModuleResolver>")
      .finish()
  }
}

impl MechRuntime {
  pub fn builder() -> RuntimeBuilder {
    RuntimeBuilder::new()
  }

  pub fn new(config: RuntimeConfig) -> MResult<Self> {
    RuntimeBuilder::new().config(config).build()
  }

  pub fn id(&self) -> RuntimeId {
    self.id
  }

  pub fn config(&self) -> &RuntimeConfig {
    &self.config
  }

  pub fn program(&self) -> &MechProgram {
    &self.program
  }

  pub fn program_mut(&mut self) -> &mut MechProgram {
    &mut self.program
  }

  pub fn store(&self) -> &dyn MechStore {
    self.store.as_ref()
  }

  pub fn store_mut(&mut self) -> &mut dyn MechStore {
    self.store.as_mut()
  }

  pub fn capability_kernel(&self) -> &dyn CapabilityKernel {
    self.capability_kernel.as_ref()
  }

  pub fn capability_kernel_mut(&mut self) -> &mut dyn CapabilityKernel {
    self.capability_kernel.as_mut()
  }

  pub fn module_resolver(&self) -> &dyn ModuleResolver {
    self.module_resolver.as_ref()
  }

  pub fn module_resolver_mut(&mut self) -> &mut dyn ModuleResolver {
    self.module_resolver.as_mut()
  }

  // ---------------------------------------------------------------------------
  // ID helpers
  // ---------------------------------------------------------------------------

  pub fn next_object_id(&mut self) -> ObjectId {
    self.id_generator.object_id()
  }

  pub fn next_actor_id(&mut self) -> ActorId {
    self.id_generator.actor_id()
  }

  pub fn next_task_id(&mut self) -> TaskId {
    self.id_generator.task_id()
  }

  pub fn next_capability_id(&mut self) -> CapabilityId {
    self.id_generator.capability_id()
  }

  pub fn next_transaction_id(&mut self) -> TransactionId {
    self.id_generator.transaction_id()
  }

  pub fn next_event_id(&mut self) -> EventId {
    self.id_generator.event_id()
  }

  /// Local fallback for message IDs.
  ///
  /// If MessageId is moved into id.rs, replace this with IdGenerator::message_id.
  pub fn next_message_id(&mut self) -> MessageId {
    MessageId(self.id_generator.event_id().as_u128())
  }

  // ---------------------------------------------------------------------------
  // Program execution
  // ---------------------------------------------------------------------------

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "program.started")
        .with_subject(format!("runtime:{}", self.id)),
    )?;

    let result = self.program.run_string(source);

    match &result {
      Ok(_) => {
        let event_id = self.next_event_id();
        self.append_event(
          RuntimeEvent::new(event_id, "program.completed")
            .with_subject(format!("runtime:{}", self.id)),
        )?;
      }
      Err(error) => {
        let event_id = self.next_event_id();
        self.append_event(
          RuntimeEvent::new(event_id, "program.failed")
            .with_subject(format!("runtime:{}", self.id))
            .with_message(format!("{:?}", error)),
        )?;
      }
    }

    result
  }

  pub fn run_module(&mut self, version: ModuleVersionId) -> MResult<Value> {
    let Some(record) = self.store.get_module_version(version)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "module_version",
          id: version.to_string(),
        },
        None,
      ));
    };

    let Some(source) = record.source else {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "run_module",
          reason: "module version has no source".to_string(),
        },
        None,
      ));
    };

    match source {
      MechSourceCode::String(source) => self.run_string(&source),
      other => self.program.run_source(&other),
    }
  }

  // ---------------------------------------------------------------------------
  // Modules
  // ---------------------------------------------------------------------------

  /// Store a module record if it does not exist yet.
  pub fn ensure_module(&mut self, name: &str) -> MResult<ModuleId> {
    if let Some(module) = self.store.find_module_by_name(name)? {
      return Ok(module.id);
    }

    let id = module_id(name);
    let module = ModuleRecord::new(id, name);
    self.store.put_module(module)
  }

  /// Compile/store a source module version.
  ///
  /// This does not yet run the full compiler. It creates a version record using
  /// deterministic module version identity.
  pub fn put_source_module(
    &mut self,
    name: &str,
    source: &str,
    compiler_version: &str,
  ) -> MResult<ModuleVersionId> {
    let module = self.ensure_module(name)?;
    let version_id = module_version_id(source, compiler_version, &[]);

    if self.store.get_module_version(version_id)?.is_some() {
      return Ok(version_id);
    }

    let version = ModuleVersionRecord::new(version_id, module, 1)
      .with_source(MechSourceCode::String(source.to_string()));

    self.store.put_module_version(version)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "module.version.created")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(name.to_string()),
    )?;

    Ok(version_id)
  }

  pub fn resolve_and_put_module(
    &mut self,
    name: &str,
    compiler_version: &str,
  ) -> MResult<Option<ModuleVersionId>> {
    let Some(resolved) = self.module_resolver.resolve(name)? else {
      return Ok(None);
    };

    resolved.validate()?;

    Ok(Some(self.put_source_module(
      &resolved.name,
      &resolved.source,
      compiler_version,
    )?))
  }

  pub fn activate_module_version(
    &mut self,
    module: ModuleId,
    version: ModuleVersionId,
  ) -> MResult<()> {
    self.store.set_active_module_version(module, version)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "module.version.activated")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(version.to_string()),
    )?;

    Ok(())
  }

  pub fn active_module_version(&self, module: ModuleId) -> MResult<Option<ModuleVersionId>> {
    self.store.get_active_module_version(module)
  }

  // ---------------------------------------------------------------------------
  // Capabilities
  // ---------------------------------------------------------------------------

  pub fn grant_capability(
    &mut self,
    capability: Arc<dyn Capability>,
  ) -> MResult<CapabilityId> {
    capability.validate()?;

    let id = capability.id();

    self
      .capability_kernel
      .grant(CapabilityGrant::new(capability.clone()))?;

    self.store.grant_capability(id, capability)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "capability.granted")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn revoke_capability(&mut self, capability: CapabilityId) -> MResult<()> {
    self
      .capability_kernel
      .revoke(CapabilityRevocation::new(capability))?;

    self.store.revoke_capability(capability)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "capability.revoked")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(capability.to_string()),
    )?;

    Ok(())
  }

  pub fn check_capability(
    &mut self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.capability_kernel.check(request)
  }

  pub fn get_capability(
    &self,
    id: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>> {
    self.store.get_capability(id)
  }

  // ---------------------------------------------------------------------------
  // Objects
  // ---------------------------------------------------------------------------

  pub fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    let id = self.store.put_object(object)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "object.created")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
    self.store.get_object(id)
  }

  pub fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    let id = self.store.update_object(object)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "object.updated")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  // ---------------------------------------------------------------------------
  // Tasks
  // ---------------------------------------------------------------------------

  pub fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
    let id = self.store.put_task(task)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "task.created")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn start_task(
    &mut self,
    subject: impl Into<String>,
    module_version: Option<ModuleVersionId>,
    capabilities: Vec<CapabilityId>,
  ) -> MResult<TaskId> {
    let id = self.next_task_id();

    let mut task = TaskRecord::new(id, subject)
      .with_status(TaskStatus::running())
      .with_capabilities(capabilities);

    if let Some(module_version) = module_version {
      task = task.with_module_version(module_version);
    }

    self.put_task(task)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "task.started")
        .with_subject(format!("task:{}", id)),
    )?;

    Ok(id)
  }

  pub fn get_task(&self, id: TaskId) -> MResult<Option<TaskRecord>> {
    self.store.get_task(id)
  }

  pub fn update_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
    self.store.update_task(task)
  }

  pub fn complete_task(&mut self, id: TaskId) -> MResult<()> {
    let Some(mut task) = self.store.get_task(id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    task.status = TaskStatus::completed();
    self.store.update_task(task)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "task.completed")
        .with_subject(format!("task:{}", id)),
    )?;

    Ok(())
  }

  pub fn fail_task(&mut self, id: TaskId, reason: impl Into<String>) -> MResult<()> {
    let Some(mut task) = self.store.get_task(id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    task.status = TaskStatus::failed();
    self.store.update_task(task)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "task.failed")
        .with_subject(format!("task:{}", id))
        .with_message(reason.into()),
    )?;

    Ok(())
  }

  // ---------------------------------------------------------------------------
  // Actors and Messages
  // ---------------------------------------------------------------------------

  pub fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    let id = self.store.put_actor(actor)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "actor.created")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn create_actor(
    &mut self,
    subject: impl Into<String>,
    behavior: Option<ModuleVersionId>,
    state: Option<ObjectId>,
    capabilities: Vec<CapabilityId>,
  ) -> MResult<ActorId> {
    let id = self.next_actor_id();

    let mut actor = ActorRecord::new(id, subject)
      .with_capabilities(capabilities);

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

  pub fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    self.store.update_actor(actor)
  }

  pub fn send_message(
    &mut self,
    actor: ActorId,
    kind: impl Into<String>,
    payload: Vec<u8>,
  ) -> MResult<MessageId> {
    let id = self.next_message_id();
    let message = MessageRecord::new(id, actor, kind, payload);

    self.store.enqueue_message(actor, message)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "actor.message.sent")
        .with_subject(format!("actor:{}", actor))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn pop_message(&mut self, actor: ActorId) -> MResult<Option<MessageRecord>> {
    self.store.pop_message(actor)
  }

  pub fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>> {
    self.store.peek_message(actor)
  }

  // ---------------------------------------------------------------------------
  // Transactions and Events
  // ---------------------------------------------------------------------------

  pub fn commit_transaction(
    &mut self,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    let id = self.store.commit_transaction(transaction)?;

    let event_id = self.next_event_id();
    self.append_event(
      RuntimeEvent::new(event_id, "transaction.committed")
        .with_subject(format!("runtime:{}", self.id))
        .with_message(id.to_string()),
    )?;

    Ok(id)
  }

  pub fn get_transaction(
    &self,
    id: TransactionId,
  ) -> MResult<Option<TransactionRecord>> {
    self.store.get_transaction(id)
  }

  pub fn list_transactions(
    &self,
    limit: Option<usize>,
  ) -> MResult<Vec<TransactionRecord>> {
    self.store.list_transactions(limit)
  }

  pub fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId> {
    self.store.append_event(event)
  }

  pub fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>> {
    self.store.get_event(id)
  }

  pub fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>> {
    self.store.list_events(limit)
  }

  // ---------------------------------------------------------------------------
  // Shutdown
  // ---------------------------------------------------------------------------

  pub fn shutdown(&mut self) -> MResult<()> {
    let event_id = self.next_event_id();

    self.append_event(
      RuntimeEvent::new(event_id, "runtime.shutdown")
        .with_subject(format!("runtime:{}", self.id)),
    )?;

    Ok(())
  }
}

// -----------------------------------------------------------------------------
// Runtime Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidRuntimeRecordError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidRuntimeRecordError {
  fn name(&self) -> &str {
    "InvalidRuntimeRecord"
  }

  fn message(&self) -> String {
    format!("Invalid runtime record field `{}`: {}", self.field, self.reason)
  }
}

fn invalid_runtime<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidRuntimeRecordError { field, reason },
    None,
  ))
}

#[derive(Debug, Clone)]
pub struct RuntimeRecordNotFoundError {
  pub record_type: &'static str,
  pub id: String,
}

impl MechErrorKind for RuntimeRecordNotFoundError {
  fn name(&self) -> &str {
    "RuntimeRecordNotFound"
  }

  fn message(&self) -> String {
    format!("{} record not found: {}", self.record_type, self.id)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeInvalidOperationError {
  pub operation: &'static str,
  pub reason: String,
}

impl MechErrorKind for RuntimeInvalidOperationError {
  fn name(&self) -> &str {
    "RuntimeInvalidOperation"
  }

  fn message(&self) -> String {
    format!("Invalid runtime operation `{}`: {}", self.operation, self.reason)
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;
  use crate::capability::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject,
  };

  #[test]
  fn runtime_builds() {
    let runtime = RuntimeBuilder::new().build().unwrap();
    assert!(!runtime.id().is_zero());
  }

  #[test]
  fn runtime_runs_string() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    let result = runtime.run_string("x := 1");

    assert!(result.is_ok());
  }

  #[test]
  fn source_module_round_trip() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    let version = runtime
      .put_source_module("main", "x := 1", "0.3.5")
      .unwrap();

    let loaded = runtime
      .store()
      .get_module_version(version)
      .unwrap()
      .unwrap();

    assert_eq!(loaded.id, version);
  }

  #[test]
  fn in_memory_resolver_can_resolve_module() {
    let mut resolver = InMemoryModuleResolver::new();
    resolver.insert("main", "x := 1");

    let mut runtime = RuntimeBuilder::new()
      .module_resolver(resolver)
      .build()
      .unwrap();

    let version = runtime
      .resolve_and_put_module("main", "0.3.5")
      .unwrap()
      .unwrap();

    assert!(!version.is_zero());
  }

  #[test]
  fn grant_and_check_capability() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let capability = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    runtime
      .grant_capability(Arc::new(capability))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert_eq!(
      runtime.check_capability(&request).unwrap(),
      CapabilityId(1),
    );
  }

  #[test]
  fn actor_message_flow() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    let actor = runtime
      .create_actor("actor:1", None, None, Vec::new())
      .unwrap();

    let message = runtime
      .send_message(actor, "ping", b"hello".to_vec())
      .unwrap();

    assert!(!message.is_zero());

    let popped = runtime.pop_message(actor).unwrap().unwrap();

    assert_eq!(popped.kind, "ping");
    assert_eq!(popped.payload, b"hello");
  }

  #[test]
  fn transaction_commit_records_event() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    let tx = TransactionRecord::new(
      runtime.next_transaction_id(),
      "task:1",
    );

    let id = runtime.commit_transaction(tx).unwrap();

    assert!(!id.is_zero());

    let events = runtime.list_events(None).unwrap();
    assert!(
      events
        .iter()
        .any(|event| event.kind == "transaction.committed")
    );
  }

  #[test]
  fn shutdown_records_event() {
    let mut runtime = RuntimeBuilder::new().build().unwrap();

    runtime.shutdown().unwrap();

    let events = runtime.list_events(None).unwrap();
    assert!(
      events
        .iter()
        .any(|event| event.kind == "runtime.shutdown")
    );
  }
}