use super::super::{EventId, ObjectId, ObjectRecord, RuntimeEventKind};
use super::{event_count, new_runtime};

#[test]
fn transaction_commit_persists_staged_events_once() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let started_id = context
        .events
        .iter()
        .find(|event| event.kind == (RuntimeEventKind::TransactionStarted { transaction_id }))
        .map(|event| event.id)
        .unwrap();

    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "hello"),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "updated"),
        )
        .unwrap();

    let staged_event_ids: Vec<EventId> = context
        .events
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::ObjectCreated { .. } | RuntimeEventKind::ObjectUpdated { .. }
            )
        })
        .map(|event| event.id)
        .collect();

    assert_eq!(
        runtime.commit_runtime_transaction(&mut context).unwrap(),
        transaction_id,
    );

    let object = runtime.get_object(ObjectId(100)).unwrap().unwrap();
    assert_eq!(object.data, b"updated");

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectCreated {
                object_id: ObjectId(100),
            },),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectUpdated {
                object_id: ObjectId(100),
            },),
        1,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::TransactionCommitted { transaction_id },),
        1,
    );
    let commit_event_id = context
        .events
        .iter()
        .find(|event| event.kind == (RuntimeEventKind::TransactionCommitted { transaction_id }))
        .map(|event| event.id)
        .unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|event| event.id == commit_event_id)
            .count(),
        1,
    );

    let record = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert!(record.events.contains(&started_id));
    assert!(record.events.contains(&commit_event_id));
    for event_id in &staged_event_ids {
        assert!(record.events.contains(event_id));
        assert_eq!(
            events.iter().filter(|event| event.id == *event_id).count(),
            1,
        );
    }

    let mut unique = record.events.clone();
    unique.sort_by_key(|id| id.as_u128());
    unique.dedup();
    assert_eq!(unique.len(), record.events.len());
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, None);
}
