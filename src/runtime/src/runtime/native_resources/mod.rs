//! Snapshot text ports for native programs using the runtime's resource APIs.
//!
//! Binding checks every requirement without observing inputs or preparing an
//! effect. Calls retain ordinary resource authorization; context-aware calls
//! join the host's transaction. This is not canonical resident artifact
//! activation or a resident scheduler turn.

use mech_core::{
    ExecutionResourceRequest, MResult, MechError, MechErrorKind, ResourceDelivery, ResourceIntent,
    Value, ValueData,
};

use super::MechRuntime;
use crate::{
    CapabilityRequest, RuntimeCapabilityOperation, RuntimeContext, RuntimeId, RuntimeResourceKey,
    RuntimeResourceReadRequest, RuntimeResourceWriteCommand, RuntimeResourceWriteIntent,
};

/// Ports admitted for one runtime. Requests cannot be constructed or replaced
/// through a binding; callers select only an existing requirement index.
#[derive(Clone, Debug)]
pub struct NativeResourceBindings {
    runtime: RuntimeId,
    requests: Box<[ExecutionResourceRequest]>,
}

impl NativeResourceBindings {
    pub fn len(&self) -> usize {
        self.requests.len()
    }
    pub fn is_empty(&self) -> bool {
        self.requests.is_empty()
    }
}

#[derive(Clone, Debug)]
pub struct NativeResourceBindingError {
    pub reason: String,
}

impl MechErrorKind for NativeResourceBindingError {
    fn name(&self) -> &str {
        "NativeResourceBindingError"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}

fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        NativeResourceBindingError {
            reason: reason.into(),
        },
        None,
    ))
}

impl MechRuntime {
    /// Admit all snapshot text reads and sends before executing a native entry.
    /// Grants must already have been supplied by the host. Admission previews
    /// authority without consuming a use; every actual call checks it again.
    pub fn bind_native_resources(
        &mut self,
        requirements: &[ExecutionResourceRequest],
    ) -> MResult<NativeResourceBindings> {
        self.ensure_runtime_mutation_allowed("bind_native_resources")?;
        let context = self.runtime_context()?;
        for request in requirements {
            let expected_operation = match request.intent {
                ResourceIntent::Read => "read",
                ResourceIntent::Send => "write",
                ResourceIntent::Assign => return invalid("native ports do not support assignment"),
            };
            if request.delivery != ResourceDelivery::Snapshot {
                return invalid("native ports do not support live delivery");
            }
            if request.operation != expected_operation || request.context_name.is_empty() {
                return invalid("native resource intent, operation or context does not match");
            }
            let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
            if key.base_uri != request.base_uri || key.path != request.path {
                return invalid("native resource identity must already be canonical");
            }
            let equivalent = self.resources.equivalent_base_uris_for(&key.base_uri)?;
            let owner = equivalent
                .iter()
                .find_map(|base| self.host_interfaces.context_for_base_uri(base))
                .ok_or_else(|| {
                    MechError::new(
                        NativeResourceBindingError {
                            reason: "native resource has no materialized host context".into(),
                        },
                        None,
                    )
                })?;
            if owner.name != request.context_name
                || !owner
                    .operations
                    .iter()
                    .any(|operation| operation == &request.operation)
            {
                return invalid("native resource does not match its materialized host context");
            }
            let capability = CapabilityRequest::from_keys(
                &context.subject,
                &request.operation,
                key.capability_resource(),
            );
            self.preview_capability_for_execution(&context, &capability)?;
            self.resources.preflight_native_text_resource(request)?;
        }
        Ok(NativeResourceBindings {
            runtime: self.id(),
            requests: requirements.into(),
        })
    }

    fn native_resource<'a>(
        &self,
        bindings: &'a NativeResourceBindings,
        index: usize,
        intent: ResourceIntent,
    ) -> MResult<&'a ExecutionResourceRequest> {
        if bindings.runtime != self.id() {
            return invalid("native resource bindings belong to another runtime");
        }
        let Some(request) = bindings.requests.get(index) else {
            return invalid("native resource requirement index is not bound");
        };
        if request.intent != intent {
            return invalid("native resource port has the wrong intent");
        }
        Ok(request)
    }

    pub fn read_native_resource(
        &mut self,
        bindings: &NativeResourceBindings,
        index: usize,
    ) -> MResult<Value> {
        let mut context = self.runtime_context()?;
        self.read_native_resource_with_context(bindings, index, &mut context)
    }

    /// Read an admitted port using the supplied runtime context. An explicit
    /// transaction remains open for the host to commit or abort after execution.
    pub fn read_native_resource_with_context(
        &mut self,
        bindings: &NativeResourceBindings,
        index: usize,
        context: &mut RuntimeContext,
    ) -> MResult<Value> {
        let request = self.native_resource(bindings, index, ResourceIntent::Read)?;
        let value = self
            .read_resource_with_context(
                context,
                RuntimeResourceReadRequest {
                    base_uri: request.base_uri.clone(),
                    path: request.path.clone(),
                    context_name: request.context_name.clone(),
                },
            )?
            .into_value();
        if !matches!(value.data(), ValueData::String(_)) {
            return invalid("native text observation returned a non-string value");
        }
        Ok(value)
    }

    pub fn write_native_resource(
        &mut self,
        bindings: &NativeResourceBindings,
        index: usize,
        value: Value,
    ) -> MResult<()> {
        let mut context = self.runtime_context()?;
        self.write_native_resource_with_context(bindings, index, value, &mut context)
    }

    /// Stage a send in the supplied context's transaction. Delivery follows the
    /// existing provider effect protocol when the host commits that transaction.
    pub fn write_native_resource_with_context(
        &mut self,
        bindings: &NativeResourceBindings,
        index: usize,
        value: Value,
        context: &mut RuntimeContext,
    ) -> MResult<()> {
        let request = self.native_resource(bindings, index, ResourceIntent::Send)?;
        if !matches!(value.data(), ValueData::String(_)) {
            return invalid("native text send requires a string value");
        }
        self.write_resource_with_context(
            context,
            RuntimeResourceWriteCommand {
                base_uri: request.base_uri.clone(),
                path: request.path.clone(),
                context_name: request.context_name.clone(),
                operation: RuntimeCapabilityOperation::Write,
                intent: RuntimeResourceWriteIntent::Send,
                value,
            },
        )
        .map(|_| ())
    }
}

#[cfg(test)]
mod tests;
