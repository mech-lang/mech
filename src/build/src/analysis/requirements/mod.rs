use mech_core::ApplicationRequirement;
use mech_runtime::RuntimeConfig;

use crate::{
    NativeActorBootstrap,
    plan::{PlannedApplicationRequirement, PlannedHostInstance, PlannedResourceGrantKey},
};

mod actor;
mod config;
mod external;
mod grants;
mod ownership;
#[cfg(test)]
mod tests;

pub use config::normalize_native_runtime_config;
pub(crate) use external::NativeBytecodeContractResolver;
pub(super) use grants::validate_resource_authorization;
#[cfg(test)]
pub(super) use grants::validate_resource_requirement;
pub(crate) use grants::{
    grant_covers_resource, planned_resource_grant, planned_resource_request,
    runtime_resource_grant, runtime_resource_grant_target,
};
pub(super) use ownership::{
    MaterializedConfiguredHost, materialize_configured_hosts, resolve_resource_owner,
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApplicationRequirementAnalysis {
    pub runtime_config: RuntimeConfig,
    pub actor_bootstrap: Option<NativeActorBootstrap>,
    pub application_requirements: Vec<PlannedApplicationRequirement>,
    pub hosts: Vec<PlannedHostInstance>,
    pub run_grants: Vec<PlannedResourceGrantKey>,
    pub live: bool,
}

pub(crate) fn application_requires_hosting(requirements: &[ApplicationRequirement]) -> bool {
    !requirements.is_empty()
}

#[cfg(test)]
pub(crate) fn analyze_application_requirements(
    requirements: &[ApplicationRequirement],
    runtime_config: Option<&crate::NativeRuntimeConfig>,
    host_catalog: &crate::host::NativeHostCatalog,
    target: Option<&str>,
) -> mech_core::MResult<ApplicationRequirementAnalysis> {
    use std::collections::BTreeSet;

    use mech_core::ResourceDelivery;

    use crate::{
        NativeHostFunctionContext,
        error::{NativeBuildErrorKind, native_build_error},
    };

    let normalized_supplied_config = runtime_config
        .map(normalize_native_runtime_config)
        .transpose()?;
    let planned_runtime_config = normalized_supplied_config
        .as_ref()
        .map(|config| config.runtime.clone())
        .unwrap_or_default();
    let actor_bootstrap = normalized_supplied_config
        .as_ref()
        .and_then(|config| config.actor_bootstrap.clone());
    let mut actor_turn_required = false;
    let mut live_resource_required = false;
    for requirement in requirements {
        match requirement {
            ApplicationRequirement::HostFunction(request) => {
                let linkage = host_catalog.function(&request.name).ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeHostFunctionLinkageMissing {
                            name: request.name.clone(),
                        },
                        None,
                    )
                })?;
                actor_turn_required |= linkage.context == NativeHostFunctionContext::ActorTurn;
            }
            ApplicationRequirement::Resource(request) => {
                live_resource_required |= request.delivery == ResourceDelivery::Live;
            }
        }
    }
    match (actor_turn_required, actor_bootstrap.is_some()) {
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
    if actor_turn_required && live_resource_required {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeActorLiveApplicationUnsupported,
            None,
        ));
    }
    let needs_resources = requirements
        .iter()
        .any(|requirement| matches!(requirement, ApplicationRequirement::Resource(_)));
    if !needs_resources
        && normalized_supplied_config
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

    if requirements.is_empty() {
        return Ok(ApplicationRequirementAnalysis {
            runtime_config: planned_runtime_config,
            actor_bootstrap,
            application_requirements: Vec::new(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
            live: false,
        });
    }

    let normalized_config = if needs_resources {
        Some(normalized_supplied_config.ok_or_else(|| {
            native_build_error(
                NativeBuildErrorKind::NativeRuntimeConfigMissing {
                    requirement: "resource requirement".to_owned(),
                },
                None,
            )
        })?)
    } else {
        None
    };
    let materialized = match &normalized_config {
        Some(config) => materialize_configured_hosts(config, host_catalog, target)?,
        None => Vec::new(),
    };
    let configured_run_grants = normalized_config
        .as_ref()
        .map(|config| config.run_grants.clone())
        .unwrap_or_default();
    let mut planned = Vec::with_capacity(requirements.len());
    let mut selected_host_instances = BTreeSet::new();
    let mut run_grants = Vec::new();
    let mut live = false;

    for requirement in requirements {
        match requirement {
            ApplicationRequirement::HostFunction(request) => {
                let linkage = host_catalog.function(&request.name).ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeHostFunctionLinkageMissing {
                            name: request.name.clone(),
                        },
                        None,
                    )
                })?;
                planned.push(PlannedApplicationRequirement::HostFunction {
                    name: request.name.clone(),
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
            }
            ApplicationRequirement::Resource(request) => {
                let owner = resolve_resource_owner(request, &materialized)?;
                let grant = validate_resource_requirement(request, &owner, &configured_run_grants)?;
                selected_host_instances.insert(owner.host.config.name.clone());
                run_grants.push(grant);
                live |= request.delivery == ResourceDelivery::Live;
                planned.push(PlannedApplicationRequirement::Resource {
                    request: planned_resource_request(request),
                    owner: owner.planned_owner(),
                });
            }
        }
    }

    planned.sort();
    planned.dedup();
    let mut hosts = materialized
        .iter()
        .filter(|host| selected_host_instances.contains(&host.config.name))
        .map(MaterializedConfiguredHost::planned)
        .collect::<Vec<_>>();
    hosts.sort_by(|lhs, rhs| (&lhs.name, &lhs.provider).cmp(&(&rhs.name, &rhs.provider)));
    run_grants.sort();
    run_grants.dedup();
    Ok(ApplicationRequirementAnalysis {
        runtime_config: planned_runtime_config,
        actor_bootstrap,
        application_requirements: planned,
        hosts,
        run_grants,
        live,
    })
}
