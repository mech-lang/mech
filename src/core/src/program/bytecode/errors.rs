use crate::{MResult, MechError, MechErrorKind, ValueKind};

#[cfg(feature = "no_std")]
use alloc::string::String;

use super::RuntimeType;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeValidationError {
    pub reason: String,
}

impl MechErrorKind for BytecodeValidationError {
    fn name(&self) -> &str {
        "BytecodeValidation"
    }

    fn message(&self) -> String {
        self.reason.clone()
    }
}

pub(crate) fn invalid<T>(reason: impl Into<String>) -> MResult<T> {
    Err(MechError::new(
        BytecodeValidationError {
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc())
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeConstantUnsupported {
    pub runtime_type: RuntimeType,
    pub source_value_kind: ValueKind,
    pub reason: String,
}

impl MechErrorKind for BytecodeConstantUnsupported {
    fn name(&self) -> &str {
        "BytecodeConstantUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "bytecode v1 does not support constant {:?} from {:?}: {}",
            self.runtime_type, self.source_value_kind, self.reason,
        )
    }
}

pub fn unsupported_constant(
    runtime_type: RuntimeType,
    source_value_kind: ValueKind,
    reason: impl Into<String>,
) -> MechError {
    MechError::new(
        BytecodeConstantUnsupported {
            runtime_type,
            source_value_kind,
            reason: reason.into(),
        },
        None,
    )
    .with_compiler_loc()
}
