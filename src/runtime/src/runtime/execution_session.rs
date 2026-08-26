use super::extension::{catch_extension, invoke_extension};
use super::transaction::{
    ActiveRuntimeTransaction, ActiveRuntimeTransactionState, check_transactional_capability,
};
use super::{MechRuntime, RuntimeInvalidOperationError};
use crate::{
    ActorId, ActorRecord, CapabilityId, CapabilityKernel, CapabilityRequest, EventId,
    HostCallPolicy, HostFunctionNotFoundError, HostRegistry, IdGenerator, InvalidHostFunctionError,
    MechStore, ObjectId, ObjectRecord, PreparedRuntimeEffect, RegisteredHostFunction,
    RuntimeCallContext, RuntimeCapabilityOperation, RuntimeContext, RuntimeEffectId, RuntimeEvent,
    RuntimeEventKind, RuntimeId, RuntimeManagedServices, RuntimePreparedHostCall,
    RuntimeResourceKey, RuntimeResourceReadRequest, RuntimeResourceRegistry,
    RuntimeResourceWriteIntent, RuntimeResourceWriteRequest, RuntimeTransactionNotFoundError,
    RuntimeValueSnapshot, default_host_capability_request,
};
use mech_core::{
    ExecutionHostFunctionRequest, ExecutionResourceRequest, LegacyValue, MResult, MechError,
    MechExecutionServices, ResourceIntent, ValueCell,
};

pub(crate) struct RuntimeExecutionSession<'a> {
    pub(crate) runtime_id: RuntimeId,
    pub(crate) max_events: Option<usize>,
    pub(crate) context: &'a mut RuntimeContext,
    pub(crate) transaction: &'a mut ActiveRuntimeTransaction,
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
    transaction: &'a mut ActiveRuntimeTransaction,
    id_generator: &'a mut dyn IdGenerator,
    store: &'a mut dyn MechStore,
    capability_kernel: &'a mut dyn CapabilityKernel,
    resources: &'a mut RuntimeResourceRegistry,
    event_sequence: &'a mut u64,
    context: &'a mut RuntimeContext,
}

