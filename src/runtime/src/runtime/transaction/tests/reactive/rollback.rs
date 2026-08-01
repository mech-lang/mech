use super::super::super::program::{
    reset_runtime_program_checkpoint_count, runtime_program_checkpoint_count,
};
use super::{ReactiveTransactionalProbe, add_panicking_test_function, add_test_function};
use crate::runtime::test_support::{
    capabilities::grant_read,
    providers::{test_provider_with, test_runtime},
    values::source_cell,
};
use crate::{
    MechRuntime, ObjectId, ObjectRecord, PreparedRuntimeEffect, ResourceBudget, RuntimeEventKind,
    RuntimeHealth, RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputValue,
};
use mech_core::{FunctionCatalogBuilder, GenericError, MResult, MechError};
use mech_engine::FunctionSystem;
use std::any::Any;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Mutex};

fn panic_message(panic: Box<dyn Any + Send>) -> String {
    if let Some(message) = panic.downcast_ref::<String>() {
        return message.clone();
    }
    if let Some(message) = panic.downcast_ref::<&'static str>() {
        return (*message).to_string();
    }
    "non-string reactive panic payload".to_string()
}

#[test]
fn function_system_survives_reactive_program_replacement() {
    let catalog = Arc::new(FunctionCatalogBuilder::new().build().unwrap());
    let function_system = FunctionSystem::from_catalog(Arc::clone(&catalog));
    let legacy_boundary = Arc::clone(function_system.legacy_boundary());
    let mut runtime = MechRuntime::builder()
        .function_system(function_system)
        .build()
        .unwrap();
    let _ = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "function_system_identity",
            |runtime, _| {
                assert!(Arc::ptr_eq(runtime.program.function_catalog(), &catalog));
                assert!(Arc::ptr_eq(
                    runtime.program.function_system().legacy_boundary(),
                    &legacy_boundary,
                ));
                Ok(())
            },
        )
        .unwrap();

    assert!(Arc::ptr_eq(runtime.program.function_catalog(), &catalog));
    assert!(Arc::ptr_eq(
        runtime.program.function_system().legacy_boundary(),
        &legacy_boundary,
    ));
}

#[test]
fn reactive_step_budget_failure_cleans_up_without_poisoning() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, calls) = add_test_function(&mut runtime, None);
    let mut context = runtime
        .runtime_context()
        .unwrap()
        .with_budget(ResourceBudget::default().with_max_steps(0));
    reset_runtime_program_checkpoint_count();

    let error = runtime.step_with_context(&mut context, 0).unwrap_err();

    assert_eq!(error.kind_name(), "ResourceBudgetExceeded");
    assert_eq!(*output.borrow(), 0);
    assert_eq!(*calls.borrow(), 0);
    assert!(!runtime.is_poisoned());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert_eq!(runtime_program_checkpoint_count(), 0);
}

#[test]
fn failed_implicit_turn_restores_program_and_removes_envelope() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, calls) = add_test_function(&mut runtime, Some(1));
    reset_runtime_program_checkpoint_count();

    let error = runtime.step(1).unwrap_err();

    assert_eq!(error.kind_name(), "GenericError");
    assert_eq!(*output.borrow(), 0);
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(runtime_program_checkpoint_count(), 0);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
}

#[test]
fn implicit_reactive_panic_cleans_transaction_before_unwind() {
    const PANIC_MESSAGE: &str = "deliberate implicit reactive panic";

    let mut runtime = MechRuntime::builder().build().unwrap();
    let _output = add_panicking_test_function(&mut runtime, PANIC_MESSAGE);
    let mut context = runtime.runtime_context().unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        runtime.step_with_context(&mut context, 1).unwrap();
    }))
    .expect_err("reactive panic must escape");

    assert_eq!(panic_message(panic), PANIC_MESSAGE);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(
        runtime.runtime_health(),
        RuntimeHealth::Poisoned(_)
    ));

    let error = runtime.run_string("after-panic := 1").unwrap_err();
    assert_eq!(error.kind_name(), "RuntimePoisoned");
    assert_ne!(error.kind_name(), "RuntimeProgramBusy");
}

