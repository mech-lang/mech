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
// Basic Key Implementations
// -----------------------------------------------------------------------------

/// Default string-backed subject key.
///
/// This is a convenience implementation. Hosts may supply their own Subject
/// implementations.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasicSubject {
  key: String,
}

impl BasicSubject {
  pub fn new(key: impl Into<String>) -> Self {
    Self { key: key.into() }
  }

  pub fn runtime(id: RuntimeId) -> Self {
    Self::new(format!("runtime:{}", id))
  }

  pub fn node(id: NodeId) -> Self {
    Self::new(format!("node:{}", id))
  }

  pub fn module(id: ModuleId) -> Self {
    Self::new(format!("module:{}", id))
  }

  pub fn actor(id: ActorId) -> Self {
    Self::new(format!("actor:{}", id))
  }

  pub fn task(id: TaskId) -> Self {
    Self::new(format!("task:{}", id))
  }

  pub fn host(name: impl AsRef<str>) -> Self {
    Self::new(format!("host:{}", name.as_ref()))
  }
}

impl Subject for BasicSubject {
  fn key(&self) -> &str {
    &self.key
  }
}

impl std::fmt::Display for BasicSubject {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.key)
  }
}

/// Default string-backed resource key.
///
/// This is a convenience implementation. Hosts may supply their own Resource
/// implementations.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasicResource {
  key: String,
}

impl BasicResource {
  pub fn new(key: impl Into<String>) -> Self {
    Self { key: key.into() }
  }

  pub fn runtime(id: RuntimeId) -> Self {
    Self::new(format!("runtime://{}", id))
  }

  pub fn node(id: NodeId) -> Self {
    Self::new(format!("node://{}", id))
  }

  pub fn module(id: ModuleId) -> Self {
    Self::new(format!("module://{}", id))
  }

  pub fn object(id: ObjectId) -> Self {
    Self::new(format!("object://{}", id))
  }

  pub fn actor(id: ActorId) -> Self {
    Self::new(format!("actor://{}", id))
  }

  pub fn task(id: TaskId) -> Self {
    Self::new(format!("task://{}", id))
  }

  pub fn database(name: impl AsRef<str>) -> Self {
    Self::new(format!("db://{}", name.as_ref()))
  }

  pub fn table(name: impl AsRef<str>) -> Self {
    Self::new(format!("table://{}", name.as_ref()))
  }

  pub fn host_api(name: impl AsRef<str>) -> Self {
    Self::new(format!("host-api://{}", name.as_ref()))
  }

  pub fn file(path: impl AsRef<str>) -> Self {
    Self::new(format!("fs://{}", path.as_ref()))
  }

  pub fn network(endpoint: impl AsRef<str>) -> Self {
    Self::new(format!("net://{}", endpoint.as_ref()))
  }
}

impl Resource for BasicResource {
  fn key(&self) -> &str {
    &self.key
  }
}

impl std::fmt::Display for BasicResource {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.key)
  }
}

/// Default string-backed operation key.
///
/// This is a convenience implementation. Hosts may supply their own Operation
/// implementations.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BasicOperation {
  key: String,
}

impl BasicOperation {
  pub fn new(key: impl Into<String>) -> Self {
    Self { key: key.into() }
  }

  pub fn read() -> Self {
    Self::new("read")
  }

  pub fn write() -> Self {
    Self::new("write")
  }

  pub fn execute() -> Self {
    Self::new("execute")
  }

  pub fn import() -> Self {
    Self::new("import")
  }

  pub fn spawn() -> Self {
    Self::new("spawn")
  }

  pub fn send() -> Self {
    Self::new("send")
  }

  pub fn receive() -> Self {
    Self::new("receive")
  }

  pub fn query() -> Self {
    Self::new("query")
  }

  pub fn grant() -> Self {
    Self::new("grant")
  }

  pub fn revoke() -> Self {
    Self::new("revoke")
  }

  pub fn attenuate() -> Self {
    Self::new("attenuate")
  }

  pub fn delegate() -> Self {
    Self::new("delegate")
  }
}

impl Operation for BasicOperation {
  fn key(&self) -> &str {
    &self.key
  }
}

impl std::fmt::Display for BasicOperation {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{}", self.key)
  }
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

// -----------------------------------------------------------------------------
// Capability Kernel Trait
// -----------------------------------------------------------------------------

/// Capability authority graph and checking interface.
///
/// This is the main runtime integration point. Store-backed, distributed,
/// audited, cryptographic-token-based, or host-specific authority systems should
/// implement this trait.
pub trait CapabilityKernel: std::fmt::Debug + Send {
  fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId>;

  fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()>;

  fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId>;

  fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>>;

  fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>>;

  fn derive_capability(
    &mut self,
    derivation: CapabilityDerivation,
  ) -> MResult<CapabilityId>;

  fn is_revoked(&self, id: CapabilityId) -> MResult<bool>;
}

// -----------------------------------------------------------------------------
// Token, Signing, Verification, Key Resolution, and Validation Traits
// -----------------------------------------------------------------------------

const BASIC_CAPABILITY_TOKEN_DOMAIN: &[u8] = b"mech.capability-token.v1";

/// Optional portable capability representation.
///
/// Tokens are useful when authority crosses a trust boundary. Local runtime
/// authority should normally be represented in the kernel/store, not only as a
/// bearer token.
///
/// Signature verification is not authorization. A valid signature proves only
/// that the token was signed by a trusted issuer key. Runtime authorization
/// still needs to resolve and check the referenced capabilities.
pub trait CapabilityToken: std::fmt::Debug + Send + Sync {
  fn id(&self) -> CapabilityId;

  fn subject_key(&self) -> &str;

  fn issuer_key(&self) -> &str;

  fn key_id(&self) -> &str;

  fn audience(&self) -> Option<&str>;

  fn issued_at_ms(&self) -> u64;

  fn not_before_ms(&self) -> Option<u64>;

  fn expires_at_ms(&self) -> Option<u64>;

  fn capability_ids(&self) -> &[CapabilityId];

  fn signature(&self) -> Option<&[u8]>;

  fn validate_unsigned(&self) -> MResult<()>;

  fn signing_payload(&self) -> MResult<Vec<u8>>;
}

