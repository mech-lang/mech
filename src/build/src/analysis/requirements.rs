use std::collections::BTreeSet;

use mech_core::{ApplicationRequirement, ExecutionResourceRequest, MResult, ResourceDelivery};
use mech_runtime::{
    HostInstanceConfig, MaterializedHostInterface, RunResourceGrantConfig, RuntimeConfig,
    RuntimeResourceKey, materialize_host_manifest, validate_run_resource_grant,
};

use crate::{
    NativeRuntimeConfig,
    error::{NativeBuildErrorKind, native_build_error},
    host::{NativeHostCatalog, NativeHostLinkage, NativeTargetFamily},
    plan::{PlannedApplicationRequirement, PlannedHostInstance},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApplicationRequirementAnalysis {
    pub runtime_config: RuntimeConfig,
    pub application_requirements: Vec<PlannedApplicationRequirement>,
    pub hosts: Vec<PlannedHostInstance>,
    pub run_grants: Vec<RunResourceGrantConfig>,
    pub live: bool,
}

pub(crate) fn application_requires_hosting(requirements: &[ApplicationRequirement]) -> bool {
    !requirements.is_empty()
}

/// Resolves host and resource requirements exclusively through the trusted
/// native-host catalog and normalized runtime configuration.
pub(crate) fn analyze_application_requirements(
    requirements: &[ApplicationRequirement],
    runtime_config: Option<&NativeRuntimeConfig>,
    host_catalog: &NativeHostCatalog,
    target: Option<&str>,
) -> MResult<ApplicationRequirementAnalysis> {
    // Validate and normalize every supplied configuration before any
    // requirement-based early return. Scalar runtime settings are part of the
    // frozen plan identity even when no concrete host is selected.
    let normalized_supplied_config = runtime_config.map(normalize_runtime_config).transpose()?;
    let planned_runtime_config = normalized_supplied_config
        .as_ref()
        .map(|config| config.runtime.clone())
        .unwrap_or_default();
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
            application_requirements: Vec::new(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
            live: false,
        });
    }

    let normalized_config = if needs_resources {
        let config = normalized_supplied_config.ok_or_else(|| {
            native_build_error(
                NativeBuildErrorKind::NativeRuntimeConfigMissing {
                    requirement: "resource requirement".to_owned(),
                },
                None,
            )
        })?;
        Some(config)
    } else {
        None
    };

    let materialized = match &normalized_config {
        Some(config) => materialize_configured_hosts(config, host_catalog, target)?,
        None => Vec::new(),
    };
    let mut hosts = materialized
        .iter()
        .map(MaterializedConfiguredHost::planned)
        .collect::<Vec<_>>();
    hosts.sort_by(|lhs, rhs| (&lhs.name, &lhs.provider).cmp(&(&rhs.name, &rhs.provider)));

    let run_grants = normalized_config
        .as_ref()
        .map(|config| config.run_grants.clone())
        .unwrap_or_default();
    let mut planned = Vec::with_capacity(requirements.len());
    let mut live = false;

    for requirement in requirements {
        match requirement {
            ApplicationRequirement::HostFunction(request) => {
                let linkage = host_catalog.function(&request.name).ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeHostProviderUnknown {
                            provider: format!("host-function: {}", request.name),
                        },
                        None,
                    )
                })?;
                planned.push(PlannedApplicationRequirement::HostFunction {
                    name: request.name.clone(),
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
                validate_resource_requirement(request, owner, &run_grants)?;
                live |= request.delivery == ResourceDelivery::Live;
                planned.push(PlannedApplicationRequirement::Resource {
                    base_uri: request.base_uri.clone(),
                    path: request.path.clone(),
                    context_name: request.context_name.clone(),
                    operation: request.operation.clone(),
                    intent: request.intent,
                    delivery: request.delivery,
                    host_instance: owner.config.name.clone(),
                    provider: owner.config.provider.clone(),
                });
            }
        }
    }

    planned.sort();
    planned.dedup();
    Ok(ApplicationRequirementAnalysis {
        runtime_config: planned_runtime_config,
        application_requirements: planned,
        hosts,
        run_grants,
        live,
    })
}

