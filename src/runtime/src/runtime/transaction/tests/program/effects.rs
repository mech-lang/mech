use super::support::{CommitDecisionEffect, invoke_host_callback, savepoint_effect};
use crate::runtime::gate_a_probe::{gate_a_cost_snapshot, reset_gate_a_costs};
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, InMemoryDocsProvider, MechRuntime, PlannedStagedHostFunction,
    PreparedRuntimeEffect, RuntimeCapabilityOperation, RuntimeEventKind, RuntimePreparedHostCall,
    RuntimeResourceWriteIntent, RuntimeResourceWriteRequest, RuntimeValueSnapshot,
};
use mech_core::{LegacyValue, MResult, MechSourceCode};
use std::sync::{Arc, Mutex};

fn snapshot(value: LegacyValue) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn staged_host_effect_records_execution_session_transaction_snapshot() {
    let name = "gate-a/session-snapshot";
    let mut runtime = test_runtime_builder()
        .host_function(PlannedStagedHostFunction::new(
            name,
            |_context, _args| Ok(RuntimeValueSnapshot::empty()),
            |_context, _args| {
                Ok(RuntimePreparedHostCall {
                    value: RuntimeValueSnapshot::empty(),
                    effect: savepoint_effect("session-snapshot"),
                })
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, CapabilityId(799), name);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    reset_gate_a_costs();
    invoke_host_callback(&mut runtime, &mut context, name).unwrap();
    let costs = gate_a_cost_snapshot();

    assert_eq!(costs.runtime_transaction_savepoint_clone_count, 1);
    assert_eq!(costs.runtime_transaction_savepoint_items, 3);
    runtime
        .abort_runtime_transaction(&mut context, "probe fixture complete")
        .unwrap();
}

#[test]
fn resource_provider_staging_failure_leaves_effect_journal_unchanged() {
    let mut runtime = MechRuntime::builder()
        .resource_provider(Box::new(InMemoryDocsProvider::new()))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let result = runtime.write_resource_with_context(
        &mut context,
        RuntimeResourceWriteRequest {
            base_uri: "docs://manual".to_string(),
            path: String::new(),
            context_name: "manual".to_string(),
            operation: RuntimeCapabilityOperation::Write,
            value: LegacyValue::Bool(mech_core::Ref::new(true)),
            intent: RuntimeResourceWriteIntent::Assign,
        },
    );

    assert!(result.is_err());
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .effects
            .len(),
        0,
    );

    runtime
        .abort_runtime_transaction(&mut context, "discard failed staging")
        .unwrap();
}

#[test]
fn committed_implicit_participant_failure_never_rolls_back_program() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut builder = test_runtime_builder();
    for (name, fail_commit) in [
        ("participant-commit/first", false),
        ("participant-commit/second", true),
    ] {
        let effect_log = log.clone();
        builder = builder
            .host_function(PlannedStagedHostFunction::new(
                name,
                |_context, _args| Ok(snapshot(LegacyValue::F64(mech_core::Ref::new(1.0)))),
                move |_context, _args| {
                    Ok(RuntimePreparedHostCall {
                        value: snapshot(LegacyValue::F64(mech_core::Ref::new(1.0))),
                        effect: PreparedRuntimeEffect::Transactional(Box::new(
                            CommitDecisionEffect {
                                name,
                                log: effect_log.clone(),
                                fail_commit,
                            },
                        )),
                    })
                },
            ))
            .unwrap();
    }
    let mut runtime = builder.build().unwrap();
    for (id, name) in [
        (CapabilityId(800), "participant-commit/first"),
        (CapabilityId(801), "participant-commit/second"),
    ] {
        grant_host_call(&mut runtime, id, name);
    }
    let mut context = runtime.runtime_context().unwrap();

    let operation: MResult<()> = runtime.with_atomic_program_operation(
        &mut context,
        "participant_commit_failure_test",
        |runtime, context| {
            runtime.program.run_source(&MechSourceCode::String(
                "participant-commit-symbol := 41".to_string(),
            ))?;
            invoke_host_callback(runtime, context, "participant-commit/first")?;
            invoke_host_callback(runtime, context, "participant-commit/second")?;
            Ok(())
        },
    );
    let error = operation.unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "participant-commit/first:prepare",
            "participant-commit/second:prepare",
            "participant-commit/first:commit",
            "participant-commit/second:commit",
        ],
    );
    assert!(
        runtime
            .program
            .root_symbol_value("participant-commit-symbol")
            .is_ok()
    );
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.is_poisoned());
    let transactions = runtime.list_transactions(None).unwrap();
    assert_eq!(transactions.len(), 1);
    assert!(
        !runtime
            .list_events(None)
            .unwrap()
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }) })
    );
}
