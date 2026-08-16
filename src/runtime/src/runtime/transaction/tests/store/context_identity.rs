use super::new_runtime;
use crate::{
    ActorId, ActorRecord, CapabilityRequest, HostCall, MessageId, MessageRecord, ObjectId,
    ObjectRecord, RuntimeContext, TaskId, TransactionId,
};

#[test]
fn rejects_foreign_runtime_context_before_object_write_and_events() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(
        runtime_b
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(900), "note", "foreign"),
            )
            .is_err()
    );

    assert!(runtime_b.get_object(ObjectId(900)).unwrap().is_none());
    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
}

#[test]
fn nonexistent_transaction_context_does_not_fall_through_to_durable_writes() {
    let mut runtime = new_runtime();
    runtime
        .put_actor(ActorRecord::new(ActorId(1), "actor:1"))
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    context.transaction = Some(TransactionId(404));
    let events_before = runtime.list_events(None).unwrap();

    assert!(
        runtime
            .put_object_with_context(
                &mut context,
                ObjectRecord::text(ObjectId(901), "note", "missing-tx"),
            )
            .is_err()
    );
    assert!(
        runtime
            .send_message_with_context(&mut context, ActorId(1), "ping", b"missing-tx".to_vec())
            .is_err()
    );

    assert!(runtime.get_object(ObjectId(901)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
}

#[test]
fn transaction_subject_mismatch_cannot_stage_commit_or_abort_owner_can_finish() {
    let mut runtime = new_runtime();
    runtime
        .put_actor(ActorRecord::new(ActorId(1), "owner"))
        .unwrap();
    let mut owner_context = runtime.runtime_context().unwrap();
    owner_context.subject = "owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut owner_context).unwrap();
    let events_after_begin = runtime.list_events(None).unwrap();

    let mut other_context = runtime.runtime_context().unwrap();
    other_context.subject = "other".to_string();
    other_context.transaction = Some(transaction_id);

    assert!(
        runtime
            .put_object_with_context(
                &mut other_context,
                ObjectRecord::text(ObjectId(902), "note", "wrong-owner"),
            )
            .is_err()
    );
    assert!(
        runtime
            .send_message_with_context(
                &mut other_context,
                ActorId(1),
                "ping",
                b"wrong-owner".to_vec()
            )
            .is_err()
    );
    assert!(
        runtime
            .commit_runtime_transaction(&mut other_context)
            .is_err()
    );
    assert!(
        runtime
            .abort_runtime_transaction(&mut other_context, "wrong-owner")
            .is_err()
    );

    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_object(ObjectId(902)).unwrap().is_none());
    assert!(runtime.pop_message(ActorId(1)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_begin);
    assert!(other_context.events.is_empty());

    assert_eq!(
        runtime
            .commit_runtime_transaction(&mut owner_context)
            .unwrap(),
        transaction_id
    );
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn stale_aborted_transaction_context_is_rejected_not_durable() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let mut stale_context = context.clone();
    runtime
        .abort_runtime_transaction(&mut context, "rollback")
        .unwrap();
    let events_after_abort = runtime.list_events(None).unwrap();

    assert!(
        runtime
            .put_object_with_context(
                &mut stale_context,
                ObjectRecord::text(ObjectId(903), "note", "stale"),
            )
            .is_err()
    );

    assert_eq!(stale_context.transaction, Some(transaction_id));
    assert!(runtime.get_object(ObjectId(903)).unwrap().is_none());
    assert_eq!(runtime.list_events(None).unwrap(), events_after_abort);
}

#[test]
fn active_transaction_context_clone_is_rejected_before_transaction_consumption() {
    let mut runtime = new_runtime();
    let mut owner_context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut owner_context).unwrap();
    let mut cloned_context = owner_context.clone();

    let error = runtime
        .abort_runtime_transaction(&mut cloned_context, "cloned context")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeTransactionContextMismatch");
    assert!(
        error
            .full_chain_message()
            .contains("event storage does not match")
    );
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(cloned_context.transaction, Some(transaction_id));
    assert!(!runtime.is_poisoned());

    runtime
        .abort_runtime_transaction(&mut owner_context, "owner cleanup")
        .unwrap();
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn transaction_context_identity_includes_task_actor_message_and_state() {
    fn assert_mismatch(mutate: impl FnOnce(&mut RuntimeContext)) {
        let mut runtime = new_runtime();
        let mut owner = runtime.runtime_context().unwrap();
        let transaction_id = runtime.begin_transaction(&mut owner).unwrap();
        let mut mismatched = owner.clone();
        mutate(&mut mismatched);

        let error = runtime
            .put_object_with_context(
                &mut mismatched,
                ObjectRecord::text(ObjectId(906), "note", "identity mismatch"),
            )
            .unwrap_err();

        assert_eq!(error.kind_name(), "RuntimeTransactionContextMismatch");
        assert!(runtime.get_object(ObjectId(906)).unwrap().is_none());
        assert!(runtime.active_transactions.contains_key(&transaction_id));
        runtime
            .abort_runtime_transaction(&mut owner, "identity mismatch cleanup")
            .unwrap();
    }

    assert_mismatch(|context| context.task = Some(TaskId(1)));
    assert_mismatch(|context| context.actor = Some(ActorId(2)));
    assert_mismatch(|context| {
        context.actor_message = Some(MessageRecord::new(
            MessageId(3),
            ActorId(2),
            "identity",
            Vec::new(),
        ));
    });
    assert_mismatch(|context| context.actor_state = Some(ObjectId(4)));
}

#[test]
fn foreign_context_rejected_before_host_and_capability_boundaries() {
    let runtime_a = new_runtime();
    let mut runtime_b = new_runtime();
    let mut context = runtime_a.runtime_context().unwrap();
    let events_before = runtime_b.list_events(None).unwrap();

    assert!(
        runtime_b
            .call_host_with_context(&mut context, HostCall::new("missing/host", Vec::new()))
            .is_err()
    );
    assert!(
        runtime_b
            .check_capability_with_context(
                &mut context,
                &CapabilityRequest::from_keys("subject", "op", "resource"),
            )
            .is_err()
    );

    assert_eq!(runtime_b.list_events(None).unwrap(), events_before);
    assert!(context.events.is_empty());
}

#[test]
fn historical_transaction_record_context_is_valid_without_active_transaction() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    context.subject = "historical-owner".to_string();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    let mut record_context = runtime.context_for_transaction(&record).unwrap();

    assert_eq!(record_context.runtime, runtime.id);
    assert_eq!(record_context.subject, record.subject);
    assert_eq!(record_context.transaction, None);
    runtime
        .put_object_with_context(
            &mut record_context,
            ObjectRecord::text(ObjectId(905), "note", "historical"),
        )
        .unwrap();
    assert!(runtime.get_object(ObjectId(905)).unwrap().is_some());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_transaction(transaction_id).unwrap().is_some());
}
