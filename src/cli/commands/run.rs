use std::path::{Path, PathBuf};

use clap::ArgMatches;
use colored::*;
use mech_core::*;
use mech_program::*;
use mech_runtime::{RuntimeConfig, FS_LIST, FS_READ, MECH_TOOL_SUBJECT};

use crate::generate_uuid;
use crate::cli::capabilities;
use crate::cli::config;
use crate::cli::paths::source_extension;
use crate::cli::source_discovery::{
  collect_sources,
  DedupePolicy,
  DiscoveryOptions,
  MissingPathPolicy,
};
use crate::cli::run::{
  classify_run_inputs,
  cli_host_capability_passthrough_values,
  cli_host_capability_selection,
  effective_run_runtime_config,
  new_cli_runtime,
  run_cli_source,
  run_cli_source_code,
  RunInputMode,
};

const RUN_EXTENSIONS: &[&str] = &["mec", "🤖", "mecb", "mdoc", "mpkg", "m", "csv", "js"];
const RUN_DIRECTORY_EXTENSIONS: &[&str] = &["mec", "🤖", "mdoc", "mpkg"];
const SKIP_SOURCE_DIRS: &[&str] = &["target", ".git", "dist", "out"];

pub(crate) struct RunRootFlags {
  pub debug: bool,
  pub trace: bool,
  pub time: bool,
  pub repl: bool,
  pub root_rounds_per_step: Option<usize>,
}

pub(crate) struct RunCommandOutcome {
  pub repl_flag: bool,
  pub repl_runtime_config: Option<RuntimeConfig>,
  #[cfg(all(feature = "run", feature = "repl"))]
  pub repl_seed_program: Option<MechProgram>,
}

fn skip_directory_run_source(path: &Path) -> bool {
  matches!(source_extension(path).as_deref(), Some("mecb"))
}

pub(crate) fn collect_run_targets(path: &Path) -> MResult<Vec<PathBuf>> {
  let mut ids = mech_runtime::DefaultIdGenerator::new();
  let mut authority = mech_runtime::HostFilesystemAuthority::new(MECH_TOOL_SUBJECT, mech_runtime::SharedCapabilityKernel::new());
  let root = if path.is_dir() { path } else { path.parent().unwrap_or_else(|| Path::new(".")) };
  authority.grant_path(&mut ids, root, true, [FS_READ, FS_LIST])?;
  collect_run_targets_with_capabilities(path, authority.kernel())
}

fn collect_run_targets_with_capabilities(path: &Path, kernel: &mech_runtime::SharedCapabilityKernel) -> MResult<Vec<PathBuf>> {
  if path.is_file() {
    let mut kernel = kernel.clone();
    mech_runtime::check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_READ, path)?;
  } else if path.is_dir() {
    let mut kernel = kernel.clone();
    mech_runtime::check_fs_capability(&mut kernel, MECH_TOOL_SUBJECT, FS_LIST, path)?;
  }
  let entries = collect_sources(
    &[path.to_path_buf()],
    path,
    DiscoveryOptions {
      allowed_file_extensions: RUN_EXTENSIONS,
      recursive_file_extensions: RUN_DIRECTORY_EXTENSIONS,
      skip_dir_names: SKIP_SOURCE_DIRS,
      follow_file_symlinks: true,
      follow_dir_symlinks: false,
      missing_path_policy: MissingPathPolicy::SkipBrokenSymlink,
      dedupe_policy: DedupePolicy::LogicalPath,
    },
  )?;
  let mut out = entries.into_iter().map(|entry| entry.logical_path).collect::<Vec<_>>();
  out.sort();
  Ok(out)
}

