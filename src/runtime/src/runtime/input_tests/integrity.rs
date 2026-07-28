use std::sync::{
    Arc, Mutex,
    atomic::{AtomicUsize, Ordering},
};

use mech_core::{MResult, Ref, Value};

use super::super::{RuntimeBuilder, RuntimeHealth};
use crate::runtime::test_support::{
    capabilities::{CapabilityUseProbe, grant_host_call_with_limit, grant_read, grant_write},
    events::{assert_event_before, events_since},
    providers::{TestResourceProvider, test_provider_with, test_runtime_with_output},
    stores::StoreCommitProbe,
    values::{bool_value, f64_value, host_f64_argument, source_value, symbol_value},
};
use crate::{
    CapabilityId, PlannedStagedHostFunction, PreparedRuntimeEffect, RuntimeCallContext,
    RuntimeEffectMetadata, RuntimeEffectSource, RuntimeEventKind, RuntimeHostInput,
    RuntimeHostInputSource, RuntimeHostInputValue, RuntimePreparedHostCall,
    RuntimeTransactionalEffect, RuntimeValueSnapshot, SharedCapabilityKernel,
};

const TEST_CLOCK_BASE_URI: &str = "test://clock/ticks";
const TEST_OUTPUT_BASE_URI: &str = "test://effects/output";

#[derive(Debug, Default)]
struct ReceiverCounters {
    stage_count: AtomicUsize,
    prepare_count: AtomicUsize,
    commit_count: AtomicUsize,
    abort_count: AtomicUsize,
    delivery_count: AtomicUsize,
    staged_payloads: Mutex<Vec<f64>>,
    prepared_payloads: Mutex<Vec<f64>>,
    committed_payloads: Mutex<Vec<f64>>,
    aborted_payloads: Mutex<Vec<f64>>,
    delivered_payloads: Mutex<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
struct ReceiverSnapshot {
    stage_count: usize,
    prepare_count: usize,
    commit_count: usize,
    abort_count: usize,
    delivery_count: usize,
    staged_payloads: Vec<f64>,
    prepared_payloads: Vec<f64>,
    committed_payloads: Vec<f64>,
    aborted_payloads: Vec<f64>,
    delivered_payloads: Vec<f64>,
}

fn receiver_snapshot(counters: &ReceiverCounters) -> ReceiverSnapshot {
    ReceiverSnapshot {
        stage_count: counters.stage_count.load(Ordering::SeqCst),
        prepare_count: counters.prepare_count.load(Ordering::SeqCst),
        commit_count: counters.commit_count.load(Ordering::SeqCst),
        abort_count: counters.abort_count.load(Ordering::SeqCst),
        delivery_count: counters.delivery_count.load(Ordering::SeqCst),
        staged_payloads: counters.staged_payloads.lock().unwrap().clone(),
        prepared_payloads: counters.prepared_payloads.lock().unwrap().clone(),
        committed_payloads: counters.committed_payloads.lock().unwrap().clone(),
        aborted_payloads: counters.aborted_payloads.lock().unwrap().clone(),
        delivered_payloads: counters.delivered_payloads.lock().unwrap().clone(),
    }
}

fn acceptance_payloads(payloads: &[f64]) -> String {
    format!(
        "[{}]",
        payloads
            .iter()
            .map(|payload| payload.to_string())
            .collect::<Vec<_>>()
            .join(", "),
    )
}

#[derive(Debug)]
struct ReceiverTransactionalEffect {
    payload: f64,
    counters: Arc<ReceiverCounters>,
}

impl RuntimeTransactionalEffect for ReceiverTransactionalEffect {
    fn metadata(&self) -> RuntimeEffectMetadata {
        RuntimeEffectMetadata::new(
            RuntimeEffectSource::HostFunction {
                name: "test/receiver-send".to_string(),
            },
            "receiver-send",
        )
        .with_resource("test://receiver/delivery")
    }

