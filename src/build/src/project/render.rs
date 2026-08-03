use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::PathBuf;

use mech_core::{MResult, ResourceDelivery};
use mech_runtime::{ConfigValue, LogLevel, RuntimeConfig};

use crate::analysis::requirements::normalize_runtime_config;
use crate::error::{NativeBuildErrorKind, native_build_error};
use crate::plan::{
    NATIVE_BUILD_PLAN_SCHEMA, NativeApplicationKind, NativeBuildPlan, NativeBuildRequest,
    NativeRuntimeConfig, PlannedApplicationRequirement, PlannedDependencySource,
    PlannedPackageSource, compute_plan_sha256, sha256_hex,
};

use super::{
    GeneratedDependency, GeneratedNativeProject, GeneratedSourceSet, NativeProjectManifest,
    render_native_project_manifest, rust_string_literal, validate_project_installer_path,
};

const FORBIDDEN_GENERATED_PACKAGES: &[&str] =
    &["mech-stdlib", "mech-syntax", "mech-bytecode", "mech-build"];

/// Convert the packages selected by planning into exact generated-Cargo
/// dependencies. The conversion is deliberately closed over trusted plan data;
/// bytecode strings are never consulted here.
pub fn generated_dependencies_from_plan(
    plan: &NativeBuildPlan,
) -> MResult<Vec<GeneratedDependency>> {
    if !plan
        .packages
        .iter()
        .any(|package| package.package == "mech-core")
        || !plan
            .packages
            .iter()
            .any(|package| package.package == "mech-engine")
    {
        return project_invalid("every generated application requires mech-core and mech-engine");
    }
    let has_runtime = plan
        .packages
        .iter()
        .any(|package| package.package == "mech-runtime");
    match plan.application_kind {
        NativeApplicationKind::Engine if has_runtime => {
            return project_invalid("an engine application may not depend on mech-runtime");
        }
        NativeApplicationKind::Hosted if !has_runtime => {
            return project_invalid("a hosted application requires mech-runtime");
        }
        _ => {}
    }

    let mut dependencies = Vec::with_capacity(plan.packages.len());
    for package in &plan.packages {
        if FORBIDDEN_GENERATED_PACKAGES.contains(&package.package.as_str()) {
            return project_invalid(format!(
                "generated applications may not depend on `{}`",
                package.package
            ));
        }
        let dependency = match &package.source {
            PlannedPackageSource::Registry { version } => GeneratedDependency::registry(
                &package.package,
                &package.crate_name,
                version,
                package.cargo_features.iter().cloned(),
            )?,
            PlannedPackageSource::Workspace { path } => GeneratedDependency::workspace(
                &package.package,
                &package.crate_name,
                path,
                package.cargo_features.iter().cloned(),
            )?,
        };
        dependencies.push(dependency);
    }
    dependencies.sort_by(|left, right| {
        left.crate_name
            .cmp(&right.crate_name)
            .then_with(|| left.package.cmp(&right.package))
    });
    Ok(dependencies)
}

/// Validate that a request and a public plan still describe the same immutable
/// native application before any project files are rendered.
pub fn validate_generation_identity(
    request: &NativeBuildRequest,
    plan: &NativeBuildPlan,
) -> MResult<()> {
    if plan.schema != NATIVE_BUILD_PLAN_SCHEMA {
        return project_invalid(format!(
            "unsupported native build plan schema `{}`",
            plan.schema
        ));
    }
    if sha256_hex(&request.bytecode) != plan.bytecode_sha256 {
        return project_invalid("request bytecode does not match the native build plan");
    }
    if request.binary_name != plan.binary_name {
        return project_invalid("request binary name does not match the native build plan");
    }
    if request.target != plan.target {
        return project_invalid("request target does not match the native build plan");
    }
    if request.profile != plan.profile {
        return project_invalid("request profile does not match the native build plan");
    }
    if compute_plan_sha256(plan)? != plan.plan_sha256 {
        return project_invalid("native build plan digest does not match its contents");
    }

    let inferred_kind =
        if !plan.application_requirements.is_empty() || request.runtime_config.is_some() {
            NativeApplicationKind::Hosted
        } else {
            NativeApplicationKind::Engine
        };
    if inferred_kind != plan.application_kind {
        return project_invalid("native build plan application kind contradicts its requirements");
    }
    validate_dependency_source_consistency(plan)?;
    validate_runtime_config_implications(plan, request.runtime_config.as_ref())
}

