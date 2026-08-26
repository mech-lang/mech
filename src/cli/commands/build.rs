use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use mech_build::{
    NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile, NativeBuildRequest,
    NativeDependencySource, NativeEmit, NativeRuntimeConfig,
};
use mech_core::*;
use mech_runtime::{HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig, SourceRequest};

use crate::cli::module_execution::{module_runtime_config, prepare_source_program_compiler};
use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::source_discovery::{DiscoveryOptions, MissingPathPolicy, collect_sources};

const BUILD_EXTENSIONS: &[&str] = &["mec", "🤖", "mdoc", "mpkg"];
const BUILD_SKIP_DIRECTORIES: &[&str] = &[".git", "dist", "out", "target"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildEmit {
    Native,
    Bytecode,
    CargoProject,
    Plan,
}

impl From<BuildEmit> for NativeEmit {
    fn from(value: BuildEmit) -> Self {
        match value {
            BuildEmit::Native => NativeEmit::Native,
            BuildEmit::Bytecode => NativeEmit::Bytecode,
            BuildEmit::CargoProject => NativeEmit::CargoProject,
            BuildEmit::Plan => NativeEmit::Plan,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildProfile {
    Debug,
    Release,
}

impl From<BuildProfile> for NativeBuildProfile {
    fn from(value: BuildProfile) -> Self {
        match value {
            BuildProfile::Debug => NativeBuildProfile::Debug,
            BuildProfile::Release => NativeBuildProfile::Release,
        }
    }
}

pub(crate) fn command() -> Command {
    Command::new("build")
        .about("Build a Mech program as a native application, bytecode, plan, or Cargo project.")
        .arg(
            Arg::new("mech_build_file_paths")
                .help("Exactly one resident source root or one .mecb bytecode file")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("build_emit")
                .long("emit")
                .value_name("EMIT")
                .value_parser(["native", "bytecode", "cargo-project", "plan"])
                .default_value("native")
                .help("Artifact to emit: native, bytecode, cargo-project, or plan."),
        )
        .arg(
            Arg::new("build_name")
                .long("name")
                .value_name("NAME")
                .help("Deterministic application binary name."),
        )
        .arg(
            Arg::new("output_path")
                .short('o')
                .long("out")
                .value_name("PATH")
                .help("Exact artifact output path."),
        )
        .arg(
            Arg::new("build_target")
                .long("target")
                .value_name("TARGET")
                .help("Cargo target triple for native application generation."),
        )
        .arg(
            Arg::new("build_profile")
                .long("profile")
                .value_name("PROFILE")
                .value_parser(["debug", "release"])
                .default_value("release")
                .help("Native Cargo profile: debug or release."),
        )
        .arg(
            Arg::new("workspace_root")
                .long("workspace-root")
                .value_name("PATH")
                .help("Use exact workspace component paths instead of the published registry."),
        )
        .arg(
            Arg::new("keep_project")
                .long("keep-project")
                .action(ArgAction::SetTrue)
                .help("Copy the deterministic generated project beside the emitted artifact."),
        )
        .arg(
            Arg::new("offline")
                .long("offline")
                .action(ArgAction::SetTrue)
                .help("Pass Cargo --offline while generating and building native projects."),
        )
}

fn is_bytecode_source_path(path: &str) -> bool {
    Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("mecb"))
}

pub(crate) fn validate_build_bytecode_inputs(paths: &[String]) -> MResult<usize> {
    if paths.is_empty() {
        return build_error(
            "no build inputs supplied; pass source root(s) or exactly one .mecb file",
        );
    }
    let bytecode_count = paths
        .iter()
        .filter(|path| is_bytecode_source_path(path))
        .count();
    if bytecode_count > 0 && bytecode_count != paths.len() {
        return build_error(
            "Cannot mix bytecode (.mecb) inputs with source inputs in `mech build`; build bytecode inputs separately or rebuild from source.",
        );
    }
    if bytecode_count > 1 {
        return build_error(
            "Cannot combine multiple bytecode (.mecb) inputs in one `mech build` invocation.",
        );
    }
    Ok(bytecode_count)
}

pub(crate) struct BuildOptions {
    pub paths: Vec<String>,
    pub emit: BuildEmit,
    pub name: Option<String>,
    pub output_path: Option<PathBuf>,
    pub target: Option<String>,
    pub profile: BuildProfile,
    pub config_path: Option<String>,
    pub no_config: bool,
    pub workspace_root: Option<PathBuf>,
    pub keep_project: bool,
    pub offline: bool,
    pub debug: bool,
    pub trace: bool,
    pub rounds_per_step: usize,
}

impl BuildOptions {
    pub(crate) fn from_matches(
        root: RootFlags,
        root_matches: &ArgMatches,
        matches: &ArgMatches,
    ) -> MResult<Self> {
        let emit = parse_emit(
            matches
                .get_one::<String>("build_emit")
                .expect("build emit has a clap default"),
        )?;
        let profile = parse_profile(
            matches
                .get_one::<String>("build_profile")
                .expect("build profile has a clap default"),
        )?;
        let options = Self {
            paths: matches
                .get_many::<String>("mech_build_file_paths")
                .map_or_else(Vec::new, |files| files.cloned().collect()),
            emit,
            name: matches.get_one::<String>("build_name").cloned(),
            output_path: matches.get_one::<String>("output_path").map(PathBuf::from),
            target: matches.get_one::<String>("build_target").cloned(),
            profile,
            config_path: root_matches.get_one::<String>("config").cloned(),
            no_config: root_matches.get_flag("no_config"),
            workspace_root: matches
                .get_one::<String>("workspace_root")
                .map(PathBuf::from),
            keep_project: matches.get_flag("keep_project"),
            offline: matches.get_flag("offline"),
            debug: root.debug,
            trace: root.trace,
            rounds_per_step: root.rounds_per_step.unwrap_or(10_000),
        };
        if options.emit == BuildEmit::CargoProject && options.keep_project {
            return build_error("`--emit cargo-project` cannot be combined with `--keep-project`");
        }
        if options.emit != BuildEmit::Bytecode || options.keep_project {
            if let Some(name) = options.name.as_deref() {
                mech_build::validate_project_binary_name(name)?;
            }
            if let Some(target) = options.target.as_deref() {
                mech_build::validate_project_target_triple(target)?;
            }
        }
        Ok(options)
    }
}

pub(crate) fn run(options: BuildOptions) -> MResult<CliOutcome> {
    let bytecode_count = validate_build_bytecode_inputs(&options.paths)?;
    let binary_name = options
        .name
        .clone()
        .unwrap_or_else(|| inferred_binary_name(&options.paths[0]));
    if options.emit != BuildEmit::Bytecode || options.keep_project {
        mech_build::validate_project_binary_name(&binary_name)?;
    }

    let (bytecode, loaded_config) = if bytecode_count == 1 {
        let path = PathBuf::from(&options.paths[0]);
        let bytecode = fs::read(&path)?;
        let config = load_build_config(&options, Some(&path))?;
        validate_production_build_config(config.as_ref(), &binary_name)?;
        (bytecode, config)
    } else {
        let loaded_config = load_build_config(&options, None)?;
        validate_production_build_config(loaded_config.as_ref(), &binary_name)?;
        let source_roots = discover_source_roots(&options.paths)?;
        if source_roots.is_empty() {
            return Err(MechError::new(
                mech_runtime::ResidentRouteFailure {
                    class: mech_runtime::ResidentRouteFailureClass::SemanticUnsupported,
                    reason: "production builds require at least one resident source root"
                        .to_string(),
                },
                None,
            ));
        }
        enforce_production_build_source_shape(&source_roots)?;
        let configured_hosts = configured_hosts(loaded_config.as_ref());
        let run_grants = configured_run_grants(loaded_config.as_ref());
        let cli_grants = crate::cli::host_grants::effective_cli_host_grants(
            loaded_config.as_ref(),
            crate::cli::host_grants::CliHostCapabilitySelection::default(),
        )?;
        let mut planner_config = module_runtime_config(
            format!("{binary_name}-planner"),
            options.debug,
            options.trace,
            options.rounds_per_step,
        )?;
        if let Some(config) = loaded_config.as_ref() {
            planner_config =
                crate::apply_runtime_config_patch(planner_config, &config.document.runtime)?;
        }
        let (mut compiler, roots) = prepare_source_program_compiler(
            planner_config,
            &cli_grants,
            &configured_hosts,
            &run_grants,
            None,
            &source_roots,
        )?;
        let requests = roots
            .iter()
            .map(|root| SourceRequest::from_filesystem_path(root))
            .collect::<MResult<Vec<_>>>()?;
        let options = mech_runtime::ModuleBuildOptions::new(
            env!("CARGO_PKG_VERSION"),
            "v0.3",
            "native",
            &[],
            &[],
        );
        let bytecode = compiler.compile_roots(&requests, options)?.into_parts().1;
        (bytecode, loaded_config)
    };

    let parsed = ParsedProgram::from_bytes(&bytecode)?;
    validate_build_product_capabilities(
        !parsed.artifact.compute_regions.is_empty(),
        options.emit,
        options.keep_project,
    )?;

    let requested_output = options
        .output_path
        .clone()
        .unwrap_or_else(|| default_output_path(options.emit, &binary_name));
    let requested_project_output = if options.keep_project && options.emit != BuildEmit::Native {
        let project_output = project_output_path(&requested_output);
        refuse_existing_project_output(&project_output)?;
        Some(project_output)
    } else {
        None
    };

    if options.emit == BuildEmit::Bytecode {
        copy_exact_file_bytes(&bytecode, &requested_output)?;
        println!(
            "[Output] Mech bytecode written to: {}",
            requested_output.display()
        );
        if !options.keep_project {
            return Ok(CliOutcome::success());
        }
    }

    let runtime_config = native_runtime_config(
        loaded_config.as_ref(),
        &binary_name,
        !parsed.requirements.is_empty(),
        parsed
            .requirements
            .iter()
            .any(|requirement| matches!(requirement, ApplicationRequirement::Resource(_))),
    )?
    .as_ref()
    .map(mech_build::normalize_native_runtime_config)
    .transpose()?;
    let dependency_source = match options.workspace_root.as_ref() {
        Some(root) => {
            let root = root.canonicalize()?;
            NativeDependencySource::Workspace { root }
        }
        None => NativeDependencySource::Registry {
            version: mech_build::MECH_COMPONENT_VERSION.to_owned(),
        },
    };
    let environment = NativeBuildEnvironment {
        function_catalog: mech_stdlib::native_plan_catalog(),
        host_catalog: mech_build::selected_native_host_catalog()?,
        dependency_source,
    };
    let request = NativeBuildRequest {
        bytecode,
        runtime_config,
        target: options.target.clone(),
        profile: options.profile.into(),
        binary_name: binary_name.clone(),
        output: requested_output.clone(),
        emit: options.emit.into(),
        keep_project: options.keep_project,
        offline: options.offline,
    };
    let builder = NativeApplicationBuilder::new(environment);
    let plan = builder.plan(&request)?;

    match options.emit {
        BuildEmit::Plan => {
            let plan_json = serde_json::to_vec_pretty(&plan).map_err(|error| {
                MechError::new(
                    GenericError {
                        msg: format!("failed to serialize native build plan: {error}"),
                    },
                    None,
                )
                .with_compiler_loc()
            })?;
            copy_exact_file_bytes(&plan_json, &requested_output)?;
            if let Some(project_output) = requested_project_output {
                builder.generate_at(&request, &plan, project_output)?;
            }
            println!(
                "[Output] Native build plan written to: {}",
                requested_output.display()
            );
        }
        BuildEmit::CargoProject => {
            refuse_existing_project_output(&requested_output)?;
            builder.generate_at(&request, &plan, &requested_output)?;
            println!(
                "[Output] Native Cargo project written to: {}",
                requested_output.display()
            );
        }
        BuildEmit::Native => {
            let artifact = builder.build(&request, &plan)?;
            let output = options
                .output_path
                .clone()
                .unwrap_or_else(|| default_native_output_path(&binary_name, &artifact.executable));
            let project_output = if options.keep_project {
                let project_output = project_output_path(&output);
                refuse_existing_project_output(&project_output)?;
                Some(project_output)
            } else {
                None
            };
            copy_exact_file(&artifact.executable, &output)?;
            if let Some(project_output) = project_output {
                builder.generate_at(&request, &plan, project_output)?;
            }
            println!(
                "[Output] Native Mech application written to: {}",
                output.display()
            );
        }
        BuildEmit::Bytecode => {
            let project_output = requested_project_output
                .expect("bytecode keep-project output was preflighted before artifact writes");
            builder.generate_at(&request, &plan, project_output)?;
        }
    }
    Ok(CliOutcome::success())
}

fn build_error<T>(message: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        GenericError {
            msg: message.into(),
        },
        None,
    )
    .with_compiler_loc())
}

fn validate_build_product_capabilities(
    has_compute_regions: bool,
    emit: BuildEmit,
    keep_project: bool,
) -> MResult<()> {
    if !has_compute_regions || (emit == BuildEmit::Bytecode && !keep_project) {
        return Ok(());
    }
    Err(MechError::new(
        mech_runtime::ResidentRouteFailure {
            class: mech_runtime::ResidentRouteFailureClass::SemanticUnsupported,
            reason: format!(
                "`mech build --emit {}` cannot package named compute regions yet; use `--emit bytecode` without `--keep-project` to preserve their metadata, or run the mixed source with a configured compute host",
                build_emit_name(emit),
            ),
        },
        None,
    ))
}

fn build_emit_name(emit: BuildEmit) -> &'static str {
    match emit {
        BuildEmit::Native => "native",
        BuildEmit::Bytecode => "bytecode",
        BuildEmit::CargoProject => "cargo-project",
        BuildEmit::Plan => "plan",
    }
}

