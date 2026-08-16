use mech_core::{GenericError, MResult, MechError};

use crate::{CapabilityId, MechRuntime, SequentialIdGenerator};

use super::super::RuntimeCapabilityOverlay;
use super::{capability, request};

#[test]
fn failed_capability_rebuild_restores_previous_valid_overlay() {
    let id = CapabilityId(100);
    let mut overlay = RuntimeCapabilityOverlay::default();
    overlay.stage_use(id).unwrap();
    overlay.stage_revocation(id).unwrap();
    let mark = overlay.mark();

    let error = overlay.stage_use(id).unwrap_err();

    assert_eq!(error.kind_name(), "RuntimeInvalidOperation");
    assert_eq!(overlay.mark(), mark);
    assert_eq!(overlay.usage_deltas().collect::<Vec<_>>(), vec![(id, 1)],);
    assert_eq!(overlay.revocations().collect::<Vec<_>>(), vec![id]);

    let overflow = RuntimeCapabilityOverlay::incremented_usage(id, u64::MAX).unwrap_err();
    assert_eq!(overflow.kind_name(), "RuntimeInvalidOperation");
}

#[test]
fn failed_retained_operation_truncates_capability_overlay() {
    let mut runtime = MechRuntime::builder()
        .id_generator(SequentialIdGenerator::starting_at(1))
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    let id = CapabilityId(100);

    let result: MResult<()> = runtime.with_atomic_module_operation(
        &mut context,
        "test_capability_overlay",
        |runtime, context| {
            runtime.grant_capability_with_context(context, capability(id, "task:1", true))?;
            Err(MechError::new(
                GenericError {
                    msg: "deliberate operation failure".to_string(),
                },
                None,
            ))
        },
    );

    assert_eq!(result.unwrap_err().kind_name(), "GenericError");
    assert!(
        runtime
            .active_runtime_transaction(transaction_id)
            .unwrap()
            .capabilities
            .is_empty()
    );
    assert!(
        runtime
            .check_capability_with_context(&mut context, &request("task:1"))
            .is_err()
    );
    assert!(runtime.get_capability(id).unwrap().is_none());
    runtime
        .abort_runtime_transaction(&mut context, "test cleanup")
        .unwrap();
}

#[test]
fn capability_overlay_commits_kernel_and_store_together() {
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
    runtime.commit_runtime_transaction(&mut context).unwrap();

    assert!(runtime.get_capability(id).unwrap().is_some());
    assert!(runtime.capability_kernel().get(id).unwrap().is_some());
    assert_eq!(runtime.check_capability(&request("task:1")).unwrap(), id);
}
