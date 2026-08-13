// Capability methods
// ---------------------------------------------------------------------------

// These methods manage capabilities within the runtime, allowing for granting, revoking, and checking capabilities. A capability represents a permission or access right to perform certain actions or access certain resources. In Mech, they are used to control access to various runtime features and resources, ensuring that actors and tasks can only perform actions they are authorized for, granting fine-grained control over resources and actions in the runtime, etc.
//
// The methods include:
// - `grant_capability`: Grants a capability to the runtime and emits a CapabilityGranted event.
// - `revoke_capability`: Revokes a capability from the runtime and emits a CapabilityRevoked event.
// - `check_capability`: Checks if a capability request is valid and returns the corresponding CapabilityId if it is.
// - `get_capability`: Retrieves a capability by its ID.

// Like with actors, there is a _with_context version of each method, allowing for transactional operations and proper event emission within the context of an active transaction.

use crate::runtime::MechRuntime;
use crate::runtime::extension::{invoke_extension, invoke_extension_value};
use crate::{
    Capability, CapabilityAlreadyExistsError, CapabilityGrant, CapabilityId, CapabilityKernel,
    CapabilityNotFoundError, CapabilityNotRevocableError, CapabilityRequest, CapabilityRevocation,
    RuntimeAuthorityScope, RuntimeCapabilityGrantRollbackFailed, RuntimeCapabilitySnapshot,
    RuntimeContext, RuntimeEventKind, RuntimeInvalidOperationError,
    RuntimeTransactionNotFoundError,
};
use mech_core::{MResult, MechError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

#[derive(Clone, Debug)]
pub(in crate::runtime) enum RuntimeCapabilityMutation {
    Grant(Arc<dyn Capability>),
    Revoke(CapabilityId),
    Use(CapabilityId),
}

#[derive(Clone, Debug, Default)]
pub(in crate::runtime) struct RuntimeCapabilityOverlay {
    operations: Vec<RuntimeCapabilityMutation>,
    grants: HashMap<CapabilityId, Arc<dyn Capability>>,
    grant_order: Vec<CapabilityId>,
    revocations: HashSet<CapabilityId>,
    uses: HashMap<CapabilityId, u64>,
    usage_order: Vec<CapabilityId>,
}

impl RuntimeCapabilityOverlay {
    pub(in crate::runtime) fn mark(&self) -> usize {
        self.operations.len()
    }

    pub(in crate::runtime) fn is_empty(&self) -> bool {
        self.grants.is_empty() && self.revocations.is_empty() && self.uses.is_empty()
    }

    pub(in crate::runtime) fn stage_grant(
        &mut self,
        capability: Arc<dyn Capability>,
    ) -> MResult<()> {
        self.stage_operation(RuntimeCapabilityMutation::Grant(capability))
    }

    pub(in crate::runtime) fn stage_revocation(&mut self, capability: CapabilityId) -> MResult<()> {
        self.stage_operation(RuntimeCapabilityMutation::Revoke(capability))
    }

    pub(in crate::runtime) fn stage_use(&mut self, capability: CapabilityId) -> MResult<()> {
        self.stage_operation(RuntimeCapabilityMutation::Use(capability))
    }

    pub(in crate::runtime) fn rollback_to(&mut self, mark: usize) -> MResult<()> {
        if mark > self.operations.len() {
            return Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "rollback_capability_overlay",
                    reason: format!(
                        "capability savepoint mark {} exceeds overlay length {}",
                        mark,
                        self.operations.len(),
                    ),
                },
                None,
            ));
        }
        self.operations.truncate(mark);
        self.rebuild()
    }

    pub(in crate::runtime) fn provisional(
        &self,
        capability: CapabilityId,
    ) -> Option<Arc<dyn Capability>> {
        self.grants.get(&capability).cloned()
    }

    pub(in crate::runtime) fn check(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<Option<CapabilityId>> {
        let selected = self.preview_check(request, scope)?;
        if let Some(capability) = selected {
            self.stage_use(capability)?;
        }
        Ok(selected)
    }

    pub(in crate::runtime) fn preview_check(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<Option<CapabilityId>> {
        for id in &self.grant_order {
            if !scope.contains(*id) {
                continue;
            }
            let Some(capability) = self.grants.get(id) else {
                continue;
            };
            let subject = invoke_extension_value("capability", "subject_key", || {
                capability.subject_key().to_string()
            })?;
            if subject != request.subject {
                continue;
            }
            let max_uses =
                invoke_extension_value("capability", "max_uses", || capability.max_uses())?;
            if let Some(max_uses) = max_uses {
                let actual = self.uses.get(id).copied().unwrap_or(0);
                if actual >= max_uses {
                    continue;
                }
            }
            let decision = invoke_extension("capability", "preview_check", || {
                capability.preview_check(request)
            })?;
            if decision.allowed {
                return Ok(Some(*id));
            }
        }
        Ok(None)
    }

    pub(in crate::runtime) fn grants(
        &self,
    ) -> impl Iterator<Item = (CapabilityId, Arc<dyn Capability>)> + '_ {
        self.grant_order.iter().filter_map(|id| {
            self.grants
                .get(id)
                .map(|capability| (*id, capability.clone()))
        })
    }

    pub(in crate::runtime) fn usage_deltas(
        &self,
    ) -> impl Iterator<Item = (CapabilityId, u64)> + '_ {
        self.usage_order.iter().filter_map(|id| {
            let uses = self.uses.get(id).copied().unwrap_or(0);
            (uses != 0).then_some((*id, uses))
        })
    }

    pub(in crate::runtime) fn pending_uses(&self) -> &HashMap<CapabilityId, u64> {
        &self.uses
    }

    pub(in crate::runtime) fn revocations(&self) -> impl Iterator<Item = CapabilityId> + '_ {
        self.revocations.iter().copied()
    }

    pub(in crate::runtime) fn revocation_ids(&self) -> HashSet<CapabilityId> {
        self.revocations.clone()
    }

    fn stage_operation(&mut self, operation: RuntimeCapabilityMutation) -> MResult<()> {
        self.operations.push(operation);
        if let Err(error) = self.rebuild() {
            self.operations.pop();
            if let Err(rollback_error) = self.rebuild() {
                return Err(rollback_error.with_source(error));
            }
            return Err(error);
        }
        Ok(())
    }

    fn rebuild(&mut self) -> MResult<()> {
        self.grants.clear();
        self.grant_order.clear();
        self.revocations.clear();
        self.uses.clear();
        self.usage_order.clear();
        for operation in &self.operations {
            match operation {
                RuntimeCapabilityMutation::Grant(capability) => {
                    let id = invoke_extension_value("capability", "id", || capability.id())?;
                    self.revocations.remove(&id);
                    if !self.grants.contains_key(&id) {
                        self.grant_order.push(id);
                    }
                    self.grants.insert(id, capability.clone());
                }
                RuntimeCapabilityMutation::Revoke(capability) => {
                    if self.grants.remove(capability).is_some() {
                        self.grant_order.retain(|id| id != capability);
                        self.uses.remove(capability);
                        self.usage_order.retain(|id| id != capability);
                    } else {
                        self.revocations.insert(*capability);
                    }
                }
                RuntimeCapabilityMutation::Use(capability) => {
                    if self.revocations.contains(capability) {
                        return Err(MechError::new(
                            RuntimeInvalidOperationError {
                                operation: "rebuild_capability_overlay",
                                reason: format!(
                                    "capability {} was used after transaction-local revocation",
                                    capability,
                                ),
                            },
                            None,
                        ));
                    }
                    if !self.uses.contains_key(capability) {
                        self.usage_order.push(*capability);
                    }
                    let current = self.uses.get(capability).copied().unwrap_or(0);
                    let next = Self::incremented_usage(*capability, current)?;
                    self.uses.insert(*capability, next);
                }
            }
        }
        Ok(())
    }

    fn incremented_usage(capability: CapabilityId, current: u64) -> MResult<u64> {
        current.checked_add(1).ok_or_else(|| {
            MechError::new(
                RuntimeInvalidOperationError {
                    operation: "rebuild_capability_overlay",
                    reason: format!(
                        "capability {} transaction-local usage count overflowed",
                        capability,
                    ),
                },
                None,
            )
        })
    }
}

