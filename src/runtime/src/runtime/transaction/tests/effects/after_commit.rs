use std::sync::{Arc, Mutex};

use crate::{MechRuntime, PreparedRuntimeEffect, RuntimeEffectFailurePhase, RuntimeEventKind};

use super::{
    FailingEventIdGenerator, PanicEffectPhase, PanickingAfterCommitEffect, after_commit,
    transactional,
};

#[test]
fn committed_effect_audit_failure_still_delivers_after_commit() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
        .id_generator(FailingEventIdGenerator::new([6]))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
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
            PreparedRuntimeEffect::AfterCommit(Box::new(after_commit("after", log.clone()))),
        )
        .unwrap();

    let outcome = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert_eq!(outcome.delivery_failures, Vec::new());
    assert_eq!(outcome.audit_failures.len(), 1);
    assert_eq!(
        outcome.audit_failures[0].phase,
        RuntimeEffectFailurePhase::Audit,
    );
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "transactional:prepare",
            "transactional:commit",
            "after:deliver",
        ],
    );
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, None);
}

#[test]
fn after_commit_delivery_failure_keeps_committed_runtime_healthy() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(after_commit("first", log.clone()))),
        )
        .unwrap();
    let mut second = after_commit("second", log.clone());
    second.fail_deliver = true;
    let failing_effect_id = runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(second)),
        )
        .unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(after_commit("third", log.clone()))),
        )
        .unwrap();

    let outcome = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert_eq!(outcome.transaction_id, transaction_id);
    assert_eq!(outcome.delivery_failures.len(), 1);
    assert_eq!(outcome.delivery_failures[0].effect_id, failing_effect_id,);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["first:deliver", "second:deliver", "third:deliver"],
    );
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, None);
    assert!(runtime.get_transaction(transaction_id).unwrap().is_some());
    let events = runtime.list_events(None).unwrap();
    assert!(
        events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::EffectDelivered { .. }) })
    );
    assert!(events.iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::EffectDeliveryFailed {
            effect_id,
            ..
          } if effect_id == failing_effect_id
        )
    }));
}

#[test]
fn after_commit_delivery_panic_continues_and_keeps_runtime_healthy() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    for (name, panic_at) in [("first", Some(PanicEffectPhase::Deliver)), ("second", None)] {
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                PreparedRuntimeEffect::AfterCommit(Box::new(PanickingAfterCommitEffect {
                    name,
                    panic_at,
                    log: log.clone(),
                })),
            )
            .unwrap();
    }

    let outcome = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert_eq!(outcome.delivery_failures.len(), 1);
    assert_eq!(
        *log.lock().unwrap(),
        vec!["first:deliver", "second:deliver"],
    );
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(!runtime.is_poisoned());
}
