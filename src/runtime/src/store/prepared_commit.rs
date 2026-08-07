use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use mech_core::{MResult, MechError};

use super::{
    ActorId, Capability, CapabilityId, InMemoryStore, MessageId, MessageRecord, ModuleRecord,
    ModuleVersionRecord, ObjectRecord, RuntimeEvent, RuntimeStoreCommit,
    StoreCapacityReservationError, TaskRecord, TransactionRecord, validation,
};

pub(super) enum PreparedInsert<T> {
    Insert(T),
    ExistingIdentical,
}

pub(super) struct PreparedModulePut {
    pub(super) insert: PreparedInsert<ModuleRecord>,
    pub(super) index_name: Option<String>,
}

pub(super) struct PreparedModuleVersionPut {
    pub(super) insert: PreparedInsert<ModuleVersionRecord>,
}

pub(super) struct PreparedMessageAck {
    pub(super) actor: ActorId,
    pub(super) message: MessageId,
    pub(super) present: bool,
}

pub(super) struct PreparedMessageEnqueue {
    pub(super) actor: ActorId,
    pub(super) message: MessageRecord,
}

pub(super) struct PreparedCapabilityGrant {
    pub(super) id: CapabilityId,
    pub(super) subject: String,
    pub(super) capability: Arc<dyn Capability>,
}

pub(super) struct PreparedCapabilitySubjectUpdate {
    pub(super) subject: String,
    pub(super) ids: Vec<CapabilityId>,
    pub(super) existing: bool,
}

pub(super) struct PreparedMailboxCreate {
    pub(super) actor: ActorId,
    pub(super) mailbox: VecDeque<MessageRecord>,
}

pub(super) struct PreparedInMemoryCommit {
    pub(super) transaction: TransactionRecord,
    pub(super) module_puts: Vec<PreparedModulePut>,
    pub(super) module_version_puts: Vec<PreparedModuleVersionPut>,
    pub(super) object_puts: Vec<ObjectRecord>,
    pub(super) object_updates: Vec<ObjectRecord>,
    pub(super) task_updates: Vec<TaskRecord>,
    pub(super) actor_updates: Vec<super::ActorRecord>,
    pub(super) mailbox_creates: Vec<PreparedMailboxCreate>,
    pub(super) message_acks: Vec<PreparedMessageAck>,
    pub(super) message_enqueues: Vec<PreparedMessageEnqueue>,
    pub(super) capability_grants: Vec<PreparedCapabilityGrant>,
    pub(super) capability_subject_updates: Vec<PreparedCapabilitySubjectUpdate>,
    pub(super) capability_revocations: Vec<CapabilityId>,
    pub(super) events: Vec<RuntimeEvent>,
}

impl PreparedInMemoryCommit {
    pub(super) fn prepare(store: &mut InMemoryStore, commit: RuntimeStoreCommit) -> MResult<Self> {
        let validated = validation::validate_runtime_commit(store, &commit)?;
        let mailbox_creates = reserve_store_capacity(store, &commit, &validated)?;
        let message_acks = prepare_message_acks(store, &commit.message_acks)?;
        let capability_subject_updates = prepare_capability_subject_updates(
            store,
            &commit.capability_grants,
            &validated.capability_subjects,
        )?;

        let module_puts = prepare_module_puts(commit.module_puts, validated.module_inserts)?;
        let module_version_puts = commit
            .module_version_puts
            .into_iter()
            .zip(validated.module_version_inserts)
            .map(|(version, insert)| PreparedModuleVersionPut {
                insert: if insert {
                    PreparedInsert::Insert(version)
                } else {
                    PreparedInsert::ExistingIdentical
                },
            })
            .collect();
        let capability_grants = commit
            .capability_grants
            .into_iter()
            .zip(validated.capability_subjects)
            .map(|((id, capability), subject)| PreparedCapabilityGrant {
                id,
                subject,
                capability,
            })
            .collect();
        let message_enqueues = commit
            .message_enqueues
            .into_iter()
            .map(|(actor, message)| PreparedMessageEnqueue { actor, message })
            .collect();

        Ok(Self {
            transaction: commit.transaction,
            module_puts,
            module_version_puts,
            object_puts: commit.object_puts,
            object_updates: commit.object_updates,
            task_updates: commit.task_updates,
            actor_updates: commit.actor_updates,
            mailbox_creates,
            message_acks,
            message_enqueues,
            capability_grants,
            capability_subject_updates,
            capability_revocations: commit.capability_revocations,
            events: commit.events,
        })
    }
}

