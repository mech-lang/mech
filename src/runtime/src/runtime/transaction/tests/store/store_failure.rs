use crate::{
    InMemoryStore, MechRuntime, ObjectId, ObjectRecord, RuntimeEventKind, TransactionId,
};
use super::{event_count, new_runtime};

#[test]
fn transaction_commit_failure_is_atomic() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "hello"),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(200), "note", "missing"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());

    assert!(runtime.get_object(ObjectId(100)).unwrap().is_none());
    assert!(runtime.get_object(ObjectId(200)).unwrap().is_none());
    assert!(runtime.get_transaction(TransactionId(1)).unwrap().is_none());

    let events = runtime.list_events(None).unwrap();
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectCreated {
                object_id: ObjectId(100),
            },),
        0,
    );
    assert_eq!(
        event_count(&events, |kind| kind
            == &RuntimeEventKind::ObjectUpdated {
                object_id: ObjectId(200),
            },),
        0,
    );
}

#[test]
fn transaction_commit_failure_keeps_transaction_active() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();

    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(100), "note", "hello"),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(200), "note", "missing"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
        .abort_runtime_transaction(&mut context, "failed commit")
        .unwrap();
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn store_read_panic_is_converted_and_runtime_recovers() {
    let mut store = InMemoryStore::new();
    store.panic_on_get_object_for_test();
    let mut runtime = MechRuntime::builder().store(store).build().unwrap();

    let error = runtime.get_object(ObjectId(1)).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate store read panic"));
    assert!(!runtime.is_poisoned());
    runtime.run_string("store-read-recovery := 1.0").unwrap();
}
