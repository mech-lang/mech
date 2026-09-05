use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
    path::PathBuf,
    sync::Arc,
};

use mech_build::{
    NativeActorBootstrap, NativeApplicationBuilder, NativeApplicationKind, NativeBuildEnvironment,
    NativeBuildPlan, NativeBuildProfile, NativeBuildRequest, NativeDependencySource, NativeEmit,
    NativeHostCatalog, NativeHostFunctionContext, NativeHostFunctionLinkage, NativeRuntimeConfig,
    WorkspacePackage, fingerprint_workspace,
};
#[cfg(not(feature = "full-hosts"))]
use mech_build::{NativeHostLinkage, NativeTargetFamily};
#[cfg(not(feature = "full-hosts"))]
use mech_core::Value;
use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram,
    EncodedConstant, ExecutionHostFunctionRequest, FunctionCatalog, FunctionCatalogBuilder,
    FunctionInvocation, FunctionRuntimeType, FunctionValueRepresentation, MResult, MatrixStorage,
    MechFunction, MechFunctionCompiler, MechFunctionFactory, MechFunctionImpl,
    NativeFunctionLinkage, Ref, Register, RuntimeFamilyId, RuntimeFunctionContract,
    RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType, RuntimeTypeTag, hash_str,
    write_bytecode,
};
use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};
use sha2::{Digest, Sha256};

#[path = "support/isolated.rs"]
pub mod isolated;
use isolated::{OwnerProfile, RunnerAction, fixture_path, run_owner, workspace_root};
#[cfg(not(feature = "full-hosts"))]
use mech_runtime::{
    HostContextManifest, HostManifestConfig, RuntimeHostFactory, RuntimeHostInstallation,
    RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeResourceWriteIntent,
    RuntimeResourceWritePreflightRequest, materialize_host_manifest,
};

const LITERAL_F64: &[u8] =
    include_bytes!("../../../tests/architecture/bytecode-v1/literal-f64.mecb");
const CLI_STDOUT: &[u8] = include_bytes!("../../../tests/architecture/bytecode-v1/cli-stdout.mecb");

#[cfg(not(feature = "full-hosts"))]
fn cli_manifest() -> MResult<HostManifestConfig> {
    Ok(HostManifestConfig {
        provider: "cli".to_owned(),
        contexts: vec![
            HostContextManifest {
                name: "env".to_owned(),
                base_uri_template: "cli://{instance}/env".to_owned(),
                operations: vec!["read".to_owned()],
            },
            HostContextManifest {
                name: "stdout".to_owned(),
                base_uri_template: "cli://{instance}/stdout".to_owned(),
                operations: vec!["write".to_owned()],
            },
            HostContextManifest {
                name: "stderr".to_owned(),
                base_uri_template: "cli://{instance}/stderr".to_owned(),
                operations: vec!["write".to_owned()],
            },
        ],
    })
}

#[cfg(not(feature = "full-hosts"))]
fn validate_cli_settings(_instance: &str, _settings: &ConfigValue) -> MResult<()> {
    Ok(())
}

#[cfg(not(feature = "full-hosts"))]
#[derive(Debug)]
struct PlanningCliResourceProvider {
    instance: String,
}

#[cfg(not(feature = "full-hosts"))]
impl RuntimeResourceProvider for PlanningCliResourceProvider {
    fn scheme(&self) -> &str {
        "cli"
    }

    fn base_uris(&self) -> Vec<String> {
        ["env", "stdout", "stderr"]
            .into_iter()
            .map(|context| format!("cli://{}/{context}", self.instance))
            .collect()
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<Value> {
        unreachable!("native planning does not execute resource access")
    }

    fn preflight_write(&self, request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        let output = request.base_uri == format!("cli://{}/stdout", self.instance)
            || request.base_uri == format!("cli://{}/stderr", self.instance);
        if output
            && request.intent == RuntimeResourceWriteIntent::Send
            && matches!(request.path.as_str(), "text" | "line")
        {
            Ok(())
        } else {
            Err(mech_core::MechError::new(
                mech_core::GenericError {
                    msg: "unsupported planning CLI write".into(),
                },
                None,
            ))
        }
    }
}

#[cfg(not(feature = "full-hosts"))]
#[derive(Debug)]
struct PlanningCliHostFactory {
    manifest: HostManifestConfig,
}

#[cfg(not(feature = "full-hosts"))]
impl RuntimeHostFactory for PlanningCliHostFactory {
    fn provider_name(&self) -> &str {
        "cli"
    }