fn parse_emit(value: &str) -> MResult<BuildEmit> {
    match value {
        "native" => Ok(BuildEmit::Native),
        "bytecode" => Ok(BuildEmit::Bytecode),
        "cargo-project" => Ok(BuildEmit::CargoProject),
        "plan" => Ok(BuildEmit::Plan),
        _ => build_error(format!("unsupported build emit `{value}`")),
    }
}

fn parse_profile(value: &str) -> MResult<BuildProfile> {
    match value {
        "debug" => Ok(BuildProfile::Debug),
        "release" => Ok(BuildProfile::Release),
        _ => build_error(format!("unsupported build profile `{value}`")),
    }
}

fn inferred_binary_name(input: &str) -> String {
    Path::new(input)
        .file_stem()
        .or_else(|| Path::new(input).file_name())
        .map(|part| part.to_string_lossy().to_string())
        .unwrap_or_else(|| "mech_app".to_owned())
}

fn default_output_path(emit: BuildEmit, binary_name: &str) -> PathBuf {
    let base = PathBuf::from("target/mech").join(binary_name);
    match emit {
        BuildEmit::Native => base,
        BuildEmit::Bytecode => base.with_extension("mecb"),
        BuildEmit::CargoProject => PathBuf::from(format!("{}.cargo", base.display())),
        BuildEmit::Plan => base.with_extension("build-plan.json"),
    }
}

