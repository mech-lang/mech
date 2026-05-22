//! Runtime configuration for Mech.
//!
//! This module defines scalar runtime configuration only.
//!
//! It intentionally does not select concrete implementations for storage,
//! scheduling, module resolution, event sinks, transports, allocators, clocks,
//! or capability kernels. Those should be supplied through a RuntimeBuilder
//! using traits such as:
//!
//! - MechStore
//! - ModuleResolver
//! - Scheduler
//! - CapabilityKernel
//! - EventSink
//! - Transport
//! - IdGenerator
//!
//! RuntimeConfig should remain serializable, copyable in spirit, and free of
//! host callbacks or boxed trait objects.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use mech_core::{MResult, MechError, MechErrorKind};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeConfig {
  /// Human-readable runtime name.
  pub name: String,

  /// Execution and resource limits.
  pub limits: RuntimeLimits,

  /// Diagnostics, tracing, profiling, and debug behavior.
  pub diagnostics: DiagnosticsConfig,

}

impl Default for RuntimeConfig {
  fn default() -> Self {
    Self {
      name: "runtime".to_string(),
      limits: RuntimeLimits::default(),
      diagnostics: DiagnosticsConfig::default(),
    }
  }
}

impl RuntimeConfig {
  pub fn new(name: impl Into<String>) -> Self {
    Self {
      name: name.into(),
      ..Self::default()
    }
  }

  pub fn with_limits(mut self, limits: RuntimeLimits) -> Self {
    self.limits = limits;
    self
  }

  pub fn with_diagnostics(mut self, diagnostics: DiagnosticsConfig) -> Self {
    self.diagnostics = diagnostics;
    self
  }

  pub fn validate(&self) -> MResult<()> {
    self.limits.validate()?;
    Ok(())
  }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuntimeLimits {
  /// Default maximum interpreter steps per task or actor turn.
  ///
  /// None means no explicit step limit.
  pub max_steps_per_turn: Option<u64>,

  /// Default maximum wall-clock time per task or actor turn, in milliseconds.
  ///
  /// None means no explicit wall-clock limit.
  pub max_turn_duration_ms: Option<u64>,

  /// Default memory budget for runtime-managed allocations, in bytes.
  ///
  /// This is advisory until the runtime has allocator integration.
  pub max_memory_bytes: Option<u64>,

  /// Maximum number of tasks allowed to exist at once.
  pub max_tasks: Option<u64>,

  /// Maximum number of actors allowed to exist at once.
  pub max_actors: Option<u64>,

  /// Maximum queued messages per actor.
  pub max_actor_mailbox_len: Option<u64>,

  /// Maximum bytes accepted for one source/module.
  pub max_source_bytes: Option<u64>,

  /// Maximum runtime events retained in memory.
  ///
  /// Durable event retention is a store concern, not a config enum here.
  pub max_in_memory_events: Option<u64>,
}

impl Default for RuntimeLimits {
  fn default() -> Self {
    Self {
      max_steps_per_turn: Some(10_000),
      max_turn_duration_ms: Some(1_000),
      max_memory_bytes: None,
      max_tasks: Some(10_000),
      max_actors: Some(10_000),
      max_actor_mailbox_len: Some(10_000),
      max_source_bytes: Some(10 * 1024 * 1024),
      max_in_memory_events: Some(100_000),
    }
  }
}

impl RuntimeLimits {
  /// Unbounded limits for trusted local use.
  ///
  /// This is not suitable for untrusted plugins, network-loaded code,
  /// multi-tenant runtimes, or user-submitted scripts.
  pub fn trusted() -> Self {
    Self {
      max_steps_per_turn: None,
      max_turn_duration_ms: None,
      max_memory_bytes: None,
      max_tasks: None,
      max_actors: None,
      max_actor_mailbox_len: None,
      max_source_bytes: None,
      max_in_memory_events: None,
    }
  }

  pub fn validate(&self) -> MResult<()> {
    require_nonzero_opt("limits.max_steps_per_turn", self.max_steps_per_turn)?;
    require_nonzero_opt("limits.max_turn_duration_ms", self.max_turn_duration_ms)?;
    require_nonzero_opt("limits.max_memory_bytes", self.max_memory_bytes)?;
    require_nonzero_opt("limits.max_tasks", self.max_tasks)?;
    require_nonzero_opt("limits.max_actors", self.max_actors)?;
    require_nonzero_opt("limits.max_actor_mailbox_len", self.max_actor_mailbox_len)?;
    require_nonzero_opt("limits.max_source_bytes", self.max_source_bytes)?;
    require_nonzero_opt("limits.max_in_memory_events", self.max_in_memory_events)?;
    Ok(())
  }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogLevel {
  Error,
  Warn,
  Info,
  Debug,
  Trace,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagnosticsConfig {
  /// Whether interpreter/runtime tracing is enabled.
  pub trace_enabled: bool,

  /// Whether profiling is enabled.
  pub profile_enabled: bool,

  /// Whether debug diagnostics are enabled.
  pub debug_enabled: bool,

  /// Whether to include verbose event data in diagnostics.
  pub log_level: LogLevel,

}

impl Default for DiagnosticsConfig {
  fn default() -> Self {
    Self {
      trace_enabled: false,
      profile_enabled: false,
      debug_enabled: false,
      log_level: LogLevel::Info,
    }
  }
}

#[derive(Debug, Clone)]
pub struct InvalidRuntimeConfigError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidRuntimeConfigError {
  fn name(&self) -> &str {
    "InvalidRuntimeConfig"
  }
  fn message(&self) -> String {
    format!("Invalid runtime config field `{}`: {}", self.field, self.reason)
  }
}

fn require_nonzero_opt(field: &'static str, value: Option<u64>) -> MResult<()> {
  if matches!(value, Some(0)) {
    return Err(MechError::new(
      InvalidRuntimeConfigError {
        field,
        reason: "must be greater than zero",
      },
      None,
    ));
  }

  Ok(())
}

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn default_config_is_valid() {
    let config = RuntimeConfig::default();
    assert!(config.validate().is_ok());
  }

  #[test]
  fn trusted_limits_are_unbounded() {
    let limits = RuntimeLimits::trusted();

    assert_eq!(limits.max_steps_per_turn, None);
    assert_eq!(limits.max_turn_duration_ms, None);
    assert_eq!(limits.max_memory_bytes, None);
  }

  #[test]
  fn zero_limit_is_invalid() {
    let mut config = RuntimeConfig::default();
    config.limits.max_steps_per_turn = Some(0);

    assert!(config.validate().is_err());
  }

}