    fn manifest(&self) -> &HostManifestConfig {
        &self.manifest
    }

    fn validate_settings(&self, instance: &str, settings: &ConfigValue) -> MResult<()> {
        validate_cli_settings(instance, settings)
    }

    fn instantiate(
        &self,
        instance: &str,
        settings: &ConfigValue,
    ) -> MResult<RuntimeHostInstallation> {
        self.validate_settings(instance, settings)?;
        Ok(RuntimeHostInstallation {
            interface: materialize_host_manifest(instance, &self.manifest)?,
            resource_providers: vec![Box::new(PlanningCliResourceProvider {
                instance: instance.to_owned(),
            })],
            input_drivers: Vec::new(),
        })
    }
}

#[cfg(not(feature = "full-hosts"))]
fn cli_planning_factory() -> MResult<Box<dyn RuntimeHostFactory>> {
    Ok(Box::new(PlanningCliHostFactory {
        manifest: cli_manifest()?,
    }))
}

#[cfg(feature = "full-hosts")]
fn cli_host_catalog() -> Arc<NativeHostCatalog> {
    mech_build::standard_native_host_catalog().unwrap()
}

#[cfg(not(feature = "full-hosts"))]
fn cli_host_catalog() -> Arc<NativeHostCatalog> {
    let mut catalog = NativeHostCatalog::new();
    catalog
        .insert_provider(NativeHostLinkage {
            provider: "cli",
            package: "mech-terminal",
            crate_name: "mech_terminal",
            cargo_features: &["provider"],
            factory_path: "mech_terminal::CliHostFactory::new",
            supported_targets: &[NativeTargetFamily::Unix, NativeTargetFamily::Windows],
            manifest: cli_manifest,
            validate_settings: validate_cli_settings,
            planning_factory: cli_planning_factory,
        })
        .unwrap();
    Arc::new(catalog)
}

fn environment(function_catalog: Arc<FunctionCatalog>) -> NativeBuildEnvironment {
    NativeBuildEnvironment {
        function_catalog,
        host_catalog: cli_host_catalog(),
        dependency_source: NativeDependencySource::Registry {
            version: "0.3.5".to_owned(),
        },
    }
}

fn empty_catalog() -> Arc<FunctionCatalog> {
    Arc::new(FunctionCatalogBuilder::new().build().unwrap())
}

fn request(bytecode: &[u8]) -> NativeBuildRequest {
    NativeBuildRequest {
        bytecode: bytecode.to_vec(),
        instruction_type_bindings: None,
        instruction_type_binding_requirements: None,
        runtime_config: None,
        target: None,
        profile: NativeBuildProfile::Debug,
        binary_name: "native_app".to_owned(),
        output: PathBuf::from("ignored-output"),
        emit: NativeEmit::Plan,
        keep_project: false,
        offline: true,
    }
}

fn cli_runtime_config(provider: &str, operations: &[&str], paths: &[&str]) -> NativeRuntimeConfig {
    NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: None,
        hosts: vec![HostInstanceConfig {
            name: "cli".to_owned(),
            provider: provider.to_owned(),
            settings: ConfigValue::Map(BTreeMap::new()),
        }],
        run_grants: vec![RunResourceGrantConfig {
            target: "cli/stdout".to_owned(),
            operations: operations
                .iter()
                .map(|operation| (*operation).to_owned())
                .collect(),
            paths: paths.iter().map(|path| (*path).to_owned()).collect(),
        }],
    }
}

fn unaddressed_runtime_configs() -> Vec<NativeRuntimeConfig> {
    vec![
        NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "unused".to_owned(),
                provider: "cli".to_owned(),
                settings: ConfigValue::Map(BTreeMap::new()),
            }],
            run_grants: Vec::new(),
        },
        NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
            hosts: Vec::new(),
            run_grants: vec![RunResourceGrantConfig {
                target: "unused/output".to_owned(),
                operations: vec!["write".to_owned()],
                paths: vec!["line".to_owned()],
            }],
        },
    ]
}

fn plan(bytecode: &[u8]) -> mech_build::NativeBuildPlan {
    NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request(bytecode))
        .unwrap()
}

