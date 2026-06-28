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
mod capability;
mod errors;
mod execution;
mod host;
mod id;
mod module;
mod object;
mod schedule;
mod service;
mod task;
mod transaction;

use crate::runtime::errors::*;
use crate::runtime::host::*;

use std::sync::Arc;
use std::cell::RefCell;
use std::collections::{HashMap, HashSet};

use mech_core::{
  browser_capability_error, BrowserDomPath, BROWSER_DOM_PROVIDER_URI, MResult, MechError,
  MechErrorKind, MechSourceCode, Value,
  NativeFunctionCompiler, MechFunctionImpl, Register, CompileCtx, MechFunctionCompiler,
  hash_str, ModuleManifestCatalog, ModuleManifestConfig,
};

use mech_program::{
  MechProgram, MechProgramConfig, MechProgramEnvironment
};

use crate::capability::{
  BasicCapabilityKernel, Capability, CapabilityGrant, CapabilityKernel,
  CapabilityRequest, CapabilityRevocation,
};

use crate::config::RuntimeConfig;

use crate::context::{
  ResourceBudget, RuntimeContext, RuntimeContextBuilder, RuntimeTurnOutcome, RuntimeContextRegistry,
  RuntimeContextBase, RuntimeContextBinding, RuntimeContextCapabilityScope
};

use crate::event::{
  RuntimeEvent, RuntimeEventKind,
};

use crate::host::{
  default_host_capability_request, DefaultHostCallPolicy, HostCall, HostCallPolicy, HostFunction,
  HostFunctionNotFoundError, HostRegistry, InMemoryHostRegistry,
};

use crate::id::{
  module_id, ActorId, CapabilityId, DefaultIdGenerator,
  EventId, IdGenerator, MessageId, ModuleId, ModuleVersionId, ObjectId,
  RuntimeId, TaskId, TransactionId,
};

use crate::resolver::{
  InMemorySourceResolver, ResolvedSource, SourceRequest, SourceResolver,
  SourceAddressReference, SourceExportDeclaration, SourceImportAlias,
  SourceImportKind, SourceScope, module_namespace_for_import,
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
  RuntimeTransaction, RuntimeTransactionNotFoundError,
};

use crate::actor::ActorTurn;

use crate::{RuntimeServices};

use crate::actor_behavior::{
  ActorBehaviorDriver, ActorBehaviorRuntime, NoActorBehaviorDriver,
};

use crate::module::{ModuleBuilder, ModuleBuildOptions, ModuleDependencyGraph};

use crate::{register_config_spec_grants, register_config_spec_resources, HostInstanceConfig, HostInterfaceCatalog, InMemoryDocsProvider, RunResourceGrantConfig, RuntimeCapabilityGrant, RuntimeCapabilityGrantInput, RuntimeCapabilityGrantRegistry, RuntimeCapabilityOperation, RuntimeConfigSpec, RuntimeHostFactory, RuntimeHostFactoryRegistry, RuntimeResourceCapabilityDenied, RuntimeCapabilityGrantDenied, resource_base_matches, RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceRegistry, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest};

thread_local! {
  static ACTIVE_RUNTIME_PROGRAM_HOST: RefCell<Option<RuntimeProgramHostTarget>> =
    RefCell::new(None);
}


fn default_host_interfaces() -> MResult<HostInterfaceCatalog> {
  let mut catalog = HostInterfaceCatalog::new();
  catalog.register(crate::MaterializedHostInterface {
    instance: "cli".to_string(),
    provider: "cli".to_string(),
    contexts: vec![
      crate::MaterializedHostContext { name: "env".to_string(), base_uri: "cli://cli/env".to_string(), operations: vec!["read".to_string()] },
      crate::MaterializedHostContext { name: "stdout".to_string(), base_uri: "cli://cli/stdout".to_string(), operations: vec!["write".to_string()] },
      crate::MaterializedHostContext { name: "stderr".to_string(), base_uri: "cli://cli/stderr".to_string(), operations: vec!["write".to_string()] },
    ],
  })?;
  catalog.register(crate::MaterializedHostInterface {
    instance: "browser".to_string(),
    provider: "browser".to_string(),
    contexts: vec![crate::MaterializedHostContext { name: "dom".to_string(), base_uri: mech_core::BROWSER_DOM_PROVIDER_URI.to_string(), operations: vec!["read".to_string(), "write".to_string()] }],
  })?;
  Ok(catalog)
}

