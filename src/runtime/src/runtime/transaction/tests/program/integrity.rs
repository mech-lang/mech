use super::support::{CommitDecisionEffect, savepoint_effect};
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::ids::ScriptedEventIdGenerator;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::runtime::transaction::RuntimeExecutionTransactionState;
use crate::{
    CapabilityId, EventId, PlannedStagedHostFunction, PreparedRuntimeEffect, RuntimeEventKind,
    RuntimeIntegrityConstraintFailureReason, RuntimePreparedHostCall, RuntimeValueSnapshot,
};
use mech_core::{Value, hash_str};
use std::sync::{Arc, Mutex};

#[test]
fn invalid_implicit_program_operation_rolls_back_before_publication() {
    let mut runtime = test_runtime_builder().build().unwrap();
    runtime.run_string("integrity-anchor := 1.0").unwrap();
    let events_before = runtime.list_events(None).unwrap().len();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(
            &mut context,
            "integrity-discarded := 2.0\nintegrity-invalid! := false",
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-anchor")
            .is_ok()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-discarded")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-invalid!")
            .is_err()
    );
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    let events = runtime.list_events(None).unwrap();
    let new_events = &events[events_before..];
    assert!(
        new_events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }) })
    );
    assert!(
        !new_events
            .iter()
            .any(|event| { matches!(event.kind, RuntimeEventKind::ProgramCompleted { .. }) })
    );
    let audit_event = new_events
        .iter()
        .find(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::IntegrityConstraintViolated { .. }
            )
        })
        .expect("integrity rollback audit must be durable");
    let abort_position = new_events
        .iter()
        .position(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
        .expect("implicit transaction abort must be durable");
    let audit_position = new_events
        .iter()
        .position(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::IntegrityConstraintViolated { .. }
            )
        })
        .unwrap();
    assert!(
        abort_position < audit_position,
        "integrity audit must follow transaction cleanup",
    );
    let RuntimeEventKind::IntegrityConstraintViolated {
        violations: audit, ..
    } = &audit_event.kind
    else {
        unreachable!();
    };
    assert_eq!(audit.len(), 1);
    assert_eq!(audit[0].name, "integrity-invalid!");
    assert_eq!(
        audit[0].reason,
        RuntimeIntegrityConstraintFailureReason::EvaluatedFalse,
    );
    assert_eq!(audit[0].actual.as_deref(), Some("false"));
    assert_eq!(audit[0].expected.as_deref(), Some("true"));
    assert!(!format!("{audit:?}").contains("@0x"));
    #[cfg(feature = "serde")]
    {
        let serialized = serde_json::to_string(&audit_event.kind).unwrap();
        assert!(!serialized.contains("@0x"));
        assert!(!serialized.contains("RefCell"));
        assert!(!serialized.contains("tokens"));
    }
}

#[test]
fn integrity_audit_append_failure_preserves_original_error_and_health() {
    let mut runtime = test_runtime_builder()
        .id_generator(ScriptedEventIdGenerator::new(
            1,
            [
                EventId(100),
                EventId(101),
                EventId(102),
                EventId(103),
                EventId(104),
                EventId(100),
                EventId(105),
            ],
        ))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();

    let error = runtime
        .run_string_with_context(&mut context, "audit-invalid! := false")
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert_eq!(context.transaction, None);
    assert!(runtime.program.root_symbol_value("audit-invalid!").is_err());
    assert!(!runtime.is_poisoned());
    let events = runtime.list_events(None).unwrap();
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
    );
    assert!(
        events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::ProgramFailed { .. }))
    );
    assert!(events.iter().all(|event| !matches!(
        event.kind,
        RuntimeEventKind::IntegrityConstraintViolated { .. }
    )));
}

#[test]
fn invalid_explicit_program_operation_rolls_back_only_its_savepoint() {
    let mut runtime = test_runtime_builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "integrity-kept := 1.0\nintegrity-valid! := true",
        )
        .unwrap();

    let error = runtime
        .run_string_with_context(
            &mut context,
            "integrity-discarded := 2.0\nintegrity-invalid! := false",
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert!(runtime.program.root_symbol_value("integrity-kept").is_ok());
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-valid!")
            .is_ok()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-discarded")
            .is_err()
    );
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-invalid!")
            .is_err()
    );
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::IntegrityConstraintViolated {
            transaction_id: id,
            ..
          } if id == transaction_id
        )
    }));

    runtime
        .abort_runtime_transaction(&mut context, "discard explicit integrity test")
        .unwrap();
    assert_eq!(context.transaction, None);
    assert!(runtime.program.root_symbol_value("integrity-kept").is_err());
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
          event.kind,
          RuntimeEventKind::IntegrityConstraintViolated {
            transaction_id: id,
            ..
          } if id == transaction_id
        )
    }));
}

