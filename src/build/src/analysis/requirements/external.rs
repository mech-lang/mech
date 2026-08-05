use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirement, BytecodeExternalContractResolver, BytecodeHostCallContract,
    BytecodeResourceReadContract, BytecodeResourceWriteContract, MResult, MechError,
    ResourceIntent, Value, validate_stable_value_update,
};
use mech_runtime::{
    ActorHostPlanningState, RuntimeCapabilityOperation, RuntimeConfig, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
};

use crate::{
    NativeHostFunctionContext, NativeRuntimeConfig,
    error::{NativeBuildErrorKind, native_build_error},
    host::NativeHostCatalog,
};

use super::{
    ApplicationRequirementAnalysis, MaterializedConfiguredHost, exact_resource_grant,
    materialize_configured_hosts, normalize_native_runtime_config, resolve_resource_owner,
    validate_resource_authorization,
};
use crate::plan::PlannedApplicationRequirement;

pub(crate) struct NativeBytecodeContractResolver<'catalog> {
    host_catalog: &'catalog NativeHostCatalog,
    materialized: Vec<MaterializedConfiguredHost<'catalog>>,
    normalized_config: Option<NativeRuntimeConfig>,
    actor_state: Option<ActorHostPlanningState>,
    actor_required: bool,
    selected_host_instances: BTreeSet<String>,
    planned: Vec<PlannedApplicationRequirement>,
    run_grants: Vec<mech_runtime::RunResourceGrantConfig>,
    live: bool,
}

impl<'catalog> NativeBytecodeContractResolver<'catalog> {
    pub(crate) fn new(
        requirements: &[ApplicationRequirement],
        runtime_config: Option<&NativeRuntimeConfig>,
        host_catalog: &'catalog NativeHostCatalog,
        target: Option<&str>,
    ) -> MResult<Self> {
        let normalized_config = runtime_config
            .map(normalize_native_runtime_config)
            .transpose()?;
        let needs_resources = requirements
            .iter()
            .any(|requirement| matches!(requirement, ApplicationRequirement::Resource(_)));
        let materialized = if needs_resources {
            materialize_configured_hosts(
                normalized_config.as_ref().ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeRuntimeConfigMissing {
                            requirement: "resource requirement".to_owned(),
                        },
                        None,
                    )
                })?,
                host_catalog,
                target,
            )?
        } else {
            Vec::new()
        };
        let actor_state = normalized_config
            .as_ref()
            .and_then(|config| config.actor_bootstrap.as_ref())
            .map(|bootstrap| {
                ActorHostPlanningState::new(
                    &bootstrap.subject,
                    bootstrap.message_kind.clone(),
                    bootstrap.message_payload.clone(),
                    bootstrap.initial_state.clone(),
                )
            });

        Ok(Self {
            host_catalog,
            materialized,
            normalized_config,
            actor_state,
            actor_required: false,
            selected_host_instances: BTreeSet::new(),
            planned: Vec::new(),
            run_grants: Vec::new(),
            live: false,
        })
    }

    pub(crate) fn finish(mut self) -> MResult<ApplicationRequirementAnalysis> {
        let actor_bootstrap = self
            .normalized_config
            .as_ref()
            .and_then(|config| config.actor_bootstrap.clone());
        match (self.actor_required, actor_bootstrap.is_some()) {
            (true, false) => {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeActorBootstrapMissing,
                    None,
                ));
            }
            (false, true) => {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeActorBootstrapUnused,
                    None,
                ));
            }
            _ => {}
        }
        if self.actor_required && self.live {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeActorLiveApplicationUnsupported,
                None,
            ));
        }
        if self.selected_host_instances.is_empty()
            && self
                .normalized_config
                .as_ref()
                .is_some_and(|config| !config.hosts.is_empty() || !config.run_grants.is_empty())
        {
            return Err(native_build_error(
                NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                    reason: "host instances and run grants cannot be addressed by a build plan without resource requirements"
                        .to_owned(),
                },
                None,
            ));
        }

        self.planned.sort();
        self.planned.dedup();
        self.run_grants.sort_by(|lhs, rhs| {
            (&lhs.target, &lhs.operations, &lhs.paths).cmp(&(
                &rhs.target,
                &rhs.operations,
                &rhs.paths,
            ))
        });
        self.run_grants.dedup();
        let mut hosts = self
            .materialized
            .iter()
            .filter(|host| self.selected_host_instances.contains(&host.config.name))
            .map(MaterializedConfiguredHost::planned)
            .collect::<Vec<_>>();
        hosts.sort_by(|lhs, rhs| (&lhs.name, &lhs.provider).cmp(&(&rhs.name, &rhs.provider)));

        Ok(ApplicationRequirementAnalysis {
            runtime_config: self
                .normalized_config
                .as_ref()
                .map(|config| config.runtime.clone())
                .unwrap_or_else(RuntimeConfig::default),
            actor_bootstrap,
            application_requirements: self.planned,
            hosts,
            run_grants: self.run_grants,
            live: self.live,
        })
    }

    fn record_resource_requirement(
        &mut self,
        request: &mech_core::ExecutionResourceRequest,
        host_instance: String,
        provider: String,
        host_context: String,
        grant_target: String,
    ) {
        self.selected_host_instances.insert(host_instance.clone());
        self.run_grants
            .push(exact_resource_grant(request, grant_target));
        self.live |= request.delivery == mech_core::ResourceDelivery::Live;
        self.planned.push(PlannedApplicationRequirement::Resource {
            base_uri: request.base_uri.clone(),
            path: request.path.clone(),
            context_name: request.context_name.clone(),
            host_context,
            operation: request.operation.clone(),
            intent: request.intent,
            delivery: request.delivery,
            host_instance,
            provider,
        });
    }
}

