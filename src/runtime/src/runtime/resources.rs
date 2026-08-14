use super::transaction::{RuntimeCommitResolution, RuntimeExecutionTransactionMode};
use super::{MechRuntime, RuntimeExecutionMode, RuntimeInvalidOperationError};
use crate::{
    CapabilityId, CapabilityRequest, ResourcePathCapability, RunResourceGrantConfig,
    RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation, RuntimeContext, RuntimeEffectId,
    RuntimeResourceKey, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    RuntimeValueSnapshot, TransactionId,
};
use mech_core::{LegacyValue, MResult, MechError, MechErrorKind};
use std::sync::Arc;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeResourceBinding {
    pub name: String,
    pub base_uri: String,
    pub root_path: String,
}

#[derive(Debug, Clone)]
pub struct RuntimeResourceBindingError {
    pub resource: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeResourceBindingError {
    fn name(&self) -> &str {
        "RuntimeResourceBinding"
    }

    fn message(&self) -> String {
        format!(
            "runtime resource binding `{}` failed: {}",
            self.resource, self.reason
        )
    }
}

pub(in crate::runtime) fn runtime_resource_binding_error(
    resource: impl Into<String>,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        RuntimeResourceBindingError {
            resource: resource.into(),
            reason: reason.into(),
        },
        None,
    )
}

pub(in crate::runtime) fn validate_resource_binding_name(name: &str) -> bool {
    !name.is_empty()
        && name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_' || byte == b'-')
}

