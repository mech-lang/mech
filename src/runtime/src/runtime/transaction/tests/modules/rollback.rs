use super::super::RuntimeModuleJournal;
use super::module;
use crate::{MechRuntime, RuntimeConfig, RuntimeInvalidOperationError};
use mech_core::MechError;

#[test]
fn rollback_retains_prefix_and_removes_suffix() {
    let mut journal = RuntimeModuleJournal::new();
    let first = module("memory://first.mec", "first");
    let second = module("memory://second.mec", "second");
    journal.stage_module(first.clone()).unwrap();
    let mark = journal.mark();
    journal.stage_module(second.clone()).unwrap();

    journal.rollback_to(mark).unwrap();

    assert_eq!(journal.get_module(first.id), Some(&first));
    assert!(journal.get_module(second.id).is_none());
    assert!(journal.find_module_by_name("memory://second.mec").is_none(),);
}

#[test]
fn atomic_operation_rollback_removes_later_module_work() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let earlier = module("memory://earlier.mec", "earlier");
    let later = module("memory://later.mec", "later");
    runtime
        .active_execution_transaction_mut(transaction_id)
        .unwrap()
        .modules
        .stage_module(earlier.clone())
        .unwrap();

    let error = runtime
        .with_atomic_module_operation(
            &mut context,
            "module_journal_savepoint_test",
            |runtime, _| {
                runtime
                    .active_execution_transaction_mut(transaction_id)?
                    .modules
                    .stage_module(later.clone())?;
                Err::<(), _>(MechError::new(
                    RuntimeInvalidOperationError {
                        operation: "module_journal_savepoint_test",
                        reason: "deliberate failure".to_string(),
                    },
                    None,
                ))
            },
        )
        .unwrap_err();

    assert!(error.kind_as::<RuntimeInvalidOperationError>().is_some());
    let journal = &runtime
        .active_execution_transaction(transaction_id)
        .unwrap()
        .modules;
    assert_eq!(journal.get_module(earlier.id), Some(&earlier));
    assert!(journal.get_module(later.id).is_none());
}
