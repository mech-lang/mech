use std::collections::HashMap;
use std::sync::Arc;

use crate::{
    BasicCapability, BasicConstraints, Capability, CapabilityId, CapabilityKernel, MechRuntime,
    ObjectId, ObjectRecord, SharedCapabilityKernel,
};

use super::super::RuntimeCapabilityOverlay;
use super::{limited_capability, request};

#[test]
fn capability_use_journal_savepoints_restore_only_later_uses() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();
    let empty_mark = overlay.mark();

    overlay.stage_use(id).unwrap();
    assert!(!overlay.is_empty());
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 1)]);

    let later_mark = overlay.mark();
    overlay.stage_use(id).unwrap();
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 2)]);

    overlay.rollback_to(later_mark).unwrap();
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 1)]);

    overlay.rollback_to(empty_mark).unwrap();
    assert!(overlay.is_empty());
    assert!(overlay.usage_deltas().next().is_none());
}

#[test]
fn capability_use_journal_preserves_live_usage_order() {
    let first = CapabilityId(100);
    let second = CapabilityId(101);
    let mut overlay = RuntimeCapabilityOverlay::default();

    overlay.stage_use(second).unwrap();
    overlay.stage_use(first).unwrap();
    overlay.stage_use(second).unwrap();

    assert_eq!(
        overlay.usage_deltas().collect::<Vec<_>>(),
        vec![(second, 2), (first, 1)],
    );
    assert_eq!(
        overlay.pending_uses(),
        &HashMap::from([(first, 1), (second, 2)]),
    );
}

#[test]
fn provisional_capability_usage_is_committed_to_live_kernel() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, limited_capability(id, 1))
        .unwrap();
    assert_eq!(
        runtime
            .check_capability_with_context(&mut context, &request("task:1"))
            .unwrap(),
        id,
    );
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.check_capability(&request("task:1")).is_err());
}

#[test]
fn store_failure_restores_provisional_usage_delta() {
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let mut runtime = MechRuntime::builder()
        .capability_kernel(kernel)
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, limited_capability(id, 2))
        .unwrap();
    runtime
        .check_capability_with_context(&mut context, &request("task:1"))
        .unwrap();
    runtime
        .update_object_with_context(
            &mut context,
            ObjectRecord::text(ObjectId(500), "note", "missing"),
        )
        .unwrap();

    assert!(runtime.commit_runtime_transaction(&mut context).is_err());
    assert_eq!(context.transaction, Some(transaction_id));
    assert!(observed_kernel.get(id).unwrap().is_none());
    assert_eq!(observed_kernel.successful_uses_for_test(id), 0);
    assert_eq!(
        runtime
            .active_runtime_transaction(transaction_id)
            .unwrap()
            .capabilities
            .usage_deltas()
            .collect::<Vec<_>>(),
        vec![(id, 1)],
    );

    runtime
        .abort_runtime_transaction(&mut context, "usage restore cleanup")
        .unwrap();
}

#[test]
fn provisional_capability_enforces_use_limit() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let limited: Arc<dyn Capability> = Arc::new(
        BasicCapability::from_keys(id, "task:1", "db://users", [":read"]).with_constraints(
            BasicConstraints {
                max_uses: Some(1),
                ..BasicConstraints::default()
            },
        ),
    );

    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, limited)
        .unwrap();
    assert_eq!(
        runtime
            .check_capability_with_context(&mut context, &request("task:1"))
            .unwrap(),
        id,
    );
    assert!(
        runtime
            .check_capability_with_context(&mut context, &request("task:1"))
            .is_err()
    );
}

#[test]
fn provisional_revocation_does_not_consume_live_use_limit() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut administrative = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    let limited: Arc<dyn Capability> = Arc::new(
        BasicCapability::from_keys(id, "task:1", "db://users", [":read"]).with_constraints(
            BasicConstraints {
                max_uses: Some(1),
                ..BasicConstraints::default()
            },
        ),
    );
    runtime
        .grant_capability_with_context(&mut administrative, limited)
        .unwrap();

    let mut owner = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();
    runtime
        .revoke_capability_with_context(&mut owner, id)
        .unwrap();
    assert!(
        runtime
            .check_capability_with_context(&mut owner, &request("task:1"))
            .is_err()
    );
    runtime
        .abort_runtime_transaction(&mut owner, "test abort")
        .unwrap();

    assert_eq!(runtime.check_capability(&request("task:1")).unwrap(), id,);
}
