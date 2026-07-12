use mech_core::{MResult, MechError, MechErrorKind};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum RuntimeCapabilityOperation {
  Read,
  Write,
  Custom(String),
}

impl RuntimeCapabilityOperation {
  pub fn name(&self) -> &str {
    match self {
      RuntimeCapabilityOperation::Read => "read",
      RuntimeCapabilityOperation::Write => "write",
      RuntimeCapabilityOperation::Custom(name) => name.as_str(),
    }
  }

  pub fn from_name(name: impl Into<String>) -> MResult<Self> {
    let name = name.into();
    if name.is_empty() {
      return Err(MechError::new(
        RuntimeCapabilityOperationInvalid {
          reason: "operation cannot be empty".to_string(),
        },
        None,
      ));
    }

    Ok(match name.as_str() {
      "read" => RuntimeCapabilityOperation::Read,
      "write" => RuntimeCapabilityOperation::Write,
      _ => RuntimeCapabilityOperation::Custom(name),
    })
  }
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityOperationInvalid {
  pub reason: String,
}

impl MechErrorKind for RuntimeCapabilityOperationInvalid {
  fn name(&self) -> &str {
    "RuntimeCapabilityOperationInvalid"
  }

  fn message(&self) -> String {
    format!("invalid runtime capability operation: {}", self.reason)
  }
}
