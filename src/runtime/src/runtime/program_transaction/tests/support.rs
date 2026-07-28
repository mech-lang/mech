use super::super::{MResult, MechError, RuntimeInvalidOperationError};
use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeEffectMetadata, RuntimeEffectSource,
    RuntimeTransactionalEffect,
};
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct SavepointAfterCommitEffect {
    name: &'static str,
}

impl RuntimeAfterCommitEffect for SavepointAfterCommitEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: self.name.to_string(),
            },
            "savepoint-test",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        Ok(())
    }
}

pub(super) fn savepoint_effect(name: &'static str) -> PreparedRuntimeEffect {
    PreparedRuntimeEffect::AfterCommit(Box::new(SavepointAfterCommitEffect { name }))
}

#[derive(Debug)]
pub(super) struct CommitDecisionEffect {
    pub(super) name: &'static str,
    pub(super) log: Arc<Mutex<Vec<String>>>,
    pub(super) fail_commit: bool,
}

impl RuntimeTransactionalEffect for CommitDecisionEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::HostFunction {
                name: self.name.to_string(),
            },
            "commit-decision",
        )
    }

    fn prepare(&mut self) -> MResult<()> {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:prepare", self.name));
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:commit", self.name));
        if self.fail_commit {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "commit_decision_test",
                    reason: format!("{} deliberate commit failure", self.name),
                },
                None,
            ));
        }
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.log
            .lock()
            .unwrap()
            .push(format!("{}:abort", self.name));
        Ok(())
    }
}