#[test]
fn invalid_explicit_integrity_suffix_discards_only_its_effects() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut builder = test_runtime_builder();
    for name in ["integrity/a", "integrity/b", "integrity/c"] {
        let effect_log = log.clone();
        builder = builder
            .host_function(PlannedStagedHostFunction::new(
                name,
                |_context, _args| {
                    RuntimeValueSnapshot::try_capture(&Value::F64(mech_core::Ref::new(1.0)))
                },
                move |_context, _args| {
                    Ok(RuntimePreparedHostCall {
                        value: RuntimeValueSnapshot::try_capture(&Value::F64(
                            mech_core::Ref::new(1.0),
                        ))?,
                        effect: PreparedRuntimeEffect::Transactional(Box::new(
                            CommitDecisionEffect {
                                name,
                                log: effect_log.clone(),
                                fail_commit: false,
                            },
                        )),
                    })
                },
            ))
            .unwrap();
    }
    let mut runtime = builder.build().unwrap();
    for (id, name) in [
        (CapabilityId(810), "integrity/a"),
        (CapabilityId(811), "integrity/b"),
        (CapabilityId(812), "integrity/c"),
    ] {
        grant_host_call(&mut runtime, id, name);
    }
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .run_string_with_context(&mut context, "a-result := integrity/a()\na-safe! := true")
        .unwrap();
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .effects
            .len(),
        1,
    );

    let error = runtime
        .run_string_with_context(&mut context, "b-result := integrity/b()\nb-safe! := false")
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.program.root_symbol_value("a-result").is_ok());
    assert!(runtime.program.root_symbol_value("b-result").is_err());
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .effects
            .len(),
        1,
    );
    assert_eq!(*log.lock().unwrap(), vec!["integrity/b:abort"],);
    assert!(!runtime.is_poisoned());

    runtime
        .run_string_with_context(&mut context, "c-result := integrity/c()\nc-safe! := true")
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(
        *log.lock().unwrap(),
        vec![
            "integrity/b:abort",
            "integrity/a:prepare",
            "integrity/c:prepare",
            "integrity/a:commit",
            "integrity/c:commit",
        ],
    );
    assert!(runtime.program.root_symbol_value("a-result").is_ok());
    assert!(runtime.program.root_symbol_value("c-result").is_ok());
    assert!(runtime.program.root_symbol_value("b-result").is_err());
    assert_eq!(context.transaction, None);
    assert!(runtime.active_transactions.is_empty());
    assert!(!runtime.is_poisoned());
}

#[test]
fn final_explicit_commit_revalidates_without_consuming_transaction() {
    let mut runtime = test_runtime_builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .run_string_with_context(&mut context, "integrity-final! := true")
        .unwrap();
    runtime
        .stage_runtime_effect_with_context(&mut context, savepoint_effect("integrity-final"))
        .unwrap();
    let result = runtime
        .program
        .interpreter()
        .state
        .borrow()
        .integrity_constraints
        .get(&hash_str("integrity-final!"))
        .unwrap()
        .result
        .clone();
    if let Value::Bool(value) = &*result.borrow() {
        *value.borrow_mut() = false;
    } else {
        panic!("constraint result must be bool");
    }

    let audit_count_before = runtime
        .list_events(None)
        .unwrap()
        .iter()
        .filter(|event| {
            matches!(
                event.kind,
                RuntimeEventKind::IntegrityConstraintViolated { .. }
            )
        })
        .count();
    let error = runtime
        .commit_runtime_transaction(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.active_transactions.contains_key(&transaction_id));
    let transaction = runtime
        .active_execution_transaction(transaction_id)
        .unwrap();
    assert_eq!(transaction.state, RuntimeExecutionTransactionState::Active);
    assert_eq!(transaction.effects.len(), 1);
    assert_eq!(
        runtime
            .list_events(None)
            .unwrap()
            .iter()
            .filter(|event| {
                matches!(
                    event.kind,
                    RuntimeEventKind::IntegrityConstraintViolated { .. }
                )
            })
            .count(),
        audit_count_before,
    );
    runtime
        .abort_runtime_transaction(&mut context, "discard final integrity candidate")
        .unwrap();
    assert_eq!(context.transaction, None);
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(
        runtime
            .program
            .root_symbol_value("integrity-final!")
            .is_err()
    );
}
