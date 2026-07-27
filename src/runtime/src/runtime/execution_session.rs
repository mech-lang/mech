use super::*;

use mech_core::MechExecutionServices;
use crate::{
  CapabilityRequest, HostFunctionTransactionMode,
  HostFunctionTransactionUnsupportedError,
  InvalidHostFunctionError, PreparedRuntimeEffect,
  RuntimePreparedHostCall,
};

pub(crate) struct RuntimeExecutionSession<'a> {
  pub(crate) runtime_id: RuntimeId,
  pub(crate) max_events: Option<usize>,
  pub(crate) context: &'a mut RuntimeContext,
  pub(crate) transaction: &'a mut RuntimeExecutionTransaction,
  pub(crate) id_generator: &'a mut dyn IdGenerator,
  pub(crate) store: &'a mut dyn MechStore,
  pub(crate) capability_kernel: &'a mut dyn CapabilityKernel,
  pub(crate) resources: &'a mut RuntimeResourceRegistry,
  pub(crate) host_registry: &'a dyn HostRegistry,
  pub(crate) host_policy: &'a dyn HostCallPolicy,
  pub(crate) event_sequence: &'a mut u64,
}

struct RuntimeSessionServices<'a> {
  runtime_id: RuntimeId,
  max_events: Option<usize>,
  transaction: &'a mut RuntimeExecutionTransaction,
  id_generator: &'a mut dyn IdGenerator,
  store: &'a mut dyn MechStore,
  capability_kernel: &'a mut dyn CapabilityKernel,
  resources: &'a mut RuntimeResourceRegistry,
  event_sequence: &'a mut u64,
}

impl RuntimeSessionServices<'_> {
  fn validate_context(
    &self,
    context: &RuntimeContext,
  ) -> MResult<()> {
    context.validate()?;
    if context.runtime != self.runtime_id {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "runtime_execution_session",
          reason: format!(
            "context runtime {} does not match runtime {}",
            context.runtime,
            self.runtime_id,
          ),
        },
        None,
      ));
    }
    if context.transaction != Some(self.transaction.store.id) {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "runtime_execution_session",
          reason: "execution context is not attached to the session transaction".to_string(),
        },
        None,
      ));
    }
    if self.transaction.state != RuntimeExecutionTransactionState::Active {
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "runtime_execution_session",
          reason: format!(
            "transaction {} is not active",
            self.transaction.store.id,
          ),
        },
        None,
      ));
    }
    Ok(())
  }

  fn trim_context_events(
    max_events: Option<usize>,
    events: &mut Vec<RuntimeEvent>,
  ) {
    let Some(max_events) = max_events else {
      return;
    };
    if events.len() > max_events {
      events.drain(0..(events.len() - max_events));
    }
  }

  fn emit_event(
    &mut self,
    context: &mut RuntimeContext,
    kind: RuntimeEventKind,
  ) -> MResult<EventId> {
    self.validate_context(context)?;
    let context_events_before = context.events.clone();
    let event = RuntimeEvent::new(
      self.id_generator.event_id(),
      {
        *self.event_sequence =
          (*self.event_sequence).saturating_add(1);
        *self.event_sequence
      },
      kind,
    );
    let id = event.id;
    context.push_event(event.clone());
    Self::trim_context_events(
      self.max_events,
      &mut context.events,
    );
    if let Err(error) = self.transaction.store.stage_event(event) {
      context.events = context_events_before;
      return Err(error);
    }
    Ok(id)
  }

  fn check_capability(
    &mut self,
    context: &mut RuntimeContext,
    request: &CapabilityRequest,
  ) -> MResult<CapabilityId> {
    self.validate_context(context)?;
    context.charge_step()?;
    if let Some(capability) =
      self.transaction.capabilities.check(request)?
    {
      return Ok(capability);
    }
    let revocations =
      self.transaction.capabilities.revocation_ids();
    let pending_uses =
      self.transaction.capabilities.pending_uses().clone();
    let capability = self
      .capability_kernel
      .preview_check_excluding_with_pending_uses(
        request,
        &revocations,
        &pending_uses,
      )?;
    self
      .transaction
      .capabilities
      .stage_use(capability)?;
    Ok(capability)
  }

  fn stage_effect(
    &mut self,
    context: &mut RuntimeContext,
    effect: PreparedRuntimeEffect,
  ) -> MResult<RuntimeEffectId> {
    self.validate_context(context)?;
    let cost = effect.cost();
    let metadata = effect.metadata();
    let protocol = effect.protocol();
    context.charge_bytes(cost.bytes)?;
    context.charge_items(cost.items)?;
    let store_before = self.transaction.store.clone();
    let effect_mark = self.transaction.effects.mark();
    let context_events_before = context.events.clone();
    let transaction_id = self.transaction.store.id;
    let effect_id = self
      .transaction
      .effects
      .stage(transaction_id, effect);
    if let Err(error) = self.emit_event(
      context,
      RuntimeEventKind::EffectStaged {
        effect_id,
        source: metadata.source,
        operation: metadata.operation,
        resource: metadata.resource,
        protocol,
      },
    ) {
      self.transaction.store = store_before;
      let cleanup =
        self.transaction.effects.rollback_to(effect_mark);
      context.events = context_events_before;
      if cleanup.is_empty() {
        return Err(error);
      }
      return Err(MechError::new(
        RuntimeInvalidOperationError {
          operation: "runtime_execution_session",
          reason: format!(
            "effect staging failed and cleanup was incomplete: original={error:?}; cleanup={cleanup:?}",
          ),
        },
        None,
      ));
    }
    Ok(effect_id)
  }
}