pub trait CapabilitySigner: std::fmt::Debug + Send + Sync {
  fn sign(&self, payload: &[u8]) -> MResult<Vec<u8>>;
}

pub trait CapabilityVerifier: std::fmt::Debug + Send + Sync {
  fn verify(&self, payload: &[u8], signature: &[u8]) -> MResult<()>;
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CapabilitySigningKeyStatus {
  Active,
  Retired,
  Revoked,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CapabilitySigningKeyRecord {
  pub issuer: String,
  pub key_id: String,
  pub algorithm: String,
  pub public_key: Vec<u8>,
  pub status: CapabilitySigningKeyStatus,
  pub created_at_ms: u64,
  pub not_before_ms: Option<u64>,
  pub not_after_ms: Option<u64>,
}

impl CapabilitySigningKeyRecord {
  pub fn validate(&self) -> MResult<()> {
    if self.issuer.trim().is_empty() {
      return invalid_key("issuer", "must not be empty");
    }

    if self.key_id.trim().is_empty() {
      return invalid_key("key_id", "must not be empty");
    }

    if self.algorithm.trim().is_empty() {
      return invalid_key("algorithm", "must not be empty");
    }

    if self.public_key.is_empty() {
      return invalid_key("public_key", "must not be empty");
    }

    if let (Some(start), Some(end)) = (self.not_before_ms, self.not_after_ms) {
      if start > end {
        return invalid_key("not_before_ms", "must be less than or equal to not_after_ms");
      }
    }

    Ok(())
  }

  pub fn may_verify_at(&self, now_ms: u64) -> bool {
    if self.status == CapabilitySigningKeyStatus::Revoked {
      return false;
    }

    if let Some(not_before) = self.not_before_ms {
      if now_ms < not_before {
        return false;
      }
    }

    if let Some(not_after) = self.not_after_ms {
      if now_ms > not_after {
        return false;
      }
    }

    true
  }

  pub fn may_sign_at(&self, now_ms: u64) -> bool {
    self.status == CapabilitySigningKeyStatus::Active && self.may_verify_at(now_ms)
  }
}

/// Resolves issuer-scoped verification keys.
pub trait CapabilityKeyResolver: std::fmt::Debug + Send + Sync {
  fn key_record(
    &self,
    issuer: &str,
    key_id: &str,
  ) -> MResult<Option<CapabilitySigningKeyRecord>>;

  fn verifier_for(
    &self,
    issuer: &str,
    key_id: &str,
  ) -> MResult<Option<Arc<dyn CapabilityVerifier>>>;
}

/// Resolves token-referenced capabilities and token revocation state.
pub trait CapabilityTokenResolver: std::fmt::Debug + Send + Sync {
  fn is_token_revoked(&self, token: CapabilityId) -> MResult<bool>;

  fn resolve_capability(
    &self,
    capability: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>>;

  fn is_capability_revoked(&self, capability: CapabilityId) -> MResult<bool>;
}

#[derive(Clone, Debug)]
pub struct CapabilityTokenValidationRequest<'a> {
  pub token: &'a dyn CapabilityToken,
  pub now_ms: u64,
  pub audience: Option<&'a str>,
  pub max_clock_skew_ms: u64,
}

#[derive(Clone, Debug)]
pub struct ValidatedCapabilityToken {
  pub token: CapabilityId,
  pub subject: String,
  pub issuer: String,
  pub key_id: String,
  pub capability_ids: Vec<CapabilityId>,
  pub capabilities: Vec<Arc<dyn Capability>>,
}

pub trait CapabilityTokenValidator: std::fmt::Debug + Send + Sync {
  fn validate_token(
    &self,
    request: CapabilityTokenValidationRequest<'_>,
  ) -> MResult<ValidatedCapabilityToken>;
}

#[derive(Debug)]
pub struct BasicCapabilityTokenValidator<K, R> {
  pub keys: K,
  pub resolver: R,
}

impl<K, R> BasicCapabilityTokenValidator<K, R> {
  pub fn new(keys: K, resolver: R) -> Self {
    Self { keys, resolver }
  }
}

impl<K, R> CapabilityTokenValidator for BasicCapabilityTokenValidator<K, R>
where
  K: CapabilityKeyResolver,
  R: CapabilityTokenResolver,
{
  fn validate_token(
    &self,
    request: CapabilityTokenValidationRequest<'_>,
  ) -> MResult<ValidatedCapabilityToken> {
    let token = request.token;
    token.validate_unsigned()?;

    let key_record = self
      .keys
      .key_record(token.issuer_key(), token.key_id())?
      .ok_or_else(|| {
        MechError::new(
          CapabilityKeyNotFoundError {
            issuer: token.issuer_key().to_string(),
            key_id: token.key_id().to_string(),
          },
          None,
        )
      })?;

    key_record.validate()?;

    if !key_record.may_verify_at(request.now_ms) {
      return Err(MechError::new(
        CapabilityKeyNotUsableError {
          issuer: token.issuer_key().to_string(),
          key_id: token.key_id().to_string(),
          reason: "key is not usable for verification at this time".to_string(),
        },
        None,
      ));
    }

    let verifier = self
      .keys
      .verifier_for(token.issuer_key(), token.key_id())?
      .ok_or_else(|| {
        MechError::new(
          CapabilityKeyNotFoundError {
            issuer: token.issuer_key().to_string(),
            key_id: token.key_id().to_string(),
          },
          None,
        )
      })?;

    let signature = token.signature().ok_or_else(|| {
      MechError::new(
        InvalidCapabilityTokenError {
          token: token.id(),
          reason: "missing signature".to_string(),
        },
        None,
      )
    })?;

    let payload = token.signing_payload()?;
    verifier.verify(&payload, signature)?;

    if self.resolver.is_token_revoked(token.id())? {
      return Err(MechError::new(
        CapabilityTokenRevokedError { token: token.id() },
        None,
      ));
    }

    if let Some(audience) = token.audience() {
      let Some(expected) = request.audience else {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: "token has an audience but no expected audience was supplied".to_string(),
          },
          None,
        ));
      };

      if audience != expected {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: format!("wrong audience: expected `{}`, actual `{}`", expected, audience),
          },
          None,
        ));
      }
    }

    let earliest = token
      .not_before_ms()
      .unwrap_or(token.issued_at_ms())
      .saturating_sub(request.max_clock_skew_ms);

    if request.now_ms < earliest {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: token.id(),
          reason: "token is not yet valid".to_string(),
        },
        None,
      ));
    }

    if let Some(expires_at) = token.expires_at_ms() {
      let latest = expires_at.saturating_add(request.max_clock_skew_ms);
      if request.now_ms > latest {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: "token has expired".to_string(),
          },
          None,
        ));
      }
    }

    let mut capabilities = Vec::with_capacity(token.capability_ids().len());

    for capability_id in token.capability_ids() {
      if self.resolver.is_capability_revoked(*capability_id)? {
        return Err(MechError::new(
          CapabilityRevokedError {
            capability: *capability_id,
          },
          None,
        ));
      }

      let capability = self
        .resolver
        .resolve_capability(*capability_id)?
        .ok_or_else(|| {
          MechError::new(
            CapabilityNotFoundError {
              capability: *capability_id,
            },
            None,
          )
        })?;

      capability.validate()?;

      if capability.subject_key() != token.subject_key() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: token.id(),
            reason: format!(
              "capability `{}` belongs to `{}`, not token subject `{}`",
              capability.id(),
              capability.subject_key(),
              token.subject_key(),
            ),
          },
          None,
        ));
      }

      capabilities.push(capability);
    }

    Ok(ValidatedCapabilityToken {
      token: token.id(),
      subject: token.subject_key().to_string(),
      issuer: token.issuer_key().to_string(),
      key_id: token.key_id().to_string(),
      capability_ids: token.capability_ids().to_vec(),
      capabilities,
    })
  }
}

