use std::sync::Arc;

use mech_core::{GenericError, MResult, MechError, Ref, Value};
use mech_runtime::{
    ActorId, ActorRecord, BasicCapability, BasicOperation, BasicResource, BasicSubject,
    CapabilityId, EventId, InMemoryStore, MechRuntime, MechStore, MessageId, MessageRecord,
    ModuleRecord, ModuleVersionId, ModuleVersionRecord, ObjectId, ObjectRecord,
    ResourcePathCapability, RuntimeConfig, RuntimeContext, RuntimeEvent, RuntimeEventKind,
    RuntimeHostInput, RuntimeHostInputDriver, RuntimeHostInputSource, RuntimeHostInputValue,
    RuntimeIngress, RuntimeResourceProvider, RuntimeResourceReadRequest, RuntimeStoreCommit,
    SequentialIdGenerator, TaskId, TaskRecord, TransactionId, TransactionRecord, module_id,
};

use super::source_runtime_builder;

const INPUT_BASE_URI: &str = "test://clock/ticks";
const INPUT_PATH: &str = "value";

#[derive(Debug)]
struct HistoryInputProvider;

#[derive(Debug, Default)]
struct HistoryInputDriver;

impl RuntimeHostInputDriver for HistoryInputDriver {
    fn drives(&self, source: &RuntimeHostInputSource) -> bool {
        source.base_uri() == INPUT_BASE_URI && source.path() == INPUT_PATH
    }

    fn attach(&mut self, _ingress: RuntimeIngress) -> MResult<()> {
        Ok(())
    }

    fn start(&mut self) -> MResult<()> {
        Ok(())
    }

    fn stop(&mut self) -> MResult<()> {
        Ok(())
    }

    fn is_live(&self) -> bool {
        true
    }
}

impl RuntimeResourceProvider for HistoryInputProvider {
    fn scheme(&self) -> &str {
        "test"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![INPUT_BASE_URI.to_string()]
    }

    fn read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri == INPUT_BASE_URI && request.path == INPUT_PATH {
            return Ok(Value::F64(Ref::new(1.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: "missing Gate A benchmark input".to_string(),
            },
            None,
        ))
    }

    fn plan_read(&self, request: RuntimeResourceReadRequest) -> MResult<Value> {
        if request.base_uri == INPUT_BASE_URI && request.path == INPUT_PATH {
            return Ok(Value::F64(Ref::new(0.0)));
        }
        Err(MechError::new(
            GenericError {
                msg: "missing Gate A benchmark planning input".to_string(),
            },
            None,
        ))
    }
}

pub struct HistoryTurnFixture {
    pub runtime: MechRuntime,
    pub context: RuntimeContext,
    input_source: RuntimeHostInputSource,
}

impl HistoryTurnFixture {
    pub fn new() -> Self {
        Self::with_store_context_history_and_retention(InMemoryStore::new(), 0, None)
    }

    pub fn with_accepted_turns(count: usize) -> Self {
        Self::with_store_context_history_and_retention(
            accepted_turn_store(count),
            count.saturating_mul(2),
            None,
        )
    }

    pub fn with_event_retention(limit: usize) -> Self {
        Self::with_store_context_history_and_retention(InMemoryStore::new(), 0, Some(limit as u64))
    }

