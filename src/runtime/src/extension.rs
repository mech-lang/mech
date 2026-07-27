use std::any::Any;

use mech_core::{
  MResult, MechError, MechErrorKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeExtensionPanicked {
  pub component: String,
  pub operation: String,
  pub payload: String,
}

impl MechErrorKind for RuntimeExtensionPanicked {
  fn name(&self) -> &str {
    "RuntimeExtensionPanicked"
  }

  fn message(&self) -> String {
    format!(
      "Runtime extension `{}` panicked during `{}`: {}",
      self.component,
      self.operation,
      self.payload,
    )
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeStoreCommitIndeterminate {
  pub transaction_id: crate::TransactionId,
  pub payload: String,
}

impl MechErrorKind for RuntimeStoreCommitIndeterminate {
  fn name(&self) -> &str {
    "RuntimeStoreCommitIndeterminate"
  }

  fn message(&self) -> String {
    format!(
      "Runtime store commit for transaction {} panicked; durable state is indeterminate: {}",
      self.transaction_id,
      self.payload,
    )
  }
}

#[derive(Debug)]
pub(crate) struct RuntimeExtensionPanic {
  pub(crate) component: String,
  pub(crate) operation: String,
  pub(crate) payload: String,
}

impl RuntimeExtensionPanic {
  pub(crate) fn into_error(self) -> MechError {
    MechError::new(
      RuntimeExtensionPanicked {
        component: self.component,
        operation: self.operation,
        payload: self.payload,
      },
      None,
    )
  }
}

#[cfg(panic = "unwind")]
pub(crate) fn catch_extension<T>(
  component: impl Into<String>,
  operation: impl Into<String>,
  callback: impl FnOnce() -> T,
) -> Result<T, RuntimeExtensionPanic> {
  let component = component.into();
  let operation = operation.into();
  std::panic::catch_unwind(
    std::panic::AssertUnwindSafe(callback),
  )
  .map_err(|payload| RuntimeExtensionPanic {
    component,
    operation,
    payload: panic_payload(payload.as_ref()),
  })
}

#[cfg(not(panic = "unwind"))]
pub(crate) fn catch_extension<T>(
  _component: impl Into<String>,
  _operation: impl Into<String>,
  callback: impl FnOnce() -> T,
) -> Result<T, RuntimeExtensionPanic> {
  Ok(callback())
}

pub(crate) fn invoke_extension<T>(
  component: impl Into<String>,
  operation: impl Into<String>,
  callback: impl FnOnce() -> MResult<T>,
) -> MResult<T> {
  match catch_extension(component, operation, callback) {
    Ok(result) => result,
    Err(panic) => Err(panic.into_error()),
  }
}

pub(crate) fn invoke_extension_value<T>(
  component: impl Into<String>,
  operation: impl Into<String>,
  callback: impl FnOnce() -> T,
) -> MResult<T> {
  match catch_extension(component, operation, callback) {
    Ok(value) => Ok(value),
    Err(panic) => Err(panic.into_error()),
  }
}

fn panic_payload(payload: &(dyn Any + Send)) -> String {
  if let Some(message) = payload.downcast_ref::<&str>() {
    return (*message).to_string();
  }
  if let Some(message) = payload.downcast_ref::<String>() {
    return message.clone();
  }
  "non-string panic payload".to_string()
}
