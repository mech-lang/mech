use super::super::{CapabilityId, MechRuntime, RuntimeEventKind, Value};
use super::support::CommitDecisionEffect;
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::{
    InMemoryDocsProvider, PlannedStagedHostFunction, PreparedRuntimeEffect,
    RuntimeCapabilityOperation, RuntimePreparedHostCall, RuntimeResourceWriteIntent,
    RuntimeResourceWriteRequest,
};
use std::sync::{Arc, Mutex};

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
            value: Value::Bool(mech_core::Ref::new(true)),
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
    let mut builder = MechRuntime::builder();
    for (name, fail_commit) in [("round4/first", false), ("round4/second", true)] {
        let effect_log = log.clone();
        builder = builder
            .host_function(PlannedStagedHostFunction::new(
                name,
                |_context, _args| Ok(Value::F64(mech_core::Ref::new(1.0)).into()),
                move |_context, _args| {
                    Ok(RuntimePreparedHostCall {
                        value: Value::F64(mech_core::Ref::new(1.0)).into(),
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
        (CapabilityId(800), "round4/first"),
        (CapabilityId(801), "round4/second"),
    ] {
        grant_host_call(&mut runtime, id, name);
    }
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
    .run_string_with_context(
      &mut context,
      "round4-committed-symbol := 41\nfirst-result := round4/first()\nsecond-result := round4/second()",
    )
    .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "round4/first:prepare",
            "round4/second:prepare",
            "round4/first:commit",
            "round4/second:commit",
        ],
    );
    assert!(
        runtime
            .program
            .root_symbol_value("round4-committed-symbol")
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