fn validate_dependency_source_consistency(plan: &NativeBuildPlan) -> MResult<()> {
    match &plan.dependency_source {
        PlannedDependencySource::Registry { version } => {
            if plan.workspace_fingerprint.is_some() {
                return project_invalid("a registry plan may not carry a workspace fingerprint");
            }
            if plan.packages.iter().any(|package| {
                !matches!(
                    &package.source,
                    PlannedPackageSource::Registry {
                        version: package_version
                    } if package_version == version
                )
            }) {
                return project_invalid(
                    "registry plan packages must use the plan's exact registry version",
                );
            }
        }
        PlannedDependencySource::Workspace => {
            if plan.workspace_fingerprint.is_none() {
                return project_invalid("a workspace plan requires a workspace fingerprint");
            }
            if plan
                .packages
                .iter()
                .any(|package| !matches!(package.source, PlannedPackageSource::Workspace { .. }))
            {
                return project_invalid(
                    "workspace plan packages must use workspace-relative paths",
                );
            }
        }
    }
    Ok(())
}

fn validate_runtime_config_implications(
    plan: &NativeBuildPlan,
    runtime_config: Option<&NativeRuntimeConfig>,
) -> MResult<()> {
    // Rendering is public independently of planning, so enforce the same
    // addressability rule here even for host-free and host-function-only plans.
    let normalized_config = runtime_config
        .map(normalize_runtime_config)
        .transpose()?
        .unwrap_or(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        });
    if normalized_config.runtime != plan.runtime_config {
        return project_invalid("request runtime settings do not match the normalized build plan");
    }
    if runtime_config.is_some() && plan.application_kind != NativeApplicationKind::Hosted {
        return project_invalid(
            "a request with runtime configuration requires a hosted native plan",
        );
    }
    let resources = plan
        .application_requirements
        .iter()
        .filter_map(|requirement| match requirement {
            PlannedApplicationRequirement::Resource { delivery, .. } => Some(*delivery),
            PlannedApplicationRequirement::HostFunction { .. } => None,
        })
        .collect::<Vec<_>>();
    if plan.live != resources.contains(&ResourceDelivery::Live) {
        return project_invalid(
            "native build plan live mode contradicts its resource requirements",
        );
    }

    if resources.is_empty() {
        if !plan.hosts.is_empty() || !plan.run_grants.is_empty() {
            return project_invalid(
                "a plan without resource requirements may not select hosts or run grants",
            );
        }
        if !normalized_config.hosts.is_empty() || !normalized_config.run_grants.is_empty() {
            return project_invalid(
                "a request without resource requirements may not configure hosts or run grants",
            );
        }
        return Ok(());
    }

    if runtime_config.is_none() {
        return Err(project_error(
            "resource project generation requires runtime config",
        ));
    }
    if normalized_config.run_grants != plan.run_grants {
        return project_invalid("request run grants do not match the normalized build plan");
    }
    if normalized_config.hosts.len() != plan.hosts.len()
        || normalized_config
            .hosts
            .iter()
            .zip(&plan.hosts)
            .any(|(configured, planned)| {
                configured.name != planned.name
                    || configured.provider != planned.provider
                    || configured.settings != planned.settings
            })
    {
        return project_invalid("request hosts do not match the normalized build plan");
    }
    Ok(())
}

/// Render the exact function-catalog reconstruction. Runtime factory installers
/// are ordered by numeric runtime ID. Host functions belong to RuntimeBuilder
/// and are installed by `src/runtime.rs`.
pub fn render_catalog_source(plan: &NativeBuildPlan) -> MResult<String> {
    let mut runtime_functions = plan.runtime_functions.clone();
    runtime_functions.sort_by_key(|function| function.runtime_id);
    if runtime_functions
        .windows(2)
        .any(|pair| pair[0].runtime_id == pair[1].runtime_id)
    {
        return project_invalid("native build plan contains a duplicate runtime function ID");
    }

    let mut installers = Vec::new();
    for function in runtime_functions {
        validate_project_installer_path(&function.installer_path)?;
        installers.push(function.installer_path);
    }

    let mut seen = BTreeSet::new();
    if let Some(duplicate) = installers
        .iter()
        .find(|installer| !seen.insert(installer.as_str()))
    {
        return project_invalid(format!(
            "native build plan repeats installer path `{duplicate}`"
        ));
    }

    let mut source = String::from(
        "use std::sync::Arc;\n\nuse mech_core::{\n    FunctionCatalog,\n    FunctionCatalogBuilder,\n    MResult,\n};\n\npub fn function_catalog() -> MResult<Arc<FunctionCatalog>> {\n    let mut builder = FunctionCatalogBuilder::new();\n",
    );
    for installer in installers {
        write!(
            &mut source,
            "\n    {installer}(\n        &mut builder,\n    )?;\n"
        )
        .expect("writing to String cannot fail");
    }
    source.push_str("\n    Ok(Arc::new(builder.build()?))\n}\n");
    Ok(source)
}

