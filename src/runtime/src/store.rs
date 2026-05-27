//! Store abstraction for the Mech runtime.
//!
//! `MechStore` is the database boundary. It defines what the runtime needs from
//! persistence without selecting a backend by enum.
//!
//! Durable backends should implement `MechStore` directly:
//!
//! - SQLite store
//! - Postgres store
//! - embedded KV store
//! - distributed database store
//! - host-provided store
//!
//! The included `InMemoryStore` is only the default implementation for tests,
//! prototypes, and the first runtime shell.

#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use mech_core::{MResult, MechError, MechErrorKind, MechSourceCode};

use crate::capability::{Capability, CapabilityRequest};
use crate::event::RuntimeEvent;
use crate::id::{
    ActorId, CapabilityId, EventId, MessageId, ModuleId, ModuleVersionId, ObjectId, TaskId,
    TransactionId,
};
use crate::resolver::{SourceExportDeclaration, SourceImportDeclaration};

// -----------------------------------------------------------------------------
// Store Trait
// -----------------------------------------------------------------------------

pub trait MechStore: std::fmt::Debug + Send {
    fn put_module(&mut self, module: ModuleRecord) -> MResult<ModuleId>;

    fn get_module(&self, id: ModuleId) -> MResult<Option<ModuleRecord>>;

    fn find_module_by_name(&self, name: &str) -> MResult<Option<ModuleRecord>>;

    fn put_module_version(&mut self, version: ModuleVersionRecord) -> MResult<ModuleVersionId>;

    fn get_module_version(&self, id: ModuleVersionId) -> MResult<Option<ModuleVersionRecord>>;

    fn set_active_module_version(
        &mut self,
        module: ModuleId,
        version: ModuleVersionId,
    ) -> MResult<()>;

    fn get_active_module_version(&self, module: ModuleId) -> MResult<Option<ModuleVersionId>>;

    fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId>;

    fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>>;

    fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId>;

    fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId>;

    fn get_task(&self, id: TaskId) -> MResult<Option<TaskRecord>>;

    fn update_task(&mut self, task: TaskRecord) -> MResult<TaskId>;

    fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId>;

    fn get_actor(&self, id: ActorId) -> MResult<Option<ActorRecord>>;

    fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId>;

    fn enqueue_message(&mut self, actor: ActorId, message: MessageRecord) -> MResult<MessageId>;

    fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>>;

    fn ack_message(&mut self, actor: ActorId, message: MessageId) -> MResult<()>;

    fn pop_message(&mut self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        let Some(message) = self.peek_message(actor)? else {
            return Ok(None);
        };

        self.ack_message(actor, message.id)?;
        Ok(Some(message))
    }

    fn grant_capability(
        &mut self,
        id: CapabilityId,
        capability: Arc<dyn Capability>,
    ) -> MResult<CapabilityId>;

    fn get_capability(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>>;

    fn list_capabilities_for_subject(&self, subject_key: &str) -> MResult<Vec<CapabilityId>>;

    fn revoke_capability(&mut self, id: CapabilityId) -> MResult<()>;

    fn is_capability_revoked(&self, id: CapabilityId) -> MResult<bool>;

    fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId>;

    fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>>;

    fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>>;

    fn commit_transaction(&mut self, tx: TransactionRecord) -> MResult<TransactionId>;

    fn get_transaction(&self, id: TransactionId) -> MResult<Option<TransactionRecord>>;

    fn list_transactions(&self, limit: Option<usize>) -> MResult<Vec<TransactionRecord>>;
}

// -----------------------------------------------------------------------------
// Module Records
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleRecord {
    pub id: ModuleId,
    pub name: String,
    pub description: Option<String>,
}

impl ModuleRecord {
    pub fn new(id: ModuleId, name: impl Into<String>) -> Self {
        Self {
            id,
            name: name.into(),
            description: None,
        }
    }

    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("module.id", "must not be zero");
        }

