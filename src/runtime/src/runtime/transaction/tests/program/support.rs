use crate::{
    MechRuntime, PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeContext,
    RuntimeEffectMetadata, RuntimeEffectSource, RuntimeInvalidOperationError,
    RuntimeTransactionalEffect,
};
use mech_core::{
    ExecutionHostFunctionRequest, LegacyValue, MResult, MechError, MechExecutionServices,
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

/// Executes a host callback inside the transaction already owned by the
/// surrounding program operation. Source-specialized host nodes deliberately
/// preserve their planned output, so transaction tests use the execution
/// service boundary directly when the callback itself is the behavior under
/// test.
pub(super) fn invoke_host_callback(
    runtime: &mut MechRuntime,
    context: &mut RuntimeContext,
    name: &str,
) -> MResult<LegacyValue> {
    runtime.with_runtime_execution_session(context, |services| {
        services.invoke_host_function(
            &ExecutionHostFunctionRequest {
                name: name.to_string(),
            },
            &[],
        )
    })
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
