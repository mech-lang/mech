use std::collections::{HashMap, HashSet};

use mech_core::{MResult, MechError, MechErrorKind};

use crate::ModuleVersionId;

#[derive(Clone, Debug, Default)]
pub struct ModuleGraphBuildState {
  stack: Vec<String>,
  seen: HashSet<String>,
  cache: HashMap<String, ModuleVersionId>,
}

impl ModuleGraphBuildState {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn cached(&self, canonical_uri: &str) -> Option<ModuleVersionId> {
    self.cache.get(canonical_uri).copied()
  }

  pub fn cache(&mut self, canonical_uri: impl Into<String>, module_version: ModuleVersionId) {
    self.cache.insert(canonical_uri.into(), module_version);
  }

  pub fn enter(&mut self, canonical_uri: impl Into<String>) -> MResult<()> {
    let canonical_uri = canonical_uri.into();

    if self.seen.contains(&canonical_uri) {
      let mut cycle = self.stack.clone();
      cycle.push(canonical_uri);

      return Err(MechError::new(ModuleDependencyCycleError { cycle }, None));
    }

    self.seen.insert(canonical_uri.clone());
    self.stack.push(canonical_uri);

    Ok(())
  }

  pub fn exit(&mut self, canonical_uri: &str) {
    self.stack.pop();
    self.seen.remove(canonical_uri);
  }
}

#[derive(Debug, Clone)]
pub struct ModuleDependencyCycleError {
  pub cycle: Vec<String>,
}

impl MechErrorKind for ModuleDependencyCycleError {
  fn name(&self) -> &str {
    "ModuleDependencyCycle"
  }

  fn message(&self) -> String {
    format!("module dependency cycle detected: {}", self.cycle.join(" -> "))
  }
}