/// Clones and deterministically normalizes the runtime configuration fields
/// that are permitted to influence a native build plan.
pub(crate) fn normalize_runtime_config(
    config: &NativeRuntimeConfig,
) -> MResult<NativeRuntimeConfig> {
    config.runtime.validate()?;

    let mut hosts = config.hosts.clone();
    hosts.sort_by(|lhs, rhs| (&lhs.name, &lhs.provider).cmp(&(&rhs.name, &rhs.provider)));
    if let Some(duplicate) = hosts.windows(2).find(|pair| pair[0].name == pair[1].name) {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeRuntimeConfigDuplicateHostInstance {
                instance: duplicate[0].name.clone(),
            },
            None,
        ));
    }

    let mut run_grants = config.run_grants.clone();
    for grant in &mut run_grants {
        grant.operations.sort();
        grant.operations.dedup();
        grant.paths.sort();
        grant.paths.dedup();
        validate_run_resource_grant(grant)?;
    }
    run_grants.sort_by(|lhs, rhs| {
        (&lhs.target, &lhs.operations, &lhs.paths).cmp(&(&rhs.target, &rhs.operations, &rhs.paths))
    });
    run_grants.dedup();

    Ok(NativeRuntimeConfig {
        runtime: config.runtime.clone(),
        hosts,
        run_grants,
    })
}

/// Tests whether one normalized run grant authorizes the exact operation and
/// path required by a resource instruction. URI ownership and target selection
/// must be resolved through the trusted host catalog first.
pub(crate) fn grant_covers_resource(
    grant: &RunResourceGrantConfig,
    target: &str,
    request: &ExecutionResourceRequest,
) -> bool {
    grant.target == target
        && grant
            .operations
            .binary_search_by(|operation| operation.as_str().cmp(&request.operation))
            .is_ok()
        && grant
            .paths
            .iter()
            .any(|scope| resource_path_scope_matches(scope, &request.path))
}

#[derive(Clone)]
struct MaterializedConfiguredHost<'catalog> {
    config: HostInstanceConfig,
    interface: MaterializedHostInterface,
    linkage: &'catalog NativeHostLinkage,
}

impl MaterializedConfiguredHost<'_> {
    fn planned(&self) -> PlannedHostInstance {
        PlannedHostInstance {
            name: self.config.name.clone(),
            provider: self.config.provider.clone(),
            package: self.linkage.package.to_owned(),
            crate_name: self.linkage.crate_name.to_owned(),
            cargo_features: self
                .linkage
                .cargo_features
                .iter()
                .map(|feature| (*feature).to_owned())
                .collect(),
            factory_path: self.linkage.factory_path.to_owned(),
            settings: self.config.settings.clone(),
        }
    }
}

fn materialize_configured_hosts<'catalog>(
    config: &NativeRuntimeConfig,
    host_catalog: &'catalog NativeHostCatalog,
    target: Option<&str>,
) -> MResult<Vec<MaterializedConfiguredHost<'catalog>>> {
    let target_family = NativeTargetFamily::resolve(target);
    config
        .hosts
        .iter()
        .map(|host| {
            let linkage = host_catalog.provider(&host.provider).ok_or_else(|| {
                native_build_error(
                    NativeBuildErrorKind::NativeHostProviderUnknown {
                        provider: host.provider.clone(),
                    },
                    None,
                )
            })?;
            if !linkage.supported_targets.contains(&target_family) {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeTargetUnsupported {
                        provider: host.provider.clone(),
                        target: target.map(str::to_owned),
                    },
                    None,
                ));
            }
            (linkage.validate_settings)(&host.name, &host.settings).map_err(|error| {
                native_build_error(
                    NativeBuildErrorKind::NativeHostSettingsInvalid {
                        instance: host.name.clone(),
                        reason: error.display_message(),
                    },
                    None,
                )
            })?;
            let manifest = (linkage.manifest)()?;
            let interface = materialize_host_manifest(&host.name, &manifest)?;
            Ok(MaterializedConfiguredHost {
                config: host.clone(),
                interface,
                linkage,
            })
        })
        .collect()
}

