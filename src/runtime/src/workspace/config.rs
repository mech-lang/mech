use super::*;

#[derive(Clone, Debug)]
pub struct RuntimeWorkspaceConfig {
  pub root: PathBuf,
  pub targets: Vec<RuntimeWorkspaceTarget>,
  pub folders: Vec<RuntimeWorkspaceFolder>,
  pub capability_subject: Option<String>,
}

impl RuntimeWorkspaceConfig {
  pub fn new(root: impl Into<PathBuf>) -> Self {
    Self {
      root: root.into(),
      targets: Vec::new(),
      folders: Vec::new(),
      capability_subject: None,
    }
  }

  pub fn capability_subject(mut self, subject: impl Into<String>) -> Self {
    self.capability_subject = Some(subject.into());
    self
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

  pub fn folder(
    mut self,
    specifier: impl Into<String>,
  ) -> Self {
    self.folders.push(RuntimeWorkspaceFolder {
      specifier: specifier.into(),
      recursive: true,
    });
    self
  }

  pub fn folder_recursive(
    mut self,
    specifier: impl Into<String>,
    recursive: bool,
  ) -> Self {
    self.folders.push(RuntimeWorkspaceFolder {
      specifier: specifier.into(),
      recursive,
    });
    self
  }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceTarget {
  pub name: String,
  pub specifier: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeWorkspaceFolder {
  pub specifier: String,
  pub recursive: bool,
}