impl InMemoryStore {
    pub(super) fn apply_prepared_runtime_commit(
        &mut self,
        prepared: PreparedInMemoryCommit,
    ) -> super::TransactionId {
        let PreparedInMemoryCommit {
            transaction,
            module_puts,
            module_version_puts,
            object_puts,
            object_updates,
            task_updates,
            actor_updates,
            mailbox_creates,
            message_acks,
            message_enqueues,
            capability_grants,
            capability_subject_updates,
            capability_revocations,
            events,
        } = prepared;

        for prepared_module in module_puts {
            let PreparedModulePut { insert, index_name } = prepared_module;
            if let PreparedInsert::Insert(module) = insert {
                let index_name = index_name.expect("prepared module insert must own its index key");
                self.modules_by_name.insert(index_name, module.id);
                self.modules.insert(module.id, module);
            }
        }
        for version in module_version_puts {
            if let PreparedInsert::Insert(version) = version.insert {
                self.module_versions.insert(version.id, version);
            }
        }
        for object in object_puts {
            self.objects.insert(object.id, object);
        }
        for object in object_updates {
            self.objects.insert(object.id, object);
        }
        for task in task_updates {
            self.tasks.insert(task.id, task);
        }
        for actor in actor_updates {
            self.actors.insert(actor.id, actor);
        }
        for mailbox in mailbox_creates {
            let replaced = self.mailboxes.insert(mailbox.actor, mailbox.mailbox);
            debug_assert!(replaced.is_none(), "prepared mailbox must be absent");
        }
        for ack in message_acks {
            if ack.present {
                let mailbox = self
                    .mailboxes
                    .get_mut(&ack.actor)
                    .expect("prepared acknowledgement actor must have a mailbox");
                let index = mailbox
                    .iter()
                    .position(|message| message.id == ack.message)
                    .expect("prepared acknowledgement message must remain present");
                mailbox
                    .remove(index)
                    .expect("prepared acknowledgement index must remain valid");
            }
        }
        for enqueue in message_enqueues {
            self.mailboxes
                .get_mut(&enqueue.actor)
                .expect("prepared enqueue actor must have a mailbox")
                .push_back(enqueue.message);
        }
        for grant in capability_grants {
            debug_assert!(capability_subject_updates.iter().any(|update| {
                update.subject == grant.subject && update.ids.contains(&grant.id)
            }));
            self.revoked_capabilities.insert(grant.id, false);
            self.capabilities.insert(grant.id, grant.capability);
        }
        for update in capability_subject_updates {
            if update.existing {
                self.capabilities_by_subject
                    .get_mut(&update.subject)
                    .expect("prepared capability subject index must exist")
                    .extend(update.ids);
            } else {
                let replaced = self
                    .capabilities_by_subject
                    .insert(update.subject, update.ids);
                debug_assert!(
                    replaced.is_none(),
                    "prepared capability subject must be new"
                );
            }
        }
        for capability in capability_revocations {
            let previous = self.revoked_capabilities.insert(capability, true);
            debug_assert!(previous.is_some(), "prepared capability must exist");
        }
        for event in events {
            self.event_order.push(event.id);
            self.events.insert(event.id, event);
            self.prune_events();
        }

        let id = transaction.id;
        self.transactions.insert(id, transaction);
        self.transaction_order.push(id);
        #[cfg(any(test, feature = "runtime_bench_probes"))]
        crate::runtime::gate_a_probe::record_transaction_committed();
        id
    }
}

