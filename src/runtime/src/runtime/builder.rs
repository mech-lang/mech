//! Runtime construction and dependency assembly.

use super::extension;
use super::live_state::LiveRegistrationMode;
use super::resources::{runtime_resource_binding_error, validate_resource_binding_name};
use super::transaction::RuntimeHealth;
use super::{MechRuntime, RuntimeExecutionMode};
use crate::{
    ActorBehaviorDriver, BasicCapabilityKernel, CapabilityKernel, DEFAULT_HOST_INPUT_CAPACITY,
    DefaultHostCallPolicy, DefaultIdGenerator, HostCallPolicy, HostInstanceConfig,
    HostInterfaceCatalog, HostRegistry, IdGenerator, InMemoryDocsProvider, InMemoryHostRegistry,
    InMemoryScheduler, InMemorySourceResolver, InMemoryStore, MechStore, ModuleBuilder,
    NoActorBehaviorDriver, RegisteredHostFunction, RunResourceGrantConfig, RuntimeConfig,
    RuntimeConfigSpec, RuntimeEventKind, RuntimeHostFactory, RuntimeHostFactoryRegistry,
    RuntimeHostInputDriver, RuntimeHostInputQueueState, RuntimeResourceProvider,
    RuntimeResourceRegistry, Scheduler, SchedulerPolicy, SourceResolver,
    materialize_config_spec_grants, register_config_spec_resources,
};
use mech_core::FunctionCatalog;
use mech_core::{MResult, ModuleManifestCatalog, ModuleManifestConfig};
use mech_engine::{MechProgram, MechProgramConfig, MechProgramEnvironment};
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;
use std::sync::Arc;

// -----------------------------------------------------------------------------
// Runtime Builder
// -----------------------------------------------------------------------------

pub struct RuntimeBuilder {
    config: RuntimeConfig,
    execution_mode: RuntimeExecutionMode,
    function_catalog: Arc<FunctionCatalog>,
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
    input_drivers: Vec<Box<dyn RuntimeHostInputDriver>>,
    host_input_capacity: usize,
    host_factories: RuntimeHostFactoryRegistry,
    host_instances: Vec<HostInstanceConfig>,
    run_grants: Vec<RunResourceGrantConfig>,
    module_manifests: ModuleManifestCatalog,
    resource_bindings: Vec<(String, String)>,
    context_export_bindings: Vec<(String, String, String)>,
}

impl std::fmt::Debug for RuntimeBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let function_catalog = "<FunctionCatalog>";

        f.debug_struct("RuntimeBuilder")
            .field("config", &self.config)
            .field("execution_mode", &self.execution_mode)
            .field("function_catalog", &function_catalog)
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
            .field("resource_bindings", &self.resource_bindings)
            .field("context_export_bindings", &self.context_export_bindings)
            .finish()
    }
}

impl Default for RuntimeBuilder {
    fn default() -> Self {
        Self {
            config: RuntimeConfig::default(),
            execution_mode: RuntimeExecutionMode::Execute,
            function_catalog: mech_engine::empty_function_catalog(),
            id_generator: Box::new(DefaultIdGenerator::new()),
            store: Box::new(extension::RuntimeStoreBoundary::new(Box::new(
                InMemoryStore::new(),
            ))),
            capability_kernel: Box::new(extension::RuntimeCapabilityKernelBoundary::new(Box::new(
                BasicCapabilityKernel::new(),
            ))),
            source_resolver: Box::new(InMemorySourceResolver::new()),
            host_registry: Box::new(InMemoryHostRegistry::new()),
            host_policy: Box::new(DefaultHostCallPolicy),
            scheduler: Box::new(InMemoryScheduler::new()),
            scheduler_policy: SchedulerPolicy::default(),
            actor_behavior_driver: Box::new(NoActorBehaviorDriver::new()),
            module_builder: ModuleBuilder::new(),
            config_specs: Vec::new(),
            resource_providers: Vec::new(),
            input_drivers: Vec::new(),
            host_input_capacity: DEFAULT_HOST_INPUT_CAPACITY,
            host_factories: RuntimeHostFactoryRegistry::new(),
            host_instances: Vec::new(),
            run_grants: Vec::new(),
            module_manifests: ModuleManifestCatalog::new(),
            resource_bindings: Vec::new(),
            context_export_bindings: Vec::new(),
        }
    }
}

