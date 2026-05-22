//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It wraps the current
//! program/interpreter layer and owns the system-level components:
//!
//! - ID generator
//! - store
//! - capability kernel
//! - source resolver
//! - runtime config
//!
//! RuntimeContext is used as the per-operation execution envelope. It carries
//! subject/task/actor/module/transaction identity, resource budget, capabilities,
//! and accumulated events.

use std::sync::Arc;

use mech_core::{
  MResult, MechError, MechErrorKind, MechSourceCode, Value,
};

use mech_program::{
  MechProgram, MechProgramConfig, MechProgramEnvironment,
};

use crate::capability::{
  BasicCapabilityKernel, Capability, CapabilityGrant, CapabilityKernel,
  CapabilityRequest, CapabilityRevocation,
};

use crate::config::RuntimeConfig;

use crate::context::{
  ResourceBudget, RuntimeContext, RuntimeContextBuilder,
};

use crate::event::{
  RuntimeEvent, RuntimeEventKind,
};

use crate::id::{
  module_id, module_version_id, ActorId, CapabilityId, DefaultIdGenerator,
  EventId, IdGenerator, ModuleId, ModuleVersionId, ObjectId, RuntimeId,
  TaskId, TransactionId, MessageId,
};

use crate::resolver::{
  InMemorySourceResolver, ResolvedSource, SourceRequest, SourceResolver,
};

use crate::store::{
  ActorRecord, InMemoryStore, MechStore, MessageRecord,
  ModuleRecord, ModuleVersionRecord, ObjectRecord, TaskRecord, TaskStatus,
  TransactionRecord,
};

// -----------------------------------------------------------------------------
// Runtime Builder
// -----------------------------------------------------------------------------

pub struct RuntimeBuilder {
  config: RuntimeConfig,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  source_resolver: Box<dyn SourceResolver>,
}

impl std::fmt::Debug for RuntimeBuilder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RuntimeBuilder")
      .field("config", &self.config)
      .field("id_generator", &"<dyn IdGenerator>")
      .field("store", &"<dyn MechStore>")
      .field("capability_kernel", &"<dyn CapabilityKernel>")
      .field("source_resolver", &"<dyn SourceResolver>")
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
      source_resolver: Box::new(InMemorySourceResolver::new()),
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

  pub fn source_resolver(
    mut self,
    source_resolver: impl SourceResolver + 'static,
  ) -> Self {
    self.source_resolver = Box::new(source_resolver);
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
      event_sequence: 0,
      config: self.config,
      program: MechProgram::new(program_config),
      id_generator: self.id_generator,
      store: self.store,
      capability_kernel: self.capability_kernel,
      source_resolver: self.source_resolver,
    };

    let mut context = runtime.runtime_context()?;

    runtime.emit_event_to_context(
      &mut context,
      RuntimeEventKind::RuntimeCreated {
        runtime_id: runtime.id,
      },
    )?;

    Ok(runtime)
  }
}

// -----------------------------------------------------------------------------
// MechRuntime
// -----------------------------------------------------------------------------

pub struct MechRuntime {
  id: RuntimeId,
  event_sequence: u64,
  config: RuntimeConfig,
  program: MechProgram,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  source_resolver: Box<dyn SourceResolver>,
}

impl std::fmt::Debug for MechRuntime {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("MechRuntime")
      .field("id", &self.id)
      .field("event_sequence", &self.event_sequence)
      .field("config", &self.config)
      .field("program", &"<MechProgram>")
      .field("id_generator", &"<dyn IdGenerator>")
      .field("store", &"<dyn MechStore>")
      .field("capability_kernel", &"<dyn CapabilityKernel>")
      .field("source_resolver", &"<dyn SourceResolver>")
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

  pub fn source_resolver(&self) -> &dyn SourceResolver {
    self.source_resolver.as_ref()
  }

  pub fn source_resolver_mut(&mut self) -> &mut dyn SourceResolver {
    self.source_resolver.as_mut()
  }

