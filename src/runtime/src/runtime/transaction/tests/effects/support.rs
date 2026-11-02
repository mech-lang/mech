use std::collections::HashSet;
#[cfg(feature = "source")]
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::{MResult, MechError, MechErrorKind};

#[cfg(feature = "source")]
use crate::PreparedRuntimeEffect;
use crate::{
    ActorId, CapabilityId, EventId, IdGenerator, MessageId, NodeId, ObjectId,
    RuntimeAfterCommitEffect, RuntimeCompensatableEffect, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeId, RuntimeTransactionalEffect, TaskId, TransactionId,
};

#[derive(Debug)]
pub(super) struct FailingEventIdGenerator {
    next: u128,
    event_call: usize,
    fail_calls: HashSet<usize>,
}

impl FailingEventIdGenerator {
    pub(super) fn new(fail_calls: impl IntoIterator<Item = usize>) -> Self {
        Self {
            next: 1,
            event_call: 0,
            fail_calls: fail_calls.into_iter().collect(),
        }
    }

    fn next_id(&mut self) -> u128 {
        let id = self.next;
        self.next = self.next.saturating_add(1);
        id
    }
}

impl IdGenerator for FailingEventIdGenerator {
    fn runtime_id(&mut self) -> RuntimeId {
        RuntimeId(self.next_id())
    }

    fn object_id(&mut self) -> ObjectId {
        ObjectId(self.next_id())
    }

    fn actor_id(&mut self) -> ActorId {
        ActorId(self.next_id())
    }

    fn task_id(&mut self) -> TaskId {
        TaskId(self.next_id())
    }

    fn capability_id(&mut self) -> CapabilityId {
        CapabilityId(self.next_id())
    }

    fn transaction_id(&mut self) -> TransactionId {
        TransactionId(self.next_id())
    }

    fn event_id(&mut self) -> EventId {
        self.event_call = self.event_call.saturating_add(1);
        if self.fail_calls.contains(&self.event_call) {
            EventId(0)
        } else {
            EventId(1_000 + self.event_call as u128)
        }
    }

    fn node_id(&mut self) -> NodeId {
        NodeId(self.next_id())
    }

    fn message_id(&mut self) -> MessageId {
        MessageId(self.next_id())
    }
}

#[derive(Debug, Clone)]
struct SyntheticEffectError {
    message: String,
}

impl MechErrorKind for SyntheticEffectError {
    fn name(&self) -> &str {
        "SyntheticEffectError"
    }

    fn message(&self) -> String {
        self.message.clone()
    }
}

pub(super) fn synthetic_error(message: impl Into<String>) -> MechError {
    MechError::new(
        SyntheticEffectError {
            message: message.into(),
        },
        None,
    )
}

fn record(log: &Arc<Mutex<Vec<String>>>, entry: impl Into<String>) {
    log.lock().unwrap().push(entry.into());
}

fn synthetic_metadata(name: &str) -> RuntimeEffectMetadata {
    RuntimeEffectMetadata::new(
        RuntimeEffectSource::Custom {
            name: name.to_string(),
        },
        "synthetic",
    )
}

#[derive(Debug)]
pub(super) struct SyntheticTransactionalEffect {
    pub(super) name: &'static str,
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) fail_prepare: bool,
    pub(super) fail_commit: bool,
    pub(super) fail_abort: bool,
}

impl RuntimeTransactionalEffect for SyntheticTransactionalEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn prepare(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:prepare", self.name));
        if self.fail_prepare {
            return Err(synthetic_error(format!("{} prepare failed", self.name,)));
        }
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:commit", self.name));
        if self.fail_commit {
            return Err(synthetic_error(format!("{} commit failed", self.name,)));
        }
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:abort", self.name));
        if self.fail_abort {
            return Err(synthetic_error(format!("{} abort failed", self.name,)));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct SyntheticCompensatableEffect {
    pub(super) name: &'static str,
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) fail_apply: bool,
    pub(super) fail_compensate: bool,
    pub(super) fail_abort: bool,
}

impl RuntimeCompensatableEffect for SyntheticCompensatableEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn apply(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:apply", self.name));
        if self.fail_apply {
            return Err(synthetic_error(format!("{} apply failed", self.name,)));
        }
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:compensate", self.name));
        if self.fail_compensate {
            return Err(synthetic_error(format!("{} compensate failed", self.name,)));
        }
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:abort", self.name));
        if self.fail_abort {
            return Err(synthetic_error(format!("{} abort failed", self.name,)));
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct SyntheticAfterCommitEffect {
    pub(super) name: &'static str,
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) fail_deliver: bool,
}

#[derive(Debug)]
#[cfg(feature = "source")]
pub(super) struct FailOnceAbortEffect {
    pub(super) attempts: Arc<AtomicUsize>,
}

#[cfg(feature = "source")]
impl RuntimeTransactionalEffect for FailOnceAbortEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata("fail-once-abort")
    }

    fn prepare(&mut self) -> MResult<()> {
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        if self.attempts.fetch_add(1, Ordering::SeqCst) == 0 {
            return Err(synthetic_error("deliberate first abort failure"));
        }
        Ok(())
    }
}

