#![cfg_attr(windows, feature(windows_process_extensions_main_thread_handle))]

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
    WorkspacePackage, fingerprint_workspace, render_catalog_source,
};
#[cfg(not(feature = "full-hosts"))]
use mech_build::{NativeHostLinkage, NativeTargetFamily};
#[cfg(not(feature = "full-hosts"))]
use mech_core::Value;
use mech_core::{
    ApplicationRequirement, BytecodeCompilerContext, BytecodeInstruction, BytecodeProgram,
    EncodedConstant, ExecutionHostFunctionRequest, FunctionCatalog, FunctionCatalogBuilder,
    FunctionInvocation, FunctionRuntimeType, FunctionValueOutput, FunctionValueRepresentation,
    MResult, MatrixStorage, MechFunction, MechFunctionCompiler, MechFunctionFactory,
    MechFunctionImpl, NativeFunctionLinkage, ParsedProgram, Register, RuntimeFamilyId,
    RuntimeFunctionContract, RuntimeFunctionSignature, RuntimeOutputAliasPolicy, RuntimeType,
    RuntimeTypeTag, hash_str, write_bytecode, write_bytecode_with_artifact,
};
use mech_engine::{
    ApplicationRequirementTable, ProgramArtifactDraft, encode_program_artifact_bytecode_v1,
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

#[test]
#[ignore = "explicit CI preparation stage; normal planning execution consumes the prepared profiles"]
fn prepare_planning_owner_runners() {
    isolated::prepare_owner_runners(
        &[OwnerProfile::Standard, OwnerProfile::Fixed],
        "planning-owner-preparation",
    )
    .unwrap_or_else(|error| panic!("{error}"));
}

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

fn assert_artifact_only_bytecode(bytecode: &[u8], expected_operations: &[&str]) {
    let parsed = ParsedProgram::from_bytes(bytecode).unwrap();
    assert_eq!(parsed.header.register_count, 0);
    assert_eq!(parsed.header.instruction_count, 0);
    assert!(parsed.types.is_empty());
    assert!(parsed.constants.is_empty());
    assert!(parsed.constant_blob.is_empty());
    assert!(parsed.symbols.is_empty());
    assert!(parsed.mutable_symbols.is_empty());
    assert!(
        parsed.instructions.is_empty(),
        "canonical artifact unexpectedly retained legacy instructions"
    );
    assert!(parsed.dictionary.is_empty());
    assert!(
        parsed
            .instructions
            .iter()
            .filter_map(BytecodeInstruction::runtime_function)
            .next()
            .is_none(),
        "canonical artifact unexpectedly retained legacy runtime IDs"
    );
    let artifact = mech_engine::decode_program_artifact_bytecode_v1(bytecode).unwrap();
    let operations = artifact
        .operation_references()
        .into_iter()
        .map(|operation| operation.canonical_name())
        .collect::<BTreeSet<_>>();
    let expected = expected_operations
        .iter()
        .map(|operation| (*operation).to_owned())
        .collect::<BTreeSet<_>>();
    assert_eq!(operations, expected);
}

fn assert_artifact_only_plan(
    plan: &NativeBuildPlan,
    expected_core_features: &[&str],
    expected_engine_features: &[&str],
    expected_runtime_features: &[&str],
) {
    assert_eq!(plan.application_kind, NativeApplicationKind::Engine);
    assert!(
        plan.runtime_functions.is_empty(),
        "canonical artifact must not reconstruct legacy runtime installers: {:#?}",
        plan.runtime_functions
    );
    assert!(plan.runtime_types.is_empty());
    assert_eq!(
        plan.packages
            .iter()
            .map(|package| package.package.as_str())
            .collect::<Vec<_>>(),
        ["mech-core", "mech-engine", "mech-runtime"]
    );
    assert_eq!(
        plan.core_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected_core_features
    );
    assert_eq!(
        plan.engine_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected_engine_features
    );
    assert_eq!(
        plan.runtime_features
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        expected_runtime_features
    );

    let catalog = render_catalog_source(plan).unwrap();
    assert!(catalog.contains("mech_engine::install_intrinsic_resident"));
    assert!(!catalog.contains("__mech_native::install_"));
}

fn assert_owner_resident_artifact(
    profile: OwnerProfile,
    fixture: &str,
    case: &str,
    expected_operations: &[&str],
    expected_core_features: &[&str],
    expected_engine_features: &[&str],
    expected_runtime_features: &[&str],
) -> NativeBuildPlan {
    let fixture = fixture_path(fixture);
    assert_artifact_only_bytecode(&fs::read(&fixture).unwrap(), expected_operations);
    let plan = run_owner(
        profile,
        RunnerAction::Plan,
        case,
        fixture,
        "native_planning",
        false,
    )
    .plan;
    assert_artifact_only_plan(
        &plan,
        expected_core_features,
        expected_engine_features,
        expected_runtime_features,
    );
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
fn canonical_source_native_plan_uses_only_resident_operation_authority() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("left := 1.0\nright := 2.0\nleft + right\n")
        .unwrap();
    assert_eq!(
        product
            .artifact()
            .operation_references()
            .into_iter()
            .map(|operation| operation.canonical_name())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["math/add".to_owned()])
    );
    let (_, bytecode, bindings, requirements, _) = product.into_native_parts();
    assert_artifact_only_bytecode(&bytecode, &["math/add"]);
    let mut request = request(&bytecode);
    request.instruction_type_bindings = Some(bindings);
    request.instruction_type_binding_requirements = Some(requirements);
    let plan = NativeApplicationBuilder::new(environment(catalog))
        .plan(&request)
        .unwrap();
    assert_artifact_only_plan(
        &plan,
        &["f64", "program"],
        &["f64", "runtime"],
        &["f64", "resident-routing", "runtime", "string"],
    );
}