/// Render the exact engine-only entry point from the Phase 1 contract.
pub fn render_engine_main_source() -> String {
    String::from(
        r#"mod catalog;

use mech_core::{MResult, Value};
use mech_engine::{
    MechProgram,
    MechProgramConfig,
};

static PROGRAM: &[u8] =
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program.mecb"
    ));

fn run() -> MResult<()> {
    let catalog = catalog::function_catalog()?;

    let mut program =
        MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            catalog,
        );

    let value = program.run_bytecode(PROGRAM)?;

    if !matches!(value, Value::Empty) {
        println!("{value}");
    }

    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error.display_message());
        std::process::exit(1);
    }
}
"#,
    )
}

/// Render the exact hosted entry point from the Phase 1 contract.
pub fn render_hosted_main_source() -> String {
    String::from(
        r#"mod catalog;
mod runtime;

static PROGRAM: &[u8] =
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program.mecb"
    ));

fn run() -> mech_core::MResult<()> {
    let catalog = catalog::function_catalog()?;
    let mut runtime =
        runtime::runtime_builder(catalog)?.build()?;

    let mut context = runtime.runtime_context()?;

    let value = runtime.install_bytecode_with_context(
        &mut context,
        PROGRAM,
    )?;

    if !value.is_empty() {
        println!("{}", value.into_value());
    }

    runtime.shutdown()
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{}", error.display_message());
        std::process::exit(1);
    }
}
"#,
    )
}

/// Render direct Rust constructors for the trusted runtime configuration,
/// selected host instances, factories, and run grants.
pub fn render_runtime_source(
    plan: &NativeBuildPlan,
    runtime_config: Option<&NativeRuntimeConfig>,
) -> MResult<String> {
    validate_runtime_config_implications(plan, runtime_config)?;
    if plan.application_kind == NativeApplicationKind::Engine {
        return Ok(String::from(
            "// Engine-only native applications do not construct mech-runtime.\n",
        ));
    }
    let runtime = &plan.runtime_config;

    let mut factories = BTreeMap::<&str, &str>::new();
    for host in &plan.hosts {
        validate_project_installer_path(&host.factory_path)?;
        match factories.insert(&host.provider, &host.factory_path) {
            Some(existing) if existing != host.factory_path => {
                return project_invalid(format!(
                    "provider `{}` has conflicting factory paths",
                    host.provider
                ));
            }
            _ => {}
        }
    }

    let mut source = String::from(
        "use std::{collections::BTreeMap, sync::Arc};\n\nuse mech_core::{FunctionCatalog, MResult};\nuse mech_runtime::{\n    ConfigValue, DiagnosticsConfig, HostInstanceConfig, LogLevel,\n    RunResourceGrantConfig, RuntimeBuilder, RuntimeConfig, RuntimeLimits,\n};\n\npub fn runtime_builder(\n    catalog: Arc<FunctionCatalog>,\n) -> MResult<RuntimeBuilder> {\n",
    );
    write!(
        &mut source,
        "    let config = {};\n\n    let mut builder = RuntimeBuilder::new()\n        .function_catalog(catalog)\n        .config(config);\n",
        render_runtime_config(runtime)
    )
    .expect("writing to String cannot fail");

    let mut host_functions = plan
        .application_requirements
        .iter()
        .filter_map(|requirement| match requirement {
            PlannedApplicationRequirement::HostFunction {
                name,
                installer_path,
                ..
            } => Some((name, installer_path)),
            PlannedApplicationRequirement::Resource { .. } => None,
        })
        .collect::<Vec<_>>();
    host_functions.sort();
    for (_, installer_path) in host_functions {
        validate_project_installer_path(installer_path)?;
        write!(&mut source, "\n    builder = {installer_path}(builder)?;\n")
            .expect("writing to String cannot fail");
    }

    for factory_path in factories.values() {
        write!(
            &mut source,
            "\n    builder = builder.host_factory(\n        Box::new(\n            {factory_path}()?,\n        ),\n    )?;\n"
        )
        .expect("writing to String cannot fail");
    }
    for host in &plan.hosts {
        write!(
            &mut source,
            "\n    builder = builder.host_instance(HostInstanceConfig {{\n        name: {}.to_string(),\n        provider: {}.to_string(),\n        settings: {},\n    }});\n",
            rust_string_literal(&host.name),
            rust_string_literal(&host.provider),
            render_config_value(&host.settings)
        )
        .expect("writing to String cannot fail");
    }
    for grant in &plan.run_grants {
        write!(
            &mut source,
            "\n    builder = builder.run_resource_grant(RunResourceGrantConfig {{\n        target: {}.to_string(),\n        operations: {},\n        paths: {},\n    }});\n",
            rust_string_literal(&grant.target),
            render_string_vec(&grant.operations),
            render_string_vec(&grant.paths)
        )
        .expect("writing to String cannot fail");
    }
    source.push_str("\n    Ok(builder)\n}\n");
    Ok(source)
}

