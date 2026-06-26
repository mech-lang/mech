use mech_core::{MResult, MechError};
use crate::InvalidConfigField;
use super::catalog::{MaterializedHostContext, MaterializedHostInterface};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostManifestConfig {
  pub provider: String,
  pub contexts: Vec<HostContextManifest>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HostContextManifest {
  pub name: String,
  pub base_uri_template: String,
  pub operations: Vec<String>,
}

pub fn validate_host_manifest(manifest: &HostManifestConfig) -> MResult<()> {
  if manifest.provider.trim().is_empty() { return invalid("host.provider must be non-empty"); }
  let mut names = std::collections::BTreeSet::new();
  for context in &manifest.contexts {
    if context.name.trim().is_empty() { return invalid("host.contexts[].name must be non-empty"); }
    if !names.insert(context.name.clone()) { return invalid(format!("duplicate host context `{}`", context.name)); }
    if context.base_uri_template.trim().is_empty() || !context.base_uri_template.contains("://") { return invalid(format!("host context `{}` base-uri must contain `://`", context.name)); }
    if context.operations.is_empty() { return invalid(format!("host context `{}` operations must contain at least one operation", context.name)); }
    for op in &context.operations {
      if op != "read" && op != "write" { return invalid(format!("unknown host context operation `{op}`")); }
    }
  }
  Ok(())
}

pub fn materialize_host_manifest(instance: &str, manifest: &HostManifestConfig) -> MResult<MaterializedHostInterface> {
  validate_host_manifest(manifest)?;
  Ok(MaterializedHostInterface {
    instance: instance.to_string(),
    provider: manifest.provider.clone(),
    contexts: manifest.contexts.iter().map(|context| MaterializedHostContext {
      name: context.name.clone(),
      base_uri: context.base_uri_template.replace("{instance}", instance),
      operations: context.operations.clone(),
    }).collect(),
  })
}

fn invalid<T>(message: impl Into<String>) -> MResult<T> {
  Err(MechError::new(InvalidConfigField::new(message), None))
}