  // ---------------------------------------------------------------------------
  // Context helpers
  // ---------------------------------------------------------------------------

  pub fn default_budget(&self) -> ResourceBudget {
    let mut budget = ResourceBudget::default();

    if let Some(max_steps) = self.config.limits.max_steps_per_turn {
      budget = budget.with_max_steps(max_steps);
    }

    if let Some(max_bytes) = self.config.limits.max_memory_bytes {
      budget = budget.with_max_bytes(max_bytes);
    }

    if let Some(max_messages) = self.config.limits.max_actor_mailbox_len {
      budget = budget.with_max_messages(max_messages);
    }

    budget
  }

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

    builder.build()
  }

  pub fn context_for_transaction(
    &self,
    transaction: &TransactionRecord,
  ) -> MResult<RuntimeContext> {
    RuntimeContextBuilder::new(self.id)
      .subject(transaction.subject.clone())
      .transaction(transaction.id)
      .budget(self.default_budget())
      .build()
  }

  // ---------------------------------------------------------------------------
  // Event helpers
  // ---------------------------------------------------------------------------

  pub fn next_event_sequence(&mut self) -> u64 {
    let sequence = self.event_sequence;
    self.event_sequence = self.event_sequence.saturating_add(1);
    sequence
  }

  fn make_event(&mut self, kind: RuntimeEventKind) -> RuntimeEvent {
    RuntimeEvent::new(
      self.next_event_id(),
      self.next_event_sequence(),
      kind,
    )
  }

  fn emit_event(&mut self, kind: RuntimeEventKind) -> MResult<EventId> {
    let event = self.make_event(kind);
    self.append_event(event)
  }

  fn emit_event_to_context(
    &mut self,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    context.validate()?;

    let event = self.make_event(kind);
    let id = event.id;

    context.push_event(event.clone());
    self.append_event(event)?;

    Ok(id)
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

  pub fn next_message_id(&mut self) -> MessageId {
    MessageId(self.id_generator.event_id().as_u128())
  }

  // ---------------------------------------------------------------------------
  // Program execution
  // ---------------------------------------------------------------------------

  pub fn run_string(&mut self, source: &str) -> MResult<Value> {
    let mut context = self.runtime_context()?;
    self.run_string_with_context(&mut context, source)
  }

  pub fn run_string_with_context(
    &mut self,
    context: &mut RuntimeContext,
    source: &str,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ProgramStarted {
        task_id: context.task,
      },
    )?;

    let result = self.program.run_string(source);

    match &result {
      Ok(_) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramCompleted {
            task_id: context.task,
          },
        )?;
      }
      Err(error) => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramFailed {
            task_id: context.task,
            message: format!("{:?}", error),
          },
        )?;
      }
    }

    result
  }

  pub fn run_module(&mut self, version: ModuleVersionId) -> MResult<Value> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.run_module_with_context(&mut context, version)
  }

  pub fn run_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    version: ModuleVersionId,
  ) -> MResult<Value> {
    context.validate()?;
    context.charge_step()?;

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
      MechSourceCode::String(source) => self.run_string_with_context(context, &source),
      other => {
        self.emit_event_to_context(
          context,
          RuntimeEventKind::ProgramStarted {
            task_id: context.task,
          },
        )?;

        let result = self.program.run_source(&other);

        match &result {
          Ok(_) => {
            self.emit_event_to_context(
              context,
              RuntimeEventKind::ProgramCompleted {
                task_id: context.task,
              },
            )?;
          }
          Err(error) => {
            self.emit_event_to_context(
              context,
              RuntimeEventKind::ProgramFailed {
                task_id: context.task,
                message: format!("{:?}", error),
              },
            )?;
          }
        }

        result
      }
    }
  }

  // ---------------------------------------------------------------------------
  // Sources and Modules
  // ---------------------------------------------------------------------------

  pub fn ensure_module(
    &mut self,
    name: &str,
    canonical_uri: &str,
  ) -> MResult<ModuleId> {
    if let Some(module) = self.store.find_module_by_name(canonical_uri)? {
      return Ok(module.id);
    }

    let id = module_id(canonical_uri);
    let module = ModuleRecord::new(id, canonical_uri)
      .with_description(name.to_string());

    self.store.put_module(module)
  }

  pub fn resolve_source(
    &self,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    let request = request.into();
    request.validate()?;

    self.source_resolver.resolve(&request)
  }

  pub fn resolve_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    context.validate()?;
    context.charge_step()?;

    let request = request.into();
    request.validate()?;

    let resolved = self.source_resolver.resolve(&request)?;

    if let Some(source) = &resolved {
      self.emit_event_to_context(
        context,
        RuntimeEventKind::SourceResolved {
          canonical_uri: source.canonical_uri.clone(),
        },
      )?;
    }

    Ok(resolved)
  }

  pub fn resolve_source_evented(
    &mut self,
    request: impl Into<SourceRequest>,
  ) -> MResult<Option<ResolvedSource>> {
    let mut context = self.runtime_context()?;
    self.resolve_source_with_context(&mut context, request)
  }

  pub fn store_resolved_module_source(
    &mut self,
    resolved: ResolvedSource,
    compiler_version: &str,
    language_edition: &str,
    target: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<ModuleVersionId> {
    let mut context = self.runtime_context()?;

    self.store_resolved_module_source_with_context(
      &mut context,
      resolved,
      compiler_version,
      language_edition,
      target,
      feature_flags,
      capability_requirements,
    )
  }

  pub fn store_resolved_module_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    resolved: ResolvedSource,
    compiler_version: &str,
    language_edition: &str,
    target: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<ModuleVersionId> {
    context.validate()?;
    context.charge_step()?;
    resolved.validate()?;

    if !resolved.is_executable_mech_source() {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "store_resolved_module_source",
          reason: format!(
            "resolved source `{}` is not executable Mech source",
            resolved.canonical_uri
          ),
        },
        None,
      ));
    }

    let module = self.ensure_module(&resolved.name, &resolved.canonical_uri)?;
    let source_fingerprint = source_fingerprint(&resolved.source)?;
    let dependency_versions: Vec<ModuleVersionId> = Vec::new();

    let version_id = module_version_id(
      &source_fingerprint,
      compiler_version,
      language_edition,
      target,
      feature_flags,
      &dependency_versions,
      capability_requirements,
    );

    if self.store.get_module_version(version_id)?.is_some() {
      return Ok(version_id);
    }

    let version = ModuleVersionRecord::new(version_id, module, 1)
      .with_source(resolved.source)
      .with_dependencies(dependency_versions)
      .with_capability_requirements(resolved.capability_requirements);

    self.store.put_module_version(version)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleCompiled {
        module_version: version_id,
      },
    )?;

    Ok(version_id)
  }

  pub fn resolve_and_store_module_source(
    &mut self,
    request: impl Into<SourceRequest>,
    compiler_version: &str,
    language_edition: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<Option<ModuleVersionId>> {
    let mut context = self.runtime_context()?;

    self.resolve_and_store_module_source_with_context(
      &mut context,
      request,
      compiler_version,
      language_edition,
      feature_flags,
      capability_requirements,
    )
  }

  pub fn resolve_and_store_module_source_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: impl Into<SourceRequest>,
    compiler_version: &str,
    language_edition: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<Option<ModuleVersionId>> {
    let Some(resolved) = self.resolve_source_with_context(context, request)? else {
      return Ok(None);
    };

    let target = runtime_target();

    Ok(Some(self.store_resolved_module_source_with_context(
      context,
      resolved,
      compiler_version,
      language_edition,
      &target,
      feature_flags,
      capability_requirements,
    )?))
  }

  pub fn put_source_module(
    &mut self,
    name: &str,
    canonical_uri: &str,
    source: &str,
    compiler_version: &str,
    language_edition: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<ModuleVersionId> {
    let mut context = self.runtime_context()?;

    self.put_source_module_with_context(
      &mut context,
      name,
      canonical_uri,
      source,
      compiler_version,
      language_edition,
      feature_flags,
      capability_requirements,
    )
  }

  pub fn put_source_module_with_context(
    &mut self,
    context: &mut RuntimeContext,
    name: &str,
    canonical_uri: &str,
    source: &str,
    compiler_version: &str,
    language_edition: &str,
    feature_flags: &[&str],
    capability_requirements: &[&str],
  ) -> MResult<ModuleVersionId> {
    let resolved = ResolvedSource::new(
      name,
      canonical_uri,
      MechSourceCode::String(source.to_string()),
    );

    let target = runtime_target();

    self.store_resolved_module_source_with_context(
      context,
      resolved,
      compiler_version,
      language_edition,
      &target,
      feature_flags,
      capability_requirements,
    )
  }

  pub fn activate_module_version(
    &mut self,
    module: ModuleId,
    version: ModuleVersionId,
  ) -> MResult<()> {
    let mut context = self.runtime_context()?
      .with_module_version(version);

    self.activate_module_version_with_context(&mut context, module, version)
  }

  pub fn activate_module_version_with_context(
    &mut self,
    context: &mut RuntimeContext,
    module: ModuleId,
    version: ModuleVersionId,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;

    self.store.set_active_module_version(module, version)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ModuleActivated {
        module_version: version,
      },
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
    let mut context = self.runtime_context()?;
    self.grant_capability_with_context(&mut context, capability)
  }

  pub fn grant_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: Arc<dyn Capability>,
  ) -> MResult<CapabilityId> {
    context.validate()?;
    context.charge_step()?;
    capability.validate()?;

    let id = capability.id();

    self
      .capability_kernel
      .grant(CapabilityGrant::new(capability.clone()))?;

    self.store.grant_capability(id, capability)?;
    context.add_capability(id);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::CapabilityGranted {
        capability_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn revoke_capability(&mut self, capability: CapabilityId) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self.revoke_capability_with_context(&mut context, capability)
  }

  pub fn revoke_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    capability: CapabilityId,
  ) -> MResult<()> {
    context.validate()?;
    context.charge_step()?;

    self
      .capability_kernel
      .revoke(CapabilityRevocation::new(capability))?;

    self.store.revoke_capability(capability)?;
    context.remove_capability(capability);

    self.emit_event_to_context(
      context,
      RuntimeEventKind::CapabilityRevoked {
        capability_id: capability,
      },
    )?;

    Ok(())
  }

  pub fn check_capability(
    &mut self,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.capability_kernel.check(request)
  }

  pub fn check_capability_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    context.validate()?;
    context.charge_step()?;
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
    let mut context = self.runtime_context()?;
    self.put_object_with_context(&mut context, object)
  }

  pub fn put_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    context.validate()?;
    context.charge_bytes(object.data.len() as u64)?;

    let id = self.store.put_object(object)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ObjectCreated {
        object_id: id,
      },
    )?;

    Ok(id)
  }

  pub fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
    self.store.get_object(id)
  }

  pub fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
    let mut context = self.runtime_context()?;
    self.update_object_with_context(&mut context, object)
  }

  pub fn update_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    context.validate()?;
    context.charge_bytes(object.data.len() as u64)?;

    let id = self.store.update_object(object)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ObjectUpdated {
        object_id: id,
      },
    )?;

    Ok(id)
  }

  // ---------------------------------------------------------------------------
  // Tasks
  // ---------------------------------------------------------------------------

  pub fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
    let mut context = self.context_for_task(&task)?;
    self.put_task_with_context(&mut context, task)
  }

  pub fn put_task_with_context(
    &mut self,
    context: &mut RuntimeContext,
    task: TaskRecord,
  ) -> MResult<TaskId> {
    context.validate()?;
    context.charge_step()?;

    let id = self.store.put_task(task)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TaskCreated {
        task_id: id,
      },
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

    let mut context = self.context_for_task(&task)?;
    self.put_task_with_context(&mut context, task)?;

    self.emit_event_to_context(
      &mut context,
      RuntimeEventKind::TaskStarted {
        task_id: id,
      },
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

    let mut context = self.context_for_task(&task)?;

    task.status = TaskStatus::completed();
    self.store.update_task(task)?;

    self.emit_event_to_context(
      &mut context,
      RuntimeEventKind::TaskCompleted {
        task_id: id,
      },
    )?;

    Ok(())
  }

  pub fn fail_task(&mut self, id: TaskId, reason: impl Into<String>) -> MResult<()> {
    let reason = reason.into();

    let Some(mut task) = self.store.get_task(id)? else {
      return Err(MechError::new(
        RuntimeRecordNotFoundError {
          record_type: "task",
          id: id.to_string(),
        },
        None,
      ));
    };

    let mut context = self.context_for_task(&task)?;

    task.status = TaskStatus::failed();
    self.store.update_task(task)?;

    self.emit_event_to_context(
      &mut context,
      RuntimeEventKind::TaskFailed {
        task_id: id,
        message: reason,
      },
    )?;

    Ok(())
  }

  // ---------------------------------------------------------------------------
  // Actors and Messages
  // ---------------------------------------------------------------------------

  pub fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
    let mut context = self.context_for_actor(&actor)?;
    self.put_actor_with_context(&mut context, actor)
  }

  pub fn put_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId> {
    context.validate()?;
    context.charge_step()?;

    let id = self.store.put_actor(actor)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::ActorCreated {
        actor_id: id,
      },
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
    context.validate()?;
    context.charge_messages(1)?;
    context.charge_bytes(payload.len() as u64)?;

    let id = self.next_message_id();
    let message = MessageRecord::new(id, actor, kind, payload);

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
    let mut context = self.context_for_transaction(&transaction)?;
    self.commit_transaction_with_context(&mut context, transaction)
  }

  pub fn commit_transaction_with_context(
    &mut self,
    context: &mut RuntimeContext,
    transaction: TransactionRecord,
  ) -> MResult<TransactionId> {
    context.validate()?;
    context.charge_step()?;

    let id = self.store.commit_transaction(transaction)?;

    self.emit_event_to_context(
      context,
      RuntimeEventKind::TransactionCommitted {
        transaction_id: id,
      },
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
    let mut context = self.runtime_context()?;

    self.emit_event_to_context(
      &mut context,
      RuntimeEventKind::RuntimeShutdown {
        runtime_id: self.id,
      },
    )?;

    Ok(())
  }
}

