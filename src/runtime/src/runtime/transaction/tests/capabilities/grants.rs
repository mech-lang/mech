use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use crate::runtime::test_support::ids::ScriptedEventIdGenerator;
use crate::{
    BasicCapabilityKernel, CapabilityGrant, CapabilityId, CapabilityKernel, CapabilityRequest,
    EventId, InMemoryStore, MechRuntime, MechStore, RuntimeAuthorityScope,
    RuntimeCapabilityGrantRollbackFailed, RuntimeCapabilityGrantSpec, RuntimeCapabilityOperation,
    RuntimeConfig, RuntimeConfigSpec, RuntimeEvent, RuntimeEventKind, SequentialIdGenerator,
};

use super::super::RuntimeCapabilityOverlay;
use super::{FailingRollbackKernel, capability, request};

#[test]
fn capability_grant_store_failure_never_grants_kernel_authority() {
    let mut store = InMemoryStore::new();
    store
        .grant_capability(
            CapabilityId(100),
            capability(CapabilityId(100), "task:1", true),
        )
        .unwrap();

    let mut runtime = MechRuntime::builder().store(store).build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let durable_events_before = runtime.list_events(None).unwrap();

    let error = runtime
        .grant_capability_with_context(&mut context, capability(CapabilityId(100), "task:1", true))
        .unwrap_err();

    assert_eq!(error.kind_name(), "StoreRecordAlreadyExists");
    assert!(
        runtime
            .capability_kernel()
            .get(CapabilityId(100))
            .unwrap()
            .is_none()
    );
    assert_eq!(context.authority, RuntimeAuthorityScope::AllForSubject,);
    assert!(!context.events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::CapabilityGranted {
                capability_id: CapabilityId(100),
            }
        )
    }));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_some());
    assert_eq!(
        runtime
            .store()
            .list_capabilities_for_subject("task:1")
            .unwrap(),
        vec![CapabilityId(100)],
    );
    assert_eq!(runtime.list_events(None).unwrap(), durable_events_before);
}

#[test]
fn capability_grant_kernel_failure_rolls_back_store() {
    let mut kernel = BasicCapabilityKernel::new();
    kernel
        .grant(CapabilityGrant::new(capability(
            CapabilityId(100),
            "task:1",
            true,
        )))
        .unwrap();

    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let context_events_before = context.events.clone();
    let context_authority_before = context.authority.clone();

    let error = runtime
        .grant_capability_with_context(&mut context, capability(CapabilityId(100), "task:1", true))
        .unwrap_err();

    assert_eq!(error.kind_name(), "CapabilityAlreadyExists");
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(
        runtime
            .capability_kernel()
            .get(CapabilityId(100))
            .unwrap()
            .is_some()
    );
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.authority, context_authority_before);
}

#[test]
fn capability_grant_event_failure_removes_non_revocable_live_authority() {
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(1);
    let mut runtime = MechRuntime::builder()
        .config(config)
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [EventId(100), EventId(100)],
        ))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.events.push(RuntimeEvent::new(
        EventId(999),
        999,
        RuntimeEventKind::RuntimeTickStarted,
    ));
    let context_events_before = context.events.clone();

    let error = runtime
        .grant_capability_with_context(&mut context, capability(CapabilityId(100), "task:1", false))
        .unwrap_err();

    assert_eq!(error.kind_name(), "StoreRecordAlreadyExists");
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.authority, RuntimeAuthorityScope::AllForSubject,);
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(
        runtime
            .capability_kernel()
            .get(CapabilityId(100))
            .unwrap()
            .is_none()
    );
    assert!(runtime.check_capability(&request("task:1")).is_err());
    assert!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::RuntimeCreated { .. }) })
    );
    assert!(
        !runtime
            .list_events(None)
            .unwrap()
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::CapabilityGranted { .. }) })
    );
}

#[test]
fn capability_grant_failed_transactional_event_staging_is_compensated() {
    let mut runtime = MechRuntime::builder()
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [EventId(100), EventId(101), EventId(0)],
        ))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let context_events_before = context.events.clone();

    let error = runtime
        .grant_capability_with_context(&mut context, capability(CapabilityId(100), "task:1", true))
        .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidRuntimeEvent");
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(
        !runtime
            .active_transactions
            .get(&transaction_id)
            .unwrap()
            .store
            .staged_events()
            .any(|event| matches!(event.kind, RuntimeEventKind::CapabilityGranted { .. }))
    );
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert!(
        runtime
            .capability_kernel()
            .get(CapabilityId(100))
            .unwrap()
            .is_none()
    );
    assert_eq!(context.authority, RuntimeAuthorityScope::AllForSubject,);

    runtime
        .abort_runtime_transaction(&mut context, "test cleanup")
        .unwrap();
}