fn assert_owner_runtime_function(
    profile: OwnerProfile,
    fixture: &str,
    case: &str,
    name: &str,
    installer_path: &str,
    package: &str,
) -> NativeBuildPlan {
    let plan = run_owner(
        profile,
        RunnerAction::Plan,
        case,
        fixture_path(fixture),
        "native_planning",
        false,
    )
    .plan;
    assert_eq!(plan.application_kind, NativeApplicationKind::Engine);
    let function = plan
        .runtime_functions
        .iter()
        .find(|function| function.runtime_name == name)
        .unwrap_or_else(|| panic!("actual owner catalog did not plan `{name}`"));
    assert_eq!(function.runtime_name, name);
    assert_eq!(function.runtime_id, hash_str(name));
    assert_eq!(function.installer_path, installer_path);
    assert_eq!(function.package, package);
    plan
}

#[test]
fn literal_only_bytecode_yields_an_engine_plan_without_runtime_config() {
    let plan = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Plan,
        "literal",
        fixture_path("literal-f64.mecb"),
        "native_literal_planning",
        false,
    )
    .plan;

    assert_eq!(plan.application_kind, NativeApplicationKind::Engine);
    assert!(plan.runtime_functions.is_empty());
    assert!(plan.application_requirements.is_empty());
    assert!(plan.hosts.is_empty());
    assert!(plan.run_grants.is_empty());
    let workspace_lock = fs::read(workspace_root().join("Cargo.lock")).unwrap();
    assert_eq!(
        plan.dependency_resolution_seed_sha256,
        format!("{:x}", Sha256::digest(workspace_lock))
    );
    assert!(plan.core_features.iter().any(|feature| feature == "f64"));
    assert!(plan.engine_features.iter().any(|feature| feature == "f64"));
}

#[test]
fn host_free_plan_accepts_scalar_runtime_config_as_plan_identity() {
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    let mut request = request(LITERAL_F64);
    let mut runtime = RuntimeConfig::default();
    runtime.name = "custom-native-runtime".to_owned();
    runtime.limits.max_steps_per_turn = Some(777);
    request.runtime_config = Some(NativeRuntimeConfig {
        runtime: runtime.clone(),
        actor_bootstrap: None,
        hosts: Vec::new(),
        run_grants: Vec::new(),
    });

    let plan = builder.plan(&request).unwrap();
    assert_eq!(plan.application_kind, NativeApplicationKind::Hosted);
    assert_eq!(plan.runtime_config, runtime);
}

#[test]
fn production_builder_rejects_actor_bootstrap_before_planning() {
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    let mut request = request(LITERAL_F64);
    request.runtime_config = Some(NativeRuntimeConfig {
        runtime: RuntimeConfig::default(),
        actor_bootstrap: Some(NativeActorBootstrap {
            subject: "actor:test".to_owned(),
            message_kind: "test".to_owned(),
            message_payload: "payload".to_owned(),
            initial_state: None,
        }),
        hosts: Vec::new(),
        run_grants: Vec::new(),
    });

    let error = builder.plan(&request).unwrap_err();
    assert_eq!(error.kind_name(), "NativeActorBootstrapUnsupported");
    assert!(error.display_message().contains("actor bootstrap"));
}

#[test]
fn host_free_plan_rejects_unaddressed_hosts_and_grants() {
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    for config in unaddressed_runtime_configs() {
        let mut request = request(LITERAL_F64);
        request.runtime_config = Some(config);

        let error = builder.plan(&request).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
    }
}

#[test]
fn host_function_only_plan_rejects_unaddressed_runtime_config() {
    const HOST_FUNCTION: &str = "native-host-function";
    let mut host_catalog = NativeHostCatalog::new();
    host_catalog
        .insert_function(NativeHostFunctionLinkage {
            name: HOST_FUNCTION,
            context: NativeHostFunctionContext::Standalone,
            package: "mech-test-host",
            crate_name: "mech_test_host",
            cargo_features: &["provider"],
            installer_path: "mech_test_host::install_native_host_function",
        })
        .unwrap();
    let mut build_environment = environment(empty_catalog());
    build_environment.host_catalog = Arc::new(host_catalog);

    let builder = NativeApplicationBuilder::new(build_environment);
    for config in unaddressed_runtime_configs() {
        let mut request = request(&host_function_only_bytecode(HOST_FUNCTION));
        request.runtime_config = Some(config);

        let error = builder.plan(&request).unwrap_err();
        assert_eq!(error.kind_name(), "NativeRuntimeConfigUnsupported");
    }
}

#[cfg(feature = "full-hosts")]
#[test]
fn standard_catalog_rejects_untrusted_host_functions() {
    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request(&host_function_only_bytecode(
            "untrusted/arbitrary/function",
        )))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeHostFunctionLinkageMissing");
}

