use super::*;

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceConfig {
  pub root: PathBuf,
  pub targets: Vec<RuntimeWorkspaceTarget>,
}

impl RuntimeWorkspaceConfig {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      root: root.into(),
      targets: Vec::new(),
    }
  }

  pub fn target(
    mut self,
    name: impl Into<String>,
    specifier: impl Into<String>,
  ) -> Self {
    self.targets.push(RuntimeWorkspaceTarget {
      name: name.into(),
      specifier: specifier.into(),
    });
    self
  }
}


#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceTarget {
  pub name: String,
  pub specifier: String,
}