//! Trait-first capability system for the Mech runtime.
//!
//! This module intentionally avoids a fixed `Capability` enum.
//!
//! Capabilities are not a closed vocabulary. Host applications, machine crates,
//! database layers, distributed runtimes, UI surfaces, filesystems, and network
//! transports should be able to define their own subjects, resources,
//! operations, capability records, token formats, signing schemes, and kernels.
//!
//! The core abstraction is:
//!
//! - Subject: who holds or uses authority
//! - Resource: what authority applies to
//! - Operation: what action is requested
//! - Capability: an authority-bearing object
//! - CapabilityKernel: the authority graph/checking implementation
//! - CapabilityToken: optional portable/signed authority representation
//!
//! The `Basic*` types at the bottom are only a default implementation for tests,
//! prototypes, and the first runtime shell. They are not the architecture.

use crate::*;

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use mech_core::{MResult, MechError, MechErrorKind};

use crate::id::{
  ActorId, CapabilityId, ModuleId, NodeId, ObjectId, RuntimeId, TaskId,
};

// -----------------------------------------------------------------------------
// Key Traits
// -----------------------------------------------------------------------------

/// A subject is an entity that can hold or use authority.
///
/// Examples:
///
/// - runtime
/// - node
/// - module
/// - actor
/// - task
/// - host
/// - plugin
/// - user/session
pub trait Subject: std::fmt::Debug + Send + Sync {
  fn key(&self) -> &str;
}

/// A resource is something protected by the capability system.
///
/// Examples:
///
/// - `db://main`
/// - `table://users`
/// - `object://<id>`
/// - `actor://<id>`
/// - `host-api://render`
/// - `fs://tmp/foo`
/// - `net://api.example.com`
/// - `worker://gpu-pool`
pub trait Resource: std::fmt::Debug + Send + Sync {
  fn key(&self) -> &str;
}

/// An operation is an action on a resource.
///
/// Examples:
///
/// - `read`
/// - `write`
/// - `execute`
/// - `import`
/// - `spawn`
/// - `send`
/// - `receive`
/// - `query`
/// - `grant`
/// - `revoke`
pub trait Operation: std::fmt::Debug + Send + Sync {
  fn key(&self) -> &str;
}

// -----------------------------------------------------------------------------
// Requests, Context, and Decisions
// -----------------------------------------------------------------------------

/// Context supplied during a capability check.
///
/// These fields are intentionally generic. A capability implementation may
/// ignore them or apply additional host-specific context outside this struct.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CapabilityContext {
  pub local: bool,
  pub bytes: Option<u64>,
  pub items: Option<u64>,
  pub duration_ms: Option<u64>,
}

impl CapabilityContext {
  pub fn local() -> Self {
    Self {
      local: true,
      ..Self::default()
    }
  }

  pub fn remote() -> Self {
    Self {
      local: false,
      ..Self::default()
    }
  }

  pub fn with_bytes(mut self, bytes: u64) -> Self {
    self.bytes = Some(bytes);
    self
  }

  pub fn with_items(mut self, items: u64) -> Self {
    self.items = Some(items);
    self
  }

  pub fn with_duration_ms(mut self, duration_ms: u64) -> Self {
    self.duration_ms = Some(duration_ms);
    self
  }
}

/// A normalized capability check request.
///
/// This stores normalized string keys so it can cross trait-object boundaries
/// without lifetime or associated-type complications.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityRequest {
  pub subject: String,
  pub operation: String,
  pub resource: String,
  pub context: CapabilityContext,
}

impl CapabilityRequest {
  pub fn new(
    subject: &dyn Subject,
    operation: &dyn Operation,
    resource: &dyn Resource,
  ) -> Self {
    Self {
      subject: subject.key().to_string(),
      operation: operation.key().to_string(),
      resource: resource.key().to_string(),
      context: CapabilityContext::local(),
    }
  }

  pub fn from_keys(
    subject: impl Into<String>,
    operation: impl Into<String>,
    resource: impl Into<String>,
  ) -> Self {
    Self {
      subject: subject.into(),
      operation: operation.into(),
      resource: resource.into(),
      context: CapabilityContext::local(),
    }
  }

  pub fn with_context(mut self, context: CapabilityContext) -> Self {
    self.context = context;
    self
  }
}

/// Result of asking a capability whether it authorizes a request.
///
/// This is a struct, not an enum, so custom kernels can add richer behavior
/// without matching on a fixed decision vocabulary.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilityDecision {
  pub allowed: bool,
  pub reason: Option<String>,
}

impl CapabilityDecision {
  pub fn allow() -> Self {
    Self {
      allowed: true,
      reason: None,
    }
  }

  pub fn deny(reason: impl Into<String>) -> Self {
    Self {
      allowed: false,
      reason: Some(reason.into()),
    }
  }
}

