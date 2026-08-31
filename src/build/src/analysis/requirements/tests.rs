use std::collections::{BTreeMap, BTreeSet};

use mech_core::{
    ApplicationRequirement, BytecodeInstruction, BytecodeProgram, EncodedConstant,
    ExecutionHostFunctionRequest, ExecutionResourceRequest, FunctionCatalog, MResult,
    ParsedProgram, ResourceDelivery, ResourceIntent, RuntimeType, Value, ValueCell, write_bytecode,
};
use mech_runtime::{
    ConfigValue, HostContextManifest, HostInstanceConfig, HostManifestConfig, LogLevel,
    RunResourceGrantConfig, RuntimeConfig, RuntimeHostFactory, RuntimeHostInputDriver,
    RuntimeHostInputSource, RuntimeHostInstallation, RuntimeIngress, RuntimeResourceProvider,
    RuntimeResourceReadRequest, RuntimeResourceWritePreflightRequest, materialize_host_manifest,
};

use super::*;
#[cfg(feature = "full-hosts")]
use crate::host::standard_native_host_catalog;
use crate::host::{NativeHostFunctionLinkage, NativeTargetFamily};
use crate::{
    NativeActorBootstrap, NativeHostCatalog, NativeHostFunctionContext, NativeHostLinkage,
    NativeRuntimeConfig,
};

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

fn planned_grant(
    host_instance: &str,
    host_context: &str,
    operation: &str,
    path: &str,
) -> PlannedResourceGrantKey {
    PlannedResourceGrantKey {
        host_instance: host_instance.to_owned(),
        host_context: host_context.to_owned(),
        operation: operation.to_owned(),
        path: path.to_owned(),
    }
}