fn prepare_module_puts(
    modules: Vec<ModuleRecord>,
    inserts: Vec<bool>,
) -> MResult<Vec<PreparedModulePut>> {
    let mut prepared = Vec::new();
    prepared
        .try_reserve(modules.len())
        .map_err(|_| reservation_error("prepared_module_puts"))?;

    for (module, insert) in modules.into_iter().zip(inserts) {
        let (insert, index_name) = if insert {
            let mut index_name = String::new();
            index_name
                .try_reserve_exact(module.name.len())
                .map_err(|_| reservation_error("prepared_module_name"))?;
            index_name.push_str(&module.name);
            (PreparedInsert::Insert(module), Some(index_name))
        } else {
            (PreparedInsert::ExistingIdentical, None)
        };
        prepared.push(PreparedModulePut { insert, index_name });
    }

    Ok(prepared)
}

fn reserve_store_capacity(
    store: &mut InMemoryStore,
    commit: &RuntimeStoreCommit,
    validated: &validation::RuntimeCommitValidation,
) -> MResult<Vec<PreparedMailboxCreate>> {
    let module_inserts = validated
        .module_inserts
        .iter()
        .filter(|insert| **insert)
        .count();
    reserve_hash_map(&mut store.modules, module_inserts, "modules")?;
    reserve_hash_map(
        &mut store.modules_by_name,
        module_inserts,
        "modules_by_name",
    )?;
    reserve_hash_map(
        &mut store.module_versions,
        validated
            .module_version_inserts
            .iter()
            .filter(|insert| **insert)
            .count(),
        "module_versions",
    )?;
    reserve_hash_map(
        &mut store.active_module_versions,
        0,
        "active_module_versions",
    )?;
    reserve_hash_map(&mut store.objects, commit.object_puts.len(), "objects")?;
    reserve_hash_map(&mut store.tasks, 0, "tasks")?;
    reserve_hash_map(&mut store.actors, 0, "actors")?;

    let mut enqueue_counts = HashMap::<ActorId, usize>::new();
    enqueue_counts
        .try_reserve(commit.message_enqueues.len())
        .map_err(|_| reservation_error("mailbox_enqueue_counts"))?;
    for (actor, _) in &commit.message_enqueues {
        *enqueue_counts.entry(*actor).or_default() += 1;
    }
    let mut mailbox_create_ids = HashSet::<ActorId>::new();
    mailbox_create_ids
        .try_reserve(commit.actor_updates.len() + enqueue_counts.len())
        .map_err(|_| reservation_error("mailbox_create_ids"))?;
    for actor in &commit.actor_updates {
        if !store.mailboxes.contains_key(&actor.id) {
            mailbox_create_ids.insert(actor.id);
        }
    }
    for actor in enqueue_counts.keys() {
        if !store.mailboxes.contains_key(actor) {
            mailbox_create_ids.insert(*actor);
        }
    }
    reserve_hash_map(&mut store.mailboxes, mailbox_create_ids.len(), "mailboxes")?;
    for (actor, count) in &enqueue_counts {
        if let Some(mailbox) = store.mailboxes.get_mut(actor) {
            mailbox
                .try_reserve(*count)
                .map_err(|_| reservation_error("mailbox"))?;
        }
    }
    let mut mailbox_creates = Vec::new();
    mailbox_creates
        .try_reserve(mailbox_create_ids.len())
        .map_err(|_| reservation_error("prepared_mailboxes"))?;
    for actor in mailbox_create_ids {
        let mut mailbox = VecDeque::new();
        mailbox
            .try_reserve(enqueue_counts.get(&actor).copied().unwrap_or(0))
            .map_err(|_| reservation_error("prepared_mailbox"))?;
        mailbox_creates.push(PreparedMailboxCreate { actor, mailbox });
    }

    reserve_hash_map(
        &mut store.capabilities,
        commit.capability_grants.len(),
        "capabilities",
    )?;
    let unique_new_subjects = validated
        .capability_subjects
        .iter()
        .filter(|subject| !store.capabilities_by_subject.contains_key(subject.as_str()))
        .collect::<HashSet<_>>()
        .len();
    reserve_hash_map(
        &mut store.capabilities_by_subject,
        unique_new_subjects,
        "capabilities_by_subject",
    )?;
    let mut existing_subject_counts = HashMap::<&str, usize>::new();
    existing_subject_counts
        .try_reserve(validated.capability_subjects.len())
        .map_err(|_| reservation_error("capability_subject_counts"))?;
    for subject in &validated.capability_subjects {
        if store.capabilities_by_subject.contains_key(subject) {
            *existing_subject_counts.entry(subject.as_str()).or_default() += 1;
        }
    }
    for (subject, count) in existing_subject_counts {
        store
            .capabilities_by_subject
            .get_mut(subject)
            .expect("validated subject index must remain present")
            .try_reserve(count)
            .map_err(|_| reservation_error("capability_subject_ids"))?;
    }
    reserve_hash_map(
        &mut store.revoked_capabilities,
        commit.capability_grants.len(),
        "revoked_capabilities",
    )?;
    reserve_hash_map(&mut store.events, commit.events.len(), "events")?;
    store
        .event_order
        .try_reserve(commit.events.len())
        .map_err(|_| reservation_error("event_order"))?;
    reserve_hash_map(&mut store.transactions, 1, "transactions")?;
    store
        .transaction_order
        .try_reserve(1)
        .map_err(|_| reservation_error("transaction_order"))?;

    Ok(mailbox_creates)
}