// -----------------------------------------------------------------------------
// Runtime Builder
// -----------------------------------------------------------------------------

pub struct RuntimeBuilder {
  config: RuntimeConfig,
  id_generator: Box<dyn IdGenerator>,
  store: Box<dyn MechStore>,
  capability_kernel: Box<dyn CapabilityKernel>,
  source_resolver: Box<dyn SourceResolver>,
  host_registry: Box<dyn HostRegistry>,
  host_policy: Box<dyn HostCallPolicy>,
  scheduler: Box<dyn Scheduler>,
  scheduler_policy: SchedulerPolicy,
  actor_behavior_driver: Box<dyn ActorBehaviorDriver>,
  module_builder: ModuleBuilder,
  config_specs: Vec<RuntimeConfigSpec>,
  resource_providers: Vec<Box<dyn RuntimeResourceProvider>>,
  host_factories: RuntimeHostFactoryRegistry,
  host_instances: Vec<HostInstanceConfig>,
  run_grants: Vec<RunResourceGrantConfig>,
  module_manifests: ModuleManifestCatalog,
}

impl std::fmt::Debug for RuntimeBuilder {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    f.debug_struct("RuntimeBuilder")
      .field("config", &self.config)
      .field("id_generator", &"<dyn IdGenerator>")
      .field("store", &"<dyn MechStore>")
      .field("capability_kernel", &"<dyn CapabilityKernel>")
      .field("source_resolver", &"<dyn SourceResolver>")
      .field("host_registry", &"<dyn HostRegistry>")
      .field("host_policy", &"<dyn HostCallPolicy>")
      .field("scheduler", &"<dyn Scheduler>")
      .field("scheduler_policy", &self.scheduler_policy)
      .field("actor_behavior_driver", &"<dyn ActorBehaviorDriver>")
      .field("module_builder", &self.module_builder)
      .field("config_specs", &self.config_specs)
      .field("resource_providers", &self.resource_providers.len())
      .field("host_factories", &self.host_factories)
      .field("host_instances", &self.host_instances)
      .field("run_grants", &self.run_grants)
      .field("module_manifests", &self.module_manifests)
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
      host_registry: Box::new(InMemoryHostRegistry::new()),
      host_policy: Box::new(DefaultHostCallPolicy),
      scheduler: Box::new(InMemoryScheduler::new()),
      scheduler_policy: SchedulerPolicy::default(),
      actor_behavior_driver: Box::new(NoActorBehaviorDriver::new()),
      module_builder: ModuleBuilder::new(),
      config_specs: Vec::new(),
      resource_providers: Vec::new(),
      host_factories: RuntimeHostFactoryRegistry::new(),
      host_instances: Vec::new(),
      run_grants: Vec::new(),
      module_manifests: ModuleManifestCatalog::new(),
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

  pub fn host_registry(
    mut self,
    host_registry: impl HostRegistry + 'static,
  ) -> Self {
    self.host_registry = Box::new(host_registry);
    self
  }

  pub fn host_policy(
    mut self,
    host_policy: impl HostCallPolicy + 'static,
  ) -> Self {
    self.host_policy = Box::new(host_policy);
    self
  }

  pub fn scheduler(
    mut self,
    scheduler: impl Scheduler + 'static,
  ) -> Self {
    self.scheduler = Box::new(scheduler);
    self
  }

  pub fn scheduler_policy(mut self, scheduler_policy: SchedulerPolicy) -> Self {
    self.scheduler_policy = scheduler_policy;
    self
  }

