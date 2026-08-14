use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::path::{Path, PathBuf};

use mech_core::{MResult, ResourceDelivery};
use mech_runtime::{ConfigValue, LogLevel, RuntimeConfig};

use crate::NativeHostFunctionContext;
use crate::analysis::requirements::{
    grant_covers_resource, normalize_native_runtime_config, planned_resource_grant,
    runtime_resource_grant,
};
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
    if plan.live {
        dependencies.push(GeneratedDependency::registry(
            "ctrlc",
            "ctrlc",
            "3.5.2",
            std::iter::empty::<&str>(),
        )?);
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
    if plan.dependency_resolution_seed_sha256.len() != 64
        || !plan
            .dependency_resolution_seed_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return project_invalid(
            "native dependency resolution seed digest is not lowercase SHA-256",
        );
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
        .map(normalize_native_runtime_config)
        .transpose()?
        .unwrap_or(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
            actor_bootstrap: None,
        });
    crate::validate_production_native_runtime_config(&normalized_config)?;
    if normalized_config.runtime != plan.runtime_config {
        return project_invalid("request runtime settings do not match the normalized build plan");
    }
    if normalized_config.actor_bootstrap != plan.actor_bootstrap {
        return project_invalid("request actor bootstrap does not match the normalized build plan");
    }
    if runtime_config.is_some() && plan.application_kind != NativeApplicationKind::Hosted {
        return project_invalid(
            "a request with runtime configuration requires a hosted native plan",
        );
    }
    let resource_requirements = plan
        .application_requirements
        .iter()
        .filter_map(|requirement| match requirement {
            PlannedApplicationRequirement::Resource { request, owner } => Some((request, owner)),
            PlannedApplicationRequirement::HostFunction { .. } => None,
        })
        .collect::<Vec<_>>();
    let actor_turn_required = plan.application_requirements.iter().any(|requirement| {
        matches!(
            requirement,
            PlannedApplicationRequirement::HostFunction {
                context: NativeHostFunctionContext::ActorTurn,
                ..
            }
        )
    });
    match (actor_turn_required, plan.actor_bootstrap.is_some()) {
        (true, false) => return project_invalid("actor-turn plan has no actor bootstrap"),
        (false, true) => return project_invalid("plan has an unused actor bootstrap"),
        _ => {}
    }
    if plan.live
        != resource_requirements
            .iter()
            .any(|(request, _)| request.delivery == ResourceDelivery::Live)
    {
        return project_invalid(
            "native build plan live mode contradicts its resource requirements",
        );
    }
    if actor_turn_required && plan.live {
        return project_invalid("actor-turn plans cannot contain live resources");
    }

    if resource_requirements.is_empty() {
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
    let required_host_names = resource_requirements
        .iter()
        .map(|(_, owner)| owner.host_instance.clone())
        .collect::<BTreeSet<_>>();
    let planned_host_names = plan
        .hosts
        .iter()
        .map(|host| host.name.clone())
        .collect::<BTreeSet<_>>();
    if planned_host_names != required_host_names || planned_host_names.len() != plan.hosts.len() {
        return project_invalid("native build plan hosts are not the exact resource-owner set");
    }
    for planned in &plan.hosts {
        let Some(configured) = normalized_config
            .hosts
            .iter()
            .find(|configured| configured.name == planned.name)
        else {
            return project_invalid("native build plan selects an unconfigured host instance");
        };
        if configured.provider != planned.provider || configured.settings != planned.settings {
            return project_invalid("planned host metadata does not match runtime configuration");
        }
        if resource_requirements.iter().any(|(_, owner)| {
            owner.host_instance == planned.name && owner.provider != planned.provider
        }) {
            return project_invalid("planned resource owner provider does not match its host");
        }
    }

    let expected_grant_keys = resource_requirements
        .iter()
        .map(|(request, owner)| planned_resource_grant(request, owner))
        .collect::<BTreeSet<_>>();
    let actual_grant_keys = plan.run_grants.iter().cloned().collect::<BTreeSet<_>>();
    if actual_grant_keys.len() != plan.run_grants.len() {
        return project_invalid("native build plan contains duplicate structured run grants");
    }
    if actual_grant_keys != expected_grant_keys {
        return project_invalid("native build plan run grants are not exact resource operations");
    }
    for grant in &plan.run_grants {
        if !normalized_config
            .run_grants
            .iter()
            .any(|configured| grant_covers_resource(configured, grant))
        {
            return project_invalid("planned resource is not covered by configured run grants");
        }
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
    if plan.application_kind == NativeApplicationKind::Hosted {
        source.push_str(
            "\n    mech_engine::install_intrinsic_resident(\n        &mut builder,\n    )?;\n",
        );
    }
    source.push_str("\n    Ok(Arc::new(builder.build()?))\n}\n");
    Ok(source)
}

/// Render the engine-only entry point. `--once` is accepted uniformly across
/// generated executables even though an engine application is already one-shot.
pub fn render_engine_main_source() -> String {
    String::from(
        r#"mod catalog;

use mech_core::{LegacyValue, MResult};
use mech_engine::{
    MechProgram,
    MechProgramConfig,
};

static PROGRAM: &[u8] =
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program.mecb"
    ));

fn parse_once() -> Result<bool, ()> {
    let mut arguments = std::env::args().skip(1);
    match (arguments.next(), arguments.next()) {
        (None, None) => Ok(false),
        (Some(argument), None) if argument == "--once" => Ok(true),
        _ => Err(()),
    }
}

fn usage() {
    eprintln!("usage: generated-app [--once]");
}

fn run(_once: bool) -> MResult<()> {
    let catalog = catalog::function_catalog()?;

    let mut program =
        MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            catalog,
        );

    let value = program.run_bytecode(PROGRAM)?;

    if !matches!(value, LegacyValue::Empty) {
        println!("{value}");
    }

    Ok(())
}

