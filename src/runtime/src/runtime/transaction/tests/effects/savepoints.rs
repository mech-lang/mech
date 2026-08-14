use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::MResult;

use crate::{MechRuntime, PreparedRuntimeEffect, RuntimeEffectId, RuntimeEventKind, TransactionId};

use super::super::RuntimeEffectJournal;
use super::{CostedAfterCommit, FailOnceAbortEffect, effect, synthetic_error, transactional};

#[test]
fn journal_rollback_does_not_reuse_effect_sequences() {
    let transaction = TransactionId(7);
    let mut journal = RuntimeEffectJournal::new();

    assert_eq!(journal.stage(transaction, effect("a")).sequence, 0);
    let mark = journal.mark();
    assert_eq!(journal.stage(transaction, effect("b")).sequence, 1);
    assert!(journal.rollback_to(mark).is_empty());
    assert_eq!(journal.stage(transaction, effect("c")).sequence, 2);

    assert_eq!(journal.len(), 2);
    assert_eq!(journal.next_sequence(), 3);
}

#[test]
fn failed_savepoint_cleanup_preserves_effect_for_outer_abort_retry() {
    let attempts = Arc::new(AtomicUsize::new(0));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let result: MResult<()> = runtime.with_atomic_module_operation(
        &mut context,
        "fail_once_effect_cleanup",
        |runtime, context| {
            let effect_id = runtime.stage_runtime_effect_with_context(
                context,
                PreparedRuntimeEffect::Transactional(Box::new(FailOnceAbortEffect {
                    attempts: attempts.clone(),
                })),
            )?;
            assert_eq!(effect_id.sequence, 0);
            Err(synthetic_error("deliberate retained operation failure"))
        },
    );

    assert_eq!(
        result.unwrap_err().kind_name(),
        "RuntimeProgramRollbackFailed",
    );
    assert!(runtime.is_poisoned());
    let transaction = runtime
        .active_execution_transaction(transaction_id)
        .unwrap();
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(transaction.effects.next_sequence(), 1);
    assert_eq!(attempts.load(Ordering::SeqCst), 1);

    runtime
        .abort_runtime_transaction(&mut context, "retry retained effect abort")
        .unwrap();
    assert_eq!(attempts.load(Ordering::SeqCst), 2);
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn savepoint_rollback_discards_effect_and_staging_event() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let result: MResult<RuntimeEffectId> = runtime.with_atomic_module_operation(
        &mut context,
        "effect_staging_event_rollback",
        |runtime, context| {
            let effect_id =
                runtime.stage_runtime_effect_with_context(context, effect("rolled-back"))?;
            Err(synthetic_error(format!(
                "deliberate rollback for {}",
                effect_id,
            )))
        },
    );

    assert_eq!(result.unwrap_err().kind_name(), "SyntheticEffectError");
    assert!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .effects
            .is_empty()
    );
    assert!(
        !context
            .events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::EffectStaged { .. }) })
    );
    runtime
        .abort_runtime_transaction(&mut context, "test cleanup")
        .unwrap();
}

#[test]
fn rolled_back_effect_cost_is_not_refunded() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let bytes_before = context.budget.used_bytes;
    let items_before = context.budget.used_items;

    let result: MResult<()> = runtime.with_atomic_module_operation(
        &mut context,
        "costed_effect_failure",
        |runtime, context| {
            runtime.stage_runtime_effect_with_context(
                context,
                PreparedRuntimeEffect::AfterCommit(Box::new(CostedAfterCommit {
                    cost: crate::RuntimeEffectCost {
                        bytes: 17,
                        items: 3,
                    },
                })),
            )?;
            Err(synthetic_error("deliberate costed operation failure"))
        },
    );

    assert_eq!(result.unwrap_err().kind_name(), "SyntheticEffectError");
    assert_eq!(context.budget.used_bytes, bytes_before + 17);
    assert_eq!(context.budget.used_items, items_before + 3);
    let transaction = runtime
        .active_execution_transaction(transaction_id)
        .unwrap();
    assert_eq!(transaction.effects.len(), 0);
    assert_eq!(transaction.effects.next_sequence(), 1);

    runtime
        .abort_runtime_transaction(&mut context, "cost test cleanup")
        .unwrap();
}

#[test]
fn outer_abort_discards_effects_in_reverse_order() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    for name in ["first", "second", "third"] {
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                PreparedRuntimeEffect::Transactional(Box::new(transactional(name, log.clone()))),
            )
            .unwrap();
    }

    runtime
        .abort_runtime_transaction(&mut context, "discard")
        .unwrap();

    assert_eq!(
        *log.lock().unwrap(),
        vec!["third:abort", "second:abort", "first:abort"],
    );
}