  pub fn actor_behavior_driver(
    mut self,
    actor_behavior_driver: impl ActorBehaviorDriver + 'static,
  ) -> Self {
    self.actor_behavior_driver = Box::new(actor_behavior_driver);
    self
  }

  pub fn module_builder(mut self, module_builder: ModuleBuilder) -> Self {
    self.module_builder = module_builder;
    self
  }

  pub fn config_spec(
    mut self,
    spec: RuntimeConfigSpec,
  ) -> Self {
    self.config_specs.push(spec);
    self
  }

  pub fn module_manifest(mut self, manifest: ModuleManifestConfig) -> MResult<Self> {
    self.module_manifests.register(manifest)?;
    Ok(self)
  }

  pub fn host_factory(mut self, factory: Box<dyn RuntimeHostFactory>) -> MResult<Self> {
    self.host_factories.register(factory)?;
    Ok(self)
  }

  pub fn host_instance(mut self, config: HostInstanceConfig) -> Self {
    self.host_instances.push(config);
    self
  }

  pub fn run_resource_grant(mut self, grant: RunResourceGrantConfig) -> Self {
    self.run_grants.push(grant);
    self
  }

  pub fn resource_provider(
    mut self,
    provider: Box<dyn RuntimeResourceProvider>,
  ) -> Self {
    self.resource_providers.push(provider);
    self
  }

  pub fn in_memory_docs(
    mut self,
    provider: InMemoryDocsProvider,
  ) -> Self {
    self.resource_providers.push(Box::new(provider));
    self
  }

