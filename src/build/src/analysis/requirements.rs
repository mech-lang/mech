use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    ApplicationRequirement, BytecodeInstruction, ExecutionResourceRequest, MResult, MechError,
    ParsedProgram, ResourceDelivery, Value,
};
use mech_runtime::{
    ActorHostPlanningState, HostInstanceConfig, MaterializedHostContext, MaterializedHostInterface,
    RunResourceGrantConfig, RuntimeCapabilityOperation, RuntimeConfig, RuntimeResourceKey,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest, materialize_host_manifest,
    validate_run_resource_grant,
};

use crate::{
    NativeActorBootstrap, NativeHostFunctionContext, NativeRuntimeConfig,
    error::{NativeBuildErrorKind, NativeHostAddressabilityInvalid, native_build_error},
    host::{NativeHostCatalog, NativeHostLinkage, NativeTargetFamily},
    plan::{PlannedApplicationRequirement, PlannedHostInstance},
};

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct ApplicationRequirementAnalysis {
    pub runtime_config: RuntimeConfig,
    pub actor_bootstrap: Option<NativeActorBootstrap>,
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
                let grant_target =
                    validate_resource_requirement(request, &owner, &configured_run_grants)?;
                selected_host_instances.insert(owner.host.config.name.clone());
                run_grants.push(exact_resource_grant(request, grant_target));
                live |= request.delivery == ResourceDelivery::Live;
                planned.push(PlannedApplicationRequirement::Resource {
                    base_uri: request.base_uri.clone(),
                    path: request.path.clone(),
                    context_name: request.context_name.clone(),
                    host_context: owner.context.name.clone(),
                    operation: request.operation.clone(),
                    intent: request.intent,
                    delivery: request.delivery,
                    host_instance: owner.host.config.name.clone(),
                    provider: owner.host.config.provider.clone(),
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
    run_grants.sort_by(|lhs, rhs| {
        (&lhs.target, &lhs.operations, &lhs.paths).cmp(&(&rhs.target, &rhs.operations, &rhs.paths))
    });
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

/// Replays the detached constant seeds used by external bytecode instructions
/// through the trusted host/provider planning contracts. Requirement analysis
/// proves linkage and authorization; this pass proves that the generated
/// runtime can actually bind every input and stable output cell.
pub(crate) fn validate_application_instruction_contracts(
    program: &ParsedProgram,
    runtime_config: Option<&NativeRuntimeConfig>,
    host_catalog: &NativeHostCatalog,
    target: Option<&str>,
) -> MResult<()> {
    let normalized_config = runtime_config
        .map(normalize_native_runtime_config)
        .transpose()?;
    let materialized = if program
        .requirements
        .iter()
        .any(|requirement| matches!(requirement, ApplicationRequirement::Resource(_)))
    {
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
    let mut actor_state = normalized_config
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
    let constants = program.decode_constants()?;
    let mut registers = vec![None::<Value>; program.header.register_count as usize];

    for (instruction_index, instruction) in program.instructions.iter().enumerate() {
        if let BytecodeInstruction::ConstLoad { dst, constant } = instruction {
            let value = constants
                .get(*constant as usize)
                .expect("validated bytecode constant index")
                .try_deep_snapshot()?;
            *registers
                .get_mut(*dst as usize)
                .expect("validated bytecode register index") = Some(value);
            continue;
        }

        let seed = |register: u32| -> MResult<Value> {
            registers
                .get(register as usize)
                .and_then(Option::as_ref)
                .cloned()
                .ok_or_else(|| {
                    application_instruction_error(
                        instruction_index,
                        format!("register {register} has no detached constant seed"),
                    )
                })
        };

        match instruction {
            BytecodeInstruction::HostCall {
                requirement,
                dst,
                arguments,
            } => {
                let request = match program.requirements.get(*requirement as usize) {
                    Some(ApplicationRequirement::HostFunction(request)) => request,
                    _ => unreachable!("core runtime-contract validation checks requirement kind"),
                };
                let linkage = host_catalog.function(&request.name).ok_or_else(|| {
                    native_build_error(
                        NativeBuildErrorKind::NativeHostFunctionLinkageMissing {
                            name: request.name.clone(),
                        },
                        None,
                    )
                })?;
                let arguments = arguments
                    .iter()
                    .map(|argument| seed(*argument))
                    .collect::<MResult<Vec<_>>>()?;
                let planned = match linkage.context {
                    NativeHostFunctionContext::ActorTurn => actor_state
                        .as_mut()
                        .ok_or_else(|| {
                            native_build_error(
                                NativeBuildErrorKind::NativeActorBootstrapMissing,
                                None,
                            )
                        })?
                        .plan(&request.name, &arguments)
                        .map_err(|error| {
                            application_instruction_error(
                                instruction_index,
                                format!(
                                    "host function `{}` rejected its arguments: {}",
                                    request.name,
                                    error.display_message(),
                                ),
                            )
                            .with_source(error)
                        })?,
                    NativeHostFunctionContext::Standalone => {
                        return Err(application_instruction_error(
                            instruction_index,
                            format!(
                                "host function `{}` has no trusted native planning contract",
                                request.name,
                            ),
                        ));
                    }
                };
                validate_stable_output_seed(
                    instruction_index,
                    *dst,
                    &seed(*dst)?,
                    &planned,
                    &format!("host function `{}`", request.name),
                )?;
            }
            BytecodeInstruction::ResourceRead { requirement, dst } => {
                let request = resource_requirement(program, *requirement);
                let owner = resolve_resource_owner(request, &materialized)?;
                let planned = owner
                    .provider
                    .plan_read(RuntimeResourceReadRequest {
                        base_uri: request.base_uri.clone(),
                        path: request.path.clone(),
                        context_name: request.context_name.clone(),
                    })
                    .map_err(|error| {
                        application_instruction_error(
                            instruction_index,
                            format!(
                                "resource read `{}/{}` failed trusted planning: {}",
                                request.base_uri,
                                request.path,
                                error.display_message(),
                            ),
                        )
                        .with_source(error)
                    })?;
                validate_stable_output_seed(
                    instruction_index,
                    *dst,
                    &seed(*dst)?,
                    &planned,
                    "resource read",
                )?;
            }
            BytecodeInstruction::ResourceWrite {
                requirement, src, ..
            }
            | BytecodeInstruction::ResourceSend {
                requirement, src, ..
            } => {
                let request = resource_requirement(program, *requirement);
                let owner = resolve_resource_owner(request, &materialized)?;
                let intent = match request.intent {
                    mech_core::ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
                    mech_core::ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
                    mech_core::ResourceIntent::Read => {
                        unreachable!("core runtime-contract validation checks resource intent")
                    }
                };
                let operation = RuntimeCapabilityOperation::from_name(request.operation.clone())
                    .map_err(|error| {
                        application_instruction_error(
                            instruction_index,
                            format!("invalid resource operation `{}`", request.operation),
                        )
                        .with_source(error)
                    })?;
                owner
                    .provider
                    .plan_write(RuntimeResourceWriteRequest {
                        base_uri: request.base_uri.clone(),
                        path: request.path.clone(),
                        context_name: request.context_name.clone(),
                        operation,
                        value: seed(*src)?,
                        intent,
                    })
                    .map_err(|error| {
                        application_instruction_error(
                            instruction_index,
                            format!(
                                "resource write/send `{}/{}` rejected its payload: {}",
                                request.base_uri,
                                request.path,
                                error.display_message(),
                            ),
                        )
                        .with_source(error)
                    })?;
            }
            BytecodeInstruction::RuntimeNullary { dst, .. }
            | BytecodeInstruction::RuntimeUnary { dst, .. }
            | BytecodeInstruction::RuntimeBinary { dst, .. }
            | BytecodeInstruction::RuntimeTernary { dst, .. }
            | BytecodeInstruction::RuntimeQuaternary { dst, .. }
            | BytecodeInstruction::RuntimeVariadic { dst, .. } => {
                // Runtime producers mutate their already initialized output
                // cell. Preserve that cell's detached planned value here so a
                // later resource write can validate a computed payload such
                // as `@out/line <- x + 1`. Catalog-aware bytecode validation,
                // which runs immediately before this pass, proves the output
                // seed and every input have the exact factory representation.
                let _ = seed(*dst)?;
            }
            BytecodeInstruction::ConstLoad { .. } | BytecodeInstruction::Return { .. } => {}
        }
    }
    Ok(())
}

fn resource_requirement(program: &ParsedProgram, requirement: u32) -> &ExecutionResourceRequest {
    match program.requirements.get(requirement as usize) {
        Some(ApplicationRequirement::Resource(request)) => request,
        _ => unreachable!("core runtime-contract validation checks requirement kind"),
    }
}

fn validate_stable_output_seed(
    instruction: usize,
    register: u32,
    seed: &Value,
    planned: &Value,
    source: &str,
) -> MResult<()> {
    let expected = seed.kind();
    let actual = planned.kind();
    if expected == actual {
        return Ok(());
    }
    Err(application_instruction_error(
        instruction,
        format!(
            "{source} destination register {register} has seed kind {expected:?}, but trusted planning returns {actual:?}",
        ),
    ))
}

fn application_instruction_error(instruction: usize, reason: impl Into<String>) -> MechError {
    native_build_error(
        NativeBuildErrorKind::NativeApplicationInstructionInvalid {
            instruction: u32::try_from(instruction).unwrap_or(u32::MAX),
            reason: reason.into(),
        },
        None,
    )
}

fn exact_resource_grant(
    request: &ExecutionResourceRequest,
    target: String,
) -> RunResourceGrantConfig {
    RunResourceGrantConfig {
        target,
        operations: vec![request.operation.clone()],
        paths: vec![request.path.clone()],
    }
}

/// Clones and deterministically normalizes the runtime configuration fields
/// that are permitted to influence a native build plan.
pub fn normalize_native_runtime_config(
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

    let actor_bootstrap = config
        .actor_bootstrap
        .as_ref()
        .map(|bootstrap| {
            let subject = bootstrap.subject.trim().to_owned();
            if subject.is_empty() {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                        reason: "actor bootstrap subject must be non-empty".to_owned(),
                    },
                    None,
                ));
            }
            let message_kind = bootstrap.message_kind.trim().to_owned();
            if message_kind.is_empty() {
                return Err(native_build_error(
                    NativeBuildErrorKind::NativeRuntimeConfigUnsupported {
                        reason: "actor bootstrap message kind must be non-empty".to_owned(),
                    },
                    None,
                ));
            }
            Ok(NativeActorBootstrap {
                subject,
                message_kind,
                message_payload: bootstrap.message_payload.clone(),
                initial_state: bootstrap.initial_state.clone(),
            })
        })
        .transpose()?;

    Ok(NativeRuntimeConfig {
        runtime: config.runtime.clone(),
        hosts,
        run_grants,
        actor_bootstrap,
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

struct MaterializedConfiguredHost<'catalog> {
    config: HostInstanceConfig,
    interface: MaterializedHostInterface,
    addressable_contexts: BTreeMap<String, String>,
    resource_providers: Vec<Box<dyn RuntimeResourceProvider>>,
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
            let factory = (linkage.planning_factory)().map_err(|error| {
                addressability_error(
                    host,
                    format!("planning factory could not be constructed: {}", error.display_message()),
                )
            })?;
            let installation = factory
                .instantiate(&host.name, &host.settings)
                .map_err(|error| {
                    addressability_error(
                        host,
                        format!("planning host could not be instantiated: {}", error.display_message()),
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

struct ResolvedResourceOwner<'host, 'catalog> {
    host: &'host MaterializedConfiguredHost<'catalog>,
    context: &'host MaterializedHostContext,
    provider: &'host dyn RuntimeResourceProvider,
}

fn resolve_resource_owner<'host, 'catalog>(
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

fn validate_resource_requirement(
    request: &ExecutionResourceRequest,
    owner: &ResolvedResourceOwner<'_, '_>,
    run_grants: &[RunResourceGrantConfig],
) -> MResult<String> {
    let context = owner.context;
    let context_target = format!("{}/{}", owner.host.config.name, context.name);

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

    let provider_result = match request.intent {
        mech_core::ResourceIntent::Read => owner
            .provider
            .plan_read(RuntimeResourceReadRequest {
                base_uri: key.base_uri.clone(),
                path: key.path.clone(),
                context_name: request.context_name.clone(),
            })
            .map(|_| ()),
        mech_core::ResourceIntent::Assign | mech_core::ResourceIntent::Send => {
            let intent = match request.intent {
                mech_core::ResourceIntent::Assign => RuntimeResourceWriteIntent::Assign,
                mech_core::ResourceIntent::Send => RuntimeResourceWriteIntent::Send,
                mech_core::ResourceIntent::Read => unreachable!(),
            };
            owner
                .provider
                .preflight_write(RuntimeResourceWritePreflightRequest {
                    base_uri: key.base_uri.clone(),
                    path: key.path.clone(),
                    context_name: request.context_name.clone(),
                    operation: RuntimeCapabilityOperation::from_name(request.operation.clone())
                        .map_err(|_| {
                            native_build_error(
                                NativeBuildErrorKind::NativeResourcePathInvalid {
                                    target: context_target.clone(),
                                    path: request.path.clone(),
                                },
                                None,
                            )
                        })?,
                    intent,
                })
        }
    };
    provider_result.map_err(|_| {
        native_build_error(
            NativeBuildErrorKind::NativeResourcePathInvalid {
                target: context_target.clone(),
                path: request.path.clone(),
            },
            None,
        )
    })?;

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
    Ok(context_target)
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

    use mech_core::{
        BytecodeProgram, EncodedConstant, ExecutionHostFunctionRequest, ResourceIntent,
        RuntimeType, Value, write_bytecode,
    };
    use mech_runtime::{
        ConfigValue, HostContextManifest, HostManifestConfig, LogLevel, RuntimeConfig,
        RuntimeHostFactory, RuntimeHostInstallation, RuntimeResourceReadRequest,
    };

    use super::*;
    #[cfg(feature = "standard-hosts")]
    use crate::host::standard_native_host_catalog;
    use crate::host::{NativeHostFunctionLinkage, NativeTargetFamily};

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

    fn request_at(
        base_uri: &str,
        context_name: &str,
        operation: &str,
        path: &str,
        intent: ResourceIntent,
    ) -> ExecutionResourceRequest {
        ExecutionResourceRequest {
            base_uri: base_uri.to_owned(),
            path: path.to_owned(),
            context_name: context_name.to_owned(),
            operation: operation.to_owned(),
            intent,
            delivery: ResourceDelivery::Snapshot,
        }
    }

    fn parsed_external_program(
        constants: Vec<EncodedConstant>,
        instruction: BytecodeInstruction,
        requirement: ApplicationRequirement,
    ) -> ParsedProgram {
        let mut instructions = constants
            .iter()
            .enumerate()
            .map(|(register, _)| BytecodeInstruction::ConstLoad {
                dst: register as u32,
                constant: register as u32,
            })
            .collect::<Vec<_>>();
        instructions.push(instruction);
        instructions.push(BytecodeInstruction::Return { src: 0 });
        ParsedProgram::from_bytes(
            &write_bytecode(&BytecodeProgram {
                register_count: constants.len() as u32,
                constants,
                symbols: BTreeMap::new(),
                mutable_symbols: BTreeSet::new(),
                instructions,
                dictionary: BTreeMap::new(),
                requirements: vec![requirement],
            })
            .unwrap(),
        )
        .unwrap()
    }

    fn empty_constant() -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::Empty,
            alignment: 1,
            bytes: Vec::new(),
        }
    }

    fn string_constant(value: &str) -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: value.as_bytes().to_vec(),
        }
    }

    fn f64_constant(value: f64) -> EncodedConstant {
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: value.to_bits().to_le_bytes().to_vec(),
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

    #[derive(Debug)]
    struct TestResourceProvider {
        bases: Vec<String>,
        groups: Vec<Vec<String>>,
    }

    impl RuntimeResourceProvider for TestResourceProvider {
        fn scheme(&self) -> &str {
            "test"
        }

        fn base_uris(&self) -> Vec<String> {
            self.bases.clone()
        }

        fn equivalent_base_uri_groups(&self) -> Vec<Vec<String>> {
            self.groups.clone()
        }

        fn plan_read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
            Ok(Value::Empty)
        }

        fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
            unreachable!("resource access is not executed during native planning")
        }

        fn preflight_write(&self, _request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
            Ok(())
        }
    }

    #[derive(Clone, Copy, Debug)]
    enum TestAddressability {
        Canonical,
        Alias,
        EmptyGroup,
        DuplicateGroupEntry,
        GroupWithoutCanonical,
        GroupWithTwoCanonicals,
        AliasMissingFromBases,
        UnattachedBase,
        ConflictingProviders,
    }

    #[derive(Debug)]
    struct TestHostFactory {
        manifest: HostManifestConfig,
        addressability: TestAddressability,
    }

    impl RuntimeHostFactory for TestHostFactory {
        fn provider_name(&self) -> &str {
            "test"
        }

        fn manifest(&self) -> &HostManifestConfig {
            &self.manifest
        }

        fn validate_settings(&self, instance: &str, settings: &ConfigValue) -> MResult<()> {
            validate_test_settings(instance, settings)
        }

        fn instantiate(
            &self,
            instance: &str,
            settings: &ConfigValue,
        ) -> MResult<RuntimeHostInstallation> {
            self.validate_settings(instance, settings)?;
            let interface = materialize_host_manifest(instance, &self.manifest)?;
            let canonical = interface
                .contexts
                .iter()
                .map(|context| context.base_uri.clone())
                .collect::<Vec<_>>();
            let alias = "test://alias".to_owned();
            let resource_providers: Vec<Box<dyn RuntimeResourceProvider>> =
                match self.addressability {
                    TestAddressability::Canonical => vec![Box::new(TestResourceProvider {
                        bases: canonical.clone(),
                        groups: Vec::new(),
                    })],
                    TestAddressability::Alias => vec![Box::new(TestResourceProvider {
                        bases: vec![canonical[0].clone(), alias.clone()],
                        groups: vec![vec![canonical[0].clone(), alias]],
                    })],
                    TestAddressability::EmptyGroup => vec![Box::new(TestResourceProvider {
                        bases: canonical.clone(),
                        groups: vec![Vec::new()],
                    })],
                    TestAddressability::DuplicateGroupEntry => {
                        vec![Box::new(TestResourceProvider {
                            bases: canonical.clone(),
                            groups: vec![vec![canonical[0].clone(), canonical[0].clone()]],
                        })]
                    }
                    TestAddressability::GroupWithoutCanonical => {
                        vec![Box::new(TestResourceProvider {
                            bases: vec![canonical[0].clone(), alias.clone()],
                            groups: vec![vec![alias]],
                        })]
                    }
                    TestAddressability::GroupWithTwoCanonicals => {
                        vec![Box::new(TestResourceProvider {
                            bases: canonical.clone(),
                            groups: vec![canonical.clone()],
                        })]
                    }
                    TestAddressability::AliasMissingFromBases => {
                        vec![Box::new(TestResourceProvider {
                            bases: canonical.clone(),
                            groups: vec![vec![canonical[0].clone(), alias]],
                        })]
                    }
                    TestAddressability::UnattachedBase => {
                        vec![Box::new(TestResourceProvider {
                            bases: vec![canonical[0].clone(), alias],
                            groups: Vec::new(),
                        })]
                    }
                    TestAddressability::ConflictingProviders => vec![
                        Box::new(TestResourceProvider {
                            bases: vec![canonical[0].clone(), alias.clone()],
                            groups: vec![vec![canonical[0].clone(), alias.clone()]],
                        }),
                        Box::new(TestResourceProvider {
                            bases: vec![canonical[1].clone(), alias.clone()],
                            groups: vec![vec![canonical[1].clone(), alias]],
                        }),
                    ],
                };
            Ok(RuntimeHostInstallation {
                interface,
                resource_providers,
                input_drivers: Vec::new(),
            })
        }
    }

    fn test_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
        Ok(Box::new(TestHostFactory {
            manifest: test_manifest()?,
            addressability: TestAddressability::Canonical,
        }))
    }

    fn alias_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
        Ok(Box::new(TestHostFactory {
            manifest: test_manifest()?,
            addressability: TestAddressability::Alias,
        }))
    }

    macro_rules! planning_factory {
        ($name:ident, $mode:ident, $manifest:ident) => {
            fn $name() -> MResult<Box<dyn RuntimeHostFactory>> {
                Ok(Box::new(TestHostFactory {
                    manifest: $manifest()?,
                    addressability: TestAddressability::$mode,
                }))
            }
        };
    }

    planning_factory!(empty_group_factory, EmptyGroup, test_manifest);
    planning_factory!(duplicate_group_factory, DuplicateGroupEntry, test_manifest);
    planning_factory!(no_canonical_factory, GroupWithoutCanonical, test_manifest);
    planning_factory!(missing_alias_factory, AliasMissingFromBases, test_manifest);
    planning_factory!(unattached_base_factory, UnattachedBase, test_manifest);

    fn two_context_manifest() -> MResult<HostManifestConfig> {
        Ok(HostManifestConfig {
            provider: "test".to_owned(),
            contexts: vec![
                HostContextManifest {
                    name: "first".to_owned(),
                    base_uri_template: "test://{instance}/first".to_owned(),
                    operations: vec!["write".to_owned()],
                },
                HostContextManifest {
                    name: "second".to_owned(),
                    base_uri_template: "test://{instance}/second".to_owned(),
                    operations: vec!["write".to_owned()],
                },
            ],
        })
    }

    planning_factory!(
        two_canonical_factory,
        GroupWithTwoCanonicals,
        two_context_manifest
    );
    planning_factory!(
        conflicting_factory,
        ConflictingProviders,
        two_context_manifest
    );

    fn host_catalog_with(
        manifest: fn() -> MResult<HostManifestConfig>,
        planning_factory: fn() -> MResult<Box<dyn RuntimeHostFactory>>,
    ) -> NativeHostCatalog {
        let mut catalog = NativeHostCatalog::new();
        catalog
            .insert_provider(NativeHostLinkage {
                provider: "test",
                package: "mech-host-test",
                crate_name: "mech_host_test",
                cargo_features: &["provider"],
                factory_path: "mech_host_test::TestHostFactory::new",
                supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
                manifest,
                validate_settings: validate_test_settings,
                planning_factory,
            })
            .unwrap();
        catalog
    }

    fn host_catalog() -> NativeHostCatalog {
        let mut catalog = host_catalog_with(test_manifest, test_planning_factory);
        catalog
            .insert_function(NativeHostFunctionLinkage {
                name: "test/actor-turn",
                context: NativeHostFunctionContext::ActorTurn,
                package: "mech-runtime",
                crate_name: "mech_runtime",
                cargo_features: &["native-link", "runtime", "string"],
                installer_path: "mech_runtime::__mech_native::install_actor_message_kind",
            })
            .unwrap();
        catalog
    }

    fn actor_requirement() -> ApplicationRequirement {
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "test/actor-turn".to_owned(),
        })
    }

    fn actor_bootstrap(subject: &str, message_kind: &str) -> NativeActorBootstrap {
        NativeActorBootstrap {
            subject: subject.to_owned(),
            message_kind: message_kind.to_owned(),
            message_payload: String::new(),
            initial_state: Some(String::new()),
        }
    }

    fn actor_runtime_config(subject: &str, message_kind: &str) -> NativeRuntimeConfig {
        NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: Some(actor_bootstrap(subject, message_kind)),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        }
    }

    #[test]
    fn normalizes_hosts_and_grants_deterministically() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
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

        let normalized = normalize_native_runtime_config(&config).unwrap();
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
    fn actor_bootstrap_normalization_trims_identity_but_preserves_empty_values() {
        let normalized =
            normalize_native_runtime_config(&actor_runtime_config("  actor:alpha  ", "  alpha  "))
                .unwrap();
        assert_eq!(
            normalized.actor_bootstrap,
            Some(NativeActorBootstrap {
                subject: "actor:alpha".to_owned(),
                message_kind: "alpha".to_owned(),
                message_payload: String::new(),
                initial_state: Some(String::new()),
            })
        );
    }

    #[test]
    fn actor_bootstrap_rejects_empty_subject_and_message_kind() {
        for config in [
            actor_runtime_config("  ", "message"),
            actor_runtime_config("actor:alpha", "  "),
        ] {
            let error = normalize_native_runtime_config(&config).unwrap_err();
            assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
        }
    }

    #[test]
    fn actor_turn_requires_exactly_one_explicit_bootstrap() {
        let missing =
            analyze_application_requirements(&[actor_requirement()], None, &host_catalog(), None)
                .unwrap_err();
        assert_eq!(missing.kind_name(), "NativeActorBootstrapMissing");

        let unused = analyze_application_requirements(
            &[],
            Some(&actor_runtime_config("actor:alpha", "alpha")),
            &host_catalog(),
            None,
        )
        .unwrap_err();
        assert_eq!(unused.kind_name(), "NativeActorBootstrapUnused");

        let analysis = analyze_application_requirements(
            &[actor_requirement()],
            Some(&actor_runtime_config(" actor:alpha ", " alpha ")),
            &host_catalog(),
            None,
        )
        .unwrap();
        assert_eq!(analysis.actor_bootstrap.unwrap().subject, "actor:alpha");
        assert!(matches!(
            &analysis.application_requirements[0],
            PlannedApplicationRequirement::HostFunction {
                context: NativeHostFunctionContext::ActorTurn,
                ..
            }
        ));
    }

    #[test]
    fn actor_turn_cannot_be_combined_with_live_resources() {
        let mut live = request("write", "line");
        live.delivery = ResourceDelivery::Live;
        let mut config = actor_runtime_config("actor:alpha", "alpha");
        config.hosts.push(host("terminal", "test"));
        config
            .run_grants
            .push(grant("terminal/output", &["write"], &["line"]));
        let error = analyze_application_requirements(
            &[actor_requirement(), ApplicationRequirement::Resource(live)],
            Some(&config),
            &host_catalog(),
            None,
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeActorLiveApplicationUnsupported");
    }

    #[test]
    fn duplicate_host_instances_are_rejected() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("terminal", "other"), host("terminal", "test")],
            run_grants: Vec::new(),
        };
        let error = normalize_native_runtime_config(&config).unwrap_err();
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
            actor_bootstrap: None,
            hosts: vec![host("terminal", "test")],
            run_grants: Vec::new(),
        };

        let normalized = normalize_native_runtime_config(&config).unwrap();
        assert_eq!(normalized.runtime, runtime);
    }

    #[test]
    fn invalid_scalar_runtime_config_is_rejected_before_plan_addressing() {
        let mut runtime = RuntimeConfig::default();
        runtime.limits.max_steps_per_turn = Some(0);
        let error = normalize_native_runtime_config(&NativeRuntimeConfig {
            runtime,
            actor_bootstrap: None,
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
    fn exact_resource_requirement_prunes_unused_hosts_and_narrows_grants() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("unused", "test"), host("terminal", "test")],
            run_grants: vec![grant("terminal/output", &["read", "write"], &["*", "line"])],
        };
        let analysis = analyze_application_requirements(
            &[ApplicationRequirement::Resource(request("write", "line"))],
            Some(&config),
            &host_catalog(),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap();

        assert_eq!(analysis.hosts.len(), 1);
        assert_eq!(analysis.hosts[0].name, "terminal");
        assert_eq!(analysis.hosts[0].package, "mech-host-test");
        assert_eq!(
            analysis.run_grants,
            [grant("terminal/output", &["write"], &["line"])]
        );
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
            actor_bootstrap: None,
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
            actor_bootstrap: None,
            hosts: vec![host("cargo-feature-from-bytecode", "evil-provider")],
            run_grants: Vec::new(),
        };
        let error = analyze_application_requirements(&[], Some(&config), &host_catalog(), None)
            .unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn standard_default_instances_accept_canonical_and_alias_resource_uris() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("cli", "cli"), host("console", "console")],
            run_grants: vec![
                grant("cli/env", &["read"], &["HOME"]),
                grant("cli/stderr", &["write"], &["line"]),
                grant("cli/stdout", &["write"], &["line"]),
                grant("console/output", &["write"], &["line"]),
            ],
        };
        let catalog = standard_native_host_catalog().unwrap();
        let cases = [
            (
                "cli://cli/stdout",
                "stdout",
                "write",
                "line",
                ResourceIntent::Send,
                "cli",
                "cli/stdout",
            ),
            (
                "cli://stdout",
                "stdout",
                "write",
                "line",
                ResourceIntent::Send,
                "cli",
                "cli/stdout",
            ),
            (
                "cli://stderr",
                "stderr",
                "write",
                "line",
                ResourceIntent::Send,
                "cli",
                "cli/stderr",
            ),
            (
                "cli://env",
                "env",
                "read",
                "HOME",
                ResourceIntent::Read,
                "cli",
                "cli/env",
            ),
            (
                "console://console/output",
                "output",
                "write",
                "line",
                ResourceIntent::Send,
                "console",
                "console/output",
            ),
            (
                "console://output",
                "output",
                "write",
                "line",
                ResourceIntent::Send,
                "console",
                "console/output",
            ),
        ];

        for (base_uri, context, operation, path, intent, instance, target) in cases {
            let requirement = ApplicationRequirement::Resource(request_at(
                base_uri, context, operation, path, intent,
            ));
            let analysis = analyze_application_requirements(
                &[requirement],
                Some(&config),
                &catalog,
                Some("x86_64-unknown-linux-gnu"),
            )
            .unwrap();
            assert_eq!(analysis.hosts.len(), 1);
            assert_eq!(analysis.hosts[0].name, instance);
            assert_eq!(analysis.run_grants, [grant(target, &[operation], &[path])]);
            assert!(matches!(
                &analysis.application_requirements[0],
                PlannedApplicationRequirement::Resource {
                    base_uri: planned_base,
                    host_instance,
                    ..
                } if planned_base == base_uri && host_instance == instance
            ));
        }
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn standard_provider_rejects_invalid_bytecode_resource_paths() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("clock", "time")],
            run_grants: vec![grant("clock/clock", &["read"], &["*"])],
        };
        let error = analyze_application_requirements(
            &[ApplicationRequirement::Resource(request_at(
                "time://clock/clock",
                "clock",
                "read",
                "seconds",
                ResourceIntent::Read,
            ))],
            Some(&config),
            &standard_native_host_catalog().unwrap(),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "NativeResourcePathInvalid");
        assert!(error.kind_message().contains("seconds"));
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn standard_provider_plans_resource_read_output_seed_types() {
        let requirement = request_at(
            "time://clock/clock",
            "clock",
            "read",
            "second",
            ResourceIntent::Read,
        );
        let program = parsed_external_program(
            vec![string_constant("wrong-seed")],
            BytecodeInstruction::ResourceRead {
                requirement: 0,
                dst: 0,
            },
            ApplicationRequirement::Resource(requirement),
        );
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("clock", "time")],
            run_grants: vec![grant("clock/clock", &["read"], &["second"])],
        };

        let error = validate_application_instruction_contracts(
            &program,
            Some(&config),
            &standard_native_host_catalog().unwrap(),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
        assert!(error.kind_message().contains("seed kind String"));
        assert!(error.kind_message().contains("F64"));
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn standard_provider_plans_resource_write_payload_types() {
        let requirement = request_at(
            "scene://scene/frame",
            "frame",
            "write",
            "replace",
            ResourceIntent::Send,
        );
        let program = parsed_external_program(
            vec![empty_constant(), string_constant("not-a-scene")],
            BytecodeInstruction::ResourceSend {
                requirement: 0,
                dst: 0,
                src: 1,
            },
            ApplicationRequirement::Resource(requirement),
        );
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "scene".to_owned(),
                provider: "scene".to_owned(),
                settings: ConfigValue::Map(BTreeMap::from([
                    (
                        "renderer".to_owned(),
                        ConfigValue::String("canvas".to_owned()),
                    ),
                    (
                        "selector".to_owned(),
                        ConfigValue::String("#scene".to_owned()),
                    ),
                ])),
            }],
            run_grants: vec![grant("scene/frame", &["write"], &["replace"])],
        };

        let error = validate_application_instruction_contracts(
            &program,
            Some(&config),
            &standard_native_host_catalog().unwrap(),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();

        assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
        assert!(error.kind_message().contains("rejected its payload"));
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn trusted_actor_host_calls_plan_arity_and_output_seed_types() {
        let catalog = standard_native_host_catalog().unwrap();
        let config = actor_runtime_config("actor:planner", "message");
        let missing_argument = parsed_external_program(
            vec![string_constant("")],
            BytecodeInstruction::HostCall {
                requirement: 0,
                dst: 0,
                arguments: Vec::new(),
            },
            ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
                name: "actor/state/put".to_owned(),
            }),
        );
        let error = validate_application_instruction_contracts(
            &missing_argument,
            Some(&config),
            &catalog,
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
        assert!(error.kind_message().contains("expected 1 arguments"));

        let wrong_output = parsed_external_program(
            vec![f64_constant(0.0)],
            BytecodeInstruction::HostCall {
                requirement: 0,
                dst: 0,
                arguments: Vec::new(),
            },
            ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
                name: "actor/message/kind".to_owned(),
            }),
        );
        let error = validate_application_instruction_contracts(
            &wrong_output,
            Some(&config),
            &catalog,
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
        assert!(error.kind_message().contains("seed kind F64"));
        assert!(error.kind_message().contains("String"));
    }

    #[cfg(feature = "standard-hosts")]
    #[test]
    fn standard_aliases_do_not_address_non_default_instances() {
        let catalog = standard_native_host_catalog().unwrap();
        for (instance, provider, target, alias) in [
            ("terminal", "cli", "terminal/stdout", "cli://stdout"),
            ("display", "console", "display/output", "console://output"),
        ] {
            let config = NativeRuntimeConfig {
                runtime: RuntimeConfig::default(),
                actor_bootstrap: None,
                hosts: vec![host(instance, provider)],
                run_grants: vec![grant(target, &["write"], &["line"])],
            };
            let error = analyze_application_requirements(
                &[ApplicationRequirement::Resource(request_at(
                    alias,
                    "output",
                    "write",
                    "line",
                    ResourceIntent::Send,
                ))],
                Some(&config),
                &catalog,
                Some("x86_64-unknown-linux-gnu"),
            )
            .unwrap_err();
            assert_eq!(error.kind_name(), "NativeHostInstanceUnknown");
        }
    }

    #[test]
    fn trusted_alias_resolves_owner_but_preserves_requested_uri() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("terminal", "test")],
            run_grants: vec![grant("terminal/output", &["write"], &["line"])],
        };
        let analysis = analyze_application_requirements(
            &[ApplicationRequirement::Resource(request_at(
                "test://alias",
                "output",
                "write",
                "line",
                ResourceIntent::Send,
            ))],
            Some(&config),
            &host_catalog_with(test_manifest, alias_planning_factory),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap();
        assert_eq!(analysis.hosts[0].name, "terminal");
        assert_eq!(
            analysis.run_grants,
            [grant("terminal/output", &["write"], &["line"])]
        );
        assert!(matches!(
            &analysis.application_requirements[0],
            PlannedApplicationRequirement::Resource { base_uri, .. }
                if base_uri == "test://alias"
        ));
    }

    #[test]
    fn one_alias_claimed_by_two_configured_hosts_is_ambiguous() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("first", "test"), host("second", "test")],
            run_grants: Vec::new(),
        };
        let error = analyze_application_requirements(
            &[ApplicationRequirement::Resource(request_at(
                "test://alias",
                "output",
                "write",
                "line",
                ResourceIntent::Send,
            ))],
            Some(&config),
            &host_catalog_with(test_manifest, alias_planning_factory),
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap_err();
        assert_eq!(error.kind_name(), "NativeResourceOwnerAmbiguous");
        assert!(error.kind_message().contains("first, second"));
    }

    #[test]
    fn malformed_planning_addressability_is_rejected() {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![host("terminal", "test")],
            run_grants: vec![grant("terminal/output", &["write"], &["line"])],
        };
        let requirement = ApplicationRequirement::Resource(request("write", "line"));
        for (manifest, factory, diagnostic) in [
            (
                test_manifest as fn() -> MResult<HostManifestConfig>,
                empty_group_factory as fn() -> MResult<Box<dyn RuntimeHostFactory>>,
                "empty",
            ),
            (test_manifest, duplicate_group_factory, "duplicate"),
            (test_manifest, no_canonical_factory, "exactly one"),
            (two_context_manifest, two_canonical_factory, "found 2"),
            (test_manifest, missing_alias_factory, "not reported"),
            (test_manifest, unattached_base_factory, "neither"),
            (two_context_manifest, conflicting_factory, "maps to both"),
        ] {
            let error = analyze_application_requirements(
                std::slice::from_ref(&requirement),
                Some(&config),
                &host_catalog_with(manifest, factory),
                Some("x86_64-unknown-linux-gnu"),
            )
            .unwrap_err();
            assert_eq!(error.kind_name(), "NativeHostAddressabilityInvalid");
            assert!(
                error.kind_message().contains(diagnostic),
                "missing {diagnostic:?} in {}",
                error.kind_message()
            );
        }
    }
}
