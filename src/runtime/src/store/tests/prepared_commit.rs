use super::super::prepared_commit::PreparedInMemoryCommit;
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

fn capability(id: u128, subject: &str) -> Arc<dyn Capability> {
    Arc::new(BasicCapability::from_keys(
        CapabilityId(id),
        subject,
        "resource:test",
        [":read"],
    ))
}

#[track_caller]
fn assert_failed_commit_is_semantically_atomic(
    mut store: InMemoryStore,
    commit: RuntimeStoreCommit,
) {
    let before = InMemoryStoreSemanticSnapshot::capture(&store);
    assert!(store.commit_runtime(commit).is_err());
    assert_eq!(InMemoryStoreSemanticSnapshot::capture(&store), before);
}

fn seeded_runtime_store() -> InMemoryStore {
    let mut store = InMemoryStore::new();
    store
        .put_object(ObjectRecord::text(ObjectId(10), "note", "before"))
        .unwrap();
    store
        .put_task(TaskRecord::new(TaskId(20), "task:20"))
        .unwrap();
    store
        .put_actor(ActorRecord::new(ActorId(30), "actor:30"))
        .unwrap();
    store
        .enqueue_message(
            ActorId(30),
            MessageRecord::new(MessageId(40), ActorId(30), "first", b"one".to_vec()),
        )
        .unwrap();
    store
}

#[test]
fn prepared_complete_batch_updates_every_supported_family() {
    let mut store = seeded_runtime_store();
    store.configure_event_retention(Some(1)).unwrap();

    let module = ModuleRecord::new(
        module_id("memory://prepared/main.mec"),
        "memory://prepared/main.mec",
    );
    let first_version = ModuleVersionRecord::new(ModuleVersionId(101), module.id, 1);
    let dependent_version = ModuleVersionRecord::new(ModuleVersionId(102), module.id, 2)
        .with_dependencies(vec![first_version.id]);
    let granted = capability(50, "task:20");

    let mut commit = empty_commit(60);
    commit.module_puts = vec![module.clone()];
    commit.module_version_puts = vec![first_version.clone(), dependent_version.clone()];
    commit.object_puts = vec![ObjectRecord::text(ObjectId(11), "note", "created")];
    commit.object_updates = vec![ObjectRecord::text(ObjectId(10), "note", "after")];
    commit.task_updates =
        vec![TaskRecord::new(TaskId(20), "task:20").with_status(TaskStatus::completed())];
    commit.actor_updates = vec![ActorRecord::new(ActorId(30), "actor:30").with_status("running")];
    commit.message_acks = vec![(ActorId(30), MessageId(40)), (ActorId(30), MessageId(999))];
    commit.message_enqueues = vec![
        (
            ActorId(30),
            MessageRecord::new(MessageId(41), ActorId(30), "second", b"two".to_vec()),
        ),
        (
            ActorId(30),
            MessageRecord::new(MessageId(42), ActorId(30), "third", b"three".to_vec()),
        ),
    ];
    commit.capability_grants = vec![(CapabilityId(50), granted)];
    commit.capability_revocations = vec![CapabilityId(50)];
    commit.events = vec![
        RuntimeEvent::new(EventId(70), 0, RuntimeEventKind::RuntimeTickStarted),
        RuntimeEvent::new(EventId(71), 1, RuntimeEventKind::RuntimeTickStarted),
    ];
    commit.transaction = commit
        .transaction
        .with_write_set(vec![ObjectId(10), ObjectId(11)])
        .with_message_acks(vec![MessageId(40)])
        .with_message_sends(vec![MessageId(41), MessageId(42)])
        .with_task_updates(vec![TaskId(20)])
        .with_actor_updates(vec![ActorId(30)])
        .with_events(vec![EventId(70), EventId(71)]);

    assert_eq!(store.commit_runtime(commit).unwrap(), TransactionId(60));
    assert_eq!(store.modules.get(&module.id), Some(&module));
    assert_eq!(
        store.module_versions.get(&first_version.id),
        Some(&first_version)
    );
    assert_eq!(
        store.module_versions.get(&dependent_version.id),
        Some(&dependent_version)
    );
    assert_eq!(store.objects.get(&ObjectId(10)).unwrap().data, b"after");
    assert!(store.objects.contains_key(&ObjectId(11)));
    assert_eq!(
        store.tasks.get(&TaskId(20)).unwrap().status.name,
        "completed"
    );
    assert_eq!(store.actors.get(&ActorId(30)).unwrap().status, "running");
    assert_eq!(
        store
            .mailboxes
            .get(&ActorId(30))
            .unwrap()
            .iter()
            .map(|message| message.id)
            .collect::<Vec<_>>(),
        vec![MessageId(41), MessageId(42)]
    );
    assert_eq!(
        store.capabilities_by_subject.get("task:20"),
        Some(&vec![CapabilityId(50)])
    );
    assert_eq!(
        store.revoked_capabilities.get(&CapabilityId(50)),
        Some(&true)
    );
    assert_eq!(store.event_order, vec![EventId(71)]);
    assert_eq!(store.events.len(), 1);
    assert_eq!(store.transaction_order, vec![TransactionId(60)]);
    assert!(store.transactions.contains_key(&TransactionId(60)));
}