fn request(operation: &str, path: &str) -> ExecutionResourceRequest {
    ExecutionResourceRequest {
        base_uri: "test://terminal/output".to_owned(),
        path: path.to_owned(),
        context_name: "output".to_owned(),
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

#[cfg(feature = "full-hosts")]
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
            register_count: constants.len().max(1) as u32,
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

fn push_constant_load(
    constants: &mut Vec<EncodedConstant>,
    instructions: &mut Vec<BytecodeInstruction>,
    next_register: &mut u32,
    constant: EncodedConstant,
) -> u32 {
    let constant_id = constants
        .iter()
        .position(|existing| existing == &constant)
        .unwrap_or_else(|| {
            constants.push(constant);
            constants.len() - 1
        }) as u32;
    let register = *next_register;
    *next_register += 1;
    instructions.push(BytecodeInstruction::ConstLoad {
        dst: register,
        constant: constant_id,
    });
    register
}

#[cfg(feature = "full-hosts")]
fn validate_external_program(
    program: &ParsedProgram,
    runtime_config: Option<&NativeRuntimeConfig>,
    host_catalog: &NativeHostCatalog,
    target: Option<&str>,
) -> MResult<()> {
    let mut resolver = NativeBytecodeContractResolver::new(
        &program.requirements,
        runtime_config,
        host_catalog,
        target,
    )?;
    program.validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)?;
    resolver.finish().map(|_| ())
}

fn analyze_requirements_with_production_resolver(
    requirements: &[ApplicationRequirement],
    runtime_config: Option<&NativeRuntimeConfig>,
    host_catalog: &NativeHostCatalog,
    target: Option<&str>,
) -> MResult<ApplicationRequirementAnalysis> {
    let mut requirements = requirements.to_vec();
    requirements.sort_by(mech_core::compare_application_requirements);
    requirements.dedup();

    let mut constants = Vec::new();
    let mut instructions = Vec::new();
    let mut next_register = 0;
    for (requirement_index, requirement) in requirements.iter().enumerate() {
        match requirement {
            ApplicationRequirement::HostFunction(_) => {
                let dst = push_constant_load(
                    &mut constants,
                    &mut instructions,
                    &mut next_register,
                    string_constant(""),
                );
                instructions.push(BytecodeInstruction::HostCall {
                    requirement: requirement_index as u32,
                    dst,
                    arguments: Vec::new(),
                });
            }
            ApplicationRequirement::Resource(request) if request.intent == ResourceIntent::Read => {
                let dst = next_register;
                next_register += 1;
                instructions.push(BytecodeInstruction::ResourceRead {
                    requirement: requirement_index as u32,
                    dst,
                });
            }
            ApplicationRequirement::Resource(request) => {
                let dst = push_constant_load(
                    &mut constants,
                    &mut instructions,
                    &mut next_register,
                    empty_constant(),
                );
                let src = push_constant_load(
                    &mut constants,
                    &mut instructions,
                    &mut next_register,
                    string_constant("value"),
                );
                instructions.push(match request.intent {
                    ResourceIntent::Assign => BytecodeInstruction::ResourceWrite {
                        requirement: requirement_index as u32,
                        dst,
                        src,
                    },
                    ResourceIntent::Send => BytecodeInstruction::ResourceSend {
                        requirement: requirement_index as u32,
                        dst,
                        src,
                    },
                    ResourceIntent::Read => unreachable!(),
                });
            }
        }
    }
    if constants.is_empty() {
        push_constant_load(
            &mut constants,
            &mut instructions,
            &mut next_register,
            empty_constant(),
        );
    }
    instructions.push(BytecodeInstruction::Return { src: 0 });
    let program = ParsedProgram::from_bytes(&write_bytecode(&BytecodeProgram {
        register_count: next_register,
        constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements,
    })?)?;

    let mut resolver = NativeBytecodeContractResolver::new(
        &program.requirements,
        runtime_config,
        host_catalog,
        target,
    )?;
    program.validate_runtime_contracts_with(&FunctionCatalog::empty(), &mut resolver)?;
    resolver.finish()
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

#[cfg(feature = "full-hosts")]
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
        ValueCell::from_exact("planned".to_owned())?.snapshot()
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
        let resource_providers: Vec<Box<dyn RuntimeResourceProvider>> = match self.addressability {
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

fn live_test_manifest() -> MResult<HostManifestConfig> {
    Ok(HostManifestConfig {
        provider: "test".to_owned(),
        contexts: vec![HostContextManifest {
            name: "output".to_owned(),
            base_uri_template: "test://{instance}/output".to_owned(),
            operations: vec!["read".to_owned()],
        }],
    })
}

#[derive(Debug)]
struct PlanningInputDriver;

impl RuntimeHostInputDriver for PlanningInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == "test://terminal/output" && source.path() == "line"
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        unreachable!("native requirement analysis never attaches planning drivers")
    }

    fn start(&mut self) -> MResult<()> {
        unreachable!("native requirement analysis never starts planning drivers")
    }

    fn stop(&mut self) -> MResult<()> {
        unreachable!("native requirement analysis never stops planning drivers")
    }

    fn is_live(&self) -> bool {
        false
    }
}

#[derive(Debug)]
struct LiveTestHostFactory {
    manifest: HostManifestConfig,
    driven: bool,
}

impl RuntimeHostFactory for LiveTestHostFactory {
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
        let bases = interface
            .contexts
            .iter()
            .map(|context| context.base_uri.clone())
            .collect();
        Ok(RuntimeHostInstallation {
            interface,
            resource_providers: vec![Box::new(TestResourceProvider {
                bases,
                groups: Vec::new(),
            })],
            input_drivers: self
                .driven
                .then(|| Box::new(PlanningInputDriver) as Box<dyn RuntimeHostInputDriver>)
                .into_iter()
                .collect(),
        })
    }
}

fn driven_live_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(LiveTestHostFactory {
        manifest: live_test_manifest()?,
        driven: true,
    }))
}

