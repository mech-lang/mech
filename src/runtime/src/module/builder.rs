use mech_core::{MResult, MechError, MechErrorKind};

use crate::{
  module_id,
  module_version_id,
  ModuleVersionId,
  ResolvedSource,
  RuntimeModuleRecord,
  CapabilityRequest
};

#[derive(Clone, Debug, Default)]
pub struct ModuleBuilder;

#[derive(Clone, Copy, Debug)]
pub struct ModuleBuildOptions<'a> {
  pub compiler_version: &'a str,
  pub language_edition: &'a str,
  pub target: &'a str,
  pub feature_flags: &'a [&'a str],
  pub capability_requirements: &'a [&'a str],
}

impl<'a> ModuleBuildOptions<'a> {
  pub fn new(
    compiler_version: &'a str,
    language_edition: &'a str,
    target: &'a str,
    feature_flags: &'a [&'a str],
    capability_requirements: &'a [&'a str],
  ) -> Self {
    Self {
      compiler_version,
      language_edition,
      target,
      feature_flags,
      capability_requirements,
    }
  }
}

impl ModuleBuilder {
  pub fn new() -> Self {
    Self
  }

  pub fn build_resolved_source(
    &mut self,
    resolved: ResolvedSource,
    compiler_version: impl Into<String>,
    language_edition: impl Into<String>,
    target: impl Into<String>,
    feature_flags: &[String],
    dependency_versions: &[ModuleVersionId],
    capability_requirements: &[CapabilityRequest],
  ) -> MResult<RuntimeModuleRecord> {
    resolved.validate()?;

    if !resolved.is_executable_mech_source() {
      return Err(MechError::new(
        NonExecutableModuleSource {
          canonical_uri: resolved.canonical_uri.clone(),
        },
        None,
      ));
    }

    let compiler_version = compiler_version.into();
    let language_edition = language_edition.into();
    let target = target.into();

    let feature_flag_refs = feature_flags
      .iter()
      .map(|flag| flag.as_str())
      .collect::<Vec<_>>();

    let capability_requirement_keys = capability_requirements
      .iter()
      .map(|request| {
        format!(
          "{}:{}:{}",
          request.subject,
          request.operation,
          request.resource,
        )
      })
      .collect::<Vec<_>>();

    let capability_refs = capability_requirement_keys
      .iter()
      .map(|capability| capability.as_str())
      .collect::<Vec<_>>();

    let module_id = module_id(&resolved.canonical_uri);

    let module_version = module_version_id(
      &source_version_input(&resolved),
      &compiler_version,
      &language_edition,
      &target,
      &feature_flag_refs,
      dependency_versions,
      &capability_refs,
    );

    Ok(RuntimeModuleRecord::new(
      module_id,
      module_version,
      resolved.name,
      resolved.canonical_uri,
      resolved.kind,
      resolved.source,
      compiler_version,
      language_edition,
      target,
      feature_flags.to_vec(),
      resolved.exports,
      resolved.imports,
      dependency_versions.to_vec(),
      capability_requirements.to_vec(),
      capability_requirement_keys,
    ))
  }
}

fn source_version_input(resolved: &ResolvedSource) -> String {
  // For now this makes version identity depend on source content shape.
  // Later this should probably become a ContentHash over normalized source bytes.
  format!(
    "{:?}\nimports={:?}\nexports={:?}",
    resolved.source,
    resolved.imports,
    resolved.exports,
  )
}

#[derive(Debug, Clone)]
pub struct NonExecutableModuleSource {
  pub canonical_uri: String,
}

impl MechErrorKind for NonExecutableModuleSource {
  fn name(&self) -> &str {
    "NonExecutableModuleSource"
  }

  fn message(&self) -> String {
    format!(
      "Resolved source `{}` is not an executable Mech module source",
      self.canonical_uri,
    )
  }
}