    fn prepare(&mut self) -> MResult<()> {
        self.counters.prepare_count.fetch_add(1, Ordering::SeqCst);
        self.counters
            .prepared_payloads
            .lock()
            .unwrap()
            .push(self.payload);
        Ok(())
    }

    fn commit(&mut self) -> MResult<()> {
        self.counters.commit_count.fetch_add(1, Ordering::SeqCst);
        self.counters
            .committed_payloads
            .lock()
            .unwrap()
            .push(self.payload);
        self.counters.delivery_count.fetch_add(1, Ordering::SeqCst);
        self.counters
            .delivered_payloads
            .lock()
            .unwrap()
            .push(self.payload);
        Ok(())
    }

    fn abort(&mut self) -> MResult<()> {
        self.counters.abort_count.fetch_add(1, Ordering::SeqCst);
        self.counters
            .aborted_payloads
            .lock()
            .unwrap()
            .push(self.payload);
        Ok(())
    }
}

#[test]
fn integrity_invalid_host_input_is_restored_before_output_audit_and_recovery() {
    let provider = test_provider_with(TEST_CLOCK_BASE_URI, "value", 90.0);
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
    .run_string(
      "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\ntarget := @pulse/value\nmaximum-target := 120.0\ncontroller-command := target\nsafe-target! := target <= maximum-target\n@out/line <- controller-command",
    )
    .unwrap();
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    let initial_deliveries = output.lines().len();

    runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(100.0),
        ))
        .unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "target")), 100.0);
    assert_eq!(output.lines().len(), initial_deliveries + 1);
    let deliveries_before_invalid = output.lines().len();
    let events_before_invalid = runtime.list_events(None).unwrap().len();

    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(150.0),
        ))
        .unwrap_err();

    assert_eq!(error.kind_name(), "IntegrityConstraintViolationSet");
    let failures = error
        .kind_as::<mech_program::IntegrityConstraintViolationSet>()
        .unwrap();
    assert_eq!(failures.violations.len(), 1);
    assert_eq!(failures.violations[0].actual.as_deref(), Some("150"));
    assert_eq!(failures.violations[0].expected.as_deref(), Some("120"));
    assert_eq!(f64_value(&source_value(&runtime, &source)), 100.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "target")), 100.0);
    assert_eq!(
        f64_value(&symbol_value(&runtime, "controller-command")),
        100.0,
    );
    assert!(bool_value(&symbol_value(&runtime, "safe-target!")));
    assert_eq!(output.lines().len(), deliveries_before_invalid);
    assert!(runtime.active_transactions.is_empty());
    assert!(!runtime.is_poisoned());
    let events = runtime.list_events(None).unwrap();
    let audit = events[events_before_invalid..]
        .iter()
        .find_map(|event| match &event.kind {
            RuntimeEventKind::IntegrityConstraintViolated { violations, .. } => violations.first(),
            _ => None,
        })
        .expect("invalid candidate emits a detached integrity audit");
    assert_eq!(audit.actual.as_deref(), Some("150"));
    assert_eq!(audit.expected.as_deref(), Some("120"));

    runtime
        .apply_host_input(RuntimeHostInput::single(
            source,
            RuntimeHostInputValue::F64(110.0),
        ))
        .unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "target")), 110.0);
    assert_eq!(output.lines().len(), deliveries_before_invalid + 1);
    assert!(!runtime.is_poisoned());
}