fn undriven_live_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(LiveTestHostFactory {
        manifest: live_test_manifest()?,
        driven: false,
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
            package: "mech-test-host",
            crate_name: "mech_test_host",
            cargo_features: &["provider"],
            factory_path: "mech_test_host::TestHostFactory::new",
            supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
            manifest,
            validate_settings: validate_test_settings,
            planning_factory,
        })
        .unwrap();
    catalog
}

fn host_catalog() -> NativeHostCatalog {
    catalog_with_actor(host_catalog_with(test_manifest, test_planning_factory))
}

fn catalog_with_actor(mut catalog: NativeHostCatalog) -> NativeHostCatalog {
    for name in [
        "actor/message/kind",
        "actor/message/payload",
        "actor/state/get",
        "actor/state/id",
        "actor/state/put",
    ] {
        catalog
            .insert_function(NativeHostFunctionLinkage {
                name,
                context: NativeHostFunctionContext::ActorTurn,
                package: "mech-runtime",
                crate_name: "mech_runtime",
                cargo_features: &["native-link", "runtime", "string"],
                installer_path: "mech_runtime::__mech_native::synthetic_test_actor_installer",
            })
            .unwrap();
    }
    catalog
}

#[cfg(feature = "full-hosts")]
fn actor_host_catalog() -> NativeHostCatalog {
    catalog_with_actor(NativeHostCatalog::new())
}

fn actor_requirement() -> ApplicationRequirement {
    ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
        name: "actor/message/kind".to_owned(),
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
fn normalizes_grant_paths_with_runtime_authorization_rules() {
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("terminal", "test")],
        run_grants: vec![grant(
            "terminal/output",
            &["write"],
            &["chapter//./one", "chapter//./*", "chapter/one"],
        )],
    };

    let normalized = normalize_native_runtime_config(&config).unwrap();
    assert_eq!(normalized.run_grants[0].paths, ["chapter/*", "chapter/one"]);
}

