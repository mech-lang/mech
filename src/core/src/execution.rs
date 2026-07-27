//! Explicit services supplied while Mech functions execute.

use crate::{MResult, MechError, MechErrorKind, Value};

#[cfg(feature = "no_std")]
use alloc::{
  format,
  string::{String, ToString},
};
#[cfg(not(feature = "no_std"))]
use std::string::{String, ToString};

pub trait MechExecutionServices {
  fn invoke_native(
    &mut self,
    name: &str,
    arguments: &[Value],
  ) -> MResult<Value>;
}

#[derive(Debug, Default)]
pub struct NoMechExecutionServices;

impl MechExecutionServices for NoMechExecutionServices {
  fn invoke_native(
    &mut self,
    name: &str,
    _arguments: &[Value],
  ) -> MResult<Value> {
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