fn resolve_resource_owner<'host, 'catalog>(
    request: &ExecutionResourceRequest,
    hosts: &'host [MaterializedConfiguredHost<'catalog>],
) -> MResult<&'host MaterializedConfiguredHost<'catalog>> {
    let mut owners = hosts
        .iter()
        .filter(|host| {
            host.interface
                .contexts
                .iter()
                .any(|context| context.base_uri == request.base_uri)
        })
        .collect::<Vec<_>>();
    owners.sort_by(|lhs, rhs| lhs.config.name.cmp(&rhs.config.name));

    match owners.as_slice() {
        [owner] => Ok(*owner),
        [] => {
            let instance = resource_authority(&request.base_uri).unwrap_or(&request.base_uri);
            if hosts.iter().any(|host| host.config.name == instance) {
                Err(native_build_error(
                    NativeBuildErrorKind::NativeResourcePathInvalid {
                        target: request.base_uri.clone(),
                        path: request.path.clone(),
                    },
                    None,
                ))
            } else {
                Err(native_build_error(
                    NativeBuildErrorKind::NativeHostInstanceUnknown {
                        instance: instance.to_owned(),
                    },
                    None,
                ))
            }
        }
        _ => Err(native_build_error(
            NativeBuildErrorKind::NativeResourceOwnerAmbiguous {
                target: request.base_uri.clone(),
                instances: owners
                    .iter()
                    .map(|owner| owner.config.name.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
            None,
        )),
    }
}

fn validate_resource_requirement(
    request: &ExecutionResourceRequest,
    owner: &MaterializedConfiguredHost<'_>,
    run_grants: &[RunResourceGrantConfig],
) -> MResult<()> {
    let context = owner
        .interface
        .contexts
        .iter()
        .find(|context| context.base_uri == request.base_uri)
        .expect("resource owner was selected from this exact context");
    let context_target = format!("{}/{}", owner.config.name, context.name);

    if !context
        .operations
        .iter()
        .any(|operation| operation == &request.operation)
    {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeContextOperationInvalid {
                target: context_target,
                operation: request.operation.clone(),
            },
            None,
        ));
    }

    let key = RuntimeResourceKey::new(&request.base_uri, &request.path).map_err(|_| {
        native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: context_target.clone(),
                path: request.path.clone(),
            },
            None,
        )
    })?;
    if key.base_uri != request.base_uri || key.path != request.path {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: context_target.clone(),
                path: request.path.clone(),
            },
            None,
        ));
    }

    if !run_grants
        .iter()
        .any(|grant| grant_covers_resource(grant, &context_target, request))
    {
        return Err(native_build_error(
            NativeBuildErrorKind::NativeRunGrantMissing {
                target: context_target,
                operation: request.operation.clone(),
                path: request.path.clone(),
            },
            None,
        ));
    }
    Ok(())
}

fn resource_authority(base_uri: &str) -> Option<&str> {
    base_uri
        .split_once("://")
        .map(|(_, remainder)| remainder.split('/').next().unwrap_or_default())
        .filter(|authority| !authority.is_empty())
}

