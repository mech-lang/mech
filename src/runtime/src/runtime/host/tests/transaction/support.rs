use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use crate::{
    Capability, CapabilityDecision, CapabilityId, CapabilityRequest, RuntimeAfterCommitEffect,
    RuntimeEffectMetadata, RuntimeEffectSource, RuntimeInvalidOperationError,
    RuntimeTransactionalEffect,
};
use mech_core::{MResult, MechError};

#[derive(Debug)]
pub(super) struct RecordingHostEffect {
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) entry: String,
}

impl RuntimeAfterCommitEffect for RecordingHostEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::HostFunction {
                name: "demo/staged".to_string(),
            },
            "deliver",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push(self.entry.clone());
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct PreviewUnsupportedCapability {
    pub(super) id: CapabilityId,
    pub(super) subject: String,
    pub(super) resource: String,
}

impl Capability for PreviewUnsupportedCapability {
    fn id(&self) -> CapabilityId {
        self.id
    }

    fn subject_key(&self) -> &str {
        &self.subject
    }

    fn validate(&self) -> MResult<()> {
        Ok(())
    }

    fn check(&self, request: &CapabilityRequest) -> MResult<CapabilityDecision> {
        Ok(
            if request.subject == self.subject
                && request.operation == "call"
                && request.resource == self.resource
            {
                CapabilityDecision::allow()
            } else {
                CapabilityDecision::deny("request does not match")
            },
        )
    }
}

#[derive(Debug)]
pub(super) struct PreviewLifecycleEffect {
    pub(super) log: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug)]
pub(super) struct CountingAfterCommitEffect {
    pub(super) deliveries: Arc<AtomicUsize>,
}

impl RuntimeAfterCommitEffect for CountingAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::HostFunction {
                name: "demo/staged-limited".to_string(),
            },
            "deliver",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        self.deliveries.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

impl RuntimeTransactionalEffect for PreviewLifecycleEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::HostFunction {
                name: "demo/staged-lifecycle".to_string(),
            },
            "preview-lifecycle",
        )
    }

    fn prepare(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("prepare".to_string());
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("commit".to_string());
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.log.lock().unwrap().push("abort".to_string());
        Err(MechError::new(
            RuntimeInvalidOperationError {
                operation: "preview_lifecycle_abort",
                reason: "abort must not run for preview-only effects".to_string(),
            },
            None,
        ))
    }
}
