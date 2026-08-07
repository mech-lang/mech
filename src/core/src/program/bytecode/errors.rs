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
pub struct BytecodeUnreferencedType {
    pub type_id: u32,
}

impl MechErrorKind for BytecodeUnreferencedType {
    fn name(&self) -> &str {
        "BytecodeUnreferencedType"
    }

    fn message(&self) -> String {
        format!(
            "bytecode v1 type-table row {} is not reachable from a constant",
            self.type_id
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeUnreferencedConstant {
    pub constant: u32,
}

impl MechErrorKind for BytecodeUnreferencedConstant {
    fn name(&self) -> &str {
        "BytecodeUnreferencedConstant"
    }

    fn message(&self) -> String {
        format!(
            "bytecode v1 constant-table row {} is not referenced by ConstLoad",
            self.constant
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeUnreferencedRequirement {
    pub requirement: u32,
}

impl MechErrorKind for BytecodeUnreferencedRequirement {
    fn name(&self) -> &str {
        "BytecodeUnreferencedRequirement"
    }

    fn message(&self) -> String {
        format!(
            "bytecode v1 application-requirement row {} is not referenced by an external instruction",
            self.requirement
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeConstantUnsupported {
    pub runtime_type: RuntimeType,
    pub source_value_kind: ValueKind,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BytecodeConstantDepthExceeded {
    pub maximum_depth: usize,
}

impl MechErrorKind for BytecodeConstantDepthExceeded {
    fn name(&self) -> &str {
        "BytecodeConstantDepthExceeded"
    }

    fn message(&self) -> String {
        format!(
            "bytecode v1 constant nesting exceeds the maximum depth of {}",
            self.maximum_depth
        )
    }
}

pub(crate) fn depth_exceeded(maximum_depth: usize) -> MechError {
    MechError::new(BytecodeConstantDepthExceeded { maximum_depth }, None).with_compiler_loc()
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
