use mech_core::{MResult, MechError, Ref, Value};

use crate::capability::CapabilityRequest;
use crate::service::RuntimeManagedServices;
use crate::store::ObjectRecord;
use crate::{RuntimeCallContext, RuntimeValueSnapshot};

use crate::host::*;

fn snapshot(value: Value) -> MResult<RuntimeValueSnapshot> {
    RuntimeValueSnapshot::try_capture(&value)
}

fn invalid_context(function: &str, reason: impl Into<String>) -> MechError {
    MechError::new(
        HostInvalidContextError {
            function: function.to_string(),
            reason: reason.into(),
        },
        None,
    )
}

fn actor_capability(context: &RuntimeCallContext, function: &str) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, function))
}

#[derive(Clone, Debug, Default)]
pub struct ActorMessageKindHostFunction;

impl ActorMessageKindHostFunction {
    pub fn new() -> Self {
        Self
    }
}

impl HostFunctionPlan for ActorMessageKindHostFunction {
    fn name(&self) -> &str {
        "actor/message/kind"
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        _arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        let message = context.actor_message().ok_or_else(|| {
            invalid_context(
                self.name(),
                "no actor message is bound to the runtime context",
            )
        })?;
        snapshot(Value::String(Ref::new(message.kind.clone())))
    }

    fn estimated_cost_items(&self, _arguments: &[RuntimeValueSnapshot]) -> u64 {
        1
    }

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        actor_capability(context, self.name())
    }
}

impl PureHostFunction for ActorMessageKindHostFunction {
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.plan(context, &arguments)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActorMessagePayloadHostFunction;

impl ActorMessagePayloadHostFunction {
    pub fn new() -> Self {
        Self
    }
}

impl HostFunctionPlan for ActorMessagePayloadHostFunction {
    fn name(&self) -> &str {
        "actor/message/payload"
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        _arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        let message = context.actor_message().ok_or_else(|| {
            invalid_context(
                self.name(),
                "no actor message is bound to the runtime context",
            )
        })?;
        snapshot(Value::String(Ref::new(
            String::from_utf8_lossy(&message.payload).to_string(),
        )))
    }

    fn estimated_cost_items(&self, _arguments: &[RuntimeValueSnapshot]) -> u64 {
        1
    }

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        actor_capability(context, self.name())
    }
}

impl PureHostFunction for ActorMessagePayloadHostFunction {
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.plan(context, &arguments)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActorStateIdHostFunction;

impl ActorStateIdHostFunction {
    pub fn new() -> Self {
        Self
    }
}

impl HostFunctionPlan for ActorStateIdHostFunction {
    fn name(&self) -> &str {
        "actor/state/id"
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        _arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        match context.actor_state() {
            Some(state) => snapshot(Value::String(Ref::new(state.to_string()))),
            None => snapshot(Value::Empty),
        }
    }

    fn estimated_cost_items(&self, _arguments: &[RuntimeValueSnapshot]) -> u64 {
        1
    }

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        actor_capability(context, self.name())
    }
}

impl PureHostFunction for ActorStateIdHostFunction {
    fn invoke(
        &self,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        self.plan(context, &arguments)
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActorStateGetHostFunction;

impl ActorStateGetHostFunction {
    pub fn new() -> Self {
        Self
    }
}

impl HostFunctionPlan for ActorStateGetHostFunction {
    fn name(&self) -> &str {
        "actor/state/get"
    }

    fn plan(
        &self,
        _context: &RuntimeCallContext,
        _arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        snapshot(Value::Empty)
    }

    fn estimated_cost_items(&self, _arguments: &[RuntimeValueSnapshot]) -> u64 {
        1
    }

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        actor_capability(context, self.name())
    }
}

impl RuntimeManagedHostFunction for ActorStateGetHostFunction {
    fn invoke(
        &self,
        services: &mut dyn RuntimeManagedServices,
        context: &RuntimeCallContext,
        _arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        let Some(state) = context.actor_state() else {
            return snapshot(Value::Empty);
        };
        let Some(object) = services.get_object(state)? else {
            return snapshot(Value::Empty);
        };
        snapshot(Value::String(Ref::new(
            String::from_utf8_lossy(&object.data).to_string(),
        )))
    }
}

#[derive(Clone, Debug, Default)]
pub struct ActorStatePutHostFunction;

impl ActorStatePutHostFunction {
    pub fn new() -> Self {
        Self
    }
}

impl HostFunctionPlan for ActorStatePutHostFunction {
    fn name(&self) -> &str {
        "actor/state/put"
    }

    fn plan(
        &self,
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        if context.actor().is_none() {
            return Err(invalid_context(
                self.name(),
                "no actor is bound to the runtime context",
            ));
        }
        let values = arguments
            .iter()
            .map(RuntimeValueSnapshot::to_value)
            .collect::<Vec<_>>();
        host_arg_string(self.name(), &values, 0)?;
        snapshot(Value::String(Ref::new(String::new())))
    }

    fn estimated_cost_items(&self, _arguments: &[RuntimeValueSnapshot]) -> u64 {
        1
    }

    fn estimated_cost_bytes(&self, arguments: &[RuntimeValueSnapshot]) -> u64 {
        arguments.len() as u64
    }

    fn required_capability(&self, context: &RuntimeCallContext) -> Option<CapabilityRequest> {
        actor_capability(context, self.name())
    }
}

impl RuntimeManagedHostFunction for ActorStatePutHostFunction {
    fn invoke(
        &self,
        services: &mut dyn RuntimeManagedServices,
        context: &RuntimeCallContext,
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        let actor_id = context.actor().ok_or_else(|| {
            invalid_context(self.name(), "no actor is bound to the runtime context")
        })?;
        let values = arguments
            .iter()
            .map(RuntimeValueSnapshot::to_value)
            .collect::<Vec<_>>();
        let text = host_arg_string(self.name(), &values, 0)?;
        let object_id = services.allocate_object_id()?;
        services.put_object(ObjectRecord::text(object_id, "actor-state", text))?;
        let mut actor = services
            .get_actor(actor_id)?
            .ok_or_else(|| invalid_context(self.name(), "actor record was not found"))?;
        actor.state = Some(object_id);
        services.update_actor(actor)?;
        services.set_current_actor_state(object_id)?;
        snapshot(Value::String(Ref::new(object_id.to_string())))
    }
}
