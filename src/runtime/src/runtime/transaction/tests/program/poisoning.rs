use super::super::{
    ProgramTransactionTestFault, set_program_transaction_test_fault,
};
use crate::{MechRuntime, RuntimeHealth, RuntimeInvalidOperationError};
use mech_core::{MResult, MechError};

#[test]
fn implicit_cleanup_failure_returns_rollback_error_and_poisons_runtime() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let mut implicit_transaction_id = None;

    let result: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "implicit_cleanup_failure_test",
        |_runtime, context| {
            implicit_transaction_id = context.transaction;
            set_program_transaction_test_fault(ProgramTransactionTestFault::FailImplicitStoreAbort);
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "implicit_cleanup_failure_test",
                    reason: "deliberate implicit execution failure".to_string(),
                },
                None,
            ))
        },
    );

    let transaction_id = implicit_transaction_id.expect("implicit transaction should be active");
    let error = result.unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(runtime.is_poisoned());
    let poison = match runtime.health() {
        RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
        RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(
        poison
            .original_error
            .contains("deliberate implicit execution failure"),
    );
    assert!(poison.rollback_failures.iter().any(|failure| {
        failure.contains("staged store discard invariant failed")
            && failure.contains("transaction is not open")
    }));
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.active_program_operation.get().is_none());
}

#[test]
fn missing_implicit_envelope_during_cleanup_is_not_hidden() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let mut implicit_transaction_id = None;

    let result: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "missing_implicit_envelope_test",
        |_runtime, context| {
            implicit_transaction_id = context.transaction;
            set_program_transaction_test_fault(
                ProgramTransactionTestFault::RemoveImplicitEnvelopeBeforeCleanup,
            );
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "missing_implicit_envelope_test",
                    reason: "deliberate implicit execution failure".to_string(),
                },
                None,
            ))
        },
    );

    let transaction_id = implicit_transaction_id.expect("implicit transaction should be active");
    let error = result.unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(runtime.is_poisoned());
    let poison = match runtime.health() {
        RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
        RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(
        poison
            .original_error
            .contains("deliberate implicit execution failure"),
    );
    assert!(poison.rollback_failures.iter().any(|failure| {
        failure.contains("implicit transaction cleanup") && failure.contains("could not start")
    }));
    assert!(poison.rollback_failures.iter().any(|failure| {
        failure.contains("program owner still references implicit transaction")
    }));
    assert!(poison.rollback_failures.iter().any(|failure| {
        failure.contains("runtime context still references implicit transaction")
    }));
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.active_program_operation.get().is_none());
}

#[test]
fn incomplete_program_restore_poisons_retained_execution_until_abort() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("rollback-poison-anchor := 1").unwrap();
    assert!(runtime.program.interpreter().plan_len() > 0);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let result: MResult<()> =
        runtime.with_atomic_program_operation(&mut context, "poison_test", |runtime, _context| {
            runtime.program.interpreter().plan().0.borrow_mut().clear();
            Err(MechError::new(
                RuntimeInvalidOperationError {
                    operation: "poison_test",
                    reason: "deliberate original failure".to_string(),
                },
                None,
            ))
        });

    let error = result.unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(runtime.is_poisoned());
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    let poison = match runtime.health() {
        RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
        RuntimeHealth::Poisoned(poison) => poison,
    };
    assert_eq!(poison.operation, "poison_test");
    assert!(
        poison
            .original_error
            .contains("deliberate original failure")
    );
    assert!(!poison.rollback_failures.is_empty());

    assert_eq!(
        runtime
            .run_string("poisoned-runtime-rejected-symbol := 1")
            .unwrap_err()
            .kind_name(),
        "RuntimePoisoned",
    );
    let mut fresh_context = runtime.runtime_context().unwrap();
    assert_eq!(
        runtime
            .begin_transaction(&mut fresh_context)
            .unwrap_err()
            .kind_name(),
        "RuntimePoisoned",
    );
    assert_eq!(
        runtime
            .commit_runtime_transaction(&mut context)
            .unwrap_err()
            .kind_name(),
        "RuntimePoisoned",
    );
    assert!(runtime.list_events(None).is_ok());
    assert!(
        runtime
            .program()
            .root_symbol_value("rollback-poison-anchor")
            .is_ok()
    );

    let abort_error = runtime
        .abort_runtime_transaction(&mut context, "release poisoned owner")
        .unwrap_err();
    assert_eq!(abort_error.kind_name(), "RuntimeProgramRollbackFailed");
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.shutdown().is_ok());
}