    fn with_store_context_history_and_retention(
        store: InMemoryStore,
        context_events: usize,
        event_retention: Option<u64>,
    ) -> Self {
        let mut config = RuntimeConfig::default();
        if let Some(event_retention) = event_retention {
            config.limits.max_in_memory_events = Some(event_retention);
        }
        let mut runtime = source_runtime_builder()
            .config(config)
            .id_generator(SequentialIdGenerator::starting_at(1))
            .store(store)
            .resource_provider(Box::new(HistoryInputProvider))
            .input_driver(HistoryInputDriver)
            .build()
            .unwrap();
        let subject = runtime.runtime_context().unwrap().subject().to_string();
        let capability_id = runtime.next_capability_id();
        runtime
            .grant_capability(Arc::new(
                ResourcePathCapability::exact(
                    capability_id,
                    subject,
                    INPUT_BASE_URI,
                    ["read"],
                    INPUT_PATH,
                )
                .unwrap(),
            ))
            .unwrap();
        let mut context = runtime.runtime_context().unwrap();
        runtime
            .run_string_with_context(
                &mut context,
                &format!(
                    "@pulse := {INPUT_BASE_URI}{{:read({INPUT_PATH})}}\n\
                     output := @pulse/{INPUT_PATH}",
                ),
            )
            .unwrap();
        context.reset_for_next_turn().unwrap();
        runtime
            .gate_a_seed_context_event_history(&mut context, context_events)
            .unwrap();
        Self {
            runtime,
            context,
            input_source: RuntimeHostInputSource::new(INPUT_BASE_URI, INPUT_PATH).unwrap(),
        }
    }

    pub fn accept_turn(&mut self) {
        self.runtime
            .apply_host_input_with_context(
                &mut self.context,
                RuntimeHostInput::single(
                    self.input_source.clone(),
                    RuntimeHostInputValue::F64(2.0),
                ),
            )
            .unwrap();
    }

    pub fn populate_context_events(&mut self, count: usize) {
        self.runtime
            .gate_a_seed_context_event_history(&mut self.context, count)
            .unwrap();
    }

    pub fn warm_context_event_retention(&mut self, operations: usize) {
        for _ in 0..operations {
            self.runtime
                .gate_a_emit_representative_event(&mut self.context)
                .unwrap();
        }
    }

    pub fn context_event_lengths(&self) -> (usize, usize) {
        self.runtime
            .gate_a_context_event_lengths(&self.context)
            .unwrap()
    }

    pub fn begin_and_stage_objects(&mut self, count: usize) {
        self.runtime.begin_transaction(&mut self.context).unwrap();
        for index in 0..count {
            let id = ObjectId(10_000 + index as u128);
            self.runtime
                .put_object_with_context(
                    &mut self.context,
                    ObjectRecord::text(id, "gate-a", "staged"),
                )
                .unwrap();
        }
    }
}

fn accepted_turn_store(turn_count: usize) -> InMemoryStore {
    let mut store = InMemoryStore::new();
    for index in 0..turn_count {
        let raw = 100_000_000u128 + index as u128 * 3;
        let first = EventId(raw);
        let second = EventId(raw + 1);
        store
            .append_event(RuntimeEvent::new(
                first,
                (index * 2) as u64,
                RuntimeEventKind::RuntimeTickStarted,
            ))
            .unwrap();
        store
            .append_event(RuntimeEvent::new(
                second,
                (index * 2 + 1) as u64,
                RuntimeEventKind::RuntimeTickCompleted { work_count: 1 },
            ))
            .unwrap();
        store
            .commit_transaction(
                TransactionRecord::new(TransactionId(raw + 2), "gate-a-accepted")
                    .with_events(vec![first, second]),
            )
            .unwrap();
    }
    store
}

pub fn retained_store(record_count: usize) -> InMemoryStore {
    let mut store = InMemoryStore::new();
    for index in 0..record_count {
        let sequence = index as u64;
        let raw = index as u128 + 1;
        store
            .append_event(RuntimeEvent::new(
                EventId(raw),
                sequence,
                RuntimeEventKind::RuntimeTickStarted,
            ))
            .unwrap();
        store
            .commit_transaction(TransactionRecord::new(TransactionId(raw), "gate-a-history"))
            .unwrap();
    }
    store
}

