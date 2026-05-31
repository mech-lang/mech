use super::*;


#[derive(Debug, Clone)]
pub struct RuntimeWorkspaceInvalidConfig {
  pub reason: String,
}

impl MechErrorKind for RuntimeWorkspaceInvalidConfig {
  fn name(&self) -> &str {
    "RuntimeWorkspaceInvalidConfig"
  }

  fn message(&self) -> String {
    format!("invalid runtime workspace config: {}", self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeWorkspaceTargetNotFound {
  pub name: String,
}

impl MechErrorKind for RuntimeWorkspaceTargetNotFound {
  fn name(&self) -> &str {
    "RuntimeWorkspaceTargetNotFound"
  }

  fn message(&self) -> String {
    format!("runtime workspace target `{}` was not found", self.name)
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeWorkspaceNotLoaded;

impl MechErrorKind for RuntimeWorkspaceNotLoaded {
  fn name(&self) -> &str {
    "RuntimeWorkspaceNotLoaded"
  }

  fn message(&self) -> String {
    "runtime workspace must be loaded before it can be refreshed".to_string()
  }
}

pub(super) fn invalid_config<T>(reason: impl Into<String>) -> MResult<T> {
  Err(MechError::new(
    RuntimeWorkspaceInvalidConfig {
      reason: reason.into(),
    },
    None,
  ))
}