impl RuntimeServices for RuntimeSessionServices<'_> {
  fn next_object_id(&mut self) -> ObjectId {
    self.id_generator.object_id()
  }

  fn get_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ObjectId,
  ) -> MResult<Option<ObjectRecord>> {
    self.validate_context(context)?;
    context.record_read(id);
    if let Some(object) =
      self.transaction.store.get_staged_object(id)
    {
      return Ok(Some(object));
    }
    self.store.get_object(id)
  }

  fn put_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    self.validate_context(context)?;
    context.charge_bytes(object.data.len() as u64)?;
    let id = object.id;
    self.transaction.store.stage_put_object(object)?;
    context.record_write(id);
    self.emit_event(
      context,
      RuntimeEventKind::ObjectCreated {
        object_id: id,
      },
    )?;
    Ok(id)
  }

  fn update_object_with_context(
    &mut self,
    context: &mut RuntimeContext,
    object: ObjectRecord,
  ) -> MResult<ObjectId> {
    self.validate_context(context)?;
    context.charge_bytes(object.data.len() as u64)?;
    let id = object.id;
    self.transaction.store.stage_update_object(object)?;
    context.record_write(id);
    self.emit_event(
      context,
      RuntimeEventKind::ObjectUpdated {
        object_id: id,
      },
    )?;
    Ok(id)
  }

  fn get_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    id: ActorId,
  ) -> MResult<Option<ActorRecord>> {
    self.validate_context(context)?;
    if let Some(actor) =
      self.transaction.store.get_staged_actor(id)
    {
      return Ok(Some(actor));
    }
    self.store.get_actor(id)
  }

  fn update_actor_with_context(
    &mut self,
    context: &mut RuntimeContext,
    actor: ActorRecord,
  ) -> MResult<ActorId> {
    self.validate_context(context)?;
    let id = actor.id;
    self.transaction.store.stage_actor_update(actor)?;
    Ok(id)
  }
}

