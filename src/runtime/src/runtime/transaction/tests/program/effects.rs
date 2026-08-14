use super::support::{invoke_host_callback, savepoint_effect};
use crate::runtime::gate_a_probe::{gate_a_cost_snapshot, reset_gate_a_costs};
use crate::runtime::test_support::capabilities::grant_host_call;
use crate::runtime::test_support::providers::test_runtime_builder;
use crate::{
    CapabilityId, PlannedStagedHostFunction, RuntimePreparedHostCall, RuntimeValueSnapshot,
};

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