/// Render the complete deterministic Rust source set. `src/runtime.rs` is
/// present for both application kinds to preserve the frozen project layout.
pub fn render_project_sources(
    plan: &NativeBuildPlan,
    runtime_config: Option<&NativeRuntimeConfig>,
) -> MResult<GeneratedSourceSet> {
    let mut sources = GeneratedSourceSet::new();
    sources.insert("src/catalog.rs", render_catalog_source(plan)?)?;
    sources.insert(
        "src/main.rs",
        match plan.application_kind {
            NativeApplicationKind::Engine => render_engine_main_source(),
            NativeApplicationKind::Hosted => render_hosted_main_source(),
        },
    )?;
    sources.insert(
        "src/runtime.rs",
        render_runtime_source(plan, runtime_config)?,
    )?;
    Ok(sources)
}

/// Render all deterministic generated-project payloads in memory. The caller
/// owns filesystem materialization and Cargo execution.
pub fn render_generated_native_project(
    root: impl Into<PathBuf>,
    request: &NativeBuildRequest,
    plan: &NativeBuildPlan,
) -> MResult<GeneratedNativeProject> {
    validate_generation_identity(request, plan)?;
    let dependencies = generated_dependencies_from_plan(plan)?;
    let manifest = NativeProjectManifest::new(
        plan.binary_name.clone(),
        plan.binary_name.clone(),
        dependencies,
    )?;
    let cargo_manifest = render_native_project_manifest(&manifest)?;
    let sources = render_project_sources(plan, request.runtime_config.as_ref())?;
    let mut build_plan_json = serde_json::to_string_pretty(plan).map_err(|error| {
        project_error(format!("failed to serialize native build plan: {error}"))
    })?;
    build_plan_json.push('\n');
    Ok(GeneratedNativeProject::new(
        root,
        cargo_manifest,
        build_plan_json,
        request.bytecode.clone(),
        sources,
    ))
}

fn render_runtime_config(config: &RuntimeConfig) -> String {
    format!(
        "RuntimeConfig {{\n        name: {}.to_string(),\n        limits: RuntimeLimits {{\n            max_steps_per_turn: {},\n            max_turn_duration_ms: {},\n            max_memory_bytes: {},\n            max_tasks: {},\n            max_actors: {},\n            max_actor_mailbox_len: {},\n            max_source_bytes: {},\n            max_in_memory_events: {},\n        }},\n        diagnostics: DiagnosticsConfig {{\n            trace_enabled: {},\n            profile_enabled: {},\n            debug_enabled: {},\n            log_level: LogLevel::{},\n        }},\n    }}",
        rust_string_literal(&config.name),
        render_option_u64(config.limits.max_steps_per_turn),
        render_option_u64(config.limits.max_turn_duration_ms),
        render_option_u64(config.limits.max_memory_bytes),
        render_option_u64(config.limits.max_tasks),
        render_option_u64(config.limits.max_actors),
        render_option_u64(config.limits.max_actor_mailbox_len),
        render_option_u64(config.limits.max_source_bytes),
        render_option_u64(config.limits.max_in_memory_events),
        config.diagnostics.trace_enabled,
        config.diagnostics.profile_enabled,
        config.diagnostics.debug_enabled,
        render_log_level(&config.diagnostics.log_level),
    )
}