fn canonical_constant_bytecode(
    schema: mech_core::SchemaBody,
    data: impl FnOnce(mech_core::SchemaId) -> mech_core::ValueDataDraft,
) -> Vec<u8> {
    use mech_core::snapshot::SnapshotValidationContext;
    use mech_core::{
        ConstantStoreBuilder, OperationContractTableBuilder, SchemaBody, SchemaDraft,
        SchemaTableBuilder, ValueDraft,
    };

    let mut schemas = SchemaTableBuilder::new();
    let handle = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: schema,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let index_handle = schemas
        .insert(
            SchemaDraft {
                dimension_parameters: Box::new([]),
                body: SchemaBody::Index,
            }
            .finalize()
            .unwrap(),
        )
        .unwrap();
    let build = schemas.finish().unwrap();
    let schema = build.resolve(handle).unwrap();
    let index_schema = build.resolve(index_handle).unwrap();
    let schemas = build.table;
    let value = ValueDraft {
        schema,
        shape_values: Box::new([]),
        data: data(index_schema),
    }
    .finalize(&SnapshotValidationContext::new(&schemas))
    .unwrap();
    let mut constants = ConstantStoreBuilder::new(&schemas);
    constants.insert(value).unwrap();
    let constants = constants.finish().unwrap().store;
    let artifact = ProgramArtifactDraft {
        schemas,
        constants,
        contracts: OperationContractTableBuilder::new().finish().unwrap().table,
        requirements: ApplicationRequirementTable::empty(),
        inputs: Box::new([]),
        slots: Box::new([]),
        nodes: Box::new([]),
        bindings: Box::new([]),
        outputs: Box::new([]),
        constraints: Box::new([]),
        compute_regions: Box::new([]),
    }
    .finalize()
    .unwrap();
    let bytecode = encode_program_artifact_bytecode_v1(&artifact).unwrap();
    assert_artifact_only_bytecode(&bytecode, &[]);
    bytecode
}

#[test]
fn canonical_native_plan_admits_u32_max_index_on_a_32_bit_target() {
    let bytecode = canonical_constant_bytecode(mech_core::SchemaBody::Index, |_| {
        mech_core::ValueDataDraft::Index(u64::from(u32::MAX))
    });
    let mut request = request(&bytecode);
    request.target = Some("i686-pc-windows-msvc".to_owned());
    let plan = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request)
        .unwrap();
    assert!(plan.runtime_types.is_empty());
    assert!(plan.runtime_functions.is_empty());
}

#[test]
#[cfg(target_pointer_width = "64")]
fn canonical_native_plan_checks_index_constants_against_the_target_pointer_width() {
    let value = u64::from(u32::MAX) + 1;
    let bytecode = canonical_constant_bytecode(mech_core::SchemaBody::Index, |_| {
        mech_core::ValueDataDraft::Index(value)
    });
    let mut request = request(&bytecode);
    request.target = Some("i686-pc-windows-msvc".to_owned());
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    let error = builder.plan(&request).unwrap_err();
    assert_eq!(error.kind_name(), "NativeBuildIndexConstantOutOfRange");
    assert!(error.display_message().contains(&value.to_string()));

    request.target = Some("x86_64-unknown-linux-gnu".to_owned());
    let plan = builder.plan(&request).unwrap();
    assert!(plan.runtime_types.is_empty());
    assert!(plan.runtime_functions.is_empty());
}