// -----------------------------------------------------------------------------
// Basic Default Capability Implementation
// -----------------------------------------------------------------------------

/// Basic serializable constraints for the default capability implementation.
///
/// Custom capability implementations may ignore this entirely.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct BasicConstraints {
  pub resource_prefixes: Vec<String>,
  pub local_only: bool,
  pub max_bytes: Option<u64>,
  pub max_items: Option<u64>,
  pub max_duration_ms: Option<u64>,
  pub max_uses: Option<u64>,
}

impl BasicConstraints {
  pub fn unrestricted() -> Self {
    Self::default()
  }

  pub fn with_resource_prefix(mut self, prefix: impl Into<String>) -> Self {
    self.resource_prefixes.push(prefix.into());
    self
  }

  pub fn local_only(mut self) -> Self {
    self.local_only = true;
    self
  }

  pub fn with_max_bytes(mut self, max_bytes: u64) -> Self {
    self.max_bytes = Some(max_bytes);
    self
  }

  pub fn with_max_items(mut self, max_items: u64) -> Self {
    self.max_items = Some(max_items);
    self
  }

  pub fn with_max_duration_ms(mut self, max_duration_ms: u64) -> Self {
    self.max_duration_ms = Some(max_duration_ms);
    self
  }

  pub fn with_max_uses(mut self, max_uses: u64) -> Self {
    self.max_uses = Some(max_uses);
    self
  }

  fn validate(&self) -> MResult<()> {
    require_nonzero_opt("constraints.max_bytes", self.max_bytes)?;
    require_nonzero_opt("constraints.max_items", self.max_items)?;
    require_nonzero_opt("constraints.max_duration_ms", self.max_duration_ms)?;
    require_nonzero_opt("constraints.max_uses", self.max_uses)?;

    for prefix in &self.resource_prefixes {
      if prefix.trim().is_empty() {
        return invalid_capability(
          "constraints.resource_prefixes",
          "must not contain empty prefixes",
        );
      }
    }

    Ok(())
  }

  fn is_attenuation_of(
    &self,
    source: &BasicConstraints,
    source_capability: &BasicCapability,
  ) -> MResult<()> {
    self.validate()?;

    if source.local_only && !self.local_only {
      return Err(MechError::new(
        InvalidCapabilityDerivationError {
          reason: "derived constraints cannot relax local_only".to_string(),
        },
        None,
      ));
    }

    require_limit_not_relaxed("max_bytes", source.max_bytes, self.max_bytes)?;
    require_limit_not_relaxed("max_items", source.max_items, self.max_items)?;
    require_limit_not_relaxed(
      "max_duration_ms",
      source.max_duration_ms,
      self.max_duration_ms,
    )?;
    require_limit_not_relaxed("max_uses", source.max_uses, self.max_uses)?;

    for prefix in &self.resource_prefixes {
      if !source_capability.allows_resource(prefix) {
        return Err(MechError::new(
          InvalidCapabilityDerivationError {
            reason: format!(
              "derived resource prefix `{}` is outside source capability",
              prefix,
            ),
          },
          None,
        ));
      }
    }

    Ok(())
  }
}

/// Default trait-backed capability implementation.
///
/// This is not a closed capability vocabulary. It is a key-based implementation
/// of the Capability trait.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicCapability {
  pub id: CapabilityId,
  pub subject: String,
  pub resource: String,
  pub operations: HashSet<String>,
  pub constraints: BasicConstraints,
  pub revocable: bool,
  pub delegable: bool,
  pub attenuable: bool,
}

impl BasicCapability {
  pub fn new(
    id: CapabilityId,
    subject: &dyn Subject,
    resource: &dyn Resource,
    operations: impl IntoIterator<Item = BasicOperation>,
  ) -> Self {
    Self {
      id,
      subject: subject.key().to_string(),
      resource: resource.key().to_string(),
      operations: operations
        .into_iter()
        .map(|operation| operation.key().to_string())
        .collect(),
      constraints: BasicConstraints::default(),
      revocable: true,
      delegable: false,
      attenuable: true,
    }
  }

  pub fn from_keys(
    id: CapabilityId,
    subject: impl Into<String>,
    resource: impl Into<String>,
    operations: impl IntoIterator<Item = impl Into<String>>,
  ) -> Self {
    Self {
      id,
      subject: subject.into(),
      resource: resource.into(),
      operations: operations.into_iter().map(|op| op.into()).collect(),
      constraints: BasicConstraints::default(),
      revocable: true,
      delegable: false,
      attenuable: true,
    }
  }