fn default_native_output_path(binary_name: &str, executable: &Path) -> PathBuf {
    let artifact_name = executable
        .file_name()
        .filter(|name| name.to_string_lossy().starts_with(binary_name))
        .unwrap_or_else(|| std::ffi::OsStr::new(binary_name));
    PathBuf::from("target/mech").join(artifact_name)
}

fn project_output_path(output: &Path) -> PathBuf {
    PathBuf::from(format!("{}.project", output.display()))
}

fn discover_source_roots(inputs: &[String]) -> MResult<Vec<PathBuf>> {
    let cwd = std::env::current_dir()?;
    let mut sources = Vec::new();
    let mut seen = BTreeSet::new();
    for input in inputs {
        let input_path = PathBuf::from(input);
        let root = if input_path.is_absolute() {
            input_path
        } else {
            cwd.join(input_path)
        };
        let base = if root.is_dir() {
            root.clone()
        } else {
            root.parent().unwrap_or(&cwd).to_path_buf()
        };
        let mut discovered = collect_sources(
            &[root],
            &base,
            DiscoveryOptions {
                allowed_file_extensions: BUILD_EXTENSIONS,
                recursive_file_extensions: BUILD_EXTENSIONS,
                skip_dir_names: BUILD_SKIP_DIRECTORIES,
                follow_file_symlinks: true,
                follow_dir_symlinks: false,
                missing_path_policy: MissingPathPolicy::Error,
            },
        )?
        .into_iter()
        .map(|entry| entry.logical_path)
        .collect::<Vec<_>>();
        discovered.sort();
        for source in discovered {
            if seen.insert(source.clone()) {
                sources.push(source);
            }
        }
    }
    if sources.is_empty() {
        return build_error("no Mech source files were found in the supplied build roots");
    }
    Ok(sources)
}