pub fn minimal_commit(seed: u128) -> RuntimeStoreCommit {
    RuntimeStoreCommit {
        transaction: TransactionRecord::new(TransactionId(seed), "gate-a-minimal"),
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
        events: vec![
            RuntimeEvent::new(
                EventId(seed + 1),
                seed as u64,
                RuntimeEventKind::RuntimeTickStarted,
            ),
            RuntimeEvent::new(
                EventId(seed + 2),
                seed as u64 + 1,
                RuntimeEventKind::RuntimeTickCompleted { work_count: 0 },
            ),
        ],
    }
}

pub fn mixed_store_and_commit(seed: u128) -> (InMemoryStore, RuntimeStoreCommit) {
    let mut store = InMemoryStore::new();
    let base_module_name = "bench://gate-a/base.mec";
    let base_module_id = module_id(base_module_name);
    let module = ModuleRecord::new(base_module_id, base_module_name);
    store.put_module(module).unwrap();
    store
        .put_module_version(ModuleVersionRecord::new(
            ModuleVersionId(seed + 1),
            base_module_id,
            1,
        ))
        .unwrap();
    store
        .set_active_module_version(base_module_id, ModuleVersionId(seed + 1))
        .unwrap();
    store
        .put_object(ObjectRecord::text(ObjectId(seed), "state", "before"))
        .unwrap();
    store
        .put_task(TaskRecord::new(TaskId(seed), "task:gate-a"))
        .unwrap();
    store
        .put_actor(ActorRecord::new(ActorId(seed), "actor:gate-a"))
        .unwrap();
    store
        .enqueue_message(
            ActorId(seed),
            MessageRecord::new(
                MessageId(seed + 99),
                ActorId(seed),
                "gate-a-acknowledged",
                Vec::new(),
            ),
        )
        .unwrap();
    let old_capability = CapabilityId(seed);
    store
        .grant_capability(
            old_capability,
            Arc::new(BasicCapability::new(
                old_capability,
                &BasicSubject::new("gate-a"),
                &BasicResource::new("gate-a:old"),
                [BasicOperation::new("read")],
            )),
        )
        .unwrap();

    let next_module_name = "bench://gate-a/next.mec";
    let next_module_id = module_id(next_module_name);
    let mut version = ModuleVersionRecord::new(ModuleVersionId(seed + 11), next_module_id, 1);
    version.dependencies.push(ModuleVersionId(seed + 1));
    let new_capability = CapabilityId(seed + 10);
    let commit = RuntimeStoreCommit {
        transaction: TransactionRecord::new(TransactionId(seed + 20), "gate-a-mixed"),
        module_puts: vec![ModuleRecord::new(next_module_id, next_module_name)],
        module_version_puts: vec![version],
        capability_grants: vec![(
            new_capability,
            Arc::new(BasicCapability::new(
                new_capability,
                &BasicSubject::new("gate-a"),
                &BasicResource::new("gate-a:new"),
                [BasicOperation::new("write")],
            )),
        )],
        capability_revocations: vec![old_capability],
        object_puts: vec![ObjectRecord::text(ObjectId(seed + 10), "state", "new")],
        object_updates: vec![ObjectRecord::text(ObjectId(seed), "state", "after")],
        task_updates: vec![TaskRecord::new(TaskId(seed), "task:gate-a")],
        actor_updates: vec![ActorRecord::new(ActorId(seed), "actor:gate-a")],
        message_acks: vec![(ActorId(seed), MessageId(seed + 99))],
        message_enqueues: vec![(
            ActorId(seed),
            MessageRecord::new(MessageId(seed + 30), ActorId(seed), "gate-a", vec![1, 2, 3]),
        )],
        events: vec![
            RuntimeEvent::new(
                EventId(seed + 40),
                seed as u64,
                RuntimeEventKind::RuntimeTickStarted,
            ),
            RuntimeEvent::new(
                EventId(seed + 41),
                seed as u64 + 1,
                RuntimeEventKind::RuntimeTickCompleted { work_count: 0 },
            ),
        ],
    };
    (store, commit)
}
