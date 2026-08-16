use super::new_runtime;
use crate::PreparedRuntimeEffect;
use crate::runtime::test_support::effects::{EffectLifecycleLog, TransactionalEffectProbe};
use crate::{
    ModuleVersionId, ModuleVersionRecord, RuntimeHealth, RuntimeModuleJournalConflict, module_id,
};

#[test]
fn module_journal_validation_precedes_effect_preparation() {
    let mut runtime = new_runtime();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .active_runtime_transaction_mut(transaction_id)
        .unwrap()
        .modules
        .stage_version(ModuleVersionRecord::new(
            ModuleVersionId(10),
            module_id("memory://missing.mec"),
            1,
        ))
        .unwrap();
    let lifecycle = EffectLifecycleLog::default();
    runtime
        .active_runtime_transaction_mut(transaction_id)
        .unwrap()
        .effects
        .stage(
            transaction_id,
            PreparedRuntimeEffect::Transactional(Box::new(TransactionalEffectProbe::new(
                "module-validation-probe",
                lifecycle.clone(),
            ))),
        );

    let error = runtime
        .commit_runtime_transaction(&mut context)
        .unwrap_err();

    assert!(error.kind_as::<RuntimeModuleJournalConflict>().is_some(),);
    assert!(lifecycle.observations().is_empty());
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(matches!(runtime.health(), RuntimeHealth::Healthy));
}