  pub fn with_constraints(mut self, constraints: BasicConstraints) -> Self {
    self.constraints = constraints;
    self
  }

  pub fn revocable(mut self, value: bool) -> Self {
    self.revocable = value;
    self
  }

  pub fn delegable(mut self, value: bool) -> Self {
    self.delegable = value;
    self
  }

  pub fn attenuable(mut self, value: bool) -> Self {
    self.attenuable = value;
    self
  }

  fn allows_resource(&self, resource: &str) -> bool {
    self.resource == resource ||
      self
        .constraints
        .resource_prefixes
        .iter()
        .any(|prefix| resource.starts_with(prefix))
  }
}

impl Capability for BasicCapability {
  fn id(&self) -> CapabilityId {
    self.id
  }

  fn subject_key(&self) -> &str {
    &self.subject
  }

  fn validate(&self) -> MResult<()> {
    if self.id.is_zero() {
      return invalid_capability("id", "must not be zero");
    }

    if self.subject.trim().is_empty() {
      return invalid_capability("subject", "must not be empty");
    }

    if self.resource.trim().is_empty() {
      return invalid_capability("resource", "must not be empty");
    }

    if self.operations.is_empty() {
      return invalid_capability("operations", "must contain at least one operation");
    }

    for operation in &self.operations {
      if operation.trim().is_empty() {
        return invalid_capability("operations", "must not contain empty operation names");
      }
    }

    self.constraints.validate()
  }

  fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
    self.validate()?;

    if self.subject != request.subject {
      return Ok(CapabilityDecision::deny("capability belongs to another subject"));
    }

    if !self.operations.contains(&request.operation) {
      return Ok(CapabilityDecision::deny("operation is not allowed"));
    }

    if !self.allows_resource(&request.resource) {
      return Ok(CapabilityDecision::deny("resource is not allowed"));
    }

    if self.constraints.local_only && !request.context.local {
      return Ok(CapabilityDecision::deny("capability is local-only"));
    }

    if let (Some(max), Some(actual)) = (self.constraints.max_bytes, request.context.bytes) {
      if actual > max {
        return Ok(CapabilityDecision::deny(format!(
          "byte limit exceeded: max {}, actual {}",
          max, actual
        )));
      }
    }

    if let (Some(max), Some(actual)) = (self.constraints.max_items, request.context.items) {
      if actual > max {
        return Ok(CapabilityDecision::deny(format!(
          "item limit exceeded: max {}, actual {}",
          max, actual
        )));
      }
    }

    if let (Some(max), Some(actual)) =
      (self.constraints.max_duration_ms, request.context.duration_ms)
    {
      if actual > max {
        return Ok(CapabilityDecision::deny(format!(
          "duration limit exceeded: max {}ms, actual {}ms",
          max, actual
        )));
      }
    }

    Ok(CapabilityDecision::allow())
  }

  fn is_revocable(&self) -> bool {
    self.revocable
  }

  fn is_delegable(&self) -> bool {
    self.delegable
  }

  fn is_attenuable(&self) -> bool {
    self.attenuable
  }

  fn max_uses(&self) -> Option<u64> {
    self.constraints.max_uses
  }

  fn derive_capability(
    &self,
    request: &CapabilityDerivation,
  ) -> MResult<Arc<dyn Capability>> {
    self.validate()?;

    if request.source != self.id {
      return Err(MechError::new(
        InvalidCapabilityDerivationError {
          reason: "source capability id does not match capability being derived".to_string(),
        },
        None,
      ));
    }

    if request.requested_by != self.subject {
      return Err(MechError::new(
        CapabilityDeniedError {
          subject: request.requested_by.clone(),
          operation: request.mode.clone(),
          resource: format!("capability:{}", self.id),
          reason: "requesting subject does not hold the source capability".to_string(),
        },
        None,
      ));
    }

    if request.mode == "delegate" && !self.delegable {
      return Err(MechError::new(
        CapabilityNotDelegableError { capability: self.id },
        None,
      ));
    }

    if request.mode == "attenuate" && !self.attenuable {
      return Err(MechError::new(
        CapabilityNotAttenuableError { capability: self.id },
        None,
      ));
    }

    if request.mode != "delegate" && request.mode != "attenuate" {
      return Err(MechError::new(
        InvalidCapabilityDerivationError {
          reason: format!("unsupported derivation mode `{}`", request.mode),
        },
        None,
      ));
    }

    let mut derived = self.clone();
    derived.id = request.new_id;

    if let Some(subject) = &request.new_subject {
      if subject.trim().is_empty() {
        return invalid_capability("derivation.new_subject", "must not be empty");
      }
      derived.subject = subject.clone();
    }

    if let Some(resource) = &request.new_resource {
      if !self.allows_resource(resource) {
        return Err(MechError::new(
          CapabilityDeniedError {
            subject: request.requested_by.clone(),
            operation: request.mode.clone(),
            resource: resource.clone(),
            reason: "derived resource is outside source capability".to_string(),
          },
          None,
        ));
      }

      derived.resource = resource.clone();
    }

    if let Some(ops) = &request.allowed_operations {
      if ops.is_empty() {
        return invalid_capability(
          "derivation.allowed_operations",
          "must contain at least one operation",
        );
      }

      for op in ops {
        if op.trim().is_empty() {
          return invalid_capability(
            "derivation.allowed_operations",
            "must not contain empty operation names",
          );
        }

        if !self.operations.contains(op) {
          return Err(MechError::new(
            CapabilityDeniedError {
              subject: request.requested_by.clone(),
              operation: request.mode.clone(),
              resource: self.resource.clone(),
              reason: format!("derived operation `{}` is outside source capability", op),
            },
            None,
          ));
        }
      }

      derived.operations = ops.clone();
    }

    if let Some(constraints) = &request.constraints {
      constraints.is_attenuation_of(&self.constraints, self)?;
      derived.constraints = constraints.clone();
    }

    // Derived capabilities should not automatically be delegable.
    derived.delegable = false;

    derived.validate()?;
    Ok(Arc::new(derived))
  }
}

// -----------------------------------------------------------------------------
// In-Memory Default Kernel
// -----------------------------------------------------------------------------

