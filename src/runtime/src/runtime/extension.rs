use mech_core::{MResult, MechError};
use std::collections::{HashMap, HashSet};
use std::sync::Arc;

use crate::RuntimeStoreCommitIndeterminate;
pub(crate) use crate::extension::{catch_extension, invoke_extension, invoke_extension_value};
use crate::{
    ActorId, ActorRecord, Capability, CapabilityDerivation, CapabilityGrant, CapabilityId,
    CapabilityKernel, CapabilityKernelCheckpoint, CapabilityRequest, CapabilityRevocation, EventId,
    MechStore, MessageId, MessageRecord, ModuleId, ModuleRecord, ModuleVersionId,
    ModuleVersionRecord, ObjectId, ObjectRecord, RuntimeAuthorityScope, RuntimeEvent,
    RuntimeStoreCommit, Subject, TaskId, TaskRecord, TransactionId, TransactionRecord,
};

pub(crate) struct RuntimeCapabilityKernelBoundary {
    inner: Box<dyn CapabilityKernel>,
}

impl RuntimeCapabilityKernelBoundary {
    pub(crate) fn new(inner: Box<dyn CapabilityKernel>) -> Self {
        Self { inner }
    }
}

impl std::fmt::Debug for RuntimeCapabilityKernelBoundary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("RuntimeCapabilityKernelBoundary")
            .field(&self.inner)
            .finish()
    }
}

macro_rules! capability_kernel_extension {
    ($operation:literal, $callback:expr) => {
        invoke_extension("capability kernel", $operation, $callback)
    };
}

impl CapabilityKernel for RuntimeCapabilityKernelBoundary {
    fn checkpoint(&self) -> MResult<Box<dyn CapabilityKernelCheckpoint>> {
        capability_kernel_extension!("checkpoint", || self.inner.checkpoint())
    }

    fn restore(&mut self, checkpoint: Box<dyn CapabilityKernelCheckpoint>) -> MResult<()> {
        capability_kernel_extension!("restore", || { self.inner.restore(checkpoint) })
    }

    fn grant(&mut self, grant: CapabilityGrant) -> MResult<CapabilityId> {
        capability_kernel_extension!("grant", || self.inner.grant(grant))
    }

    fn rollback_grant(&mut self, capability: CapabilityId) -> MResult<()> {
        capability_kernel_extension!("rollback_grant", || {
            self.inner.rollback_grant(capability)
        })
    }

    fn revoke(&mut self, revocation: CapabilityRevocation) -> MResult<()> {
        capability_kernel_extension!("revoke", || self.inner.revoke(revocation))
    }

    fn check(&mut self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        capability_kernel_extension!("check", || self.inner.check(request))
    }

    fn check_scoped(
        &mut self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
    ) -> MResult<CapabilityId> {
        capability_kernel_extension!("check_scoped", || {
            self.inner.check_scoped(request, scope)
        })
    }

    fn preview_check(&self, request: &CapabilityRequest) -> MResult<CapabilityId> {
        capability_kernel_extension!("preview_check", || { self.inner.preview_check(request) })
    }

    fn check_excluding(
        &mut self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        capability_kernel_extension!("check_excluding", || {
            self.inner.check_excluding(request, excluded)
        })
    }

    fn preview_check_excluding(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
    ) -> MResult<CapabilityId> {
        capability_kernel_extension!("preview_check_excluding", || {
            self.inner.preview_check_excluding(request, excluded)
        })
    }

    fn preview_check_excluding_with_pending_uses(
        &self,
        request: &CapabilityRequest,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        capability_kernel_extension!("preview_check_excluding_with_pending_uses", || {
            self.inner
                .preview_check_excluding_with_pending_uses(request, excluded, pending_uses)
        })
    }

    fn preview_scoped_with_transaction(
        &self,
        request: &CapabilityRequest,
        scope: &RuntimeAuthorityScope,
        excluded: &HashSet<CapabilityId>,
        pending_uses: &HashMap<CapabilityId, u64>,
    ) -> MResult<CapabilityId> {
        capability_kernel_extension!("preview_scoped_with_transaction", || {
            self.inner
                .preview_scoped_with_transaction(request, scope, excluded, pending_uses)
        })
    }

