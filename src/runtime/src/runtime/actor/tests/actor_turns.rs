use std::sync::Arc;

use crate::actor_behavior::{ActorBehaviorDriver, ActorBehaviorRuntime};
use crate::{
    ActorId, ActorRecord, ActorStateGetHostFunction, ActorTurn, BasicCapability,
    BasicCapabilityKernel, BasicOperation, BasicResource, BasicSubject, CapabilityId, EventId,
    HostCall, HostFunctionPlan, InMemoryHostRegistry, MechRuntime, MessageId, MessageRecord,
    ObjectRecord, RuntimeCallContext, RuntimeContext, RuntimeEventKind,
};
use mech_core::{MResult, Ref, Value};

#[derive(Debug)]
struct PanickingActorBehaviorDriver;

impl ActorBehaviorDriver for PanickingActorBehaviorDriver {
    fn run_actor_turn(
        &mut self,
        _runtime: &mut dyn ActorBehaviorRuntime,
        _context: &mut RuntimeContext,
        _turn: &ActorTurn,
    ) -> MResult<()> {
        panic!("deliberate actor behavior panic");
    }
}

#[test]
fn actor_driver_panic_is_converted_and_driver_is_restored() {
    let mut runtime = MechRuntime::builder()
        .actor_behavior_driver(PanickingActorBehaviorDriver)
        .build()
        .unwrap();
    let actor = ActorRecord::new(ActorId(1), "actor:1");
    let message = MessageRecord::new(MessageId(1), ActorId(1), "tick", Vec::new());
    let turn = ActorTurn::new(actor, message).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let error = runtime
        .run_actor_turn_envelope(&mut context, &turn)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate actor behavior panic"));
    assert_eq!(context.subject, "actor:1");
    assert_eq!(context.actor, Some(ActorId(1)));
    assert_eq!(
        runtime
            .run_actor_turn_envelope(&mut context, &turn)
            .unwrap_err()
            .kind_name(),
        "RuntimeExtensionPanicked",
    );
    assert!(!runtime.is_poisoned());
    runtime.list_events(None).unwrap();
}