/// Default in-memory capability kernel.
///
/// This is an implementation of the trait, not the model itself.
#[derive(Clone, Debug, Default)]
pub struct BasicCapabilityKernel {
  capabilities: HashMap<CapabilityId, Arc<dyn Capability>>,
  by_subject: HashMap<String, HashSet<CapabilityId>>,
  revoked: HashSet<CapabilityId>,
  uses: HashMap<CapabilityId, u64>,
  parent: HashMap<CapabilityId, CapabilityId>,
  children: HashMap<CapabilityId, HashSet<CapabilityId>>,
}

impl BasicCapabilityKernel {
  pub fn new() -> Self {
    Self::default()
  }

  fn index_capability(&mut self, capability: Arc<dyn Capability>) -> CapabilityId {
    let id = capability.id();
    let subject = capability.subject_key().to_string();

    self
      .by_subject
      .entry(subject)
      .or_default()
      .insert(id);

    self.capabilities.insert(id, capability);
    id
  }

  fn index_derived_capability(
    &mut self,
    source: CapabilityId,
    capability: Arc<dyn Capability>,
  ) -> CapabilityId {
    let id = self.index_capability(capability);
    self.parent.insert(id, source);
    self.children.entry(source).or_default().insert(id);
    id
  }

  fn successful_uses(&self, id: CapabilityId) -> u64 {
    self.uses.get(&id).copied().unwrap_or(0)
  }

  fn increment_uses(&mut self, id: CapabilityId) {
    let value = self.uses.entry(id).or_insert(0);
    *value = value.saturating_add(1);
  }

  fn descendants_of(&self, id: CapabilityId) -> Vec<CapabilityId> {
    let mut out = Vec::new();
    let mut queue = VecDeque::new();

    if let Some(children) = self.children.get(&id) {
      for child in children {
        queue.push_back(*child);
      }
    }

    while let Some(next) = queue.pop_front() {
      out.push(next);

      if let Some(children) = self.children.get(&next) {
        for child in children {
          queue.push_back(*child);
        }
      }
    }

    out
  }
}

impl CapabilityKernel for BasicCapabilityKernel {
  fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
    let capability = grant.capability;
    capability.validate()?;

    let id = capability.id();

    if self.capabilities.contains_key(&id) {
      return Err(MechError::new(
        CapabilityAlreadyExistsError { capability: id },
        None,
      ));
    }

    Ok(self.index_capability(capability))
  }

  fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
    let Some(capability) = self.capabilities.get(&revocation.capability) else {
      return Err(MechError::new(
        CapabilityNotFoundError {
          capability: revocation.capability,
        },
        None,
      ));
    };

    if !capability.is_revocable() {
      return Err(MechError::new(
        CapabilityNotRevocableError {
          capability: revocation.capability,
        },
        None,
      ));
    }

    self.revoked.insert(revocation.capability);

    if revocation.revoke_descendants {
      for descendant in self.descendants_of(revocation.capability) {
        self.revoked.insert(descendant);
      }
    }

    Ok(())
  }

  fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
    let Some(ids) = self.by_subject.get(&request.subject) else {
      return Err(MechError::new(
        CapabilityDeniedError {
          subject: request.subject.clone(),
          operation: request.operation.clone(),
          resource: request.resource.clone(),
          reason: "subject has no capabilities".to_string(),
        },
        None,
      ));
    };

    let ids: Vec<CapabilityId> = ids.iter().copied().collect();
    let mut last_reason = None;

    for id in ids {
      if self.revoked.contains(&id) {
        last_reason = Some("capability is revoked".to_string());
        continue;
      }

      let Some(capability) = self.capabilities.get(&id) else {
        continue;
      };

      if let Some(max_uses) = capability.max_uses() {
        let actual = self.successful_uses(id);
        if actual >= max_uses {
          last_reason = Some(format!(
            "use limit exceeded: max {}, actual {}",
            max_uses, actual,
          ));
          continue;
        }
      }

      let decision = capability.check(request)?;

      if !decision.allowed {
        last_reason = decision.reason;
        continue;
      }

      // The generic Capability trait does not expose max_uses, because custom
      // capabilities can implement their own use accounting inside check().
      // The default kernel still tracks successful uses for inspection.
      self.increment_uses(id);
      return Ok(id);
    }

    Err(MechError::new(
      CapabilityDeniedError {
        subject: request.subject.clone(),
        operation: request.operation.clone(),
        resource: request.resource.clone(),
        reason: last_reason.unwrap_or_else(|| "no matching capability".to_string()),
      },
      None,
    ))
  }

  fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
    Ok(self.capabilities.get(&id).cloned())
  }

  fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>> {
    let Some(ids) = self.by_subject.get(subject.key()) else {
      return Ok(Vec::new());
    };

    Ok(ids.iter().copied().collect())
  }

  fn derive_capability(
    &mut self,
    derivation: CapabilityDerivation,
  ) -> MResult<CapabilityId> {
    let Some(source) = self.capabilities.get(&derivation.source).cloned() else {
      return Err(MechError::new(
        CapabilityNotFoundError {
          capability: derivation.source,
        },
        None,
      ));
    };

    if self.revoked.contains(&derivation.source) {
      return Err(MechError::new(
        CapabilityRevokedError {
          capability: derivation.source,
        },
        None,
      ));
    }

    if self.capabilities.contains_key(&derivation.new_id) {
      return Err(MechError::new(
        CapabilityAlreadyExistsError {
          capability: derivation.new_id,
        },
        None,
      ));
    }

    let source_id = derivation.source;
    let derived = source.derive_capability(&derivation)?;
    derived.validate()?;

    Ok(self.index_derived_capability(source_id, derived))
  }

  fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
    Ok(self.revoked.contains(&id))
  }
}

impl CapabilityTokenResolver for BasicCapabilityKernel {
  fn is_token_revoked(&self, _token: CapabilityId) -> MResult<bool> {
    // The basic in-memory kernel does not store token revocations. Hosts that
    // issue bearer tokens should use a store-backed resolver for this.
    Ok(false)
  }

  fn resolve_capability(
    &self,
    capability: CapabilityId,
  ) -> MResult<Option<Arc<dyn Capability>>> {
    self.get(capability)
  }

  fn is_capability_revoked(&self, capability: CapabilityId) -> MResult<bool> {
    self.is_revoked(capability)
  }
}

