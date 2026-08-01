use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use mech_core::{GenericError, MResult, MechError};

use crate::{
    BasicCapability, BasicCapabilityKernel, BasicConstraints, Capability, CapabilityDerivation,
    CapabilityGrant, CapabilityId, CapabilityKernel, CapabilityKernelCheckpoint, CapabilityRequest,
    CapabilityRevocation, RuntimeAuthorityScope, Subject,
};

pub(super) fn capability(id: CapabilityId, subject: &str, revocable: bool) -> Arc<dyn Capability> {
    Arc::new(BasicCapability::from_keys(id, subject, "db://users", [":read"]).revocable(revocable))
}

pub(super) fn request(subject: &str) -> CapabilityRequest {
    CapabilityRequest::from_keys(subject, ":read", "db://users")
}

pub(super) fn limited_capability(id: CapabilityId, max_uses: u64) -> Arc<dyn Capability> {
    Arc::new(
        BasicCapability::from_keys(id, "task:1", "db://users", [":read"])
            .with_constraints(BasicConstraints::default().with_max_uses(max_uses)),
    )
}

#[derive(Debug)]
pub(super) struct FailingRollbackKernel {
    pub(super) inner: BasicCapabilityKernel,
    pub(super) rollback_attempted: Arc<AtomicBool>,
}

impl CapabilityKernel for FailingRollbackKernel {
    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        self.inner.grant(grant)
    }

    fn rollback_grant(&mut self, _capability: CapabilityId) -> MResult<()> {
        self.rollback_attempted.store(true, Ordering::SeqCst);
        Err(MechError::new(
            GenericError {
                msg: "test kernel rollback failed".to_string(),
            },
            None,
        ))
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        self.inner.revoke(revocation)
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.inner.check(request)
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

#[derive(Debug, Default)]
pub(super) struct FailingCheckpointRestoreKernel {
    inner: BasicCapabilityKernel,
}

impl CapabilityKernel for FailingCheckpointRestoreKernel {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        self.inner.checkpoint()
    }

    fn restore(&mut self, _checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        Err(MechError::new(
            GenericError {
                msg: "deliberate capability checkpoint restore failure".to_string(),
            },
            None,
        ))
    }

    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        self.inner.grant(grant)
    }

    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        self.inner.rollback_grant(capability)
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        self.inner.revoke(revocation)
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.inner.check(request)
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum CapabilityPanicPhase {
    Preview,
    Check,
    Apply,
    Restore,
}

#[derive(Debug)]
pub(super) struct PanickingCapabilityKernel {
    inner: BasicCapabilityKernel,
    panic_at: CapabilityPanicPhase,
}

impl PanickingCapabilityKernel {
    pub(super) fn with_grant(
        panic_at: CapabilityPanicPhase,
        capability: Arc<dyn Capability>,
    ) -> Self {
        let mut inner = BasicCapabilityKernel::new();
        inner.grant(CapabilityGrant::new(capability)).unwrap();
        Self { inner, panic_at }
    }
}

impl CapabilityKernel for PanickingCapabilityKernel {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        self.inner.checkpoint()
    }

    fn restore(&mut self, checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        if self.panic_at == CapabilityPanicPhase::Restore {
            panic!("deliberate capability restore panic");
        }
        self.inner.restore(checkpoint)
    }

    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        self.inner.grant(grant)
    }

    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        self.inner.rollback_grant(capability)
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        self.inner.revoke(revocation)
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        if self.panic_at == CapabilityPanicPhase::Check {
            panic!("deliberate capability check panic");
        }
        self.inner.check(request)
    }

    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        if self.panic_at == CapabilityPanicPhase::Check {
            panic!("deliberate capability check panic");
        }
        self.inner.check_scoped(request, scope)
    }

    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        if self.panic_at == CapabilityPanicPhase::Preview {
            panic!("deliberate capability preview panic");
        }
        self.inner
            .preview_scoped_with_transaction(request, scope, excluded, pending_uses)
    }

    fn apply_usage_delta(&mut self, capability: CapabilityId, uses: u64) -> MResult<()> {
        if self.panic_at == CapabilityPanicPhase::Apply {
            panic!("deliberate capability apply panic");
        }
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
