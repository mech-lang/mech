use crate::RuntimeEventKind;
use crate::runtime::test_support::providers::test_runtime_builder;

#[test]
fn program_transaction_implicit_success_commits_program_store_and_events() {
    let mut runtime = test_runtime_builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .run_string_with_context(&mut context, "implicit-committed-symbol := 7")
        .unwrap();

    assert!(
        runtime
            .program
            .root_symbol_value("implicit-committed-symbol")
            .is_ok(),
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.list_transactions(None).unwrap().len(), 1);
    let events = runtime.list_events(None).unwrap();
    assert!(
        events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. }) })
    );
    assert!(
        events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::TransactionCommitted { .. }) })
    );
}