#[test]
fn scalar_add_resolves_only_the_exact_scalar_installer() {
    assert_owner_runtime_function(
        OwnerProfile::Standard,
        "scalar-add-f64.mecb",
        "scalar",
        "AddSS<f64>",
        "mech_math::__mech_native::install_add_ss_f64",
        "mech-math",
    );
}

#[test]
fn fixed_profile_matrix_add_resolves_the_canonical_dynamic_installer() {
    let plan = assert_owner_runtime_function(
        OwnerProfile::Fixed,
        "fixed-matrix-add-f64.mecb",
        "fixed",
        "AddMDMD<f64>",
        "mech_math::__mech_native::install_add_mdmd_f64",
        "mech-math",
    );
    assert!(plan.engine_features.iter().any(|feature| feature == "bool"));
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "vectord")
    );
    assert!(plan.core_features.iter().all(|feature| feature != "bool"));
    assert!(
        plan.core_features
            .iter()
            .all(|feature| feature != "vectord")
    );
}

#[test]
fn dynamic_matrix_add_resolves_the_exact_dynamic_matrix_installer() {
    assert_owner_runtime_function(
        OwnerProfile::Standard,
        "dynamic-matrix-add-f64.mecb",
        "dynamic",
        "AddMDMD<f64>",
        "mech_math::__mech_native::install_add_mdmd_f64",
        "mech-math",
    );
}

#[test]
fn variadic_horzcat_resolves_the_exact_variadic_installer() {
    assert_owner_runtime_function(
        OwnerProfile::Standard,
        "variadic-horzcat-f64.mecb",
        "variadic",
        "matrix/horzcat",
        "mech_engine::__mech_native::install_value_horizontal_concatenation",
        "mech-engine",
    );
}

#[test]
fn cli_stdout_yields_a_hosted_plan() {
    let plan = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Plan,
        "cli",
        fixture_path("cli-stdout.mecb"),
        "native_cli_planning",
        false,
    )
    .plan;

    assert_eq!(plan.application_kind, NativeApplicationKind::Hosted);
    assert_eq!(plan.hosts.len(), 1);
    assert_eq!(plan.hosts[0].name, "cli");
    assert_eq!(plan.hosts[0].provider, "cli");
    assert_eq!(plan.hosts[0].package, "mech-terminal");
    assert_eq!(
        plan.hosts[0].factory_path,
        "mech_terminal::CliHostFactory::new"
    );
    assert_eq!(plan.run_grants.len(), 1);
    assert_eq!(plan.runtime_config.name, "native-generated-runtime");
    assert!(
        plan.runtime_features
            .iter()
            .any(|feature| feature == "string")
    );
}

#[test]
fn hosted_bytecode_without_runtime_config_fails_before_generation() {
    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request(CLI_STDOUT))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeRuntimeConfigMissing");
}

#[test]
fn unknown_runtime_ids_fail_before_generation() {
    let bytecode = runtime_nullary_bytecode(0x0123_4567_89ab_cdef);
    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request(&bytecode))
        .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
}

#[derive(Debug)]
struct PlanningFunction {
    _output: Ref<f64>,
}

impl MechFunctionImpl for PlanningFunction {
    fn solve_result(&self) -> MResult<()> {
        Ok(())
    }

    fn to_string(&self) -> String {
        "PlanningFunction".into()
    }
}

impl MechFunctionCompiler for PlanningFunction {
    fn compile(&self, _ctx: &mut dyn BytecodeCompilerContext) -> MResult<Register> {
        Ok(0)
    }
}

impl MechFunctionFactory for PlanningFunction {
    fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
        mech_core::ImplementationMemoryClass::NoAdditionalScratch
    }

    const SIGNATURE: RuntimeFunctionSignature =
        RuntimeFunctionSignature::nullary(<f64 as FunctionRuntimeType>::REPRESENTATION);

    fn new_invocation(invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
        let output = invocation.expect_nullary()?;
        Ok(Box::new(PlanningFunction {
            _output: output.try_ref()?,
        }))
    }
}

#[test]
fn known_runtime_ids_without_native_metadata_fail_before_generation() {
    const NAME: &str = "KnownButUnlinked";
    let mut catalog = FunctionCatalogBuilder::new();
    catalog
        .insert_runtime_factory::<PlanningFunction>(
            NAME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            RuntimeFamilyId::from_name(NAME),
        )
        .unwrap();
    let bytecode = runtime_nullary_bytecode(hash_str(NAME));

    let error = NativeApplicationBuilder::new(environment(Arc::new(catalog.build().unwrap())))
        .plan(&request(&bytecode))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeRuntimeFunctionLinkageMissing");
}