fn main() {
    let once = match parse_once() {
        Ok(once) => once,
        Err(()) => {
            usage();
            std::process::exit(2);
        }
    };
    if let Err(error) = run(once) {
        eprintln!("{}", error.display_message());
        std::process::exit(1);
    }
}
"#,
    )
}

/// Render a hosted entry point. Live applications add only their exact
/// Ctrl-C dependency and do not burden one-shot hosted applications.
pub fn render_hosted_main_source(live: bool) -> String {
    if live {
        return String::from(
            r#"mod catalog;
mod runtime;

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use mech_core::{GenericError, MResult, MechError};

static PROGRAM: &[u8] =
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program.mecb"
    ));

fn generated_error(message: impl Into<String>) -> MechError {
    MechError::new(GenericError { msg: message.into() }, None)
}

#[derive(Clone, Copy)]
struct GeneratedArguments {
    once: bool,
    runtime_info: bool,
    max_live_turns: Option<usize>,
}

fn parse_arguments() -> Result<GeneratedArguments, ()> {
    let mut parsed = GeneratedArguments {
        once: false,
        runtime_info: false,
        max_live_turns: None,
    };
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--once" => parsed.once = true,
            "--runtime-info" => parsed.runtime_info = true,
            "--max-live-turns" => {
                let value = arguments.next().ok_or(())?.parse::<usize>().map_err(|_| ())?;
                if value == 0 {
                    return Err(());
                }
                parsed.max_live_turns = Some(value);
            }
            _ => return Err(()),
        }
    }
    Ok(parsed)
}

fn usage() {
    eprintln!("usage: generated-app [--once] [--runtime-info] [--max-live-turns N]");
}

fn runtime_info_json(runtime: &mech_runtime::MechRuntime) -> String {
    let info = runtime.program_execution_info();
    let route = match info.route {
        mech_runtime::RuntimeProgramRoute::None => "none",
        mech_runtime::RuntimeProgramRoute::ResidentPure => "resident-pure",
        mech_runtime::RuntimeProgramRoute::ResidentExternal => "resident-external",
        _ => "invalid-production-route",
    };
    let policy = "require-resident";
    let revision = info.program_revision.map(|revision| {
        format!("\"{}\"", revision.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>())
    }).unwrap_or_else(|| "null".to_string());
    let plan = info.plan_generation.map(|value| value.get().saturating_add(1).to_string()).unwrap_or_else(|| "null".to_string());
    let layout = info.layout_generation.map(|value| value.get().saturating_add(1).to_string()).unwrap_or_else(|| "null".to_string());
    format!("{{\"route\":\"{route}\",\"routing_policy\":\"{policy}\",\"program_revision\":{revision},\"plan_generation\":{plan},\"layout_generation\":{layout},\"requirements\":{},\"observations\":{},\"effects\":{},\"resident_accepted_turns\":{},\"resident_rejected_turns\":{},\"coalesced_host_packets\":{},\"ignored_host_packets\":{}}}", info.requirement_count, info.observation_count, info.effect_count, info.resident_accepted_turns, info.resident_rejected_turns, info.coalesced_host_packets, info.ignored_host_packets)
}

