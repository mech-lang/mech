use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::{
    Capability, CapabilityId, ObjectId, ObjectRecord, PreparedRuntimeEffect, RuntimeEventKind,
};

use super::support::{
    counting_after_commit_effect, runtime_with_sources, staged_test_capability, test_module_options,
};

#[test]
fn retained_root_graph_begins_inside_hidden_program_transaction() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);

    runtime
        .resolve_and_run_root_module("root.mec", test_module_options())
        .unwrap();

    let events = runtime.list_events(None).unwrap();
    let transaction_started = events
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::TransactionStarted { .. }))
        .unwrap();
    let source_resolved = events
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::SourceResolved { .. }))
        .unwrap();
    let compiled = events
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::ModuleCompiled { .. }))
        .unwrap();

    assert!(transaction_started < source_resolved);
    assert!(source_resolved < compiled);
    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_some(),);
    assert!(runtime.root_symbol_value("answer").is_ok());
    assert!(runtime.active_transactions.is_empty());
}

#[test]
fn standalone_run_module_does_not_replace_retained_program() {
    let mut runtime = runtime_with_sources(&[("isolated.mec", "isolated := 42\nisolated\n")]);
    runtime.run_string("retained := 7").unwrap();
    let version = runtime
        .resolve_and_store_module_source("isolated.mec", test_module_options())
        .unwrap()
        .unwrap();

    runtime.run_module(version).unwrap();

    assert!(runtime.root_symbol_value("retained").is_ok());
    assert!(runtime.root_symbol_value("isolated").is_err());
}

#[test]
fn graph_object_capability_and_effect_commit_together() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .resolve_and_run_root_module_with_context(&mut context, "root.mec", test_module_options())
        .unwrap();
    let object = ObjectRecord::text(ObjectId(920), "module-transaction", "committed");
    runtime
        .put_object_with_context(&mut context, object.clone())
        .unwrap();
    let (capability, request) = staged_test_capability(&runtime, CapabilityId(920));
    runtime
        .grant_capability_with_context(&mut context, capability.clone())
        .unwrap();
    let deliveries = Arc::new(AtomicUsize::new(0));
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::AfterCommit(Box::new(counting_after_commit_effect(
                deliveries.clone(),
            ))),
        )
        .unwrap();

    runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap();

    assert!(runtime
        .store
        .find_module_by_name("memory:root.mec")
        .unwrap()
        .is_some(),);
    assert_eq!(runtime.get_object(object.id).unwrap(), Some(object));
    assert_eq!(runtime.check_capability(&request).unwrap(), capability.id(),);
    assert_eq!(deliveries.load(Ordering::SeqCst), 1);
    assert!(runtime.root_symbol_value("answer").is_ok());
}

#[test]
fn direct_resolution_ensure_activation_and_reactive_boundaries_are_unchanged() {
    let mut runtime = runtime_with_sources(&[("root.mec", "answer := 42\nanswer\n")]);
    let events_before = runtime.list_events(None).unwrap();
    let transactions_before = runtime.list_transactions(None).unwrap().len();

    assert!(runtime.resolve_source("root.mec").unwrap().is_some(),);
    assert_eq!(runtime.list_events(None).unwrap(), events_before);
    assert_eq!(
        runtime.list_transactions(None).unwrap().len(),
        transactions_before,
    );

    let direct_module = runtime
        .ensure_module("direct", "memory:direct.mec")
        .unwrap();
    assert!(runtime.store.get_module(direct_module).unwrap().is_some(),);
    assert_eq!(
        runtime.list_transactions(None).unwrap().len(),
        transactions_before,
    );

    let version = runtime
        .resolve_and_store_module_source("root.mec", test_module_options())
        .unwrap()
        .unwrap();
    let owner = runtime
        .store
        .get_module_version(version)
        .unwrap()
        .unwrap()
        .module;
    let transaction_count = runtime.list_transactions(None).unwrap().len();
    runtime.activate_module_version(owner, version).unwrap();
    assert_eq!(runtime.active_module_version(owner).unwrap(), Some(version),);
    assert_eq!(
        runtime.list_transactions(None).unwrap().len(),
        transaction_count,
    );

    runtime.run_string("reactive-boundary := 1").unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime.step_with_context(&mut context, 1).unwrap();
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id),);
    runtime
        .abort_runtime_transaction(&mut context, "reactive boundary cleanup")
        .unwrap();
}
