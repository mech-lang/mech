use mech_core::{LegacyValue, MResult, MechError, Ref, hash_str};

use crate::capability::CapabilityRequest;
use crate::service::RuntimeManagedServices;
use crate::store::ObjectRecord;
use crate::{RuntimeCallContext, RuntimeValueSnapshot};

use crate::host::*;

/// Deterministic, effect-free actor state used while specializing source and
/// validating bytecode host calls for native applications.
#[derive(Clone, Debug)]
pub struct ActorHostPlanningState {
    message_kind: String,
    message_payload: String,
    actor_bound: bool,
    state: Option<String>,
    state_id: Option<String>,
    state_id_seed: u64,
}

impl ActorHostPlanningState {
    pub fn new(
        subject: &str,
        message_kind: impl Into<String>,
        message_payload: impl Into<String>,
        initial_state: Option<String>,
    ) -> Self {
        let state_id_seed = hash_str(subject);
        let state_id = initial_state
            .as_ref()
            .map(|_| format!("planning-state-{state_id_seed:016x}"));
        Self {
            message_kind: message_kind.into(),
            message_payload: message_payload.into(),
            actor_bound: true,
            state: initial_state,
            state_id,
            state_id_seed,
        }
    }

    /// Plans one trusted actor host call and advances the synthetic state for
    /// subsequent calls in the same source/bytecode instruction sequence.
    pub fn plan(&mut self, name: &str, arguments: &[LegacyValue]) -> MResult<LegacyValue> {
        match name {
            "actor/message/kind" => {
                expect_arity(name, arguments, 0)?;
                Ok(LegacyValue::String(Ref::new(self.message_kind.clone())))
            }
            "actor/message/payload" => {
                expect_arity(name, arguments, 0)?;
                Ok(LegacyValue::String(Ref::new(self.message_payload.clone())))
            }
            "actor/state/get" => {
                expect_arity(name, arguments, 0)?;
                Ok(match &self.state {
                    Some(state) => LegacyValue::String(Ref::new(state.clone())),
                    None => LegacyValue::Empty,
                })
            }
            "actor/state/id" => {
                expect_arity(name, arguments, 0)?;
                Ok(match &self.state_id {
                    Some(state_id) => LegacyValue::String(Ref::new(state_id.clone())),
                    None => LegacyValue::Empty,
                })
            }
            "actor/state/put" => {
                if !self.actor_bound {
                    return Err(invalid_context(
                        name,
                        "no actor is bound to the runtime context",
                    ));
                }
                expect_arity(name, arguments, 1)?;
                let state = host_arg_string(name, arguments, 0)?;
                let state_id = format!("planning-state-put-{:016x}", self.state_id_seed);
                self.state = Some(state);
                self.state_id = Some(state_id.clone());
                Ok(LegacyValue::String(Ref::new(state_id)))
            }
            _ => Err(invalid_context(
                name,
                "host function is not part of the trusted actor planning surface",
            )),
        }
    }
}

fn snapshot(value: LegacyValue) -> MResult<RuntimeValueSnapshot> {
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
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        expect_arity(self.name(), arguments, 0)?;
        let message = context.actor_message().ok_or_else(|| {
            invalid_context(
                self.name(),
                "no actor message is bound to the runtime context",
            )
        })?;
        snapshot(LegacyValue::String(Ref::new(message.kind.clone())))
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
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        expect_arity(self.name(), arguments, 0)?;
        let message = context.actor_message().ok_or_else(|| {
            invalid_context(
                self.name(),
                "no actor message is bound to the runtime context",
            )
        })?;
        snapshot(LegacyValue::String(Ref::new(
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
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        expect_arity(self.name(), arguments, 0)?;
        match context.actor_state() {
            Some(state) => snapshot(LegacyValue::String(Ref::new(state.to_string()))),
            None => snapshot(LegacyValue::Empty),
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
        context: &RuntimeCallContext,
        arguments: &[RuntimeValueSnapshot],
    ) -> MResult<RuntimeValueSnapshot> {
        expect_arity(self.name(), arguments, 0)?;
        match context.actor_state() {
            Some(_) => snapshot(LegacyValue::String(Ref::new(String::new()))),
            None => snapshot(LegacyValue::Empty),
        }
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
        arguments: Vec<RuntimeValueSnapshot>,
    ) -> MResult<RuntimeValueSnapshot> {
        expect_arity(self.name(), &arguments, 0)?;
        let Some(state) = context.actor_state() else {
            return snapshot(LegacyValue::Empty);
        };
        let value = services
            .get_object(state)?
            .map(|object| String::from_utf8_lossy(&object.data).to_string())
            .unwrap_or_default();
        snapshot(LegacyValue::String(Ref::new(value)))
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
        expect_arity(self.name(), &values, 1)?;
        host_arg_string(self.name(), &values, 0)?;
        snapshot(LegacyValue::String(Ref::new(String::new())))
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
        expect_arity(self.name(), &values, 1)?;
        let text = host_arg_string(self.name(), &values, 0)?;
        let object_id = services.allocate_object_id()?;
        services.put_object(ObjectRecord::text(object_id, "actor-state", text))?;
        let mut actor = services
            .get_actor(actor_id)?
            .ok_or_else(|| invalid_context(self.name(), "actor record was not found"))?;
        actor.state = Some(object_id);
        services.update_actor(actor)?;
        services.set_current_actor_state(object_id)?;
        snapshot(LegacyValue::String(Ref::new(object_id.to_string())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn planning_state_tracks_put_before_get_and_id() {
        let mut state = ActorHostPlanningState::new("actor:test", "message", "payload", None);

        assert_eq!(
            state.plan("actor/state/get", &[]).unwrap(),
            LegacyValue::Empty
        );
        assert_eq!(
            state.plan("actor/state/id", &[]).unwrap(),
            LegacyValue::Empty
        );
        assert!(matches!(
            state
                .plan(
                    "actor/state/put",
                    &[LegacyValue::String(Ref::new("created".to_owned()))],
                )
                .unwrap(),
            LegacyValue::String(_)
        ));
        assert_eq!(
            state.plan("actor/state/get", &[]).unwrap(),
            LegacyValue::String(Ref::new("created".to_owned())),
        );
        assert!(matches!(
            state.plan("actor/state/id", &[]).unwrap(),
            LegacyValue::String(_)
        ));
    }

    #[test]
    fn planning_state_enforces_exact_actor_arities_and_put_type() {
        let mut state = ActorHostPlanningState::new("actor:test", "message", "payload", None);

        assert!(
            state
                .plan("actor/message/kind", &[LegacyValue::Empty])
                .is_err()
        );
        assert!(state.plan("actor/state/put", &[]).is_err());
        assert!(
            state
                .plan("actor/state/put", &[LegacyValue::Empty])
                .is_err()
        );
        assert!(
            state
                .plan(
                    "actor/state/put",
                    &[
                        LegacyValue::String(Ref::new("one".to_owned())),
                        LegacyValue::String(Ref::new("two".to_owned())),
                    ],
                )
                .is_err()
        );
    }
}
