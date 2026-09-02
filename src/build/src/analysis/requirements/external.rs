use std::collections::BTreeSet;

use mech_core::{
    ApplicationRequirement, BytecodeExternalContractResolver, BytecodeHostCallContract,
    BytecodeResourceReadContract, BytecodeResourceWriteContract, MResult, MechError,
    ResourceIntent, Value, ValueData, validate_stable_value_update,
};
use mech_runtime::{
    RuntimeCapabilityOperation, RuntimeConfig, RuntimeResourceWriteCommand,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest,
};

use crate::{
    NativeHostFunctionContext, NativeRuntimeConfig,
    error::{NativeBuildErrorKind, native_build_error},
    host::NativeHostCatalog,
};

use super::actor::NativeActorPlanning;
use super::{
    ApplicationRequirementAnalysis, MaterializedConfiguredHost, materialize_configured_hosts,
    normalize_native_runtime_config, planned_resource_request, resolve_resource_owner,
    runtime_resource_grant_target, validate_resource_authorization,
};
use crate::plan::{PlannedApplicationRequirement, PlannedResourceGrantKey, PlannedResourceOwner};

pub(crate) struct NativeBytecodeContractResolver<'catalog> {
    host_catalog: &'catalog NativeHostCatalog,
    materialized: Vec<MaterializedConfiguredHost<'catalog>>,
    normalized_config: Option<NativeRuntimeConfig>,
    actor: NativeActorPlanning,
    selected_host_instances: BTreeSet<String>,
    planned: Vec<PlannedApplicationRequirement>,
    run_grants: Vec<PlannedResourceGrantKey>,
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
        if !needs_resources
            && normalized_config
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
        let actor = NativeActorPlanning::new(normalized_config.as_ref());

        Ok(Self {
            host_catalog,
            materialized,
            normalized_config,
            actor,
            selected_host_instances: BTreeSet::new(),
            planned: Vec::new(),
            run_grants: Vec::new(),
            live: false,
        })
    }

    pub(crate) fn finish(mut self) -> MResult<ApplicationRequirementAnalysis> {
        let actor_bootstrap = self.actor.finish(self.live)?;
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
        self.run_grants.sort();
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
        owner: PlannedResourceOwner,
        grant: PlannedResourceGrantKey,
    ) {
        self.selected_host_instances
            .insert(owner.host_instance.clone());
        self.run_grants.push(grant);
        self.live |= request.delivery == mech_core::ResourceDelivery::Live;
        self.planned.push(PlannedApplicationRequirement::Resource {
            request: planned_resource_request(request, &owner.host_context),
            owner,
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
        let planned = match linkage.context {
            NativeHostFunctionContext::ActorTurn => self.actor.plan(
                contract.instruction,
                &contract.request.name,
                contract.arguments,
            )?,
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
                    contract.output_seed.data().kind(),
                    planned.data().kind(),
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
        let (planned, owner, grant, driven_live) = {
            let owner = resolve_resource_owner(contract.request, &self.materialized)?;
            let configured_grants = self
                .normalized_config
                .as_ref()
                .map(|config| config.run_grants.as_slice())
                .unwrap_or_default();
            let grant =
                validate_resource_authorization(contract.request, &owner, configured_grants)?;
            let planned = owner
                .provider
                .plan_read(mech_runtime::RuntimeResourceReadRequest {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: owner.context.name.clone(),
                })
                .map_err(|error| {
                    native_build_error(
                        NativeBuildErrorKind::NativeResourcePathInvalid {
                            target: runtime_resource_grant_target(&grant),
                            path: contract.request.path.clone(),
                        },
                        None,
                    )
                    .with_source(error)
                })?;
            let driven_live = if contract.request.delivery == mech_core::ResourceDelivery::Live {
                owner
                    .has_input_driver_for(contract.request)
                    .map_err(|error| {
                        application_instruction_error(
                            contract.instruction,
                            format!(
                                "live resource read `{}/{}` has an invalid input source: {}",
                                contract.request.base_uri,
                                contract.request.path,
                                error.display_message(),
                            ),
                        )
                        .with_source(error)
                    })?
            } else {
                false
            };
            (planned, owner.planned_owner(), grant, driven_live)
        };
        if contract.request.delivery == mech_core::ResourceDelivery::Live && !driven_live {
            return Err(application_instruction_error(
                contract.instruction,
                format!(
                    "live resource read `{}/{}` is not driven by its materialized native host",
                    contract.request.base_uri, contract.request.path,
                ),
            ));
        }
        self.record_resource_requirement(contract.request, owner, grant);
        Ok(planned)
    }

    fn validate_resource_write(
        &mut self,
        contract: BytecodeResourceWriteContract<'_>,
    ) -> MResult<()> {
        if !matches!(contract.output_seed.data(), ValueData::Tuple(values) if values.is_empty()) {
            return Err(application_instruction_error(
                contract.instruction,
                format!(
                    "resource write/send destination must have an Empty seed, found {:?}",
                    contract.output_seed.data().kind(),
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
        let (owner, grant) = {
            let owner = resolve_resource_owner(contract.request, &self.materialized)?;
            let configured_grants = self
                .normalized_config
                .as_ref()
                .map(|config| config.run_grants.as_slice())
                .unwrap_or_default();
            let grant =
                validate_resource_authorization(contract.request, &owner, configured_grants)?;
            owner
                .provider
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: owner.context.name.clone(),
                    operation: operation.clone(),
                    intent,
                })
                .map_err(|error| {
                    native_build_error(
                        NativeBuildErrorKind::NativeResourcePathInvalid {
                            target: runtime_resource_grant_target(&grant),
                            path: contract.request.path.clone(),
                        },
                        None,
                    )
                    .with_source(error)
                })?;
            owner
                .provider
                .plan_write(RuntimeResourceWriteCommand {
                    base_uri: contract.request.base_uri.clone(),
                    path: contract.request.path.clone(),
                    context_name: owner.context.name.clone(),
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
            (owner.planned_owner(), grant)
        };
        self.record_resource_requirement(contract.request, owner, grant);
        Ok(())
    }
}

pub(super) fn application_instruction_error(
    instruction: u32,
    reason: impl Into<String>,
) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeApplicationInstructionInvalid {
            instruction,
            reason: reason.into(),
        },
        None,
    )
}