// -----------------------------------------------------------------------------
// Runtime Errors
// -----------------------------------------------------------------------------

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
// Helpers
// -----------------------------------------------------------------------------

fn runtime_target() -> String {
  format!("{}-{}", std::env::consts::OS, std::env::consts::ARCH)
}

fn source_fingerprint(source: &MechSourceCode) -> MResult<String> {
  match source {
    MechSourceCode::String(source) => Ok(source.clone()),

    MechSourceCode::ByteCode(bytes) => Ok(hex_bytes(bytes)),

    MechSourceCode::Tree(tree) => Ok(format!("{:?}", tree)),

    MechSourceCode::Program(sources) => {
      let mut out = String::new();

      for source in sources {
        out.push_str(&source_fingerprint(source)?);
        out.push('\n');
      }

      Ok(out)
    }

    other => Err(MechError::new(
      RuntimeInvalidOperationError {
        operation: "source_fingerprint",
        reason: format!("cannot fingerprint non-executable source: {:?}", other),
      },
      None,
    )),
  }
}

fn hex_bytes(bytes: &[u8]) -> String {
  let mut out = String::with_capacity(bytes.len() * 2);

  for byte in bytes {
    use std::fmt::Write;
    let _ = write!(&mut out, "{:02x}", byte);
  }

  out
}