// -----------------------------------------------------------------------------
// Token Support: Basic Signed Token Shape
// -----------------------------------------------------------------------------

/// Default token payload.
///
/// This is not required by the kernel. It is a simple serializable token shape
/// for hosts that want portable capability tokens. Signing/verification remains
/// trait-based.
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BasicCapabilityToken {
  pub id: CapabilityId,
  pub subject: String,
  pub capability_ids: Vec<CapabilityId>,
  pub issuer: String,
  pub audience: Option<String>,
  pub key_id: String,
  pub issued_at_ms: u64,
  pub not_before_ms: Option<u64>,
  pub expires_at_ms: Option<u64>,
  pub nonce: Option<Vec<u8>>,
  pub signature: Option<Vec<u8>>,
}

impl BasicCapabilityToken {
  pub fn new(
    id: CapabilityId,
    subject: &dyn Subject,
    issuer: &dyn Subject,
    key_id: impl Into<String>,
    issued_at_ms: u64,
    capability_ids: Vec<CapabilityId>,
  ) -> Self {
    Self {
      id,
      subject: subject.key().to_string(),
      capability_ids,
      issuer: issuer.key().to_string(),
      audience: None,
      key_id: key_id.into(),
      issued_at_ms,
      not_before_ms: None,
      expires_at_ms: None,
      nonce: None,
      signature: None,
    }
  }

  pub fn with_audience(mut self, audience: impl Into<String>) -> Self {
    self.audience = Some(audience.into());
    self
  }

  pub fn with_not_before_ms(mut self, not_before_ms: u64) -> Self {
    self.not_before_ms = Some(not_before_ms);
    self
  }

  pub fn with_expiration_ms(mut self, expires_at_ms: u64) -> Self {
    self.expires_at_ms = Some(expires_at_ms);
    self
  }

  pub fn with_nonce(mut self, nonce: Vec<u8>) -> Self {
    self.nonce = Some(nonce);
    self
  }

  pub fn sign(&mut self, signer: &dyn CapabilitySigner) -> MResult<()> {
    if self.signature.is_some() {
      return Err(MechError::new(
        CapabilityTokenAlreadySignedError { token: self.id },
        None,
      ));
    }

    let payload = self.signing_payload()?;
    let signature = signer.sign(&payload)?;
    self.signature = Some(signature);
    Ok(())
  }

  pub fn clear_signature(&mut self) {
    self.signature = None;
  }
}

impl CapabilityToken for BasicCapabilityToken {
  fn id(&self) -> CapabilityId {
    self.id
  }

  fn subject_key(&self) -> &str {
    &self.subject
  }

  fn issuer_key(&self) -> &str {
    &self.issuer
  }

  fn key_id(&self) -> &str {
    &self.key_id
  }

  fn audience(&self) -> Option<&str> {
    self.audience.as_deref()
  }

  fn issued_at_ms(&self) -> u64 {
    self.issued_at_ms
  }

  fn not_before_ms(&self) -> Option<u64> {
    self.not_before_ms
  }

  fn expires_at_ms(&self) -> Option<u64> {
    self.expires_at_ms
  }

  fn capability_ids(&self) -> &[CapabilityId] {
    &self.capability_ids
  }

  fn signature(&self) -> Option<&[u8]> {
    self.signature.as_deref()
  }

  fn validate_unsigned(&self) -> MResult<()> {
    if self.id.is_zero() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "token id must not be zero".to_string(),
        },
        None,
      ));
    }

    if self.subject.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "subject must not be empty".to_string(),
        },
        None,
      ));
    }

    if self.issuer.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "issuer must not be empty".to_string(),
        },
        None,
      ));
    }

    if self.key_id.trim().is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "key_id must not be empty".to_string(),
        },
        None,
      ));
    }

    if let Some(audience) = &self.audience {
      if audience.trim().is_empty() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "audience must not be empty when present".to_string(),
          },
          None,
        ));
      }
    }

    if self.capability_ids.is_empty() {
      return Err(MechError::new(
        InvalidCapabilityTokenError {
          token: self.id,
          reason: "token must reference at least one capability".to_string(),
        },
        None,
      ));
    }

    for capability_id in &self.capability_ids {
      if capability_id.is_zero() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "token must not reference zero capability ids".to_string(),
          },
          None,
        ));
      }
    }

    if let Some(not_before) = self.not_before_ms {
      if not_before < self.issued_at_ms {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "not_before_ms must be greater than or equal to issued_at_ms".to_string(),
          },
          None,
        ));
      }
    }

    if let Some(expires_at) = self.expires_at_ms {
      if expires_at < self.issued_at_ms {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "expires_at_ms must be greater than or equal to issued_at_ms".to_string(),
          },
          None,
        ));
      }

      if let Some(not_before) = self.not_before_ms {
        if expires_at < not_before {
          return Err(MechError::new(
            InvalidCapabilityTokenError {
              token: self.id,
              reason: "expires_at_ms must be greater than or equal to not_before_ms".to_string(),
            },
            None,
          ));
        }
      }
    }

    if let Some(nonce) = &self.nonce {
      if nonce.is_empty() {
        return Err(MechError::new(
          InvalidCapabilityTokenError {
            token: self.id,
            reason: "nonce must not be empty when present".to_string(),
          },
          None,
        ));
      }
    }

    Ok(())
  }

  fn signing_payload(&self) -> MResult<Vec<u8>> {
    self.validate_unsigned()?;

    let mut out = Vec::new();

    push_len_prefixed(&mut out, BASIC_CAPABILITY_TOKEN_DOMAIN);
    out.extend_from_slice(&self.id.as_u128().to_le_bytes());
    push_len_prefixed(&mut out, self.subject.as_bytes());
    push_len_prefixed(&mut out, self.issuer.as_bytes());
    push_len_prefixed(&mut out, self.key_id.as_bytes());
    out.extend_from_slice(&self.issued_at_ms.to_le_bytes());

    match self.not_before_ms {
      Some(value) => {
        out.push(1);
        out.extend_from_slice(&value.to_le_bytes());
      }
      None => out.push(0),
    }

    match self.expires_at_ms {
      Some(value) => {
        out.push(1);
        out.extend_from_slice(&value.to_le_bytes());
      }
      None => out.push(0),
    }

    match &self.audience {
      Some(value) => {
        out.push(1);
        push_len_prefixed(&mut out, value.as_bytes());
      }
      None => out.push(0),
    }

    match &self.nonce {
      Some(value) => {
        out.push(1);
        push_len_prefixed(&mut out, value);
      }
      None => out.push(0),
    }

    let mut ids = self.capability_ids.clone();
    ids.sort();

    out.extend_from_slice(&(ids.len() as u64).to_le_bytes());
    for id in ids {
      out.extend_from_slice(&id.as_u128().to_le_bytes());
    }

    Ok(out)
  }
}

