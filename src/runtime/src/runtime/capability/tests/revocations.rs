use std::sync::Arc;

use crate::{BasicCapability, CapabilityId, CapabilityRequest, MechRuntime, SequentialIdGenerator};

use super::super::RuntimeCapabilityOverlay;
use super::{capability, limited_capability, request};

#[test]
fn provisional_grant_use_revoke_cancels_all_authority_work() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();

    overlay.stage_grant(limited_capability(id, 2)).unwrap();
    overlay.stage_use(id).unwrap();
    overlay.stage_revocation(id).unwrap();

    assert!(overlay.is_empty());
    assert!(overlay.grants().next().is_none());
    assert!(overlay.revocations().next().is_none());
    assert!(overlay.usage_deltas().next().is_none());
}

#[test]
fn live_use_revoke_and_later_provisional_regrant_are_ordered() {
    let id = CapabilityId(100);
    let mut live_overlay = RuntimeCapabilityOverlay::default();
    live_overlay.stage_use(id).unwrap();
    live_overlay.stage_revocation(id).unwrap();
    assert_eq!(
        live_overlay.usage_deltas().collect::<Vec<_>>(),
        vec![(id, 1)],
    );
    assert_eq!(live_overlay.revocations().collect::<Vec<_>>(), vec![id],);

    let mut provisional_overlay = RuntimeCapabilityOverlay::default();
    provisional_overlay
        .stage_grant(limited_capability(id, 2))
        .unwrap();
    provisional_overlay.stage_use(id).unwrap();
    provisional_overlay.stage_revocation(id).unwrap();
    provisional_overlay
        .stage_grant(limited_capability(id, 2))
        .unwrap();
    provisional_overlay.stage_use(id).unwrap();
    assert_eq!(
        provisional_overlay.usage_deltas().collect::<Vec<_>>(),
        vec![(id, 1)],
    );
}

#[test]
fn provisional_revocation_does_not_leak_to_other_transactions() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut administrative = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime
        .grant_capability_with_context(&mut administrative, capability(id, "task:1", true))
        .unwrap();

    let mut owner = runtime.runtime_context().unwrap();
    let mut observer = runtime.runtime_context().unwrap();
    runtime.begin_transaction(&mut owner).unwrap();
    runtime
        .revoke_capability_with_context(&mut owner, id)
        .unwrap();

    assert!(runtime
        .check_capability_with_context(&mut owner, &request("task:1"))
        .is_err());
    assert_eq!(
        runtime
            .check_capability_with_context(&mut observer, &request("task:1"))
            .unwrap(),
        id,
    );
    assert!(!runtime.capability_kernel().is_revoked(id).unwrap());

    runtime
        .abort_runtime_transaction(&mut owner, "test abort")
        .unwrap();
    assert_eq!(
        runtime
            .check_capability_with_context(&mut owner, &request("task:1"))
            .unwrap(),
        id,
    );
}

#[test]
fn provisional_grant_then_revoke_cancels_commit_work() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);

    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, capability(id, "task:1", true))
        .unwrap();
    runtime
        .revoke_capability_with_context(&mut context, id)
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.get_capability(id).unwrap().is_none());
    assert!(runtime.capability_kernel().get(id).unwrap().is_none());
}

#[test]
fn regrant_after_cancellation_commits_latest_capability() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let id = CapabilityId(100);
    runtime.begin_transaction(&mut context).unwrap();
    runtime
        .grant_capability_with_context(&mut context, capability(id, "task:1", true))
        .unwrap();
    runtime
        .revoke_capability_with_context(&mut context, id)
        .unwrap();
    runtime
        .grant_capability_with_context(
            &mut context,
            Arc::new(BasicCapability::from_keys(
                id,
                "task:1",
                "db://projects",
                [":read"],
            )),
        )
        .unwrap();
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.check_capability(&request("task:1")).is_err());
    assert_eq!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                "task:1",
                ":read",
                "db://projects",
            ))
            .unwrap(),
        id,
    );
}
