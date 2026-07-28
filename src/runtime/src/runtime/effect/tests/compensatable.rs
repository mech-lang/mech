use std::sync::{Arc, Mutex};

use crate::{
    MechRuntime, ObjectId, ObjectRecord, PreparedRuntimeEffect, RuntimeEventKind, RuntimeHealth,
};

use super::{compensatable, transactional, PanicEffectPhase, PanickingCompensatableEffect};

#[test]
fn apply_failure_compensates_and_aborts_in_reverse_phase_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional(
                "transactional",
                log.clone(),
            ))),
        )
        .unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(compensatable("first", log.clone()))),
        )
        .unwrap();
    let mut second = compensatable("second", log.clone());
    second.fail_apply = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(second)),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "transactional:prepare",
            "first:apply",
            "second:apply",
            "first:compensate",
            "transactional:abort",
        ],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(!runtime.is_poisoned());
    assert!(context
        .events
        .iter()
        .any(|event| { matches!(event.kind, RuntimeEventKind::EffectCompensated { .. }) }));
    assert!(context
        .events
        .iter()
        .any(|event| { matches!(event.kind, RuntimeEventKind::EffectAborted { .. }) }));

    runtime
        .abort_runtime_transaction(&mut context, "apply test cleanup")
        .unwrap();
}

#[test]
fn store_failure_compensates_effect_and_keeps_transaction_active() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(compensatable(
                "reversible",
                log.clone(),
            ))),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(900), "missing", "update"),
        )
        .unwrap();

    assert!(runtime
        .commit_runtime_transaction_detailed(&mut context)
        .is_err());

    assert_eq!(
        *log.lock().unwrap(),
        vec!["reversible:apply", "reversible:compensate"],
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(!runtime.is_poisoned());

    runtime
        .abort_runtime_transaction(&mut context, "store test cleanup")
        .unwrap();
}

#[test]
fn compensation_failure_poisons_runtime_with_complete_diagnostic() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let mut first = compensatable("first", log.clone());
    first.fail_compensate = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(first)),
        )
        .unwrap();
    let mut second = compensatable("second", log.clone());
    second.fail_apply = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(second)),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    let poison = match runtime.health() {
        RuntimeHealth::Healthy => panic!("runtime should be poisoned"),
        RuntimeHealth::Poisoned(poison) => poison,
    };
    assert!(poison.original_error.contains("second apply failed"));
    assert!(poison
        .rollback_failures
        .iter()
        .any(|failure| failure.contains("first compensate failed")));
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
            event.kind,
            RuntimeEventKind::EffectCompensationFailed { .. }
        )
    }));

    assert!(runtime
        .abort_runtime_transaction(&mut context, "poison test cleanup")
        .is_err());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn compensatable_apply_panic_is_retryable_after_successful_cleanup() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(PanickingCompensatableEffect {
                name: "apply",
                panic_at: Some(PanicEffectPhase::Apply),
                log: log.clone(),
            })),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert_eq!(*log.lock().unwrap(), vec!["apply:apply"]);
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(!runtime.is_poisoned());
}
