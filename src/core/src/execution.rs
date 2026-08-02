//! Explicit services supplied while Mech functions execute.

use crate::{MResult, MechError, MechErrorKind, Value};

#[cfg(feature = "no_std")]
use alloc::{
    format,
    string::{String, ToString},
};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ResourceIntent {
    Read = 1,
    Assign = 2,
    Send = 3,
}

impl ResourceIntent {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            1 => Some(Self::Read),
            2 => Some(Self::Assign),
            3 => Some(Self::Send),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum ResourceDelivery {
    Snapshot = 0,
    Live = 1,
}

impl ResourceDelivery {
    pub(crate) fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Snapshot),
            1 => Some(Self::Live),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionHostFunctionRequest {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ExecutionResourceRequest {
    pub base_uri: String,
    pub path: String,
    pub context_name: String,
    pub operation: String,
    pub intent: ResourceIntent,
    pub delivery: ResourceDelivery,
}

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ApplicationRequirement {
    HostFunction(ExecutionHostFunctionRequest),
    Resource(ExecutionResourceRequest),
}

pub trait MechExecutionServices {
    fn invoke_native(&mut self, name: &str, arguments: &[Value]) -> MResult<Value>;
}

#[derive(Debug, Default)]
pub struct NoMechExecutionServices;

impl MechExecutionServices for NoMechExecutionServices {
    fn invoke_native(&mut self, name: &str, _arguments: &[Value]) -> MResult<Value> {
        Err(MechError::new(
            MechExecutionUnsupportedError {
                function: name.to_string(),
            },
            None,
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MechExecutionUnsupportedError {
    pub function: String,
}

impl MechErrorKind for MechExecutionUnsupportedError {
    fn name(&self) -> &str {
        "MechExecutionUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "native function `{}` requires execution services",
            self.function,
        )
    }
}
