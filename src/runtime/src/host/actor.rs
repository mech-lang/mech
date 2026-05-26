use mech_core::{Ref, MResult, MechError, Value};

use crate::capability::CapabilityRequest;

use crate::service::RuntimeServices;

use crate::store::ObjectRecord;

use crate::host::*;

// -----------------------------------------------------------------------------
// Actor Context Host Functions
// -----------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct ActorMessageKindHostFunction;

impl ActorMessageKindHostFunction {
  pub fn new() -> Self {
    Self
  }
}

impl Default for ActorMessageKindHostFunction {
  fn default() -> Self {
    Self::new()
  }
}

impl HostFunction for ActorMessageKindHostFunction {
  fn name(&self) -> &str {
    "actor/message/kind"
  }

  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    _args: Vec<Value>,
  ) -> MResult<Value> {
    let Some(kind) = context.actor_message_kind() else {
      return Err(MechError::new(
        HostInvalidContextError {
          function: self.name().to_string(),
          reason: "no actor message is bound to the runtime context".to_string(),
        },
        None,
      ));
    };

    Ok(Value::String(Ref::new(kind.to_string())))
  }

  fn estimated_cost_items(&self, _args: &[Value]) -> u64 {
    1
  }

  fn estimated_cost_bytes(&self, _args: &[Value]) -> u64 {
    0
  }

  fn required_capability(
    &self,
    context: &RuntimeContext,
  ) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, self.name()))
  }
}

#[derive(Clone, Debug)]
pub struct ActorMessagePayloadHostFunction;

impl ActorMessagePayloadHostFunction {
  pub fn new() -> Self {
    Self
  }
}

impl Default for ActorMessagePayloadHostFunction {
  fn default() -> Self {
    Self::new()
  }
}

impl HostFunction for ActorMessagePayloadHostFunction {
  fn name(&self) -> &str {
    "actor/message/payload"
  }

  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    _args: Vec<Value>,
  ) -> MResult<Value> {
    let Some(payload) = context.actor_message_payload() else {
      return Err(MechError::new(
        HostInvalidContextError {
          function: self.name().to_string(),
          reason: "no actor message is bound to the runtime context".to_string(),
        },
        None,
      ));
    };

    Ok(Value::String(Ref::new(String::from_utf8_lossy(payload).to_string())))
  }

  fn estimated_cost_items(&self, _args: &[Value]) -> u64 {
    1
  }

  fn estimated_cost_bytes(&self, _args: &[Value]) -> u64 {
    context_payload_cost_unavailable()
  }

  fn required_capability(
    &self,
    context: &RuntimeContext,
  ) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, self.name()))
  }
}

#[derive(Clone, Debug)]
pub struct ActorStateIdHostFunction;

impl ActorStateIdHostFunction {
  pub fn new() -> Self {
    Self
  }
}

impl Default for ActorStateIdHostFunction {
  fn default() -> Self {
    Self::new()
  }
}

impl HostFunction for ActorStateIdHostFunction {
  fn name(&self) -> &str {
    "actor/state/id"
  }

  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    _args: Vec<Value>,
  ) -> MResult<Value> {
    let Some(state) = context.actor_state() else {
      return Ok(Value::Empty);
    };

    Ok(Value::String(Ref::new(state.to_string())))
  }

  fn estimated_cost_items(&self, _args: &[Value]) -> u64 {
    1
  }

  fn estimated_cost_bytes(&self, _args: &[Value]) -> u64 {
    0
  }

  fn required_capability(
    &self,
    context: &RuntimeContext,
  ) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, self.name()))
  }
}

#[derive(Clone, Debug)]
pub struct ActorStateGetHostFunction;

impl ActorStateGetHostFunction {
  pub fn new() -> Self {
    Self
  }
}

impl Default for ActorStateGetHostFunction {
  fn default() -> Self {
    Self::new()
  }
}

impl HostFunction for ActorStateGetHostFunction {
  fn name(&self) -> &str {
    "actor/state/get"
  }

  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    _args: Vec<Value>,
  ) -> MResult<Value> {
    let Some(state) = context.actor_state() else {
      return Ok(Value::Empty);
    };

    let Some(object) = services.get_object_with_context(context, state)? else {
      return Ok(Value::Empty);
    };

    Ok(Value::String(Ref::new(String::from_utf8_lossy(&object.data).to_string())))
  }

  fn estimated_cost_items(&self, _args: &[Value]) -> u64 {
    1
  }

  fn estimated_cost_bytes(&self, _args: &[Value]) -> u64 {
    0
  }

  fn required_capability(
    &self,
    context: &RuntimeContext,
  ) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, self.name()))
  }
}

#[derive(Clone, Debug)]
pub struct ActorStatePutHostFunction;

impl ActorStatePutHostFunction {
  pub fn new() -> Self {
    Self
  }
}

impl Default for ActorStatePutHostFunction {
  fn default() -> Self {
    Self::new()
  }
}

impl HostFunction for ActorStatePutHostFunction {
  fn name(&self) -> &str {
    "actor/state/put"
  }

  fn call(
    &self,
    services: &mut dyn RuntimeServices,
    context: &mut RuntimeContext,
    args: Vec<Value>,
  ) -> MResult<Value> {
    let Some(actor_id) = context.actor else {
      return Err(MechError::new(
        HostInvalidContextError {
          function: self.name().to_string(),
          reason: "no actor is bound to the runtime context".to_string(),
        },
        None,
      ));
    };

    let text = host_arg_string(self.name(), &args, 0)?;

    let object_id = services.next_object_id();

    let object = ObjectRecord::text(
      object_id,
      "actor-state",
      text,
    );

    services.put_object_with_context(context, object)?;

    let Some(mut actor) = services.get_actor_with_context(context, actor_id)? else {
      return Err(MechError::new(
        HostInvalidContextError {
          function: self.name().to_string(),
          reason: "actor record was not found".to_string(),
        },
        None,
      ));
    };

    actor.state = Some(object_id);

    services.update_actor_with_context(context, actor)?;

    context.actor_state = Some(object_id);

    Ok(Value::String(Ref::new(object_id.to_string())))
  }

  fn estimated_cost_items(&self, _args: &[Value]) -> u64 {
    1
  }

  fn estimated_cost_bytes(&self, args: &[Value]) -> u64 {
    args.len() as u64
  }

  fn required_capability(
    &self,
    context: &RuntimeContext,
  ) -> Option<CapabilityRequest> {
    Some(default_host_capability_request(context, self.name()))
  }
}