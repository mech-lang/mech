use std::cell::RefCell;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

use mech_core::{
    LegacyValue, MResult, MechMap, Ref, ValRef, ValueSnapshotBorrowConflict,
    ValueSnapshotCollectionCollision, hash_str,
};

use crate::runtime::test_support::capabilities::{
    CapabilityUseProbe, grant_host_call, grant_resource,
};
use crate::runtime::test_support::events::events_since;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::runtime::test_support::stores::StoreCommitProbe;
use crate::{
    CapabilityId, DeterministicHostFunction, HostCall, HostCallPolicy, InMemorySourceResolver,
    ModuleBuildOptions, PreparedRuntimeEffect, RegisteredHostFunction, RuntimeAfterCommitEffect,
    RuntimeBuilder, RuntimeCallContext, RuntimeCapabilityOperation, RuntimeEffectMetadata,
    RuntimeEffectSource, RuntimeEventKind, RuntimeResourceProvider, RuntimeResourceReadRequest,
    RuntimeResourceWriteIntent, RuntimeResourceWritePreflightRequest, RuntimeResourceWriteRequest,
    RuntimeValueSnapshot, SharedCapabilityKernel,
};

const SNAPSHOT_RESOURCE: &str = "snapshot://boundary";
const SNAPSHOT_PATH: &str = "value";

thread_local! {
    static HOST_RESULT_CYCLE: RefCell<Option<ValRef>> =
        const { RefCell::new(None) };
    static MODULE_PLAN_CYCLE: RefCell<Option<ValRef>> =
        const { RefCell::new(None) };
    static SOURCE_PLAN_CYCLE: RefCell<Option<ValRef>> =
        const { RefCell::new(None) };
}

fn cyclic_node() -> ValRef {
    let node = Ref::new(LegacyValue::Empty);
    *node.borrow_mut() = LegacyValue::MutableReference(node.clone());
    node
}

fn thread_local_cycle(
    slot: &'static std::thread::LocalKey<RefCell<Option<ValRef>>>,
) -> LegacyValue {
    let node = cyclic_node();
    slot.with(|stored| {
        *stored.borrow_mut() = Some(node.clone());
    });
    LegacyValue::MutableReference(node)
}

fn clear_thread_local_cycle(slot: &'static std::thread::LocalKey<RefCell<Option<ValRef>>>) {
    slot.with(|stored| {
        if let Some(node) = stored.borrow_mut().take() {
            *node.borrow_mut() = LegacyValue::Empty;
        }
    });
}

fn assert_cycle_error(error: &mech_core::MechError) {
    assert_eq!(
        error.kind_name(),
        "ValueSnapshotCycleUnsupported",
        "{error:?}",
    );
    let rendered = format!("{error:?}");
    assert!(rendered.contains("mutable-reference"), "{rendered}");
    assert!(!rendered.contains("@0x"), "{rendered}");
    assert!(!rendered.contains("0x"), "{rendered}");
}

fn assert_borrow_conflict(error: &mech_core::MechError, phase: &str, node: &str) {
    assert_eq!(
        error.kind_name(),
        "ValueSnapshotBorrowConflict",
        "{error:?}",
    );
    let conflict = error
        .kind_as::<ValueSnapshotBorrowConflict>()
        .expect("borrow conflict kind");
    assert_eq!(conflict.phase, phase);
    assert_eq!(conflict.node, node);
    let rendered = format!("{error:?}");
    assert!(!rendered.contains("@0x"), "{rendered}");
    assert!(!rendered.contains("0x"), "{rendered}");
}

fn assert_collection_collision(error: &mech_core::MechError, collection: &str) {
    assert_eq!(
        error.kind_name(),
        "ValueSnapshotCollectionCollision",
        "{error:?}",
    );
    let collision = error
        .kind_as::<ValueSnapshotCollectionCollision>()
        .expect("collection collision kind");
    assert_eq!(collision.collection, collection);
    assert_eq!(collision.first_index, 0);
    assert_eq!(collision.second_index, 1);
}