        if self.name.trim().is_empty() {
            return invalid_store_record("module.name", "must not be empty");
        }

        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleImportEdge {
    pub import: SourceImportDeclaration,
    pub dependency: ModuleVersionId,
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ModuleVersionRecord {
    pub id: ModuleVersionId,
    pub module: ModuleId,
    pub version: u64,
    pub source: Option<MechSourceCode>,
    pub bytecode: Option<Vec<u8>>,
    pub exports: Vec<SourceExportDeclaration>,
    pub imports: Vec<SourceImportDeclaration>,
    pub dependencies: Vec<ModuleVersionId>,
    pub import_edges: Vec<ModuleImportEdge>,
    pub capability_requirements: Vec<CapabilityRequest>,
}

impl ModuleVersionRecord {
    pub fn new(id: ModuleVersionId, module: ModuleId, version: u64) -> Self {
        Self {
            id,
            module,
            version,
            source: None,
            bytecode: None,
            exports: Vec::new(),
            imports: Vec::new(),
            dependencies: Vec::new(),
            import_edges: Vec::new(),
            capability_requirements: Vec::new(),
        }
    }

    pub fn with_source(mut self, source: MechSourceCode) -> Self {
        self.source = Some(source);
        self
    }

    pub fn with_bytecode(mut self, bytecode: Vec<u8>) -> Self {
        self.bytecode = Some(bytecode);
        self
    }

    pub fn with_dependencies(mut self, dependencies: Vec<ModuleVersionId>) -> Self {
        self.dependencies = dependencies;
        self
    }

    pub fn with_exports(mut self, exports: Vec<SourceExportDeclaration>) -> Self {
        self.exports = exports;
        self
    }

    pub fn with_imports(mut self, imports: Vec<SourceImportDeclaration>) -> Self {
        self.imports = imports;
        self
    }

    pub fn with_import_edges(mut self, import_edges: Vec<ModuleImportEdge>) -> Self {
        self.import_edges = import_edges;
        self
    }

    pub fn with_capability_requirements(mut self, requirements: Vec<CapabilityRequest>) -> Self {
        self.capability_requirements = requirements;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("module_version.id", "must not be zero");
        }

        if self.module.is_zero() {
            return invalid_store_record("module_version.module", "must not be zero");
        }

        if self.version == 0 {
            return invalid_store_record("module_version.version", "must be greater than zero");
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Object Records
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ObjectRecord {
    pub id: ObjectId,
    pub kind: String,
    pub version: u64,
    pub encoding: String,
    pub data: Vec<u8>,
}

impl ObjectRecord {
    pub fn new(
        id: ObjectId,
        kind: impl Into<String>,
        encoding: impl Into<String>,
        data: Vec<u8>,
    ) -> Self {
        Self {
            id,
            kind: kind.into(),
            version: 1,
            encoding: encoding.into(),
            data,
        }
    }

    pub fn text(id: ObjectId, kind: impl Into<String>, text: impl Into<String>) -> Self {
        Self::new(id, kind, "text/plain", text.into().into_bytes())
    }

    pub fn json(id: ObjectId, kind: impl Into<String>, json: impl Into<String>) -> Self {
        Self::new(id, kind, "application/json", json.into().into_bytes())
    }

    pub fn bytes(id: ObjectId, kind: impl Into<String>, data: Vec<u8>) -> Self {
        Self::new(id, kind, "application/octet-stream", data)
    }

    pub fn with_version(mut self, version: u64) -> Self {
        self.version = version;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("object.id", "must not be zero");
        }

        if self.kind.trim().is_empty() {
            return invalid_store_record("object.kind", "must not be empty");
        }

        if self.version == 0 {
            return invalid_store_record("object.version", "must be greater than zero");
        }

        if self.encoding.trim().is_empty() {
            return invalid_store_record("object.encoding", "must not be empty");
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Task Records
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskStatus {
    pub name: String,
}

impl TaskStatus {
    pub fn new(name: impl Into<String>) -> Self {
        Self { name: name.into() }
    }

    pub fn pending() -> Self {
        Self::new("pending")
    }

    pub fn running() -> Self {
        Self::new("running")
    }

    pub fn completed() -> Self {
        Self::new("completed")
    }

    pub fn failed() -> Self {
        Self::new("failed")
    }

    pub fn cancelled() -> Self {
        Self::new("cancelled")
    }

    pub fn validate(&self) -> MResult<()> {
        if self.name.trim().is_empty() {
            return invalid_store_record("task.status", "must not be empty");
        }

        Ok(())
    }
}

impl Default for TaskStatus {
    fn default() -> Self {
        Self::pending()
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskRecord {
    pub id: TaskId,
    pub module_version: Option<ModuleVersionId>,
    pub subject: String,
    pub status: TaskStatus,
    pub capabilities: Vec<CapabilityId>,
}

impl TaskRecord {
    pub fn new(id: TaskId, subject: impl Into<String>) -> Self {
        Self {
            id,
            module_version: None,
            subject: subject.into(),
            status: TaskStatus::pending(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_module_version(mut self, module_version: ModuleVersionId) -> Self {
        self.module_version = Some(module_version);
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<CapabilityId>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn with_status(mut self, status: TaskStatus) -> Self {
        self.status = status;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("task.id", "must not be zero");
        }

        if self.subject.trim().is_empty() {
            return invalid_store_record("task.subject", "must not be empty");
        }

        self.status.validate()
    }
}

// -----------------------------------------------------------------------------
// Actor and Message Records
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ActorRecord {
    pub id: ActorId,
    pub subject: String,
    pub behavior: Option<ModuleVersionId>,
    pub state: Option<ObjectId>,
    pub status: String,
    pub capabilities: Vec<CapabilityId>,
}

impl ActorRecord {
    pub fn new(id: ActorId, subject: impl Into<String>) -> Self {
        Self {
            id,
            subject: subject.into(),
            behavior: None,
            state: None,
            status: "ready".to_string(),
            capabilities: Vec::new(),
        }
    }

    pub fn with_behavior(mut self, behavior: ModuleVersionId) -> Self {
        self.behavior = Some(behavior);
        self
    }

    pub fn with_state(mut self, state: ObjectId) -> Self {
        self.state = Some(state);
        self
    }

    pub fn with_status(mut self, status: impl Into<String>) -> Self {
        self.status = status.into();
        self
    }

    pub fn with_capabilities(mut self, capabilities: Vec<CapabilityId>) -> Self {
        self.capabilities = capabilities;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("actor.id", "must not be zero");
        }

        if self.subject.trim().is_empty() {
            return invalid_store_record("actor.subject", "must not be empty");
        }

        if self.status.trim().is_empty() {
            return invalid_store_record("actor.status", "must not be empty");
        }

        Ok(())
    }
}

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MessageRecord {
    pub id: MessageId,
    pub actor: ActorId,
    pub sender: Option<String>,
    pub kind: String,
    pub payload: Vec<u8>,
}

impl MessageRecord {
    pub fn new(id: MessageId, actor: ActorId, kind: impl Into<String>, payload: Vec<u8>) -> Self {
        Self {
            id,
            actor,
            sender: None,
            kind: kind.into(),
            payload,
        }
    }

    pub fn with_sender(mut self, sender: impl Into<String>) -> Self {
        self.sender = Some(sender.into());
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("message.id", "must not be zero");
        }

        if self.actor.is_zero() {
            return invalid_store_record("message.actor", "must not be zero");
        }

        if self.kind.trim().is_empty() {
            return invalid_store_record("message.kind", "must not be empty");
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// Transactions
// -----------------------------------------------------------------------------

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TransactionRecord {
    pub id: TransactionId,
    pub subject: String,

    pub read_set: Vec<ObjectId>,
    pub write_set: Vec<ObjectId>,

    pub message_acks: Vec<MessageId>,
    pub message_sends: Vec<MessageId>,

    pub task_updates: Vec<TaskId>,
    pub actor_updates: Vec<ActorId>,

    pub events: Vec<EventId>,
}

impl TransactionRecord {
    pub fn new(id: TransactionId, subject: impl Into<String>) -> Self {
        Self {
            id,
            subject: subject.into(),

            read_set: Vec::new(),
            write_set: Vec::new(),

            message_acks: Vec::new(),
            message_sends: Vec::new(),

            task_updates: Vec::new(),
            actor_updates: Vec::new(),

            events: Vec::new(),
        }
    }

    pub fn with_read_set(mut self, read_set: Vec<ObjectId>) -> Self {
        self.read_set = read_set;
        self
    }

    pub fn with_write_set(mut self, write_set: Vec<ObjectId>) -> Self {
        self.write_set = write_set;
        self
    }

    pub fn with_message_acks(mut self, message_acks: Vec<MessageId>) -> Self {
        self.message_acks = message_acks;
        self
    }

    pub fn with_message_sends(mut self, message_sends: Vec<MessageId>) -> Self {
        self.message_sends = message_sends;
        self
    }

    pub fn with_task_updates(mut self, task_updates: Vec<TaskId>) -> Self {
        self.task_updates = task_updates;
        self
    }

    pub fn with_actor_updates(mut self, actor_updates: Vec<ActorId>) -> Self {
        self.actor_updates = actor_updates;
        self
    }

    pub fn with_events(mut self, events: Vec<EventId>) -> Self {
        self.events = events;
        self
    }

    pub fn validate(&self) -> MResult<()> {
        if self.id.is_zero() {
            return invalid_store_record("transaction.id", "must not be zero");
        }

        if self.subject.trim().is_empty() {
            return invalid_store_record("transaction.subject", "must not be empty");
        }

        Ok(())
    }
}

// -----------------------------------------------------------------------------
// In-Memory Store
// -----------------------------------------------------------------------------

#[derive(Clone, Debug, Default)]
pub struct InMemoryStore {
    modules: HashMap<ModuleId, ModuleRecord>,
    modules_by_name: HashMap<String, ModuleId>,
    module_versions: HashMap<ModuleVersionId, ModuleVersionRecord>,
    active_module_versions: HashMap<ModuleId, ModuleVersionId>,

    objects: HashMap<ObjectId, ObjectRecord>,

    tasks: HashMap<TaskId, TaskRecord>,

    actors: HashMap<ActorId, ActorRecord>,
    mailboxes: HashMap<ActorId, VecDeque<MessageRecord>>,

    capabilities: HashMap<CapabilityId, Arc<dyn Capability>>,
    capabilities_by_subject: HashMap<String, Vec<CapabilityId>>,
    revoked_capabilities: HashMap<CapabilityId, bool>,

    events: HashMap<EventId, RuntimeEvent>,
    event_order: Vec<EventId>,

    transactions: HashMap<TransactionId, TransactionRecord>,
    transaction_order: Vec<TransactionId>,
}

impl InMemoryStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn ensure_module_exists(&self, module: ModuleId) -> MResult<()> {
        if !self.modules.contains_key(&module) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "module",
                    id: module.to_string(),
                },
                None,
            ));
        }

        Ok(())
    }

    fn ensure_module_version_exists(&self, version: ModuleVersionId) -> MResult<()> {
        if !self.module_versions.contains_key(&version) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "module_version",
                    id: version.to_string(),
                },
                None,
            ));
        }

        Ok(())
    }

    fn ensure_actor_exists(&self, actor: ActorId) -> MResult<()> {
        if !self.actors.contains_key(&actor) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "actor",
                    id: actor.to_string(),
                },
                None,
            ));
        }