impl MechExecutionServices for RuntimeExecutionSession<'_> {
  fn invoke_native(
    &mut self,
    name: &str,
    arguments: &[Value],
  ) -> MResult<Value> {
    let Self {
      runtime_id,
      max_events,
      context,
      transaction,
      id_generator,
      store,
      capability_kernel,
      resources,
      host_registry,
      host_policy,
      event_sequence,
    } = self;
    let mut services = RuntimeSessionServices {
      runtime_id: *runtime_id,
      max_events: *max_events,
      transaction,
      id_generator: &mut **id_generator,
      store: &mut **store,
      capability_kernel: &mut **capability_kernel,
      resources,
      event_sequence,
    };
    services.validate_context(context)?;
    if name.trim().is_empty() {
      return Err(MechError::new(
        InvalidHostFunctionError {
          field: "name",
          reason: "must not be empty",
        },
        None,
      ));
    }
    services.emit_event(
      context,
      RuntimeEventKind::HostCallStarted {
        name: name.to_string(),
      },
    )?;
    let Some(function) = host_registry.get_function(name)? else {
      services.emit_event(
        context,
        RuntimeEventKind::HostCallFailed {
          name: name.to_string(),
          message: "host function not found".to_string(),
        },
      )?;
      return Err(MechError::new(
        HostFunctionNotFoundError {
          name: name.to_string(),
        },
        None,
      ));
    };

    let result = (|| -> MResult<Value> {
      let mode = function.transaction_mode();
      if mode == HostFunctionTransactionMode::ImmediateOnly {
        return Err(MechError::new(
          HostFunctionTransactionUnsupportedError {
            function: function.name().to_string(),
            mode,
          },
          None,
        ));
      }
      host_policy.validate_call(
        context,
        function.as_ref(),
        arguments,
      )?;
      context.charge_items(
        function.estimated_cost_items(arguments),
      )?;
      context.charge_bytes(
        function.estimated_cost_bytes(arguments),
      )?;
      let capability_request = function
        .required_capability(context)
        .unwrap_or_else(|| {
          default_host_capability_request(
            context,
            function.name(),
          )
        });
      services.check_capability(
        context,
        &capability_request,
      )?;
      match mode {
        HostFunctionTransactionMode::Staged => {
          let RuntimePreparedHostCall {
            value,
            effect,
          } = function.stage_call(
            &mut services,
            context,
            arguments.to_vec(),
          )?;
          services.stage_effect(context, effect)?;
          Ok(value)
        }
        HostFunctionTransactionMode::Pure
        | HostFunctionTransactionMode::RuntimeManaged => {
          function.call(
            &mut services,
            context,
            arguments.to_vec(),
          )
        }
        HostFunctionTransactionMode::ImmediateOnly => {
          unreachable!()
        }
      }
    })();

    match &result {
      Ok(_) => {
        services.emit_event(
          context,
          RuntimeEventKind::HostCallCompleted {
            name: name.to_string(),
          },
        )?;
      }
      Err(error) => {
        services.emit_event(
          context,
          RuntimeEventKind::HostCallFailed {
            name: name.to_string(),
            message: format!("{error:?}"),
          },
        )?;
      }
    }
    result
  }
}

impl MechRuntime {
  fn execution_session_max_events(&self) -> Option<usize> {
    self
      .config
      .limits
      .max_in_memory_events
      .map(|max| usize::try_from(max).unwrap_or(usize::MAX))
  }

  pub(super) fn with_retained_program_execution_session<T>(
    &mut self,
    context: &mut RuntimeContext,
    execute: impl FnOnce(
      &mut MechProgram,
      &mut RuntimeExecutionSession<'_>,
    ) -> MResult<T>,
  ) -> MResult<T> {
    let transaction_id = Self::context_transaction_id(context)?;
    let max_events = self.execution_session_max_events();
    let MechRuntime {
      id,
      event_sequence,
      program,
      id_generator,
      store,
      capability_kernel,
      host_registry,
      host_policy,
      active_transactions,
      resources,
      ..
    } = self;
    let transaction = active_transactions
      .get_mut(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError {
            transaction_id,
          },
          None,
        )
      })?;
    let mut session = RuntimeExecutionSession {
      runtime_id: *id,
      max_events,
      context,
      transaction,
      id_generator: id_generator.as_mut(),
      store: store.as_mut(),
      capability_kernel: capability_kernel.as_mut(),
      resources,
      host_registry: host_registry.as_ref(),
      host_policy: host_policy.as_ref(),
      event_sequence,
    };
    execute(program, &mut session)
  }

  pub(super) fn with_isolated_program_execution_session<T>(
    &mut self,
    context: &mut RuntimeContext,
    program: &mut MechProgram,
    execute: impl FnOnce(
      &mut MechProgram,
      &mut RuntimeExecutionSession<'_>,
    ) -> MResult<T>,
  ) -> MResult<T> {
    let transaction_id = Self::context_transaction_id(context)?;
    let max_events = self.execution_session_max_events();
    let MechRuntime {
      id,
      event_sequence,
      id_generator,
      store,
      capability_kernel,
      host_registry,
      host_policy,
      active_transactions,
      resources,
      ..
    } = self;
    let transaction = active_transactions
      .get_mut(&transaction_id)
      .ok_or_else(|| {
        MechError::new(
          RuntimeTransactionNotFoundError {
            transaction_id,
          },
          None,
        )
      })?;
    let mut session = RuntimeExecutionSession {
      runtime_id: *id,
      max_events,
      context,
      transaction,
      id_generator: id_generator.as_mut(),
      store: store.as_mut(),
      capability_kernel: capability_kernel.as_mut(),
      resources,
      host_registry: host_registry.as_ref(),
      host_policy: host_policy.as_ref(),
      event_sequence,
    };
    execute(program, &mut session)
  }
}