impl BytecodeExternalContractResolver for NativeBytecodeContractResolver<'_> {
    fn validate_host_call(&mut self, contract: BytecodeHostCallContract<'_>) -> MResult<Value> {
        let linkage = self
            .host_catalog
            .function(&contract.request.name)
            .ok_or_else(|| {
                native_build_error(
                    NativeBuildErrorKind::NativeHostFunctionLinkageMissing {
                        name: contract.request.name.clone(),
                    },
                    None,
                )
            })?;
        self.actor_required |= linkage.context == NativeHostFunctionContext::ActorTurn;
        let planned = match linkage.context {
            NativeHostFunctionContext::ActorTurn => self
                .actor_state
                .as_mut()
                .ok_or_else(|| {
                    native_build_error(NativeBuildErrorKind::NativeActorBootstrapMissing, None)
                })?
                .plan(&contract.request.name, contract.arguments)
                .map_err(|error| {
                    application_instruction_error(
                        contract.instruction,
                        format!(
                            "host function `{}` rejected its arguments: {}",
                            contract.request.name,
                            error.display_message(),
                        ),
                    )
                    .with_source(error)
                })?,
            NativeHostFunctionContext::Standalone => {
                return Err(application_instruction_error(
                    contract.instruction,
                    format!(
                        "host function `{}` has no trusted native planning contract",
                        contract.request.name,
                    ),
                ));
            }
        };
        validate_stable_value_update(contract.output_seed, &planned).map_err(|error| {
            application_instruction_error(
                contract.instruction,
                format!(
                    "host function `{}` destination has seed kind {:?}, but trusted planning returns {:?}: {}",
                    contract.request.name,
                    contract.output_seed.kind(),
                    planned.kind(),
                    error.display_message(),
                ),
            )
            .with_source(error)
        })?;
        self.planned
            .push(PlannedApplicationRequirement::HostFunction {
                name: contract.request.name.clone(),
                context: linkage.context,
                package: linkage.package.to_owned(),
                crate_name: linkage.crate_name.to_owned(),
                installer_path: linkage.installer_path.to_owned(),
                cargo_features: linkage
                    .cargo_features
                    .iter()
                    .map(|feature| (*feature).to_owned())
                    .collect(),
            });
        Ok(planned)
    }

    fn validate_resource_read(
        &mut self,
        contract: BytecodeResourceReadContract<'_>,
    ) -> MResult<Value> {
        let (planned, host_instance, provider, host_context, grant_target) = {
            let owner = resolve_resource_owner(contract.request, &self.materialized)?;
            let configured_grants = self
                .normalized_config
                .as_ref()
                .map(|config| config.run_grants.as_slice())
                .unwrap_or_default();
            let grant_target =
                validate_resource_authorization(contract.request, &owner, configured_grants)?;
            let planned = owner
                .provider
                .plan_read(mech_runtime::RuntimeResourceReadRequest {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: contract.request.context_name.clone(),
                })
                .map_err(|error| {
                    native_build_error(
                        NativeBuildErrorKind::NativeResourcePathInvalid {
                            target: grant_target.clone(),
                            path: contract.request.path.clone(),
                        },
                        None,
                    )
                    .with_source(error)
                })?;
            (
                planned,
                owner.host.config.name.clone(),
                owner.host.config.provider.clone(),
                owner.context.name.clone(),
                grant_target,
            )
        };
        validate_stable_value_update(contract.output_seed, &planned).map_err(|error| {
            application_instruction_error(
                contract.instruction,
                format!(
                    "resource read destination has seed kind {:?}, but trusted planning returns {:?}: {}",
                    contract.output_seed.kind(),
                    planned.kind(),
                    error.display_message(),
                ),
            )
            .with_source(error)
        })?;
        self.record_resource_requirement(
            contract.request,
            host_instance,
            provider,
            host_context,
            grant_target,
        );
        Ok(planned)
    }

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()> {
        if contract.output_seed != &Value::Empty {
            return Err(application_instruction_error(
                contract.instruction,
                format!(
                    "resource write/send destination must have an Empty seed, found {:?}",
                    contract.output_seed.kind(),
                ),
            ));
        }
        let intent = match contract.request.intent {
            ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
            ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
            ResourceIntent::Read => unreachable!("shared traversal validates resource intent"),
        };
        let operation = RuntimeCapabilityOperation::from_name(contract.request.operation.clone())
            .map_err(|error| {
            application_instruction_error(
                contract.instruction,
                format!(
                    "invalid resource operation `{}`",
                    contract.request.operation
                ),
            )
            .with_source(error)
        })?;
        let (host_instance, provider, host_context, grant_target) = {
            let owner = resolve_resource_owner(contract.request, &self.materialized)?;
            let configured_grants = self
                .normalized_config
                .as_ref()
                .map(|config| config.run_grants.as_slice())
                .unwrap_or_default();
            let grant_target =
                validate_resource_authorization(contract.request, &owner, configured_grants)?;
            owner
                .provider
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: contract.request.context_name.clone(),
                    operation: operation.clone(),
                    intent,
                })
                .map_err(|error| {
                    native_build_error(
                        NativeBuildErrorKind::NativeResourcePathInvalid {
                            target: grant_target.clone(),
                            path: contract.request.path.clone(),
                        },
                        None,
                    )
                    .with_source(error)
                })?;
            owner
                .provider
                .plan_write(RuntimeResourceWriteRequest {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: contract.request.context_name.clone(),
                    operation,
                    value: contract.source.clone(),
                    intent,
                })
                .map_err(|error| {
                    application_instruction_error(
                        contract.instruction,
                        format!(
                            "resource write/send `{}/{}` rejected its payload: {}",
                            contract.request.base_uri,
                            contract.request.path,
                            error.display_message(),
                        ),
                    )
                    .with_source(error)
                })?;
            (
                owner.host.config.name.clone(),
                owner.host.config.provider.clone(),
                owner.context.name.clone(),
                grant_target,
            )
        };
        self.record_resource_requirement(
            contract.request,
            host_instance,
            provider,
            host_context,
            grant_target,
        );
        Ok(())
    }
}

fn application_instruction_error(instruction: u32, reason: impl Into<String>) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeApplicationInstructionInvalid {
            instruction,
            reason: reason.into(),
        },
        None,
    )
}