impl RuntimeSessionServices<'_> {
    fn validate_context(&self) -> MResult<()> {
        let context = &*self.context;
        context.validate()?;
        if context.runtime != self.runtime_id {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "runtime_execution_session",
                    reason: format!(
                        "context runtime {} does not match runtime {}",
                        context.runtime, self.runtime_id,
                    ),
                },
                None,
            ));
        }
        if context.transaction != Some(self.transaction.store.id) {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "runtime_execution_session",
                    reason: "execution context is not attached to the session transaction"
                        .to_string(),
                },
                None,
            ));
        }
        if self.transaction.state != ActiveRuntimeTransactionState::Active {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "runtime_execution_session",
                    reason: format!("transaction {} is not active", self.transaction.store.id,),
                },
                None,
            ));
        }
        Ok(())
    }

    fn trim_context_events(max_events: Option<usize>, context: &mut RuntimeContext) {
        let Some(max_events) = max_events else {
            return;
        };
        context.events.retain_last(max_events);
    }

    fn emit_event(&mut self, kind: RuntimeEventKind) -> MResult<EventId> {
        self.validate_context()?;
        let event = RuntimeEvent::new(
            self.id_generator.event_id(),
            {
                let sequence = *self.event_sequence;
                *self.event_sequence = (*self.event_sequence).saturating_add(1);
                sequence
            },
            kind,
        );
        let id = event.id;
        self.transaction.store.stage_event(event.clone())?;
        self.context.push_event(event);
        Self::trim_context_events(self.max_events, self.context);
        Ok(id)
    }

    fn check_capability(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.validate_context()?;
        self.context.charge_step()?;
        check_transactional_capability(
            self.capability_kernel,
            &mut self.transaction.capabilities,
            &self.context.authority,
            request,
        )
    }

    fn check_resource_capability(
        &mut self,
        operation: &RuntimeCapabilityOperation,
        key: &RuntimeResourceKey,
    ) -> MResult<CapabilityId> {
        let request = CapabilityRequest::from_keys(
            &self.context.subject,
            operation.name(),
            key.capability_resource(),
        );
        self.check_capability(&request)
    }

    fn read_external_resource(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        self.validate_context()?;
        if request.intent != ResourceIntent::Read {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "execution_read_resource",
                    reason: format!("resource read received {:?} intent", request.intent),
                },
                None,
            ));
        }
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        let operation = RuntimeCapabilityOperation::from_name(request.operation.clone())?;
        self.check_resource_capability(&operation, &key)?;

        let resource_identity = self.resources.staged_resource_identity_for(&key.base_uri)?;
        if let Some(value) = self
            .transaction
            .effects
            .staged_resource_value(&resource_identity, &key.path)
        {
            return value.try_deep_snapshot();
        }

        let runtime_request = RuntimeResourceReadRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
        };
        let value = self.resources.read(runtime_request)?;
        value.try_deep_snapshot()
    }

    fn write_external_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        value: &LegacyValue,
    ) -> MResult<()> {
        self.validate_context()?;
        let intent = match request.intent {
            ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
            ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
            ResourceIntent::Read => {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "execution_write_resource",
                        reason: "resource write received Read intent".to_string(),
                    },
                    None,
                ));
            }
        };
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        let operation = RuntimeCapabilityOperation::from_name(request.operation.clone())?;
        self.check_resource_capability(&operation, &key)?;

        let value = value.try_deep_snapshot()?;
        let runtime_request = RuntimeResourceWriteRequest {
            base_uri: key.base_uri,
            path: key.path,
            context_name: request.context_name.clone(),
            operation,
            value: value.clone(),
            intent,
        };
        let staged_resource = if intent == RuntimeResourceWriteIntent::Assign {
            Some((
                self.resources
                    .staged_resource_identity_for(&runtime_request.base_uri)?,
                runtime_request.path.clone(),
                value,
            ))
        } else {
            None
        };
        let effect = self.resources.prepare_write(runtime_request)?;
        match staged_resource {
            Some((base_uri, path, value)) => {
                self.stage_resource_effect(effect, base_uri, path, value)?;
            }
            None => {
                self.stage_effect(effect)?;
            }
        }
        Ok(())
    }

    fn stage_effect(&mut self, effect: PreparedRuntimeEffect) -> MResult<RuntimeEffectId> {
        self.validate_context()?;
        let (metadata, protocol) = catch_extension("prepared runtime effect", "metadata", || {
            (effect.metadata(), effect.protocol())
        })
        .map_err(|panic| panic.into_error())?;
        let cost = metadata.cost;
        self.context.charge_bytes(cost.bytes)?;
        self.context.charge_items(cost.items)?;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
            self.transaction.store.gate_a_staged_item_count(),
        );
        let store_before = self.transaction.store.clone();
        let effect_mark = self.transaction.effects.mark();
        let transaction_id = self.transaction.store.id;
        let effect_id = self.transaction.effects.stage(transaction_id, effect);
        if let Err(error) = self.emit_event(RuntimeEventKind::EffectStaged {
            effect_id,
            source: metadata.source,
            operation: metadata.operation,
            resource: metadata.resource,
            protocol,
        }) {
            self.transaction.store = store_before;
            let cleanup = self.transaction.effects.rollback_to(effect_mark);
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

    fn stage_resource_effect(
        &mut self,
        effect: PreparedRuntimeEffect,
        resource_identity: String,
        path: String,
        value: LegacyValue,
    ) -> MResult<RuntimeEffectId> {
        self.validate_context()?;
        let (metadata, protocol) = catch_extension("prepared runtime effect", "metadata", || {
            (effect.metadata(), effect.protocol())
        })
        .map_err(|panic| panic.into_error())?;
        let cost = metadata.cost;
        self.context.charge_bytes(cost.bytes)?;
        self.context.charge_items(cost.items)?;
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
            self.transaction.store.gate_a_staged_item_count(),
        );
        let store_before = self.transaction.store.clone();
        let effect_mark = self.transaction.effects.mark();
        let transaction_id = self.transaction.store.id;
        let effect_id = self.transaction.effects.stage_resource_write(
            transaction_id,
            effect,
            resource_identity,
            path,
            value,
        );
        if let Err(error) = self.emit_event(RuntimeEventKind::EffectStaged {
            effect_id,
            source: metadata.source,
            operation: metadata.operation,
            resource: metadata.resource,
            protocol,
        }) {
            self.transaction.store = store_before;
            let cleanup = self.transaction.effects.rollback_to(effect_mark);
            if cleanup.is_empty() {
                return Err(error);
            }
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "runtime_execution_session",
                    reason: format!(
                        "resource effect staging failed and cleanup was incomplete: original={error:?}; cleanup={cleanup:?}",
                    ),
                },
                None,
            ));
        }
        Ok(effect_id)
    }
}

