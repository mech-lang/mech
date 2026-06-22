use clap::{Arg, ArgAction};
use mech_core::*;
use mech_host_cli::{CliResourceProvider, StdCliBackend};
use mech_runtime::{
  MechRuntime, RuntimeBuilder, RuntimeCapabilityGrant, RuntimeCapabilityOperation, RuntimeConfig,
  RuntimeEvent, RuntimeEventKind,
};

use crate::cli::config;

pub fn new_cli_runtime(
  config: RuntimeConfig,
  cli_grants: &config::EffectiveCliHostGrants,
) -> MResult<MechRuntime> {
  let mut runtime = RuntimeBuilder::new()
    .config(config)
    .resource_provider(Box::new(CliResourceProvider::new(StdCliBackend)))
    .build()?;

  grant_cli_runner_capabilities(&mut runtime, cli_grants)?;

  Ok(runtime)
}

pub fn effective_run_runtime_config(
  loaded_config: Option<&crate::LoadedMechConfig>,
  name: String,
  debug_enabled: bool,
  trace_enabled: bool,
  profile_enabled: bool,
  rounds_per_step: Option<usize>,
) -> MResult<RuntimeConfig> {
  let default_runtime_patch = mech_runtime::RuntimeConfigPatch::default();

  let mut config = crate::apply_runtime_config_patch(
    RuntimeConfig::default(),
    loaded_config
      .as_ref()
      .map(|loaded| &loaded.document.runtime)
      .unwrap_or(&default_runtime_patch),
  )?;

  config.name = name;

  if debug_enabled {
    config.diagnostics.debug_enabled = true;
  }

  if trace_enabled {
    config.diagnostics.trace_enabled = true;
  }

  if profile_enabled {
    config.diagnostics.profile_enabled = true;
  }

  if let Some(rounds_per_step) = rounds_per_step {
    config.limits.max_steps_per_turn = Some(rounds_per_step as u64);
  }

  config.validate()?;
  Ok(config)
}

fn print_run_runtime_events(events: &[RuntimeEvent]) {
  for event in events {
    match &event.kind {
      RuntimeEventKind::ProgramProfiled { duration_ns, .. } => {
        println!("Cycle Time: {} ns", duration_ns);
      }
      _ => {}
    }
  }
}

pub fn run_cli_source(runtime: &mut MechRuntime, source: &str) -> MResult<Value> {
  let mut context = runtime.runtime_context()?;
  let result = runtime.run_string_with_context(&mut context, source);
  print_run_runtime_events(&context.events);
  result
}

fn grant_cli_runner_capabilities(
  runtime: &mut MechRuntime,
  grants: &config::EffectiveCliHostGrants,
) -> MResult<()> {
  let subject = runtime.runtime_context()?.subject;

  if !grants.env_read_paths.is_empty() {
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject: subject.clone(),
      resource: "cli://env".to_string(),
      operations: vec![RuntimeCapabilityOperation::Read],
      paths: grants.env_read_paths.clone(),
    })?;
  }

  if !grants.stdout_write_paths.is_empty() {
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject: subject.clone(),
      resource: "cli://stdout".to_string(),
      operations: vec![RuntimeCapabilityOperation::Write],
      paths: grants.stdout_write_paths.clone(),
    })?;
  }

  if !grants.stderr_write_paths.is_empty() {
    runtime.grant_capability(RuntimeCapabilityGrant {
      subject,
      resource: "cli://stderr".to_string(),
      operations: vec![RuntimeCapabilityOperation::Write],
      paths: grants.stderr_write_paths.clone(),
    })?;
  }

  Ok(())
}

pub fn cli_host_capability_args() -> Vec<Arg> {
  vec![
    Arg::new("deny_default_capabilities")
      .long("deny-default-capabilities")
      .help("Disable default CLI host capability profiles for this run")
      .global(true)
      .action(ArgAction::SetTrue),
    Arg::new("capabilities")
      .long("capabilities")
      .value_name("CAPABILITY")
      .help("Enable named CLI host capability profiles for this run, e.g. :cli/stdout")
      .global(true)
      .num_args(1..)
      .action(ArgAction::Append),
  ]
}

fn cli_host_capability_values(cli_matches: &clap::ArgMatches) -> Vec<String> {
  cli_matches
    .get_many::<String>("capabilities")
    .into_iter()
    .flatten()
    .cloned()
    .collect()
}

pub fn cli_host_capability_selection(
  cli_matches: &clap::ArgMatches,
  _run_matches: Option<&clap::ArgMatches>,
) -> config::CliHostCapabilitySelection {
  let deny_defaults = cli_matches.get_flag("deny_default_capabilities");

  let profiles = cli_host_capability_values(cli_matches)
    .into_iter()
    .filter(|value| value.starts_with(':'))
    .collect();

  config::CliHostCapabilitySelection {
    include_defaults: !deny_defaults,
    profiles,
  }
}

pub fn cli_host_capability_passthrough_values(
  cli_matches: &clap::ArgMatches,
  _run_matches: Option<&clap::ArgMatches>,
) -> Vec<String> {
  cli_host_capability_values(cli_matches)
    .into_iter()
    .filter(|value| !value.starts_with(':'))
    .collect()
}