        Ok(())
    }
}

impl MechStore for InMemoryStore {
    fn put_module(&mut self, module: ModuleRecord) -> MResult<ModuleId> {
        module.validate()?;

        if self.modules.contains_key(&module.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "module",
                    id: module.id.to_string(),
                },
                None,
            ));
        }

        if self.modules_by_name.contains_key(&module.name) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "module.name",
                    id: module.name.clone(),
                },
                None,
            ));
        }

        let id = module.id;
        self.modules_by_name.insert(module.name.clone(), id);
        self.modules.insert(id, module);
        Ok(id)
    }

    fn get_module(&self, id: ModuleId) -> MResult<Option<ModuleRecord>> {
        Ok(self.modules.get(&id).cloned())
    }

    fn find_module_by_name(&self, name: &str) -> MResult<Option<ModuleRecord>> {
        let Some(id) = self.modules_by_name.get(name) else {
            return Ok(None);
        };

        Ok(self.modules.get(id).cloned())
    }

    fn put_module_version(&mut self, version: ModuleVersionRecord) -> MResult<ModuleVersionId> {
        version.validate()?;
        self.ensure_module_exists(version.module)?;

        if self.module_versions.contains_key(&version.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "module_version",
                    id: version.id.to_string(),
                },
                None,
            ));
        }

        let id = version.id;
        self.module_versions.insert(id, version);
        Ok(id)
    }

    fn get_module_version(&self, id: ModuleVersionId) -> MResult<Option<ModuleVersionRecord>> {
        Ok(self.module_versions.get(&id).cloned())
    }

    fn set_active_module_version(
        &mut self,
        module: ModuleId,
        version: ModuleVersionId,
    ) -> MResult<()> {
        self.ensure_module_exists(module)?;
        self.ensure_module_version_exists(version)?;

        let version_record = self.module_versions.get(&version).unwrap();

        if version_record.module != module {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "active_module_version",
                    reason: "module version does not belong to module",
                },
                None,
            ));
        }

        self.active_module_versions.insert(module, version);
        Ok(())
    }

    fn get_active_module_version(&self, module: ModuleId) -> MResult<Option<ModuleVersionId>> {
        Ok(self.active_module_versions.get(&module).copied())
    }

    fn put_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        object.validate()?;

        if self.objects.contains_key(&object.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "object",
                    id: object.id.to_string(),
                },
                None,
            ));
        }

        let id = object.id;
        self.objects.insert(id, object);
        Ok(id)
    }

    fn get_object(&self, id: ObjectId) -> MResult<Option<ObjectRecord>> {
        Ok(self.objects.get(&id).cloned())
    }

    fn update_object(&mut self, object: ObjectRecord) -> MResult<ObjectId> {
        object.validate()?;

        if !self.objects.contains_key(&object.id) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "object",
                    id: object.id.to_string(),
                },
                None,
            ));
        }

        let id = object.id;
        self.objects.insert(id, object);
        Ok(id)
    }

    fn put_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
        task.validate()?;

        if self.tasks.contains_key(&task.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "task",
                    id: task.id.to_string(),
                },
                None,
            ));
        }

        let id = task.id;
        self.tasks.insert(id, task);
        Ok(id)
    }

    fn get_task(&self, id: TaskId) -> MResult<Option<TaskRecord>> {
        Ok(self.tasks.get(&id).cloned())
    }

    fn update_task(&mut self, task: TaskRecord) -> MResult<TaskId> {
        task.validate()?;

        if !self.tasks.contains_key(&task.id) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "task",
                    id: task.id.to_string(),
                },
                None,
            ));
        }

        let id = task.id;
        self.tasks.insert(id, task);
        Ok(id)
    }

    fn put_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        actor.validate()?;

        if self.actors.contains_key(&actor.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "actor",
                    id: actor.id.to_string(),
                },
                None,
            ));
        }

        let id = actor.id;
        self.mailboxes.entry(id).or_default();
        self.actors.insert(id, actor);
        Ok(id)
    }

    fn get_actor(&self, id: ActorId) -> MResult<Option<ActorRecord>> {
        Ok(self.actors.get(&id).cloned())
    }

    fn update_actor(&mut self, actor: ActorRecord) -> MResult<ActorId> {
        actor.validate()?;

        if !self.actors.contains_key(&actor.id) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "actor",
                    id: actor.id.to_string(),
                },
                None,
            ));
        }

        let id = actor.id;
        self.actors.insert(id, actor);
        self.mailboxes.entry(id).or_default();
        Ok(id)
    }

    fn enqueue_message(&mut self, actor: ActorId, message: MessageRecord) -> MResult<MessageId> {
        self.ensure_actor_exists(actor)?;
        message.validate()?;

        if message.actor != actor {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "message.actor",
                    reason: "message actor does not match target actor",
                },
                None,
            ));
        }

        let id = message.id;
        self.mailboxes.entry(actor).or_default().push_back(message);
        Ok(id)
    }

    fn peek_message(&self, actor: ActorId) -> MResult<Option<MessageRecord>> {
        self.ensure_actor_exists(actor)?;

        Ok(self
            .mailboxes
            .get(&actor)
            .and_then(|mailbox| mailbox.front().cloned()))
    }

    fn ack_message(&mut self, actor: ActorId, message: MessageId) -> MResult<()> {
        self.ensure_actor_exists(actor)?;

        let Some(mailbox) = self.mailboxes.get_mut(&actor) else {
            return Ok(());
        };

        if let Some(index) = mailbox.iter().position(|queued| queued.id == message) {
            mailbox.remove(index);
        }

        Ok(())
    }

    fn grant_capability(
        &mut self,
        id: CapabilityId,
        capability: Arc<dyn Capability>,
    ) -> MResult<CapabilityId> {
        capability.validate()?;

        if id.is_zero() {
            return invalid_store_record("capability.id", "must not be zero");
        }

        if capability.id() != id {
            return Err(MechError::new(
                InvalidStoreRecordError {
                    field: "capability.id",
                    reason: "capability id does not match grant id",
                },
                None,
            ));
        }

        if self.capabilities.contains_key(&id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "capability",
                    id: id.to_string(),
                },
                None,
            ));
        }

        self.capabilities_by_subject
            .entry(capability.subject_key().to_string())
            .or_default()
            .push(id);

        self.revoked_capabilities.insert(id, false);
        self.capabilities.insert(id, capability);
        Ok(id)
    }

    fn get_capability(&self, id: CapabilityId) -> MResult<Option<Arc<dyn Capability>>> {
        Ok(self.capabilities.get(&id).cloned())
    }

    fn list_capabilities_for_subject(&self, subject_key: &str) -> MResult<Vec<CapabilityId>> {
        Ok(self
            .capabilities_by_subject
            .get(subject_key)
            .cloned()
            .unwrap_or_default())
    }

    fn revoke_capability(&mut self, id: CapabilityId) -> MResult<()> {
        let Some(capability) = self.capabilities.get(&id) else {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "capability",
                    id: id.to_string(),
                },
                None,
            ));
        };

        if !capability.is_revocable() {
            return Err(MechError::new(
                StoreCapabilityNotRevocableError { capability: id },
                None,
            ));
        }

        self.revoked_capabilities.insert(id, true);
        Ok(())
    }

    fn is_capability_revoked(&self, id: CapabilityId) -> MResult<bool> {
        if !self.capabilities.contains_key(&id) {
            return Err(MechError::new(
                StoreRecordNotFoundError {
                    record_type: "capability",
                    id: id.to_string(),
                },
                None,
            ));
        }

        Ok(self.revoked_capabilities.get(&id).copied().unwrap_or(false))
    }

    fn append_event(&mut self, event: RuntimeEvent) -> MResult<EventId> {
        event.validate()?;

        if self.events.contains_key(&event.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "event",
                    id: event.id.to_string(),
                },
                None,
            ));
        }

        let id = event.id;
        self.events.insert(id, event);
        self.event_order.push(id);
        Ok(id)
    }

    fn get_event(&self, id: EventId) -> MResult<Option<RuntimeEvent>> {
        Ok(self.events.get(&id).cloned())
    }

    fn list_events(&self, limit: Option<usize>) -> MResult<Vec<RuntimeEvent>> {
        let iter = self.event_order.iter().rev();

        let ids: Vec<EventId> = match limit {
            Some(limit) => iter.take(limit).copied().collect(),
            None => iter.copied().collect(),
        };

        Ok(ids
            .into_iter()
            .rev()
            .filter_map(|id| self.events.get(&id).cloned())
            .collect())
    }

    fn commit_transaction(&mut self, tx: TransactionRecord) -> MResult<TransactionId> {
        tx.validate()?;

        if self.transactions.contains_key(&tx.id) {
            return Err(MechError::new(
                StoreRecordAlreadyExistsError {
                    record_type: "transaction",
                    id: tx.id.to_string(),
                },
                None,
            ));
        }

        let id = tx.id;
        self.transactions.insert(id, tx);
        self.transaction_order.push(id);
        Ok(id)
    }

    fn get_transaction(&self, id: TransactionId) -> MResult<Option<TransactionRecord>> {
        Ok(self.transactions.get(&id).cloned())
    }

    fn list_transactions(&self, limit: Option<usize>) -> MResult<Vec<TransactionRecord>> {
        let iter = self.transaction_order.iter().rev();

        let ids: Vec<TransactionId> = match limit {
            Some(limit) => iter.take(limit).copied().collect(),
            None => iter.copied().collect(),
        };

        Ok(ids
            .into_iter()
            .rev()
            .filter_map(|id| self.transactions.get(&id).cloned())
            .collect())
    }
}

