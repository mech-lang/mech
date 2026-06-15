#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::{GenericError, MResult, MechError, MechErrorKind};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleManifestConfig {
  pub name: String,
  pub exports: Vec<ModuleManifestExportConfig>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleManifestExportConfig {
  pub name: String,
  pub kind: ModuleManifestExportKind,
  pub base_uri: String,
  pub operations: Vec<String>,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ModuleManifestExportKind {
  Context,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ModuleManifestCatalog {
  manifests: Vec<ModuleManifestConfig>,
}

impl ModuleManifestCatalog {
  pub fn new() -> Self { Self::default() }

  pub fn with_builtin_browser() -> Self {
    let mut catalog = Self::new();
    let _ = catalog.register(builtin_browser_module_manifest());
    catalog
  }

  pub fn register(&mut self, manifest: ModuleManifestConfig) -> MResult<()> {
    if self.manifest(&manifest.name).is_some() {
      return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` is already registered", manifest.name) }, None).with_compiler_loc());
    }
    self.manifests.push(manifest);
    Ok(())
  }

  pub fn manifest(&self, module: &str) -> Option<&ModuleManifestConfig> {
    self.manifests.iter().find(|manifest| manifest.name == module)
  }

  pub fn export(&self, module: &str, item: &str) -> Option<&ModuleManifestExportConfig> {
    self.manifest(module)?.exports.iter().find(|export| export.name == item)
  }

  pub fn context_export(&self, module: &str, item: &str) -> MResult<&ModuleManifestExportConfig> {
    let manifest = self.manifest(module).ok_or_else(|| MechError::new(GenericError { msg: format!("Module manifest `{module}` is not registered") }, None).with_compiler_loc())?;
    let export = manifest.exports.iter().find(|export| export.name == item).ok_or_else(|| MechError::new(GenericError { msg: format!("Module export `{module}/{item}` is not declared in its manifest") }, None).with_compiler_loc())?;
    match export.kind {
      ModuleManifestExportKind::Context => Ok(export),
    }
  }
}

pub fn builtin_browser_module_manifest() -> ModuleManifestConfig {
  ModuleManifestConfig {
    name: "browser".to_string(),
    exports: vec![ModuleManifestExportConfig {
      name: "dom".to_string(),
      kind: ModuleManifestExportKind::Context,
      base_uri: "browser://dom".to_string(),
      operations: vec!["read".to_string(), "write".to_string()],
    }],
  }
}