impl RuntimeManagedServices for RuntimeSessionServices<'_> {
    fn allocate_object_id(&mut self) -> MResult<ObjectId> {
        self.validate_context()?;
        Ok(self.id_generator.object_id())
    }

    fn get_object(&mut self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
        self.validate_context()?;
        self.context.record_read(id);
        if let Some(object) = self.transaction.store.get_staged_object(id) {
            return Ok(Some(object));
        }
        self.store.get_object(id)
    }

    fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        self.validate_context()?;
        self.context.charge_bytes(object.data.len() as u64)?;
        let id = object.id;
        self.transaction.store.stage_put_object(object)?;
        self.context.record_write(id);
        self.emit_event(RuntimeEventKind::ObjectCreated { object_id: id })?;
        Ok(id)
    }

    fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        self.validate_context()?;
        self.context.charge_bytes(object.data.len() as u64)?;
        let id = object.id;
        self.transaction.store.stage_update_object(object)?;
        self.context.record_write(id);
        self.emit_event(RuntimeEventKind::ObjectUpdated { object_id: id })?;
        Ok(id)
    }

    fn get_actor(&mut self, id: ActorId) -> MResult<Option<ActorRecord>> {
        self.validate_context()?;
        if let Some(actor) = self.transaction.store.get_staged_actor(id) {
            return Ok(Some(actor));
        }
        self.store.get_actor(id)
    }

    fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        self.validate_context()?;
        let id = actor.id;
        self.transaction.store.stage_actor_update(actor)?;
        Ok(id)
    }

    fn set_current_actor_state(&mut self, state: ObjectId) -> MResult<()> {
        self.validate_context()?;
        self.context.actor_state = Some(state);
        self.transaction.context_identity.set_actor_state(state);
        Ok(())
    }
}

