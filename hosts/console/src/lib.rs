pub mod module;
pub mod provider;

#[cfg(feature = "browser")]
pub mod browser;
#[cfg(feature = "native")]
pub mod native;

pub use module::{console_host_manifest, CONSOLE_HOST_MCFG};
pub use provider::{
  validate_console_settings, ConsoleBackend, ConsoleHostFactory, ConsoleResourceProvider,
  RecordingConsoleBackend,
};

#[cfg(feature = "browser")]
pub use browser::{BrowserConsoleBackend, BrowserConsoleHostFactory};
#[cfg(feature = "native")]
pub use native::{NativeConsoleBackend, NativeConsoleHostFactory};

use mech_core::{MechError, MechErrorKind};

#[derive(Debug, Clone)]
pub struct ConsoleHostError {
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for ConsoleHostError {
  fn name(&self) -> &str { "ConsoleHost" }
  fn message(&self) -> String { format!("{}: {}", self.resource, self.reason) }
}

pub(crate) fn console_error(resource: impl Into<String>, reason: impl Into<String>) -> MechError {
  MechError::new(ConsoleHostError { resource: resource.into(), reason: reason.into() }, None)
}
