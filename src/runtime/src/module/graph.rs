use std::collections::{HashMap, HashSet};

use crate::id::ModuleVersionId;

#[derive(Debug, Default, Clone)]
pub struct ModuleDependencyGraph {
  stack: Vec<String>,
  seen: HashSet<String>,
  cache: HashMap<String, ModuleVersionId>,
}

impl ModuleDependencyGraph {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn cached_version(&self, canonical_uri: &str) -> Option<ModuleVersionId> {
    self.cache.get(canonical_uri).copied()
  }

  pub fn enter(&mut self, canonical_uri: &str) -> Option<Vec<String>> {
    if self.seen.contains(canonical_uri) {
      let mut cycle = self.stack.clone();
      cycle.push(canonical_uri.to_string());
      return Some(cycle);
    }

    self.seen.insert(canonical_uri.to_string());
    self.stack.push(canonical_uri.to_string());
    None
  }

  pub fn leave(&mut self, canonical_uri: &str) {
    let popped = self.stack.pop();

    debug_assert_eq!(
      popped.as_deref(),
      Some(canonical_uri),
      "module dependency graph leave order mismatch",
    );

    self.seen.remove(canonical_uri);
  }

  pub fn cache_version(&mut self, canonical_uri: &str, module_version: ModuleVersionId) {
    self.cache.insert(canonical_uri.to_string(), module_version);
  }
}