#[test]
fn explicit_reactive_panic_aborts_transaction_before_unwind() {
    const PANIC_MESSAGE: &str = "deliberate explicit reactive panic";

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let _output = add_panicking_test_function(&mut runtime, PANIC_MESSAGE);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(ReactiveTransactionalProbe {
                log: log.clone(),
                fail_prepare: false,
                fail_commit: false,
                fail_abort: false,
            })),
        )
        .unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        runtime.step_with_context(&mut context, 1).unwrap();
    }))
    .expect_err("reactive panic must escape");

    assert_eq!(panic_message(panic), PANIC_MESSAGE);
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.get_transaction(transaction_id).unwrap().is_none());
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(*log.lock().unwrap(), vec!["abort"]);
    assert!(matches!(
        runtime.runtime_health(),
        RuntimeHealth::Poisoned(_)
    ));
}

#[test]
fn host_input_reactive_panic_cleans_transaction_before_unwind() {
    const PANIC_MESSAGE: &str = "deliberate host-input reactive panic";
    const CLOCK_URI: &str = "test://clock/ticks";

    let mut runtime = test_runtime(test_provider_with(CLOCK_URI, "value", 1.0));
    grant_read(&mut runtime, CLOCK_URI, "value");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();
    let source = RuntimeHostInputSource::new(CLOCK_URI, "value").unwrap();
    let _output = add_panicking_test_function(&mut runtime, PANIC_MESSAGE);
    let input_cell = source_cell(&runtime, &source);
    let plan = runtime.program.interpreter().plan();
    let panic_node = plan.borrow().len() - 1;
    assert!(
        plan.borrow_mut()
            .add_reactive_dependency(panic_node, input_cell)
    );
    let events_before = runtime.list_events(None).unwrap().len();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        runtime
            .apply_host_input_with_context(
                &mut context,
                RuntimeHostInput::single(source, RuntimeHostInputValue::F64(9.0)),
            )
            .unwrap();
    }))
    .expect_err("reactive panic must escape from the host-input turn");

    assert_eq!(panic_message(panic), PANIC_MESSAGE);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.active_program_operation.get().is_none());
    assert!(matches!(
        runtime.runtime_health(),
        RuntimeHealth::Poisoned(_)
    ));
    let operation_events = &runtime.list_events(None).unwrap()[events_before..];
    assert!(
        operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
    );
    assert!(
        !operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionCommitted { .. }))
    );
    assert!(!operation_events.iter().any(|event| matches!(
        event.kind,
        RuntimeEventKind::TransactionalEffectCommitted { .. }
            | RuntimeEventKind::EffectDelivered { .. }
    )));
}

#[test]
fn reactive_panic_cleanup_failure_poisons_before_unwind() {
    const PANIC_MESSAGE: &str = "deliberate cleanup-failure reactive panic";

    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let _output = add_panicking_test_function(&mut runtime, PANIC_MESSAGE);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(ReactiveTransactionalProbe {
                log: log.clone(),
                fail_prepare: false,
                fail_commit: false,
                fail_abort: true,
            })),
        )
        .unwrap();

    let panic = catch_unwind(AssertUnwindSafe(|| {
        runtime.step_with_context(&mut context, 1).unwrap();
    }))
    .expect_err("reactive panic must escape despite cleanup failure");

    assert_eq!(panic_message(panic), PANIC_MESSAGE);
    assert_eq!(*log.lock().unwrap(), vec!["abort"]);
    assert!(runtime.get_transaction(transaction_id).unwrap().is_none());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    let RuntimeHealth::Poisoned(poison) = runtime.runtime_health() else {
        panic!("runtime must be poisoned after panic cleanup failure");
    };
    assert!(poison.original_error.contains(PANIC_MESSAGE));
    assert!(
        poison
            .rollback_failures
            .iter()
            .any(|failure| failure.contains("deliberate reactive abort failure"))
    );
    assert!(poison.rollback_failures.iter().any(|failure| failure.contains(
        "retained program state is not trusted after panic unwound through the compact reactive journal",
    )));
}

