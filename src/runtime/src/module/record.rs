use mech_core::MechSourceCode;

use crate::{
  ModuleId,
  ModuleVersionId,
  SourceExportDeclaration,
  SourceKind,
  CapabilityRequest
};

#[derive(Clone, Debug)]
pub struct RuntimeModuleRecord {
  pub module_id: ModuleId,
  pub module_version: ModuleVersionId,
  pub name: String,
  pub canonical_uri: String,
  pub kind: SourceKind,
  pub source: MechSourceCode,
  pub compiler_version: String,
  pub language_edition: String,
  pub target: String,
  pub feature_flags: Vec<String>,
  pub exports: Vec<SourceExportDeclaration>,
  pub dependency_versions: Vec<ModuleVersionId>,
  pub capability_requirements: Vec<CapabilityRequest>,
  pub capability_requirement_keys: Vec<String>,
}

impl RuntimeModuleRecord {
  pub fn new(
    module_id: ModuleId,
    module_version: ModuleVersionId,
    name: impl Into<String>,
    canonical_uri: impl Into<String>,
    kind: SourceKind,
    source: MechSourceCode,
    compiler_version: impl Into<String>,
    language_edition: impl Into<String>,
    target: impl Into<String>,
    feature_flags: Vec<String>,
    exports: Vec<SourceExportDeclaration>,
    dependency_versions: Vec<ModuleVersionId>,
    capability_requirements: Vec<CapabilityRequest>,
    capability_requirement_keys: Vec<String>,
  ) -> Self {
    Self {
      module_id,
      module_version,
      name: name.into(),
      canonical_uri: canonical_uri.into(),
      kind,
      source,
      compiler_version: compiler_version.into(),
      language_edition: language_edition.into(),
      target: target.into(),
      feature_flags,
      exports,
      dependency_versions,
      capability_requirements,
      capability_requirement_keys,
    }
  }
}