impl MechExecutionServices for RuntimeExecutionSession<'_> {
    fn invoke_host_function(
        &mut self,
        request: &ExecutionHostFunctionRequest,
        arguments: &[LegacyValue],
    ) -> MResult<LegacyValue> {
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
            ..
        } = self;
        let name = request.name.as_str();
        let mut services = RuntimeSessionServices {
            runtime_id: *runtime_id,
            max_events: *max_events,
            transaction,
            id_generator: &mut **id_generator,
            store: &mut **store,
            capability_kernel: &mut **capability_kernel,
            resources,
            event_sequence,
            context,
        };
        services.validate_context()?;
        if name.trim().is_empty() {
            return Err(MechError::new(
                InvalidHostFunctionError {
                    field: "name",
                    reason: "must not be empty",
                },
                None,
            ));
        }
        services.emit_event(RuntimeEventKind::HostCallStarted {
            name: name.to_string(),
        })?;
        let Some(function) = invoke_extension("host registry", "get_function", || {
            host_registry.get_function(name)
        })?
        else {
            services.emit_event(RuntimeEventKind::HostCallFailed {
                name: name.to_string(),
                message: "host function not found".to_string(),
            })?;
            return Err(MechError::new(
                HostFunctionNotFoundError {
                    name: name.to_string(),
                },
                None,
            ));
        };

        let call_context = RuntimeCallContext::capture(services.context);
        let result = (|| -> MResult<LegacyValue> {
            let arguments = arguments
                .iter()
                .map(RuntimeValueSnapshot::try_capture)
                .collect::<MResult<Vec<_>>>()?;
            invoke_extension("host call policy", "validate_call", || {
                host_policy.validate_call(&call_context, &function, &arguments)
            })?;
            let component = format!("host function `{name}`");
            let (estimated_items, estimated_bytes) =
                catch_extension(component.clone(), "plan cost", || {
                    (
                        function.estimated_cost_items(&arguments),
                        function.estimated_cost_bytes(&arguments),
                    )
                })
                .map_err(|panic| panic.into_error())?;
            services.context.charge_items(estimated_items)?;
            services.context.charge_bytes(estimated_bytes)?;
            let capability_request =
                catch_extension(component.clone(), "required_capability", || {
                    function.required_capability(&call_context)
                })
                .map_err(|panic| panic.into_error())?
                .unwrap_or_else(|| default_host_capability_request(&call_context, name));
            services.check_capability(&capability_request)?;
            match function {
                RegisteredHostFunction::Pure(function) => {
                    let snapshot = invoke_extension(component, "invoke", || {
                        function.invoke(&call_context, arguments)
                    })?;
                    snapshot.into_value().try_deep_snapshot()
                }
                RegisteredHostFunction::RuntimeManaged(function) => {
                    let snapshot = invoke_extension(component, "invoke", || {
                        function.invoke(&mut services, &call_context, arguments)
                    })?;
                    snapshot.into_value().try_deep_snapshot()
                }
                RegisteredHostFunction::Staged(function) => {
                    let RuntimePreparedHostCall { value, effect } =
                        invoke_extension(component, "prepare", || {
                            function.prepare(&call_context, arguments)
                        })?;
                    let value = value.into_value().try_deep_snapshot()?;
                    services.stage_effect(effect)?;
                    Ok(value)
                }
            }
        })();

        match &result {
            Ok(_) => {
                services.emit_event(RuntimeEventKind::HostCallCompleted {
                    name: name.to_string(),
                })?;
            }
            Err(error) => {
                services.emit_event(RuntimeEventKind::HostCallFailed {
                    name: name.to_string(),
                    message: format!("{error:?}"),
                })?;
            }
        }
        result
    }

    fn read_resource(&mut self, request: &ExecutionResourceRequest) -> MResult<LegacyValue> {
        let Self {
            runtime_id,
            max_events,
            context,
            transaction,
            id_generator,
            store,
            capability_kernel,
            resources,
            event_sequence,
            ..
        } = self;
        RuntimeSessionServices {
            runtime_id: *runtime_id,
            max_events: *max_events,
            transaction,
            id_generator: &mut **id_generator,
            store: &mut **store,
            capability_kernel: &mut **capability_kernel,
            resources,
            event_sequence,
            context,
        }
        .read_external_resource(request)
    }

    fn plan_resource_read_output(
        &mut self,
        request: &ExecutionResourceRequest,
    ) -> MResult<LegacyValue> {
        let Self {
            runtime_id,
            max_events,
            context,
            transaction,
            id_generator,
            store,
            capability_kernel,
            resources,
            event_sequence,
            ..
        } = self;
        RuntimeSessionServices {
            runtime_id: *runtime_id,
            max_events: *max_events,
            transaction,
            id_generator: &mut **id_generator,
            store: &mut **store,
            capability_kernel: &mut **capability_kernel,
            resources,
            event_sequence,
            context,
        }
        .read_external_resource(request)
    }

    fn write_resource(
        &mut self,
        request: &ExecutionResourceRequest,
        value: &LegacyValue,
    ) -> MResult<()> {
        let Self {
            runtime_id,
            max_events,
            context,
            transaction,
            id_generator,
            store,
            capability_kernel,
            resources,
            event_sequence,
            ..
        } = self;
        RuntimeSessionServices {
            runtime_id: *runtime_id,
            max_events: *max_events,
            transaction,
            id_generator: &mut **id_generator,
            store: &mut **store,
            capability_kernel: &mut **capability_kernel,
            resources,
            event_sequence,
            context,
        }
        .write_external_resource(request, value)
    }

    fn bind_live_resource(
        &mut self,
        _interpreter_id: u64,
        _request: &ExecutionResourceRequest,
        _target: ValueCell,
    ) -> MResult<()> {
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "bind_live_resource",
                reason: "live resource targets belong to the resident compiler session".to_string(),
            },
            None,
        ))
    }
}

impl MechRuntime {
    fn execution_session_max_events(&self) -> Option<usize> {
        self.config
            .limits
            .max_in_memory_events
            .map(|max| usize::try_from(max).unwrap_or(usize::MAX))
    }

    pub(super) fn with_runtime_execution_session<T>(
        &mut self,
        context: &mut RuntimeContext,
        execute: impl FnOnce(&mut RuntimeExecutionSession<'_>) -> MResult<T>,
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
                MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None)
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
        execute(&mut session)
    }
}