fn resource_path_scope_matches(scope: &str, path: &str) -> bool {
    if scope == "*" {
        return true;
    }
    let Some(prefix) = scope.strip_suffix("/*") else {
        return scope == path;
    };
    path == prefix
        || path
            .strip_prefix(prefix)
            .is_some_and(|remainder| remainder.starts_with('/'))
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeMap;

    use mech_core::ResourceIntent;
    use mech_runtime::{
        ConfigValue, HostContextManifest, HostManifestConfig, LogLevel, RuntimeConfig,
    };

    use super::*;
    use crate::host::NativeTargetFamily;

    fn host(name: &str, provider: &str) -> HostInstanceConfig {
        HostInstanceConfig {
            name: name.to_owned(),
            provider: provider.to_owned(),
            settings: ConfigValue::Map(BTreeMap::new()),
        }
    }

    fn grant(target: &str, operations: &[&str], paths: &[&str]) -> RunResourceGrantConfig {
        RunResourceGrantConfig {
            target: target.to_owned(),
            operations: operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect(),
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        }
    }

    fn request(operation: &str, path: &str) -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: "test://terminal/output".to_owned(),
            path: path.to_owned(),
            context_name: "out".to_owned(),
            operation: operation.to_owned(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
        }
    }

    fn test_manifest() -> MResult<HostManifestConfig> {
        Ok(HostManifestConfig {
            provider: "test".to_owned(),
            contexts: vec![HostContextManifest {
                name: "output".to_owned(),
                base_uri_template: "test://{instance}/output".to_owned(),
                operations: vec!["write".to_owned()],
            }],
        })
    }

    fn validate_test_settings(_instance: &str, settings: &ConfigValue) -> MResult<()> {
        assert!(matches!(settings, ConfigValue::Map(_)));
        Ok(())
    }

    fn host_catalog() -> NativeHostCatalog {
        let mut catalog = NativeHostCatalog::new();
        catalog
            .insert_provider(NativeHostLinkage {
                provider: "test",
                package: "mech-host-test",
                crate_name: "mech_host_test",
                cargo_features: &["provider"],
                factory_path: "mech_host_test::TestHostFactory::new",
                supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
                manifest: test_manifest,
                validate_settings: validate_test_settings,
            })
            .unwrap();
        catalog
    }

    #[test]
    fn normalizes_hosts_and_grants_deterministically() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host("z", "test"), host("a", "test")],
            run_grants: vec![
                grant("z/output", &["write"], &["text"]),
                grant(
                    "a/output",
                    &["write", "read", "write"],
                    &["line", "*", "line"],
                ),
            ],
        };

        let normalized = normalize_runtime_config(&config).unwrap();
        assert_eq!(
            normalized
                .hosts
                .iter()
                .map(|host| host.name.as_str())
                .collect::<Vec<_>>(),
            ["a", "z"]
        );
        assert_eq!(normalized.run_grants[0].target, "a/output");
        assert_eq!(normalized.run_grants[0].operations, ["read", "write"]);
        assert_eq!(normalized.run_grants[0].paths, ["*", "line"]);
    }

    #[test]
    fn duplicate_host_instances_are_rejected() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host("terminal", "other"), host("terminal", "test")],
            run_grants: Vec::new(),
        };
        let error = normalize_runtime_config(&config).unwrap_err();
        assert_eq!(
            error.kind_name(),
            "NativeRuntimeConfigDuplicateHostInstance"
        );
    }

    #[test]
    fn non_default_scalar_runtime_config_is_normalized_into_plan_identity() {
        let mut runtime = RuntimeConfig::default();
        runtime.name = "custom-native-runtime".to_owned();
        runtime.limits.max_steps_per_turn = Some(321);
        runtime.diagnostics.trace_enabled = true;
        runtime.diagnostics.log_level = LogLevel::Debug;
        let config = NativeRuntimeConfig {
            runtime: runtime.clone(),
            hosts: vec![host("terminal", "test")],
            run_grants: Vec::new(),
        };

        let normalized = normalize_runtime_config(&config).unwrap();
        assert_eq!(normalized.runtime, runtime);
    }

    #[test]
    fn invalid_scalar_runtime_config_is_rejected_before_plan_addressing() {
        let mut runtime = RuntimeConfig::default();
        runtime.limits.max_steps_per_turn = Some(0);
        let error = normalize_runtime_config(&NativeRuntimeConfig {
            runtime,
            hosts: Vec::new(),
            run_grants: Vec::new(),
        })
        .unwrap_err();
        assert_eq!(error.kind_name(), "InvalidRuntimeConfig");
    }

    #[test]
    fn grant_matching_respects_exact_and_prefix_path_boundaries() {
        let exact = grant("terminal/output", &["write"], &["line"]);
        assert!(grant_covers_resource(
            &exact,
            "terminal/output",
            &request("write", "line")
        ));
        assert!(!grant_covers_resource(
            &exact,
            "terminal/output",
            &request("write", "line/one")
        ));

        let prefix = grant("terminal/output", &["read", "write"], &["chapter/*"]);
        assert!(grant_covers_resource(
            &prefix,
            "terminal/output",
            &request("write", "chapter")
        ));
        assert!(grant_covers_resource(
            &prefix,
            "terminal/output",
            &request("write", "chapter/one")
        ));
        assert!(!grant_covers_resource(
            &prefix,
            "terminal/output",
            &request("write", "chapter-two")
        ));
    }

    #[test]
    fn exact_resource_requirement_selects_trusted_provider_metadata() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host("terminal", "test")],
            run_grants: vec![grant("terminal/output", &["write"], &["line"])],
        };
        let analysis = analyze_application_requirements(
            &[ApplicationRequirement::Resource(request("write", "line"))],
            Some(&config),
            &host_catalog(),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap();

        assert_eq!(analysis.hosts.len(), 1);
        assert_eq!(analysis.hosts[0].package, "mech-host-test");
        assert_eq!(analysis.run_grants, config.run_grants);
        assert!(!analysis.live);
        assert!(matches!(
            &analysis.application_requirements[0],
            PlannedApplicationRequirement::Resource { provider, .. } if provider == "test"
        ));
    }

    #[test]
    fn hosted_requirement_needs_config_and_grant() {
        let requirement = ApplicationRequirement::Resource(request("write", "line"));
        let missing_config = analyze_application_requirements(
            std::slice::from_ref(&requirement),
            None,
            &host_catalog(),
            None,
        )
        .unwrap_err();
        assert_eq!(missing_config.kind_name(), "NativeRuntimeConfigMissing");

        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host("terminal", "test")],
            run_grants: Vec::new(),
        };
        let missing_grant =
            analyze_application_requirements(&[requirement], Some(&config), &host_catalog(), None)
                .unwrap_err();
        assert_eq!(missing_grant.kind_name(), "NativeRunGrantMissing");
    }

    #[test]
    fn host_free_analysis_rejects_unaddressed_untrusted_config_strings() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![host("cargo-feature-from-bytecode", "evil-provider")],
            run_grants: Vec::new(),
        };
        let error = analyze_application_requirements(&[], Some(&config), &host_catalog(), None)
            .unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
    }
}
