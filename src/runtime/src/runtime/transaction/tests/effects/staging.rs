use std::sync::{Arc, Mutex};

use crate::{
    ActiveRuntimeEffectPhase, InMemorySourceResolver, MechRuntime, PreparedRuntimeEffect,
    RuntimeEffectId, RuntimeEventKind, SourceRequest, TransactionId,
    runtime::effect_journal::RuntimeEffectJournal,
};

use super::{SensitiveAfterCommit, after_commit, transactional};

#[test]
fn exact_resident_effect_staging_accepts_sparse_ordinals_and_cleans_rejections() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let transaction = TransactionId(77);
    let mut journal = RuntimeEffectJournal::new();
    journal
        .stage_exact(
            RuntimeEffectId {
                transaction,
                sequence: 4,
            },
            PreparedRuntimeEffect::Transactional(Box::new(transactional("four", log.clone()))),
        )
        .expect("first materialized effect may have a sparse plan ordinal");
    journal
        .stage_exact(
            RuntimeEffectId {
                transaction,
                sequence: 9,
            },
            PreparedRuntimeEffect::Transactional(Box::new(transactional("nine", log.clone()))),
        )
        .expect("later materialized effects need only be strictly ordered");
    assert_eq!(journal.next_sequence(), 10);

    let rejected = journal.stage_exact(
        RuntimeEffectId {
            transaction,
            sequence: 8,
        },
        PreparedRuntimeEffect::Transactional(Box::new(transactional("rejected", log.clone()))),
    );
    assert!(rejected.is_err());
    assert_eq!(log.lock().unwrap().as_slice(), ["rejected:abort"]);

    assert!(journal.abort_all().is_empty());
    assert_eq!(
        log.lock().unwrap().as_slice(),
        ["rejected:abort", "nine:abort", "four:abort"]
    );
}

#[test]
fn transaction_history_persists_effect_metadata_without_payload() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let secret = "raw-secret-payload-must-not-be-durable";
    let effect_id = runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(SensitiveAfterCommit {
                secret_payload: secret.to_string(),
            })),
        )
        .unwrap();

    runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    let transaction = runtime.get_transaction(transaction_id).unwrap().unwrap();
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(transaction.effects[0].id, effect_id);
    assert_eq!(
        transaction.effects[0].protocol,
        crate::RuntimeEffectProtocol::AfterCommit,
    );
    assert_eq!(
        transaction.effects[0].resource.as_deref(),
        Some("test://metadata-only"),
    );
    assert!(!format!("{:?}", transaction).contains(secret));
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::EffectDelivered { effect_id: delivered }
            if delivered == effect_id
        )
    }));
}

#[test]
fn mutation_is_rejected_while_an_effect_phase_is_active() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .active_effect_phase
        .set(Some(ActiveRuntimeEffectPhase::Preparing));

    let error = runtime.begin_transaction(&mut context).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectOperationReentrant");
    assert_eq!(context.transaction, None);
    assert!(runtime.active_transactions.is_empty());
}

#[test]
fn source_resolver_replacement_is_rejected_while_an_effect_phase_is_active() {
    let mut runtime = MechRuntime::builder()
        .source_resolver(InMemorySourceResolver::new().with_string("retained-source", "x := 1"))
        .build()
        .unwrap();
    runtime
        .active_effect_phase
        .set(Some(ActiveRuntimeEffectPhase::Preparing));

    let error = runtime
        .set_source_resolver(InMemorySourceResolver::new())
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectOperationReentrant");
    assert!(
        runtime
            .source_resolver()
            .resolve(&SourceRequest::new("retained-source"))
            .unwrap()
            .is_some()
    );
}

#[test]
fn broken_effect_identity_poisons_before_external_work() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(after_commit("identity", log.clone()))),
        )
        .unwrap();
    runtime
        .active_runtime_transaction_mut(transaction_id)
        .unwrap()
        .effects
        .entries[0]
        .id
        .transaction = TransactionId(transaction_id.0.saturating_add(1));

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    assert!(log.lock().unwrap().is_empty());
    assert_eq!(context.transaction, Some(transaction_id));
}
