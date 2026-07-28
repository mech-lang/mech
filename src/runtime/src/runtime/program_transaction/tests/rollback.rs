use super::super::{EventId, MechRuntime, MechSourceCode, RuntimeEventKind, hash_str};
use crate::runtime::test_support::ids::ScriptedEventIdGenerator;

#[test]
fn program_transaction_outer_abort_restores_program_baseline() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let root_interpreter_id = runtime.program.interpreter().id;
    let plan_len_before = runtime.program.interpreter().plan_len();

    runtime
        .with_atomic_program_operation(
            &mut context,
            "program_transaction_test",
            |runtime, _context| {
                runtime.program.run_source(&MechSourceCode::String(
                    "outer-transaction-symbol := 42".to_string(),
                ))
            },
        )
        .unwrap();

    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(
        runtime
            .program
            .root_symbol_value("outer-transaction-symbol")
            .is_ok(),
    );

    runtime
        .abort_runtime_transaction(&mut context, "discard outer transaction program")
        .unwrap();

    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(runtime.program.interpreter().id, root_interpreter_id);
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_before);
    assert!(
        runtime
            .program
            .root_symbol_value("outer-transaction-symbol")
            .is_err()
    );
}

#[test]
fn program_transaction_implicit_partial_failure_restores_everything() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("implicit-rollback-anchor := 1").unwrap();
    let anchor = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("implicit-rollback-anchor"))
        .unwrap()
        .clone();
    let anchor_address = anchor.addr();
    let plan_len_before = runtime.program.interpreter().plan_len();
    let live_before = runtime.live_state_snapshot();
    let transactions_before = runtime.list_transactions(None).unwrap().len();
    let events_before = runtime.list_events(None).unwrap().len();
    let mut context = runtime.runtime_context().unwrap();
    let source = MechSourceCode::Program(vec![
        MechSourceCode::String(
            "implicit-rollback-partial := implicit-rollback-anchor + 1".to_string(),
        ),
        MechSourceCode::String(
            "implicit-rollback-failure := missing-implicit-rollback-value + 1".to_string(),
        ),
    ]);

    let error = runtime.run_source_with_context(&mut context, &source);

    assert_eq!(error.unwrap_err().kind_name(), "UndefinedVariable");
    assert!(
        runtime
            .program
            .root_symbol_value("implicit-rollback-partial")
            .is_err()
    );
    assert_eq!(runtime.program.interpreter().plan_len(), plan_len_before);
    assert_eq!(
        runtime
            .program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str("implicit-rollback-anchor"))
            .unwrap()
            .addr(),
        anchor_address,
    );
    assert_eq!(
        runtime.live_state_snapshot().context_template.is_some(),
        live_before.context_template.is_some(),
    );
    assert_eq!(
        runtime.live_state_snapshot().input_bindings,
        live_before.input_bindings,
    );
    assert_eq!(
        runtime.live_state_snapshot().persistent_sends.len(),
        live_before.persistent_sends.len(),
    );
    assert_eq!(
        runtime.live_state_snapshot().registration_mode,
        live_before.registration_mode,
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(
        runtime.list_transactions(None).unwrap().len(),
        transactions_before,
    );
    let events = runtime.list_events(None).unwrap();
    let new_events = &events[events_before..];
    assert!(
        new_events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }) })
    );
    assert!(
        new_events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }) })
    );
    assert!(
        !new_events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. }) })
    );
}

#[test]
fn completion_event_staging_failure_rolls_back_implicit_program() {
    let mut runtime = MechRuntime::builder()
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [
                EventId(100),
                EventId(101),
                EventId(102),
                EventId(102),
                EventId(103),
                EventId(104),
            ],
        ))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "completion-event-staging-failure := 1")
        .unwrap_err();

    assert_eq!(error.kind_name(), "InvalidRuntimeTransaction");
    assert!(
        runtime
            .program
            .root_symbol_value("completion-event-staging-failure")
            .is_err()
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.list_transactions(None).unwrap().is_empty());
    let events = runtime.list_events(None).unwrap();
    assert!(
        events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }) })
    );
    assert!(
        events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }) })
    );
    assert!(
        !events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. }) })
    );
    assert!(!runtime.is_poisoned());
}
