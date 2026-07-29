//! Detached values and summaries returned by the public runtime API.

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};
use std::ops::Deref;

use mech_core::{Value, ValueKind};

use crate::{CapabilityId, ModuleVersionId};

#[cfg(feature = "invariant_define")]
use mech_program::IntegrityConstraintReport;

/// A detached snapshot of a runtime value graph.
///
/// No reachable `Ref<T>` is shared with the live runtime. Sharing and cycles
/// within the captured graph are preserved using newly allocated cells.
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

impl From<Value> for RuntimeValueSnapshot {
  fn from(value: Value) -> Self {
    Self::capture(&value)
  }
}

impl Deref for RuntimeValueSnapshot {
  type Target = Value;

  fn deref(&self) -> &Self::Target {
    self.as_value()
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

#[cfg(feature = "invariant_define")]
#[derive(Clone, Debug)]
pub struct RuntimeRootModuleExecutionReport {
  pub result: RuntimeValueSnapshot,
  pub integrity: IntegrityConstraintReport,
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
