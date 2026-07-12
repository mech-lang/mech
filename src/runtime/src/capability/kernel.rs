use crate::*;

use mech_core::*;
use std::sync::Arc;
use std::collections::{HashMap, HashSet, VecDeque};

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
// Shared In-Memory Kernel
// -----------------------------------------------------------------------------

use std::sync::Mutex;

/// Cloneable handle to one in-memory capability graph.
#[derive(Clone, Debug)]
pub struct SharedCapabilityKernel {
  inner: Arc<Mutex<BasicCapabilityKernel>>,
}

impl Default for SharedCapabilityKernel {
  fn default() -> Self {
    Self::new()
  }
}

impl SharedCapabilityKernel {
  pub fn new() -> Self {
    Self::from_kernel(BasicCapabilityKernel::new())
  }

  pub fn from_kernel(kernel: BasicCapabilityKernel) -> Self {
    Self { inner: Arc::new(Mutex::new(kernel)) }
  }

}

impl CapabilityKernel for SharedCapabilityKernel {
  fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> { self.inner.lock().unwrap().grant(grant) }
  fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> { self.inner.lock().unwrap().revoke(revocation) }
  fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> { self.inner.lock().unwrap().check(request) }
  fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> { self.inner.lock().unwrap().get(id) }
  fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>> { self.inner.lock().unwrap().list_for_subject(subject) }
  fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId> { self.inner.lock().unwrap().derive_capability(derivation) }
  fn is_revoked(&self, id: CapabilityId) -> MResult<bool> { self.inner.lock().unwrap().is_revoked(id) }
}
