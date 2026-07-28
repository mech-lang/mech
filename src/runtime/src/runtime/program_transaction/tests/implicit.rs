use super::super::{MechRuntime, RuntimeEventKind};

#[test]
fn program_transaction_implicit_success_commits_program_store_and_events() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .run_string_with_context(&mut context, "round3-implicit-success := 7")
        .unwrap();

    assert!(
        runtime
            .program
            .root_symbol_value("round3-implicit-success")
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
