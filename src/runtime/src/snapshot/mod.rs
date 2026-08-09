//! Detached values and summaries returned by the public runtime API.

use std::collections::BTreeMap;
use std::fmt::{Debug, Display, Formatter};

use mech_core::{LegacyValue, MResult, MechError, ValueKind};

use crate::{CapabilityId, ModuleVersionId};

#[cfg(feature = "invariant_define")]
use mech_engine::IntegrityConstraintReport;

#[cfg(test)]
#[path = "tests.rs"]
mod tests;

/// An owned, detached snapshot of an acyclic runtime value graph.
///
/// No reachable `Ref<T>` is shared with the live runtime or with another cloned
/// `RuntimeValueSnapshot`. Sharing within one snapshot is preserved.
///
/// Cyclic values are rejected by `try_capture`.
#[derive(PartialEq)]
pub struct RuntimeValueSnapshot {
    value: LegacyValue,
}

impl RuntimeValueSnapshot {
    pub fn try_capture(value: &LegacyValue) -> MResult<Self> {
        Ok(Self {
            value: value.try_deep_snapshot()?,
        })
    }

    pub fn empty() -> Self {
        Self {
            value: LegacyValue::Empty,
        }
    }

    /// Returns `true` exactly when the detached value is [`LegacyValue::Empty`].
    ///
    /// ```
    /// use mech_runtime::RuntimeValueSnapshot;
    ///
    /// assert!(RuntimeValueSnapshot::empty().is_empty());
    /// ```
    pub fn is_empty(&self) -> bool {
        matches!(&self.value, LegacyValue::Empty)
    }

    pub fn kind(&self) -> ValueKind {
        self.value.kind()
    }

    pub fn to_value(&self) -> LegacyValue {
        self.value
            .try_deep_snapshot()
            .expect("RuntimeValueSnapshot invariant violated: stored value must remain acyclic")
    }

    pub fn into_value(self) -> LegacyValue {
        self.value
    }
}

impl Clone for RuntimeValueSnapshot {
    fn clone(&self) -> Self {
        Self {
            value: self
                .value
                .try_deep_snapshot()
                .expect("RuntimeValueSnapshot invariant violated during clone"),
        }
    }
}

impl TryFrom<LegacyValue> for RuntimeValueSnapshot {
    type Error = MechError;

    fn try_from(value: LegacyValue) -> MResult<Self> {
        Self::try_capture(&value)
    }
}

impl TryFrom<&LegacyValue> for RuntimeValueSnapshot {
    type Error = MechError;

    fn try_from(value: &LegacyValue) -> MResult<Self> {
        Self::try_capture(value)
    }
}

pub trait TryIntoRuntimeValueSnapshot {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot>;
}

impl TryIntoRuntimeValueSnapshot for RuntimeValueSnapshot {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot> {
        Ok(self)
    }
}

impl TryIntoRuntimeValueSnapshot for LegacyValue {
    fn try_into_runtime_value_snapshot(self) -> MResult<RuntimeValueSnapshot> {
        RuntimeValueSnapshot::try_capture(&self)
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