fn event_count(
    events: &[crate::RuntimeEvent],
    predicate: impl Fn(&RuntimeEventKind) -> bool,
) -> usize {
    events.iter().filter(|event| predicate(&event.kind)).count()
}

#[derive(Debug)]
struct NoopAfterCommit;

impl RuntimeAfterCommitEffect for NoopAfterCommit {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::ResourceProvider {
                scheme: "snapshot".to_string(),
            },
            "write",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        Ok(())
    }
}

#[derive(Debug)]
struct CountingHostPolicy {
    calls: Arc<AtomicUsize>,
}

impl HostCallPolicy for CountingHostPolicy {
    fn validate_call(
        &self,
        _context: &RuntimeCallContext,
        _function: &RegisteredHostFunction,
        _arguments: &[RuntimeValueSnapshot],
    ) -> MResult<()> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
}

#[derive(Debug)]
struct SnapshotBoundaryProvider {
    cycle_reads: Arc<AtomicBool>,
    cycle: ValRef,
    preflight_calls: Arc<AtomicUsize>,
    prepare_calls: Arc<AtomicUsize>,
}

impl RuntimeResourceProvider for SnapshotBoundaryProvider {
    fn scheme(&self) -> &str {
        "snapshot"
    }

    fn base_uris(&self) -> Vec<String> {
        vec![SNAPSHOT_RESOURCE.to_string()]
    }

    fn read(&self, _request: RuntimeResourceReadRequest) -> MResult<LegacyValue> {
        if self.cycle_reads.swap(false, Ordering::SeqCst) {
            Ok(LegacyValue::MutableReference(self.cycle.clone()))
        } else {
            Ok(LegacyValue::F64(Ref::new(41.0)))
        }
    }

    fn preflight_write(&self, _request: RuntimeResourceWritePreflightRequest) -> MResult<()> {
        self.preflight_calls.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }

    fn prepare_write(
        &self,
        _request: RuntimeResourceWriteRequest,
    ) -> MResult<PreparedRuntimeEffect> {
        self.prepare_calls.fetch_add(1, Ordering::SeqCst);
        Ok(PreparedRuntimeEffect::AfterCommit(Box::new(
            NoopAfterCommit,
        )))
    }
}

fn resource_read_request() -> RuntimeResourceReadRequest {
    RuntimeResourceReadRequest {
        base_uri: SNAPSHOT_RESOURCE.to_string(),
        path: SNAPSHOT_PATH.to_string(),
        context_name: "snapshot".to_string(),
    }
}

