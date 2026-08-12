use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use clap::{Arg, ArgAction, ArgMatches, Command};
use mech_build::{
    NativeActorBootstrap, NativeApplicationBuilder, NativeBuildEnvironment, NativeBuildProfile,
    NativeBuildRequest, NativeDependencySource, NativeEmit, NativeRuntimeConfig,
};
use mech_core::*;
use mech_runtime::{HostInstanceConfig, RunResourceGrantConfig, RuntimeConfig};

use crate::cli::module_execution::{execute_planning_source_module_roots, module_runtime_config};
use crate::cli::outcome::{CliOutcome, RootFlags};
use crate::source_discovery::{DedupePolicy, DiscoveryOptions, MissingPathPolicy, collect_sources};

const BUILD_EXTENSIONS: &[&str] = &["mec", "🤖", "mdoc", "mpkg"];
const BUILD_SKIP_DIRECTORIES: &[&str] = &[".git", "dist", "out", "target"];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum BuildEmit {
    Native,
    Bytecode,
    CargoProject,
    Plan,
    Mlir,
    Rust,
}

impl From<BuildEmit> for NativeEmit {
    fn from(value: BuildEmit) -> Self {
        match value {
            BuildEmit::Native => NativeEmit::Native,
            BuildEmit::Bytecode => NativeEmit::Bytecode,
            BuildEmit::CargoProject => NativeEmit::CargoProject,
            BuildEmit::Plan => NativeEmit::Plan,
            BuildEmit::Mlir | BuildEmit::Rust => {
                unreachable!("source emit returns before native planning")
            }
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
        .about("Build a Mech program as a native application, bytecode, plan, Cargo project, Rust kernel, or MLIR module.")
        .arg(
            Arg::new("mech_build_file_paths")
                .help("One or more source roots, or exactly one .mecb bytecode file")
                .required(false)
                .action(ArgAction::Append),
        )
        .arg(
            Arg::new("build_emit")
                .long("emit")
                .value_name("EMIT")
                .value_parser(["native", "bytecode", "cargo-project", "plan", "mlir", "rust"])
                .default_value("native")
                .help("Artifact to emit: native, bytecode, cargo-project, plan, Rust kernel, or MLIR source."),
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
                .help("Cargo target triple, an MLIR accelerator target, or cpu:f32 for a relaxed Rust kernel."),
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
            Arg::new("build_aot")
                .long("aot")
                .action(ArgAction::SetTrue)
                .help("Compile a fully supported numeric turn into the generated executable."),
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
    pub aot: bool,
    pub config_path: Option<String>,
    pub no_config: bool,
    pub workspace_root: Option<PathBuf>,
    pub keep_project: bool,
    pub offline: bool,
    pub debug: bool,
    pub trace: bool,
    pub time: bool,
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
            aot: matches.get_flag("build_aot"),
            config_path: root_matches.get_one::<String>("config").cloned(),
            no_config: root_matches.get_flag("no_config"),
            workspace_root: matches
                .get_one::<String>("workspace_root")
                .map(PathBuf::from),
            keep_project: matches.get_flag("keep_project"),
            offline: matches.get_flag("offline"),
            debug: root.debug,
            trace: root.trace,
            time: root.time,
            rounds_per_step: root.rounds_per_step.unwrap_or(10_000),
        };
        if options.emit == BuildEmit::CargoProject && options.keep_project {
            return build_error("`--emit cargo-project` cannot be combined with `--keep-project`");
        }
        if matches!(options.emit, BuildEmit::Mlir | BuildEmit::Rust) {
            if !options.aot {
                return build_error("numeric source emission requires `--aot`");
            }
            if options.keep_project {
                return build_error(
                    "numeric source emission cannot be combined with `--keep-project`",
                );
            }
            match options.emit {
                BuildEmit::Mlir => {
                    if let Some(target) = options.target.as_deref() {
                        validate_mlir_target(target)?;
                    }
                }
                BuildEmit::Rust => validate_rust_target(options.target.as_deref())?,
                _ => unreachable!(),
            }
        }
        if options.emit != BuildEmit::Bytecode || options.keep_project {
            if let Some(name) = options.name.as_deref() {
                mech_build::validate_project_binary_name(name)?;
            }
            if !matches!(options.emit, BuildEmit::Mlir | BuildEmit::Rust)
                && let Some(target) = options.target.as_deref()
            {
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
        ParsedProgram::from_bytes(&bytecode)?;
        let config = load_build_config(&options, Some(&path))?;
        (bytecode, config)
    } else {
        let loaded_config = load_build_config(&options, None)?;
        let source_roots = discover_source_roots(&options.paths)?;
        let configured_hosts = configured_hosts(loaded_config.as_ref());
        let run_grants = configured_run_grants(loaded_config.as_ref());
        let planner_config = module_runtime_config(
            format!("{binary_name}-planner"),
            options.debug,
            options.trace,
            options.time,
            options.rounds_per_step,
        )?;
        let mut runtime = execute_planning_source_module_roots(
            planner_config,
            &configured_hosts,
            &run_grants,
            loaded_config
                .as_ref()
                .and_then(|config| config.document.build.as_ref())
                .and_then(|build| build.actor.as_ref()),
            &source_roots,
        )?;
        (runtime.compile_program_bytecode()?, loaded_config)
    };

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

    if options.emit == BuildEmit::Mlir {
        let catalog = mech_stdlib::source_catalog();
        let program = match options.target.as_deref() {
            Some(target) if is_nvidia_mlir_target(target) => {
                mech_build::aot::lower_bytecode_mlir_gpu(&bytecode, &catalog)
            }
            Some("apple:metal-f32") => {
                mech_build::aot::lower_bytecode_mlir_spirv_f32(&bytecode, &catalog)
            }
            Some(target) => unreachable!("validated MLIR target `{target}`"),
            None => mech_build::aot::lower_bytecode_mlir(&bytecode, &catalog),
        }
        .map_err(|reason| {
            MechError::new(
                GenericError {
                    msg: format!("MLIR numeric lowering rejected the program: {reason}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        copy_exact_file_bytes(program.source.as_bytes(), &requested_output)?;
        println!(
            "[Output] Mech MLIR written to: {}",
            requested_output.display()
        );
        return Ok(CliOutcome::success());
    }

    if options.emit == BuildEmit::Rust {
        let catalog = mech_stdlib::source_catalog();
        let program = match options.target.as_deref() {
            Some("cpu:f32") => mech_build::aot::lower_bytecode_rust_f32(&bytecode, &catalog),
            None => mech_build::aot::lower_bytecode(&bytecode, &catalog),
            Some(target) => unreachable!("validated Rust target `{target}`"),
        }
        .map_err(|reason| {
            MechError::new(
                GenericError {
                    msg: format!("Rust numeric lowering rejected the program: {reason}"),
                },
                None,
            )
            .with_compiler_loc()
        })?;
        copy_exact_file_bytes(program.source.as_bytes(), &requested_output)?;
        println!(
            "[Output] Mech Rust kernel written to: {}",
            requested_output.display()
        );
        return Ok(CliOutcome::success());
    }

    let parsed = ParsedProgram::from_bytes(&bytecode)?;
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
        // AOT lowering reactivates the semantic artifact to recover typed
        // resident layouts and initial state. The restricted native planning
        // catalog contains contract metadata but intentionally omits some
        // executable factories needed for that activation.
        function_catalog: if options.aot {
            mech_stdlib::source_catalog()
        } else {
            mech_stdlib::native_plan_catalog()
        },
        host_catalog: mech_build::selected_native_host_catalog()?,
        dependency_source,
    };
    let request = NativeBuildRequest {
        bytecode,
        aot: options.aot,
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
        BuildEmit::Mlir | BuildEmit::Rust => {
            unreachable!("numeric source returns before native planning")
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

fn parse_emit(value: &str) -> MResult<BuildEmit> {
    match value {
        "native" => Ok(BuildEmit::Native),
        "bytecode" => Ok(BuildEmit::Bytecode),
        "cargo-project" => Ok(BuildEmit::CargoProject),
        "plan" => Ok(BuildEmit::Plan),
        "mlir" => Ok(BuildEmit::Mlir),
        "rust" => Ok(BuildEmit::Rust),
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

fn is_nvidia_mlir_target(target: &str) -> bool {
    target
        .strip_prefix("nvidia:sm_")
        .is_some_and(|arch| !arch.is_empty() && arch.bytes().all(|byte| byte.is_ascii_digit()))
}

fn validate_mlir_target(target: &str) -> MResult<()> {
    if is_nvidia_mlir_target(target) || target == "apple:metal-f32" {
        Ok(())
    } else {
        build_error(format!(
            "unsupported MLIR target `{target}`; omit `--target` for CPU MLIR, use `nvidia:sm_<compute-capability>`, or use `apple:metal-f32`"
        ))
    }
}

fn validate_rust_target(target: Option<&str>) -> MResult<()> {
    match target {
        None | Some("cpu:f32") => Ok(()),
        Some(target) => build_error(format!(
            "unsupported Rust kernel target `{target}`; omit `--target` for f64 or use `cpu:f32`"
        )),
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
        BuildEmit::Mlir => base.with_extension("mlir"),
        BuildEmit::Rust => base.with_extension("rs"),
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
                dedupe_policy: DedupePolicy::LogicalPath,
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
        actor_bootstrap: config
            .document
            .build
            .as_ref()
            .and_then(|build| build.actor.as_ref())
            .map(|actor| NativeActorBootstrap {
                subject: actor.subject.clone(),
                message_kind: actor.message_kind.clone(),
                message_payload: actor.message_payload.clone(),
                initial_state: actor.initial_state.clone(),
            }),
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
            default_output_path(BuildEmit::Mlir, "demo"),
            PathBuf::from("target/mech/demo.mlir")
        );
        assert_eq!(
            default_output_path(BuildEmit::Rust, "demo"),
            PathBuf::from("target/mech/demo.rs")
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
    fn aot_is_an_explicit_opt_in() {
        let default = command()
            .try_get_matches_from(["build", "demo.mec"])
            .unwrap();
        assert!(!default.get_flag("build_aot"));
        let enabled = command()
            .try_get_matches_from(["build", "--aot", "demo.mec"])
            .unwrap();
        assert!(enabled.get_flag("build_aot"));
    }

    #[test]
    fn mlir_is_an_explicit_emit_kind() {
        let matches = command()
            .try_get_matches_from(["build", "--emit", "mlir", "demo.mec"])
            .unwrap();
        assert_eq!(
            parse_emit(matches.get_one::<String>("build_emit").unwrap()).unwrap(),
            BuildEmit::Mlir
        );
        assert!(!matches.get_flag("build_aot"));
    }

    #[test]
    fn rust_is_an_explicit_emit_kind_with_an_optional_f32_target() {
        let matches = command()
            .try_get_matches_from([
                "build", "--aot", "--emit", "rust", "--target", "cpu:f32", "demo.mec",
            ])
            .unwrap();
        assert_eq!(
            parse_emit(matches.get_one::<String>("build_emit").unwrap()).unwrap(),
            BuildEmit::Rust
        );
        assert!(validate_rust_target(Some("cpu:f32")).is_ok());
        assert!(validate_rust_target(None).is_ok());
        assert!(validate_rust_target(Some("apple:metal-f32")).is_err());
    }

    #[test]
    fn nvidia_mlir_targets_are_explicit_and_strict() {
        assert!(is_nvidia_mlir_target("nvidia:sm_86"));
        assert!(is_nvidia_mlir_target("nvidia:sm_120"));
        assert!(!is_nvidia_mlir_target("nvidia:compute_86"));
        assert!(!is_nvidia_mlir_target("nvidia:sm_"));
        assert!(validate_mlir_target("nvidia:sm_86").is_ok());
        assert!(validate_mlir_target("apple:metal-f32").is_ok());
        assert!(validate_mlir_target("x86_64-unknown-linux-gnu").is_err());
    }
}
