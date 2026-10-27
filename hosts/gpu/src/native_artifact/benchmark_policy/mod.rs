//! Explicit benchmark policy control, not an alternative program evaluator.
//!
//! Both policies retain the real BasicCapabilityKernel grant/usage/checkpoint
//! state. Trusted mode bypasses capability-specific decisions only for a live,
//! in-scope, unlimited-use capability belonging to the requesting subject.
//! Revocation, scoped/excluded identities and finite-use capabilities remain
//! enforced. Native admission, schemas, work, memory and effects are untouched.
//! This is NOT a measurement of all runtime safety overhead. Production hosted
//! features do not enable this module or expose a launch-policy switch.
//! The launcher grants ResourcePathCapability values. A different unlimited-use
//! capability's internal check/preview constraints would also be bypassed; this
//! wrapper must not be presented as preserving those custom authorization rules.
use mech_core::{
    MResult, ManagedMemoryBudget, ManagedMemoryReservation, MechError, MemoryRuntimeError,
    MemorySpace,
};
use mech_runtime::{
    BasicCapabilityKernel, Capability, CapabilityDerivation, CapabilityGrant, CapabilityId,
    CapabilityKernel, CapabilityKernelCheckpoint, CapabilityRequest, CapabilityRevocation,
    NativeResourceBindingError, RuntimeAuthorityScope, Subject,
};
use std::alloc::Layout;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NativeBenchmarkPolicy {
    Secured,
    Trusted,
}
impl NativeBenchmarkPolicy {
    pub fn parse(value: &str) -> MResult<Self> {
        match value {
            "secured" => Ok(Self::Secured),
            "trusted" => Ok(Self::Trusted),
            _ => Err(invalid(
                "benchmark policy must be explicitly secured or trusted",
            )),
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Self::Secured => "secured",
            Self::Trusted => "trusted",
        }
    }
}
fn invalid(reason: impl Into<String>) -> MechError {
    MechError::new(
        NativeResourceBindingError {
            reason: reason.into(),
        },
        None,
    )
}
fn bytes(value: usize) -> MResult<u64> {
    u64::try_from(value).map_err(|_| invalid("benchmark policy storage exceeds u64"))
}

#[derive(Debug)]
struct Carriers {
    // Retire the allocation/Arc aliases before its exclusive backing charge.
    entries: Vec<Arc<dyn Capability>>,
    _storage: ManagedMemoryReservation,
}
impl Carriers {
    fn new(memory: &ManagedMemoryBudget, capacity: usize) -> MResult<Self> {
        let layout = Layout::array::<Arc<dyn Capability>>(capacity)
            .map_err(|_| invalid("benchmark policy carrier Layout overflows"))?;
        let requested = bytes(layout.size())?;
        let storage = memory.reserve_capacity(requested)?;
        let mut entries = Vec::new();
        entries
            .try_reserve_exact(capacity)
            .map_err(|_| MemoryRuntimeError::AllocationFailed {
                object: None,
                requested,
                alignment: layout.align() as u32,
                space: MemorySpace::Host,
            })?;
        if entries.capacity() != capacity {
            return Err(invalid(
                "benchmark policy carrier capacity exceeds admitted Layout",
            ));
        }
        Ok(Self {
            entries,
            _storage: storage,
        })
    }
    fn copy(&self, memory: &ManagedMemoryBudget) -> MResult<Self> {
        let mut copied = Self::new(memory, self.entries.capacity())?;
        copied.entries.extend(self.entries.iter().cloned());
        Ok(copied)
    }
}