#[test]
fn failed_explicit_turn_releases_or_preserves_ownership_by_position() {
    let mut first_runtime = MechRuntime::builder().build().unwrap();
    let (first_output, _) = add_test_function(&mut first_runtime, Some(1));
    let mut first_context = first_runtime.runtime_context().unwrap();
    let first_transaction = first_runtime.begin_transaction(&mut first_context).unwrap();

    assert!(
        first_runtime
            .step_with_context(&mut first_context, 1)
            .is_err()
    );
    assert_eq!(*first_output.borrow(), 0);
    assert_eq!(first_runtime.program_transaction_owner, None);
    assert!(
        first_runtime
            .active_execution_transaction(first_transaction)
            .unwrap()
            .program
            .is_none()
    );
    first_runtime
        .abort_runtime_transaction(&mut first_context, "first failure")
        .unwrap();

    let mut later_runtime = MechRuntime::builder().build().unwrap();
    let (later_output, _) = add_test_function(&mut later_runtime, Some(2));
    let mut later_context = later_runtime.runtime_context().unwrap();
    let later_transaction = later_runtime.begin_transaction(&mut later_context).unwrap();
    later_runtime
        .step_with_context(&mut later_context, 1)
        .unwrap();

    assert!(
        later_runtime
            .step_with_context(&mut later_context, 1)
            .is_err()
    );
    assert_eq!(*later_output.borrow(), 1);
    assert_eq!(
        later_runtime.program_transaction_owner,
        Some(later_transaction),
    );
    assert!(
        later_runtime
            .active_execution_transaction(later_transaction)
            .unwrap()
            .program
            .is_some()
    );
    later_runtime
        .abort_runtime_transaction(&mut later_context, "later failure")
        .unwrap();
    assert_eq!(*later_output.borrow(), 0);
}

#[test]
fn failed_first_explicit_turn_retains_owner_when_rollback_is_incomplete() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let error: MechError = runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "incomplete_first_explicit_turn_rollback",
            |runtime, context| {
                runtime.stage_runtime_effect_with_context(
                    context,
                    PreparedRuntimeEffect::Transactional(Box::new(ReactiveTransactionalProbe {
                        log: log.clone(),
                        fail_prepare: false,
                        fail_commit: false,
                        fail_abort: true,
                    })),
                )?;
                Err::<(), _>(MechError::new(
                    GenericError {
                        msg: "deliberate first explicit turn failure".to_string(),
                    },
                    None,
                ))
            },
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(format!("{error:?}").contains("deliberate first explicit turn failure"));
    assert!(format!("{error:?}").contains("deliberate reactive abort failure"));
    assert_eq!(*output.borrow(), 0);
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id),);
    assert!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .program
            .is_some()
    );
    assert!(runtime.is_poisoned());
    assert_eq!(*log.lock().unwrap(), vec!["abort"]);
}

#[test]
fn failed_reactive_turn_rolls_back_staged_object_and_program_state() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(800);

    let result: MResult<()> = runtime.with_atomic_reactive_turn_for_test(
        &mut context,
        "failed_object_turn",
        |runtime, context| {
            runtime.put_object_with_context(
                context,
                ObjectRecord::text(object_id, "note", "provisional"),
            )?;
            Err(MechError::new(
                GenericError {
                    msg: "deliberate object turn failure".to_string(),
                },
                None,
            ))
        },
    );

    assert_eq!(result.unwrap_err().kind_name(), "GenericError");
    assert_eq!(*output.borrow(), 0);
    assert!(runtime.get_object(object_id).unwrap().is_none());
    assert!(runtime.active_transactions.is_empty());
}

#[test]
fn pre_store_effect_failure_rolls_back_reactive_and_runtime_state() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(930);

    let error = runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "reactive_prepare_failure",
            |runtime, context| {
                runtime.put_object_with_context(
                    context,
                    ObjectRecord::text(object_id, "note", "must roll back"),
                )?;
                runtime.stage_runtime_effect_with_context(
                    context,
                    PreparedRuntimeEffect::Transactional(Box::new(ReactiveTransactionalProbe {
                        log: log.clone(),
                        fail_prepare: true,
                        fail_commit: false,
                        fail_abort: false,
                    })),
                )?;
                Ok(())
            },
        )
        .unwrap_err();

    assert!(format!("{error:?}").contains("deliberate reactive prepare failure"));
    assert_eq!(*output.borrow(), 0);
    assert!(runtime.get_object(object_id).unwrap().is_none());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(matches!(runtime.health, RuntimeHealth::Healthy));
    assert_eq!(*log.lock().unwrap(), vec!["prepare", "abort"]);
}
