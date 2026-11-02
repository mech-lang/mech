use crate::{
    CapabilityId, MechRuntime, ObjectId, ObjectRecord, RuntimeHealth, SequentialIdGenerator,
};

use super::{
    CapabilityPanicPhase, FailingCheckpointRestoreKernel, PanickingCapabilityKernel, capability,
    limited_capability, request,
};

#[test]
fn store_commit_failure_restores_capability_kernel_checkpoint() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, capability(id, "task:1", true))
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(500), "note", "missing"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
    assert!(runtime.get_capability(id).unwrap().is_none());

    runtime
        .abort_runtime_transaction(&mut context, "test cleanup")
        .unwrap();
}

#[test]
fn capability_checkpoint_restore_failure_poisons_runtime() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .capability_kernel(FailingCheckpointRestoreKernel::default())
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, capability(id, "task:1", true))
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(500), "note", "missing"),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    let RuntimeHealth::Poisoned(poison) = &runtime.health else {
        panic!("runtime must retain capability cleanup failure");
    };
    assert!(
        poison.rollback_failures.iter().any(|failure| {
            failure.contains("deliberate capability checkpoint restore failure")
        })
    );
}

#[test]
fn capability_preview_panic_is_an_ordinary_transaction_failure() {
    let id = CapabilityId(100);
    let kernel = PanickingCapabilityKernel::with_grant(
        CapabilityPanicPhase::Preview,
        limited_capability(id, 2),
    );
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let error = runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate capability preview panic"));
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(!runtime.is_poisoned());
    runtime
        .abort_runtime_transaction(&mut context, "preview panic cleanup")
        .unwrap();
}

#[test]
fn capability_check_panic_is_converted_without_poisoning() {
    let id = CapabilityId(100);
    let kernel = PanickingCapabilityKernel::with_grant(
        CapabilityPanicPhase::Check,
        limited_capability(id, 2),
    );
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();

    let error = runtime.check_capability(&request("task:1")).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate capability check panic"));
    assert!(!runtime.is_poisoned());
    runtime.list_events(None).unwrap();
}

#[test]
fn capability_apply_panic_rolls_back_before_store_commit() {
    let id = CapabilityId(100);
    let kernel = PanickingCapabilityKernel::with_grant(
        CapabilityPanicPhase::Apply,
        limited_capability(id, 2),
    );
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeExtensionPanicked");
    assert!(format!("{error:?}").contains("deliberate capability apply panic"));
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(!runtime.is_poisoned());
    runtime
        .abort_runtime_transaction(&mut context, "apply panic cleanup")
        .unwrap();
}

#[test]
fn capability_restore_panic_poisons_after_store_failure() {
    let id = CapabilityId(100);
    let kernel = PanickingCapabilityKernel::with_grant(
        CapabilityPanicPhase::Restore,
        limited_capability(id, 2),
    );
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(500), "note", "missing"),
        )
        .unwrap();

    let error = runtime
        .commit_runtime_transaction_detailed(&mut context)
        .unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeEffectCleanupFailed");
    assert!(runtime.is_poisoned());
    let RuntimeHealth::Poisoned(poison) = &runtime.health else {
        panic!("runtime must retain capability restore panic");
    };
    assert!(
        poison
            .rollback_failures
            .iter()
            .any(|failure| { failure.contains("deliberate capability restore panic") })
    );
}
