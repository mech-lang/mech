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