#[test]
fn rejects_non_finite_host_settings_recursively() {
    for value in [f64::INFINITY, f64::NEG_INFINITY, f64::NAN] {
        let config = NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "terminal".to_owned(),
                provider: "test".to_owned(),
                settings: ConfigValue::Map(BTreeMap::from([(
                    "nested".to_owned(),
                    ConfigValue::List(vec![ConfigValue::Float(value)]),
                )])),
            }],
            run_grants: Vec::new(),
        };
        let error = normalize_native_runtime_config(&config).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
        assert!(error.kind_message().contains("non-finite float"));
        assert!(error.kind_message().contains("nested[0]"));
    }
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
    let missing = analyze_requirements_with_production_resolver(
        &[actor_requirement()],
        None,
        &host_catalog(),
        None,
    )
    .unwrap_err();
    assert_eq!(missing.kind_name(), "NativeActorBootstrapMissing");

    let unused = analyze_requirements_with_production_resolver(
        &[],
        Some(&actor_runtime_config("actor:alpha", "alpha")),
        &host_catalog(),
        None,
    )
    .unwrap_err();
    assert_eq!(unused.kind_name(), "NativeActorBootstrapUnused");

    let analysis = analyze_requirements_with_production_resolver(
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
    let live = ExecutionResourceRequest {
        base_uri: "test://terminal/output".to_owned(),
        path: "line".to_owned(),
        context_name: "output".to_owned(),
        operation: "read".to_owned(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Live,
    };
    let mut config = actor_runtime_config("actor:alpha", "alpha");
    config.hosts.push(host("terminal", "test"));
    config
        .run_grants
        .push(grant("terminal/output", &["read"], &["line"]));
    let error = analyze_requirements_with_production_resolver(
        &[actor_requirement(), ApplicationRequirement::Resource(live)],
        Some(&config),
        &catalog_with_actor(host_catalog_with(
            live_test_manifest,
            driven_live_planning_factory,
        )),
        None,
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "NativeActorLiveApplicationUnsupported");
}

#[test]
fn live_native_plans_require_an_exact_materialized_input_driver() {
    let request = ApplicationRequirement::Resource(ExecutionResourceRequest {
        base_uri: "test://terminal/output".to_owned(),
        path: "line".to_owned(),
        context_name: "output".to_owned(),
        operation: "read".to_owned(),
        intent: ResourceIntent::Read,
        delivery: ResourceDelivery::Live,
    });
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("terminal", "test")],
        run_grants: vec![grant("terminal/output", &["read"], &["line"])],
    };

    let driven = analyze_requirements_with_production_resolver(
        std::slice::from_ref(&request),
        Some(&config),
        &host_catalog_with(live_test_manifest, driven_live_planning_factory),
        None,
    )
    .unwrap();
    assert!(driven.live);

    let error = analyze_requirements_with_production_resolver(
        &[request],
        Some(&config),
        &host_catalog_with(live_test_manifest, undriven_live_planning_factory),
        None,
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
    assert!(error.kind_message().contains("not driven"));
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
        &planned_grant("terminal", "output", "write", "line")
    ));
    assert!(!grant_covers_resource(
        &exact,
        &planned_grant("terminal", "output", "write", "line/one")
    ));

    let prefix = grant("terminal/output", &["read", "write"], &["chapter/*"]);
    assert!(grant_covers_resource(
        &prefix,
        &planned_grant("terminal", "output", "write", "chapter")
    ));
    assert!(grant_covers_resource(
        &prefix,
        &planned_grant("terminal", "output", "write", "chapter/one")
    ));
    assert!(!grant_covers_resource(
        &prefix,
        &planned_grant("terminal", "output", "write", "chapter-two")
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
    let analysis = analyze_requirements_with_production_resolver(
        &[ApplicationRequirement::Resource(request("write", "line"))],
        Some(&config),
        &host_catalog(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap();

    assert_eq!(analysis.hosts.len(), 1);
    assert_eq!(analysis.hosts[0].name, "terminal");
    assert_eq!(analysis.hosts[0].package, "mech-test-host");
    assert_eq!(
        analysis.run_grants,
        [planned_grant("terminal", "output", "write", "line")]
    );
    assert!(!analysis.live);
    assert!(matches!(
        &analysis.application_requirements[0],
        PlannedApplicationRequirement::Resource { owner, .. } if owner.provider == "test"
    ));
}

#[test]
fn runtime_normalized_grant_paths_authorize_canonical_bytecode_paths() {
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("terminal", "test")],
        run_grants: vec![grant("terminal/output", &["write"], &["chapter//./one"])],
    };
    let analysis = analyze_requirements_with_production_resolver(
        &[ApplicationRequirement::Resource(request(
            "write",
            "chapter/one",
        ))],
        Some(&config),
        &host_catalog(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap();

    assert_eq!(
        analysis.run_grants,
        [planned_grant("terminal", "output", "write", "chapter/one",)]
    );
}

#[test]
fn resource_requirement_context_name_must_match_resolved_owner() {
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("terminal", "test")],
        run_grants: vec![grant("terminal/output", &["write"], &["line"])],
    };
    let mut mismatched = request("write", "line");
    mismatched.context_name = "different-output".to_owned();

    let error = analyze_requirements_with_production_resolver(
        &[ApplicationRequirement::Resource(mismatched)],
        Some(&config),
        &host_catalog(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "NativeResourceContextInvalid");
    assert!(error.kind_message().contains("different-output"));
    assert!(error.kind_message().contains("output"));
}

#[test]
fn hosted_requirement_needs_config_and_grant() {
    let requirement = ApplicationRequirement::Resource(request("write", "line"));
    let missing_config = analyze_requirements_with_production_resolver(
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
    let missing_grant = analyze_requirements_with_production_resolver(
        &[requirement],
        Some(&config),
        &host_catalog(),
        None,
    )
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
    let error =
        analyze_requirements_with_production_resolver(&[], Some(&config), &host_catalog(), None)
            .unwrap_err();
    assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
}

#[cfg(feature = "full-hosts")]
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

    for (base_uri, context, operation, path, intent, instance, _target) in cases {
        let requirement = ApplicationRequirement::Resource(request_at(
            base_uri, context, operation, path, intent,
        ));
        let analysis = analyze_requirements_with_production_resolver(
            &[requirement],
            Some(&config),
            &catalog,
            Some("x86_64-unknown-linux-gnu"),
        )
        .unwrap();
        assert_eq!(analysis.hosts.len(), 1);
        assert_eq!(analysis.hosts[0].name, instance);
        assert_eq!(
            analysis.run_grants,
            [planned_grant(instance, context, operation, path)]
        );
        assert!(matches!(
            &analysis.application_requirements[0],
            PlannedApplicationRequirement::Resource {
                request,
                owner,
            } if request.base_uri == base_uri
                && owner.host_instance == instance
                && owner.host_context == context
        ));
    }
}

#[cfg(feature = "full-hosts")]
#[test]
fn cli_stdout_and_stderr_keep_distinct_structured_owners() {
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("cli", "cli")],
        run_grants: vec![
            grant("cli/stdout", &["write"], &["line"]),
            grant("cli/stderr", &["write"], &["line"]),
        ],
    };
    let analysis = analyze_requirements_with_production_resolver(
        &[
            ApplicationRequirement::Resource(request_at(
                "cli://stdout",
                "stdout",
                "write",
                "line",
                ResourceIntent::Send,
            )),
            ApplicationRequirement::Resource(request_at(
                "cli://stderr",
                "stderr",
                "write",
                "line",
                ResourceIntent::Send,
            )),
        ],
        Some(&config),
        &standard_native_host_catalog().unwrap(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap();

    assert_eq!(
        analysis.run_grants,
        [
            planned_grant("cli", "stderr", "write", "line"),
            planned_grant("cli", "stdout", "write", "line"),
        ]
    );
    let owners = analysis
        .application_requirements
        .iter()
        .filter_map(|requirement| match requirement {
            PlannedApplicationRequirement::Resource { owner, .. } => Some((
                owner.host_instance.as_str(),
                owner.host_context.as_str(),
                owner.canonical_base_uri.as_str(),
            )),
            PlannedApplicationRequirement::HostFunction { .. } => None,
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        owners,
        BTreeSet::from([
            ("cli", "stderr", "cli://cli/stderr"),
            ("cli", "stdout", "cli://cli/stdout"),
        ])
    );
}

#[cfg(feature = "full-hosts")]
#[test]
fn standard_provider_rejects_invalid_bytecode_resource_paths() {
    let requirement = request_at(
        "time://clock/clock",
        "clock",
        "read",
        "seconds",
        ResourceIntent::Read,
    );
    let program = parsed_external_program(
        vec![],
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
        run_grants: vec![grant("clock/clock", &["read"], &["*"])],
    };
    let error = validate_external_program(
        &program,
        Some(&config),
        &standard_native_host_catalog().unwrap(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "NativeResourcePathInvalid");
    assert!(error.kind_message().contains("seconds"));
}

#[cfg(feature = "full-hosts")]
#[test]
fn standard_provider_plans_resource_read_output_types() {
    let requirement = request_at(
        "time://clock/clock",
        "clock",
        "read",
        "second",
        ResourceIntent::Read,
    );
    let program = parsed_external_program(
        vec![],
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

    validate_external_program(
        &program,
        Some(&config),
        &standard_native_host_catalog().unwrap(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap();
}

#[cfg(feature = "full-hosts")]
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

    let error = validate_external_program(
        &program,
        Some(&config),
        &standard_native_host_catalog().unwrap(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
    assert!(error.kind_message().contains("rejected its payload"));
}

#[cfg(feature = "full-hosts")]
#[test]
fn trusted_actor_host_calls_plan_arity_and_output_seed_types() {
    let catalog = actor_host_catalog();
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
    let error = validate_external_program(
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
    let error = validate_external_program(
        &wrong_output,
        Some(&config),
        &catalog,
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
    assert!(error.kind_message().contains("seed kind F64"));
    assert!(error.kind_message().contains("String"));

    let wrong_input = parsed_external_program(
        vec![string_constant(""), f64_constant(1.0)],
        BytecodeInstruction::HostCall {
            requirement: 0,
            dst: 0,
            arguments: vec![1],
        },
        ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
            name: "actor/state/put".to_owned(),
        }),
    );
    let error = validate_external_program(
        &wrong_input,
        Some(&config),
        &catalog,
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();
    assert_eq!(error.kind_name(), "NativeApplicationInstructionInvalid");
    assert!(error.kind_message().contains("expected string argument 0"));
}

#[cfg(feature = "full-hosts")]
#[test]
fn actor_put_before_get_updates_the_shared_abstract_register_sequence() {
    let mut config = actor_runtime_config("actor:planner", "message");
    config.actor_bootstrap.as_mut().unwrap().initial_state = None;
    let put = ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
        name: "actor/state/put".to_owned(),
    });
    let get = ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
        name: "actor/state/get".to_owned(),
    });
    let mut requirements = vec![put.clone(), get.clone()];
    requirements.sort();
    let put_requirement = requirements.iter().position(|item| item == &put).unwrap() as u32;
    let get_requirement = requirements.iter().position(|item| item == &get).unwrap() as u32;
    let program = ParsedProgram::from_bytes(
        &write_bytecode(&BytecodeProgram {
            register_count: 3,
            constants: vec![string_constant(""), string_constant("created")],
            symbols: BTreeMap::new(),
            mutable_symbols: BTreeSet::new(),
            instructions: vec![
                BytecodeInstruction::ConstLoad {
                    dst: 0,
                    constant: 0,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 1,
                    constant: 1,
                },
                BytecodeInstruction::ConstLoad {
                    dst: 2,
                    constant: 0,
                },
                BytecodeInstruction::HostCall {
                    requirement: put_requirement,
                    dst: 0,
                    arguments: vec![1],
                },
                BytecodeInstruction::HostCall {
                    requirement: get_requirement,
                    dst: 2,
                    arguments: Vec::new(),
                },
                BytecodeInstruction::Return { src: 2 },
            ],
            dictionary: BTreeMap::new(),
            requirements,
        })
        .unwrap(),
    )
    .unwrap();

    validate_external_program(
        &program,
        Some(&config),
        &actor_host_catalog(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap();
}

#[cfg(feature = "full-hosts")]
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
        let error = analyze_requirements_with_production_resolver(
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
    let analysis = analyze_requirements_with_production_resolver(
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
        [planned_grant("terminal", "output", "write", "line")]
    );
    assert!(matches!(
        &analysis.application_requirements[0],
        PlannedApplicationRequirement::Resource { request, owner }
            if request.base_uri == "test://alias"
                && owner.canonical_base_uri == "test://terminal/output"
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
    let error = analyze_requirements_with_production_resolver(
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
fn configured_grant_cannot_cross_host_instance_ownership() {
    let config = NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![host("first", "test"), host("second", "test")],
        run_grants: vec![grant("second/output", &["write"], &["line"])],
    };
    let error = analyze_requirements_with_production_resolver(
        &[ApplicationRequirement::Resource(request_at(
            "test://first/output",
            "output",
            "write",
            "line",
            ResourceIntent::Send,
        ))],
        Some(&config),
        &host_catalog(),
        Some("x86_64-unknown-linux-gnu"),
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "NativeRunGrantMissing");
    assert!(error.kind_message().contains("first/output"));
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
        let error = analyze_requirements_with_production_resolver(
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