#[test]
fn preparation_failure_matrix_preserves_exact_semantic_state() {
    let canonical = "memory://prepared/failure.mec";
    let canonical_id = module_id(canonical);

    let mut store = InMemoryStore::new();
    store
        .modules
        .insert(canonical_id, ModuleRecord::new(canonical_id, canonical));
    store.modules_by_name.insert(canonical.into(), canonical_id);
    let mut commit = empty_commit(1);
    commit.module_puts.push(ModuleRecord::new(
        canonical_id,
        "memory://prepared/conflict.mec",
    ));
    assert_failed_commit_is_semantically_atomic(store, commit);

    let mut store = InMemoryStore::new();
    store
        .modules_by_name
        .insert(canonical.into(), ModuleId(canonical_id.0 + 1));
    let mut commit = empty_commit(2);
    commit
        .module_puts
        .push(ModuleRecord::new(canonical_id, canonical));
    assert_failed_commit_is_semantically_atomic(store, commit);

    let mut commit = empty_commit(3);
    commit.module_version_puts.push(ModuleVersionRecord::new(
        ModuleVersionId(1),
        ModuleId(999),
        1,
    ));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(4);
    let module = ModuleRecord::new(canonical_id, canonical);
    commit.module_puts.push(module.clone());
    commit.module_version_puts.push(
        ModuleVersionRecord::new(ModuleVersionId(1), module.id, 1)
            .with_dependencies(vec![ModuleVersionId(999)]),
    );
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(5);
    commit.module_puts.push(module.clone());
    let invalid_import = SourceImportDeclaration {
        specifier: "browser/dom".into(),
        alias: Some(SourceImportAlias::Context("ui".into())),
        module: Some("browser".into()),
        item: Some("dom".into()),
        kind: crate::resolver::SourceImportKind::Single { name: "dom".into() },
    };
    commit.module_version_puts.push(
        ModuleVersionRecord::new(ModuleVersionId(1), module.id, 1)
            .with_imports(vec![invalid_import.clone()])
            .with_dependencies(vec![ModuleVersionId(999)])
            .with_import_edges(vec![ModuleImportEdge {
                scope: SourceScope::Program,
                import: invalid_import,
                dependency: ModuleVersionId(999),
            }]),
    );
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(6);
    commit.object_puts = vec![
        ObjectRecord::text(ObjectId(1), "note", "one"),
        ObjectRecord::text(ObjectId(1), "note", "two"),
    ];
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(7);
    commit
        .object_updates
        .push(ObjectRecord::text(ObjectId(1), "note", "missing"));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(8);
    commit
        .task_updates
        .push(TaskRecord::new(TaskId(1), "missing"));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(9);
    commit
        .actor_updates
        .push(ActorRecord::new(ActorId(1), "missing"));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(10);
    commit.message_enqueues.push((
        ActorId(30),
        MessageRecord::new(MessageId(1), ActorId(31), "wrong", Vec::new()),
    ));
    assert_failed_commit_is_semantically_atomic(seeded_runtime_store(), commit);

    let mut commit = empty_commit(11);
    commit.message_enqueues.push((
        ActorId(999),
        MessageRecord::new(MessageId(1), ActorId(999), "missing", Vec::new()),
    ));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(12);
    commit
        .capability_grants
        .push((CapabilityId(1), capability(2, "subject")));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(13);
    commit.capability_grants = vec![
        (CapabilityId(1), capability(1, "subject")),
        (CapabilityId(1), capability(1, "subject")),
    ];
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(14);
    commit.capability_grants.push((
        CapabilityId(1),
        Arc::new(
            BasicCapability::from_keys(CapabilityId(1), "subject", "resource", [":read"])
                .revocable(false),
        ),
    ));
    commit.capability_revocations.push(CapabilityId(1));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(15);
    commit.capability_revocations.push(CapabilityId(999));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(16);
    commit.events = vec![
        RuntimeEvent::new(EventId(1), 0, RuntimeEventKind::RuntimeTickStarted),
        RuntimeEvent::new(EventId(1), 1, RuntimeEventKind::RuntimeTickStarted),
    ];
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(17);
    commit.events.push(RuntimeEvent::new(
        EventId::ZERO,
        0,
        RuntimeEventKind::RuntimeTickStarted,
    ));
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut store = InMemoryStore::new();
    store
        .commit_transaction(TransactionRecord::new(TransactionId(18), "existing"))
        .unwrap();
    assert_failed_commit_is_semantically_atomic(store, empty_commit(18));

    let mut commit = empty_commit(19);
    commit.transaction.id = TransactionId::ZERO;
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);

    let mut commit = empty_commit(20);
    commit.module_puts.push(module);
    commit.module_version_puts = vec![
        ModuleVersionRecord::new(ModuleVersionId(1), canonical_id, 1),
        ModuleVersionRecord::new(ModuleVersionId(1), canonical_id, 2),
    ];
    assert_failed_commit_is_semantically_atomic(InMemoryStore::new(), commit);
}

