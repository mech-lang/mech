use super::transaction::{RuntimeCommitResolution, RuntimeTransactionScope};
use super::{MechRuntime, RuntimeInvalidOperationError};
use crate::{
    CapabilityId, CapabilityRequest, ResourcePathCapability, RunResourceGrantConfig,
    RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation, RuntimeContext, RuntimeEffectId,
    RuntimeResourceKey, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWriteRequest, RuntimeValueSnapshot, TransactionId,
};
use mech_core::{LegacyValue, MResult, MechError};
use std::sync::Arc;

impl MechRuntime {
    fn validate_resource_transaction_scope(
        &self,
        context: &RuntimeContext,
        #[cfg(feature = "source")] operation: &'static str,
        #[cfg(not(feature = "source"))] _: &'static str,
    ) -> MResult<()> {
        let transaction_id = Self::context_transaction_id(context)?;
        match self.active_runtime_transaction(transaction_id)?.scope {
            RuntimeTransactionScope::Explicit
            | RuntimeTransactionScope::ImplicitResourceOperation => Ok(()),
            #[cfg(feature = "source")]
            RuntimeTransactionScope::ImplicitModuleOperation => Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation,
                    reason: format!(
                        "transaction {} belongs to an implicit module operation",
                        transaction_id,
                    ),
                },
                None,
            )),
        }
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
                RuntimeTransactionScope::ImplicitResourceOperation,
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
                Ok(RuntimeCommitResolution::CommittedWithError(error)) => Err(error),
                Err(error) => Err(self.cleanup_failed_implicit_resource_operation(
                    context,
                    transaction_id,
                    "write_resource_with_context",
                    error,
                )),
            };
        }

        self.validate_resource_transaction_scope(context, "write_resource_with_context")?;

        request.value = request.value.try_deep_snapshot()?;
        self.authorize_resource_with_context(context, &request.operation, &key)?;
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
                    self.poison_runtime_operation(
                        operation,
                        Some(transaction_id),
                        original_error_text,
                        failures,
                    )
                }
            }
            Err(cleanup_error) => self.poison_runtime_operation(
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
                RuntimeTransactionScope::ImplicitResourceOperation,
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
                Ok(RuntimeCommitResolution::CommittedWithError(error)) => Err(error),
                Err(error) => Err(self.cleanup_failed_implicit_resource_operation(
                    context,
                    transaction_id,
                    "read_resource_with_context",
                    error,
                )),
            };
        }
        self.validate_resource_transaction_scope(context, "read_resource_with_context")?;
        self.authorize_resource_with_context(context, &RuntimeCapabilityOperation::Read, &key)?;
        if context.transaction.is_some() {
            let transaction_id = context.transaction.unwrap();
            let resource_identity = self
                .resources
                .staged_resource_identity_for(&request.base_uri)?;
            if let Some(value) = self
                .active_runtime_transaction(transaction_id)?
                .effects
                .staged_resource_value(&resource_identity, &request.path)
            {
                return finish(value);
            }
        }
        let value = self.resources.read(request)?;
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