impl RuntimeBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn input_driver(mut self, driver: impl RuntimeHostInputDriver + 'static) -> Self {
        self.input_drivers.push(Box::new(driver));
        self
    }

    #[cfg(test)]
    pub(crate) fn test_input_driver(self, driver: impl RuntimeHostInputDriver + 'static) -> Self {
        self.input_driver(driver)
    }

    pub fn config(mut self, config: RuntimeConfig) -> Self {
        self.config = config;
        self
    }

    pub fn planning(mut self) -> Self {
        self.execution_mode = RuntimeExecutionMode::Plan;
        self
    }

    pub fn function_catalog(mut self, catalog: Arc<FunctionCatalog>) -> Self {
        self.function_catalog = catalog;
        self
    }

    pub fn id_generator(mut self, id_generator: impl IdGenerator + 'static) -> Self {
        self.id_generator = Box::new(id_generator);
        self
    }

    pub fn store(mut self, store: impl MechStore + 'static) -> Self {
        self.store = Box::new(extension::RuntimeStoreBoundary::new(Box::new(store)));
        self
    }

    pub fn capability_kernel(mut self, capability_kernel: impl CapabilityKernel + 'static) -> Self {
        self.capability_kernel = Box::new(extension::RuntimeCapabilityKernelBoundary::new(
            Box::new(capability_kernel),
        ));
        self
    }

    pub fn source_resolver(mut self, source_resolver: impl SourceResolver + 'static) -> Self {
        self.source_resolver = Box::new(source_resolver);
        self
    }

    pub fn host_registry(mut self, host_registry: impl HostRegistry + 'static) -> Self {
        self.host_registry = Box::new(host_registry);
        self
    }

    pub fn host_function(mut self, function: impl Into<RegisteredHostFunction>) -> MResult<Self> {
        let function = function.into();
        extension::invoke_extension("host registry", "register_function", || {
            self.host_registry.register_function(function)
        })?;
        Ok(self)
    }

    pub fn host_policy(mut self, host_policy: impl HostCallPolicy + 'static) -> Self {
        self.host_policy = Box::new(host_policy);
        self
    }

    pub fn scheduler(mut self, scheduler: impl Scheduler + 'static) -> Self {
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

    pub fn config_spec(mut self, spec: RuntimeConfigSpec) -> Self {
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

    pub fn resource_binding(
        mut self,
        name: impl Into<String>,
        uri: impl Into<String>,
    ) -> MResult<Self> {
        let name = name.into();
        if !validate_resource_binding_name(&name) {
            return Err(runtime_resource_binding_error(
                name,
                "resource binding names must be non-empty simple tokens",
            ));
        }
        self.resource_bindings.push((name, uri.into()));
        Ok(self)
    }

    pub fn context_export_binding(
        mut self,
        alias: impl Into<String>,
        module: impl Into<String>,
        item: impl Into<String>,
    ) -> MResult<Self> {
        let alias = alias.into();
        if !validate_resource_binding_name(&alias) {
            return Err(runtime_resource_binding_error(
                alias,
                "context export aliases must be non-empty simple tokens",
            ));
        }
        self.context_export_bindings
            .push((alias, module.into(), item.into()));
        Ok(self)
    }

    pub fn resource_provider(mut self, provider: Box<dyn RuntimeResourceProvider>) -> Self {
        self.resource_providers.push(provider);
        self
    }

    pub fn in_memory_docs(mut self, provider: InMemoryDocsProvider) -> Self {
        self.resource_providers.push(Box::new(provider));
        self
    }

    pub fn host_input_capacity(mut self, capacity: usize) -> Self {
        self.host_input_capacity = capacity;
        self
    }

    pub fn build(mut self) -> MResult<MechRuntime> {
        self.config.validate()?;
        self.scheduler_policy.validate()?;
        if self.host_input_capacity == 0 {
            return Err(crate::input::input_error(
                "RuntimeHostInputCapacityInvalid",
                "host input queue capacity must be greater than zero",
            ));
        }

        let runtime_id = self.id_generator.runtime_id();

        let program_config = MechProgramConfig {
            name: self.config.name.clone(),
            environment: MechProgramEnvironment {
                trace_enabled: self.config.diagnostics.trace_enabled,
                debug_enabled: self.config.diagnostics.debug_enabled,
                profile_enabled: self.config.diagnostics.profile_enabled,
                rounds_per_step: self.config.limits.max_steps_per_turn_as_usize()?,
            },
        };

        let mut host_interfaces = HostInterfaceCatalog::new();
        for host_instance in &self.host_instances {
            let installation = self.host_factories.instantiate(host_instance)?;
            host_interfaces.register(installation.interface)?;
            self.resource_providers
                .extend(installation.resource_providers);
            self.input_drivers.extend(installation.input_drivers);
        }

        let max_events = self
            .config
            .limits
            .max_in_memory_events
            .map(|value| usize::try_from(value).unwrap_or(usize::MAX));
        self.store.configure_event_retention(max_events)?;

        let function_catalog = Arc::clone(&self.function_catalog);
        let program =
            MechProgram::with_function_catalog(program_config, Arc::clone(&function_catalog));

        let mut runtime = MechRuntime {
            id: runtime_id,
            event_sequence: 0,
            config: self.config,
            execution_mode: self.execution_mode,
            function_catalog,
            program,
            #[cfg(feature = "resident-production")]
            active_program: Default::default(),
            #[cfg(feature = "resident-production")]
            program_execution_info: Default::default(),
            #[cfg(feature = "resident-production")]
            next_resident_instance: 1,
            #[cfg(feature = "resident-production")]
            resident_production_probe: Default::default(),
            id_generator: self.id_generator,
            store: self.store,
            capability_kernel: self.capability_kernel,
            source_resolver: self.source_resolver,
            host_registry: self.host_registry,
            host_policy: self.host_policy,
            scheduler: self.scheduler,
            scheduler_policy: self.scheduler_policy,
            active_transactions: HashMap::new(),
            program_transaction_owner: None,
            active_program_operation: Rc::new(Cell::new(None)),
            active_effect_phase: Rc::new(Cell::new(None)),
            health: RuntimeHealth::Healthy,
            actor_behavior_driver: self.actor_behavior_driver,
            module_builder: self.module_builder,
            resources: RuntimeResourceRegistry::new(),
            resource_bindings: HashMap::new(),
            external_requirements: Default::default(),
            live_registration_mode: LiveRegistrationMode::RetainedRoot,
            live_input_bindings: BTreeMap::new(),
            host_input_queue: std::sync::Arc::new(std::sync::Mutex::new(
                RuntimeHostInputQueueState::new(self.host_input_capacity),
            )),
            input_drivers: self.input_drivers,
            attached_input_driver_count: 0,
            live_context_template: None,
            input_driver_cleanup_armed: false,
            host_interfaces,
            module_manifests: self.module_manifests,
        };

        for spec in &self.config_specs {
            register_config_spec_resources(&mut runtime.resources, spec)?;
            let capabilities = materialize_config_spec_grants(runtime.id_generator.as_mut(), spec)?;
            for capability in capabilities {
                runtime.grant_capability(capability)?;
            }
        }

        for provider in self.resource_providers {
            runtime.register_resource_provider(provider)?;
        }

        for grant in &self.run_grants {
            runtime.install_run_resource_grant(grant)?;
        }

        for (name, uri) in self.resource_bindings {
            runtime.bind_resource_root(name, uri)?;
        }

        for (alias, module, item) in self.context_export_bindings {
            runtime.bind_context_export(&alias, &module, &item)?;
        }

        if runtime.execution_mode == RuntimeExecutionMode::Execute {
            let ingress = runtime.ingress();
            for index in 0..runtime.input_drivers.len() {
                if let Err(error) =
                    extension::invoke_extension("host input driver", "attach", || {
                        runtime.input_drivers[index].attach(ingress.clone())
                    })
                {
                    let _ = runtime.close_ingress();
                    let mut cleanup_failures = Vec::new();
                    for rollback_index in (0..=index).rev() {
                        if let Err(cleanup_error) =
                            extension::invoke_extension("host input driver", "stop", || {
                                runtime.input_drivers[rollback_index].stop()
                            })
                        {
                            cleanup_failures.push(format!(
                                "input driver {} stop failed: {:?}",
                                rollback_index, cleanup_error,
                            ));
                        }
                    }
                    runtime.attached_input_driver_count = 0;
                    runtime.input_driver_cleanup_armed = false;
                    if !cleanup_failures.is_empty() {
                        return Err(runtime.poison_program_operation(
                            "build",
                            None,
                            format!("{:?}", error),
                            cleanup_failures,
                        ));
                    }
                    return Err(error);
                }
                runtime.attached_input_driver_count += 1;
            }
            runtime.input_driver_cleanup_armed = true;
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

#[cfg(test)]
mod tests {
    use super::{MechProgramConfig, RuntimeBuilder, RuntimeExecutionMode};
    use mech_core::FunctionCatalogBuilder;
    use std::sync::Arc;

    #[test]
    fn execution_mode_defaults_to_execute_and_planning_selects_plan() {
        let execute = RuntimeBuilder::new().build().unwrap();
        let plan = RuntimeBuilder::new().planning().build().unwrap();

        assert_eq!(execute.execution_mode(), RuntimeExecutionMode::Execute);
        assert_eq!(plan.execution_mode(), RuntimeExecutionMode::Plan);
    }

    #[test]
    fn custom_function_catalog_reaches_retained_and_runtime_created_programs() {
        let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
        let runtime = RuntimeBuilder::new()
            .function_catalog(Arc::clone(&catalog))
            .build()
            .unwrap();

        assert!(Arc::ptr_eq(&runtime.function_catalog, &catalog));
        assert!(Arc::ptr_eq(runtime.program().function_catalog(), &catalog));

        let isolated = runtime.new_program(MechProgramConfig::default());
        assert!(Arc::ptr_eq(isolated.function_catalog(), &catalog));
    }

    #[test]
    fn bare_runtime_uses_an_empty_catalog() {
        let runtime = RuntimeBuilder::new().build().unwrap();

        assert_eq!(runtime.function_catalog.runtime_factory_count(), 0);
        assert_eq!(runtime.function_catalog.specializer_count(), 0);
        assert_eq!(runtime.function_catalog.intrinsic_specializer_count(), 0);
        assert_eq!(runtime.function_catalog.all_exports().len(), 0);
    }

    #[cfg(feature = "source")]
    #[test]
    fn bare_runtime_rejects_source_catalog_operations() {
        let mut runtime = RuntimeBuilder::new().build().unwrap();

        let error = runtime.run_string("x := 1").unwrap_err();

        assert_eq!(error.kind_name(), "FunctionOperationUnavailable");
        assert!(error.kind_message().contains("var/define"));
    }

    #[cfg(feature = "source")]
    #[test]
    fn injected_intrinsic_catalog_executes_source() {
        let mut catalog = FunctionCatalogBuilder::new();
        mech_engine::install_intrinsic_runtime(&mut catalog).unwrap();
        mech_engine::install_intrinsic_source(&mut catalog).unwrap();
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(Arc::new(catalog.build().unwrap()))
            .build()
            .unwrap();

        runtime.run_string("x := 1\nx").unwrap();
    }
}
