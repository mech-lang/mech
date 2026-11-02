use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{InMemoryStore, ObjectId, ObjectRecord, RuntimeConfig};

#[test]
fn store_commit_panic_is_indeterminate_and_cleans_transaction_scope() {
    let mut store = InMemoryStore::new();
    store.panic_on_commit_runtime_for_test();
    let mut config = RuntimeConfig::default();
    config.limits.max_in_memory_events = Some(1);
    let mut runtime = test_runtime_builder()
        .config(config)
        .store(store)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(41), "seed", "retained-before-transaction"),
        )
        .unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(42), "state", "committed-before-panic"),
        )
        .unwrap();
    runtime
        .put_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(43), "state", "latest-visible-event"),
        )
        .unwrap();
    assert_eq!(context.events().len(), 1);
    assert!(context.event_storage_physical_len() > context.events().len());

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeStoreCommitIndeterminate");
    assert!(format!("{error:?}").contains("deliberate store commit panic"));
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert_eq!(context.event_storage_physical_len(), context.events().len());
    assert_eq!(context.event_storage_physical_len(), 1);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(
        runtime.get_object(ObjectId(41)).unwrap(),
        Some(ObjectRecord::text(
            ObjectId(41),
            "seed",
            "retained-before-transaction",
        )),
    );
}