#[test]
fn integrity_invalid_host_input_aborts_staged_receiver_before_commit() {
    let receiver = Arc::new(ReceiverCounters::default());
    let receiver_for_stage = receiver.clone();
    let receiver_host = PlannedStagedHostFunction::new(
        "test/receiver-send",
        |_context: &RuntimeCallContext, arguments: &[RuntimeValueSnapshot]| {
            let payload = host_f64_argument(&arguments[0]);
            Ok(Value::F64(Ref::new(payload)).into())
        },
        move |_context: &RuntimeCallContext, arguments: Vec<RuntimeValueSnapshot>| {
            let payload = host_f64_argument(&arguments[0]);
            receiver_for_stage
                .stage_count
                .fetch_add(1, Ordering::SeqCst);
            receiver_for_stage
                .staged_payloads
                .lock()
                .unwrap()
                .push(payload);
            Ok(RuntimePreparedHostCall {
                value: Value::F64(Ref::new(payload)).into(),
                effect: PreparedRuntimeEffect::Transactional(Box::new(
                    ReceiverTransactionalEffect {
                        payload,
                        counters: receiver_for_stage.clone(),
                    },
                )),
            })
        },
    );
    let (store, store_commit_probe) = StoreCommitProbe::new();
    let kernel = SharedCapabilityKernel::new();
    let observed_kernel = kernel.clone();
    let provider = TestResourceProvider::new().with_value(
        TEST_CLOCK_BASE_URI,
        "value",
        Value::F64(Ref::new(90.0)),
    );
    let mut runtime = RuntimeBuilder::new()
        .store(store)
        .capability_kernel(kernel)
        .resource_provider(Box::new(provider))
        .host_function(receiver_host)
        .unwrap()
        .build()
        .unwrap();
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    let receiver_capability_id = CapabilityId(970);
    grant_host_call_with_limit(
        &mut runtime,
        receiver_capability_id,
        "test/receiver-send",
        8,
    );
    let capability_use_probe = CapabilityUseProbe::new(observed_kernel, receiver_capability_id);
    runtime
        .run_string(
            r#"@pulse := test://clock/ticks{:read(value)}

target := @pulse/value
maximum-target := 120.0

receiver-result := test/receiver-send(target)

safe-target! := receiver-result <= maximum-target
"#,
        )
        .unwrap();
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    assert_eq!(f64_value(&source_value(&runtime, &source)), 90.0);
    let baseline_target = f64_value(&symbol_value(&runtime, "target"));
    assert_eq!(baseline_target, 90.0);
    let maximum_target = f64_value(&symbol_value(&runtime, "maximum-target"));
    assert_eq!(maximum_target, 120.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "receiver-result")), 90.0);
    assert!(bool_value(&symbol_value(&runtime, "safe-target!")));
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);

    eprintln!("=== TRANSACTIONAL INTEGRITY ACCEPTANCE TEST ===");
    eprintln!();
    eprintln!("baseline");
    eprintln!("target={baseline_target}");
    eprintln!("maximum={maximum_target}");

    let installed_receiver = receiver_snapshot(&receiver);
    let installed_store_commits = store_commit_probe.calls();
    let installed_capability_uses = capability_use_probe.committed_uses();
    let installed_events = runtime.list_events(None).unwrap().len();

    let before_100 = receiver_snapshot(&receiver);
    let store_before_100 = store_commit_probe.calls();
    let capability_before_100 = capability_use_probe.committed_uses();
    let events_before_100 = runtime.list_events(None).unwrap().len();
    runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(100.0),
        ))
        .unwrap();
    assert_eq!(f64_value(&source_value(&runtime, &source)), 100.0);
    let live_target = f64_value(&symbol_value(&runtime, "target"));
    assert_eq!(live_target, 100.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "receiver-result")), 100.0);
    assert!(bool_value(&symbol_value(&runtime, "safe-target!")));
    let after_100 = receiver_snapshot(&receiver);
    let staged = &after_100.staged_payloads[before_100.staged_payloads.len()..];
    let prepared = &after_100.prepared_payloads[before_100.prepared_payloads.len()..];
    let committed = &after_100.committed_payloads[before_100.committed_payloads.len()..];
    let aborted = &after_100.aborted_payloads[before_100.aborted_payloads.len()..];
    let delivered = &after_100.delivered_payloads[before_100.delivered_payloads.len()..];
    let store_commits = store_commit_probe.calls() - store_before_100;
    let capability_uses = capability_use_probe.committed_uses() - capability_before_100;
    assert_eq!(after_100.stage_count, before_100.stage_count + 1);
    assert_eq!(after_100.prepare_count, before_100.prepare_count + 1);
    assert_eq!(after_100.commit_count, before_100.commit_count + 1);
    assert_eq!(after_100.delivery_count, before_100.delivery_count + 1);
    assert_eq!(after_100.abort_count, before_100.abort_count);
    assert_eq!(staged, &[100.0]);
    assert_eq!(prepared, &[100.0]);
    assert_eq!(committed, &[100.0]);
    assert_eq!(delivered, &[100.0]);
    assert_eq!(aborted, &[] as &[f64]);
    assert_eq!(store_commits, 1);
    assert_eq!(capability_uses, 1);
    let operation_events = events_since(&runtime, events_before_100);
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionStarted { .. }))
            .count(),
        1,
    );
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionCommitted { .. }))
            .count(),
        1,
    );
    assert!(
        !operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
    );
    assert!(!operation_events.iter().any(|event| matches!(
        event.kind,
        RuntimeEventKind::IntegrityConstraintViolated { .. }
    )));
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);

    eprintln!();
    eprintln!("TURN input=100");
    eprintln!("decision=COMMIT");
    eprintln!("receiver.stage={}", acceptance_payloads(staged));
    eprintln!("receiver.prepare={}", acceptance_payloads(prepared));
    eprintln!("receiver.commit={}", acceptance_payloads(committed));
    eprintln!("receiver.abort={}", acceptance_payloads(aborted));
    eprintln!("receiver.deliver={}", acceptance_payloads(delivered));
    eprintln!("store_commits=+{store_commits}");
    eprintln!("capability_uses=+{capability_uses}");
    eprintln!("live_target={live_target}");
    eprintln!("runtime_health={:?}", runtime.runtime_health());

    let before_150 = receiver_snapshot(&receiver);
    let store_before_150 = store_commit_probe.calls();
    let capability_before_150 = capability_use_probe.committed_uses();
    let events_before_150 = runtime.list_events(None).unwrap().len();
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(150.0),
        ))
        .unwrap_err();
    let failures = error
        .kind_as::<mech_program::IntegrityConstraintViolationSet>()
        .expect("invalid candidate returns integrity violations");
    assert_eq!(failures.violations.len(), 1);
    let violation = &failures.violations[0];
    assert_eq!(violation.name, "safe-target!");
    assert_eq!(
        violation.reason,
        mech_program::IntegrityConstraintFailureReason::EvaluatedFalse,
    );
    assert_eq!(violation.actual.as_deref(), Some("150"));
    assert_eq!(violation.expected.as_deref(), Some("120"));
    let after_150 = receiver_snapshot(&receiver);
    let staged = &after_150.staged_payloads[before_150.staged_payloads.len()..];
    let prepared = &after_150.prepared_payloads[before_150.prepared_payloads.len()..];
    let committed = &after_150.committed_payloads[before_150.committed_payloads.len()..];
    let aborted = &after_150.aborted_payloads[before_150.aborted_payloads.len()..];
    let delivered = &after_150.delivered_payloads[before_150.delivered_payloads.len()..];
    let store_commits = store_commit_probe.calls() - store_before_150;
    let capability_uses = capability_use_probe.committed_uses() - capability_before_150;
    assert_eq!(after_150.stage_count, before_150.stage_count + 1);
    assert_eq!(after_150.abort_count, before_150.abort_count + 1);
    assert_eq!(after_150.prepare_count, before_150.prepare_count);
    assert_eq!(after_150.commit_count, before_150.commit_count);
    assert_eq!(after_150.delivery_count, before_150.delivery_count);
    assert_eq!(staged, &[150.0]);
    assert_eq!(aborted, &[150.0]);
    assert_eq!(prepared, &[] as &[f64]);
    assert_eq!(committed, &[] as &[f64]);
    assert_eq!(delivered, &[] as &[f64]);
    assert_eq!(f64_value(&source_value(&runtime, &source)), 100.0);
    let restored_live_target = f64_value(&symbol_value(&runtime, "target"));
    assert_eq!(restored_live_target, 100.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "receiver-result")), 100.0);
    assert!(bool_value(&symbol_value(&runtime, "safe-target!")));
    assert_eq!(store_commits, 0);
    assert_eq!(capability_uses, 0);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
    let operation_events = events_since(&runtime, events_before_150);
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionStarted { .. }))
            .count(),
        1,
    );
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
            .count(),
        1,
    );
    assert!(
        !operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionCommitted { .. }))
    );
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(
                event.kind,
                RuntimeEventKind::IntegrityConstraintViolated { .. }
            ))
            .count(),
        1,
    );
    assert_event_before(
        &operation_events,
        |kind| matches!(kind, RuntimeEventKind::TransactionAborted { .. }),
        |kind| matches!(kind, RuntimeEventKind::IntegrityConstraintViolated { .. }),
    );
    let audit = operation_events
        .iter()
        .find_map(|event| match &event.kind {
            RuntimeEventKind::IntegrityConstraintViolated { violations, .. } => violations.first(),
            _ => None,
        })
        .unwrap();
    assert_eq!(audit.actual.as_deref(), Some("150"));
    assert_eq!(audit.expected.as_deref(), Some("120"));
    assert_eq!(f64_value(&symbol_value(&runtime, "target")), 100.0);
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
    assert!(!runtime.is_poisoned());

    eprintln!();
    eprintln!("TURN input=150");
    eprintln!("decision=ABORT");
    eprintln!("constraint={}", violation.name);
    eprintln!("actual={}", violation.actual.as_deref().unwrap());
    eprintln!(
        "expected_maximum={}",
        violation.expected.as_deref().unwrap(),
    );
    eprintln!("receiver.stage={}", acceptance_payloads(staged));
    eprintln!("receiver.prepare={}", acceptance_payloads(prepared));
    eprintln!("receiver.commit={}", acceptance_payloads(committed));
    eprintln!("receiver.abort={}", acceptance_payloads(aborted));
    eprintln!("receiver.deliver={}", acceptance_payloads(delivered));
    eprintln!("store_commits=+{store_commits}");
    eprintln!("capability_uses=+{capability_uses}");
    eprintln!("restored_live_target={restored_live_target}");
    eprintln!("event_order=TransactionAborted before IntegrityConstraintViolated",);
    eprintln!("runtime_health={:?}", runtime.runtime_health());

    let before_110 = receiver_snapshot(&receiver);
    let store_before_110 = store_commit_probe.calls();
    let capability_before_110 = capability_use_probe.committed_uses();
    let events_before_110 = runtime.list_events(None).unwrap().len();
    runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(110.0),
        ))
        .unwrap();
    assert_eq!(f64_value(&source_value(&runtime, &source)), 110.0);
    let live_target = f64_value(&symbol_value(&runtime, "target"));
    assert_eq!(live_target, 110.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "receiver-result")), 110.0);
    assert!(bool_value(&symbol_value(&runtime, "safe-target!")));
    let after_110 = receiver_snapshot(&receiver);
    let staged = &after_110.staged_payloads[before_110.staged_payloads.len()..];
    let prepared = &after_110.prepared_payloads[before_110.prepared_payloads.len()..];
    let committed = &after_110.committed_payloads[before_110.committed_payloads.len()..];
    let aborted = &after_110.aborted_payloads[before_110.aborted_payloads.len()..];
    let delivered = &after_110.delivered_payloads[before_110.delivered_payloads.len()..];
    let store_commits = store_commit_probe.calls() - store_before_110;
    let capability_uses = capability_use_probe.committed_uses() - capability_before_110;
    assert_eq!(after_110.stage_count, before_110.stage_count + 1);
    assert_eq!(after_110.prepare_count, before_110.prepare_count + 1);
    assert_eq!(after_110.commit_count, before_110.commit_count + 1);
    assert_eq!(after_110.delivery_count, before_110.delivery_count + 1);
    assert_eq!(after_110.abort_count, before_110.abort_count);
    assert_eq!(staged, &[110.0]);
    assert_eq!(prepared, &[110.0]);
    assert_eq!(committed, &[110.0]);
    assert_eq!(delivered, &[110.0]);
    assert_eq!(aborted, &[] as &[f64]);
    assert_eq!(store_commits, 1);
    assert_eq!(capability_uses, 1);
    let operation_events = events_since(&runtime, events_before_110);
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionStarted { .. }))
            .count(),
        1,
    );
    assert_eq!(
        operation_events
            .iter()
            .filter(|event| matches!(event.kind, RuntimeEventKind::TransactionCommitted { .. }))
            .count(),
        1,
    );
    assert!(
        !operation_events
            .iter()
            .any(|event| matches!(event.kind, RuntimeEventKind::TransactionAborted { .. }))
    );
    assert!(!operation_events.iter().any(|event| matches!(
        event.kind,
        RuntimeEventKind::IntegrityConstraintViolated { .. }
    )));
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);

    eprintln!();
    eprintln!("TURN input=110");
    eprintln!("decision=COMMIT");
    eprintln!("receiver.stage={}", acceptance_payloads(staged));
    eprintln!("receiver.prepare={}", acceptance_payloads(prepared));
    eprintln!("receiver.commit={}", acceptance_payloads(committed));
    eprintln!("receiver.abort={}", acceptance_payloads(aborted));
    eprintln!("receiver.deliver={}", acceptance_payloads(delivered));
    eprintln!("store_commits=+{store_commits}");
    eprintln!("capability_uses=+{capability_uses}");
    eprintln!("live_target={live_target}");
    eprintln!("runtime_health={:?}", runtime.runtime_health());

    let staged = &after_110.staged_payloads[installed_receiver.staged_payloads.len()..];
    let prepared = &after_110.prepared_payloads[installed_receiver.prepared_payloads.len()..];
    let committed = &after_110.committed_payloads[installed_receiver.committed_payloads.len()..];
    let aborted = &after_110.aborted_payloads[installed_receiver.aborted_payloads.len()..];
    let delivered = &after_110.delivered_payloads[installed_receiver.delivered_payloads.len()..];
    let store_commits = store_commit_probe.calls() - installed_store_commits;
    let capability_uses = capability_use_probe.committed_uses() - installed_capability_uses;
    assert_eq!(committed, &[100.0, 110.0]);
    assert_eq!(delivered, &[100.0, 110.0]);
    assert_eq!(staged, &[100.0, 150.0, 110.0]);
    assert_eq!(aborted, &[150.0]);
    assert_eq!(prepared, &[100.0, 110.0]);
    assert_eq!(store_commits, 2);
    assert_eq!(capability_uses, 2);
    assert!(runtime.list_events(None).unwrap().len() > installed_events);
    assert_eq!(runtime.runtime_health(), RuntimeHealth::Healthy);

    eprintln!();
    eprintln!("SUMMARY");
    eprintln!("staged={}", acceptance_payloads(staged));
    eprintln!("prepared={}", acceptance_payloads(prepared));
    eprintln!("committed={}", acceptance_payloads(committed));
    eprintln!("aborted={}", acceptance_payloads(aborted));
    eprintln!("delivered={}", acceptance_payloads(delivered));
    eprintln!("store_commits=+{store_commits}");
    eprintln!("capability_uses=+{capability_uses}");
    eprintln!("runtime_health={:?}", runtime.runtime_health());
    eprintln!();
    eprintln!("=== ACCEPTANCE TEST PASSED ===");
}