fn enforce_production_build_source_shape(source_roots: &[PathBuf]) -> MResult<()> {
    if source_roots.len() > 1 {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::MultipleRootsUnsupported,
                reason: "production builds accept exactly one resident program root".to_string(),
            },
            None,
        ));
    }
    Ok(())
}

fn load_build_config(
    options: &BuildOptions,
    bytecode_path: Option<&Path>,
) -> MResult<Option<crate::LoadedMechConfig>> {
    if options.no_config {
        return Ok(None);
    }
    let current_dir = std::env::current_dir()?;
    if let Some(config) = options.config_path.as_deref() {
        let config = PathBuf::from(config);
        let config = if config.is_absolute() {
            config
        } else {
            current_dir.join(config)
        };
        return crate::load_mech_config_path(config, None).map(Some);
    }
    if let Some(bytecode_path) = bytecode_path {
        let path = bytecode_path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(mech_runtime::DEFAULT_CONFIG_FILENAME);
        return if path.is_file() {
            crate::load_mech_config_path(path, None).map(Some)
        } else {
            Ok(None)
        };
    }
    crate::load_optional_mech_config(&current_dir, None, false, &options.paths)
}

fn configured_hosts(config: Option<&crate::LoadedMechConfig>) -> Vec<HostInstanceConfig> {
    config
        .map(|loaded| loaded.document.hosts.clone())
        .unwrap_or_default()
}