fn prepare_message_acks(
    store: &InMemoryStore,
    acks: &[(ActorId, MessageId)],
) -> MResult<Vec<PreparedMessageAck>> {
    let mut mailbox_ids = HashMap::<ActorId, Vec<MessageId>>::new();
    mailbox_ids
        .try_reserve(acks.len())
        .map_err(|_| reservation_error("prepared_message_ack_mailboxes"))?;
    let mut prepared = Vec::new();
    prepared
        .try_reserve(acks.len())
        .map_err(|_| reservation_error("prepared_message_acks"))?;

    for (actor, message) in acks {
        if !mailbox_ids.contains_key(actor) {
            let source = store.mailboxes.get(actor);
            let mut ids = Vec::new();
            ids.try_reserve(source.map(VecDeque::len).unwrap_or(0))
                .map_err(|_| reservation_error("prepared_message_ack_ids"))?;
            if let Some(mailbox) = source {
                ids.extend(mailbox.iter().map(|queued| queued.id));
            }
            mailbox_ids.insert(*actor, ids);
        }
        let ids = mailbox_ids
            .get_mut(actor)
            .expect("prepared acknowledgement mailbox must exist");
        let present = ids
            .iter()
            .position(|candidate| candidate == message)
            .map(|index| {
                ids.remove(index);
            })
            .is_some();
        prepared.push(PreparedMessageAck {
            actor: *actor,
            message: *message,
            present,
        });
    }

    Ok(prepared)
}

fn prepare_capability_subject_updates(
    store: &InMemoryStore,
    grants: &[(CapabilityId, Arc<dyn Capability>)],
    subjects: &[String],
) -> MResult<Vec<PreparedCapabilitySubjectUpdate>> {
    let mut update_indexes = HashMap::<&str, usize>::new();
    update_indexes
        .try_reserve(subjects.len())
        .map_err(|_| reservation_error("prepared_capability_subject_indexes"))?;
    let mut updates = Vec::<PreparedCapabilitySubjectUpdate>::new();
    updates
        .try_reserve(subjects.len())
        .map_err(|_| reservation_error("prepared_capability_subject_updates"))?;

    for ((id, _), subject) in grants.iter().zip(subjects) {
        let index = if let Some(index) = update_indexes.get(subject.as_str()).copied() {
            index
        } else {
            let index = updates.len();
            updates.push(PreparedCapabilitySubjectUpdate {
                subject: subject.clone(),
                ids: Vec::new(),
                existing: store.capabilities_by_subject.contains_key(subject),
            });
            update_indexes.insert(subject.as_str(), index);
            index
        };
        updates[index].ids.push(*id);
    }

    Ok(updates)
}

fn reserve_hash_map<K, V>(
    map: &mut HashMap<K, V>,
    additional: usize,
    structure: &'static str,
) -> MResult<()>
where
    K: Eq + std::hash::Hash,
{
    map.try_reserve(additional)
        .map_err(|_| reservation_error(structure))
}

fn reservation_error(structure: &'static str) -> MechError {
    MechError::new(StoreCapacityReservationError { structure }, None)
}