#[test]
fn prepared_apply_consumes_plan_and_returns_transaction_id_directly() {
    let mut store = InMemoryStore::new();
    let prepared = PreparedInMemoryCommit::prepare(&mut store, empty_commit(77)).unwrap();
    let id: TransactionId = store.apply_prepared_runtime_commit(prepared);

    assert_eq!(id, TransactionId(77));
    assert_eq!(store.transaction_order, vec![TransactionId(77)]);
}

#[test]
fn event_validation_simulates_per_append_retention() {
    let mut store = InMemoryStore::new();
    store.configure_event_retention(Some(1)).unwrap();
    store
        .append_event(RuntimeEvent::new(
            EventId(1),
            0,
            RuntimeEventKind::RuntimeTickStarted,
        ))
        .unwrap();

    let mut commit = empty_commit(78);
    commit.events = vec![
        RuntimeEvent::new(EventId(2), 1, RuntimeEventKind::RuntimeTickStarted),
        RuntimeEvent::new(EventId(1), 2, RuntimeEventKind::RuntimeTickStarted),
    ];

    assert_eq!(store.commit_runtime(commit).unwrap(), TransactionId(78));
    assert_eq!(store.event_order, VecDeque::from([EventId(1)]));
    assert_eq!(store.events.get(&EventId(1)).unwrap().sequence, 2);
    assert!(!store.events.contains_key(&EventId(2)));
}