#[derive(Debug)]
pub struct NativeBenchmarkCapabilityKernel {
    policy: NativeBenchmarkPolicy,
    inner: BasicCapabilityKernel,
    carriers: Carriers,
    _object: ManagedMemoryReservation,
    memory: ManagedMemoryBudget,
}
#[derive(Debug)]
struct Checkpoint {
    inner: BasicCapabilityKernel,
    carriers: Carriers,
    _object: ManagedMemoryReservation,
}
impl NativeBenchmarkCapabilityKernel {
    /// Capacity is frozen from the real launch grant count before installation.
    /// New wrapper/carrier and checkpoint backing is admitted on the SAME caller
    /// account; this does not claim to meter the pre-existing general runtime's
    /// store/kernel maps or process RSS.
    pub fn new(
        policy: NativeBenchmarkPolicy,
        grant_capacity: usize,
        memory: &ManagedMemoryBudget,
    ) -> MResult<Self> {
        let object = memory.reserve_capacity(bytes(std::mem::size_of::<Self>())?)?;
        let carriers = Carriers::new(memory, grant_capacity)?;
        Ok(Self {
            policy,
            inner: BasicCapabilityKernel::new(),
            carriers,
            _object: object,
            memory: memory.clone(),
        })
    }
    fn trusted_carrier(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<Option<CapabilityId>> {
        if self.policy != NativeBenchmarkPolicy::Trusted {
            return Ok(None);
        }
        for carrier in &self.carriers.entries {
            let id = carrier.id();
            if carrier.subject_key() != request.subject
                || carrier.max_uses().is_some()
                || !scope.contains(id)
                || excluded.contains(&id)
                || self.inner.is_revoked(id)?
            {
                continue;
            }
            // Administrative changes must not leave an old Arc acting as a
            // permit for a replacement or removed capability with the same ID.
            if self
                .inner
                .get(id)?
                .as_ref()
                .is_some_and(|current| Arc::ptr_eq(current, carrier))
            {
                return Ok(Some(id));
            }
        }
        Ok(None)
    }
    fn consume_trusted(&mut self, id: CapabilityId) -> MResult<CapabilityId> {
        self.inner.apply_usage_delta(id, 1)?;
        Ok(id)
    }
}
impl CapabilityKernel for NativeBenchmarkCapabilityKernel {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        let object = self
            .memory
            .reserve_capacity(bytes(std::mem::size_of::<Checkpoint>())?)?;
        let carriers = self.carriers.copy(&self.memory)?;
        Ok(Box::new(Checkpoint {
            inner: self.inner.clone(),
            carriers,
            _object: object,
        }))
    }
    fn restore(&mut self, checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        let checkpoint = checkpoint
            .into_any()
            .downcast::<Checkpoint>()
            .map_err(|_| invalid("wrong benchmark policy checkpoint type"))?;
        if checkpoint._object.budget() != self.memory
            || checkpoint.carriers.entries.capacity() != self.carriers.entries.capacity()
        {
            return Err(invalid(
                "benchmark policy checkpoint account/capacity changed",
            ));
        }
        let Checkpoint {
            inner,
            carriers,
            _object,
        } = *checkpoint;
        self.inner = inner;
        self.carriers = carriers;
        // Only checkpoint ownership retires here. The kernel's own object
        // reservation is never replaced with a smaller checkpoint reservation.
        drop(_object);
        Ok(())
    }
    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        let carrier = Arc::clone(&grant.capability);
        let existing = self
            .carriers
            .entries
            .iter()
            .position(|cap| cap.id() == carrier.id());
        if existing.is_none() && self.carriers.entries.len() == self.carriers.entries.capacity() {
            return Err(invalid("benchmark policy initial grant capacity exhausted"));
        }
        let id = self.inner.grant(grant)?;
        if let Some(index) = existing {
            self.carriers.entries[index] = carrier;
        } else {
            self.carriers.entries.push(carrier);
        }
        Ok(id)
    }
    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        self.inner.rollback_grant(capability)?;
        if let Some(index) = self
            .carriers
            .entries
            .iter()
            .position(|cap| cap.id() == capability)
        {
            self.carriers.entries.swap_remove(index);
        }
        Ok(())
    }
    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        self.inner.revoke(revocation)
    }
    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        if let Some(id) = self.trusted_carrier(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            &HashSet::new(),
        )? {
            self.consume_trusted(id)
        } else {
            self.inner.check(request)
        }
    }
    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        if let Some(id) = self.trusted_carrier(request, scope, &HashSet::new())? {
            self.consume_trusted(id)
        } else {
            self.inner.check_scoped(request, scope)
        }
    }
    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.trusted_carrier(
            request,
            &RuntimeAuthorityScope::AllForSubject,
            &HashSet::new(),
        )?
        .map_or_else(|| self.inner.preview_check(request), Ok)
    }
    fn check_excluding(
        &mut self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        if let Some(id) =
            self.trusted_carrier(request, &RuntimeAuthorityScope::AllForSubject, excluded)?
        {
            self.consume_trusted(id)
        } else {
            self.inner.check_excluding(request, excluded)
        }
    }
    fn preview_check_excluding(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        self.trusted_carrier(request, &RuntimeAuthorityScope::AllForSubject, excluded)?
            .map_or_else(|| self.inner.preview_check_excluding(request, excluded), Ok)
    }
    fn preview_check_excluding_with_pending_uses(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
        pending: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.trusted_carrier(request, &RuntimeAuthorityScope::AllForSubject, excluded)?
            .map_or_else(
                || {
                    self.inner
                        .preview_check_excluding_with_pending_uses(request, excluded, pending)
                },
                Ok,
            )
    }
    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        self.trusted_carrier(request, scope, excluded)?.map_or_else(
            || {
                self.inner
                    .preview_scoped_with_transaction(request, scope, excluded, pending)
            },
            Ok,
        )
    }
    fn apply_usage_delta(&mut self, capability: CapabilityId, uses: u64) -> MResult<()> {
        self.inner.apply_usage_delta(capability, uses)
    }
    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        self.inner.get(id)
    }
    fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>> {
        self.inner.list_for_subject(subject)
    }
    fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId> {
        self.inner.derive_capability(derivation)
    }
    fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
        self.inner.is_revoked(id)
    }
}

#[cfg(test)]
mod tests;
