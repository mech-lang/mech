//! Runtime shell for Mech.
//!
//! `MechRuntime` is the host-facing runtime object. It wraps the current
//! program/interpreter layer and owns the system-level components:
//!
//! - ID generator
//! - store
//! - capability kernel
//! - source resolver
//! - host registry
//! - host call policy
//! - scheduler
//! - runtime config
//!
//! RuntimeContext is used as the per-operation execution envelope. It carries
//! subject/task/actor/module/transaction identity, resource budget, capabilities,
//! and accumulated events.

mod actor;
mod builder;
mod errors;
mod execution;
mod execution_session;
pub(crate) mod extension;
mod host;
mod id;
mod module;
mod object;
mod schedule;
mod task;
mod transaction;

#[cfg(test)]
mod input_tests;

#[cfg(test)]
pub(crate) mod test_support;

pub use self::builder::RuntimeBuilder;
pub use self::errors::*;
pub use self::transaction::{
  RuntimeHealth,
  RuntimePoisonRecord,
};
use self::transaction::{
  ActiveRuntimeProgramOperation,
};
use self::transaction::{
  RuntimeExecutionTransaction,
  RuntimeExecutionTransactionMode,
  RuntimeExecutionTransactionState,
};
use self::transaction::RuntimeCommitResolution;
use crate::{ActiveRuntimeEffectPhase, RuntimeEffectId};
use std::sync::Arc;
use std::cell::Cell;
use std::rc::Rc;
use std::collections::{HashMap, HashSet};
#[cfg(all(
  target_arch = "wasm32",
  target_os = "unknown",
))]
use web_time::Instant;

#[cfg(not(all(
  target_arch = "wasm32",
  target_os = "unknown",
)))]
use std::time::Instant;

use mech_core::{
  MResult, MechError, MechErrorKind, MechSourceCode, Value, ValRef,
  NativeFunctionCompiler, MechFunctionImpl, Register, CompileCtx, MechFunctionCompiler,
  ModuleManifestCatalog, ModuleManifestConfig,
};
use mech_program::{
  MechProgram, MechProgramConfig, MechProgramEnvironment, ProgramInputId
};


