use super::super::{ObjectId, ObjectRecord, RuntimeEventKind};
use super::{event_count, new_runtime};

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
