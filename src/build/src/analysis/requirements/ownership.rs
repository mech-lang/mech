use std::collections::{BTreeMap, BTreeSet};

use mech_core::{ExecutionResourceRequest, MResult, MechError};
use mech_runtime::{
    HostInstanceConfig, MaterializedHostContext, MaterializedHostInterface, RuntimeResourceKey,
    RuntimeResourceProvider, materialize_host_manifest,
};

use crate::{
    NativeRuntimeConfig,
    error::{NativeBuildErrorKind, NativeHostAddressabilityInvalid, native_build_error},
    host::{NativeHostCatalog, NativeHostLinkage, NativeTargetFamily},
    plan::{PlannedHostInstance, PlannedResourceOwner},
};

pub(crate) struct MaterializedConfiguredHost<'catalog> {
    pub(super) config: HostInstanceConfig,
    interface: MaterializedHostInterface,
    addressable_contexts: BTreeMap<String, String>,
    resource_providers: Vec<Box<dyn RuntimeResourceProvider>>,
    linkage: &'catalog NativeHostLinkage,
}

impl MaterializedConfiguredHost<'_> {
    pub(super) fn planned(&self) -> PlannedHostInstance {
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

pub(crate) fn materialize_configured_hosts<'catalog>(
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
            let factory = (linkage.planning_factory)().map_err(|error| {
                addressability_error(
                    host,
                    format!(
                        "planning factory could not be constructed: {}",
                        error.display_message()
                    ),
                )
            })?;
            let installation = factory
                .instantiate(&host.name, &host.settings)
                .map_err(|error| {
                    addressability_error(
                        host,
                        format!(
                            "planning host could not be instantiated: {}",
                            error.display_message()
                        ),
                    )
                })?;
            if installation.interface != interface {
                return Err(addressability_error(
                    host,
                    "planning installation interface differs from the materialized trusted manifest",
                ));
            }
            let addressable_contexts = build_host_addressability(
                host,
                &interface,
                &installation.resource_providers,
            )?;
            Ok(MaterializedConfiguredHost {
                config: host.clone(),
                interface,
                addressable_contexts,
                resource_providers: installation.resource_providers,
                linkage,
            })
        })
        .collect()
}

fn build_host_addressability(
    host: &HostInstanceConfig,
    interface: &MaterializedHostInterface,
    providers: &[Box<dyn RuntimeResourceProvider>],
) -> MResult<BTreeMap<String, String>> {
    let canonical_contexts = interface
        .contexts
        .iter()
        .map(|context| context.base_uri.clone())
        .collect::<BTreeSet<_>>();
    let mut addressable_contexts = canonical_contexts
        .iter()
        .map(|canonical| (canonical.clone(), canonical.clone()))
        .collect::<BTreeMap<_, _>>();
    let mut reported_canonical_contexts = BTreeSet::new();

    for provider in providers {
        let bases = provider.base_uris();
        let base_set = bases.iter().cloned().collect::<BTreeSet<_>>();
        if base_set.len() != bases.len() {
            return Err(addressability_error(
                host,
                "a planning resource provider reports duplicate base URIs",
            ));
        }
        for base in &base_set {
            validate_addressable_base(host, base)?;
            if canonical_contexts.contains(base) {
                reported_canonical_contexts.insert(base.clone());
            }
        }

        let mut grouped_bases = BTreeSet::new();
        for group in provider.equivalent_base_uri_groups() {
            if group.is_empty() {
                return Err(addressability_error(
                    host,
                    "a planning resource provider reports an empty equivalent-base-URI group",
                ));
            }
            let members = group.iter().cloned().collect::<BTreeSet<_>>();
            if members.len() != group.len() {
                return Err(addressability_error(
                    host,
                    "an equivalent-base-URI group contains duplicate entries",
                ));
            }
            if let Some(missing) = members.iter().find(|member| !base_set.contains(*member)) {
                return Err(addressability_error(
                    host,
                    format!(
                        "equivalent base URI `{missing}` is not reported by the provider's base_uris()"
                    ),
                ));
            }
            let canonical = members
                .iter()
                .filter(|member| canonical_contexts.contains(*member))
                .collect::<Vec<_>>();
            if canonical.len() != 1 {
                return Err(addressability_error(
                    host,
                    format!(
                        "equivalent-base-URI group must contain exactly one canonical materialized context, found {}",
                        canonical.len()
                    ),
                ));
            }
            let canonical = (*canonical[0]).clone();
            for member in members {
                insert_addressable_context(
                    host,
                    &mut addressable_contexts,
                    member.clone(),
                    canonical.clone(),
                )?;
                grouped_bases.insert(member);
            }
        }

        if let Some(unattached) = base_set
            .iter()
            .find(|base| !canonical_contexts.contains(*base) && !grouped_bases.contains(*base))
        {
            return Err(addressability_error(
                host,
                format!(
                    "base URI `{unattached}` is neither a materialized context nor part of a valid equivalence group"
                ),
            ));
        }
    }

    if let Some(missing) = canonical_contexts
        .iter()
        .find(|canonical| !reported_canonical_contexts.contains(*canonical))
    {
        return Err(addressability_error(
            host,
            format!(
                "canonical materialized context `{missing}` is not reported by a planning resource provider"
            ),
        ));
    }

    Ok(addressable_contexts)
}

