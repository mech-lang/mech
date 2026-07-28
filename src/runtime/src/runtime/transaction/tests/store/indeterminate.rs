use crate::{InMemoryStore, MechRuntime};
use mech_core::Value;

#[test]
fn store_commit_panic_is_indeterminate_and_never_rolled_back() {
    let mut store = InMemoryStore::new();
    store.panic_on_commit_runtime_for_test();
    let mut runtime = MechRuntime::builder().store(store).build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .run_string_with_context(&mut context, "store-commit-panic-state := 42.0")
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeStoreCommitIndeterminate");
    assert!(format!("{error:?}").contains("deliberate store commit panic"));
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    let retained = runtime
        .root_symbol_value("store-commit-panic-state")
        .unwrap();
    match retained.as_value() {
        Value::F64(value) => assert_eq!(*value.borrow(), 42.0),
        other => panic!("expected retained f64 value, got {other:?}"),
    }
}
