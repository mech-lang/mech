use super::{event_count, new_runtime};
use crate::runtime::test_support::effects::{EffectLifecycleLog, TransactionalEffectProbe};
use crate::runtime::test_support::stores::AppendEventFailureProbe;
use crate::{MechRuntime, ObjectId, ObjectRecord, PreparedRuntimeEffect, RuntimeEventKind};

fn runtime_with_append_failure_probe() -> (MechRuntime, AppendEventFailureProbe) {
    let (store, probe) = AppendEventFailureProbe::new();
    let runtime = MechRuntime::builder().store(store).build().unwrap();
    (runtime, probe)
}

fn has_event(
    events: &[crate::RuntimeEvent],
    predicate: impl Fn(&RuntimeEventKind) -> bool,
) -> bool {
    events.iter().any(|event| predicate(&event.kind))
}

#[test]
fn transaction_abort_discards_staged_events() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "hello"),
        )
        .unwrap();

    let staged_event_id = context
        .events
        .iter()
        .find(|event| {
            event.kind
                == (RuntimeEventKind::ObjectCreated {
                    object_id: ObjectId(100),
                })
        })
        .map(|event| event.id)
        .unwrap();

    runtime
        .abort_runtime_transaction(&mut context, "abort")
        .unwrap();

    assert!(
        !context
            .events
            .iter()
            .any(|event| event.id == staged_event_id)
    );
    assert!(runtime.get_event(staged_event_id).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_transaction(transaction_id).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::TransactionStarted { transaction_id },),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::TransactionAborted {
                transaction_id,
                message: "abort".to_string(),
            },),
        1,
    );
}

#[test]
fn explicit_abort_reports_transaction_aborted_publication_failure() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    probe.fail_next_transaction_aborted();

    let error = runtime
        .abort_runtime_transaction(&mut context, "publication failure")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeOperationRollbackFailed");
    assert!(error.full_chain_message().contains(&format!(
        "transaction-aborted event publication failed for transaction {transaction_id}",
    )));
}

#[test]
fn failed_abort_marker_is_absent_from_context_and_store() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    probe.fail_next_transaction_aborted();

    runtime
        .abort_runtime_transaction(&mut context, "publication failure")
        .unwrap_err();

    let durable = runtime.list_events(None).unwrap();
    assert!(has_event(&durable, |kind| matches!(
        kind,
        RuntimeEventKind::TransactionStarted {
            transaction_id: started,
        } if *started == transaction_id
    )));
    assert!(!has_event(&durable, |kind| matches!(
        kind,
        RuntimeEventKind::TransactionAborted {
            transaction_id: aborted,
            ..
        } if *aborted == transaction_id
    )));
    assert!(!has_event(&context.events, |kind| matches!(
        kind,
        RuntimeEventKind::TransactionAborted {
            transaction_id: aborted,
            ..
        } if *aborted == transaction_id
    )));
}

#[test]
fn failed_abort_marker_poisons_runtime_after_cleanup() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    probe.fail_next_transaction_aborted();

    runtime
        .abort_runtime_transaction(&mut context, "publication failure")
        .unwrap_err();

    assert!(runtime.is_poisoned());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
}

fn stage_abortable_effect(
    runtime: &mut MechRuntime,
    context: &mut crate::RuntimeContext,
) -> crate::RuntimeEffectId {
    runtime
        .stage_runtime_effect_with_context(
            context,
            PreparedRuntimeEffect::Transactional(Box::new(TransactionalEffectProbe::new(
                "abort-publication",
                EffectLifecycleLog::default(),
            ))),
        )
        .unwrap()
}

#[test]
fn effect_aborted_publication_failure_is_reported() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    let effect_id = stage_abortable_effect(&mut runtime, &mut context);
    probe.fail_next_effect_aborted();

    let error = runtime
        .abort_runtime_transaction(&mut context, "effect publication failure")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeOperationRollbackFailed");
    assert!(error.full_chain_message().contains(&format!(
        "effect-aborted event publication failed for effect {effect_id}",
    )));
}

#[test]
fn effect_marker_failure_does_not_skip_transaction_aborted_marker() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    stage_abortable_effect(&mut runtime, &mut context);
    probe.fail_next_effect_aborted();

    runtime
        .abort_runtime_transaction(&mut context, "effect publication failure")
        .unwrap_err();

    let durable = runtime.list_events(None).unwrap();
    assert_eq!(
        event_count(&durable, |kind| kind
            == &RuntimeEventKind::TransactionAborted {
                transaction_id,
                message: "effect publication failure".to_string(),
            }),
        1,
    );
    assert_eq!(
        event_count(&context.events, |kind| kind
            == &RuntimeEventKind::TransactionAborted {
                transaction_id,
                message: "effect publication failure".to_string(),
            }),
        1,
    );
}

#[test]
fn multiple_abort_publication_failures_are_aggregated() {
    let (mut runtime, probe) = runtime_with_append_failure_probe();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let effect_id = stage_abortable_effect(&mut runtime, &mut context);
    probe.fail_next_effect_aborted();
    probe.fail_next_transaction_aborted();

    let error = runtime
        .abort_runtime_transaction(&mut context, "multiple publication failures")
        .unwrap_err();
    let message = error.full_chain_message();

    assert!(message.contains(&format!(
        "effect-aborted event publication failed for effect {effect_id}",
    )));
    assert!(message.contains(&format!(
        "transaction-aborted event publication failed for transaction {transaction_id}",
    )));
    assert!(runtime.is_poisoned());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
}