#[test]
fn malicious_runtime_type_mismatch_fails_before_native_analysis() {
    const NAME: &str = "LinkedPlanningFunction";
    let mut catalog = FunctionCatalogBuilder::new();
    catalog
        .insert_runtime_factory_with_linkage::<PlanningFunction>(
            NAME,
            RuntimeFunctionContract::no_matrix(RuntimeOutputAliasPolicy::DisallowInputAlias),
            NativeFunctionLinkage {
                package: "mech-test",
                crate_name: "mech_test",
                installer_path: "mech_test::__mech_native::install",
                cargo_features: vec!["native-link", "runtime"],
            },
            RuntimeFamilyId::from_name(NAME),
        )
        .unwrap();
    let bytecode = runtime_nullary_bytecode_with_constant(
        hash_str(NAME),
        EncodedConstant {
            runtime_type: RuntimeType::I8,
            alignment: 1,
            bytes: vec![7],
        },
    );

    let error = NativeApplicationBuilder::new(environment(Arc::new(catalog.build().unwrap())))
        .plan(&request(&bytecode))
        .unwrap_err();
    assert_eq!(error.kind_name(), "BytecodeRuntimeContractViolation");
    assert!(error.kind_message().contains(NAME));
}

#[test]
fn malicious_matrix_relations_and_aliases_fail_without_materializing_a_project() {
    struct MustNotInstantiate;

    impl MechFunctionFactory for MustNotInstantiate {
        fn implementation_memory_class() -> mech_core::ImplementationMemoryClass {
            mech_core::ImplementationMemoryClass::NoAdditionalScratch
        }

        const SIGNATURE: RuntimeFunctionSignature = RuntimeFunctionSignature::binary(
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
            FunctionValueRepresentation::AnyValue,
        );

        fn new_invocation(_invocation: FunctionInvocation) -> MResult<Box<dyn MechFunction>> {
            panic!("malformed matrix relations must fail before factory construction")
        }
    }
    let mut catalog = FunctionCatalogBuilder::new();
    for (name, installer_path, contract) in [
        (
            "AddMDMD<f64>",
            "mech_test::__mech_native::install_add_mdmd",
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "AddM2M2<f64>",
            "mech_test::__mech_native::install_add_m2m2",
            RuntimeFunctionContract::same_shape(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "MatMulMDMD<f64>",
            "mech_test::__mech_native::install_matmul_mdmd",
            RuntimeFunctionContract::matrix_product(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
        (
            "MatrixSolveMDVD<f64>",
            "mech_test::__mech_native::install_solve_mdvd",
            RuntimeFunctionContract::linear_solve(RuntimeOutputAliasPolicy::DisallowInputAlias),
        ),
    ] {
        catalog
            .insert_runtime_factory_with_linkage::<MustNotInstantiate>(
                name,
                contract,
                NativeFunctionLinkage {
                    package: "mech-test",
                    crate_name: "mech_test",
                    installer_path,
                    cargo_features: vec!["native-link", "runtime"],
                },
                RuntimeFamilyId::from_name(name),
            )
            .unwrap();
    }
    let temporary = tempfile::tempdir().unwrap();
    let output = temporary.path().join("must-not-exist");
    let builder = NativeApplicationBuilder::new(environment(Arc::new(catalog.build().unwrap())));
    let prior_plan = builder.plan(&request(LITERAL_F64)).unwrap();
    let prior_plan_snapshot = prior_plan.clone();

    let dynamic = |rows, cols| f64_matrix_constant(MatrixStorage::MatrixD, rows, cols);
    let vector = |rows| f64_matrix_constant(MatrixStorage::VectorD, rows, 1);
    let cases = [
        (
            "AddMDMD<f64>",
            vec![dynamic(2, 2), dynamic(2, 2), dynamic(3, 3)],
            (0, 1, 2),
        ),
        (
            "AddMDMD<f64>",
            vec![dynamic(3, 3), dynamic(2, 2), dynamic(2, 2)],
            (0, 1, 2),
        ),
        (
            "AddMDMD<f64>",
            vec![dynamic(2, 2), dynamic(2, 2)],
            (0, 0, 1),
        ),
        (
            "AddMDMD<f64>",
            vec![dynamic(2, 2), dynamic(2, 2)],
            (1, 0, 1),
        ),
        (
            "AddM2M2<f64>",
            vec![dynamic(2, 2), dynamic(2, 2)],
            (0, 0, 1),
        ),
        (
            "MatMulMDMD<f64>",
            vec![dynamic(2, 4), dynamic(2, 3), dynamic(2, 4)],
            (0, 1, 2),
        ),
        (
            "MatMulMDMD<f64>",
            vec![dynamic(3, 4), dynamic(2, 3), dynamic(3, 4)],
            (0, 1, 2),
        ),
        (
            "MatrixSolveMDVD<f64>",
            vec![vector(3), dynamic(2, 3), vector(3)],
            (0, 1, 2),
        ),
        (
            "MatrixSolveMDVD<f64>",
            vec![vector(2), dynamic(2, 2), vector(3)],
            (0, 1, 2),
        ),
    ];

    for (name, constants, (dst, lhs, rhs)) in cases {
        let mut malformed = request(&runtime_binary_bytecode(
            hash_str(name),
            constants,
            dst,
            lhs,
            rhs,
        ));
        malformed.output = output.clone();
        let error = builder.plan(&malformed).unwrap_err();
        assert_eq!(
            error.kind_name(),
            "BytecodeRuntimeContractViolation",
            "{name}"
        );
        assert!(error.kind_message().contains(name));
        assert!(!output.exists());
        assert_eq!(prior_plan, prior_plan_snapshot);
    }
}

#[test]
fn unknown_and_browser_providers_fail_before_generation() {
    for provider in ["untrusted", "browser"] {
        let mut request = request(CLI_STDOUT);
        request.runtime_config = Some(cli_runtime_config(provider, &["write"], &["line"]));

        let error = NativeApplicationBuilder::new(environment(empty_catalog()))
            .plan(&request)
            .unwrap_err();
        assert_eq!(error.kind_name(), "NativeHostProviderUnknown");
    }
}

#[test]
fn cli_rejects_an_explicit_unsupported_target_family() {
    let mut request = request(CLI_STDOUT);
    request.target = Some("thumbv7em-none-eabihf".to_owned());
    request.runtime_config = Some(cli_runtime_config("cli", &["write"], &["line"]));

    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request)
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeTargetUnsupported");
}

#[test]
fn registry_dependency_source_requires_an_exact_version() {
    let mut environment = environment(empty_catalog());
    environment.dependency_source = NativeDependencySource::Registry {
        version: "^0.3".to_owned(),
    };
    let error = NativeApplicationBuilder::new(environment)
        .plan(&request(LITERAL_F64))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeDependencyInvalid");
}

#[test]
fn registry_dependency_source_must_match_the_component_release() {
    let mut environment = environment(empty_catalog());
    environment.dependency_source = NativeDependencySource::Registry {
        version: "0.3.6".to_owned(),
    };
    let error = NativeApplicationBuilder::new(environment)
        .plan(&request(LITERAL_F64))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeComponentVersionMismatch");
}

#[test]
fn workspace_component_version_mismatch_blocks_generation() {
    let temporary = tempfile::tempdir().unwrap();
    let core = temporary.path().join("src/core");
    fs::create_dir_all(&core).unwrap();
    fs::write(
        core.join("Cargo.toml"),
        "[package]\nname = \"mech-core\"\nversion = \"0.0.0\"\n",
    )
    .unwrap();

    let mut environment = environment(empty_catalog());
    environment.dependency_source = NativeDependencySource::Workspace {
        root: temporary.path().to_path_buf(),
    };
    let error = NativeApplicationBuilder::new(environment)
        .plan(&request(LITERAL_F64))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeComponentVersionMismatch");
}

#[test]
fn missing_run_grants_fail_before_generation() {
    let mut request = request(CLI_STDOUT);
    let mut config = cli_runtime_config("cli", &["write"], &["line"]);
    config.run_grants.clear();
    request.runtime_config = Some(config);

    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request)
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeRunGrantMissing");
}

#[test]
fn bytecode_strings_cannot_select_cargo_packages_or_features() {
    const UNTRUSTED: &str = "attacker-selected-package";
    let bytecode = untrusted_string_bytecode(UNTRUSTED);
    let plan = plan(&bytecode);

    assert!(plan.packages.iter().all(|package| {
        package.package != UNTRUSTED
            && package.crate_name != UNTRUSTED
            && package
                .cargo_features
                .iter()
                .all(|feature| feature != UNTRUSTED)
    }));
    assert!(
        plan.core_features
            .iter()
            .chain(&plan.engine_features)
            .chain(&plan.runtime_features)
            .all(|feature| feature != UNTRUSTED)
    );
}

#[test]
fn unrelated_program_types_do_not_become_machine_features() {
    let temporary = tempfile::tempdir().unwrap();
    let bytecode = temporary.path().join("string-and-scalar-add.mecb");
    fs::write(&bytecode, string_and_scalar_add_bytecode()).unwrap();
    let plan = run_owner(
        OwnerProfile::Standard,
        RunnerAction::Plan,
        "synthetic-string-scalar",
        &bytecode,
        "native_synthetic_string_scalar",
        false,
    )
    .plan;
    assert!(plan.core_features.iter().any(|feature| feature == "string"));
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "string")
    );

    let math = plan
        .packages
        .iter()
        .find(|package| package.package == "mech-math")
        .unwrap();
    assert!(math.cargo_features.iter().any(|feature| feature == "f64"));
    assert!(
        math.cargo_features
            .iter()
            .all(|feature| feature != "string")
    );
}

