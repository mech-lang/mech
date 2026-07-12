#[cfg(feature = "serde")]
use serde::{Serialize, Deserialize};

use crate::*;
use mech_core::*;
use std::sync::Arc;
use std::collections::{HashMap, HashSet};

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
    Self::new(format!("runtime://{}", id))
  }

  pub fn node(id: NodeId) -> Self {
    Self::new(format!("node://{}", id))
  }

  pub fn module(id: ModuleId) -> Self {
    Self::new(format!("module://{}", id))
  }

  pub fn actor(id: ActorId) -> Self {
    Self::new(format!("actor://{}", id))
  }

  pub fn task(id: TaskId) -> Self {
    Self::new(format!("task://{}", id))
  }

  pub fn host(name: impl AsRef<str>) -> Self {
    Self::new(format!("host://{}", name.as_ref()))
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
    Self::new(":read")
  }

  pub fn write() -> Self {
    Self::new(":write")
  }

  pub fn execute() -> Self {
    Self::new(":execute")
  }

  pub fn import() -> Self {
    Self::new(":import")
  }

  pub fn spawn() -> Self {
    Self::new(":spawn")
  }

  pub fn send() -> Self {
    Self::new(":send")
  }

  pub fn receive() -> Self {
    Self::new(":receive")
  }

  pub fn query() -> Self {
    Self::new(":query")
  }

  pub fn grant() -> Self {
    Self::new(":grant")
  }

  pub fn revoke() -> Self {
    Self::new(":revoke")
  }

  pub fn attenuate() -> Self {
    Self::new(":attenuate")
  }

  pub fn delegate() -> Self {
    Self::new(":delegate")
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
// Basic In-Memory Key Registry
// -----------------------------------------------------------------------------

pub trait CapabilityVerifier: std::fmt::Debug + Send + Sync {
  fn verify(&self, payload: &[u8], signature: &[u8]) -> MResult<()>;
}

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