fn run(arguments: GeneratedArguments) -> (MResult<()>, Vec<MechError>) {
    let catalog = match catalog::function_catalog() {
        Ok(catalog) => catalog,
        Err(error) => return (Err(error), Vec::new()),
    };
    let mut runtime = match runtime::runtime_builder(catalog).and_then(|builder| builder.build()) {
        Ok(runtime) => runtime,
        Err(error) => return (Err(error), Vec::new()),
    };
    let runtime_constructed = true;
    let mut drivers_started = false;
    let primary = (|| -> MResult<()> {
        let durability = runtime.config().program_routing.resident_durability;
        let outcome = runtime.load_production_bytecode_program(
            PROGRAM,
            durability,
        )?;
        let value = outcome.initial_value;

        if !value.is_empty() {
            println!("{}", value.into_value());
        }

        if arguments.once {
            if arguments.runtime_info {
                println!("MECH_RUNTIME_INFO {}", runtime_info_json(&runtime));
            }
            return Ok(());
        }

        let interrupted = Arc::new(AtomicBool::new(false));
        let handler_flag = Arc::clone(&interrupted);
        ctrlc::set_handler(move || {
            handler_flag.store(true, Ordering::SeqCst);
        })
        .map_err(|error| generated_error(format!("failed to install Ctrl-C handler: {error}")))?;

        drivers_started = true;
        runtime.start_input_drivers()?;
        let mut completed_live_turns = 0usize;
        while !interrupted.load(Ordering::SeqCst) {
            if arguments.max_live_turns.is_some_and(|limit| completed_live_turns >= limit) {
                break;
            }
            let drain_limit = arguments.max_live_turns
                .map(|limit| limit.saturating_sub(completed_live_turns))
                .unwrap_or(64)
                .min(64);
            let outcomes = runtime.drain_host_inputs(drain_limit)?;
            completed_live_turns = completed_live_turns.saturating_add(
                outcomes.iter().filter(|outcome| {
                    outcome.turn.is_some() || outcome.resident_turn.is_some()
                }).count(),
            );
            if outcomes.is_empty() {
                std::thread::sleep(Duration::from_millis(10));
            }
        }
        if arguments.runtime_info {
            println!("MECH_RUNTIME_INFO {}", runtime_info_json(&runtime));
        }
        Ok(())
    })();

    let mut cleanup = Vec::new();
    if drivers_started {
        if let Err(error) = runtime.stop_input_drivers() {
            cleanup.push(error);
        }
    }
    if runtime_constructed {
        if let Err(error) = runtime.shutdown() {
            cleanup.push(error);
        }
    }
    (primary, cleanup)
}

fn main() {
    let arguments = match parse_arguments() {
        Ok(arguments) => arguments,
        Err(()) => {
            usage();
            std::process::exit(2);
        }
    };
    let (primary, cleanup) = run(arguments);
    let mut failed = false;
    if let Err(error) = primary {
        eprintln!("{}", error.display_message());
        failed = true;
    }
    for error in cleanup {
        eprintln!("cleanup: {}", error.display_message());
        failed = true;
    }
    if failed {
        std::process::exit(1);
    }
}
"#,
        );
    }

    String::from(
        r#"mod catalog;
mod runtime;

use mech_core::{MResult, MechError};

static PROGRAM: &[u8] =
    include_bytes!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/program.mecb"
    ));

#[derive(Clone, Copy)]
struct GeneratedArguments {
    runtime_info: bool,
}