#[test]
fn capability_grant_incomplete_rollback_is_reported_and_continues() {
    let rollback_attempted = Arc::new(AtomicBool::new(false));
    let kernel = FailingRollbackKernel {
        inner: BasicCapabilityKernel::new(),
        rollback_attempted: rollback_attempted.clone(),
    };
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(1);
    let mut runtime = MechRuntime::builder()
        .config(config)
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [EventId(100), EventId(100)],
        ))
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.events.push(RuntimeEvent::new(
        EventId(999),
        999,
        RuntimeEventKind::RuntimeTickStarted,
    ));
    let context_events_before = context.events.clone();

    let error = runtime
        .grant_capability_with_context(&mut context, capability(CapabilityId(100), "task:1", false))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeCapabilityGrantRollbackFailed");
    assert_eq!(
        error.source.as_ref().unwrap().kind_name(),
        "StoreRecordAlreadyExists",
    );
    let rollback = error
        .kind_as::<RuntimeCapabilityGrantRollbackFailed>()
        .unwrap();
    assert!(
        rollback
            .rollback_failures
            .iter()
            .any(|failure| failure.contains("capability kernel: GenericError"))
    );
    assert!(rollback_attempted.load(Ordering::SeqCst));
    assert!(runtime.get_capability(CapabilityId(100)).unwrap().is_none());
    assert_eq!(context.events, context_events_before);
    assert_eq!(context.authority, RuntimeAuthorityScope::AllForSubject,);
    assert!(
        runtime
            .capability_kernel()
            .get(CapabilityId(100))
            .unwrap()
            .is_some()
    );
}

#[test]
fn capability_grant_success_updates_every_component_once() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    assert_eq!(
        runtime
            .grant_capability_with_context(&mut context, capability(id, "task:1", true))
            .unwrap(),
        id,
    );
    assert!(runtime.get_capability(id).unwrap().is_some());
    assert!(runtime.capability_kernel().get(id).unwrap().is_some());
    assert_eq!(runtime.check_capability(&request("task:1")).unwrap(), id);
    assert!(context.authority.contains(id));
    assert_eq!(
        context
            .events
            .iter()
            .filter(|event| {
                matches!(
                  event.kind,
                  RuntimeEventKind::CapabilityGranted { capability_id } if capability_id == id
                )
            })
            .count(),
        1,
    );
    assert_eq!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                  event.kind,
                  RuntimeEventKind::CapabilityGranted { capability_id } if capability_id == id
                )
            })
            .count(),
        1,
    );
}

#[test]
fn configuration_grants_use_the_runtime_store_and_kernel() {
    let subject = "task://configured";
    let mut runtime = MechRuntime::builder()
        .config_spec(
            RuntimeConfigSpec::new().with_capability_grant(
                RuntimeCapabilityGrantSpec::new(subject, "docs://manual")
                    .with_operation(RuntimeCapabilityOperation::Read)
                    .with_path("intro/*"),
            ),
        )
        .build()
        .unwrap();
    let capability = runtime
        .list_events(None)
        .unwrap()
        .into_iter()
        .find_map(|event| match event.kind {
            RuntimeEventKind::CapabilityGranted { capability_id } => Some(capability_id),
            _ => None,
        })
        .expect("configuration grant event");

    assert!(runtime.store.get_capability(capability).unwrap().is_some());
    assert!(runtime.capability_kernel.get(capability).unwrap().is_some(),);
    assert_eq!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                subject,
                "read",
                "docs://manual/intro/title",
            ))
            .unwrap(),
        capability,
    );
}

#[test]
fn provisional_capability_grant_is_visible_only_to_its_transaction() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut owner = runtime.runtime_context().unwrap();
    let mut observer = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut owner).unwrap();
    runtime
        .grant_capability_with_context(&mut owner, capability(id, "task:1", true))
        .unwrap();

    assert_eq!(
        runtime
            .check_capability_with_context(&mut owner, &request("task:1"))
            .unwrap(),
        id,
    );
    assert!(
        runtime
            .check_capability_with_context(&mut observer, &request("task:1"))
            .is_err()
    );
    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());

    runtime
        .abort_runtime_transaction(&mut owner, "test abort")
        .unwrap();
    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
}

#[test]
fn provisional_capability_selection_follows_stable_grant_order() {
    let first = CapabilityId(100);
    let second = CapabilityId(101);
    let mut overlay = RuntimeCapabilityOverlay::default();
    overlay
        .stage_grant(capability(first, "task:1", true))
        .unwrap();
    overlay
        .stage_grant(capability(second, "task:1", true))
        .unwrap();

    for _ in 0..32 {
        assert_eq!(
            overlay
                .preview_check(&request("task:1"), &RuntimeAuthorityScope::AllForSubject,)
                .unwrap(),
            Some(first),
        );
    }
    assert_eq!(
        overlay
            .check(&request("task:1"), &RuntimeAuthorityScope::AllForSubject,)
            .unwrap(),
        Some(first),
    );
    assert_eq!(
        overlay.grants().map(|(id, _)| id).collect::<Vec<_>>(),
        vec![first, second],
    );
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(first, 1)],);
}