#[cfg(target_pointer_width = "64")]
#[cfg(target_pointer_width = "64")]
fn nested_canonical_index_cases(
    value: u64,
) -> Vec<(
    &'static str,
    mech_core::SchemaBody,
    mech_core::ValueDataDraft,
)> {
    use mech_core::snapshot::{
        EnumDraft, MapEntryDraft, NamedValueDraft, OptionDraft, TableColumnDraft,
    };
    use mech_core::{
        CardinalitySpec, DimensionExpr, EnumVariantSchema, NominalKey, SchemaBody, SchemaField,
        ValueDataDraft,
    };
    let extent = || CardinalitySpec::Exact(DimensionExpr::Constant(1));
    let tuple = || SchemaBody::Tuple(vec![SchemaBody::Index].into_boxed_slice());
    let tuple_data =
        || ValueDataDraft::Tuple(vec![ValueDataDraft::Index(value)].into_boxed_slice());
    vec![
        ("tuple", tuple(), tuple_data()),
        (
            "option",
            SchemaBody::Option(Box::new(tuple())),
            ValueDataDraft::Option(OptionDraft {
                present: true,
                value: Some(Box::new(tuple_data())),
            }),
        ),
        (
            "record",
            SchemaBody::Record(
                vec![SchemaField {
                    name: "index".into(),
                    schema: tuple(),
                }]
                .into_boxed_slice(),
            ),
            ValueDataDraft::Record(
                vec![NamedValueDraft {
                    name: "index".into(),
                    value: tuple_data(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "packed matrix",
            SchemaBody::Matrix {
                element: Box::new(SchemaBody::Index),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(vec![ValueDataDraft::Index(value)].into_boxed_slice()),
        ),
        (
            "nested matrix",
            SchemaBody::Matrix {
                element: Box::new(tuple()),
                dimensions: vec![DimensionExpr::Constant(1), DimensionExpr::Constant(1)]
                    .into_boxed_slice(),
            },
            ValueDataDraft::Matrix(vec![tuple_data()].into_boxed_slice()),
        ),
        (
            "packed table",
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "index".into(),
                    schema: SchemaBody::Index,
                }]
                .into_boxed_slice(),
                rows: extent(),
            },
            ValueDataDraft::Table(
                vec![TableColumnDraft {
                    name: "index".into(),
                    values: vec![ValueDataDraft::Index(value)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "nested table",
            SchemaBody::Table {
                columns: vec![SchemaField {
                    name: "index".into(),
                    schema: tuple(),
                }]
                .into_boxed_slice(),
                rows: extent(),
            },
            ValueDataDraft::Table(
                vec![TableColumnDraft {
                    name: "index".into(),
                    values: vec![tuple_data()].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "set",
            SchemaBody::Set {
                element: Box::new(tuple()),
                cardinality: extent(),
            },
            ValueDataDraft::Set(vec![tuple_data()].into_boxed_slice()),
        ),
        (
            "map key",
            SchemaBody::Map {
                key: Box::new(tuple()),
                value: Box::new(SchemaBody::Bool),
                cardinality: extent(),
            },
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![tuple_data(), ValueDataDraft::Bool(true)].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "map value",
            SchemaBody::Map {
                key: Box::new(SchemaBody::Bool),
                value: Box::new(tuple()),
                cardinality: extent(),
            },
            ValueDataDraft::Map(
                vec![MapEntryDraft {
                    items: vec![ValueDataDraft::Bool(true), tuple_data()].into_boxed_slice(),
                }]
                .into_boxed_slice(),
            ),
        ),
        (
            "enum payload",
            SchemaBody::Enum {
                key: NominalKey::from_bytes([23; 32]),
                variants: vec![EnumVariantSchema {
                    name: "index".into(),
                    payload: Some(tuple()),
                }]
                .into_boxed_slice(),
            },
            ValueDataDraft::Enum(EnumDraft {
                ordinal: 0,
                payload: Some(Box::new(tuple_data())),
            }),
        ),
    ]
}

#[test]
#[cfg(target_pointer_width = "64")]
fn canonical_native_plan_checks_index_constants_in_supported_nested_aggregates() {
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    for value in [u64::from(u32::MAX), u64::from(u32::MAX) + 1] {
        for (name, schema, data) in nested_canonical_index_cases(value) {
            let bytecode = canonical_constant_bytecode(schema, |_| data);
            let mut request = request(&bytecode);
            request.target = Some("i686-pc-windows-msvc".to_owned());
            let result = builder.plan(&request);
            if value == u64::from(u32::MAX) {
                assert!(result.is_ok(), "{name}: {result:?}");
            } else {
                assert_eq!(
                    result.unwrap_err().kind_name(),
                    "NativeBuildIndexConstantOutOfRange",
                    "{name}",
                );
            }
            request.target = Some("x86_64-unknown-linux-gnu".to_owned());
            assert!(builder.plan(&request).is_ok(), "{name}");
        }
    }
}

#[test]
#[cfg(target_pointer_width = "64")]
fn canonical_native_plan_checks_index_constants_inside_dynamic_values() {
    use mech_core::{SchemaBody, ValueDataDraft, ValueDraft};
    let bytecode = canonical_constant_bytecode(SchemaBody::Dynamic, |schema| {
        ValueDataDraft::Dynamic(Some(Box::new(ValueDraft {
            schema,
            shape_values: Box::new([]),
            data: ValueDataDraft::Index(u64::from(u32::MAX) + 1),
        })))
    });
    let mut request = request(&bytecode);
    request.target = Some("i686-pc-windows-msvc".to_owned());
    let builder = NativeApplicationBuilder::new(environment(empty_catalog()));
    assert_eq!(
        builder.plan(&request).unwrap_err().kind_name(),
        "NativeBuildIndexConstantOutOfRange",
    );
    request.target = Some("x86_64-unknown-linux-gnu".to_owned());
    builder.plan(&request).unwrap();
}

#[test]
fn canonical_native_plan_rejects_parallel_legacy_program_authority() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("value := 1.0\nvalue\n")
        .unwrap();
    let (_, bytecode, _, _, _) = product.into_native_parts();
    let parsed = ParsedProgram::from_bytes(&bytecode).unwrap();
    let hybrid = write_bytecode_with_artifact(
        &BytecodeProgram {
            register_count: 1,
            constants: vec![EncodedConstant {
                runtime_type: RuntimeType::F64,
                alignment: 8,
                bytes: 99.0_f64.to_bits().to_le_bytes().to_vec(),
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
            requirements: parsed.requirements,
        },
        &parsed.artifact,
    )
    .unwrap();

    let error = NativeApplicationBuilder::new(environment(catalog))
        .plan(&request(&hybrid))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeProgramArtifactInvalid");
    assert!(
        error
            .display_message()
            .contains("parallel legacy execution authority")
    );
}

#[test]
fn canonical_native_plan_rejects_unreferenced_artifact_requirement_authority() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("value := 1.0\nvalue\n")
        .unwrap();
    let artifact = product.artifact();
    let augmented = ProgramArtifactDraft {
        schemas: artifact.schemas().clone(),
        constants: artifact.constants().clone(),
        contracts: artifact.contracts().clone(),
        requirements: ApplicationRequirementTable::from_canonical_entries(vec![
            ApplicationRequirement::HostFunction(ExecutionHostFunctionRequest {
                name: "unreferenced/host-function".to_owned(),
            }),
        ])
        .unwrap(),
        inputs: artifact.inputs().to_vec().into_boxed_slice(),
        slots: artifact.slots().to_vec().into_boxed_slice(),
        nodes: artifact.nodes().to_vec().into_boxed_slice(),
        bindings: artifact.bindings().to_vec().into_boxed_slice(),
        outputs: artifact.outputs().to_vec().into_boxed_slice(),
        constraints: artifact.constraints().to_vec().into_boxed_slice(),
        compute_regions: artifact.compute_regions().to_vec().into_boxed_slice(),
    }
    .finalize()
    .unwrap();
    let bytecode = encode_program_artifact_bytecode_v1(&augmented).unwrap();

    let error = NativeApplicationBuilder::new(environment(catalog))
        .plan(&request(&bytecode))
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeProgramArtifactInvalid");
    assert!(
        error
            .display_message()
            .contains("is not owned by an artifact operation node")
    );
}

#[test]
fn canonical_native_plan_retains_exact_resident_operation_identity() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("left := 1.0\nright := 2.0\nleft < right\n")
        .unwrap();
    assert_eq!(
        product
            .artifact()
            .operation_references()
            .into_iter()
            .map(|operation| operation.canonical_name())
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["compare/lt".to_owned()])
    );
    let (_, bytecode, bindings, requirements, _) = product.into_native_parts();
    assert_artifact_only_bytecode(&bytecode, &["compare/lt"]);
    let mut request = request(&bytecode);
    request.instruction_type_bindings = Some(bindings);
    request.instruction_type_binding_requirements = Some(requirements);
    let plan = NativeApplicationBuilder::new(environment(catalog))
        .plan(&request)
        .unwrap();
    assert_artifact_only_plan(
        &plan,
        &["bool", "f64", "program"],
        &["bool", "f64", "runtime"],
        &["bool", "f64", "resident-routing", "runtime", "string"],
    );
}

#[test]
fn canonical_native_plan_fails_closed_without_its_resident_operation() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(catalog)
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("left := 1.0\nright := 2.0\nleft + right\n")
        .unwrap();
    let (_, bytecode, bindings, requirements, _) = product.into_native_parts();
    assert_artifact_only_bytecode(&bytecode, &["math/add"]);
    let mut request = request(&bytecode);
    request.instruction_type_bindings = Some(bindings);
    request.instruction_type_binding_requirements = Some(requirements);

    let error = NativeApplicationBuilder::new(environment(empty_catalog()))
        .plan(&request)
        .unwrap_err();
    assert_eq!(error.kind_name(), "NativeProgramArtifactInvalid");
    assert!(
        error
            .display_message()
            .contains("resident activation failed")
    );
}

#[test]
fn canonical_literal_aggregate_contributes_native_type_features() {
    let catalog = mech_stdlib::source_native_plan_catalog();
    let mut compiler = mech_runtime::RuntimeBuilder::new()
        .function_catalog(Arc::clone(&catalog))
        .build_compiler()
        .unwrap();
    let product = compiler
        .compile_canonical_source("value := (1.0, true)\nvalue\n")
        .unwrap();
    let (_, bytecode, bindings, requirements, _) = product.into_native_parts();
    let mut request = request(&bytecode);
    request.instruction_type_bindings = Some(bindings);
    request.instruction_type_binding_requirements = Some(requirements);
    let plan = NativeApplicationBuilder::new(environment(catalog))
        .plan(&request)
        .unwrap();
    for feature in ["bool", "f64", "tuple"] {
        assert!(plan.core_features.iter().any(|actual| actual == feature));
        assert!(plan.engine_features.iter().any(|actual| actual == feature));
        assert!(plan.runtime_features.iter().any(|actual| actual == feature));
    }
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
fn scalar_add_uses_the_exact_resident_artifact_closure() {
    assert_owner_resident_artifact(
        OwnerProfile::Standard,
        "scalar-add-f64.mecb",
        "scalar",
        &["math/add"],
        &["f64", "program"],
        &["f64", "runtime"],
        &["f64", "resident-routing", "runtime", "string"],
    );
}

#[test]
fn fixed_profile_matrix_add_uses_the_exact_resident_artifact_closure() {
    let plan = assert_owner_resident_artifact(
        OwnerProfile::Fixed,
        "fixed-matrix-add-f64.mecb",
        "fixed",
        &["math/add"],
        &["f64", "matrix2", "program"],
        &["bool", "f64", "matrix2", "runtime", "vector2"],
        &["f64", "matrix2", "resident-routing", "runtime", "string"],
    );
    assert!(plan.engine_features.iter().any(|feature| feature == "bool"));
    assert!(
        plan.engine_features
            .iter()
            .any(|feature| feature == "vector2")
    );
    assert!(plan.core_features.iter().all(|feature| feature != "bool"));
    assert!(
        plan.core_features
            .iter()
            .all(|feature| feature != "vector2")
    );
}

#[test]
fn dynamic_matrix_add_uses_the_exact_resident_artifact_closure() {
    assert_owner_resident_artifact(
        OwnerProfile::Standard,
        "dynamic-matrix-add-f64.mecb",
        "dynamic",
        &["math/add"],
        &["f64", "matrixd", "program"],
        &["f64", "matrixd", "runtime"],
        &["f64", "matrixd", "resident-routing", "runtime", "string"],
    );
}

#[test]
fn variadic_horzcat_uses_the_exact_resident_artifact_closure() {
    assert_owner_resident_artifact(
        OwnerProfile::Standard,
        "variadic-horzcat-f64.mecb",
        "variadic",
        &[],
        &["f64", "program", "row_vectord"],
        &["bool", "f64", "row_vectord", "runtime", "vectord"],
        &[
            "f64",
            "resident-routing",
            "row_vectord",
            "runtime",
            "string",
        ],
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
    _output: FunctionValueOutput,
}

impl MechFunctionImpl for PlanningFunction {
    fn solve_managed(
        &self,
        _frame: &mut mech_core::KernelMemoryFrame<'_>,
        _services: &mut dyn mech_core::MechExecutionServices,
    ) -> MResult<mech_core::ReactiveSolveStatus> {
        (|| -> MResult<()> { Ok(()) })()?;
        Ok(mech_core::ReactiveSolveStatus::Changed)
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
            _output: output.value(),
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