// -----------------------------------------------------------------------------
// Capability Traits
// -----------------------------------------------------------------------------

/// A capability is an authority-bearing object.
///
/// A capability implementation decides how to validate itself and how to check
/// a request. This trait is intentionally not tied to one resource model.
pub trait Capability: std::fmt::Debug + Send + Sync {
  fn id(&self) -> CapabilityId;

  fn subject_key(&self) -> &str;

  fn validate(&self) -> MResult<()>;

  fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision>;

  fn is_revocable(&self) -> bool {
    true
  }

  fn is_delegable(&self) -> bool {
    false
  }

  fn is_attenuable(&self) -> bool {
    true
  }

  /// Maximum number of successful uses allowed for this capability.
  ///
  /// Custom capability implementations may account for use limits internally.
  /// When this returns `Some`, the default kernel enforces the limit before
  /// accepting another successful use.
  fn max_uses(&self) -> Option<u64> {
    None
  }

  /// Derive a new capability from this one.
  ///
  /// Implementations may choose to support delegation, attenuation, both, or
  /// neither. The default denies derivation.
  fn derive_capability(
    &self,
    request: &CapabilityDerivation,
  ) -> MResult<Arc<dyn Capability>> {
    let _ = request;
    Err(MechError::new(
      CapabilityDerivationUnsupportedError {
        capability: self.id(),
      },
      None,
    ))
  }
}

/// Capability derivation request.
///
/// This single request type supports both delegation and attenuation without
/// introducing a fixed derivation enum. The `mode` string is conventional:
///
/// - `delegate`
/// - `attenuate`
///
/// Custom kernels may use other modes.
#[derive(Clone, Debug)]
pub struct CapabilityDerivation {
  pub mode: String,
  pub source: CapabilityId,
  pub new_id: CapabilityId,
  pub requested_by: String,
  pub new_subject: Option<String>,
  pub new_resource: Option<String>,
  pub allowed_operations: Option<HashSet<String>>,
  pub constraints: Option<BasicConstraints>,
}

impl CapabilityDerivation {
  pub fn delegate(
    source: CapabilityId,
    new_id: CapabilityId,
    requested_by: &dyn Subject,
    new_subject: &dyn Subject,
  ) -> Self {
    Self {
      mode: "delegate".to_string(),
      source,
      new_id,
      requested_by: requested_by.key().to_string(),
      new_subject: Some(new_subject.key().to_string()),
      new_resource: None,
      allowed_operations: None,
      constraints: None,
    }
  }

  pub fn attenuate(
    source: CapabilityId,
    new_id: CapabilityId,
    requested_by: &dyn Subject,
  ) -> Self {
    Self {
      mode: "attenuate".to_string(),
      source,
      new_id,
      requested_by: requested_by.key().to_string(),
      new_subject: None,
      new_resource: None,
      allowed_operations: None,
      constraints: None,
    }
  }

  pub fn with_subject(mut self, subject: &dyn Subject) -> Self {
    self.new_subject = Some(subject.key().to_string());
    self
  }

  pub fn with_resource(mut self, resource: &dyn Resource) -> Self {
    self.new_resource = Some(resource.key().to_string());
    self
  }

  pub fn with_operations<I>(mut self, operations: I) -> Self
  where
    I: IntoIterator,
    I::Item: AsRef<str>,
  {
    self.allowed_operations = Some(
      operations
        .into_iter()
        .map(|operation| operation.as_ref().to_string())
        .collect(),
    );
    self
  }

  pub fn with_constraints(mut self, constraints: BasicConstraints) -> Self {
    self.constraints = Some(constraints);
    self
  }
}

/// Capability grant request.
#[derive(Clone, Debug)]
pub struct CapabilityGrant {
  pub capability: Arc<dyn Capability>,
  pub issued_by: Option<String>,
}

impl CapabilityGrant {
  pub fn new(capability: Arc<dyn Capability>) -> Self {
    Self {
      capability,
      issued_by: None,
    }
  }

  pub fn issued_by(mut self, subject: &dyn Subject) -> Self {
    self.issued_by = Some(subject.key().to_string());
    self
  }
}

/// Capability revocation request.
#[derive(Clone, Debug)]
pub struct CapabilityRevocation {
  pub capability: CapabilityId,
  pub revoked_by: Option<String>,
  pub revoke_descendants: bool,
}

impl CapabilityRevocation {
  pub fn new(capability: CapabilityId) -> Self {
    Self {
      capability,
      revoked_by: None,
      revoke_descendants: true,
    }
  }

  pub fn revoked_by(mut self, subject: &dyn Subject) -> Self {
    self.revoked_by = Some(subject.key().to_string());
    self
  }

  pub fn revoke_descendants(mut self, value: bool) -> Self {
    self.revoke_descendants = value;
    self
  }
}