fn configured_run_grants(config: Option<&crate::LoadedMechConfig>) -> Vec<RunResourceGrantConfig> {
    config
        .and_then(|loaded| loaded.document.run.as_ref())
        .map(|run| run.grants.clone())
        .unwrap_or_default()
}

fn validate_production_build_config(
    config: Option<&crate::LoadedMechConfig>,
    binary_name: &str,
) -> MResult<()> {
    let runtime = match config {
        Some(config) => crate::apply_runtime_config_patch(
            RuntimeConfig::new(binary_name),
            &config.document.runtime,
        )?,
        None => RuntimeConfig::new(binary_name),
    };
    runtime.validate()?;
    if config
        .and_then(|config| config.document.build.as_ref())
        .and_then(|build| build.actor.as_ref())
        .is_some()
    {
        return Err(MechError::new(
            mech_runtime::ResidentRouteFailure {
                class: mech_runtime::ResidentRouteFailureClass::SemanticUnsupported,
                reason: "actor bootstrap is not part of the production resident program contract"
                    .to_string(),
            },
            None,
        ));
    }
    Ok(())
}

fn native_runtime_config(
    config: Option<&crate::LoadedMechConfig>,
    binary_name: &str,
    has_application_requirements: bool,
    needs_resource_config: bool,
) -> MResult<Option<NativeRuntimeConfig>> {
    let Some(config) = config else {
        if !has_application_requirements {
            return Ok(None);
        }
        if needs_resource_config {
            return Ok(None);
        }
        return Ok(Some(NativeRuntimeConfig {
            runtime: RuntimeConfig::new(binary_name),
            hosts: Vec::new(),
            run_grants: Vec::new(),
            actor_bootstrap: None,
        }));
    };
    let runtime = crate::apply_runtime_config_patch(
        RuntimeConfig::new(binary_name),
        &config.document.runtime,
    )?;
    let (hosts, run_grants) = if needs_resource_config {
        (
            configured_hosts(Some(config)),
            configured_run_grants(Some(config)),
        )
    } else {
        (Vec::new(), Vec::new())
    };
    Ok(Some(NativeRuntimeConfig {
        runtime,
        hosts,
        run_grants,
        actor_bootstrap: None,
    }))
}

