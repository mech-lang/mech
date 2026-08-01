// Runtime Errors
// -----------------------------------------------------------------------------

// This module defines error types used by the runtime. These errors capture various failure modes that can occur during the runtime's operation, such as module resolution failures, dependency cycles, record not found errors, invalid operations, and more.

// See /src/core/src/error.rs for the base error types and traits used by these runtime errors.

use super::transaction::RuntimePoisonRecord;
use crate::ModuleVersionId;
use crate::{ActiveRuntimeEffectPhase, CapabilityId, RuntimeEffectFailure, TransactionId};
use mech_core::MechErrorKind;

#[derive(Debug, Clone)]
pub struct RuntimeEffectOperationReentrant {
    pub active_phase: ActiveRuntimeEffectPhase,
    pub requested_operation: &'static str,
}

impl MechErrorKind for RuntimeEffectOperationReentrant {
    fn name(&self) -> &str {
        "RuntimeEffectOperationReentrant"
    }

    fn message(&self) -> String {
        format!(
            "runtime operation `{}` cannot run during effect phase {:?}",
            self.requested_operation, self.active_phase,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeEffectCleanupFailed {
    pub operation: &'static str,
    pub transaction_id: TransactionId,
    pub original_error: String,
    pub cleanup_failures: Vec<String>,
}

impl MechErrorKind for RuntimeEffectCleanupFailed {
    fn name(&self) -> &str {
        "RuntimeEffectCleanupFailed"
    }

    fn message(&self) -> String {
        format!(
            "runtime effect operation `{}` failed ({}) and cleanup was incomplete for transaction {}: {}",
            self.operation,
            self.original_error,
            self.transaction_id,
            self.cleanup_failures.join("; "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeExternalCommitIndeterminate {
    pub transaction_id: TransactionId,
    pub failures: Vec<RuntimeEffectFailure>,
    pub participant_outcomes: Vec<String>,
}

impl MechErrorKind for RuntimeExternalCommitIndeterminate {
    fn name(&self) -> &str {
        "RuntimeExternalCommitIndeterminate"
    }

    fn message(&self) -> String {
        format!(
            "runtime store transaction {} committed, but {} external participants have indeterminate commit outcomes: {}",
            self.transaction_id,
            self.failures.len(),
            self.participant_outcomes.join("; "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeTransactionContextMismatch {
    pub transaction_id: TransactionId,
    pub reason: String,
}

impl MechErrorKind for RuntimeTransactionContextMismatch {
    fn name(&self) -> &str {
        "RuntimeTransactionContextMismatch"
    }

    fn message(&self) -> String {
        format!(
            "runtime transaction {} context identity mismatch: {}",
            self.transaction_id, self.reason,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeProgramBusy {
    pub operation: &'static str,
    pub owner: TransactionId,
    pub requester: Option<TransactionId>,
}

impl MechErrorKind for RuntimeProgramBusy {
    fn name(&self) -> &str {
        "RuntimeProgramBusy"
    }

    fn message(&self) -> String {
        format!(
            "retained program operation `{}` is owned by transaction {}; requester is {:?}",
            self.operation, self.owner, self.requester,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeProgramOperationReentrant {
    pub active_operation: &'static str,
    pub requested_operation: &'static str,
    pub transaction_id: TransactionId,
}

impl MechErrorKind for RuntimeProgramOperationReentrant {
    fn name(&self) -> &str {
        "RuntimeProgramOperationReentrant"
    }

    fn message(&self) -> String {
        format!(
            "retained program operation `{}` cannot enter `{}` recursively in transaction {}",
            self.active_operation, self.requested_operation, self.transaction_id,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeTransactionalLiveRegistrationUnsupported {
    pub transaction_id: TransactionId,
    pub owner: Option<TransactionId>,
    pub active_operation: Option<&'static str>,
}

impl MechErrorKind for RuntimeTransactionalLiveRegistrationUnsupported {
    fn name(&self) -> &str {
        "RuntimeTransactionalLiveRegistrationUnsupported"
    }

    fn message(&self) -> String {
        format!(
            "transaction {} may register retained live state only inside its coordinated program operation (owner {:?}, active operation {:?})",
            self.transaction_id, self.owner, self.active_operation,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimePoisoned {
    pub operation: &'static str,
    pub poison: RuntimePoisonRecord,
}

impl MechErrorKind for RuntimePoisoned {
    fn name(&self) -> &str {
        "RuntimePoisoned"
    }

    fn message(&self) -> String {
        format!(
            "runtime operation `{}` rejected because `{}` rollback poisoned the runtime: {}",
            self.operation,
            self.poison.operation,
            self.poison.rollback_failures.join("; "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeProgramRollbackFailed {
    pub operation: &'static str,
    pub transaction_id: Option<TransactionId>,
    pub original_error: String,
    pub rollback_failures: Vec<String>,
}

impl MechErrorKind for RuntimeProgramRollbackFailed {
    fn name(&self) -> &str {
        "RuntimeProgramRollbackFailed"
    }

    fn message(&self) -> String {
        format!(
            "retained program operation `{}` failed ({}) and rollback was incomplete for transaction {:?}: {}",
            self.operation,
            self.original_error,
            self.transaction_id,
            self.rollback_failures.join("; "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeCapabilityGrantRollbackFailed {
    pub capability: CapabilityId,
    pub rollback_failures: Vec<String>,
}

impl MechErrorKind for RuntimeCapabilityGrantRollbackFailed {
    fn name(&self) -> &str {
        "RuntimeCapabilityGrantRollbackFailed"
    }

    fn message(&self) -> String {
        format!(
            "capability grant {} failed and compensation was incomplete: {}",
            self.capability,
            self.rollback_failures.join("; "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleDependencyCycleError {
    pub cycle: Vec<String>,
}

impl MechErrorKind for RuntimeModuleDependencyCycleError {
    fn name(&self) -> &str {
        "RuntimeModuleDependencyCycle"
    }

    fn message(&self) -> String {
        format!(
            "module dependency cycle detected: {}",
            self.cycle.join(" -> "),
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeRecordNotFoundError {
    pub record_type: &'static str,
    pub id: String,
}

impl MechErrorKind for RuntimeRecordNotFoundError {
    fn name(&self) -> &str {
        "RuntimeRecordNotFound"
    }

    fn message(&self) -> String {
        format!("{} record not found: {}", self.record_type, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeInvalidOperationError {
    pub operation: &'static str,
    pub reason: String,
}

impl MechErrorKind for RuntimeInvalidOperationError {
    fn name(&self) -> &str {
        "RuntimeInvalidOperation"
    }

    fn message(&self) -> String {
        format!(
            "Invalid runtime operation `{}`: {}",
            self.operation, self.reason
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleExportNotFound {
    pub dependency: String,
    pub export: String,
}

impl MechErrorKind for RuntimeModuleExportNotFound {
    fn name(&self) -> &str {
        "RuntimeModuleExportNotFound"
    }

    fn message(&self) -> String {
        format!(
            "module `{}` does not export `{}`",
            self.dependency, self.export
        )
    }
}

#[derive(Debug, Clone)]
pub struct UnknownAddressTarget {
    pub target: String,
}

impl MechErrorKind for UnknownAddressTarget {
    fn name(&self) -> &str {
        "UnknownAddressTarget"
    }

    fn message(&self) -> String {
        format!("unknown address target `{}`", self.target)
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleImportConflict {
    pub binding: String,
    pub first_import: String,
    pub second_import: String,
}

impl MechErrorKind for RuntimeModuleImportConflict {
    fn name(&self) -> &str {
        "RuntimeModuleImportConflict"
    }

    fn message(&self) -> String {
        format!(
            "import binding conflict for `{}` between `{}` and `{}`",
            self.binding, self.first_import, self.second_import
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleJournalConflict {
    pub record_type: &'static str,
    pub identity: String,
    pub reason: String,
}

impl MechErrorKind for RuntimeModuleJournalConflict {
    fn name(&self) -> &str {
        "RuntimeModuleJournalConflict"
    }

    fn message(&self) -> String {
        format!(
            "{} `{}` conflicts with the transaction module journal: {}",
            self.record_type, self.identity, self.reason,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleImportEdgeInvalid {
    pub module: ModuleVersionId,
    pub reason: String,
}

impl MechErrorKind for RuntimeModuleImportEdgeInvalid {
    fn name(&self) -> &str {
        "RuntimeModuleImportEdgeInvalid"
    }

    fn message(&self) -> String {
        format!(
            "module `{}` has invalid import edges: {}",
            self.module, self.reason,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeModuleDependencyMissingError {
    pub module: String,
    pub specifier: String,
    pub referrer: Option<String>,
}

impl MechErrorKind for RuntimeModuleDependencyMissingError {
    fn name(&self) -> &str {
        "RuntimeModuleDependencyMissing"
    }

    fn message(&self) -> String {
        match &self.referrer {
            Some(referrer) => format!(
                "module `{}` declared dependency `{}` (referrer `{}`) but it could not be resolved",
                self.module, self.specifier, referrer,
            ),
            None => format!(
                "module `{}` declared dependency `{}` but it could not be resolved",
                self.module, self.specifier,
            ),
        }
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeRootModuleSourceNotFound {
    pub specifier: String,
}

impl MechErrorKind for RuntimeRootModuleSourceNotFound {
    fn name(&self) -> &str {
        "RuntimeRootModuleSourceNotFound"
    }

    fn message(&self) -> String {
        format!(
            "root module source `{}` could not be resolved",
            self.specifier
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeProgramHostNotActiveError {
    pub function: String,
}

impl MechErrorKind for RuntimeProgramHostNotActiveError {
    fn name(&self) -> &str {
        "RuntimeProgramHostNotActive"
    }

    fn message(&self) -> String {
        format!(
            "Runtime host function `{}` was called without an active runtime context",
            self.function,
        )
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeHostFunctionNotBytecodeCompilableError {
    pub function: String,
}

impl MechErrorKind for RuntimeHostFunctionNotBytecodeCompilableError {
    fn name(&self) -> &str {
        "RuntimeHostFunctionNotBytecodeCompilable"
    }

    fn message(&self) -> String {
        format!(
            "Runtime host function `{}` cannot be compiled to bytecode yet",
            self.function,
        )
    }
}
#[derive(Debug, Clone)]
pub struct ActivationScopeEffectWithRegisterUnsupported;
impl MechErrorKind for ActivationScopeEffectWithRegisterUnsupported {
    fn name(&self) -> &str {
        "ActivationScopeEffectWithRegisterUnsupported"
    }
    fn message(&self) -> String {
        "activation scopes cannot mix local register writes and context sends".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeIsolatedActivationSendUnsupported;
impl MechErrorKind for RuntimeIsolatedActivationSendUnsupported {
    fn name(&self) -> &str {
        "RuntimeIsolatedActivationSendUnsupported"
    }
    fn message(&self) -> String {
        "activation-scoped context sends require retained live registration".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct RuntimeActivationEffectBarrierInvariantError {
    pub reason: String,
}
impl MechErrorKind for RuntimeActivationEffectBarrierInvariantError {
    fn name(&self) -> &str {
        "RuntimeActivationEffectBarrierInvariant"
    }
    fn message(&self) -> String {
        self.reason.clone()
    }
}
