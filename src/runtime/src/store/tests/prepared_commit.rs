use super::*;

#[derive(Clone, Debug, PartialEq, Eq)]
struct InMemoryStoreSemanticSnapshot {
    modules: Vec<(ModuleId, ModuleRecord)>,
    modules_by_name: Vec<(String, ModuleId)>,
    module_versions: Vec<(ModuleVersionId, ModuleVersionRecord)>,
    active_module_versions: Vec<(ModuleId, ModuleVersionId)>,
    objects: Vec<(ObjectId, ObjectRecord)>,
    tasks: Vec<(TaskId, TaskRecord)>,
    actors: Vec<(ActorId, ActorRecord)>,
    mailboxes: Vec<(ActorId, Vec<MessageRecord>)>,
    capabilities: Vec<(CapabilityId, String, bool)>,
    capabilities_by_subject: Vec<(String, Vec<CapabilityId>)>,
    revoked_capabilities: Vec<(CapabilityId, bool)>,
    events: Vec<(EventId, RuntimeEvent)>,
    event_order: Vec<EventId>,
    transactions: Vec<(TransactionId, TransactionRecord)>,
    transaction_order: Vec<TransactionId>,
}

impl InMemoryStoreSemanticSnapshot {
    fn capture(store: &InMemoryStore) -> Self {
        let mut modules = store
            .modules
            .iter()
            .map(|(id, record)| (*id, record.clone()))
            .collect::<Vec<_>>();
        modules.sort_by_key(|(id, _)| id.0);

        let mut modules_by_name = store
            .modules_by_name
            .iter()
            .map(|(name, id)| (name.clone(), *id))
            .collect::<Vec<_>>();
        modules_by_name.sort_by(|left, right| left.0.cmp(&right.0));

        let mut module_versions = store
            .module_versions
            .iter()
            .map(|(id, record)| (*id, record.clone()))
            .collect::<Vec<_>>();
        module_versions.sort_by_key(|(id, _)| id.0);

        let mut active_module_versions = store
            .active_module_versions
            .iter()
            .map(|(module, version)| (*module, *version))
            .collect::<Vec<_>>();
        active_module_versions.sort_by_key(|(module, _)| module.0);

        let mut objects = store
            .objects
            .iter()
            .map(|(id, record)| (*id, record.clone()))
            .collect::<Vec<_>>();
        objects.sort_by_key(|(id, _)| id.0);

        let mut tasks = store
            .tasks
            .iter()
            .map(|(id, record)| (*id, record.clone()))
            .collect::<Vec<_>>();
        tasks.sort_by_key(|(id, _)| id.0);

        let mut actors = store
            .actors
            .iter()
            .map(|(id, record)| (*id, record.clone()))
            .collect::<Vec<_>>();
        actors.sort_by_key(|(id, _)| id.0);

        let mut mailboxes = store
            .mailboxes
            .iter()
            .map(|(actor, mailbox)| (*actor, mailbox.iter().cloned().collect()))
            .collect::<Vec<_>>();
        mailboxes.sort_by_key(|(actor, _)| actor.0);

        let mut capabilities = store
            .capabilities
            .iter()
            .map(|(id, capability)| {
                (
                    *id,
                    capability.subject_key().to_string(),
                    store.revoked_capabilities.get(id).copied().unwrap_or(false),
                )
            })
            .collect::<Vec<_>>();
        capabilities.sort_by_key(|(id, _, _)| id.0);

        let mut capabilities_by_subject = store
            .capabilities_by_subject
            .iter()
            .map(|(subject, ids)| (subject.clone(), ids.clone()))
            .collect::<Vec<_>>();
        capabilities_by_subject.sort_by(|left, right| left.0.cmp(&right.0));

        let mut revoked_capabilities = store
            .revoked_capabilities
            .iter()
            .map(|(id, revoked)| (*id, *revoked))
            .collect::<Vec<_>>();
        revoked_capabilities.sort_by_key(|(id, _)| id.0);

        let mut events = store
            .events
            .iter()
            .map(|(id, event)| (*id, event.clone()))
            .collect::<Vec<_>>();
        events.sort_by_key(|(id, _)| id.0);

        let mut transactions = store
            .transactions
            .iter()
            .map(|(id, transaction)| (*id, transaction.clone()))
            .collect::<Vec<_>>();
        transactions.sort_by_key(|(id, _)| id.0);

        Self {
            modules,
            modules_by_name,
            module_versions,
            active_module_versions,
            objects,
            tasks,
            actors,
            mailboxes,
            capabilities,
            capabilities_by_subject,
            revoked_capabilities,
            events,
            event_order: store.event_order.clone(),
            transactions,
            transaction_order: store.transaction_order.clone(),
        }
    }
}

fn empty_commit(id: u128) -> RuntimeStoreCommit {
    RuntimeStoreCommit {
        transaction: TransactionRecord::new(TransactionId(id), "snapshot-test"),
        module_puts: Vec::new(),
        module_version_puts: Vec::new(),
        capability_grants: Vec::new(),
        capability_revocations: Vec::new(),
        object_puts: Vec::new(),
        object_updates: Vec::new(),
        task_updates: Vec::new(),
        actor_updates: Vec::new(),
        message_acks: Vec::new(),
        message_enqueues: Vec::new(),
        events: Vec::new(),
    }
}

#[test]
fn semantic_snapshot_detects_no_change_after_failed_runtime_commit() {
    let mut store = InMemoryStore::new();
    store
        .put_object(ObjectRecord::text(ObjectId(1), "note", "baseline"))
        .unwrap();
    let before = InMemoryStoreSemanticSnapshot::capture(&store);

    let mut commit = empty_commit(1);
    commit
        .object_puts
        .push(ObjectRecord::text(ObjectId(2), "note", "must roll back"));
    commit
        .object_updates
        .push(ObjectRecord::text(ObjectId(3), "note", "missing"));

    assert!(store.commit_runtime(commit).is_err());
    assert_eq!(InMemoryStoreSemanticSnapshot::capture(&store), before);
}