fn copy_exact_file_bytes(bytes: &[u8], destination: &Path) -> MResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(destination, bytes)?;
    Ok(())
}

fn copy_exact_file(source: &Path, destination: &Path) -> MResult<()> {
    if let Some(parent) = destination.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::copy(source, destination)?;
    Ok(())
}

fn refuse_existing_project_output(destination: &Path) -> MResult<()> {
    if destination.exists() {
        return build_error(format!(
            "refusing to overwrite existing Cargo project output `{}`",
            destination.display()
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_outputs_match_the_authoritative_layout() {
        assert_eq!(
            default_output_path(BuildEmit::Native, "demo"),
            PathBuf::from("target/mech/demo")
        );
        assert_eq!(
            default_output_path(BuildEmit::Bytecode, "demo"),
            PathBuf::from("target/mech/demo.mecb")
        );
        assert_eq!(
            default_output_path(BuildEmit::Plan, "demo"),
            PathBuf::from("target/mech/demo.build-plan.json")
        );
        assert_eq!(
            default_output_path(BuildEmit::CargoProject, "demo"),
            PathBuf::from("target/mech/demo.cargo")
        );
    }

    #[test]
    fn native_default_uses_the_reported_artifact_suffix() {
        assert_eq!(
            default_native_output_path("demo", Path::new("target/triple/release/demo.exe")),
            PathBuf::from("target/mech/demo.exe")
        );
        assert_eq!(
            default_native_output_path("demo", Path::new("target/release/demo")),
            PathBuf::from("target/mech/demo")
        );
    }

    #[test]
    fn project_sidecar_is_beside_the_exact_output() {
        assert_eq!(
            project_output_path(Path::new("dist/demo.build-plan.json")),
            PathBuf::from("dist/demo.build-plan.json.project")
        );
    }

    #[test]
    fn inferred_names_are_deterministic() {
        assert_eq!(inferred_binary_name("src/demo.mec"), "demo");
        assert_eq!(inferred_binary_name("artifacts/demo.mecb"), "demo");
    }

    #[test]
    fn compute_region_admission_matches_build_product_capabilities() {
        assert!(validate_build_product_capabilities(true, BuildEmit::Bytecode, false).is_ok());
        for (emit, keep_project) in [
            (BuildEmit::Native, false),
            (BuildEmit::Native, true),
            (BuildEmit::CargoProject, false),
            (BuildEmit::Plan, false),
            (BuildEmit::Plan, true),
            (BuildEmit::Bytecode, true),
        ] {
            let error = validate_build_product_capabilities(true, emit, keep_project)
                .expect_err("native-package products must reject compute regions");
            let failure = error
                .kind_as::<mech_runtime::ResidentRouteFailure>()
                .unwrap();
            assert_eq!(
                failure.class,
                mech_runtime::ResidentRouteFailureClass::SemanticUnsupported
            );
        }
        assert!(validate_build_product_capabilities(false, BuildEmit::Native, false).is_ok());
    }
}
