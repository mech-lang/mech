use mech_core::{MResult, MechError};
use crate::InvalidConfigField;

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RunResourceGrantConfig {
  pub target: String,
  pub operations: Vec<String>,
  pub paths: Vec<String>,
}

pub fn validate_run_resource_grant(grant: &RunResourceGrantConfig) -> MResult<()> {
  super::parse_host_context_target(&grant.target)?;
  if grant.operations.is_empty() { return invalid("run.grants[].operations must contain at least one operation"); }
  for op in &grant.operations { if op != "read" && op != "write" { return invalid(format!("unknown run grant operation `{op}`")); } }
  if grant.paths.is_empty() { return invalid("run.grants[].paths must contain at least one path"); }
  for path in &grant.paths { validate_grant_path(path)?; }
  Ok(())
}

pub fn validate_grant_path(path: &str) -> MResult<()> {
  if path.is_empty() { return invalid("run grant paths must be non-empty"); }
  if path == "*" { return Ok(()); }
  if path.contains('*') && !path.ends_with("/*") { return invalid(format!("invalid wildcard placement in grant path `{path}`")); }
  if path.matches('*').count() > 1 { return invalid(format!("invalid wildcard placement in grant path `{path}`")); }
  Ok(())
}

fn invalid<T>(message: impl Into<String>) -> MResult<T> { Err(MechError::new(InvalidConfigField::new(message), None)) }
