use super::super::{
    CapabilityId, CapabilityRequest, MResult, MechError, MechRuntime, ObjectId, ObjectRecord,
    RuntimeEventKind,
};
use super::{ReactiveTransactionalProbe, add_test_function};
use crate::capability::{
    BasicCapability, BasicConstraints, BasicOperation, BasicResource, BasicSubject, Capability,
    CapabilityKernel, SharedCapabilityKernel,
};
use crate::{
    PreparedRuntimeEffect, RuntimeAfterCommitEffect, RuntimeEffectMetadata, RuntimeEffectSource,
};
use mech_core::GenericError;
use std::sync::{Arc, Mutex};

#[derive(Debug)]
struct ReactiveAfterCommitFailure;

impl RuntimeAfterCommitEffect for ReactiveAfterCommitFailure {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::Custom {
                name: "reactive-after-commit-failure".to_string(),
            },
            "reactive-after-commit-failure",
        )
    }

    fn deliver(&mut self) -> MResult<()> {
        Err(MechError::new(
            GenericError {
                msg: "deliberate reactive delivery failure".to_string(),
            },
            None,
        ))
    }
}

fn limited_live_capability(id: CapabilityId, subject: &str, max_uses: u64) -> Arc<dyn Capability> {
    Arc::new(
        BasicCapability::new(
            id,
            &BasicSubject::new(subject),
            &BasicResource::new("db://reactive"),
            [BasicOperation::read()],
        )
        .with_constraints(BasicConstraints::default().with_max_uses(max_uses)),
    )
}

fn reactive_capability_request(subject: &str) -> CapabilityRequest {
    CapabilityRequest::new(
        &BasicSubject::new(subject),
        &BasicOperation::read(),
        &BasicResource::new("db://reactive"),
    )
}

#[test]
fn implicit_reactive_capability_use_commits_or_rolls_back_once() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let id = CapabilityId(700);
    runtime
        .grant_capability_with_context(
            &mut administrative,
            limited_live_capability(id, &subject, 2),
        )
        .unwrap();
    let request = reactive_capability_request(&subject);

    let mut failed_context = runtime.runtime_context().unwrap();
    let failed: MResult<()> = runtime.with_atomic_reactive_turn_for_test(
        &mut failed_context,
        "failed_capability_turn",
        |runtime, context| {
            runtime.check_capability_with_context(context, &request)?;
            Err(MechError::new(
                GenericError {
                    msg: "deliberate failed capability turn".to_string(),
                },
                None,
            ))
        },
    );
    assert_eq!(failed.unwrap_err().kind_name(), "GenericError");
    assert_eq!(observed.successful_uses_for_test(id), 0);

    let mut successful_context = runtime.runtime_context().unwrap();
    runtime
        .with_atomic_reactive_turn_for_test(
            &mut successful_context,
            "successful_capability_turn",
            |runtime, context| {
                runtime.check_capability_with_context(context, &request)?;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);
}

#[test]
fn explicit_reactive_capability_reservations_commit_or_abort() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let id = CapabilityId(701);
    runtime
        .grant_capability_with_context(
            &mut administrative,
            limited_live_capability(id, &subject, 3),
        )
        .unwrap();
    let request = reactive_capability_request(&subject);

    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "explicit_capability_turn",
            |runtime, context| {
                runtime.check_capability_with_context(context, &request)?;
                Ok(())
            },
        )
        .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 0);
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .capabilities
            .usage_deltas()
            .collect::<Vec<_>>(),
        vec![(id, 1)],
    );

    let failed: MResult<()> = runtime.with_atomic_reactive_turn_for_test(
        &mut context,
        "failed_later_capability_turn",
        |runtime, context| {
            runtime.check_capability_with_context(context, &request)?;
            Err(MechError::new(
                GenericError {
                    msg: "deliberate later capability failure".to_string(),
                },
                None,
            ))
        },
    );
    assert_eq!(failed.unwrap_err().kind_name(), "GenericError");
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .capabilities
            .usage_deltas()
            .collect::<Vec<_>>(),
        vec![(id, 1)],
    );
    assert_eq!(observed.successful_uses_for_test(id), 0);

    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);

    let mut abort_context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut abort_context).unwrap();
    runtime
        .with_atomic_reactive_turn_for_test(
            &mut abort_context,
            "aborted_capability_turn",
            |runtime, context| {
                runtime.check_capability_with_context(context, &request)?;
                Ok(())
            },
        )
        .unwrap();
    runtime
        .abort_runtime_transaction(&mut abort_context, "discard reservation")
        .unwrap();
    assert_eq!(observed.successful_uses_for_test(id), 1);
}