fn push_len_prefixed(out: &mut Vec<u8>, bytes: &[u8]) {
  out.extend_from_slice(&(bytes.len() as u64).to_le_bytes());
  out.extend_from_slice(bytes);
}

// -----------------------------------------------------------------------------
// Basic In-Memory Key Registry
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct BasicCapabilityKeyRegistry {
  records: HashMap<(String, String), CapabilitySigningKeyRecord>,
  verifiers: HashMap<(String, String), Arc<dyn CapabilityVerifier>>,
}

impl BasicCapabilityKeyRegistry {
  pub fn new() -> Self {
    Self::default()
  }

  pub fn insert(
    &mut self,
    record: CapabilitySigningKeyRecord,
    verifier: Arc<dyn CapabilityVerifier>,
  ) -> MResult<()> {
    record.validate()?;

    let key = (record.issuer.clone(), record.key_id.clone());
    self.records.insert(key.clone(), record);
    self.verifiers.insert(key, verifier);
    Ok(())
  }
}

impl CapabilityKeyResolver for BasicCapabilityKeyRegistry {
  fn key_record(
    &self,
    issuer: &str,
    key_id: &str,
  ) -> MResult<Option<CapabilitySigningKeyRecord>> {
    Ok(self
      .records
      .get(&(issuer.to_string(), key_id.to_string()))
      .cloned())
  }

  fn verifier_for(
    &self,
    issuer: &str,
    key_id: &str,
  ) -> MResult<Option<Arc<dyn CapabilityVerifier>>> {
    Ok(self
      .verifiers
      .get(&(issuer.to_string(), key_id.to_string()))
      .cloned())
  }
}

