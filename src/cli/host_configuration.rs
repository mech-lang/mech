use std::collections::BTreeSet;

use mech_core::*;
use mech_runtime::{HostInstanceConfig, RunResourceGrantConfig, RuntimeBuilder};

/// Returns configured providers in deterministic order so execute and planning
/// runtimes select factories from the same host configuration surface.
#[cfg(feature = "build")]
pub(crate) fn configured_provider_names(
    configured_hosts: &[HostInstanceConfig],
) -> BTreeSet<String> {
    configured_hosts
        .iter()
        .map(|host| host.provider.clone())
        .collect()
}

/// Materializes only host instances whose factories were registered, then
/// applies grants for those exact instances. Both `mech run` and `mech build`
/// use this path so provider selection, settings, and grant filtering cannot
/// drift between execution and effect-free planning.
pub(crate) fn materialize_host_configuration(
    mut builder: RuntimeBuilder,
    configured_hosts: &[HostInstanceConfig],
    run_grants: &[RunResourceGrantConfig],
    registered_providers: &BTreeSet<String>,
) -> MResult<(RuntimeBuilder, BTreeSet<String>)> {
    for host in configured_hosts {
        if host.name == "cli" && host.provider != "cli" {
            return Err(MechError::new(
                CliRuntimeHostConfigError {
                    reason: format!(
                        "host instance `cli` is reserved for provider `cli` and cannot be configured as provider `{}`",
                        host.provider,
                    ),
                },
                None,
            ));
        }
    }

    let mut registered_instances = BTreeSet::new();
    for host in configured_hosts {
        if registered_providers.contains(&host.provider) {
            registered_instances.insert(host.name.clone());
            builder = builder.host_instance(host.clone());
        }
    }

    builder = materialize_run_grants(builder, run_grants, &registered_instances)?;
    Ok((builder, registered_instances))
}

pub(crate) fn materialize_run_grants(
    mut builder: RuntimeBuilder,
    run_grants: &[RunResourceGrantConfig],
    registered_instances: &BTreeSet<String>,
) -> MResult<RuntimeBuilder> {
    for grant in run_grants {
        let (instance, _) = mech_runtime::parse_host_context_target(&grant.target)?;
        if registered_instances.contains(instance) {
            builder = builder.run_resource_grant(grant.clone());
        }
    }
    Ok(builder)
}

#[derive(Debug, Clone)]
struct CliRuntimeHostConfigError {
    reason: String,
}

impl MechErrorKind for CliRuntimeHostConfigError {
    fn name(&self) -> &str {
        "CliRuntimeHostConfigError"
    }

    fn message(&self) -> String {
        format!("invalid CLI runtime host config: {}", self.reason)
    }
}