fn parse_arguments() -> Result<GeneratedArguments, ()> {
    let mut parsed = GeneratedArguments { runtime_info: false };
    let mut arguments = std::env::args().skip(1);
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--once" => {}
            "--runtime-info" => parsed.runtime_info = true,
            "--max-live-turns" => {
                let value = arguments.next().ok_or(())?.parse::<usize>().map_err(|_| ())?;
                if value == 0 {
                    return Err(());
                }
            }
            _ => return Err(()),
        }
    }
    Ok(parsed)
}

fn usage() {
    eprintln!("usage: generated-app [--once] [--runtime-info] [--max-live-turns N]");
}

fn runtime_info_json(runtime: &mech_runtime::MechRuntime) -> String {
    let info = runtime.program_execution_info();
    let route = match info.route {
        mech_runtime::RuntimeProgramRoute::None => "none",
        mech_runtime::RuntimeProgramRoute::ResidentPure => "resident-pure",
        mech_runtime::RuntimeProgramRoute::ResidentExternal => "resident-external",
        _ => "invalid-production-route",
    };
    let policy = "require-resident";
    let revision = info.program_revision.map(|revision| {
        format!("\"{}\"", revision.as_bytes().iter().map(|byte| format!("{byte:02x}")).collect::<String>())
    }).unwrap_or_else(|| "null".to_string());
    let plan = info.plan_generation.map(|value| value.get().saturating_add(1).to_string()).unwrap_or_else(|| "null".to_string());
    let layout = info.layout_generation.map(|value| value.get().saturating_add(1).to_string()).unwrap_or_else(|| "null".to_string());
    format!("{{\"route\":\"{route}\",\"routing_policy\":\"{policy}\",\"program_revision\":{revision},\"plan_generation\":{plan},\"layout_generation\":{layout},\"requirements\":{},\"observations\":{},\"effects\":{},\"resident_accepted_turns\":{},\"resident_rejected_turns\":{},\"coalesced_host_packets\":{},\"ignored_host_packets\":{}}}", info.requirement_count, info.observation_count, info.effect_count, info.resident_accepted_turns, info.resident_rejected_turns, info.coalesced_host_packets, info.ignored_host_packets)
}

fn run(arguments: GeneratedArguments) -> (MResult<()>, Vec<MechError>) {
    let catalog = match catalog::function_catalog() {
        Ok(catalog) => catalog,
        Err(error) => return (Err(error), Vec::new()),
    };
    let mut runtime = match runtime::runtime_builder(catalog).and_then(|builder| builder.build()) {
        Ok(runtime) => runtime,
        Err(error) => return (Err(error), Vec::new()),
    };
    let runtime_constructed = true;
    let primary = (|| -> MResult<()> {
        let durability = runtime.config().program_routing.resident_durability;
        let outcome = runtime.load_production_bytecode_program(
            PROGRAM,
            durability,
        )?;
        let value = outcome.initial_value;

        if !value.is_empty() {
            println!("{}", value.into_value());
        }
        if arguments.runtime_info {
            println!("MECH_RUNTIME_INFO {}", runtime_info_json(&runtime));
        }
        Ok(())
    })();

    let cleanup = if runtime_constructed {
        runtime.shutdown().err().into_iter().collect()
    } else {
        Vec::new()
    };
    (primary, cleanup)
}

fn main() {
    let arguments = match parse_arguments() {
        Ok(arguments) => arguments,
        Err(()) => {
            usage();
            std::process::exit(2);
        }
    };
    let (primary, cleanup) = run(arguments);
    let mut failed = false;
    if let Err(error) = primary {
        eprintln!("{}", error.display_message());
        failed = true;
    }
    for error in cleanup {
        eprintln!("cleanup: {}", error.display_message());
        failed = true;
    }
    if failed {
        std::process::exit(1);
    }
}
"#,
    )
}