// -----------------------------------------------------------------------------
// Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidCapabilityError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidCapabilityError {
  fn name(&self) -> &str {
    "InvalidCapability"
  }

  fn message(&self) -> String {
    format!("Invalid capability field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityDeniedError {
  pub subject: String,
  pub operation: String,
  pub resource: String,
  pub reason: String,
}

impl MechErrorKind for CapabilityDeniedError {
  fn name(&self) -> &str {
    "CapabilityDenied"
  }

  fn message(&self) -> String {
    format!(
      "Capability denied: subject `{}` cannot `{}` resource `{}`: {}",
      self.subject, self.operation, self.resource, self.reason
    )
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityAlreadyExistsError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityAlreadyExistsError {
  fn name(&self) -> &str {
    "CapabilityAlreadyExists"
  }

  fn message(&self) -> String {
    format!("Capability already exists: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotFoundError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotFoundError {
  fn name(&self) -> &str {
    "CapabilityNotFound"
  }

  fn message(&self) -> String {
    format!("Capability not found: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityRevokedError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityRevokedError {
  fn name(&self) -> &str {
    "CapabilityRevoked"
  }

  fn message(&self) -> String {
    format!("Capability has been revoked: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotRevocableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotRevocableError {
  fn name(&self) -> &str {
    "CapabilityNotRevocable"
  }

  fn message(&self) -> String {
    format!("Capability is not revocable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotDelegableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotDelegableError {
  fn name(&self) -> &str {
    "CapabilityNotDelegable"
  }

  fn message(&self) -> String {
    format!("Capability is not delegable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityNotAttenuableError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityNotAttenuableError {
  fn name(&self) -> &str {
    "CapabilityNotAttenuable"
  }

  fn message(&self) -> String {
    format!("Capability is not attenuable: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityDerivationUnsupportedError {
  pub capability: CapabilityId,
}

impl MechErrorKind for CapabilityDerivationUnsupportedError {
  fn name(&self) -> &str {
    "CapabilityDerivationUnsupported"
  }

  fn message(&self) -> String {
    format!("Capability does not support derivation: {}", self.capability)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityDerivationError {
  pub reason: String,
}

impl MechErrorKind for InvalidCapabilityDerivationError {
  fn name(&self) -> &str {
    "InvalidCapabilityDerivation"
  }

  fn message(&self) -> String {
    format!("Invalid capability derivation: {}", self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityTokenError {
  pub token: CapabilityId,
  pub reason: String,
}

impl MechErrorKind for InvalidCapabilityTokenError {
  fn name(&self) -> &str {
    "InvalidCapabilityToken"
  }

  fn message(&self) -> String {
    format!("Invalid capability token `{}`: {}", self.token, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityTokenAlreadySignedError {
  pub token: CapabilityId,
}

impl MechErrorKind for CapabilityTokenAlreadySignedError {
  fn name(&self) -> &str {
    "CapabilityTokenAlreadySigned"
  }

  fn message(&self) -> String {
    format!("Capability token already signed: {}", self.token)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityTokenRevokedError {
  pub token: CapabilityId,
}

impl MechErrorKind for CapabilityTokenRevokedError {
  fn name(&self) -> &str {
    "CapabilityTokenRevoked"
  }

  fn message(&self) -> String {
    format!("Capability token has been revoked: {}", self.token)
  }
}

#[derive(Debug, Clone)]
pub struct InvalidCapabilityKeyError {
  pub field: &'static str,
  pub reason: &'static str,
}

impl MechErrorKind for InvalidCapabilityKeyError {
  fn name(&self) -> &str {
    "InvalidCapabilityKey"
  }

  fn message(&self) -> String {
    format!("Invalid capability key field `{}`: {}", self.field, self.reason)
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityKeyNotFoundError {
  pub issuer: String,
  pub key_id: String,
}

impl MechErrorKind for CapabilityKeyNotFoundError {
  fn name(&self) -> &str {
    "CapabilityKeyNotFound"
  }

  fn message(&self) -> String {
    format!(
      "Capability signing key not found: issuer `{}`, key `{}`",
      self.issuer, self.key_id,
    )
  }
}

#[derive(Debug, Clone)]
pub struct CapabilityKeyNotUsableError {
  pub issuer: String,
  pub key_id: String,
  pub reason: String,
}

impl MechErrorKind for CapabilityKeyNotUsableError {
  fn name(&self) -> &str {
    "CapabilityKeyNotUsable"
  }

  fn message(&self) -> String {
    format!(
      "Capability signing key is not usable: issuer `{}`, key `{}`: {}",
      self.issuer, self.key_id, self.reason,
    )
  }
}

fn invalid_capability<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidCapabilityError { field, reason },
    None,
  ))
}

fn require_nonzero_opt(field: &'static str, value: Option<u64>) -> MResult<()> {
  if matches!(value, Some(0)) {
    return invalid_capability(field, "must be greater than zero");
  }

  Ok(())
}

fn invalid_key<T>(field: &'static str, reason: &'static str) -> MResult<T> {
  Err(MechError::new(
    InvalidCapabilityKeyError { field, reason },
    None,
  ))
}

fn require_limit_not_relaxed(
  field: &'static str,
  source: Option<u64>,
  derived: Option<u64>,
) -> MResult<()> {
  match (source, derived) {
    (Some(_), None) => Err(MechError::new(
      InvalidCapabilityDerivationError {
        reason: format!("derived constraints cannot remove `{}` limit", field),
      },
      None,
    )),
    (Some(source), Some(derived)) if derived > source => Err(MechError::new(
      InvalidCapabilityDerivationError {
        reason: format!(
          "derived constraints cannot increase `{}` limit: source {}, derived {}",
          field, source, derived,
        ),
      },
      None,
    )),
    _ => Ok(()),
  }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn basic_capability_grant_and_check() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
  }

  #[test]
  fn basic_capability_denies_wrong_operation() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::write(),
      &resource,
    );

    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn basic_capability_resource_prefix_allows_nested_resource() {
    let subject = BasicSubject::new("task:1");
    let root = BasicResource::new("db");
    let users = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &root,
      [BasicOperation::read()],
    )
    .with_constraints(
      BasicConstraints::default()
        .with_resource_prefix("db:"),
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &users,
    );

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
  }

  #[test]
  fn revocation_blocks_use() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    kernel
      .revoke(CapabilityRevocation::new(CapabilityId(1)))
      .unwrap();

    let request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn delegation_requires_delegable_source() {
    let subject = BasicSubject::new("task:1");
    let next_subject = BasicSubject::new("task:2");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn delegable_source_can_be_delegated() {
    let subject = BasicSubject::new("task:1");
    let next_subject = BasicSubject::new("task:2");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .delegable(true);

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );

    assert_eq!(
      kernel.derive_capability(derivation).unwrap(),
      CapabilityId(2),
    );
  }

  #[test]
  fn attenuation_can_reduce_operations() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read(), BasicOperation::write()],
    );

    let mut kernel = BasicCapabilityKernel::new();

    kernel
      .grant(CapabilityGrant::new(Arc::new(cap)))
      .unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_operations(["read"]);

    assert_eq!(
      kernel.derive_capability(derivation).unwrap(),
      CapabilityId(2),
    );

    let derived = kernel
      .get(CapabilityId(2))
      .unwrap()
      .expect("derived capability should exist");

    let read_request = CapabilityRequest::new(
      &subject,
      &BasicOperation::read(),
      &resource,
    );

    assert!(derived.check(&read_request).unwrap().allowed);

    let write_request = CapabilityRequest::new(
      &subject,
      &BasicOperation::write(),
      &resource,
    );

    assert!(!derived.check(&write_request).unwrap().allowed);
  }

  #[test]
  fn token_payload_is_deterministic() {
    let subject = BasicSubject::new("task:1");
    let issuer = BasicSubject::new("host:root");

    let a = BasicCapabilityToken::new(
      CapabilityId(10),
      &subject,
      &issuer,
      "key-1",
      100,
      vec![CapabilityId(2), CapabilityId(1)],
    );

    let b = BasicCapabilityToken::new(
      CapabilityId(10),
      &subject,
      &issuer,
      "key-1",
      100,
      vec![CapabilityId(1), CapabilityId(2)],
    );

    assert_eq!(a.signing_payload().unwrap(), b.signing_payload().unwrap());
  }

  #[test]
  fn max_uses_is_enforced_by_default_kernel() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().with_max_uses(1));

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let request = CapabilityRequest::new(&subject, &BasicOperation::read(), &resource);

    assert_eq!(kernel.check(&request).unwrap(), CapabilityId(1));
    assert!(kernel.check(&request).is_err());
  }

  #[test]
  fn attenuation_cannot_relax_local_only() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().local_only());

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_constraints(BasicConstraints::default());

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn attenuation_cannot_increase_limits() {
    let subject = BasicSubject::new("task:1");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .with_constraints(BasicConstraints::default().with_max_bytes(10));

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::attenuate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
    )
    .with_constraints(BasicConstraints::default().with_max_bytes(20));

    assert!(kernel.derive_capability(derivation).is_err());
  }

  #[test]
  fn parent_revocation_revokes_descendant_by_default() {
    let subject = BasicSubject::new("task:1");
    let next_subject = BasicSubject::new("task:2");
    let resource = BasicResource::new("db:users");

    let cap = BasicCapability::new(
      CapabilityId(1),
      &subject,
      &resource,
      [BasicOperation::read()],
    )
    .delegable(true);

    let mut kernel = BasicCapabilityKernel::new();
    kernel.grant(CapabilityGrant::new(Arc::new(cap))).unwrap();

    let derivation = CapabilityDerivation::delegate(
      CapabilityId(1),
      CapabilityId(2),
      &subject,
      &next_subject,
    );
    kernel.derive_capability(derivation).unwrap();

    kernel
      .revoke(CapabilityRevocation::new(CapabilityId(1)))
      .unwrap();

    assert!(kernel.is_revoked(CapabilityId(1)).unwrap());
    assert!(kernel.is_revoked(CapabilityId(2)).unwrap());
  }

}