impl RuntimeAfterCommitEffect for SyntheticAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn deliver(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:deliver", self.name));
        if self.fail_deliver {
            return Err(synthetic_error(format!("{} delivery failed", self.name,)));
        }
        Ok(())
    }
}

pub(super) fn transactional(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
) -> SyntheticTransactionalEffect {
    SyntheticTransactionalEffect {
        name,
        log,
        fail_prepare: false,
        fail_commit: false,
        fail_abort: false,
    }
}

pub(super) fn compensatable(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
) -> SyntheticCompensatableEffect {
    SyntheticCompensatableEffect {
        name,
        log,
        fail_apply: false,
        fail_compensate: false,
        fail_abort: false,
    }
}

pub(super) fn after_commit(
    name: &'static str,
    log: Arc<Mutex<Vec<String>>>,
) -> SyntheticAfterCommitEffect {
    SyntheticAfterCommitEffect {
        name,
        log,
        fail_deliver: false,
    }
}

#[derive(Debug)]
#[cfg(feature = "source")]
pub(super) struct NoopAfterCommit {
    name: &'static str,
}

#[derive(Debug)]
#[cfg(feature = "resident-routing-source")]
pub(super) struct SensitiveAfterCommit {
    pub(super) secret_payload: String,
}

#[cfg(feature = "resident-routing-source")]
impl RuntimeAfterCommitEffect for SensitiveAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "sensitive-test".to_string(),
            },
            "deliver",
        )
        .with_resource("test://metadata-only")
    }

    fn deliver(&mut self) -> MResult<()> {
        assert!(!self.secret_payload.is_empty());
        Ok(())
    }
}

#[derive(Debug)]
#[cfg(feature = "source")]
pub(super) struct CostedAfterCommit {
    pub(super) cost: crate::RuntimeEffectCost,
}

#[cfg(feature = "source")]
impl RuntimeAfterCommitEffect for CostedAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata("costed").with_cost(self.cost)
    }

    fn deliver(&mut self) -> MResult<()> {
        Ok(())
    }
}

#[cfg(feature = "source")]
impl RuntimeAfterCommitEffect for NoopAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: self.name.to_string(),
            },
            "deliver",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        Ok(())
    }
}

#[cfg(feature = "source")]
pub(super) fn effect(name: &'static str) -> PreparedRuntimeEffect {
    PreparedRuntimeEffect::AfterCommit(Box::new(NoopAfterCommit { name }))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum PanicEffectPhase {
    Prepare,
    Commit,
    Abort,
    Apply,
    Compensate,
    Deliver,
}

#[derive(Debug)]
pub(super) struct PanickingTransactionalEffect {
    pub(super) name: &'static str,
    pub(super) panic_at: Option<PanicEffectPhase>,
    pub(super) log: Arc<Mutex<Vec<String>>>,
}

impl RuntimeTransactionalEffect for PanickingTransactionalEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn prepare(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:prepare", self.name));
        if self.panic_at == Some(PanicEffectPhase::Prepare) {
            panic!("deliberate transactional prepare panic");
        }
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:commit", self.name));
        if self.panic_at == Some(PanicEffectPhase::Commit) {
            panic!("deliberate transactional commit panic");
        }
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:abort", self.name));
        if self.panic_at == Some(PanicEffectPhase::Abort) {
            panic!("deliberate transactional abort panic");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct PanickingCompensatableEffect {
    pub(super) name: &'static str,
    pub(super) panic_at: Option<PanicEffectPhase>,
    pub(super) log: Arc<Mutex<Vec<String>>>,
}

impl RuntimeCompensatableEffect for PanickingCompensatableEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn apply(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:apply", self.name));
        if self.panic_at == Some(PanicEffectPhase::Apply) {
            panic!("deliberate compensatable apply panic");
        }
        Ok(())
    }

    fn compensate(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:compensate", self.name));
        if self.panic_at == Some(PanicEffectPhase::Compensate) {
            panic!("deliberate compensatable compensate panic");
        }
        Ok(())
    }
}

#[derive(Debug)]
pub(super) struct PanickingAfterCommitEffect {
    pub(super) name: &'static str,
    pub(super) panic_at: Option<PanicEffectPhase>,
    pub(super) log: Arc<Mutex<Vec<String>>>,
}

impl RuntimeAfterCommitEffect for PanickingAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        synthetic_metadata(self.name)
    }

    fn deliver(&mut self) -> MResult<()> {
        record(&self.log, format!("{}:deliver", self.name));
        if self.panic_at == Some(PanicEffectPhase::Deliver) {
            panic!("deliberate after-commit delivery panic");
        }
        Ok(())
    }
}
