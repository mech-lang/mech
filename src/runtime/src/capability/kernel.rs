use crate::*;

use mech_core::*;
use std::any::Any;
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

// -----------------------------------------------------------------------------
// Capability Kernel Trait
// -----------------------------------------------------------------------------

pub trait CapabilityKernelCheckpoint: std::fmt::Debug + Send {
    fn into_any(self: Box<Self>) -> Box<dyn Any>;
}

impl<T> CapabilityKernelCheckpoint for T
where
    T: std::fmt::Debug + Send + Any,
{
    fn into_any(self: Box<Self>) -> Box<dyn Any> {
        self
    }
}

/// Capability authority graph and checking interface.
///
/// This is the main runtime integration point. Store-backed, distributed,
/// audited, cryptographic-token-based, or host-specific authority systems should
/// implement this trait.
pub trait CapabilityKernel: std::fmt::Debug + Send {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "capability kernel".to_string(),
                reason: "kernel does not support transaction checkpoints".to_string(),
            },
            None,
        ))
    }

    fn restore(&mut self, _checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "capability kernel".to_string(),
                reason: "kernel does not support transaction checkpoint restore".to_string(),
            },
            None,
        ))
    }

    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId>;

    /// Administratively remove a grant that has not committed.
    ///
    /// Contract:
    ///
    /// - `grant` must not retain grant state when it returns `Err`;
    /// - `rollback_grant` is called only after `grant` returned `Ok`;
    /// - `rollback_grant` removes an uncommitted grant administratively;
    /// - it is not ordinary revocation;
    /// - it must ignore `Capability::is_revocable`;
    /// - it must be idempotent;
    /// - after successful rollback, the kernel must behave as though the grant
    ///   never occurred.
    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()>;

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()>;

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId>;

    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        match scope {
            RuntimeAuthorityScope::AllForSubject => self.check(request),
            RuntimeAuthorityScope::AllowList(_) => Err(MechError::new(
                TransactionStateUnsupportedError {
                    function: "capability kernel scoped check".to_string(),
                    reason: "custom kernel cannot enforce an authority allowlist".to_string(),
                },
                None,
            )),
        }
    }

    fn preview_check(&self, _request: &CapabilityRequest) -> MResult<CapabilityId> {
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "capability kernel preview".to_string(),
                reason: "kernel does not support non-consuming capability preview".to_string(),
            },
            None,
        ))
    }

    fn check_excluding(
        &mut self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        if excluded.is_empty() {
            return self.check(request);
        }
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "capability kernel".to_string(),
                reason: "kernel cannot exclude transaction-local revocations".to_string(),
            },
            None,
        ))
    }

    fn preview_check_excluding(
        &self,
        _request: &CapabilityRequest,
        _excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        Err(MechError::new(
      TransactionStateUnsupportedError {
        function: "capability kernel preview".to_string(),
        reason: "kernel does not support non-consuming preview with transaction-local revocations"
          .to_string(),
      },
      None,
    ))
    }

    fn preview_check_excluding_with_pending_uses(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        if pending_uses.values().any(|uses| *uses != 0) {
            return Err(MechError::new(
                TransactionStateUnsupportedError {
                    function: "capability kernel transactional preview".to_string(),
                    reason:
                        "kernel does not support preview with transaction-local use reservations"
                            .to_string(),
                },
                None,
            ));
        }

        self.preview_check_excluding(request, excluded)
    }

    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        match scope {
            RuntimeAuthorityScope::AllForSubject => {
                self.preview_check_excluding_with_pending_uses(request, excluded, pending_uses)
            }
            RuntimeAuthorityScope::AllowList(_) => Err(MechError::new(
                TransactionStateUnsupportedError {
                    function: "capability kernel transactional scoped preview".to_string(),
                    reason: "custom kernel cannot enforce an authority allowlist".to_string(),
                },
                None,
            )),
        }
    }

    fn apply_usage_delta(&mut self, _capability: CapabilityId, uses: u64) -> MResult<()> {
        if uses == 0 {
            return Ok(());
        }
        Err(MechError::new(
            TransactionStateUnsupportedError {
                function: "capability kernel usage commit".to_string(),
                reason: "kernel does not support transactional capability usage deltas".to_string(),
            },
            None,
        ))
    }

    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>>;

    fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>>;

    fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId>;

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

        self.by_subject.entry(subject).or_default().insert(id);

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

    #[cfg(test)]
    pub(crate) fn successful_uses_for_test(&self, id: CapabilityId) -> u64 {
        self.successful_uses(id)
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

    fn remove_capability_state(&mut self, capability: CapabilityId) {
        self.capabilities.remove(&capability);
        self.revoked.remove(&capability);
        self.uses.remove(&capability);

        self.by_subject.retain(|_, ids| {
            ids.remove(&capability);
            !ids.is_empty()
        });

        self.children.remove(&capability);
        self.parent
            .retain(|child, parent| *child != capability && *parent != capability);
        self.children.retain(|_, children| {
            children.remove(&capability);
            !children.is_empty()
        });
    }

    fn check_with_exclusions(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
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
            if !scope.contains(id) {
                last_reason =
                    Some("capability is outside the execution authority scope".to_string());
                continue;
            }
            if excluded.contains(&id) {
                last_reason = Some("capability is revoked by the active transaction".to_string());
                continue;
            }
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

    fn preview_check_with_exclusions_and_pending_uses(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
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

        let mut last_reason = None;
        for id in ids {
            if !scope.contains(*id) {
                last_reason =
                    Some("capability is outside the execution authority scope".to_string());
                continue;
            }
            if excluded.contains(id) {
                last_reason = Some("capability is revoked by the active transaction".to_string());
                continue;
            }
            if self.revoked.contains(id) {
                last_reason = Some("capability is revoked".to_string());
                continue;
            }

            let Some(capability) = self.capabilities.get(id) else {
                continue;
            };
            if let Some(max_uses) = capability.max_uses() {
                let committed = self.successful_uses(*id);
                let pending = pending_uses.get(id).copied().unwrap_or(0);
                let actual = committed.checked_add(pending).ok_or_else(|| {
                    MechError::new(
                        CapabilityDeniedError {
                            subject: request.subject.clone(),
                            operation: request.operation.clone(),
                            resource: request.resource.clone(),
                            reason: format!("usage count overflow for capability {}", id,),
                        },
                        None,
                    )
                })?;
                if actual >= max_uses {
                    last_reason = Some(format!(
                        "use limit exceeded: max {}, actual {}",
                        max_uses, actual,
                    ));
                    continue;
                }
            }

            let decision = capability.preview_check(request)?;
            if !decision.allowed {
                last_reason = decision.reason;
                continue;
            }
            return Ok(*id);
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
}

impl CapabilityKernel for BasicCapabilityKernel {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        Ok(Box::new(self.clone()))
    }

    fn restore(&mut self, checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        let snapshot = checkpoint
            .into_any()
            .downcast::<BasicCapabilityKernel>()
            .map_err(|_| {
                MechError::new(
                    TransactionStateUnsupportedError {
                        function: "basic capability kernel".to_string(),
                        reason: "checkpoint belongs to a different kernel implementation"
                            .to_string(),
                    },
                    None,
                )
            })?;
        *self = *snapshot;
        Ok(())
    }

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

    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        let descendants = self.descendants_of(capability);

        for descendant in descendants.into_iter().rev() {
            self.remove_capability_state(descendant);
        }

        self.remove_capability_state(capability);
        Ok(())
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
        self.check_with_exclusions(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            &HashSet::new(),
        )
    }

    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        self.check_with_exclusions(request, scope, &HashSet::new())
    }

    fn check_excluding(
        &mut self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        self.check_with_exclusions(request, &RuntimeAuthorityScope::AllForSubject, excluded)
    }

    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.preview_check_with_exclusions_and_pending_uses(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            &HashSet::new(),
            &HashMap::new(),
        )
    }

    fn preview_check_excluding(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        self.preview_check_with_exclusions_and_pending_uses(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            excluded,
            &HashMap::new(),
        )
    }

    fn preview_check_excluding_with_pending_uses(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.preview_check_with_exclusions_and_pending_uses(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            excluded,
            pending_uses,
        )
    }

    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.preview_check_with_exclusions_and_pending_uses(request, scope, excluded, pending_uses)
    }

    fn apply_usage_delta(&mut self, capability: CapabilityId, uses: u64) -> MResult<()> {
        if uses == 0 {
            return Ok(());
        }
        let Some(authority) = self.capabilities.get(&capability) else {
            return Err(MechError::new(CapabilityNotFoundError { capability }, None));
        };
        if self.revoked.contains(&capability) {
            return Err(MechError::new(CapabilityRevokedError { capability }, None));
        }
        let current = self.successful_uses(capability);
        let next = current.checked_add(uses).ok_or_else(|| {
            MechError::new(
                CapabilityDeniedError {
                    subject: authority.subject_key().to_string(),
                    operation: "commit-usage".to_string(),
                    resource: capability.to_string(),
                    reason: format!("usage count overflow for capability {}", capability,),
                },
                None,
            )
        })?;
        if let Some(max_uses) = authority.max_uses() {
            if next > max_uses {
                return Err(MechError::new(
                    CapabilityDeniedError {
                        subject: authority.subject_key().to_string(),
                        operation: "commit-usage".to_string(),
                        resource: capability.to_string(),
                        reason: format!("use limit exceeded: max {}, actual {}", max_uses, next,),
                    },
                    None,
                ));
            }
        }
        self.uses.insert(capability, next);
        Ok(())
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

    fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId> {
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

    fn resolve_capability(&self, capability: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
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
        Self {
            inner: Arc::new(Mutex::new(kernel)),
        }
    }

    #[cfg(test)]
    pub(crate) fn successful_uses_for_test(&self, id: CapabilityId) -> u64 {
        self.inner.lock().unwrap().successful_uses_for_test(id)
    }
}

impl CapabilityKernel for SharedCapabilityKernel {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        self.inner.lock().unwrap().checkpoint()
    }
    fn restore(&mut self, checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        self.inner.lock().unwrap().restore(checkpoint)
    }
    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().grant(grant)
    }
    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        self.inner.lock().unwrap().rollback_grant(capability)
    }
    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        self.inner.lock().unwrap().revoke(revocation)
    }
    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().check(request)
    }
    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().check_scoped(request, scope)
    }
    fn check_excluding(
        &mut self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        self.inner
            .lock()
            .unwrap()
            .check_excluding(request, excluded)
    }
    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().preview_check(request)
    }
    fn preview_check_excluding(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        self.inner
            .lock()
            .unwrap()
            .preview_check_excluding(request, excluded)
    }
    fn preview_check_excluding_with_pending_uses(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.inner
            .lock()
            .unwrap()
            .preview_check_excluding_with_pending_uses(request, excluded, pending_uses)
    }
    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().preview_scoped_with_transaction(
            request,
            scope,
            excluded,
            pending_uses,
        )
    }
    fn apply_usage_delta(&mut self, capability: CapabilityId, uses: u64) -> MResult<()> {
        self.inner
            .lock()
            .unwrap()
            .apply_usage_delta(capability, uses)
    }
    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        self.inner.lock().unwrap().get(id)
    }
    fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>> {
        self.inner.lock().unwrap().list_for_subject(subject)
    }
    fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId> {
        self.inner.lock().unwrap().derive_capability(derivation)
    }
    fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
        self.inner.lock().unwrap().is_revoked(id)
    }
}