pub(in crate::runtime) fn check_transactional_capability(
    capability_kernel: &mut dyn CapabilityKernel,
    overlay: &mut RuntimeCapabilityOverlay,
    scope: &RuntimeAuthorityScope,
    request: &CapabilityRequest,
) -> MResult<CapabilityId> {
    if let Some(capability) = overlay.check(request, scope)? {
        return Ok(capability);
    }
    let revocations = overlay.revocation_ids();
    let pending_uses = overlay.pending_uses().clone();
    let capability = capability_kernel.preview_scoped_with_transaction(
        request,
        scope,
        &revocations,
        &pending_uses,
    )?;
    overlay.stage_use(capability)?;
    Ok(capability)
}

fn finish_failed_capability_grant(
    capability: CapabilityId,
    original: MechError,
    rollback_failures: Vec<(&'static str, MechError)>,
) -> MechError {
    if rollback_failures.is_empty() {
        return original;
    }

    let rollback_failures = rollback_failures
        .into_iter()
        .map(|(component, error)| format!("{component}: {}", error.full_chain_message()))
        .collect();

    MechError::new(
        RuntimeCapabilityGrantRollbackFailed {
            capability,
            rollback_failures,
        },
        None,
    )
    .with_source(original)
}

impl MechRuntime {
    pub fn get_capability_snapshot(
        &self,
        id: CapabilityId,
    ) -> MResult<Option<RuntimeCapabilitySnapshot>> {
        let Some(capability) = self.capability_kernel.get(id)? else {
            return Ok(None);
        };
        let snapshot =
            invoke_extension_value("capability", "snapshot", || RuntimeCapabilitySnapshot {
                id: capability.id(),
                subject: capability.subject_key().to_string(),
                revocable: capability.is_revocable(),
                delegable: capability.is_delegable(),
                attenuable: capability.is_attenuable(),
                max_uses: capability.max_uses(),
            })?;
        Ok(Some(snapshot))
    }

    pub fn grant_capability_with_context(
        &mut self,
        context: &mut RuntimeContext,
        capability: Arc<dyn Capability>,
    ) -> MResult<CapabilityId> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("grant_capability_with_context")?;
        self.ensure_runtime_mutation_allowed("grant_capability_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        invoke_extension("capability", "validate", || capability.validate())?;

        let id = invoke_extension_value("capability", "id", || capability.id())?;

        if let Some(transaction_id) = context.transaction {
            if self
                .active_execution_transaction(transaction_id)?
                .capabilities
                .provisional(id)
                .is_some()
                || self.capability_kernel.get(id)?.is_some()
            {
                return Err(MechError::new(
                    CapabilityAlreadyExistsError { capability: id },
                    None,
                ));
            }

            let transaction = self.active_execution_transaction(transaction_id)?;
            #[cfg(any(test, feature = "runtime_bench_probes"))]
            crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
                transaction.store.gate_a_staged_item_count(),
            );
            let store_before = transaction.store.clone();
            let overlay_mark = transaction.capabilities.mark();
            let context_authority_before = context.authority.clone();

            self.active_execution_transaction_mut(transaction_id)?
                .capabilities
                .stage_grant(capability)?;
            if let Err(error) = self.emit_event_to_context(
                context,
                RuntimeEventKind::CapabilityGranted { capability_id: id },
            ) {
                let transaction = self.active_execution_transaction_mut(transaction_id)?;
                transaction.store = store_before;
                let rollback_result = transaction.capabilities.rollback_to(overlay_mark);
                context.authority = context_authority_before;
                rollback_result?;
                return Err(error);
            }
            context.add_capability(id);
            return Ok(id);
        }

        self.store.grant_capability(id, capability.clone())?;

        if let Err(error) = self
            .capability_kernel
            .grant(CapabilityGrant::new(capability))
        {
            let rollback_failures = match self.store.rollback_capability_grant(id) {
                Ok(()) => Vec::new(),
                Err(rollback_error) => vec![("capability store", rollback_error)],
            };

            return Err(finish_failed_capability_grant(id, error, rollback_failures));
        }

        if let Err(error) = self.emit_event_to_context(
            context,
            RuntimeEventKind::CapabilityGranted { capability_id: id },
        ) {
            let mut rollback_failures = Vec::new();
            if let Err(rollback_error) = self.capability_kernel.rollback_grant(id) {
                rollback_failures.push(("capability kernel", rollback_error));
            }
            if let Err(rollback_error) = self.store.rollback_capability_grant(id) {
                rollback_failures.push(("capability store", rollback_error));
            }

            return Err(finish_failed_capability_grant(id, error, rollback_failures));
        }

        context.add_capability(id);
        Ok(id)
    }

    pub fn grant_capability(&mut self, capability: Arc<dyn Capability>) -> MResult<CapabilityId> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("grant_capability")?;
        self.ensure_runtime_mutation_allowed("grant_capability")?;
        let mut context = self.runtime_context()?;
        self.grant_capability_with_context(&mut context, capability)
    }

    pub fn revoke_capability(&mut self, capability: CapabilityId) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("revoke_capability")?;
        self.ensure_runtime_mutation_allowed("revoke_capability")?;
        let mut context = self.runtime_context()?;
        self.revoke_capability_with_context(&mut context, capability)
    }

    pub fn revoke_capability_with_context(
        &mut self,
        context: &mut RuntimeContext,
        capability: CapabilityId,
    ) -> MResult<()> {
        #[cfg(feature = "resident-routing")]
        self.ensure_resident_environment_mutable("revoke_capability_with_context")?;
        self.ensure_runtime_mutation_allowed("revoke_capability_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;

        if let Some(transaction_id) = context.transaction {
            let staged = self
                .active_execution_transaction(transaction_id)?
                .capabilities
                .provisional(capability);
            let live = if staged.is_none() {
                self.capability_kernel.get(capability)?
            } else {
                None
            };
            let Some(existing) = staged.or(live) else {
                return Err(MechError::new(CapabilityNotFoundError { capability }, None));
            };
            let revocable =
                invoke_extension_value("capability", "is_revocable", || existing.is_revocable())?;
            if !revocable {
                return Err(MechError::new(
                    CapabilityNotRevocableError { capability },
                    None,
                ));
            }

            let transaction = self.active_execution_transaction(transaction_id)?;
            #[cfg(any(test, feature = "runtime_bench_probes"))]
            crate::runtime::gate_a_probe::record_runtime_transaction_savepoint_clone(
                transaction.store.gate_a_staged_item_count(),
            );
            let store_before = transaction.store.clone();
            let overlay_mark = transaction.capabilities.mark();
            let context_authority_before = context.authority.clone();

            self.active_execution_transaction_mut(transaction_id)?
                .capabilities
                .stage_revocation(capability)?;
            context.remove_capability(capability);
            if let Err(error) = self.emit_event_to_context(
                context,
                RuntimeEventKind::CapabilityRevoked {
                    capability_id: capability,
                },
            ) {
                let transaction = self.active_execution_transaction_mut(transaction_id)?;
                transaction.store = store_before;
                let rollback_result = transaction.capabilities.rollback_to(overlay_mark);
                context.authority = context_authority_before;
                rollback_result?;
                return Err(error);
            }
            return Ok(());
        }

        self.capability_kernel
            .revoke(CapabilityRevocation::new(capability))?;

        self.store.revoke_capability(capability)?;
        context.remove_capability(capability);

        self.emit_event_to_context(
            context,
            RuntimeEventKind::CapabilityRevoked {
                capability_id: capability,
            },
        )?;

        Ok(())
    }

    pub fn check_capability(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        self.ensure_runtime_mutation_allowed("check_capability")?;
        self.capability_kernel
            .check_scoped(request, &RuntimeAuthorityScope::AllForSubject)
    }

    pub fn check_capability_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: &CapabilityRequest,
    ) -> MResult<CapabilityId> {
        self.ensure_runtime_mutation_allowed("check_capability_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        self.check_capability_for_execution(context, request)
    }

    pub(in crate::runtime) fn check_capability_for_execution(
        &mut self,
        context: &RuntimeContext,
        request: &CapabilityRequest,
    ) -> MResult<CapabilityId> {
        self.validate_context_for_runtime(context)?;
        if let Some(transaction_id) = context.transaction {
            let transaction = self
                .active_transactions
                .get_mut(&transaction_id)
                .ok_or_else(|| {
                    MechError::new(RuntimeTransactionNotFoundError { transaction_id }, None)
                })?;
            return check_transactional_capability(
                self.capability_kernel.as_mut(),
                &mut transaction.capabilities,
                &context.authority,
                request,
            );
        }
        self.capability_kernel
            .check_scoped(request, &context.authority)
    }

    pub(in crate::runtime) fn preview_capability_with_context(
        &mut self,
        context: &mut RuntimeContext,
        request: &CapabilityRequest,
    ) -> MResult<CapabilityId> {
        self.ensure_runtime_mutation_allowed("preview_capability_with_context")?;
        self.validate_context_for_runtime(context)?;
        context.charge_step()?;
        self.preview_capability_for_execution(context, request)
    }

    pub(in crate::runtime) fn preview_capability_for_execution(
        &self,
        context: &RuntimeContext,
        request: &CapabilityRequest,
    ) -> MResult<CapabilityId> {
        self.validate_context_for_runtime(context)?;
        if let Some(transaction_id) = context.transaction {
            let provisional = self
                .active_execution_transaction(transaction_id)?
                .capabilities
                .preview_check(request, &context.authority)?;
            if let Some(capability) = provisional {
                return Ok(capability);
            }
            let transaction = self.active_execution_transaction(transaction_id)?;
            let revocations = transaction.capabilities.revocation_ids();
            let pending_uses = transaction.capabilities.pending_uses().clone();
            return self.capability_kernel.preview_scoped_with_transaction(
                request,
                &context.authority,
                &revocations,
                &pending_uses,
            );
        }
        self.capability_kernel.preview_scoped_with_transaction(
            request,
            &context.authority,
            &HashSet::new(),
            &HashMap::new(),
        )
    }

    pub(crate) fn get_capability(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        self.store.get_capability(id)
    }
}

#[cfg(test)]
#[path = "tests/capabilities/mod.rs"]
mod tests;