fn render_option_u64(value: Option<u64>) -> String {
    value.map_or_else(|| "None".to_owned(), |value| format!("Some({value}u64)"))
}

fn render_log_level(level: &LogLevel) -> &'static str {
    match level {
        LogLevel::Error => "Error",
        LogLevel::Warn => "Warn",
        LogLevel::Info => "Info",
        LogLevel::Debug => "Debug",
        LogLevel::Trace => "Trace",
    }
}

fn render_config_value(value: &ConfigValue) -> String {
    match value {
        ConfigValue::Null => "ConfigValue::Null".to_owned(),
        ConfigValue::Bool(value) => format!("ConfigValue::Bool({value})"),
        ConfigValue::Integer(value) => format!("ConfigValue::Integer({value}i64)"),
        ConfigValue::Float(value) => {
            format!("ConfigValue::Float(f64::from_bits({}u64))", value.to_bits())
        }
        ConfigValue::String(value) => {
            format!(
                "ConfigValue::String({}.to_string())",
                rust_string_literal(value)
            )
        }
        ConfigValue::List(values) => format!(
            "ConfigValue::List(vec![{}])",
            values
                .iter()
                .map(render_config_value)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        ConfigValue::Map(values) if values.is_empty() => {
            "ConfigValue::Map(BTreeMap::new())".to_owned()
        }
        ConfigValue::Map(values) => format!(
            "ConfigValue::Map(BTreeMap::from([{}]))",
            values
                .iter()
                .map(|(key, value)| format!(
                    "({}.to_string(), {})",
                    rust_string_literal(key),
                    render_config_value(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn render_string_vec(values: &[String]) -> String {
    format!(
        "vec![{}]",
        values
            .iter()
            .map(|value| format!("{}.to_string()", rust_string_literal(value)))
            .collect::<Vec<_>>()
            .join(", ")
    )
}

fn project_error(reason: impl Into<String>) -> mech_core::MechError {
    native_build_error(
        NativeBuildErrorKind::NativeProjectInvalid {
            reason: reason.into(),
        },
        None,
    )
}

fn project_invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(project_error(reason))
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use mech_core::{ResourceIntent, RuntimeType};
    use mech_runtime::{ConfigValue, HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

    use crate::plan::{
        NativeBuildProfile, NativeEmit, PlannedHostInstance, PlannedPackage,
        PlannedRuntimeFunction, refresh_plan_sha256,
    };

    use super::*;

    fn base_plan(kind: NativeApplicationKind) -> NativeBuildPlan {
        let mut packages = vec![
            PlannedPackage {
                package: "mech-core".into(),
                crate_name: "mech_core".into(),
                source: PlannedPackageSource::Registry {
                    version: "0.3.5".into(),
                },
                cargo_features: vec!["f64".into(), "program".into()],
            },
            PlannedPackage {
                package: "mech-engine".into(),
                crate_name: "mech_engine".into(),
                source: PlannedPackageSource::Registry {
                    version: "0.3.5".into(),
                },
                cargo_features: vec!["f64".into(), "runtime".into()],
            },
        ];
        if kind == NativeApplicationKind::Hosted {
            packages.push(PlannedPackage {
                package: "mech-runtime".into(),
                crate_name: "mech_runtime".into(),
                source: PlannedPackageSource::Registry {
                    version: "0.3.5".into(),
                },
                cargo_features: vec!["runtime".into(), "string".into()],
            });
        }
        NativeBuildPlan {
            schema: NATIVE_BUILD_PLAN_SCHEMA.into(),
            bytecode_version: 1,
            mech_version: "0.3.5".into(),
            application_kind: kind,
            runtime_config: RuntimeConfig::default(),
            bytecode_sha256: sha256_hex(b"bytecode"),
            plan_sha256: String::new(),
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "phase1-app".into(),
            runtime_functions: Vec::new(),
            runtime_types: vec![RuntimeType::F64],
            application_requirements: Vec::new(),
            packages,
            core_features: vec!["f64".into(), "program".into()],
            engine_features: vec!["f64".into(), "runtime".into()],
            runtime_features: if kind == NativeApplicationKind::Hosted {
                vec!["runtime".into(), "string".into()]
            } else {
                Vec::new()
            },
            hosts: Vec::new(),
            run_grants: Vec::new(),
            live: false,
            dependency_source: PlannedDependencySource::Registry {
                version: "0.3.5".into(),
            },
            workspace_fingerprint: None,
        }
    }

    fn request() -> NativeBuildRequest {
        NativeBuildRequest {
            bytecode: b"bytecode".to_vec(),
            runtime_config: None,
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "phase1-app".into(),
            output: PathBuf::from("ignored"),
            emit: NativeEmit::CargoProject,
            keep_project: true,
            offline: true,
        }
    }

    #[test]
    fn catalog_installers_are_sorted_by_numeric_runtime_id() {
        let mut plan = base_plan(NativeApplicationKind::Engine);
        plan.runtime_functions = vec![
            PlannedRuntimeFunction {
                runtime_id: 20,
                runtime_name: "Second".into(),
                package: "mech-math".into(),
                crate_name: "mech_math".into(),
                installer_path: "mech_math::__mech_native::install_second".into(),
                cargo_features: vec!["native-link".into()],
            },
            PlannedRuntimeFunction {
                runtime_id: 10,
                runtime_name: "First".into(),
                package: "mech-math".into(),
                crate_name: "mech_math".into(),
                installer_path: "mech_math::__mech_native::install_first".into(),
                cargo_features: vec!["native-link".into()],
            },
        ];

        let source = render_catalog_source(&plan).unwrap();
        assert!(source.find("install_first").unwrap() < source.find("install_second").unwrap());
        for forbidden in ["install_runtime", "runtime_catalog", "mech_stdlib"] {
            assert!(!source.contains(forbidden));
        }
    }

    #[test]
    fn host_function_installers_are_applied_to_runtime_builder_only() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.application_requirements = vec![PlannedApplicationRequirement::HostFunction {
            name: "actor/message/kind".into(),
            package: "mech-runtime".into(),
            crate_name: "mech_runtime".into(),
            installer_path: "mech_runtime::__mech_native::install_actor_message_kind".into(),
            cargo_features: vec!["native-link".into(), "runtime".into(), "string".into()],
        }];

        let catalog = render_catalog_source(&plan).unwrap();
        let runtime = render_runtime_source(&plan, None).unwrap();
        assert!(!catalog.contains("install_actor_message_kind"));
        assert!(runtime.contains(
            "builder = mech_runtime::__mech_native::install_actor_message_kind(builder)?;"
        ));
    }

    #[test]
    fn both_mains_embed_bytecode_and_only_hosted_main_constructs_runtime() {
        let engine = render_engine_main_source();
        let hosted = render_hosted_main_source();
        for source in [&engine, &hosted] {
            assert!(source.contains("include_bytes!(concat!("));
            assert!(source.contains("env!(\"CARGO_MANIFEST_DIR\")"));
            assert!(source.contains("/program.mecb"));
        }
        assert!(!engine.contains("mod runtime;"));
        assert!(hosted.contains("mod runtime;"));
        assert!(!engine.contains("pretty_print"));
    }

    #[test]
    fn runtime_source_constructs_config_hosts_factories_and_grants_directly() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.runtime_config.name = "phase1-native-runtime".into();
        plan.runtime_config.limits.max_steps_per_turn = Some(321);
        plan.runtime_config.diagnostics.trace_enabled = true;
        plan.runtime_config.diagnostics.log_level = LogLevel::Debug;
        let settings = ConfigValue::Map(BTreeMap::from([
            (
                "a".into(),
                ConfigValue::Float(f64::from_bits(0x3ff0_0000_0000_0000)),
            ),
            (
                "quote\"newline\n".into(),
                ConfigValue::String("value\\x".into()),
            ),
        ]));
        plan.application_requirements = vec![PlannedApplicationRequirement::Resource {
            base_uri: "cli://cli/stdout".into(),
            path: "line".into(),
            context_name: "out".into(),
            operation: "write".into(),
            intent: ResourceIntent::Send,
            delivery: ResourceDelivery::Snapshot,
            host_instance: "cli".into(),
            provider: "cli".into(),
        }];
        plan.hosts = vec![PlannedHostInstance {
            name: "cli".into(),
            provider: "cli".into(),
            package: "mech-host-cli".into(),
            crate_name: "mech_host_cli".into(),
            cargo_features: vec!["provider".into()],
            factory_path: "mech_host_cli::CliHostFactory::new".into(),
            settings: settings.clone(),
        }];
        plan.run_grants = vec![RunResourceGrantConfig {
            target: "cli/stdout".into(),
            operations: vec!["write".into()],
            paths: vec!["line".into()],
        }];
        let config = NativeRuntimeConfig {
            runtime: plan.runtime_config.clone(),
            hosts: vec![HostInstanceConfig {
                name: "cli".into(),
                provider: "cli".into(),
                settings,
            }],
            run_grants: plan.run_grants.clone(),
        };

        let source = render_runtime_source(&plan, Some(&config)).unwrap();
        assert!(source.contains("RuntimeConfig {"));
        assert!(source.contains("\"phase1-native-runtime\".to_string()"));
        assert!(source.contains("max_steps_per_turn: Some(321u64)"));
        assert!(source.contains("trace_enabled: true"));
        assert!(source.contains("log_level: LogLevel::Debug"));
        assert!(source.contains("ConfigValue::Map(BTreeMap::from"));
        assert!(source.contains("f64::from_bits(4607182418800017408u64)"));
        assert!(source.contains("mech_host_cli::CliHostFactory::new()?"));
        assert!(source.contains("HostInstanceConfig {"));
        assert!(source.contains("RunResourceGrantConfig {"));
        assert!(!source.contains("quote\"newline\n"));
        assert!(!source.contains("serde_json"));
    }

    #[test]
    fn complete_project_render_validates_identity_and_has_frozen_layout() {
        let mut plan = base_plan(NativeApplicationKind::Engine);
        refresh_plan_sha256(&mut plan).unwrap();
        let request = request();
        let project = render_generated_native_project("project", &request, &plan).unwrap();
        assert_eq!(
            project
                .sources
                .keys()
                .map(String::as_str)
                .collect::<Vec<_>>(),
            ["src/catalog.rs", "src/main.rs", "src/runtime.rs"]
        );
        assert_eq!(project.bytecode, request.bytecode);
        assert!(project.build_plan_json.ends_with('\n'));

        let mut mismatched = request;
        mismatched.binary_name = "other".into();
        assert!(render_generated_native_project("project", &mismatched, &plan).is_err());
    }

    #[test]
    fn host_free_runtime_settings_render_a_hosted_project_from_the_frozen_plan() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.runtime_config.name = "configured-native-runtime".into();
        plan.runtime_config.limits.max_steps_per_turn = Some(321);
        plan.runtime_config.diagnostics.trace_enabled = true;
        plan.runtime_config.diagnostics.log_level = LogLevel::Debug;
        refresh_plan_sha256(&mut plan).unwrap();

        let mut request = request();
        request.runtime_config = Some(NativeRuntimeConfig {
            runtime: plan.runtime_config.clone(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        });

        let project = render_generated_native_project("project", &request, &plan).unwrap();
        let runtime = &project.sources["src/runtime.rs"];
        assert!(runtime.contains("\"configured-native-runtime\".to_string()"));
        assert!(runtime.contains("max_steps_per_turn: Some(321u64)"));
        assert!(runtime.contains("trace_enabled: true"));
        assert!(runtime.contains("log_level: LogLevel::Debug"));

        request
            .runtime_config
            .as_mut()
            .unwrap()
            .runtime
            .limits
            .max_steps_per_turn = Some(322);
        assert!(render_generated_native_project("project", &request, &plan).is_err());
    }

    #[test]
    fn engine_render_rejects_unaddressed_host_configuration() {
        let mut plan = base_plan(NativeApplicationKind::Engine);
        refresh_plan_sha256(&mut plan).unwrap();
        let mut request = request();
        request.runtime_config = Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: vec![HostInstanceConfig {
                name: "unused".into(),
                provider: "cli".into(),
                settings: ConfigValue::Map(BTreeMap::new()),
            }],
            run_grants: vec![RunResourceGrantConfig {
                target: "unused/output".into(),
                operations: vec!["write".into()],
                paths: vec!["line".into()],
            }],
        });

        let error = render_generated_native_project("project", &request, &plan).unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
    }
}
