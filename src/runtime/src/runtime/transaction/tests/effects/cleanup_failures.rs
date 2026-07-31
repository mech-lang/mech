use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};

use mech_core::Value;

use crate::{
    ActorId, ActorRecord, BasicCapability, CapabilityId, CapabilityRequest, HostCall,
    InMemoryDocsProvider, InMemorySourceResolver, MechRuntime, ObjectId, ObjectRecord,
    PlannedPureHostFunction, PreparedRuntimeEffect, RuntimeCallContext, RuntimeCapabilityOperation,
    RuntimeHealth, RuntimeResourceWriteIntent, RuntimeResourceWriteRequest, RuntimeValueSnapshot,
    SharedCapabilityKernel, SourceRequest, TaskId, TaskRecord,
};

use super::{
    compensatable, effect, transactional, FailingEventIdGenerator, PanicEffectPhase,
    PanickingCompensatableEffect, PanickingTransactionalEffect,
};

#[test]
fn prepared_effect_abort_failure_poisons_runtime() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let mut first = transactional("first", log.clone());
    first.fail_abort = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(first)),
        )
        .unwrap();
    let mut second = transactional("second", log.clone());
    second.fail_prepare = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(second)),
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
    assert!(poison.original_error.contains("second prepare failed"));
    assert!(poison
        .rollback_failures
        .iter()
        .any(|failure| failure.contains("first abort failed")));
    assert_eq!(
        *log.lock().unwrap(),
        vec!["first:prepare", "second:prepare", "first:abort"],
    );

    assert!(runtime
        .abort_runtime_transaction(&mut context, "abort failure cleanup")
        .is_err());
    assert_eq!(context.transaction, None);
    assert!(!runtime.active_transactions.contains_key(&transaction_id));
}

#[test]
fn poisoned_runtime_owned_mutation_is_fail_closed() {
    let callback_calls = Arc::new(AtomicUsize::new(0));
    let observed_calls = callback_calls.clone();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .source_resolver(InMemorySourceResolver::new().with_string("retained-source", "x := 1"))
        .resource_provider(Box::new(InMemoryDocsProvider::new()))
        .host_function(PlannedPureHostFunction::new(
            "demo/poison-gate",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(RuntimeValueSnapshot::empty())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                observed_calls.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    let subject = runtime.runtime_context().unwrap().subject;
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(900),
            &subject,
            "host:demo/poison-gate",
            ["call"],
        )))
        .unwrap();
    runtime
        .grant_capability(Arc::new(BasicCapability::from_keys(
            CapabilityId(901),
            "task:1",
            "db://users",
            [":read"],
        )))
        .unwrap();
    let object_id = ObjectId(902);
    let actor_id = ActorId(903);
    let task_id = TaskId(904);
    runtime
        .put_object(ObjectRecord::text(object_id, "note", "before"))
        .unwrap();
    runtime
        .put_actor(ActorRecord::new(actor_id, "actor:poison"))
        .unwrap();
    runtime
        .put_task(TaskRecord::new(task_id, "task:poison"))
        .unwrap();

    let mut cleanup_context = runtime.runtime_context().unwrap();
    let cleanup_transaction = runtime.begin_transaction(&mut cleanup_context).unwrap();
    let mut poison_context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut poison_context).unwrap();
    let mut failing = transactional("poison-runtime", Arc::new(Mutex::new(Vec::new())));
    failing.fail_commit = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut poison_context,
            PreparedRuntimeEffect::Transactional(Box::new(failing)),
        )
        .unwrap();
    assert_eq!(
        runtime
            .commit_runtime_transaction(&mut poison_context)
            .unwrap_err()
            .kind_name(),
        "RuntimeExternalCommitIndeterminate",
    );
    assert!(runtime.is_poisoned());

    let mut poison_kinds = Vec::new();
    poison_kinds.push(
        runtime
            .call_host(HostCall::new("demo/poison-gate", Vec::new()))
            .unwrap_err()
            .kind_name(),
    );
    let used_steps_before = cleanup_context.budget.used_steps;
    let capability_uses_before = observed_kernel.successful_uses_for_test(CapabilityId(901));
    let overlay_uses_before = runtime
        .active_execution_transaction(cleanup_transaction)
        .unwrap()
        .capabilities
        .usage_deltas()
        .collect::<Vec<_>>();
    poison_kinds.push(
        runtime
            .check_capability_with_context(
                &mut cleanup_context,
                &CapabilityRequest::from_keys("task:1", ":read", "db://users"),
            )
            .unwrap_err()
            .kind_name(),
    );
    assert_eq!(cleanup_context.budget.used_steps, used_steps_before);
    assert_eq!(
        observed_kernel.successful_uses_for_test(CapabilityId(901)),
        capability_uses_before,
    );
    assert_eq!(
        runtime
            .active_execution_transaction(cleanup_transaction)
            .unwrap()
            .capabilities
            .usage_deltas()
            .collect::<Vec<_>>(),
        overlay_uses_before,
    );
    assert!(runtime
        .source_resolver()
        .resolve(&SourceRequest::new("retained-source"))
        .unwrap()
        .is_some());
    poison_kinds.push(
        runtime
            .set_source_resolver(InMemorySourceResolver::new())
            .unwrap_err()
            .kind_name(),
    );
    assert!(runtime
        .source_resolver()
        .resolve(&SourceRequest::new("retained-source"))
        .unwrap()
        .is_some());
    poison_kinds.push(
        runtime
            .grant_capability(Arc::new(BasicCapability::from_keys(
                CapabilityId(905),
                "task:1",
                "db://other",
                [":read"],
            )))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .revoke_capability(CapabilityId(901))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                "task:1",
                ":read",
                "db://users",
            ))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .write_resource(RuntimeResourceWriteRequest {
                base_uri: "docs://manual".to_string(),
                path: "poisoned".to_string(),
                context_name: "manual".to_string(),
                operation: RuntimeCapabilityOperation::Write,
                value: Value::String(mech_core::Ref::new("must-not-write".to_string())),
                intent: RuntimeResourceWriteIntent::Assign,
            })
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .update_object(ObjectRecord::text(object_id, "note", "after"))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .update_actor(ActorRecord::new(actor_id, "actor:changed"))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .update_task(TaskRecord::new(task_id, "task:changed"))
            .unwrap_err()
            .kind_name(),
    );
    poison_kinds.push(
        runtime
            .stage_runtime_effect_with_context(&mut cleanup_context, effect("must-not-stage"))
            .unwrap_err()
            .kind_name(),
    );

    assert!(poison_kinds.iter().all(|kind| *kind == "RuntimePoisoned"));
    assert_eq!(callback_calls.load(Ordering::SeqCst), 0);
    assert_eq!(
        runtime.get_object(object_id).unwrap().unwrap().data,
        b"before",
    );
    assert_eq!(
        runtime.get_actor(actor_id).unwrap().unwrap().subject,
        "actor:poison",
    );
    assert_eq!(
        runtime.get_task(task_id).unwrap().unwrap().subject,
        "task:poison",
    );
    assert!(!runtime
        .capability_kernel()
        .is_revoked(CapabilityId(901))
        .unwrap());

    runtime
        .abort_runtime_transaction(
            &mut cleanup_context,
            "poisoned runtime cleanup remains allowed",
        )
        .unwrap();
    assert_eq!(cleanup_context.transaction, None);
    assert!(!runtime
        .active_transactions
        .contains_key(&cleanup_transaction));
}