// -----------------------------------------------------------------------------
// Store Errors
// -----------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct InvalidStoreRecordError {
    pub field: &'static str,
    pub reason: &'static str,
}

impl MechErrorKind for InvalidStoreRecordError {
    fn name(&self) -> &str {
        "InvalidStoreRecord"
    }

    fn message(&self) -> String {
        format!(
            "Invalid store record field `{}`: {}",
            self.field, self.reason
        )
    }
}

fn invalid_store_record<T>(field: &'static str, reason: &'static str) -> MResult<T> {
    Err(MechError::new(
        InvalidStoreRecordError { field, reason },
        None,
    ))
}

#[derive(Debug, Clone)]
pub struct StoreRecordAlreadyExistsError {
    pub record_type: &'static str,
    pub id: String,
}

impl MechErrorKind for StoreRecordAlreadyExistsError {
    fn name(&self) -> &str {
        "StoreRecordAlreadyExists"
    }

    fn message(&self) -> String {
        format!("{} record already exists: {}", self.record_type, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct StoreRecordNotFoundError {
    pub record_type: &'static str,
    pub id: String,
}

impl MechErrorKind for StoreRecordNotFoundError {
    fn name(&self) -> &str {
        "StoreRecordNotFound"
    }

    fn message(&self) -> String {
        format!("{} record not found: {}", self.record_type, self.id)
    }
}

#[derive(Debug, Clone)]
pub struct StoreCapabilityNotRevocableError {
    pub capability: CapabilityId,
}

impl MechErrorKind for StoreCapabilityNotRevocableError {
    fn name(&self) -> &str {
        "StoreCapabilityNotRevocable"
    }

    fn message(&self) -> String {
        format!("Capability is not revocable: {}", self.capability)
    }
}

// -----------------------------------------------------------------------------
// Tests
// -----------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    use crate::capability::{BasicCapability, BasicOperation, BasicResource, BasicSubject};

    use crate::event::{RuntimeEvent, RuntimeEventKind};

    #[test]
    fn module_round_trip() {
        let mut store = InMemoryStore::new();

        let module = ModuleRecord::new(ModuleId(1), "main");
        store.put_module(module).unwrap();

        let loaded = store.get_module(ModuleId(1)).unwrap().unwrap();
        assert_eq!(loaded.name, "main");

        let found = store.find_module_by_name("main").unwrap().unwrap();
        assert_eq!(found.id, ModuleId(1));
    }

    #[test]
    fn module_version_requires_existing_module() {
        let mut store = InMemoryStore::new();

        let version = ModuleVersionRecord::new(ModuleVersionId(1), ModuleId(999), 1);

        assert!(store.put_module_version(version).is_err());
    }

    #[test]
    fn active_module_version_round_trip() {
        let mut store = InMemoryStore::new();

        store
            .put_module(ModuleRecord::new(ModuleId(1), "main"))
            .unwrap();

        store
            .put_module_version(ModuleVersionRecord::new(
                ModuleVersionId(10),
                ModuleId(1),
                1,
            ))
            .unwrap();

        store
            .set_active_module_version(ModuleId(1), ModuleVersionId(10))
            .unwrap();

        assert_eq!(
            store.get_active_module_version(ModuleId(1)).unwrap(),
            Some(ModuleVersionId(10)),
        );
    }

    #[test]
    fn object_round_trip() {
        let mut store = InMemoryStore::new();

        let object = ObjectRecord::text(ObjectId(1), "note", "hello");
        store.put_object(object).unwrap();

        let loaded = store.get_object(ObjectId(1)).unwrap().unwrap();
        assert_eq!(loaded.kind, "note");
        assert_eq!(loaded.encoding, "text/plain");
        assert_eq!(loaded.data, b"hello");
    }

    #[test]
    fn task_round_trip() {
        let mut store = InMemoryStore::new();

        let task = TaskRecord::new(TaskId(1), "task:1").with_status(TaskStatus::running());

        store.put_task(task).unwrap();

        let loaded = store.get_task(TaskId(1)).unwrap().unwrap();
        assert_eq!(loaded.status.name, "running");
    }

    #[test]
    fn actor_message_queue() {
        let mut store = InMemoryStore::new();

        store
            .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
            .unwrap();

        store
            .enqueue_message(
                ActorId(1),
                MessageRecord::new(MessageId(1), ActorId(1), "ping", b"hello".to_vec()),
            )
            .unwrap();

        let peeked = store.peek_message(ActorId(1)).unwrap().unwrap();
        assert_eq!(peeked.kind, "ping");

        let popped = store.pop_message(ActorId(1)).unwrap().unwrap();
        assert_eq!(popped.payload, b"hello");

        assert!(store.pop_message(ActorId(1)).unwrap().is_none());
    }

    #[test]
    fn actor_message_ack_removes_specific_message() {
        let mut store = InMemoryStore::new();

        store
            .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
            .unwrap();

        let first = MessageRecord::new(MessageId(1), ActorId(1), "first", b"one".to_vec());

        let second = MessageRecord::new(MessageId(2), ActorId(1), "second", b"two".to_vec());

        store.enqueue_message(ActorId(1), first).unwrap();
        store.enqueue_message(ActorId(1), second).unwrap();

        assert_eq!(
            store.peek_message(ActorId(1)).unwrap().unwrap().id,
            MessageId(1),
        );

        store.ack_message(ActorId(1), MessageId(1)).unwrap();

        assert_eq!(
            store.peek_message(ActorId(1)).unwrap().unwrap().id,
            MessageId(2),
        );

        store.ack_message(ActorId(1), MessageId(2)).unwrap();

        assert!(store.peek_message(ActorId(1)).unwrap().is_none());
    }

    #[test]
    fn capability_round_trip_and_revoke() {
        let mut store = InMemoryStore::new();

        let subject = BasicSubject::new("task:1");
        let resource = BasicResource::new("db:users");

        let capability = BasicCapability::new(
            CapabilityId(1),
            &subject,
            &resource,
            [BasicOperation::read()],
        );

        store
            .grant_capability(CapabilityId(1), Arc::new(capability))
            .unwrap();

        let ids = store.list_capabilities_for_subject("task:1").unwrap();

        assert_eq!(ids, vec![CapabilityId(1)]);

        assert!(!store.is_capability_revoked(CapabilityId(1)).unwrap());

        store.revoke_capability(CapabilityId(1)).unwrap();

        assert!(store.is_capability_revoked(CapabilityId(1)).unwrap());
    }

    #[test]
    fn event_order_is_preserved_with_limit() {
        let mut store = InMemoryStore::new();

        store
            .append_event(RuntimeEvent::new(
                EventId(1),
                0,
                RuntimeEventKind::RuntimeError {
                    message: "first".to_string(),
                },
            ))
            .unwrap();

        store
            .append_event(RuntimeEvent::new(
                EventId(2),
                1,
                RuntimeEventKind::RuntimeError {
                    message: "second".to_string(),
                },
            ))
            .unwrap();

        store
            .append_event(RuntimeEvent::new(
                EventId(3),
                2,
                RuntimeEventKind::RuntimeError {
                    message: "third".to_string(),
                },
            ))
            .unwrap();

        let events = store.list_events(Some(2)).unwrap();

        assert_eq!(events.len(), 2);
        assert_eq!(events[0].id, EventId(2));
        assert_eq!(events[1].id, EventId(3));
        assert_eq!(events[0].sequence, 1);
        assert_eq!(events[1].sequence, 2);
    }

    #[test]
    fn transaction_round_trip() {
        let mut store = InMemoryStore::new();

        let tx = TransactionRecord::new(TransactionId(1), "task:1")
            .with_read_set(vec![ObjectId(1)])
            .with_write_set(vec![ObjectId(2)])
            .with_message_acks(vec![MessageId(3)])
            .with_message_sends(vec![MessageId(4)])
            .with_task_updates(vec![TaskId(5)])
            .with_actor_updates(vec![ActorId(6)])
            .with_events(vec![EventId(7)]);

        store.commit_transaction(tx).unwrap();

        let loaded = store.get_transaction(TransactionId(1)).unwrap().unwrap();

        assert_eq!(loaded.subject, "task:1");
        assert_eq!(loaded.read_set, vec![ObjectId(1)]);
        assert_eq!(loaded.write_set, vec![ObjectId(2)]);
        assert_eq!(loaded.message_acks, vec![MessageId(3)]);
        assert_eq!(loaded.message_sends, vec![MessageId(4)]);
        assert_eq!(loaded.task_updates, vec![TaskId(5)]);
        assert_eq!(loaded.actor_updates, vec![ActorId(6)]);
        assert_eq!(loaded.events, vec![EventId(7)]);
    }
}