  pub fn build(mut self) -> MResult<MechRuntime> {
    self.config.validate()?;
    self.scheduler_policy.validate()?;

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

    let mut host_interfaces = default_host_interfaces()?;
    for host_instance in &self.host_instances {
      if let Some(existing) = host_interfaces.interface(&host_instance.name) {
        if existing.provider != host_instance.provider {
          return Err(MechError::new(RuntimeInvalidOperationError {
            operation: "host_instance",
            reason: format!(
              "host instance `{}` is built in as provider `{}` and cannot be configured as provider `{}`",
              host_instance.name, existing.provider, host_instance.provider,
            ),
          }, None));
        }
      }
      let installation = self.host_factories.instantiate(host_instance)?;
      if host_interfaces.interface(&installation.interface.instance).is_none() {
        host_interfaces.register(installation.interface)?;
      }
      self.resource_providers.extend(installation.resource_providers);
    }

    let mut runtime = MechRuntime {
      id: runtime_id,
      event_sequence: 0,
      config: self.config,
      program: MechProgram::new(program_config),
      id_generator: self.id_generator,
      store: self.store,
      capability_kernel: self.capability_kernel,
      source_resolver: self.source_resolver,
      host_registry: self.host_registry,
      host_policy: self.host_policy,
      scheduler: self.scheduler,
      scheduler_policy: self.scheduler_policy,
      active_transactions: HashMap::new(),
      actor_behavior_driver: self.actor_behavior_driver,
      module_builder: self.module_builder,
      resources: RuntimeResourceRegistry::new(),
      grants: RuntimeCapabilityGrantRegistry::new(),
      resource_bindings: HashMap::new(),
      host_interfaces,
      module_manifests: self.module_manifests,
    };

    for spec in &self.config_specs {
      register_config_spec_resources(&mut runtime.resources, spec)?;
      register_config_spec_grants(&mut runtime.grants, spec)?;
    }

    for provider in self.resource_providers {
      runtime.register_resource_provider(provider)?;
    }

    for grant in &self.run_grants {
      runtime.install_run_resource_grant(grant)?;
    }

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
  host_registry: Box<dyn HostRegistry>,
  host_policy: Box<dyn HostCallPolicy>,
  scheduler: Box<dyn Scheduler>,
  scheduler_policy: SchedulerPolicy,
  active_transactions: HashMap<TransactionId, RuntimeTransaction>,
  actor_behavior_driver: Box<dyn ActorBehaviorDriver>,
  module_builder: ModuleBuilder,
  resources: RuntimeResourceRegistry,
  grants: RuntimeCapabilityGrantRegistry,
  resource_bindings: HashMap<String, RuntimeResourceBinding>,
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
      .field("actor_behavior_driver", &"<dyn ActorBehaviorDriver>")
      .field("module_builder", &self.module_builder)
      .field("resources", &self.resources)
      .field("grants", &self.grants)
      .field("resource_bindings", &self.resource_bindings)
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
pub struct BrowserRuntimeResourceError {
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for BrowserRuntimeResourceError {
  fn name(&self) -> &str {
    "BrowserRuntimeResource"
  }

  fn message(&self) -> String {
    format!("browser runtime resource `{}` failed: {}", self.resource, self.reason)
  }
}

fn browser_runtime_resource_error(
  resource: impl Into<String>,
  reason: impl Into<String>,
) -> MechError {
  MechError::new(
    BrowserRuntimeResourceError {
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
pub struct ModuleInstance {
  pub version: ModuleVersionId,
  pub exports: HashMap<String, mech_core::ValRef>,
  pub result: Value,
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

  pub fn bind_context_export(
    &mut self,
    alias: &str,
    module: &str,
    item: &str,
  ) -> MResult<()> {
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

  pub fn bind_resource_root(
    &mut self,
    name: impl Into<String>,
    uri: impl AsRef<str>,
  ) -> MResult<()> {
    let name = name.into();
    if !validate_resource_binding_name(&name) {
      return Err(browser_runtime_resource_error(
        name,
        "resource binding names must be non-empty simple tokens",
      ));
    }

    let uri = uri.as_ref().trim_end_matches('/');
    let mut base_uri = uri.to_string();
    let mut root_path = String::new();
    if resource_base_matches(BROWSER_DOM_PROVIDER_URI, uri) {
      base_uri = BROWSER_DOM_PROVIDER_URI.to_string();
      root_path = uri
        .strip_prefix(BROWSER_DOM_PROVIDER_URI)
        .unwrap_or_default()
        .trim_matches('/')
        .to_string();
      if !root_path.is_empty() {
        BrowserDomPath::new(root_path.clone()).map_err(browser_capability_error)?;
      }
    }

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
      return Err(browser_runtime_resource_error(
        binding,
        "unknown resource root binding",
      ));
    };

    let child_path = child_path.trim_matches('/');
    let full_path = if binding_record.root_path.is_empty() {
      child_path.to_string()
    } else if child_path.is_empty() {
      binding_record.root_path.clone()
    } else {
      format!("{}/{}", binding_record.root_path, child_path)
    };
    Ok((binding_record.base_uri.clone(), full_path))
  }

  pub fn resolve_resource_path(
    &self,
    binding: &str,
    child_path: &str,
  ) -> MResult<BrowserDomPath> {
    let (base_uri, full_path) = self.resolve_bound_resource_parts(binding, child_path)?;
    if base_uri != BROWSER_DOM_PROVIDER_URI {
      return Err(browser_runtime_resource_error(
        binding,
        "only browser DOM resource bindings are supported",
      ));
    }
    BrowserDomPath::new(full_path).map_err(browser_capability_error)
  }

  pub fn read_bound_resource(
    &self,
    binding: &str,
    child_path: &str,
  ) -> MResult<Value> {
    let (base_uri, path) = self.resolve_bound_resource_parts(binding, child_path)?;
    self.resources.read(RuntimeResourceReadRequest {
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
    self.resources.write(RuntimeResourceWriteRequest {
      base_uri,
      path,
      context_name: binding.to_string(),
      operation: RuntimeCapabilityOperation::Write,
      value: value.clone(),
      intent: RuntimeResourceWriteIntent::Assign,
    })
  }

  pub fn read_browser_dom_resource(
    &self,
    binding: &str,
    child_path: &str,
  ) -> MResult<Value> {
    self.read_bound_resource(binding, child_path)
  }

  pub fn write_browser_dom_resource(
    &mut self,
    binding: &str,
    child_path: &str,
    value: &Value,
  ) -> MResult<()> {
    self.write_bound_resource(binding, child_path, value)
  }

  pub fn apply_config_spec(
    &mut self,
    spec: RuntimeConfigSpec,
  ) -> MResult<()> {
    register_config_spec_resources(&mut self.resources, &spec)?;
    register_config_spec_grants(&mut self.grants, &spec)?;
    Ok(())
  }

  pub fn grant_capability<G>(&mut self, grant: G) -> MResult<G::Output>
  where
    G: RuntimeCapabilityGrantInput,
  {
    grant.apply(self)
  }

  pub(crate) fn add_resource_capability_grant(
    &mut self,
    grant: RuntimeCapabilityGrant,
  ) -> MResult<()> {
    self.grants.add_grant(grant)
  }

  pub fn has_capability_grant(
    &self,
    subject: &str,
    resource: &str,
    operation: &RuntimeCapabilityOperation,
    path: &str,
  ) -> bool {
    self.grants.allows(subject, resource, operation, path)
  }

  pub fn install_run_resource_grant(
    &mut self,
    grant: &RunResourceGrantConfig,
  ) -> MResult<()> {
    let context = self.host_interfaces.resolve(&grant.target)?;
    for operation in &grant.operations {
      if !context.operations.iter().any(|allowed| allowed == operation) {
        return Err(MechError::new(RuntimeInvalidOperationError {
          operation: "install_run_resource_grant",
          reason: format!("host context `{}` does not expose operation `{operation}`", grant.target),
        }, None));
      }
    }
    let operations = grant.operations.iter().map(|operation| RuntimeCapabilityOperation::from_name(operation.clone())).collect::<MResult<Vec<_>>>()?;
    self.grants.add_grant(RuntimeCapabilityGrant {
      subject: format!("runtime:{}", self.id),
      resource: context.base_uri.clone(),
      operations,
      paths: grant.paths.clone(),
    })
  }

  pub fn register_resource_provider(
    &mut self,
    provider: Box<dyn RuntimeResourceProvider>,
  ) -> MResult<()> {
    self.resources.register_provider(provider)
  }

  pub fn has_resource_provider(&self, scheme: &str) -> bool {
    self.resources.has_provider(scheme)
  }

  pub fn write_resource(
    &mut self,
    request: RuntimeResourceWriteRequest,
  ) -> MResult<()> {
    self.resources.write(request)
  }

  pub fn read_resource(
    &self,
    request: RuntimeResourceReadRequest,
  ) -> MResult<Value> {
    self.resources.read(request)
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

  pub fn host_registry(&self) -> &dyn HostRegistry {
    self.host_registry.as_ref()
  }

  pub fn host_registry_mut(&mut self) -> &mut dyn HostRegistry {
    self.host_registry.as_mut()
  }

  pub fn host_policy(&self) -> &dyn HostCallPolicy {
    self.host_policy.as_ref()
  }

  pub fn host_policy_mut(&mut self) -> &mut dyn HostCallPolicy {
    self.host_policy.as_mut()
  }

  pub fn scheduler(&self) -> &dyn Scheduler {
    self.scheduler.as_ref()
  }

  pub fn scheduler_mut(&mut self) -> &mut dyn Scheduler {
    self.scheduler.as_mut()
  }

  pub fn scheduler_policy(&self) -> &SchedulerPolicy {
    &self.scheduler_policy
  }

  pub fn scheduler_policy_mut(&mut self) -> &mut SchedulerPolicy {
    &mut self.scheduler_policy
  }

  pub fn actor_behavior_driver(&self) -> &dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_ref()
  }

  pub fn actor_behavior_driver_mut(&mut self) -> &mut dyn ActorBehaviorDriver {
    self.actor_behavior_driver.as_mut()
  }

  pub fn module_builder(&self) -> &ModuleBuilder {
    &self.module_builder
  }

  pub fn set_scheduler_policy(&mut self, scheduler_policy: SchedulerPolicy) -> MResult<()> {
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
