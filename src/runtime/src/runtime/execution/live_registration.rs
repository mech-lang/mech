use crate::runtime::MechRuntime;
use crate::runtime::live_state::LiveRegistrationMode;
use crate::runtime::state::RuntimeExecutionMode;
use crate::{RuntimeHostInputSource, RuntimeLiveResourceBinding};
use mech_core::{ExecutionResourceRequest, MResult, ResourceDelivery, ResourceIntent, ValRef};
use std::collections::BTreeMap;

impl MechRuntime {
    pub(in crate::runtime) fn bind_live_resource_target_in(
        execution_mode: RuntimeExecutionMode,
        registration_mode: LiveRegistrationMode,
        bindings: &mut BTreeMap<RuntimeHostInputSource, Vec<RuntimeLiveResourceBinding>>,
        interpreter_id: u64,
        request: &ExecutionResourceRequest,
        target: ValRef,
    ) -> MResult<()> {
        if execution_mode != RuntimeExecutionMode::Execute
            || registration_mode != LiveRegistrationMode::RetainedRoot
            || request.intent != ResourceIntent::Read
            || request.delivery != ResourceDelivery::Live
        {
            return Ok(());
        }

        let source = RuntimeHostInputSource::new(&request.base_uri, &request.path)?;
        let binding = RuntimeLiveResourceBinding {
            interpreter_id,
            source: source.clone(),
            target,
        };
        let source_bindings = bindings.entry(source).or_default();
        if !source_bindings
            .iter()
            .any(|candidate| candidate.same_identity(&binding))
        {
            source_bindings.push(binding);
        }
        Ok(())
    }

    pub(super) fn with_live_registration_mode<T>(
        &mut self,
        mode: LiveRegistrationMode,
        f: impl FnOnce(&mut Self) -> MResult<T>,
    ) -> MResult<T> {
        let previous = std::mem::replace(&mut self.live_registration_mode, mode);
        let result = f(self);
        self.live_registration_mode = previous;
        result
    }

    pub fn live_input_binding_count(&self) -> usize {
        self.live_input_bindings.values().map(Vec::len).sum()
    }

    pub fn has_live_input_bindings(&self) -> bool {
        self.live_input_binding_count() > 0
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BTreeMap, ExecutionResourceRequest, LiveRegistrationMode, MechRuntime, ResourceDelivery,
        ResourceIntent, RuntimeExecutionMode, RuntimeHostInputSource,
    };
    use mech_core::{LegacyValue, Ref};

    fn live_read_request() -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://clock/ticks".to_string(),
            path: "value".to_string(),
            context_name: "pulse".to_string(),
            operation: "read".to_string(),
            intent: ResourceIntent::Read,
            delivery: ResourceDelivery::Live,
        }
    }

    #[test]
    fn repeated_live_resource_binding_is_idempotent() {
        let request = live_read_request();
        let target = Ref::new(LegacyValue::F64(Ref::new(1.0)));
        let target_address = target.as_ptr();
        let mut bindings = BTreeMap::new();

        for _ in 0..2 {
            MechRuntime::bind_live_resource_target_in(
                RuntimeExecutionMode::Execute,
                LiveRegistrationMode::RetainedRoot,
                &mut bindings,
                7,
                &request,
                target.clone(),
            )
            .unwrap();
        }

        let source = RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap();
        let source_bindings = bindings.get(&source).unwrap();
        assert_eq!(source_bindings.len(), 1);
        assert_eq!(source_bindings[0].interpreter_id, 7);
        assert_eq!(source_bindings[0].target.as_ptr(), target_address);
    }

    #[test]
    fn planning_live_resource_binding_is_not_retained() {
        let request = live_read_request();
        let mut bindings = BTreeMap::new();

        MechRuntime::bind_live_resource_target_in(
            RuntimeExecutionMode::Plan,
            LiveRegistrationMode::RetainedRoot,
            &mut bindings,
            7,
            &request,
            Ref::new(LegacyValue::F64(Ref::new(1.0))),
        )
        .unwrap();

        assert!(bindings.is_empty());
    }
}