fn validate_addressable_base(host: &HostInstanceConfig, base_uri: &str) -> MResult<()> {
    let key = RuntimeResourceKey::new(base_uri, "addressability").map_err(|error| {
        addressability_error(
            host,
            format!(
                "base URI `{base_uri}` is invalid: {}",
                error.display_message()
            ),
        )
    })?;
    if key.base_uri != base_uri {
        return Err(addressability_error(
            host,
            format!("base URI `{base_uri}` is not canonical"),
        ));
    }
    Ok(())
}

fn insert_addressable_context(
    host: &HostInstanceConfig,
    addressable_contexts: &mut BTreeMap<String, String>,
    base_uri: String,
    canonical: String,
) -> MResult<()> {
    if let Some(existing) = addressable_contexts.get(&base_uri) {
        if existing != &canonical {
            return Err(addressability_error(
                host,
                format!("base URI `{base_uri}` maps to both `{existing}` and `{canonical}`"),
            ));
        }
        return Ok(());
    }
    addressable_contexts.insert(base_uri, canonical);
    Ok(())
}

fn addressability_error(host: &HostInstanceConfig, reason: impl Into<String>) -> MechError {
    MechError::new(
        NativeHostAddressabilityInvalid {
            instance: host.name.clone(),
            provider: host.provider.clone(),
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}

pub(crate) struct ResolvedResourceOwner<'host, 'catalog> {
    pub(super) host: &'host MaterializedConfiguredHost<'catalog>,
    pub(super) context: &'host MaterializedHostContext,
    pub(super) provider: &'host dyn RuntimeResourceProvider,
}

impl ResolvedResourceOwner<'_, '_> {
    pub(super) fn planned_owner(&self) -> PlannedResourceOwner {
        PlannedResourceOwner {
            host_instance: self.host.config.name.clone(),
            provider: self.host.config.provider.clone(),
            host_context: self.context.name.clone(),
            canonical_base_uri: self.context.base_uri.clone(),
        }
    }
}

pub(crate) fn resolve_resource_owner<'host, 'catalog>(
    request: &ExecutionResourceRequest,
    hosts: &'host [MaterializedConfiguredHost<'catalog>],
) -> MResult<ResolvedResourceOwner<'host, 'catalog>> {
    let mut owners = hosts
        .iter()
        .filter_map(|host| {
            host.addressable_contexts
                .get(&request.base_uri)
                .map(|canonical| (host, canonical))
        })
        .collect::<Vec<_>>();
    owners.sort_by(|(lhs, _), (rhs, _)| lhs.config.name.cmp(&rhs.config.name));

    match owners.as_slice() {
        [(owner, canonical)] => {
            let context = owner
                .interface
                .contexts
                .iter()
                .find(|context| context.base_uri == **canonical)
                .expect("addressability maps only to a materialized context");
            let providers = owner
                .resource_providers
                .iter()
                .filter(|provider| {
                    provider
                        .base_uris()
                        .iter()
                        .any(|base| base == &request.base_uri)
                })
                .map(|provider| provider.as_ref())
                .collect::<Vec<&dyn RuntimeResourceProvider>>();
            let [provider] = providers.as_slice() else {
                return Err(addressability_error(
                    &owner.config,
                    format!(
                        "base URI `{}` must resolve to exactly one planning provider, found {}",
                        request.base_uri,
                        providers.len()
                    ),
                ));
            };
            Ok(ResolvedResourceOwner {
                host: owner,
                context,
                provider: *provider,
            })
        }
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
                    .map(|(owner, _)| owner.config.name.clone())
                    .collect::<BTreeSet<_>>()
                    .into_iter()
                    .collect(),
            },
            None,
        )),
    }
}

fn resource_authority(base_uri: &str) -> Option<&str> {
    let (_, remainder) = base_uri.split_once("://")?;
    let end = remainder.find('/').unwrap_or(remainder.len());
    let authority = &remainder[..end];
    (!authority.is_empty()).then_some(authority)
}