fn render_hosted_main_source_for_plan(plan: &NativeBuildPlan) -> String {
    render_hosted_main_source(plan.live)
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
        "use std::{collections::BTreeMap, sync::Arc};\n\nuse mech_core::{FunctionCatalog, MResult};\nuse mech_runtime::{\n    ConfigValue, DiagnosticsConfig, ProgramRoutingConfig, HostInstanceConfig, LogLevel,\n    ResidentDurabilityPolicy, ResidentRoutingPolicy, RunResourceGrantConfig,\n    RuntimeBuilder, RuntimeConfig, RuntimeLimits,\n};\n",
    );
    if let Some(actor) = &plan.actor_bootstrap {
        write!(
            &mut source,
            "\npub const ACTOR_BOOTSTRAP_SUBJECT: &str = {};\n\
             pub const ACTOR_BOOTSTRAP_MESSAGE_KIND: &str = {};\n\
             pub const ACTOR_BOOTSTRAP_MESSAGE_PAYLOAD: &str = {};\n\
             pub const ACTOR_BOOTSTRAP_INITIAL_STATE: Option<&str> = {};\n",
            rust_string_literal(&actor.subject),
            rust_string_literal(&actor.message_kind),
            rust_string_literal(&actor.message_payload),
            actor
                .initial_state
                .as_ref()
                .map(|state| format!("Some({})", rust_string_literal(state)))
                .unwrap_or_else(|| "None".to_owned()),
        )
        .expect("writing to String cannot fail");
    }
    source.push_str(
        "\npub fn runtime_builder(\n    catalog: Arc<FunctionCatalog>,\n) -> MResult<RuntimeBuilder> {\n",
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
        let grant = runtime_resource_grant(grant);
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
    // Reject unsupported actor-turn plans before selecting any source
    // template. This keeps the legacy actor bootstrap renderer unreachable
    // from every public project-rendering entry point.
    validate_runtime_config_implications(plan, runtime_config)?;
    let mut sources = GeneratedSourceSet::new();
    sources.insert("src/catalog.rs", render_catalog_source(plan)?)?;
    sources.insert(
        "src/main.rs",
        match plan.application_kind {
            NativeApplicationKind::Engine => render_engine_main_source(),
            NativeApplicationKind::Hosted => render_hosted_main_source_for_plan(plan),
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
    workspace_root: Option<&Path>,
) -> MResult<GeneratedNativeProject> {
    validate_generation_identity(request, plan)?;
    let root = root.into();
    let dependencies = generated_dependencies_from_plan(plan)?;
    let manifest = NativeProjectManifest::new(
        plan.binary_name.clone(),
        plan.binary_name.clone(),
        dependencies,
    )?;
    let cargo_manifest = render_native_project_manifest(&manifest, &root, workspace_root)?;
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
    config
        .validate_production_program_routing()
        .expect("native production plan must require resident execution");
    format!(
        "RuntimeConfig {{\n        name: {}.to_string(),\n        program_routing: ProgramRoutingConfig {{\n            resident_routing: ResidentRoutingPolicy::{},\n            resident_durability: ResidentDurabilityPolicy::{},\n        }},\n        limits: RuntimeLimits {{\n            max_steps_per_turn: {},\n            max_turn_duration_ms: {},\n            max_memory_bytes: {},\n            max_tasks: {},\n            max_actors: {},\n            max_actor_mailbox_len: {},\n            max_source_bytes: {},\n            max_in_memory_events: {},\n        }},\n        diagnostics: DiagnosticsConfig {{\n            trace_enabled: {},\n            profile_enabled: {},\n            debug_enabled: {},\n            log_level: LogLevel::{},\n        }},\n    }}",
        rust_string_literal(&config.name),
        "RequireResident",
        match config.program_routing.resident_durability {
            mech_runtime::ResidentDurabilityPolicy::Volatile => "Volatile",
            mech_runtime::ResidentDurabilityPolicy::Retained => "Retained",
            mech_runtime::ResidentDurabilityPolicy::AsynchronousDurable => "AsynchronousDurable",
            mech_runtime::ResidentDurabilityPolicy::SynchronousDurable => "SynchronousDurable",
            mech_runtime::ResidentDurabilityPolicy::ReplicatedDurable => "ReplicatedDurable",
        },
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
        NativeActorBootstrap, NativeBuildProfile, NativeEmit, PlannedHostInstance, PlannedPackage,
        PlannedResourceGrantKey, PlannedResourceOwner, PlannedResourceRequest,
        PlannedRuntimeFunction, refresh_plan_sha256,
    };
    use crate::project::GeneratedDependencySource;

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
            actor_bootstrap: None,
            bytecode_sha256: sha256_hex(b"bytecode"),
            plan_sha256: String::new(),
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "native-app".into(),
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
            dependency_resolution_seed_sha256: sha256_hex(b"registry lock seed"),
            workspace_fingerprint: None,
        }
    }

    fn request() -> NativeBuildRequest {
        NativeBuildRequest {
            bytecode: b"bytecode".to_vec(),
            runtime_config: None,
            target: None,
            profile: NativeBuildProfile::Debug,
            binary_name: "native-app".into(),
            output: PathBuf::from("ignored"),
            emit: NativeEmit::CargoProject,
            keep_project: true,
            offline: true,
        }
    }

    fn resource_requirement(
        base_uri: &str,
        host_instance: &str,
        provider: &str,
        host_context: &str,
    ) -> PlannedApplicationRequirement {
        PlannedApplicationRequirement::Resource {
            request: PlannedResourceRequest {
                base_uri: base_uri.into(),
                path: "line".into(),
                context_name: "out".into(),
                operation: "write".into(),
                intent: ResourceIntent::Send,
                delivery: ResourceDelivery::Snapshot,
            },
            owner: PlannedResourceOwner {
                host_instance: host_instance.into(),
                provider: provider.into(),
                host_context: host_context.into(),
                canonical_base_uri: base_uri.into(),
            },
        }
    }

    fn planned_grant(host_instance: &str, host_context: &str) -> PlannedResourceGrantKey {
        PlannedResourceGrantKey {
            host_instance: host_instance.into(),
            host_context: host_context.into(),
            operation: "write".into(),
            path: "line".into(),
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
    fn hosted_catalog_installs_the_production_resident_factory_surface() {
        let plan = base_plan(NativeApplicationKind::Hosted);
        let source = render_catalog_source(&plan).unwrap();

        assert!(source.contains("mech_engine::install_intrinsic_resident"));
    }

    #[test]
    fn host_function_installers_are_applied_to_runtime_builder_only() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.application_requirements = vec![PlannedApplicationRequirement::HostFunction {
            name: "actor/message/kind".into(),
            context: NativeHostFunctionContext::Standalone,
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
    fn actor_bootstrap_has_no_legacy_execution_template() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.application_requirements = [
            "actor/message/kind",
            "actor/message/payload",
            "actor/state/get",
            "actor/state/id",
            "actor/state/put",
        ]
        .into_iter()
        .map(|name| PlannedApplicationRequirement::HostFunction {
            name: name.into(),
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime".into(),
            crate_name: "mech_runtime".into(),
            installer_path: format!(
                "mech_runtime::__mech_native::install_{}",
                name.replace('/', "_")
            ),
            cargo_features: vec!["native-link".into(), "runtime".into(), "string".into()],
        })
        .collect();
        plan.actor_bootstrap = Some(NativeActorBootstrap {
            subject: "render-test-actor".into(),
            message_kind: "render-test-message".into(),
            message_payload: "render-test-payload".into(),
            initial_state: Some("render-test-state".into()),
        });

        let hosted = render_hosted_main_source_for_plan(&plan);
        assert!(!hosted.contains("legacy_interpreter"));
        assert!(!hosted.contains("install_actor_bytecode"));

        let config = NativeRuntimeConfig {
            runtime: plan.runtime_config.clone(),
            actor_bootstrap: plan.actor_bootstrap.clone(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        };
        let error = render_project_sources(&plan, Some(&config)).unwrap_err();
        assert_eq!(error.kind_name(), "NativeActorBootstrapUnsupported");
    }

    #[test]
    fn distinct_actor_bootstraps_remain_unrenderable_production_plans() {
        let mut alpha = base_plan(NativeApplicationKind::Hosted);
        alpha.application_requirements = vec![PlannedApplicationRequirement::HostFunction {
            name: "actor/message/kind".into(),
            context: NativeHostFunctionContext::ActorTurn,
            package: "mech-runtime".into(),
            crate_name: "mech_runtime".into(),
            installer_path: "mech_runtime::__mech_native::install_actor_message_kind".into(),
            cargo_features: vec!["native-link".into(), "runtime".into(), "string".into()],
        }];
        alpha.actor_bootstrap = Some(NativeActorBootstrap {
            subject: "actor:alpha".into(),
            message_kind: "alpha".into(),
            message_payload: "payload-a".into(),
            initial_state: Some("state-a".into()),
        });
        let mut beta = alpha.clone();
        beta.actor_bootstrap = Some(NativeActorBootstrap {
            subject: "actor:beta".into(),
            message_kind: "beta".into(),
            message_payload: "payload-b".into(),
            initial_state: Some("state-b".into()),
        });
        let alpha_config = NativeRuntimeConfig {
            runtime: alpha.runtime_config.clone(),
            actor_bootstrap: alpha.actor_bootstrap.clone(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        };
        let beta_config = NativeRuntimeConfig {
            runtime: beta.runtime_config.clone(),
            actor_bootstrap: beta.actor_bootstrap.clone(),
            hosts: Vec::new(),
            run_grants: Vec::new(),
        };
        for (plan, config) in [(&alpha, &alpha_config), (&beta, &beta_config)] {
            let error = render_runtime_source(plan, Some(config)).unwrap_err();
            assert_eq!(error.kind_name(), "NativeActorBootstrapUnsupported");
        }
        assert_ne!(
            compute_plan_sha256(&alpha).unwrap(),
            compute_plan_sha256(&beta).unwrap()
        );
    }

    #[test]
    fn both_mains_embed_bytecode_and_only_hosted_main_constructs_runtime() {
        let engine = render_engine_main_source();
        let hosted = render_hosted_main_source(false);
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
    fn generated_main_argument_surfaces_match_their_execution_models() {
        let engine = render_engine_main_source();
        assert!(engine.contains("Some(argument), None) if argument == \"--once\""));
        assert!(engine.contains("usage: generated-app [--once]"));
        assert!(!engine.contains("--runtime-info"));

        for source in [
            render_hosted_main_source(false),
            render_hosted_main_source(true),
        ] {
            assert!(source.contains("\"--runtime-info\" => parsed.runtime_info = true"));
            assert!(source.contains("\"--max-live-turns\""));
            assert!(
                source.contains(
                    "usage: generated-app [--once] [--runtime-info] [--max-live-turns N]"
                )
            );
            assert!(source.contains("std::process::exit(2)"));
            assert!(source.contains("\\\"resident_accepted_turns\\\":{}"));
        }
    }

    #[test]
    fn live_projects_have_an_exact_ctrlc_dependency_and_bounded_shutdown_loop() {
        let mut live_plan = base_plan(NativeApplicationKind::Hosted);
        live_plan.live = true;
        let dependencies = generated_dependencies_from_plan(&live_plan).unwrap();
        let ctrlc = dependencies
            .iter()
            .find(|dependency| dependency.package == "ctrlc")
            .unwrap();
        assert_eq!(ctrlc.crate_name, "ctrlc");
        assert!(matches!(
            &ctrlc.source,
            GeneratedDependencySource::Registry { exact_version } if exact_version == "=3.5.2"
        ));

        let live_source = render_hosted_main_source(true);
        assert!(live_source.contains("AtomicBool"));
        assert!(live_source.contains("Ordering::SeqCst"));
        assert!(live_source.contains("limit.saturating_sub(completed_live_turns)"));
        assert!(live_source.contains("runtime.drain_host_inputs(drain_limit)?"));
        assert!(live_source.contains("Duration::from_millis(10)"));
        assert!(live_source.contains("runtime.stop_input_drivers()"));
        assert!(live_source.contains("runtime.shutdown()"));
        assert!(live_source.contains("runtime_constructed"));
        assert!(live_source.contains("drivers_started"));

        let one_shot_dependencies =
            generated_dependencies_from_plan(&base_plan(NativeApplicationKind::Hosted)).unwrap();
        assert!(
            one_shot_dependencies
                .iter()
                .all(|dependency| dependency.package != "ctrlc")
        );
        assert!(!render_hosted_main_source(false).contains("ctrlc::set_handler"));
    }

    #[test]
    fn runtime_source_constructs_config_hosts_factories_and_grants_directly() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.runtime_config.name = "native-runtime".into();
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
        plan.application_requirements = vec![resource_requirement(
            "cli://cli/stdout",
            "cli",
            "cli",
            "stdout",
        )];
        plan.hosts = vec![PlannedHostInstance {
            name: "cli".into(),
            provider: "cli".into(),
            package: "mech-terminal".into(),
            crate_name: "mech_terminal".into(),
            cargo_features: vec!["provider".into()],
            factory_path: "mech_terminal::CliHostFactory::new".into(),
            settings: settings.clone(),
        }];
        plan.run_grants = vec![planned_grant("cli", "stdout")];
        let config = NativeRuntimeConfig {
            runtime: plan.runtime_config.clone(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "cli".into(),
                provider: "cli".into(),
                settings,
            }],
            run_grants: plan.run_grants.iter().map(runtime_resource_grant).collect(),
        };

        let source = render_runtime_source(&plan, Some(&config)).unwrap();
        assert!(source.contains("RuntimeConfig {"));
        assert!(source.contains("\"native-runtime\".to_string()"));
        assert!(source.contains("max_steps_per_turn: Some(321u64)"));
        assert!(source.contains("trace_enabled: true"));
        assert!(source.contains("log_level: LogLevel::Debug"));
        assert!(source.contains("ConfigValue::Map(BTreeMap::from"));
        assert!(source.contains("f64::from_bits(4607182418800017408u64)"));
        assert!(source.contains("mech_terminal::CliHostFactory::new()?"));
        assert!(source.contains("HostInstanceConfig {"));
        assert!(source.contains("RunResourceGrantConfig {"));
        assert!(!source.contains("quote\"newline\n"));
        assert!(!source.contains("serde_json"));
    }

    #[test]
    fn runtime_render_rejects_a_grant_for_a_peer_resource_context() {
        let mut plan = base_plan(NativeApplicationKind::Hosted);
        plan.application_requirements = vec![resource_requirement(
            "cli://cli/stdout",
            "cli",
            "cli",
            "stdout",
        )];
        plan.hosts = vec![PlannedHostInstance {
            name: "cli".into(),
            provider: "cli".into(),
            package: "mech-terminal".into(),
            crate_name: "mech_terminal".into(),
            cargo_features: vec!["provider".into()],
            factory_path: "mech_terminal::CliHostFactory::new".into(),
            settings: ConfigValue::Map(BTreeMap::new()),
        }];
        plan.run_grants = vec![planned_grant("cli", "stderr")];
        let config = NativeRuntimeConfig {
            runtime: plan.runtime_config.clone(),
            actor_bootstrap: None,
            hosts: vec![HostInstanceConfig {
                name: "cli".into(),
                provider: "cli".into(),
                settings: ConfigValue::Map(BTreeMap::new()),
            }],
            run_grants: vec![
                RunResourceGrantConfig {
                    target: "cli/stdout".into(),
                    operations: vec!["write".into()],
                    paths: vec!["line".into()],
                },
                runtime_resource_grant(&plan.run_grants[0]),
            ],
        };

        let error = render_runtime_source(&plan, Some(&config)).unwrap_err();
        assert!(
            error
                .full_chain_message()
                .contains("run grants are not exact resource operations")
        );
    }

    #[test]
    fn complete_project_render_validates_identity_and_has_frozen_layout() {
        let mut plan = base_plan(NativeApplicationKind::Engine);
        refresh_plan_sha256(&mut plan).unwrap();
        let request = request();
        let project = render_generated_native_project("project", &request, &plan, None).unwrap();
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
        assert!(render_generated_native_project("project", &mismatched, &plan, None).is_err());
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
            actor_bootstrap: None,
            hosts: Vec::new(),
            run_grants: Vec::new(),
        });

        let project = render_generated_native_project("project", &request, &plan, None).unwrap();
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
        assert!(render_generated_native_project("project", &request, &plan, None).is_err());
    }

    #[test]
    fn engine_render_rejects_unaddressed_host_configuration() {
        let mut plan = base_plan(NativeApplicationKind::Engine);
        refresh_plan_sha256(&mut plan).unwrap();
        let mut request = request();
        request.runtime_config = Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::default(),
            actor_bootstrap: None,
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

        let error = render_generated_native_project("project", &request, &plan, None).unwrap_err();
        assert_eq!(error.kind_name(), "NativeProjectInvalid");
    }
}