pub(crate) fn run(
  root_matches: &ArgMatches,
  run_matches: Option<&ArgMatches>,
  root_flags: RunRootFlags,
) -> MResult<RunCommandOutcome> {
  let uuid = generate_uuid();
  let explicit_run_command = run_matches.is_some();
  let mut run_inputs: Vec<String> = if let Some(run_matches) = run_matches {
    run_matches
      .get_many::<String>("mech_run_paths")
      .map_or(vec![], |files| files.map(|file| file.to_string()).collect())
  } else if let Some(m) = root_matches.get_many::<String>("mech_paths") {
    m.map(|s| s.to_string()).collect()
  } else {
    vec![]
  };
  run_inputs.extend(cli_host_capability_passthrough_values(root_matches, run_matches));

  let run_debug_flag = root_flags.debug || run_matches.map(|m| m.get_flag("debug")).unwrap_or(false);
  let run_trace_flag = root_flags.trace || run_matches.map(|m| m.get_flag("trace")).unwrap_or(false);
  let run_time_flag = root_flags.time || run_matches.map(|m| m.get_flag("time")).unwrap_or(false);
  let run_rounds_per_step = run_matches
    .and_then(|m| m.get_one::<String>("rounds-per-step"))
    .and_then(|s| s.parse::<usize>().ok())
    .or(root_flags.root_rounds_per_step);

  let run_input_mode = classify_run_inputs(run_inputs);
  let config_matches = run_matches.unwrap_or(root_matches);
  let config_inputs: Vec<String> = match &run_input_mode {
    RunInputMode::Paths(paths) => paths.clone(),
    RunInputMode::Empty | RunInputMode::InlineSource(_) => Vec::new(),
  };
  let loaded_config = config::load_run_cli_config(config_matches, &config_inputs)?;

  let runtime_config = effective_run_runtime_config(
    loaded_config.as_ref(),
    format!("program-{}", uuid),
    run_debug_flag,
    run_trace_flag,
    run_time_flag,
    run_rounds_per_step,
  )?;
  let repl_runtime_config = Some(runtime_config.clone());

  let cli_capability_selection = cli_host_capability_selection(root_matches, run_matches);
  let cli_grants = config::effective_cli_host_grants(
    loaded_config.as_ref(),
    cli_capability_selection,
  )?;

  let configured_hosts = loaded_config
    .as_ref()
    .map(|loaded| loaded.document.hosts.as_slice())
    .unwrap_or(&[]);

  let configured_run_grants = loaded_config
    .as_ref()
    .and_then(|loaded| loaded.document.run.as_ref())
    .map(|run| run.grants.as_slice())
    .unwrap_or(&[]);

  let badge = "[Mech Run]".truecolor(34, 204, 187);
  let mut fs_access = capabilities::build_filesystem_runtime_access(
    config_matches,
    loaded_config.as_ref(),
    &badge,
  )?;

  let mut runtime = new_cli_runtime(runtime_config, &cli_grants, configured_hosts, configured_run_grants)?;
  capabilities::install_file_resolver(&mut runtime, &fs_access, &std::env::current_dir()?)?;

  if let RunInputMode::InlineSource(source) = &run_input_mode {
    match run_cli_source(&mut runtime, source.trim()) {
      Ok(r) => {
        println!("{}", r.kind());
        #[cfg(feature = "pretty_print")]
        println!("{}", r.pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", r);
        std::process::exit(0);
      }
      Err(err) => {
        println!("{} {:#?}",
          "[Error]".truecolor(246,98,78),
          err
        );
        std::process::exit(1);
      }
    }
  }

  let run_paths = match run_input_mode {
    RunInputMode::Paths(paths) => paths,
    RunInputMode::Empty => Vec::new(),
    RunInputMode::InlineSource(_) => unreachable!("inline source exits before path execution"),
  };

  let options = config::effective_run_options(
    run_paths,
    loaded_config.as_ref(),
    explicit_run_command,
  )?;

  let missing_run_options = options.is_none();
  let result: MResult<Value> = if let Some(options) = options {
    if !config_matches.get_flag("no_default_capabilities") {
      let mut ids = mech_runtime::DefaultIdGenerator::new();
      for p in &options.paths {
        let path = Path::new(p);
        let grant_path = if path.is_dir() { path } else { path.parent().unwrap_or_else(|| Path::new(".")) };
        fs_access.authority.grant_path(&mut ids, grant_path, true, [FS_READ, FS_LIST, mech_runtime::FS_RESOLVE, mech_runtime::FS_IMPORT])?;
      }
    }
    let fs_kernel = fs_access.authority.kernel().clone();
    fs_access.kernel = fs_kernel.clone();
    capabilities::install_file_resolver(&mut runtime, &fs_access, &std::env::current_dir()?)?;
    let mut last = Value::Empty;
    for p in &options.paths {
      for target in collect_run_targets_with_capabilities(Path::new(p), &fs_kernel)? {
        let src = mech_runtime::read_runtime_source_file_with_capabilities(&target, Some(&fs_kernel), Some(MECH_TOOL_SUBJECT))?;
        last = run_cli_source_code(&mut runtime, &src)?;
      }
    }
    Ok(last)
  } else {
    Ok(Value::Empty)
  };

  let repl_flag = root_flags.repl || missing_run_options;
  match &result {
    Ok(r) if repl_flag => {
      #[cfg(all(feature = "run", feature = "repl"))]
      {
        return Ok(RunCommandOutcome {
          repl_flag,
          repl_runtime_config,
          repl_seed_program: Some(runtime.take_program()),
        });
      }
      #[cfg(not(feature = "repl"))]
      {
        println!("{}", r.kind());
        #[cfg(feature = "pretty_print")]
        println!("{}", r.pretty_print());
        #[cfg(not(feature = "pretty_print"))]
        println!("{:#?}", r);
        std::process::exit(0);
      }
    }
    Ok(r) => {
      println!("{}", r.kind());
      #[cfg(feature = "pretty_print")]
      println!("{}", r.pretty_print());
      #[cfg(not(feature = "pretty_print"))]
      println!("{:#?}", r);
      std::process::exit(0);
    }
    Err(err) => {
      crate::cli::app::print_mech_error(err);
      std::process::exit(1);
    }
  }

  #[allow(unreachable_code)]
  Ok(RunCommandOutcome {
    repl_flag,
    repl_runtime_config,
    #[cfg(all(feature = "run", feature = "repl"))]
    repl_seed_program: None,
  })
}