#[test]
fn runtime_managed_actor_identity_transitions_commit() {
    let mut host_registry = InMemoryHostRegistry::new();
    crate::register_actor_context_host_functions(&mut host_registry).unwrap();
    let subject = "actor:managed-identity";
    let capability_id = CapabilityId(99);
    let mut runtime = MechRuntime::builder()
        .host_registry(host_registry)
        .capability_kernel(BasicCapabilityKernel::new())
        .build()
        .unwrap();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            capability_id,
            &BasicSubject::new(subject),
            &BasicResource::new("host:actor/state/put"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let initial_state = runtime.next_object_id();
    runtime
        .put_object(ObjectRecord::text(initial_state, "actor-state", "before"))
        .unwrap();
    let actor = runtime
        .create_actor(subject, None, Some(initial_state), vec![capability_id])
        .unwrap();
    runtime.send_message(actor, "update", Vec::new()).unwrap();
    let actor_record = runtime.get_actor(actor).unwrap().unwrap();
    let mut context = runtime.context_for_actor(&actor_record).unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let turn = runtime
        .next_actor_turn_with_context(&mut context, actor)
        .unwrap()
        .unwrap();
    runtime
        .run_actor_turn_envelope(&mut context, &turn)
        .unwrap();
    runtime
        .call_host_with_context(
            &mut context,
            HostCall::new(
                "actor/state/put",
                vec![Value::String(Ref::new("after".to_string()))],
            ),
        )
        .unwrap();
    let updated_state = context.actor_state().unwrap();
    assert_ne!(updated_state, initial_state);

    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(
        runtime.get_actor(actor).unwrap().unwrap().state,
        Some(updated_state),
    );
    assert_eq!(
        runtime.get_object(updated_state).unwrap().unwrap().data,
        b"after",
    );
    assert!(runtime.peek_message(actor).unwrap().is_none());
}

#[test]
fn actor_state_get_plan_and_invoke_keep_string_shape_for_dangling_state() {
    let mut host_registry = InMemoryHostRegistry::new();
    crate::register_actor_context_host_functions(&mut host_registry).unwrap();
    let subject = "actor:dangling-state";
    let capability_id = CapabilityId(100);
    let mut runtime = MechRuntime::builder()
        .host_registry(host_registry)
        .capability_kernel(BasicCapabilityKernel::new())
        .build()
        .unwrap();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            capability_id,
            &BasicSubject::new(subject),
            &BasicResource::new("host:actor/state/get"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let dangling_state = runtime.next_object_id();
    let actor = runtime
        .create_actor(subject, None, Some(dangling_state), vec![capability_id])
        .unwrap();
    let actor_record = runtime.get_actor(actor).unwrap().unwrap();
    let mut context = runtime.context_for_actor(&actor_record).unwrap();
    assert!(runtime.get_object(dangling_state).unwrap().is_none());

    let planned = ActorStateGetHostFunction::new()
        .plan(&RuntimeCallContext::capture(&context), &[])
        .unwrap()
        .into_value();
    match planned {
        Value::String(value) => assert!(value.borrow().is_empty()),
        other => panic!("expected planned empty string for dangling actor state, got {other:?}"),
    }
    assert!(runtime.get_object(dangling_state).unwrap().is_none());

    let invoked = runtime
        .call_host_with_context(&mut context, HostCall::new("actor/state/get", Vec::new()))
        .unwrap()
        .into_value();
    match invoked {
        Value::String(value) => assert!(value.borrow().is_empty()),
        other => panic!("expected invoked empty string for dangling actor state, got {other:?}"),
    }
}

#[test]
fn transactional_actor_turn_subject_mismatch_is_rejected_before_context_mutation() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let actor = ActorRecord::new(ActorId(1), "other");
    let message = MessageRecord::new(MessageId(1), ActorId(1), "ping", Vec::new());
    let turn = ActorTurn::new(actor, message).unwrap();

    let subject_before = context.subject.clone();
    let actor_before = context.actor;
    let actor_message_before = context.actor_message.clone();
    let actor_state_before = context.actor_state;
    let context_event_ids_before: Vec<EventId> =
        context.events.iter().map(|event| event.id).collect();
    let runtime_events_before = runtime.list_events(None).unwrap();
    let staged_event_ids_before = runtime
        .active_transactions
        .get(&transaction_id)
        .unwrap()
        .store
        .staged_event_ids();
    let staged_put_count_before = runtime
        .active_transactions
        .get(&transaction_id)
        .unwrap()
        .store
        .staged_puts()
        .count();

    let error = runtime
        .run_actor_turn_envelope(&mut context, &turn)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    assert_eq!(context.subject, subject_before);
    assert_eq!(context.actor, actor_before);
    assert_eq!(context.actor_message, actor_message_before);
    assert_eq!(context.actor_state, actor_state_before);
    assert_eq!(
        context
            .events
            .iter()
            .map(|event| event.id)
            .collect::<Vec<_>>(),
        context_event_ids_before,
    );
    assert_eq!(runtime.list_events(None).unwrap(), runtime_events_before);
    assert_eq!(
        runtime
            .active_transactions
            .get(&transaction_id)
            .unwrap()
            .store
            .staged_event_ids(),
        staged_event_ids_before,
    );
    assert_eq!(
        runtime
            .active_transactions
            .get(&transaction_id)
            .unwrap()
            .store
            .staged_puts()
            .count(),
        staged_put_count_before,
    );
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
        .abort_runtime_transaction(&mut context, "rollback")
        .unwrap();
}

#[test]
fn transactional_actor_turn_succeeds_when_subject_matches_owner() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let actor = ActorRecord::new(ActorId(1), "owner");
    let message = MessageRecord::new(MessageId(1), ActorId(1), "ping", Vec::new());
    let turn = ActorTurn::new(actor, message).unwrap();
    context.bind_actor_turn(&turn);
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_actor_turn_envelope(&mut context, &turn)
        .unwrap();

    assert_eq!(context.subject, "owner");
    assert_eq!(context.actor, Some(ActorId(1)));
    assert!(context.events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::ActorTurnStarted {
                actor_id: ActorId(1)
            }
        )
    }));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
        .abort_runtime_transaction(&mut context, "rollback")
        .unwrap();
}
