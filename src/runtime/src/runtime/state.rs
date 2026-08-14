use super::builder::RuntimeBuilder;
use super::resources::RuntimeResourceBinding;
use super::transaction::{ActiveRuntimeTransaction, RuntimeHealth};
use crate::{
    ActiveRuntimeEffectPhase, CapabilityKernel, HostCallPolicy, HostInterfaceCatalog, HostRegistry,
    IdGenerator, MechStore, ModuleBuilder, RuntimeConfig, RuntimeHostInputDriver,
    RuntimeHostInputQueue, RuntimeId, RuntimeResourceRegistry, Scheduler, SchedulerPolicy,
    SourceResolver, TransactionId,
};
use mech_core::FunctionCatalog;
use mech_core::{MResult, ModuleManifestCatalog};
use std::cell::Cell;
use std::collections::HashMap;
use std::rc::Rc;
use std::sync::Arc;

#[cfg(feature = "resident-routing")]
use crate::runtime::program::ResidentProductionProbe;
#[cfg(feature = "resident-routing")]
use crate::runtime::program::{ActiveProgramExecution, RuntimeProgramExecutionInfo};

pub(in crate::runtime) struct ScopedRuntimeState<T: Copy> {
    state: Rc<Cell<Option<T>>>,
    previous: Option<T>,
}

impl<T: Copy> ScopedRuntimeState<T> {
    pub(in crate::runtime) fn enter(state: &Rc<Cell<Option<T>>>, value: T) -> Self {
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
    pub(super) id: RuntimeId,
    pub(super) event_sequence: u64,
    pub(super) config: RuntimeConfig,
    pub(super) function_catalog: Arc<FunctionCatalog>,
    #[cfg(feature = "resident-routing")]
    pub(super) active_program: ActiveProgramExecution,
    #[cfg(feature = "resident-routing")]
    pub(super) program_execution_info: RuntimeProgramExecutionInfo,
    #[cfg(feature = "resident-routing")]
    pub(super) next_resident_instance: u32,
    #[cfg(feature = "resident-routing")]
    pub(super) resident_production_probe: ResidentProductionProbe,
    pub(super) id_generator: Box<dyn IdGenerator>,
    pub(super) store: Box<dyn MechStore>,
    pub(super) capability_kernel: Box<dyn CapabilityKernel>,
    pub(super) source_resolver: Box<dyn SourceResolver>,
    pub(super) host_registry: Box<dyn HostRegistry>,
    pub(super) host_policy: Box<dyn HostCallPolicy>,
    pub(super) scheduler: Box<dyn Scheduler>,
    pub(super) scheduler_policy: SchedulerPolicy,
    pub(super) active_transactions: HashMap<TransactionId, ActiveRuntimeTransaction>,
    pub(super) active_effect_phase: Rc<Cell<Option<ActiveRuntimeEffectPhase>>>,
    pub(super) health: RuntimeHealth,
    pub(super) module_builder: ModuleBuilder,
    pub(super) resources: RuntimeResourceRegistry,
    pub(super) resource_bindings: HashMap<String, RuntimeResourceBinding>,
    pub(super) host_input_queue: RuntimeHostInputQueue,
    pub(super) input_drivers: Vec<Box<dyn RuntimeHostInputDriver>>,
    pub(super) attached_input_driver_count: usize,
    pub(super) input_driver_cleanup_armed: bool,
    pub(super) host_interfaces: HostInterfaceCatalog,
    pub(super) module_manifests: ModuleManifestCatalog,
}

impl std::fmt::Debug for MechRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MechRuntime")
            .field("id", &self.id)
            .field("event_sequence", &self.event_sequence)
            .field("config", &self.config)
            .field("function_catalog", &"<FunctionCatalog>")
            .field("active_program", &self.program_route_for_debug())
            .field("id_generator", &"<dyn IdGenerator>")
            .field("store", &"<dyn MechStore>")
            .field("capability_kernel", &"<dyn CapabilityKernel>")
            .field("source_resolver", &"<dyn SourceResolver>")
            .field("host_registry", &"<dyn HostRegistry>")
            .field("host_policy", &"<dyn HostCallPolicy>")
            .field("scheduler", &"<dyn Scheduler>")
            .field("scheduler_policy", &self.scheduler_policy)
            .field("active_transactions", &self.active_transactions.len())
            .field("active_effect_phase", &self.active_effect_phase.get())
            .field("module_builder", &self.module_builder)
            .field("resources", &self.resources)
            .field("resource_bindings", &self.resource_bindings)
            .field("input_drivers", &self.input_drivers.len())
            .field("host_interfaces", &self.host_interfaces)
            .field("module_manifests", &self.module_manifests)
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

    pub(crate) fn health(&self) -> &RuntimeHealth {
        &self.health
    }

    pub fn runtime_health(&self) -> RuntimeHealth {
        self.health.clone()
    }

    pub fn is_poisoned(&self) -> bool {
        matches!(self.health, RuntimeHealth::Poisoned(_))
    }
}

#[cfg(not(feature = "resident-routing"))]
impl MechRuntime {
    fn program_route_for_debug(&self) -> &'static str {
        "unavailable"
    }
}