#[test]
fn cyclic_resource_write_is_rejected_before_effect_staging() {
    let cycle = cyclic_node();
    let prepare_calls = Arc::new(AtomicUsize::new(0));
    let provider = SnapshotBoundaryProvider {
        cycle_reads: Arc::new(AtomicBool::new(false)),
        cycle: cycle.clone(),
        preflight_calls: Arc::new(AtomicUsize::new(0)),
        prepare_calls: prepare_calls.clone(),
    };
    let (store, store_commits) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .resource_provider(Box::new(provider))
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    let capability = grant_resource(
        &mut runtime,
        &subject,
        SNAPSHOT_RESOURCE,
        RuntimeCapabilityOperation::Write,
        &[SNAPSHOT_PATH],
    );
    let capability_uses = CapabilityUseProbe::new(observed_kernel, capability);
    let event_start = runtime.list_events(None).unwrap().len();
    let store_commits_before = store_commits.calls();
    let capability_uses_before = capability_uses.committed_uses();

    let error = runtime
        .write_resource(RuntimeResourceWriteRequest {
            base_uri: SNAPSHOT_RESOURCE.to_string(),
            path: SNAPSHOT_PATH.to_string(),
            context_name: "snapshot".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::MutableReference(cycle.clone()),
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap_err();
    *cycle.borrow_mut() = LegacyValue::Empty;

    assert_cycle_error(&error);
    assert_eq!(prepare_calls.load(Ordering::SeqCst), 0);
    assert_eq!(store_commits.calls(), store_commits_before);
    assert_eq!(capability_uses.committed_uses(), capability_uses_before,);
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::EffectStaged { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionCommitted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionAborted { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn borrowed_resource_write_is_rejected_before_provider_prepare() {
    let cell = Ref::new(41.0);
    let _borrow = cell.borrow_mut();
    let preflight_calls = Arc::new(AtomicUsize::new(0));
    let prepare_calls = Arc::new(AtomicUsize::new(0));
    let provider = SnapshotBoundaryProvider {
        cycle_reads: Arc::new(AtomicBool::new(false)),
        cycle: Ref::new(LegacyValue::Empty),
        preflight_calls: preflight_calls.clone(),
        prepare_calls: prepare_calls.clone(),
    };
    let (store, store_commits) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .resource_provider(Box::new(provider))
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    let capability = grant_resource(
        &mut runtime,
        &subject,
        SNAPSHOT_RESOURCE,
        RuntimeCapabilityOperation::Write,
        &[SNAPSHOT_PATH],
    );
    let capability_uses = CapabilityUseProbe::new(observed_kernel, capability);
    let event_start = runtime.list_events(None).unwrap().len();
    let store_commits_before = store_commits.calls();
    let capability_uses_before = capability_uses.committed_uses();

    let error = runtime
        .write_resource(RuntimeResourceWriteRequest {
            base_uri: SNAPSHOT_RESOURCE.to_string(),
            path: SNAPSHOT_PATH.to_string(),
            context_name: "snapshot".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::F64(cell.clone()),
            intent: RuntimeResourceWriteIntent::Assign,
        })
        .unwrap_err();

    assert_borrow_conflict(&error, "clone", "f64");
    assert_eq!(preflight_calls.load(Ordering::SeqCst), 0);
    assert_eq!(prepare_calls.load(Ordering::SeqCst), 0);
    assert_eq!(store_commits.calls(), store_commits_before);
    assert_eq!(capability_uses.committed_uses(), capability_uses_before,);
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::EffectStaged { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionCommitted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionAborted { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn cyclic_resource_read_is_rejected_before_implicit_commit() {
    let cycle = cyclic_node();
    let provider = SnapshotBoundaryProvider {
        cycle_reads: Arc::new(AtomicBool::new(true)),
        cycle: cycle.clone(),
        preflight_calls: Arc::new(AtomicUsize::new(0)),
        prepare_calls: Arc::new(AtomicUsize::new(0)),
    };
    let (store, store_commits) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .resource_provider(Box::new(provider))
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    let capability = grant_resource(
        &mut runtime,
        &subject,
        SNAPSHOT_RESOURCE,
        RuntimeCapabilityOperation::Read,
        &[SNAPSHOT_PATH],
    );
    let capability_uses = CapabilityUseProbe::new(observed_kernel, capability);
    let event_start = runtime.list_events(None).unwrap().len();
    let store_commits_before = store_commits.calls();
    let capability_uses_before = capability_uses.committed_uses();

    let error = runtime.read_resource(resource_read_request()).unwrap_err();
    *cycle.borrow_mut() = LegacyValue::Empty;

    assert_cycle_error(&error);
    assert_eq!(store_commits.calls(), store_commits_before);
    assert_eq!(capability_uses.committed_uses(), capability_uses_before,);
    let failed_events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&failed_events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionCommitted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&failed_events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionAborted { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());

    let recovered = runtime
        .read_resource(resource_read_request())
        .unwrap()
        .into_value();
    let LegacyValue::F64(recovered) = recovered else {
        panic!("expected recovered scalar resource");
    };
    assert_eq!(*recovered.borrow(), 41.0);
    assert!(!runtime.is_poisoned());
}

#[test]
fn cyclic_host_result_is_rejected_before_host_completion() {
    let function = DeterministicHostFunction::new(
        "snapshot/cyclic-result",
        move |_context, _arguments| -> MResult<LegacyValue> {
            Ok(thread_local_cycle(&HOST_RESULT_CYCLE))
        },
        move |_context, _arguments| -> MResult<LegacyValue> {
            Ok(thread_local_cycle(&HOST_RESULT_CYCLE))
        },
    );
    let (store, store_commits) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .host_function(function)
        .unwrap()
        .build()
        .unwrap();
    let capability = grant_host_call(&mut runtime, CapabilityId(98_001), "snapshot/cyclic-result");
    let capability_uses = CapabilityUseProbe::new(observed_kernel, capability);
    let event_start = runtime.list_events(None).unwrap().len();
    let store_commits_before = store_commits.calls();

    let error = runtime
        .call_host(HostCall::new("snapshot/cyclic-result", Vec::new()))
        .unwrap_err();
    clear_thread_local_cycle(&HOST_RESULT_CYCLE);

    assert_cycle_error(&error);
    assert_eq!(store_commits.calls(), store_commits_before);
    assert_eq!(capability_uses.committed_uses(), 0);
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallStarted { .. },
        )),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallFailed { .. },
        )),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallCompleted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::EffectStaged { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionAborted { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn stale_map_key_collision_is_rejected_before_host_invocation() {
    let left_key = Ref::new(1.0);
    let right_key = Ref::new(2.0);
    let map = Ref::new(MechMap::from_vec(vec![
        (LegacyValue::F64(left_key), LegacyValue::F64(Ref::new(10.0))),
        (
            LegacyValue::F64(right_key.clone()),
            LegacyValue::F64(Ref::new(20.0)),
        ),
    ]));
    *right_key.borrow_mut() = 1.0;

    let policy_calls = Arc::new(AtomicUsize::new(0));
    let plan_calls = Arc::new(AtomicUsize::new(0));
    let invocation_calls = Arc::new(AtomicUsize::new(0));
    let plan_count = plan_calls.clone();
    let invocation_count = invocation_calls.clone();
    let function = DeterministicHostFunction::new(
        "snapshot/stale-map-collision",
        move |_context, _arguments| -> MResult<LegacyValue> {
            plan_count.fetch_add(1, Ordering::SeqCst);
            Ok(LegacyValue::Empty)
        },
        move |_context, _arguments| -> MResult<LegacyValue> {
            invocation_count.fetch_add(1, Ordering::SeqCst);
            Ok(LegacyValue::Empty)
        },
    );
    let (store, store_commits) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .host_policy(CountingHostPolicy {
            calls: policy_calls.clone(),
        })
        .host_function(function)
        .unwrap()
        .build()
        .unwrap();
    let capability = grant_host_call(
        &mut runtime,
        CapabilityId(98_002),
        "snapshot/stale-map-collision",
    );
    let capability_uses = CapabilityUseProbe::new(observed_kernel, capability);
    let event_start = runtime.list_events(None).unwrap().len();
    let store_commits_before = store_commits.calls();

    let error = runtime
        .call_host(HostCall::new(
            "snapshot/stale-map-collision",
            vec![LegacyValue::Map(map.clone())],
        ))
        .unwrap_err();

    assert_collection_collision(&error, "map key");
    assert_eq!(policy_calls.load(Ordering::SeqCst), 0);
    assert_eq!(plan_calls.load(Ordering::SeqCst), 0);
    assert_eq!(invocation_calls.load(Ordering::SeqCst), 0);
    assert_eq!(store_commits.calls(), store_commits_before);
    assert_eq!(capability_uses.committed_uses(), 0);
    assert_eq!(map.borrow().map.len(), 2);
    assert_eq!(map.borrow().num_elements, 2);
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallStarted { .. },
        )),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallFailed { .. },
        )),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::HostCallCompleted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::EffectStaged { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::TransactionAborted { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn cyclic_root_symbol_query_returns_structured_snapshot_error() {
    let mut runtime = test_runtime_builder().build().unwrap();
    runtime.run_string("safe := 41.0").unwrap();
    let health_before = runtime.runtime_health();
    let cycle = cyclic_node();
    let cyclic_id = hash_str("cyclic");
    let symbols = runtime.program.interpreter().symbols();
    symbols.borrow_mut().insert(
        cyclic_id,
        LegacyValue::MutableReference(cycle.clone()),
        false,
    );

    let error = runtime.root_symbol_value("cyclic").unwrap_err();

    assert_cycle_error(&error);
    assert_eq!(runtime.runtime_health(), health_before);
    let safe = runtime.root_symbol_value("safe").unwrap().into_value();
    let LegacyValue::F64(safe) = safe else {
        panic!("expected safe scalar query");
    };
    assert_eq!(*safe.borrow(), 41.0);
    *cycle.borrow_mut() = LegacyValue::Empty;
    assert!(!runtime.is_poisoned());
}

#[test]
fn cyclic_host_plan_rolls_back_source_operation() {
    let mut runtime = test_runtime_builder()
        .host_function(DeterministicHostFunction::new(
            "snapshot/source-cycle",
            move |_context, _arguments| -> MResult<LegacyValue> {
                Ok(thread_local_cycle(&SOURCE_PLAN_CYCLE))
            },
            move |_context, _arguments| -> MResult<LegacyValue> {
                Ok(thread_local_cycle(&SOURCE_PLAN_CYCLE))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    runtime.run_string("baseline := 1.0").unwrap();
    let event_start = runtime.list_events(None).unwrap().len();

    let error = runtime
        .run_string("candidate := snapshot/source-cycle()\ncandidate")
        .unwrap_err();
    clear_thread_local_cycle(&SOURCE_PLAN_CYCLE);

    assert_cycle_error(&error);
    assert!(runtime.root_symbol_value("candidate").is_err());
    assert!(runtime.root_symbol_value("baseline").is_ok());
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::ProgramCompleted { .. },
        )),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::ProgramFailed { .. },
        )),
        1,
    );
    assert!(!runtime.is_poisoned());
}

#[test]
fn cyclic_module_host_plan_rolls_back_module_graph() {
    let mut resolver = InMemorySourceResolver::new();
    resolver
        .insert_string("root.mec", "answer := snapshot/module-cycle()\nanswer\n")
        .unwrap();
    let mut runtime = RuntimeBuilder::new()
        .source_resolver(resolver)
        .host_function(DeterministicHostFunction::new(
            "snapshot/module-cycle",
            move |_context, _arguments| -> MResult<LegacyValue> {
                Ok(thread_local_cycle(&MODULE_PLAN_CYCLE))
            },
            move |_context, _arguments| -> MResult<LegacyValue> {
                Ok(thread_local_cycle(&MODULE_PLAN_CYCLE))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let event_start = runtime.list_events(None).unwrap().len();

    let error = runtime
        .resolve_and_run_root_module(
            "root.mec",
            ModuleBuildOptions::new("test", "v0.3", "native", &[], &[]),
        )
        .unwrap_err();

    assert_cycle_error(&error);
    assert!(
        runtime
            .store
            .find_module_by_name("memory:root.mec")
            .unwrap()
            .is_none(),
    );
    let events = events_since(&runtime, event_start);
    assert_eq!(
        event_count(&events, |kind| matches!(
            kind,
            RuntimeEventKind::ModuleExecutionCompleted { .. }
                | RuntimeEventKind::ProgramCompleted { .. },
        )),
        0,
    );
    clear_thread_local_cycle(&MODULE_PLAN_CYCLE);
    assert!(!runtime.is_poisoned());
}