#[test]
fn enum_inline_payloads_require_an_authoritative_complete_schema() {
    let error = enum_with_f64_payload_bytecode().unwrap_err();
    assert!(
        error
            .kind_message()
            .contains("authoritative complete enum schema"),
        "{error:?}"
    );
}

#[test]
fn equivalent_normalized_runtime_configs_produce_identical_plans() {
    let mut first = request(CLI_STDOUT);
    first.output = PathBuf::from("first-output");
    first.runtime_config = Some(cli_runtime_config(
        "cli",
        &["write", "read", "write"],
        &["text", "line", "line"],
    ));

    let mut second = request(CLI_STDOUT);
    second.output = PathBuf::from("second-output");
    second.runtime_config = Some(cli_runtime_config(
        "cli",
        &["read", "write"],
        &["line", "text"],
    ));

    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    assert_eq!(
        builder.plan(&first).unwrap(),
        builder.plan(&second).unwrap()
    );
}

#[test]
fn absolute_workspace_relocation_does_not_change_the_fingerprint() {
    let temporary = tempfile::tempdir().unwrap();
    let first = temporary.path().join("first-location");
    let second = temporary.path().join("different/second-location");
    write_fingerprint_fixture(&first);
    write_fingerprint_fixture(&second);

    let package =
        WorkspacePackage::new("mech-example", "mech_example", "packages/example").unwrap();
    let first_fingerprint = fingerprint_workspace(&first, std::slice::from_ref(&package)).unwrap();
    let second_fingerprint = fingerprint_workspace(&second, &[package]).unwrap();

    assert_eq!(first_fingerprint, second_fingerprint);
    assert_eq!(first_fingerprint.as_str().len(), 64);
}

