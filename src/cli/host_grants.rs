use mech_core::*;

use crate::LoadedMechConfig;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EffectiveCliHostGrants {
  pub env_read_paths: Vec<String>,
  pub stdout_write_paths: Vec<String>,
  pub stderr_write_paths: Vec<String>,
}

impl EffectiveCliHostGrants {
  pub fn empty() -> Self {
    Self {
      env_read_paths: Vec::new(),
      stdout_write_paths: Vec::new(),
      stderr_write_paths: Vec::new(),
    }
  }
}

impl Default for EffectiveCliHostGrants {
  fn default() -> Self {
    Self::empty()
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CliHostCapabilitySelection {
  pub include_defaults: bool,
  pub profiles: Vec<String>,
}

impl Default for CliHostCapabilitySelection {
  fn default() -> Self {
    Self {
      include_defaults: true,
      profiles: Vec::new(),
    }
  }
}

const CLI_PROFILE_ENV: &str = ":cli/env";
const CLI_PROFILE_STDOUT: &str = ":cli/stdout";
const CLI_PROFILE_STDERR: &str = ":cli/stderr";

const DEFAULT_CLI_CAPABILITY_PROFILES: &[&str] = &[
  CLI_PROFILE_ENV,
  CLI_PROFILE_STDOUT,
  CLI_PROFILE_STDERR,
];

pub fn effective_cli_host_grants(
  config: Option<&LoadedMechConfig>,
  selection: CliHostCapabilitySelection,
) -> MResult<EffectiveCliHostGrants> {
  let mut grants = EffectiveCliHostGrants::empty();
  let has_explicit_cli_config_grants = match config {
    Some(config) => has_explicit_cli_run_grants(config)?,
    None => false,
  };

  if selection.include_defaults && !has_explicit_cli_config_grants {
    for profile in DEFAULT_CLI_CAPABILITY_PROFILES {
      grant_cli_profile(&mut grants, profile)?;
    }
  }

  for profile in &selection.profiles {
    grant_cli_profile(&mut grants, profile)?;
  }

  Ok(grants)
}

pub fn has_explicit_cli_run_grants(config: &LoadedMechConfig) -> MResult<bool> {
  let Some(run) = config.document.run.as_ref() else {
    return Ok(false);
  };

  if !run.grants_specified {
    return Ok(false);
  }

  if run.grants.is_empty() {
    return Ok(true);
  }

  let mut cli_instances = std::collections::BTreeSet::from(["cli".to_string()]);
  for host in &config.document.hosts {
    if host.provider == "cli" {
      cli_instances.insert(host.name.clone());
    }
  }

  for grant in &run.grants {
    let (instance, context) = mech_runtime::parse_host_context_target(&grant.target)?;
    if cli_instances.contains(instance) && matches!(context, "env" | "stdout" | "stderr") {
      return Ok(true);
    }
  }

  Ok(false)
}

fn grant_cli_profile(
  grants: &mut EffectiveCliHostGrants,
  profile: &str,
) -> MResult<()> {
  match profile {
    CLI_PROFILE_ENV => {
      if !grants.env_read_paths.iter().any(|path| path == "*") {
        grants.env_read_paths.clear();
        grants.env_read_paths.push("*".to_string());
      }
      Ok(())
    }
    CLI_PROFILE_STDOUT => {
      union_string(&mut grants.stdout_write_paths, "text");
      union_string(&mut grants.stdout_write_paths, "line");
      Ok(())
    }
    CLI_PROFILE_STDERR => {
      union_string(&mut grants.stderr_write_paths, "text");
      union_string(&mut grants.stderr_write_paths, "line");
      Ok(())
    }
    other => Err(MechError::new(
      GenericError {
        msg: format!("unknown CLI capability profile `{other}`"),
      },
      None,
    ).with_compiler_loc()),
  }
}

fn union_string(paths: &mut Vec<String>, value: &str) {
  if !paths.iter().any(|path| path == value) {
    paths.push(value.to_string());
  }
}



