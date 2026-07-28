use super::*;

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
  pub(in crate::runtime) id: RuntimeId,
  pub(in crate::runtime) event_sequence: u64,
  pub(in crate::runtime) config: RuntimeConfig,
  pub(in crate::runtime) program: MechProgram,
  pub(in crate::runtime) id_generator: Box<dyn IdGenerator>,
  pub(in crate::runtime) store: Box<dyn MechStore>,
  pub(in crate::runtime) capability_kernel: Box<dyn CapabilityKernel>,
  pub(in crate::runtime) source_resolver: Box<dyn SourceResolver>,
  pub(in crate::runtime) host_registry: Box<dyn HostRegistry>,
  pub(in crate::runtime) host_policy: Box<dyn HostCallPolicy>,
  pub(in crate::runtime) scheduler: Box<dyn Scheduler>,
  pub(in crate::runtime) scheduler_policy: SchedulerPolicy,
  pub(in crate::runtime) active_transactions: HashMap<TransactionId, RuntimeExecutionTransaction>,
  pub(in crate::runtime) program_transaction_owner: Option<TransactionId>,
  pub(in crate::runtime) active_program_operation: Rc<Cell<Option<ActiveRuntimeProgramOperation>>>,
  pub(in crate::runtime) active_effect_phase: Rc<Cell<Option<ActiveRuntimeEffectPhase>>>,
  pub(in crate::runtime) health: RuntimeHealth,
  pub(in crate::runtime) actor_behavior_driver: Box<dyn ActorBehaviorDriver>,
  pub(in crate::runtime) module_builder: ModuleBuilder,
  pub(in crate::runtime) resources: RuntimeResourceRegistry,
  pub(in crate::runtime) resource_bindings: HashMap<String, RuntimeResourceBinding>,
  pub(in crate::runtime) live_registration_mode: LiveRegistrationMode,
  pub(in crate::runtime) live_input_bindings:
    HashMap<crate::RuntimeHostInputSource, Vec<ProgramInputId>>,
  pub(in crate::runtime) host_input_queue: RuntimeHostInputQueue,
  pub(in crate::runtime) input_drivers: Vec<Box<dyn RuntimeHostInputDriver>>,
  pub(in crate::runtime) attached_input_driver_count: usize,
  pub(in crate::runtime) persistent_sends: Vec<RuntimePersistentSend>,
  pub(in crate::runtime) live_context_template: Option<RuntimeLiveContextTemplate>,
  pub(in crate::runtime) input_driver_cleanup_armed: bool,
  pub(in crate::runtime) host_interfaces: HostInterfaceCatalog,
  pub(in crate::runtime) module_manifests: ModuleManifestCatalog,
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
      .field("active_effect_phase", &self.active_effect_phase.get())
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

#[derive(Clone, Debug)]
pub(in crate::runtime) struct ModuleInstance {
  pub(in crate::runtime) version: ModuleVersionId,
  pub(in crate::runtime) exports: HashMap<String, mech_core::ValRef>,
  pub(in crate::runtime) result: Value,
}

impl ModuleInstance {
  pub(in crate::runtime) fn detached_result(&self) -> RuntimeModuleResult {
    RuntimeModuleResult {
      version: self.version,
      exports: self
        .exports
        .iter()
        .map(|(name, value)| (name.clone(), RuntimeValueSnapshot::capture(&value.borrow())))
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
      for driver in self.input_drivers[..self.attached_input_driver_count]
        .iter_mut()
        .rev()
      {
        let _ = extension::catch_extension("host input driver", "stop", || driver.stop());
      }
      self.input_driver_cleanup_armed = false;
    }
  }
}

pub(in crate::runtime) fn validate_module_import_edges(
  record: &ModuleVersionRecord,
) -> MResult<()> {
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