#[test]
fn retryable_store_failure_commits_reserved_use_without_rerun() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let (_, calls) = add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let capability_id = CapabilityId(702);
    runtime
        .grant_capability_with_context(
            &mut administrative,
            limited_live_capability(capability_id, &subject, 1),
        )
        .unwrap();
    let request = reactive_capability_request(&subject);
    let missing_object = ObjectId(703);
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "retryable_capability_turn",
            |runtime, context| {
                runtime.check_capability_with_context(context, &request)?;
                Ok(())
            },
        )
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(missing_object, "note", "staged update"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert_eq!(*calls.borrow(), 1);
    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert_eq!(
        runtime
            .active_execution_transaction(transaction_id)
            .unwrap()
            .capabilities
            .usage_deltas()
            .collect::<Vec<_>>(),
        vec![(capability_id, 1)],
    );

    runtime
        .put_object(ObjectRecord::text(
            missing_object,
            "note",
            "durable baseline",
        ))
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert_eq!(*calls.borrow(), 1);
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
    assert_eq!(
        runtime.get_object(missing_object).unwrap().unwrap().data,
        b"staged update".to_vec(),
    );
}

#[test]
fn provisional_capability_grant_and_use_commit_together() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let subject = context.subject.clone();
    let capability_id = CapabilityId(704);
    let request = reactive_capability_request(&subject);
    runtime.begin_transaction(&mut context).unwrap();

    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "provisional_grant_and_use",
            |runtime, context| {
                runtime.grant_capability_with_context(
                    context,
                    limited_live_capability(capability_id, &subject, 1),
                )?;
                runtime.check_capability_with_context(context, &request)?;
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert!(observed.get(capability_id).unwrap().is_none());
    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert!(observed.get(capability_id).unwrap().is_some());
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
}

#[test]
fn live_capability_use_commits_before_transactional_revocation() {
    let kernel = SharedCapabilityKernel::new();
    let observed = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    add_test_function(&mut runtime, None);
    let mut administrative = runtime.runtime_context().unwrap();
    let subject = administrative.subject.clone();
    let capability_id = CapabilityId(705);
    runtime
        .grant_capability_with_context(
            &mut administrative,
            limited_live_capability(capability_id, &subject, 1),
        )
        .unwrap();
    let request = reactive_capability_request(&subject);
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();

    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "live_use_then_revoke",
            |runtime, context| {
                runtime.check_capability_with_context(context, &request)?;
                runtime.revoke_capability_with_context(context, capability_id)?;
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(observed.successful_uses_for_test(capability_id), 0);
    assert!(!observed.is_revoked(capability_id).unwrap());
    runtime.commit_runtime_transaction(&mut context).unwrap();
    assert_eq!(observed.successful_uses_for_test(capability_id), 1);
    assert!(observed.is_revoked(capability_id).unwrap());
}

#[test]
fn post_store_participant_failure_never_rolls_back_reactive_state() {
    let log = Arc::new(Mutex::new(Vec::new()));
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();
    let object_id = ObjectId(931);

    let error = runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "reactive_commit_failure",
            |runtime, context| {
                runtime.put_object_with_context(
                    context,
                    ObjectRecord::text(object_id, "note", "must remain committed"),
                )?;
                runtime.stage_runtime_effect_with_context(
                    context,
                    PreparedRuntimeEffect::Transactional(Box::new(ReactiveTransactionalProbe {
                        log: log.clone(),
                        fail_prepare: false,
                        fail_commit: true,
                        fail_abort: false,
                    })),
                )?;
                Ok(())
            },
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExternalCommitIndeterminate");
    assert_eq!(*output.borrow(), 1);
    assert!(runtime.get_object(object_id).unwrap().is_some());
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    assert!(runtime.is_poisoned());
    assert_eq!(*log.lock().unwrap(), vec!["prepare", "commit"]);
}

#[test]
fn after_commit_delivery_failure_keeps_reactive_state_and_health() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let (output, _) = add_test_function(&mut runtime, None);
    let mut context = runtime.runtime_context().unwrap();

    runtime
        .with_atomic_reactive_turn_for_test(
            &mut context,
            "reactive_delivery_failure",
            |runtime, context| {
                runtime.stage_runtime_effect_with_context(
                    context,
                    PreparedRuntimeEffect::AfterCommit(Box::new(ReactiveAfterCommitFailure)),
                )?;
                Ok(())
            },
        )
        .unwrap();

    assert_eq!(*output.borrow(), 1);
    assert!(!runtime.is_poisoned());
    assert!(runtime.active_transactions.is_empty());
    assert!(runtime.list_events(None).unwrap().iter().any(|event| {
        matches!(
          &event.kind,
          RuntimeEventKind::EffectDeliveryFailed { message, .. }
            if message.contains("deliberate reactive delivery failure")
        )
    }));
}