#[derive(Clone, Debug)]
struct RuntimeLiveContextTemplate {
  runtime: RuntimeId,
  subject: String,
  task: Option<TaskId>,
  actor: Option<ActorId>,
  module_version: Option<ModuleVersionId>,
  authority: RuntimeAuthorityScope,
  budget_limits: ResourceBudget,
  actor_message: Option<MessageRecord>,
  actor_state: Option<ObjectId>,
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
struct RuntimeLiveStateSnapshot {
  context_template: Option<RuntimeLiveContextTemplate>,
  input_bindings: HashMap<crate::RuntimeHostInputSource, Vec<ProgramInputId>>,
  persistent_sends: Vec<RuntimePersistentSend>,
  registration_mode: LiveRegistrationMode,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LiveRegistrationMode {
  RetainedRoot,
  IsolatedSnapshot,
}

#[derive(Clone, Debug)]
struct RuntimePersistentSend {
  binding: RuntimeContextBinding,
  path: String,
  value: ValRef,
  schedule: RuntimePersistentSendSchedule,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum RuntimePersistentSendSchedule {
  EveryAcceptedTurn,
  Activation { interpreter_id: u64, barrier_node_id: mech_core::ReactiveNodeId },
}

use crate::capability::{
  BasicCapabilityKernel, CapabilityKernel, CapabilityRequest,
};

use crate::config::RuntimeConfig;

use crate::context::{
  ResourceBudget, RuntimeContext, RuntimeContextBuilder, RuntimeTurnOutcome,
  RuntimeContextBinding,
  ResourceBudgetExceededError, RuntimeAuthorityScope,
};

use crate::event::{
  RuntimeEvent, RuntimeEventKind,
};

use crate::host::{
  default_host_capability_request, DefaultHostCallPolicy, HostCall, HostCallPolicy,
  HostFunctionNotFoundError, HostRegistry, InMemoryHostRegistry,
};

use crate::id::{
  module_id, ActorId, CapabilityId, DefaultIdGenerator,
  EventId, IdGenerator, MessageId, ModuleId, ModuleVersionId, ObjectId,
  RuntimeId, TaskId, TransactionId,
};

use crate::resolver::{
  InMemorySourceResolver, ResolvedSource, SourceRequest, SourceResolver,
  SourceImportAlias, SourceScope,
};

use crate::scheduler::{
  collect_tick, InMemoryScheduler, ScheduledWork, Scheduler, SchedulerPolicy,
  SchedulerTick,
};

use crate::store::{
  ActorRecord, InMemoryStore, MechStore, MessageRecord, ModuleRecord,
  ModuleImportEdge, ModuleVersionRecord, ObjectRecord, TaskRecord, TaskStatus, TransactionRecord,
};

use crate::transaction::{
  RuntimeTransactionNotFoundError,
};

use crate::actor::ActorTurn;

use crate::input::RuntimeHostInputQueueState;

use crate::actor_behavior::{
  ActorBehaviorDriver, ActorBehaviorRuntime, NoActorBehaviorDriver,
};

use crate::module::{ModuleBuilder, ModuleBuildOptions, ModuleDependencyGraph};

use crate::{materialize_config_spec_grants, register_config_spec_resources, HostInstanceConfig, HostInterfaceCatalog, InMemoryDocsProvider, RegisteredHostFunction, ResourcePathCapability, RunResourceGrantConfig, RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation, RuntimeConfigSpec, RuntimeHostFactory, RuntimeHostFactoryRegistry, RuntimeModuleResult, RuntimeResourceKey, RuntimeValueSnapshot, DEFAULT_HOST_INPUT_CAPACITY, RuntimeHostInputDriver, RuntimeHostInputQueue, RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest};

// -----------------------------------------------------------------------------
// MechRuntime
// -----------------------------------------------------------------------------

struct ScopedRuntimeState<T: Copy> {
  state: Rc<Cell<Option<T>>>,
  previous: Option<T>,
}

impl<T: Copy> ScopedRuntimeState<T> {
  fn enter(
    state: &Rc<Cell<Option<T>>>,
    value: T,
  ) -> Self {
    let state = Rc::clone(state);
    let previous = state.replace(Some(value));
    Self { state, previous }
  }
}

impl<T: Copy> Drop for ScopedRuntimeState<T> {
  fn drop(&mut self) {
    self.state.set(self.previous);
  }
}

pub struct MechRuntime {
  id: RuntimeId,
  event_sequence: u64,
  config: RuntimeConfig,
  program: MechProgram,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  source_resolver: Box<dyn SourceResolver>,
  host_registry: Box<dyn HostRegistry>,
  host_policy: Box<dyn HostCallPolicy>,
  scheduler: Box<dyn Scheduler>,
  scheduler_policy: SchedulerPolicy,
  active_transactions: HashMap<TransactionId, RuntimeExecutionTransaction>,
  program_transaction_owner: Option<TransactionId>,
  active_program_operation:
    Rc<Cell<Option<ActiveRuntimeProgramOperation>>>,
  active_effect_phase:
    Rc<Cell<Option<ActiveRuntimeEffectPhase>>>,
  health: RuntimeHealth,
  actor_behavior_driver: Box<dyn ActorBehaviorDriver>,
  module_builder: ModuleBuilder,
  resources: RuntimeResourceRegistry,
  resource_bindings: HashMap<String, RuntimeResourceBinding>,
  live_registration_mode: LiveRegistrationMode,
  live_input_bindings: HashMap<crate::RuntimeHostInputSource, Vec<ProgramInputId>>,
  host_input_queue: RuntimeHostInputQueue,
  input_drivers: Vec<Box<dyn RuntimeHostInputDriver>>,
  attached_input_driver_count: usize,
  persistent_sends: Vec<RuntimePersistentSend>,
  live_context_template: Option<RuntimeLiveContextTemplate>,
  input_driver_cleanup_armed: bool,
  host_interfaces: HostInterfaceCatalog,
  module_manifests: ModuleManifestCatalog,
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
      .field("host_registry", &"<dyn HostRegistry>")
      .field("host_policy", &"<dyn HostCallPolicy>")
      .field("scheduler", &"<dyn Scheduler>")
      .field("scheduler_policy", &self.scheduler_policy)
      .field("active_transactions", &self.active_transactions.len())
      .field(
        "active_effect_phase",
        &self.active_effect_phase.get(),
      )
      .field("actor_behavior_driver", &"<dyn ActorBehaviorDriver>")
      .field("module_builder", &self.module_builder)
      .field("resources", &self.resources)
      .field("resource_bindings", &self.resource_bindings)
      .field("live_input_bindings", &self.live_input_bindings)
      .field("input_drivers", &self.input_drivers.len())
      .field("persistent_sends", &self.persistent_sends.len())
      .field("live_context_template", &self.live_context_template)
      .field("host_interfaces", &self.host_interfaces)
      .field("module_manifests", &self.module_manifests)
      .finish()
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceBinding {
  pub name: String,
  pub base_uri: String,
  pub root_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceBindingError {
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for RuntimeResourceBindingError {
  fn name(&self) -> &str {
    "RuntimeResourceBinding"
  }

  fn message(&self) -> String {
    format!("runtime resource binding `{}` failed: {}", self.resource, self.reason)
  }
}

fn runtime_resource_binding_error(
  resource: impl Into<String>,
  reason: impl Into<String>,
) -> MechError {
  MechError::new(
    RuntimeResourceBindingError {
      resource: resource.into(),
      reason: reason.into(),
    },
    None,
  )
}

fn validate_resource_binding_name(name: &str) -> bool {
  !name.is_empty()
    && name
      .bytes()
      .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

#[derive(Clone, Debug)]
struct ModuleInstance {
  version: ModuleVersionId,
  exports: HashMap<String, mech_core::ValRef>,
  result: Value,
}

impl ModuleInstance {
  fn detached_result(&self) -> RuntimeModuleResult {
    RuntimeModuleResult {
      version: self.version,
      exports: self
        .exports
        .iter()
        .map(|(name, value)| {
          (
            name.clone(),
            RuntimeValueSnapshot::capture(&value.borrow()),
          )
        })
        .collect(),
      result: RuntimeValueSnapshot::capture(&self.result),
    }
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

  pub(crate) fn program(&self) -> &MechProgram {
    &self.program
  }

  /// Low-level manual escape hatch outside runtime-owned atomic execution.
  ///
  /// Callers must not use it while a transaction owns the retained program.
  /// Runtime internals must not use it to bypass the program coordinator.
  pub(crate) fn program_mut(&mut self) -> &mut MechProgram {
    &mut self.program
  }

  pub(crate) fn health(&self) -> &RuntimeHealth {
    &self.health
  }

  pub fn runtime_health(&self) -> RuntimeHealth {
    self.health.clone()
  }

  pub fn is_poisoned(&self) -> bool {
    matches!(self.health, RuntimeHealth::Poisoned(_))
  }

  pub(crate) fn bind_context_export(
    &mut self,
    alias: &str,
    module: &str,
    item: &str,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("bind_context_export")?;
    let target = format!("{module}/{item}");
    let base_uri = match self.host_interfaces.resolve_optional(&target)? {
      Some(context) => context.base_uri.clone(),
      None => self.module_manifests.context_export(module, item)?.base_uri.clone(),
    };
    self.bind_resource_root(alias, &base_uri)
  }

  pub fn resource_binding(&self, name: &str) -> Option<&RuntimeResourceBinding> {
    self.resource_bindings.get(name)
  }

  pub(crate) fn bind_resource_root(
    &mut self,
    name: impl Into<String>,
    uri: impl AsRef<str>,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("bind_resource_root")?;
    let name = name.into();
    if !validate_resource_binding_name(&name) {
      return Err(runtime_resource_binding_error(
        name,
        "resource binding names must be non-empty simple tokens",
      ));
    }

    let uri = uri.as_ref().trim_end_matches('/').to_string();
    let base_uri = self
      .resources
      .provider_base_uri_for(&uri)?
      .unwrap_or_else(|| uri.clone());
    let root_path = uri
      .strip_prefix(&base_uri)
      .unwrap_or_default()
      .trim_matches('/')
      .to_string();

    self.resource_bindings.insert(
      name.clone(),
      RuntimeResourceBinding {
        name,
        base_uri,
        root_path,
      },
    );
    Ok(())
  }

  fn resolve_bound_resource_parts(
    &self,
    binding: &str,
    child_path: &str,
  ) -> MResult<(String, String)> {
    let Some(binding_record) = self.resource_bindings.get(binding) else {
      return Err(runtime_resource_binding_error(
        binding,
        "unknown resource root binding",
      ));
    };

    let child_path = child_path.trim_matches('/');

    let stored_root = if binding_record.root_path.is_empty() {
      binding_record.base_uri.trim_end_matches('/').to_string()
    } else {
      format!(
        "{}/{}",
        binding_record.base_uri.trim_end_matches('/'),
        binding_record.root_path.trim_matches('/'),
      )
    };

    let candidate_uri = if child_path.is_empty() {
      stored_root
    } else {
      format!("{}/{}", stored_root.trim_end_matches('/'), child_path)
    };

    if let Some(provider_base_uri) = self.resources.provider_base_uri_for(&candidate_uri)? {
      let provider_path = candidate_uri
        .strip_prefix(&provider_base_uri)
        .unwrap_or_default()
        .trim_matches('/')
        .to_string();
      return Ok((provider_base_uri, provider_path));
    }

    let full_path = if binding_record.root_path.is_empty() {
      child_path.to_string()
    } else if child_path.is_empty() {
      binding_record.root_path.clone()
    } else {
      format!("{}/{}", binding_record.root_path, child_path)
    };
    Ok((binding_record.base_uri.clone(), full_path))
  }

  pub fn read_bound_resource(
    &mut self,
    binding: &str,
    child_path: &str,
  ) -> MResult<RuntimeValueSnapshot> {
    let (base_uri, path) = self.resolve_bound_resource_parts(binding, child_path)?;
    self.read_resource(RuntimeResourceReadRequest {
      base_uri,
      path,
      context_name: binding.to_string(),
    })
  }

  pub fn write_bound_resource(
    &mut self,
    binding: &str,
    child_path: &str,
    value: &Value,
  ) -> MResult<()> {
    let (base_uri, path) = self.resolve_bound_resource_parts(binding, child_path)?;
    self.write_resource(RuntimeResourceWriteRequest {
      base_uri,
      path,
      context_name: binding.to_string(),
      operation: RuntimeCapabilityOperation::Write,
      value: value.clone(),
      intent: RuntimeResourceWriteIntent::Assign,
    })
  }

  pub fn install_run_resource_grant(
    &mut self,
    grant: &RunResourceGrantConfig,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed(
      "install_run_resource_grant",
    )?;
    let interface = self.host_interfaces.resolve(&grant.target)?;
    for operation in &grant.operations {
      if !interface.operations.iter().any(|allowed| allowed == operation) {
        return Err(MechError::new(RuntimeInvalidOperationError {
          operation: "install_run_resource_grant",
          reason: format!("host context `{}` does not expose operation `{operation}`", grant.target),
        }, None));
      }
    }
    let operations = grant
      .operations
      .iter()
      .map(|operation| {
        RuntimeCapabilityOperation::from_name(operation.clone())
      })
      .collect::<MResult<Vec<_>>>()?;
    let spec = RuntimeCapabilityGrantSpec {
      subject: format!("runtime:{}", self.id),
      resource: interface.base_uri.clone(),
      operations,
      paths: grant.paths.clone(),
    };
    let capability = Arc::new(ResourcePathCapability::from_spec(
      self.next_capability_id(),
      &spec,
    )?);
    self.grant_capability(capability).map(|_| ())
  }

  pub(crate) fn register_resource_provider(
    &mut self,
    provider: Box<dyn RuntimeResourceProvider>,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed(
      "register_resource_provider",
    )?;
    self.resources.register_provider(provider)
  }

  pub fn has_resource_provider(&self, scheme: &str) -> bool {
    self.resources.has_provider(scheme)
  }

  pub fn write_resource(
    &mut self,
    request: RuntimeResourceWriteRequest,
  ) -> MResult<()> {
    let mut context = self.runtime_context()?;
    self
      .write_resource_with_context(&mut context, request)
      .map(|_| ())
  }

  pub fn write_resource_with_context(
    &mut self,
    context: &mut RuntimeContext,
    mut request: RuntimeResourceWriteRequest,
  ) -> MResult<RuntimeEffectId> {
    self.ensure_runtime_mutation_allowed(
      "write_resource_with_context",
    )?;
    self.validate_context_for_runtime(context)?;
    let key = RuntimeResourceKey::new(
      &request.base_uri,
      &request.path,
    )?;
    request.base_uri = key.base_uri.clone();
    request.path = key.path.clone();

    if context.transaction.is_none() {
      let transaction_id = self.begin_runtime_transaction_internal(
        context,
        RuntimeExecutionTransactionMode::ImplicitResourceOperation,
      )?;
      let effect_id = match self.write_resource_with_context(context, request) {
        Ok(effect_id) => effect_id,
        Err(error) => {
          return Err(self.cleanup_failed_implicit_resource_operation(
            context,
            transaction_id,
            "write_resource_with_context",
            error,
          ));
        }
      };
      return match self.commit_runtime_transaction_internal(context) {
        Ok(RuntimeCommitResolution::Committed(_)) => Ok(effect_id),
        Ok(RuntimeCommitResolution::CommittedWithError { error, .. }) => Err(error),
        Err(error) => Err(self.cleanup_failed_implicit_resource_operation(
          context,
          transaction_id,
          "write_resource_with_context",
          error,
        )),
      };
    }

    self.authorize_resource_with_context(
      context,
      &request.operation,
      &key,
    )?;
    request.value = request.value.deep_snapshot();
    let staged_resource = if request.intent
      == RuntimeResourceWriteIntent::Assign
    {
      Some((
        request.base_uri.clone(),
        request.path.clone(),
        request.value.clone(),
      ))
    } else {
      None
    };
    let effect = self.resources.prepare_write(request)?;
    match staged_resource {
      Some((base_uri, path, value)) => {
        self.stage_runtime_resource_effect_with_context(
          context,
          effect,
          base_uri,
          path,
          value,
        )
      }
      None => self.stage_runtime_effect_with_context(context, effect),
    }
  }

  fn cleanup_failed_implicit_resource_operation(
    &mut self,
    context: &mut RuntimeContext,
    transaction_id: TransactionId,
    operation: &'static str,
    original_error: MechError,
  ) -> MechError {
    if context.transaction != Some(transaction_id) {
      return original_error;
    }
    let original_error_text = format!("{:?}", original_error);
    match self.abort_runtime_transaction_cleanup(
      context,
      "implicit resource operation failed",
      false,
    ) {
      Ok((cleaned_transaction_id, mut failures)) => {
        if cleaned_transaction_id != transaction_id {
          failures.push(format!(
            "implicit resource cleanup targeted transaction {}, expected {}",
            cleaned_transaction_id,
            transaction_id,
          ));
        }
        if failures.is_empty() {
          original_error
        } else {
          self.poison_program_operation(
            operation,
            Some(transaction_id),
            original_error_text,
            failures,
          )
        }
      }
      Err(cleanup_error) => self.poison_program_operation(
        operation,
        Some(transaction_id),
        original_error_text,
        vec![format!(
          "implicit resource cleanup could not start: {:?}",
          cleanup_error,
        )],
      ),
    }
  }

  pub fn read_resource(
    &mut self,
    request: RuntimeResourceReadRequest,
  ) -> MResult<RuntimeValueSnapshot> {
    let mut context = self.runtime_context()?;
    self.read_resource_with_context(&mut context, request)
  }

  pub fn read_resource_with_context(
    &mut self,
    context: &mut RuntimeContext,
    request: RuntimeResourceReadRequest,
  ) -> MResult<RuntimeValueSnapshot> {
    self
      .read_resource_value_with_context(context, request)
      .map(|value| RuntimeValueSnapshot::capture(&value))
  }

  pub(crate) fn read_resource_value_with_context(
    &mut self,
    context: &mut RuntimeContext,
    mut request: RuntimeResourceReadRequest,
  ) -> MResult<Value> {
    self.validate_context_for_runtime(context)?;
    let key = RuntimeResourceKey::new(
      &request.base_uri,
      &request.path,
    )?;
    request.base_uri = key.base_uri.clone();
    request.path = key.path.clone();
    if context.transaction.is_none() {
      let transaction_id = self.begin_runtime_transaction_internal(
        context,
        RuntimeExecutionTransactionMode::ImplicitResourceOperation,
      )?;
      let value = match self
        .read_resource_value_with_context(context, request)
      {
        Ok(value) => value,
        Err(error) => {
          return Err(self.cleanup_failed_implicit_resource_operation(
            context,
            transaction_id,
            "read_resource_with_context",
            error,
          ));
        }
      };
      return match self.commit_runtime_transaction_internal(context) {
        Ok(RuntimeCommitResolution::Committed(_)) => Ok(value),
        Ok(RuntimeCommitResolution::CommittedWithError { error, .. }) => {
          Err(error)
        }
        Err(error) => Err(
          self.cleanup_failed_implicit_resource_operation(
            context,
            transaction_id,
            "read_resource_with_context",
            error,
          ),
        ),
      };
    }
    self.authorize_resource_with_context(
      context,
      &RuntimeCapabilityOperation::Read,
      &key,
    )?;
    if context.transaction.is_some() {
      let transaction_id = context.transaction.unwrap();
      if let Some(value) = self
        .active_execution_transaction(transaction_id)?
        .effects
        .staged_resource_value(&request.base_uri, &request.path)
      {
        return Ok(value);
      }
    }
    self.resources.read(request)
  }

  pub(crate) fn authorize_resource_with_context(
    &mut self,
    context: &mut RuntimeContext,
    operation: &RuntimeCapabilityOperation,
    key: &RuntimeResourceKey,
  ) -> MResult<CapabilityId> {
    let request = CapabilityRequest::from_keys(
      &context.subject,
      operation.name(),
      key.capability_resource(),
    );
    self.check_capability_with_context(context, &request)
  }

  pub(crate) fn store(&self) -> &dyn MechStore {
    self.store.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn store_mut(&mut self) -> &mut dyn MechStore {
    self.store.as_mut()
  }

  pub(crate) fn capability_kernel(&self) -> &dyn CapabilityKernel {
    self.capability_kernel.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn capability_kernel_mut(&mut self) -> &mut dyn CapabilityKernel {
    self.capability_kernel.as_mut()
  }

  pub(crate) fn source_resolver(&self) -> &(dyn SourceResolver + 'static) {
    self.source_resolver.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn source_resolver_mut(&mut self) -> &mut dyn SourceResolver {
    self.source_resolver.as_mut()
  }

  pub(crate) fn set_source_resolver(
    &mut self,
    source_resolver: impl SourceResolver + 'static,
  ) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("set_source_resolver")?;
    self.source_resolver = Box::new(source_resolver);
    Ok(())
  }

  pub(crate) fn host_registry(&self) -> &dyn HostRegistry {
    self.host_registry.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn host_registry_mut(&mut self) -> &mut dyn HostRegistry {
    self.host_registry.as_mut()
  }

  pub(crate) fn host_policy(&self) -> &dyn HostCallPolicy {
    self.host_policy.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn host_policy_mut(&mut self) -> &mut dyn HostCallPolicy {
    self.host_policy.as_mut()
  }

  pub(crate) fn scheduler(&self) -> &dyn Scheduler {
    self.scheduler.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn scheduler_mut(&mut self) -> &mut dyn Scheduler {
    self.scheduler.as_mut()
  }

  pub fn scheduler_policy(&self) -> &SchedulerPolicy {
    &self.scheduler_policy
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn scheduler_policy_mut(&mut self) -> &mut SchedulerPolicy {
    &mut self.scheduler_policy
  }

  pub(crate) fn actor_behavior_driver(&self) -> &dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_ref()
  }

  /// Unchecked administrative escape hatch outside runtime-owned poison
  /// enforcement. Runtime internals must not use it to bypass mutation
  /// preflight.
  pub(crate) fn actor_behavior_driver_mut(&mut self) -> &mut dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_mut()
  }

  pub(crate) fn module_builder(&self) -> &ModuleBuilder {
    &self.module_builder
  }

  pub(crate) fn set_scheduler_policy(&mut self, scheduler_policy: SchedulerPolicy) -> MResult<()> {
    self.ensure_runtime_mutation_allowed("set_scheduler_policy")?;
    scheduler_policy.validate()?;
    self.scheduler_policy = scheduler_policy;
    Ok(())
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

    budget
  }

  fn known_source_bytes(source: &MechSourceCode) -> MResult<Option<u64>> {
    match source {
      MechSourceCode::String(source) | MechSourceCode::Html(source) => {
        Ok(Some(u64::try_from(source.as_bytes().len()).map_err(|_| {
          MechError::new(
            ResourceBudgetExceededError {
              resource: "source_bytes",
              used: u64::MAX,
              requested: 1,
              max: None,
            },
            None,
          )
        })?))
      }
      MechSourceCode::ByteCode(bytes) => {
        Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
          MechError::new(
            ResourceBudgetExceededError {
              resource: "source_bytes",
              used: u64::MAX,
              requested: 1,
              max: None,
            },
            None,
          )
        })?))
      }
      MechSourceCode::Image(_, bytes) => {
        Ok(Some(u64::try_from(bytes.len()).map_err(|_| {
          MechError::new(
            ResourceBudgetExceededError {
              resource: "source_bytes",
              used: u64::MAX,
              requested: 1,
              max: None,
            },
            None,
          )
        })?))
      }
      MechSourceCode::Program(sources) => {
        let mut total = 0u64;
        for source in sources {
          let Some(bytes) = Self::known_source_bytes(source)? else {
            return Ok(None);
          };
          total = total.checked_add(bytes).ok_or_else(|| {
            MechError::new(
              ResourceBudgetExceededError {
                resource: "source_bytes",
                used: total,
                requested: bytes,
                max: None,
              },
              None,
            )
          })?;
        }
        Ok(Some(total))
      }
      MechSourceCode::Tree(_) => Ok(None),
    }
  }

  fn enforce_source_limits(
    &self,
    context: &mut RuntimeContext,
    source: &MechSourceCode,
  ) -> MResult<()> {
    let Some(source_bytes) = Self::known_source_bytes(source)? else {
      return Ok(());
    };

    self.enforce_source_byte_count(context, source_bytes)
  }

  fn enforce_source_byte_count(
    &self,
    context: &mut RuntimeContext,
    source_bytes: u64,
  ) -> MResult<()> {
    if let Some(max) = self.config.limits.max_source_bytes {
      if source_bytes > max {
        return Err(MechError::new(
          ResourceBudgetExceededError {
            resource: "source_bytes",
            used: 0,
            requested: source_bytes,
            max: Some(max),
          },
          None,
        ));
      }
    }

    context.charge_bytes(source_bytes)
  }

  fn trim_events_to_retention(&self, events: &mut Vec<RuntimeEvent>) {
    let Some(max_events) = self.config.limits.max_in_memory_events else { return; };
    let max_events = usize::try_from(max_events).unwrap_or(usize::MAX);
    if events.len() > max_events { events.drain(0..(events.len() - max_events)); }
  }

  fn enforce_turn_duration(&self, started: Instant) -> MResult<()> {
    let Some(max) = self.config.limits.max_turn_duration_ms else { return Ok(()); };
    let requested = u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX);
    if requested > max {
      return Err(MechError::new(ResourceBudgetExceededError { resource: "turn_duration_ms", used: 0, requested, max: Some(max) }, None));
    }
    Ok(())
  }

  pub fn runtime_context(&self) -> MResult<RuntimeContext> {
    RuntimeContextBuilder::new(self.id)
      .budget(self.default_budget())
      .build()
  }

  fn validate_live_context_candidate(&self, context: &RuntimeContext) -> MResult<()> {
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
      Some(_) => Err(MechError::new(RuntimeInvalidOperationError {
        operation: "RuntimeLiveContextMismatch",
        reason: "source load attempted to change the live program execution identity or budget maxima".to_string(),
      }, None)),
      None => Ok(()),
    }
  }

  fn commit_live_context_candidate(&mut self, context: &RuntimeContext) {
    if self.live_context_template.is_none() {
      self.live_context_template = Some(RuntimeLiveContextTemplate::from_context(context));
    }
  }

  fn live_state_snapshot(&self) -> RuntimeLiveStateSnapshot {
    RuntimeLiveStateSnapshot {
      context_template: self.live_context_template.clone(),
      input_bindings: self.live_input_bindings.clone(),
      persistent_sends: self.persistent_sends.clone(),
      registration_mode: self.live_registration_mode,
    }
  }

  fn restore_live_state(&mut self, snapshot: RuntimeLiveStateSnapshot) {
    self.live_context_template = snapshot.context_template;
    self.live_input_bindings = snapshot.input_bindings;
    self.persistent_sends = snapshot.persistent_sends;
    self.live_registration_mode = snapshot.registration_mode;
  }

  fn live_turn_context(&self) -> MResult<RuntimeContext> {
    self.live_context_template
      .as_ref()
      .map(RuntimeLiveContextTemplate::fresh_context)
      .ok_or_else(|| MechError::new(RuntimeInvalidOperationError {
        operation: "RuntimeLiveContextMissing",
        reason: "host input turn requires a stored live program context".to_string(),
      }, None))
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

  pub fn context_for_actor_turn(
    &self,
    turn: &ActorTurn,
  ) -> MResult<RuntimeContext> {
    turn.validate()?;
    let actor = self
      .store
      .get_actor(turn.actor)?
      .ok_or_else(|| MechError::new(
        RuntimeInvalidOperationError {
          operation: "context_for_actor_turn",
          reason: format!(
            "actor record {} was not found",
            turn.actor,
          ),
        },
        None,
      ))?;
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

  fn validate_context_for_runtime(
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

  fn emit_event_to_context(
    &mut self,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    self.validate_context_for_runtime(context)?;

    let context_events_before = context.events.clone();
    let event = self.make_event(kind);
    let id = event.id;

    context.push_event(event.clone());
    self.trim_events_to_retention(&mut context.events);
    if let Some(transaction_id) = context.transaction {
      if let Some(transaction) = self.active_transactions.get_mut(&transaction_id) {
        if let Err(error) = transaction.store.stage_event(event) {
          context.events = context_events_before;
          return Err(error);
        }
        return Ok(id);
      }
    }

    if let Err(error) = self.store.append_event(event) {
      context.events = context_events_before;
      return Err(error);
    }

    Ok(id)
  }

  fn emit_event_immediate_to_context(
    &mut self,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    self.validate_context_for_runtime(context)?;

    let context_events_before = context.events.clone();
    let event = self.make_event(kind);
    let id = event.id;

    context.push_event(event.clone());
    self.trim_events_to_retention(&mut context.events);
    if let Err(error) = self.store.append_event(event) {
      context.events = context_events_before;
      return Err(error);
    }

    Ok(id)
  }

  fn push_persisted_event_to_context(
    &self,
    context: &mut RuntimeContext,
    event: RuntimeEvent,
  ) -> EventId {
    let id = event.id;
    context.push_event(event);
    self.trim_events_to_retention(&mut context.events);
    id
  }

  // ---------------------------------------------------------------------------
  // Shutdown
  // ---------------------------------------------------------------------------

  pub fn shutdown(&mut self) -> MResult<()> {
    let mut first_error = None;

    if let Err(error) = self.close_ingress() {
      first_error = Some(error);
    }

    if let Err(error) = self.stop_input_drivers() {
      if first_error.is_none() {
        first_error = Some(error);
      }
    }
    self.input_driver_cleanup_armed = false;

    match self.runtime_context() {
      Ok(mut context) => {
        if let Err(error) = self.emit_event_to_context(
          &mut context,
          RuntimeEventKind::RuntimeShutdown {
            runtime_id: self.id,
          },
        ) {
          if first_error.is_none() {
            first_error = Some(error);
          }
        }
      }
      Err(error) => {
        if first_error.is_none() {
          first_error = Some(error);
        }
      }
    }

    match first_error {
      Some(error) => Err(error),
      None => Ok(()),
    }
  }
}

impl Drop for MechRuntime {
  fn drop(&mut self) {
    if self.input_driver_cleanup_armed {
      let _ = self.close_ingress();
      for driver in self.input_drivers[..self.attached_input_driver_count].iter_mut().rev() {
        let _ = extension::catch_extension(
          "host input driver",
          "stop",
          || driver.stop(),
        );
      }
      self.input_driver_cleanup_armed = false;
    }
  }
}

fn validate_module_import_edges(record: &ModuleVersionRecord) -> MResult<()> {
  record.validate_import_edges().map_err(|error| {
    MechError::new(
      RuntimeModuleImportEdgeInvalid {
        module: record.id,
        reason: format!("{:?}", error),
      },
      None,
    )
  })
}
