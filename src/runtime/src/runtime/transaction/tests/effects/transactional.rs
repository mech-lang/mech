use std::sync::{Arc, Mutex};

use crate::{
    MechRuntime, PreparedRuntimeEffect, RuntimeEventKind, RuntimeExternalCommitIndeterminate,
    RuntimeHealth,
};

use super::super::RuntimeExecutionTransactionState;
use super::{
    PanicEffectPhase, PanickingAfterCommitEffect, PanickingTransactionalEffect, after_commit,
    transactional,
};

#[test]
fn prepare_failure_aborts_prepared_participants_and_stays_retryable() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional("first", log.clone()))),
        )
        .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_prepare = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(second)),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert_eq!(
        *log.lock().unwrap(),
        vec!["first:prepare", "second:prepare", "first:abort"],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .state,
        RuntimeExecutionTransactionState::Active,
    );
    assert!(!runtime.is_poisoned());
    assert!(
        context.events.iter().any(|event| {
            matches!(event.kind, RuntimeEventKind::EffectPreparationFailed { .. })
        })
    );
    assert!(
        context
            .events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::EffectAborted { .. }) })
    );

    runtime
        .abort_runtime_transaction(&mut context, "prepare test cleanup")
        .unwrap();
}

#[test]
fn provider_commit_failure_after_store_commit_is_indeterminate() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional("first", log.clone()))),
        )
        .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_commit = true;
    let failing_effect_id = runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(second)),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    let indeterminate = error
        .kind_as::<RuntimeExternalCommitIndeterminate>()
        .unwrap();
    assert_eq!(indeterminate.transaction_id, transaction_id);
    assert_eq!(indeterminate.failures.len(), 1);
    assert_eq!(indeterminate.failures[0].effect_id, failing_effect_id,);
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "first:prepare",
            "second:prepare",
            "first:commit",
            "second:commit",
        ],
    );
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.get_transaction(transaction_id).unwrap().is_some());
    let events = runtime.list_events(None).unwrap();
    assert!(events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::TransactionalEffectCommitted { .. }
        )
    }));
    assert!(events.iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::ExternalCommitIndeterminate { .. }
        )
    }));
}

#[test]
fn every_prepared_participant_receives_commit_and_all_failures_are_reported() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let delivery_log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional("first", log.clone()))),
        )
        .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_commit = true;
    let second_id = runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(second)),
        )
        .unwrap();
    let mut third = transactional("third", log.clone());
    third.fail_commit = true;
    let third_id = runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(third)),
        )
        .unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(after_commit(
                "suppressed",
                delivery_log.clone(),
            ))),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();
    let indeterminate = error
        .kind_as::<RuntimeExternalCommitIndeterminate>()
        .unwrap();

    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "first:prepare",
            "second:prepare",
            "third:prepare",
            "first:commit",
            "second:commit",
            "third:commit",
        ],
    );
    assert_eq!(
        indeterminate
            .failures
            .iter()
            .map(|failure| failure.effect_id)
            .collect::<Vec<_>>(),
        vec![second_id, third_id],
    );
    assert!(delivery_log.lock().unwrap().is_empty());
    let poison = match runtime.health() {
        RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
        RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(
        poison
            .rollback_failures
            .iter()
            .any(|outcome| outcome.contains("second commit failed"))
    );
    assert!(
        poison
            .rollback_failures
            .iter()
            .any(|outcome| outcome.contains("third commit failed"))
    );
    assert_eq!(context.transaction, None);
    assert!(runtime.get_transaction(transaction_id).unwrap().is_some());
    assert_eq!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    RuntimeEventKind::ExternalCommitIndeterminate { .. }
                )
            })
            .count(),
        2,
    );
}

#[test]
fn transactional_prepare_panic_aborts_prior_participants_and_resets_phase() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    for (name, panic_at) in [("first", None), ("second", Some(PanicEffectPhase::Prepare))] {
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                PreparedRuntimeEffect::Transactional(Box::new(PanickingTransactionalEffect {
                    name,
                    panic_at,
                    log: log.clone(),
                })),
            )
            .unwrap();
    }

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert_eq!(
        *log.lock().unwrap(),
        vec!["first:prepare", "second:prepare", "first:abort"],
    );
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(!runtime.is_poisoned());
}

#[test]
fn transactional_commit_panic_notifies_remaining_participants_and_poisons() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    for (name, panic_at) in [("first", Some(PanicEffectPhase::Commit)), ("second", None)] {
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                PreparedRuntimeEffect::Transactional(Box::new(PanickingTransactionalEffect {
                    name,
                    panic_at,
                    log: log.clone(),
                })),
            )
            .unwrap();
    }
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(PanickingAfterCommitEffect {
                name: "suppressed",
                panic_at: None,
                log: log.clone(),
            })),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "first:prepare",
            "second:prepare",
            "first:commit",
            "second:commit",
        ],
    );
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
}
