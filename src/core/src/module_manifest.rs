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

impl ModuleManifestConfig {
  pub fn validate(&self) -> MResult<()> {
    if self.name.trim().is_empty() {
      return Err(MechError::new(GenericError { msg: "Module manifest name must not be empty".to_string() }, None).with_compiler_loc());
    }
    let mut names = std::collections::HashSet::new();
    for export in &self.exports {
      if export.name.trim().is_empty() {
        return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` has an empty export name", self.name) }, None).with_compiler_loc());
      }
      if !names.insert(export.name.clone()) {
        return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` declares duplicate export `{}`", self.name, export.name) }, None).with_compiler_loc());
      }
      if !export.base_uri.contains("://") {
        return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` export `{}` has invalid base_uri `{}`", self.name, export.name, export.base_uri) }, None).with_compiler_loc());
      }
      if export.operations.is_empty() {
        return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` export `{}` must declare at least one operation", self.name, export.name) }, None).with_compiler_loc());
      }
      match export.kind {
        ModuleManifestExportKind::Context => {
          for operation in &export.operations {
            if operation != "read" && operation != "write" {
              return Err(MechError::new(GenericError { msg: format!("Module manifest `{}` context export `{}` has unsupported operation `{}`", self.name, export.name, operation) }, None).with_compiler_loc());
            }
          }
        }
      }
    }
    Ok(())
  }
}

impl ModuleManifestCatalog {
  pub fn new() -> Self { Self::default() }

  pub fn with_builtin_browser() -> Self {
    let mut catalog = Self::new();
    let _ = catalog.register(builtin_browser_module_manifest());
    catalog
  }

  pub fn register(&mut self, manifest: ModuleManifestConfig) -> MResult<()> {
    manifest.validate()?;
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