    fn apply_usage_delta(&mut self, capability: CapabilityId, uses: u64) -> MResult<()> {
        capability_kernel_extension!("apply_usage_delta", || {
            self.inner.apply_usage_delta(capability, uses)
        })
    }

    fn get(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        capability_kernel_extension!("get", || self.inner.get(id))
    }

    fn list_for_subject(&self, subject: &dyn Subject) -> MResult<Vec<CapabilityId>> {
        capability_kernel_extension!("list_for_subject", || {
            self.inner.list_for_subject(subject)
        })
    }

    fn derive_capability(&mut self, derivation: CapabilityDerivation) -> MResult<CapabilityId> {
        capability_kernel_extension!("derive_capability", || {
            self.inner.derive_capability(derivation)
        })
    }

    fn is_revoked(&self, id: CapabilityId) -> MResult<bool> {
        capability_kernel_extension!("is_revoked", || self.inner.is_revoked(id))
    }
}

pub(crate) struct RuntimeStoreBoundary {
    inner: Box<dyn MechStore>,
}

impl RuntimeStoreBoundary {
    pub(crate) fn new(inner: Box<dyn MechStore>) -> Self {
        Self { inner }
    }
}

impl std::fmt::Debug for RuntimeStoreBoundary {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_tuple("RuntimeStoreBoundary")
            .field(&self.inner)
            .finish()
    }
}

macro_rules! store_extension {
    ($operation:literal, $callback:expr) => {
        invoke_extension("store", $operation, $callback)
    };
}

impl MechStore for RuntimeStoreBoundary {
    fn put_module(&mut self, module: ModuleRecord) -> MResult<ModuleId> {
        store_extension!("put_module", || self.inner.put_module(module))
    }

    fn get_module(&self, id: ModuleId) -> MResult<Option<ModuleRecord>> {
        store_extension!("get_module", || self.inner.get_module(id))
    }

    fn find_module_by_name(&self, name: &str) -> MResult<Option<ModuleRecord>> {
        store_extension!("find_module_by_name", || self
            .inner
            .find_module_by_name(name))
    }

    fn put_module_version(&mut self, version: ModuleVersionRecord) -> MResult<ModuleVersionId> {
        store_extension!("put_module_version", || self
            .inner
            .put_module_version(version))
    }

    fn get_module_version(&self, id: ModuleVersionId) -> MResult<Option<ModuleVersionRecord>> {
        store_extension!("get_module_version", || self.inner.get_module_version(id))
    }

    fn set_active_module_version(
        &mut self,
        module: ModuleId,
        version: ModuleVersionId,
    ) -> MResult<()> {
        store_extension!("set_active_module_version", || {
            self.inner.set_active_module_version(module, version)
        })
    }

    fn get_active_module_version(&self, module: ModuleId) -> MResult<Option<ModuleVersionId>> {
        store_extension!("get_active_module_version", || {
            self.inner.get_active_module_version(module)
        })
    }

    fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        store_extension!("put_object", || self.inner.put_object(object))
    }

    fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
        store_extension!("get_object", || self.inner.get_object(id))
    }

    fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        store_extension!("update_object", || self.inner.update_object(object))
    }

    fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
        store_extension!("put_task", || self.inner.put_task(task))
    }

    fn get_task(&self, id: TaskId) -> MResult<Option<TaskRecord>> {
        store_extension!("get_task", || self.inner.get_task(id))
    }

    fn update_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
        store_extension!("update_task", || self.inner.update_task(task))
    }

    fn task_count(&self) -> MResult<u64> {
        store_extension!("task_count", || self.inner.task_count())
    }

    fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        store_extension!("put_actor", || self.inner.put_actor(actor))
    }

    fn get_actor(&self, id: ActorId) -> MResult<Option<ActorRecord>> {
        store_extension!("get_actor", || self.inner.get_actor(id))
    }

    fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        store_extension!("update_actor", || self.inner.update_actor(actor))
    }

    fn actor_count(&self) -> MResult<u64> {
        store_extension!("actor_count", || self.inner.actor_count())
    }

    fn enqueue_message(&mut self, actor: ActorId, message: MessageRecord) -> MResult<MessageId> {
        store_extension!("enqueue_message", || {
            self.inner.enqueue_message(actor, message)
        })
    }

    fn mailbox_len(&self, actor: ActorId) -> MResult<u64> {
        store_extension!("mailbox_len", || self.inner.mailbox_len(actor))
    }

    fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        store_extension!("peek_message", || self.inner.peek_message(actor))
    }

    fn list_mailbox(&self, actor: ActorId) -> MResult<Vec<MessageRecord>> {
        store_extension!("list_mailbox", || self.inner.list_mailbox(actor))
    }

    fn ack_message(&mut self, actor: ActorId, message: MessageId) -> MResult<()> {
        store_extension!("ack_message", || self.inner.ack_message(actor, message))
    }

    fn pop_message(&mut self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        store_extension!("pop_message", || self.inner.pop_message(actor))
    }

    fn grant_capability(
        &mut self,
        id: CapabilityId,
        capability: Arc<dyn Capability>,
    ) -> MResult<CapabilityId> {
        store_extension!("grant_capability", || {
            self.inner.grant_capability(id, capability)
        })
    }

    fn rollback_capability_grant(&mut self, id: CapabilityId) -> MResult<()> {
        store_extension!("rollback_capability_grant", || {
            self.inner.rollback_capability_grant(id)
        })
    }

    fn get_capability(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        store_extension!("get_capability", || self.inner.get_capability(id))
    }

    fn list_capabilities_for_subject(&self, subject_key: &str) -> MResult<Vec<CapabilityId>> {
        store_extension!("list_capabilities_for_subject", || {
            self.inner.list_capabilities_for_subject(subject_key)
        })
    }

    fn revoke_capability(&mut self, id: CapabilityId) -> MResult<()> {
        store_extension!("revoke_capability", || self.inner.revoke_capability(id))
    }

    fn is_capability_revoked(&self, id: CapabilityId) -> MResult<bool> {
        store_extension!("is_capability_revoked", || {
            self.inner.is_capability_revoked(id)
        })
    }

    fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId> {
        store_extension!("append_event", || self.inner.append_event(event))
    }

    fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>> {
        store_extension!("get_event", || self.inner.get_event(id))
    }

    fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>> {
        store_extension!("list_events", || self.inner.list_events(limit))
    }

    fn configure_event_retention(&mut self, max_events: Option<usize>) -> MResult<()> {
        store_extension!("configure_event_retention", || {
            self.inner.configure_event_retention(max_events)
        })
    }

    fn commit_runtime(&mut self, commit: RuntimeStoreCommit) -> MResult<TransactionId> {
        let transaction_id = commit.transaction.id;
        match catch_extension("store", "commit_runtime", || {
            self.inner.commit_runtime(commit)
        }) {
            Ok(result) => result,
            Err(panic) => Err(MechError::new(
                RuntimeStoreCommitIndeterminate {
                    transaction_id,
                    payload: panic.payload,
                },
                None,
            )),
        }
    }

    fn commit_transaction(&mut self, transaction: TransactionRecord) -> MResult<TransactionId> {
        store_extension!("commit_transaction", || {
            self.inner.commit_transaction(transaction)
        })
    }

    fn get_transaction(&self, id: TransactionId) -> MResult<Option<TransactionRecord>> {
        store_extension!("get_transaction", || self.inner.get_transaction(id))
    }

    fn list_transactions(&self, limit: Option<usize>) -> MResult<Vec<TransactionRecord>> {
        store_extension!("list_transactions", || {
            self.inner.list_transactions(limit)
        })
    }
}