fn runtime_nullary_bytecode(function: u64) -> Vec<u8> {
    runtime_nullary_bytecode_with_constant(
        function,
        EncodedConstant {
            runtime_type: RuntimeType::F64,
            alignment: 8,
            bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
        },
    )
}

fn f64_matrix_constant(storage: MatrixStorage, rows: u32, cols: u32) -> EncodedConstant {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&rows.to_le_bytes());
    bytes.extend_from_slice(&cols.to_le_bytes());
    for index in 0..rows.saturating_mul(cols) {
        bytes.extend_from_slice(&f64::from(index + 1).to_bits().to_le_bytes());
    }
    EncodedConstant {
        runtime_type: RuntimeType::Matrix {
            element: Box::new(RuntimeType::F64),
            storage,
            rows,
            cols,
        },
        alignment: 8,
        bytes,
    }
}

fn runtime_binary_bytecode(
    function: u64,
    constants: Vec<EncodedConstant>,
    dst: u32,
    lhs: u32,
    rhs: u32,
) -> Vec<u8> {
    let register_count = constants.len() as u32;
    let mut canonical_constants = Vec::new();
    let mut instructions = constants
        .into_iter()
        .enumerate()
        .map(|(register, constant)| {
            let constant_id = canonical_constants
                .iter()
                .position(|existing| existing == &constant)
                .unwrap_or_else(|| {
                    canonical_constants.push(constant);
                    canonical_constants.len() - 1
                }) as u32;
            BytecodeInstruction::ConstLoad {
                dst: register as u32,
                constant: constant_id,
            }
        })
        .collect::<Vec<_>>();
    instructions.push(BytecodeInstruction::RuntimeBinary {
        function,
        dst,
        lhs,
        rhs,
    });
    instructions.push(BytecodeInstruction::Return { src: dst });
    write_bytecode(&BytecodeProgram {
        register_count,
        constants: canonical_constants,
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions,
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn runtime_nullary_bytecode_with_constant(function: u64, output: EncodedConstant) -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![output],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::RuntimeNullary { function, dst: 0 },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn enum_with_f64_payload_bytecode() -> MResult<Vec<u8>> {
    let enum_name = "measurement";
    let variant_name = "reading";
    let mut bytes = 1_u32.to_le_bytes().to_vec();
    bytes.extend_from_slice(&hash_str(variant_name).to_le_bytes());
    bytes.extend_from_slice(&(variant_name.len() as u32).to_le_bytes());
    bytes.extend_from_slice(variant_name.as_bytes());
    bytes.push(1);
    let inline_f64 = (RuntimeTypeTag::F64 as u16).to_le_bytes();
    bytes.extend_from_slice(&(inline_f64.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&inline_f64);
    let payload = 42.5_f64.to_bits().to_le_bytes();
    bytes.extend_from_slice(&(payload.len() as u32).to_le_bytes());
    bytes.extend_from_slice(&payload);

    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![EncodedConstant {
            runtime_type: RuntimeType::Enum {
                id: hash_str(enum_name),
                name: enum_name.to_owned(),
            },
            alignment: 4,
            bytes,
        }],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
}

fn host_function_only_bytecode(name: &str) -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![EncodedConstant {
            runtime_type: RuntimeType::Empty,
            alignment: 1,
            bytes: Vec::new(),
        }],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::HostCall {
                requirement: 0,
                dst: 0,
                arguments: Vec::new(),
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: vec![ApplicationRequirement::HostFunction(
            ExecutionHostFunctionRequest {
                name: name.to_owned(),
            },
        )],
    })
    .unwrap()
}

fn untrusted_string_bytecode(value: &str) -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 1,
        constants: vec![EncodedConstant {
            runtime_type: RuntimeType::String,
            alignment: 1,
            bytes: value.as_bytes().to_vec(),
        }],
        symbols: BTreeMap::new(),
        mutable_symbols: BTreeSet::new(),
        instructions: vec![
            BytecodeInstruction::ConstLoad {
                dst: 0,
                constant: 0,
            },
            BytecodeInstruction::Return { src: 0 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn string_and_scalar_add_bytecode() -> Vec<u8> {
    write_bytecode(&BytecodeProgram {
        register_count: 4,
        constants: vec![
            EncodedConstant {
                runtime_type: RuntimeType::String,
                alignment: 1,
                bytes: b"unrelated".to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 1.0_f64.to_bits().to_le_bytes().to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 2.0_f64.to_bits().to_le_bytes().to_vec(),
            },
            EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 0.0_f64.to_bits().to_le_bytes().to_vec(),
            },
        ],
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
                constant: 2,
            },
            BytecodeInstruction::ConstLoad {
                dst: 3,
                constant: 3,
            },
            BytecodeInstruction::RuntimeBinary {
                function: hash_str("AddSS<f64>"),
                dst: 3,
                lhs: 1,
                rhs: 2,
            },
            BytecodeInstruction::Return { src: 3 },
        ],
        dictionary: BTreeMap::new(),
        requirements: Vec::new(),
    })
    .unwrap()
}

fn write_fingerprint_fixture(root: &std::path::Path) {
    let package = root.join("packages/example");
    fs::create_dir_all(package.join("src/nested")).unwrap();
    fs::create_dir_all(root.join("src/syntax")).unwrap();
    fs::write(
        root.join("Cargo.lock"),
        "# deterministic fixture lockfile\nversion = 4\n",
    )
    .unwrap();
    fs::write(
        package.join("Cargo.toml"),
        "[package]\nname = \"mech-example\"\nversion = \"0.3.5\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(
        root.join("src/syntax/Cargo.toml"),
        "[package]\nname = \"mech-syntax\"\nversion = \"0.3.5\"\nedition = \"2024\"\n",
    )
    .unwrap();
    fs::write(package.join("src/lib.rs"), "mod nested;\n").unwrap();
    fs::write(
        package.join("src/nested.rs"),
        "pub const VALUE: u64 = 42;\n",
    )
    .unwrap();
}