#[test]
fn prepare_failure_audit_failure_is_nonfatal() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
        .id_generator(FailingEventIdGenerator::new([5]))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional("first", log.clone()))),
        )
        .unwrap();
    let mut second = transactional("second", log);
    second.fail_prepare = true;
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(second)),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "SyntheticEffectError");
    assert!(error.full_chain_message().contains("second prepare failed"));
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    runtime
        .abort_runtime_transaction(&mut context, "prepare audit cleanup")
        .unwrap();
}

#[test]
fn compensation_audit_failure_is_nonfatal() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder()
        .id_generator(FailingEventIdGenerator::new([5]))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(compensatable("first", log.clone()))),
        )
        .unwrap();
    let mut second = compensatable("second", log);
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
    assert!(error.full_chain_message().contains("second apply failed"));
    assert!(!runtime.is_poisoned());
    assert_eq!(context.transaction, Some(transaction_id));
    runtime
        .abort_runtime_transaction(&mut context, "compensation audit cleanup")
        .unwrap();
}

#[test]
fn explicit_abort_audit_failure_is_reported_and_poisons() {
    let mut runtime = MechRuntime::builder()
        .id_generator(FailingEventIdGenerator::new([5]))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Transactional(Box::new(transactional(
                "abortable",
                Arc::new(Mutex::new(Vec::new())),
            ))),
        )
        .unwrap();

    let error = runtime
        .abort_runtime_transaction(&mut context, "audit failure abort")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert!(error
        .full_chain_message()
        .contains("event publication failed"));
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
}

#[test]
fn transactional_abort_panic_continues_cleanup_and_poisons_afterward() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    for (name, panic_at) in [("first", None), ("second", Some(PanicEffectPhase::Abort))] {
        runtime
            .stage_runtime_effect_with_context(
                &mut context,
                PreparedRuntimeEffect::Transactional(Box::new(PanickingTransactionalEffect {
                    name,
                    panic_at,
                    log: log.clone(),
                })),
            )
            .unwrap();
    }

    let error = runtime
        .abort_runtime_transaction(&mut context, "panic cleanup")
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeProgramRollbackFailed");
    assert_eq!(*log.lock().unwrap(), vec!["second:abort", "first:abort"],);
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(runtime.is_poisoned());
    assert_eq!(context.transaction, None);
}

#[test]
fn compensatable_cleanup_panic_poisons_after_store_failure() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .stage_runtime_effect_with_context(
            &mut context,
            PreparedRuntimeEffect::Compensatable(Box::new(PanickingCompensatableEffect {
                name: "compensate",
                panic_at: Some(PanicEffectPhase::Compensate),
                log: log.clone(),
            })),
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(777), "missing", "update"),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert_eq!(
        *log.lock().unwrap(),
        vec!["compensate:apply", "compensate:compensate"],
    );
    assert!(runtime.active_effect_phase.get().is_none());
    assert!(runtime.is_poisoned());
}