impl MechRuntime {
    pub(crate) fn bind_context_export(
        &mut self,
        alias: &str,
        module: &str,
        item: &str,
    ) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("bind_context_export")?;
        self.ensure_runtime_mutation_allowed("bind_context_export")?;
        let target = format!("{module}/{item}");
        let base_uri = match self.host_interfaces.resolve_optional(&target)? {
            Some(context) => context.base_uri.clone(),
            None => self
                .module_manifests
                .context_export(module, item)?
                .base_uri
                .clone(),
        };
        self.bind_resource_root(alias, &base_uri)
    }

    pub fn resource_binding(&self, name: &str) -> Option<&RuntimeResourceBinding> {
        self.resource_bindings.get(name)
    }

    pub(crate) fn bind_resource_root(
        &mut self,
        name: impl Into<String>,
        uri: impl AsRef<str>,
    ) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("bind_resource_root")?;
        self.ensure_runtime_mutation_allowed("bind_resource_root")?;
        let name = name.into();
        if !validate_resource_binding_name(&name) {
            return Err(runtime_resource_binding_error(
                name,
                "resource binding names must be non-empty simple tokens",
            ));
        }

        let uri = uri.as_ref().trim_end_matches('/').to_string();
        let base_uri = self
            .resources
            .provider_base_uri_for(&uri)?
            .unwrap_or_else(|| uri.clone());
        let root_path = uri
            .strip_prefix(&base_uri)
            .unwrap_or_default()
            .trim_matches('/')
            .to_string();

        self.resource_bindings.insert(
            name.clone(),
            RuntimeResourceBinding {
                name,
                base_uri,
                root_path,
            },
        );
        Ok(())
    }

    fn resolve_bound_resource_parts(
        &self,
        binding: &str,
        child_path: &str,
    ) -> MResult<(String, String)> {
        let Some(binding_record) = self.resource_bindings.get(binding) else {
            return Err(runtime_resource_binding_error(
                binding,
                "unknown resource root binding",
            ));
        };

        let child_path = child_path.trim_matches('/');

        let stored_root = if binding_record.root_path.is_empty() {
            binding_record.base_uri.trim_end_matches('/').to_string()
        } else {
            format!(
                "{}/{}",
                binding_record.base_uri.trim_end_matches('/'),
                binding_record.root_path.trim_matches('/'),
            )
        };

        let candidate_uri = if child_path.is_empty() {
            stored_root
        } else {
            format!("{}/{}", stored_root.trim_end_matches('/'), child_path)
        };

        if let Some(provider_base_uri) = self.resources.provider_base_uri_for(&candidate_uri)? {
            let provider_path = candidate_uri
                .strip_prefix(&provider_base_uri)
                .unwrap_or_default()
                .trim_matches('/')
                .to_string();
            return Ok((provider_base_uri, provider_path));
        }

        let full_path = if binding_record.root_path.is_empty() {
            child_path.to_string()
        } else if child_path.is_empty() {
            binding_record.root_path.clone()
        } else {
            format!("{}/{}", binding_record.root_path, child_path)
        };
        Ok((binding_record.base_uri.clone(), full_path))
    }

    pub fn read_bound_resource(
        &mut self,
        binding: &str,
        child_path: &str,
    ) -> MResult<RuntimeValueSnapshot> {
        let (base_uri, path) = self.resolve_bound_resource_parts(binding, child_path)?;
        self.read_resource(RuntimeResourceReadRequest {
            base_uri,
            path,
            context_name: binding.to_string(),
        })
    }

    pub fn write_bound_resource(
        &mut self,
        binding: &str,
        child_path: &str,
        value: &LegacyValue,
    ) -> MResult<()> {
        let (base_uri, path) = self.resolve_bound_resource_parts(binding, child_path)?;
        self.write_resource(RuntimeResourceWriteRequest {
            base_uri,
            path,
            context_name: binding.to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: value.clone(),
            intent: RuntimeResourceWriteIntent::Assign,
        })
    }

    pub fn install_run_resource_grant(&mut self, grant: &RunResourceGrantConfig) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("install_run_resource_grant")?;
        self.ensure_runtime_mutation_allowed("install_run_resource_grant")?;
        let interface = self.host_interfaces.resolve(&grant.target)?;
        for operation in &grant.operations {
            if !interface
                .operations
                .iter()
                .any(|allowed| allowed == operation)
            {
                return Err(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "install_run_resource_grant",
                        reason: format!(
                            "host context `{}` does not expose operation `{operation}`",
                            grant.target
                        ),
                    },
                    None,
                ));
            }
        }
        let operations = grant
            .operations
            .iter()
            .map(|operation| RuntimeCapabilityOperation::from_name(operation.clone()))
            .collect::<MResult<Vec<_>>>()?;
        let spec = RuntimeCapabilityGrantSpec {
            subject: format!("runtime:{}", self.id),
            resource: interface.base_uri.clone(),
            operations,
            paths: grant.paths.clone(),
        };
        let equivalent_base_uris = self
            .resources
            .equivalent_base_uris_for(&interface.base_uri)?;
        let capability = Arc::new(
            ResourcePathCapability::from_spec(self.next_capability_id(), &spec)?
                .with_equivalent_base_uris(equivalent_base_uris)?,
        );
        self.grant_capability(capability).map(|_| ())
    }

    pub(crate) fn register_resource_provider(
        &mut self,
        provider: Box<dyn RuntimeResourceProvider>,
    ) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("register_resource_provider")?;
        self.ensure_runtime_mutation_allowed("register_resource_provider")?;
        self.resources.register_provider(provider)
    }

    pub fn has_resource_provider(&self, scheme: &str) -> bool {
        self.resources.has_provider(scheme)
    }

    pub fn write_resource(&mut self, request: RuntimeResourceWriteRequest) -> MResult<()> {
        let mut context = self.runtime_context()?;
        self.write_resource_with_context(&mut context, request)
            .map(|_| ())
    }

    pub fn write_resource_with_context(
        &mut self,
        context: &mut RuntimeContext,
        mut request: RuntimeResourceWriteRequest,
    ) -> MResult<RuntimeEffectId> {
        self.ensure_runtime_mutation_allowed("write_resource_with_context")?;
        self.validate_context_for_runtime(context)?;
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        request.base_uri = key.base_uri.clone();
        request.path = key.path.clone();

        if context.transaction.is_none() {
            let transaction_id = self.begin_runtime_transaction_internal(
                context,
                RuntimeExecutionTransactionMode::ImplicitResourceOperation,
            )?;
            let effect_id = match self.write_resource_with_context(context, request) {
                Ok(effect_id) => effect_id,
                Err(error) => {
                    return Err(self.cleanup_failed_implicit_resource_operation(
                        context,
                        transaction_id,
                        "write_resource_with_context",
                        error,
                    ));
                }
            };
            return match self.commit_runtime_transaction_internal(context) {
                Ok(RuntimeCommitResolution::Committed(_)) => Ok(effect_id),
                Ok(RuntimeCommitResolution::CommittedWithError { error, .. }) => Err(error),
                Err(error) => Err(self.cleanup_failed_implicit_resource_operation(
                    context,
                    transaction_id,
                    "write_resource_with_context",
                    error,
                )),
            };
        }

        request.value = request.value.try_deep_snapshot()?;
        self.authorize_resource_with_context(context, &request.operation, &key)?;
        if self.execution_mode == RuntimeExecutionMode::Plan {
            self.resources
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: request.base_uri,
                    path: request.path,
                    context_name: request.context_name,
                    operation: request.operation,
                    intent: request.intent,
                })?;
            return Ok(RuntimeEffectId {
                transaction: context
                    .transaction
                    .expect("planning write established a transaction"),
                sequence: u64::MAX,
            });
        }
        let staged_resource = if request.intent == RuntimeResourceWriteIntent::Assign {
            Some((
                self.resources
                    .staged_resource_identity_for(&request.base_uri)?,
                request.path.clone(),
                request.value.clone(),
            ))
        } else {
            None
        };
        let effect = self.resources.prepare_write(request)?;
        match staged_resource {
            Some((base_uri, path, value)) => self
                .stage_runtime_resource_effect_with_context(context, effect, base_uri, path, value),
            None => self.stage_runtime_effect_with_context(context, effect),
        }
    }

    fn cleanup_failed_implicit_resource_operation(
        &mut self,
        context: &mut RuntimeContext,
        transaction_id: TransactionId,
        operation: &'static str,
        original_error: MechError,
    ) -> MechError {
        if context.transaction != Some(transaction_id) {
            return original_error;
        }
        let original_error_text = format!("{:?}", original_error);
        match self.abort_runtime_transaction_cleanup(context, "implicit resource operation failed")
        {
            Ok((cleaned_transaction_id, mut failures)) => {
                if cleaned_transaction_id != transaction_id {
                    failures.push(format!(
                        "implicit resource cleanup targeted transaction {}, expected {}",
                        cleaned_transaction_id, transaction_id,
                    ));
                }
                if failures.is_empty() {
                    original_error
                } else {
                    self.poison_program_operation(
                        operation,
                        Some(transaction_id),
                        original_error_text,
                        failures,
                    )
                }
            }
            Err(cleanup_error) => self.poison_program_operation(
                operation,
                Some(transaction_id),
                original_error_text,
                vec![format!(
                    "implicit resource cleanup could not start: {:?}",
                    cleanup_error,
                )],
            ),
        }
    }

    pub fn read_resource(
        &mut self,
        request: RuntimeResourceReadRequest,
    ) -> MResult<RuntimeValueSnapshot> {
        let mut context = self.runtime_context()?;
        self.read_resource_with_context(&mut context, request)
    }

    pub fn read_resource_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: RuntimeResourceReadRequest,
    ) -> MResult<RuntimeValueSnapshot> {
        self.read_resource_with_context_map(context, request, |value| {
            RuntimeValueSnapshot::try_capture(&value)
        })
    }

    fn read_resource_with_context_map<T>(
        &mut self,
        context: &mut RuntimeContext,
        mut request: RuntimeResourceReadRequest,
        finish: impl FnOnce(LegacyValue) -> MResult<T>,
    ) -> MResult<T> {
        self.validate_context_for_runtime(context)?;
        let key = RuntimeResourceKey::new(&request.base_uri, &request.path)?;
        request.base_uri = key.base_uri.clone();
        request.path = key.path.clone();
        if context.transaction.is_none() {
            let transaction_id = self.begin_runtime_transaction_internal(
                context,
                RuntimeExecutionTransactionMode::ImplicitResourceOperation,
            )?;
            let value = match self.read_resource_with_context_map(context, request, finish) {
                Ok(value) => value,
                Err(error) => {
                    return Err(self.cleanup_failed_implicit_resource_operation(
                        context,
                        transaction_id,
                        "read_resource_with_context",
                        error,
                    ));
                }
            };
            return match self.commit_runtime_transaction_internal(context) {
                Ok(RuntimeCommitResolution::Committed(_)) => Ok(value),
                Ok(RuntimeCommitResolution::CommittedWithError { error, .. }) => Err(error),
                Err(error) => Err(self.cleanup_failed_implicit_resource_operation(
                    context,
                    transaction_id,
                    "read_resource_with_context",
                    error,
                )),
            };
        }
        self.authorize_resource_with_context(context, &RuntimeCapabilityOperation::Read, &key)?;
        if self.execution_mode == RuntimeExecutionMode::Execute && context.transaction.is_some() {
            let transaction_id = context.transaction.unwrap();
            let resource_identity = self
                .resources
                .staged_resource_identity_for(&request.base_uri)?;
            if let Some(value) = self
                .active_execution_transaction(transaction_id)?
                .effects
                .staged_resource_value(&resource_identity, &request.path)
            {
                return finish(value);
            }
        }
        let value = match self.execution_mode {
            RuntimeExecutionMode::Execute => self.resources.read(request)?,
            RuntimeExecutionMode::Plan => self.resources.plan_read(request)?,
        };
        finish(value)
    }

    pub(crate) fn authorize_resource_with_context(
        &mut self,
        context: &mut RuntimeContext,
        operation: &RuntimeCapabilityOperation,
        key: &RuntimeResourceKey,
    ) -> MResult<CapabilityId> {
        let request = CapabilityRequest::from_keys(
            &context.subject,
            operation.name(),
            key.capability_resource(),
        );
        self.check_capability_with_context(context, &request)
    }
}
