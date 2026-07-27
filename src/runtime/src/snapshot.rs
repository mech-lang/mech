//! Detached values and summaries returned by the public runtime API.

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};

use mech_core::{Value, ValueKind};

use crate::{CapabilityId, ModuleVersionId};

#[derive(Clone, PartialEq)]
pub struct RuntimeValueSnapshot {
  value: Value,
}

impl RuntimeValueSnapshot {
  pub fn capture(value: &Value) -> Self {
    Self {
      value: value.deep_snapshot(),
    }
  }

  pub fn kind(&self) -> ValueKind {
    self.value.kind()
  }

  pub fn as_value(&self) -> &Value {
    &self.value
  }

  pub fn into_value(self) -> Value {
    self.value
  }
}

impl Debug for RuntimeValueSnapshot {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    Debug::fmt(&self.value, formatter)
  }
}

impl Display for RuntimeValueSnapshot {
  fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
    Display::fmt(&self.value, formatter)
  }
}

#[derive(Clone, Debug, PartialEq)]
pub struct RuntimeModuleResult {
  pub version: ModuleVersionId,
  pub exports: BTreeMap<String, RuntimeValueSnapshot>,
  pub result: RuntimeValueSnapshot,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeCapabilitySnapshot {
  pub id: CapabilityId,
  pub subject: String,
  pub revocable: bool,
  pub delegable: bool,
  pub attenuable: bool,
  pub max_uses: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeInvariantSnapshot {
  pub id: u64,
  pub name: String,
  pub passed: bool,
  pub expression: String,
  pub reason: String,
  pub evaluated_kind: String,
  pub actual: String,
  pub expected: String,
}
