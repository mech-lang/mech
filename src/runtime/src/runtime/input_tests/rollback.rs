use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use mech_core::{MechError, Ref, Value, hash_str};

use super::scheduling::{
    activation_plan_snapshot, activation_send_count, apply_f64_input, only_reactive_turn,
    recorded_f64,
};
use crate::RuntimeConfig;
use crate::runtime::test_support::{
    capabilities::{grant_host_call, grant_read, grant_read_to, grant_resource, grant_write},
    providers::{
        DeliberateHostCallError, TEST_OUTPUT_BASE_URI, TestResourceProvider, sleep_host,
        test_provider_with, test_runtime, test_runtime_builder, test_runtime_with_host,
        test_runtime_with_output, test_runtime_with_output_host,
    },
    values::{
        f64_value, host_f64_argument, plan_snapshot, register_node_for_symbol, source_cell,
        source_value, symbol_value,
    },
};
use crate::{
    CapabilityId, PlannedPureHostFunction, RuntimeCallContext, RuntimeCapabilityOperation,
    RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputValue, RuntimeValueSnapshot,
};

const TEST_CLOCK_BASE_URI: &str = "test://clock/ticks";

fn snapshot(value: Value) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn runtime_reactive_host_input_turn_failure_restores_admitted_inputs() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let fail_host = Arc::new(AtomicBool::new(false));
    let fail_host_for_call = fail_host.clone();
    let (mut runtime, output) = test_runtime_with_output_host(
        test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0),
        PlannedPureHostFunction::new(
            "demo/fails-after-first",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(Value::F64(Ref::new(
                    host_f64_argument(&args[0]) + 1.0,
                ))))
            },
            move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                host_calls.fetch_add(1, Ordering::SeqCst);
                if fail_host_for_call.load(Ordering::SeqCst) {
                    return Err(MechError::new(DeliberateHostCallError, None));
                }
                let input = host_f64_argument(&args[0]);
                Ok(snapshot(Value::F64(Ref::new(input + 1.0))))
            },
        ),
    );
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    grant_host_call(&mut runtime, CapabilityId(47), "demo/fails-after-first");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\nhost-result := demo/fails-after-first(@pulse/value)\noutput := host-result\n@out/line <- output").unwrap();
    let calls_before = calls.load(Ordering::SeqCst);
    let input_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    let input_identity = source_cell(&runtime, &input_source);
    let host_result = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("host-result"))
        .unwrap();
    let host_result_pointer = match &*host_result.borrow() {
        Value::F64(value) => value.as_ptr(),
        other => panic!("expected f64 host result, got {other:?}"),
    };
    let plan_before = plan_snapshot(&runtime);
    let lines_before = output.lines();
    assert_eq!(f64_value(&source_value(&runtime, &input_source)), 1.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
    assert_eq!(output.lines().len(), 1);
    assert_eq!(runtime.persistent_send_count(), 1);
    fail_host.store(true, Ordering::SeqCst);
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            input_source.clone(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap_err();
    assert!(format!("{error:?}").contains("DeliberateHostCallError"));
    assert!(calls.load(Ordering::SeqCst) > calls_before);
    assert_eq!(f64_value(&source_value(&runtime, &input_source)), 1.0);
    assert_eq!(source_cell(&runtime, &input_source), input_identity);
    let host_result = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("host-result"))
        .unwrap();
    match &*host_result.borrow() {
        Value::F64(value) => assert_eq!(value.as_ptr(), host_result_pointer),
        other => panic!("expected f64 host result, got {other:?}"),
    };
    assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
    assert_eq!(plan_snapshot(&runtime), plan_before);
    assert_eq!(output.lines(), lines_before);
    assert_eq!(runtime.persistent_send_count(), 1);
    runtime.run_string("recovery := 1").unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "recovery")), 1.0);
}

#[test]
fn failed_source_does_not_leave_live_state_armed() {
    let (mut runtime, output) =
        test_runtime_with_output(test_provider_with("test://clock/ticks", "value", 1.0));
    let mut context_a = runtime.runtime_context().unwrap().with_subject("subject-a");
    grant_read_to(&mut runtime, "subject-a", "test://clock/ticks", "value");
    grant_resource(
        &mut runtime,
        "subject-a",
        TEST_OUTPUT_BASE_URI,
        RuntimeCapabilityOperation::Write,
        &["line"],
    );
    let error = runtime.run_string_with_context(
    &mut context_a,
    "@out := test://effects/output{:write(line)}\n@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value\n@out/line <- output\nmissing := missing-live-value",
  );
    assert!(error.is_err());
    assert!(output.lines().is_empty());
    assert!(runtime.live_context_template.is_none());
    assert!(runtime.live_input_bindings.is_empty());
    assert!(runtime.persistent_sends.is_empty());

    let mut context_b = runtime.runtime_context().unwrap().with_subject("subject-b");
    grant_read_to(&mut runtime, "subject-b", "test://clock/ticks", "value");
    runtime
        .run_string_with_context(
            &mut context_b,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();
}

#[test]
fn duration_failure_restores_live_state() {
    let mut config = RuntimeConfig::default();
    config.limits.max_turn_duration_ms = Some(1);
    let mut runtime = test_runtime_builder()
        .config(config)
        .resource_provider(Box::new(test_provider_with(
            "test://clock/ticks",
            "value",
            1.0,
        )))
        .host_function(sleep_host("demo/source-sleep"))
        .unwrap()
        .build()
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    grant_read_to(
        &mut runtime,
        &context.subject,
        "test://clock/ticks",
        "value",
    );
    grant_host_call(&mut runtime, CapabilityId(43), "demo/source-sleep");
    let error = format!("{:?}", runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\nslow := demo/source-sleep(@pulse/value)").unwrap_err());
    assert!(
        error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"),
        "{error}"
    );
    assert!(runtime.live_context_template.is_none());
    assert!(runtime.live_input_bindings.is_empty());
    assert!(runtime.persistent_sends.is_empty());
    runtime.config.limits.max_turn_duration_ms = None;
    runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn explicit_host_input_is_provisional_until_outer_decision() {
    let mut runtime = test_runtime(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0));
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    let mut load_context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut load_context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    let mut context = runtime.live_turn_context().unwrap();
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();

    let outcome = runtime
        .apply_host_input_with_context(
            &mut context,
            RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(5.0)),
        )
        .unwrap();
    assert!(outcome.turn.is_some());
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 5.0);
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id),);
    assert!(runtime.active_transactions.contains_key(&transaction_id));

    runtime
        .abort_runtime_transaction(&mut context, "discard host input")
        .unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 1.0);
    assert_eq!(f64_value(&source_value(&runtime, &source)), 1.0);

    let mut commit_context = runtime.live_turn_context().unwrap();
    runtime.begin_transaction(&mut commit_context).unwrap();
    runtime
        .apply_host_input_with_context(
            &mut commit_context,
            RuntimeHostInput::single(source, RuntimeHostInputValue::F64(7.0)),
        )
        .unwrap();
    runtime
        .commit_runtime_transaction(&mut commit_context)
        .unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 7.0);
}

#[test]
fn activation_send_duration_failure_does_not_release_fixed_or_patterned_effects() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) =
        test_runtime_with_output_host(provider, sleep_host("demo/activation-duration-sleep"));
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    grant_host_call(
        &mut runtime,
        CapabilityId(195),
        "demo/activation-duration-sleep",
    );
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick

~> render-tick {
@out/line <-
  demo/activation-duration-sleep(render-tick)
}

~> render-tick
| selected => {
    @out/line <- selected
  }
"#,
        )
        .unwrap();

    let plan = activation_plan_snapshot(&runtime);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 2);

    runtime.config.limits.max_turn_duration_ms = Some(1);
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap_err();
    let error = format!("{error:?}");
    assert!(
        error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"),
        "{error}"
    );
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);

    runtime.config.limits.max_turn_duration_ms = None;
    let retry = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert!(retry.turn.is_some());
    assert_eq!(output.lines(), vec!["1", "1"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
}

#[test]
fn patterned_activation_register_batch_failure_does_not_leak_or_replay_send() {
    let provider = TestResourceProvider::new()
        .with_value("test://render/timer", "tick", Value::F64(Ref::new(0.0)))
        .with_value("test://other/timer", "tick", Value::F64(Ref::new(0.0)));
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_read(&mut runtime, "test://other/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
other-tick := @other/tick
other-value := other-tick
~state := 0.0
~> render-tick
| amount => {
    state = amount
    state = amount
    @out/line <- state
  }
| * => {
    fallback := 0.0
  }
"#,
        )
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap_err();
    assert!(
        format!("{error:?}").contains("ReactiveRegisterOutputConflict"),
        "unexpected error: {error:?}"
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    let unrelated = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://other/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(7.0),
        ))
        .unwrap();
    assert!(unrelated.turn.is_some());
    assert_eq!(f64_value(&symbol_value(&runtime, "other-value")), 7.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 1);
}

#[test]
fn failed_patterned_body_does_not_replay_send_on_unrelated_turn() {
    let provider = TestResourceProvider::new()
        .with_value("test://render/timer", "tick", Value::F64(Ref::new(0.0)))
        .with_value("test://other/timer", "tick", Value::F64(Ref::new(0.0)));
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let host = PlannedPureHostFunction::new(
        "demo/patterned-body-fail-second",
        |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
            Ok(snapshot(Value::F64(Ref::new(host_f64_argument(&args[0])))))
        },
        move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
            let call = host_calls.fetch_add(1, Ordering::SeqCst) + 1;
            if call == 1 {
                return Err(MechError::new(DeliberateHostCallError, None));
            }
            Ok(snapshot(Value::F64(Ref::new(host_f64_argument(&args[0])))))
        },
    );
    let (mut runtime, output) = test_runtime_with_output_host(provider, host);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_read(&mut runtime, "test://other/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    grant_host_call(
        &mut runtime,
        CapabilityId(194),
        "demo/patterned-body-fail-second",
    );
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@other := test://other/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
other-tick := @other/tick
other-value := other-tick
~state := 0.0
~> render-tick
| amount => {
    state = demo/patterned-body-fail-second(amount)
    @out/line <- state
  }
| * => {
    fallback := 0.0
  }
"#,
        )
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap_err();
    assert!(error.kind_as::<DeliberateHostCallError>().is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    let unrelated = apply_f64_input(&mut runtime, "test://other/timer", "tick", 7.0);
    assert!(unrelated.turn.is_some());
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    let retry = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert!(retry.turn.is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 1);
}

#[test]
fn activation_send_reactive_failure_produces_no_writes() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let (mut runtime, output) = test_runtime_with_output_host(
        provider,
        PlannedPureHostFunction::new(
            "demo/activation-fail-second",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(Value::F64(Ref::new(host_f64_argument(&args[0])))))
            },
            move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                let call_number = host_calls.fetch_add(1, Ordering::SeqCst) + 1;
                if call_number == 1 {
                    return Err(MechError::new(DeliberateHostCallError, None));
                }
                Ok(snapshot(Value::F64(Ref::new(host_f64_argument(&args[0])))))
            },
        ),
    );
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    grant_host_call(
        &mut runtime,
        CapabilityId(193),
        "demo/activation-fail-second",
    );
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}

render-tick := @tick/tick

~> render-tick {
@out/line <-
  demo/activation-fail-second(render-tick)
}
"#,
        )
        .unwrap();
    let plan_before = activation_plan_snapshot(&runtime);
    assert_eq!(
        calls.load(Ordering::SeqCst),
        0,
        "host invocation must not run during planning"
    );
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://render/timer", "tick").unwrap(),
            RuntimeHostInputValue::F64(1.0),
        ))
        .unwrap_err();
    assert!(
        error.kind_as::<DeliberateHostCallError>().is_some(),
        "unexpected error: {error:?}"
    );
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    assert_eq!(activation_plan_snapshot(&runtime), plan_before);
    let retry = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    assert!(retry.turn.is_some());
    assert_eq!(calls.load(Ordering::SeqCst), 2);
    assert_eq!(output.lines().len(), 1);
    assert_eq!(recorded_f64(&output, 0), 1.0);
    assert_eq!(activation_send_count(&runtime), 1);
    assert_eq!(activation_plan_snapshot(&runtime), plan_before);
}

#[test]
fn patterned_activation_guard_rejects_runtime_host_before_elaboration() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let mut runtime = test_runtime_with_host(
        TestResourceProvider::new(),
        PlannedPureHostFunction::new(
            "demo/audit",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(Value::Bool(Ref::new(true))))
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                host_calls.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(Value::Bool(Ref::new(true))))
            },
        ),
    );
    grant_host_call(&mut runtime, CapabilityId(43), "demo/audit");
    runtime
        .run_string("event := (:released, 1.0)\nsentinel := 7.0")
        .unwrap();
    let plan_before = plan_snapshot(&runtime);
    let symbols_before = runtime.program.interpreter().symbols().borrow().snapshot();
    let registrations_before = runtime
        .program
        .interpreter()
        .plan()
        .pattern_activation_registrations()
        .to_vec();

    let error = runtime
        .run_string(
            r#"
~> event
  | :pressed(x), demo/audit(x) => {
      selected := x + 0.0
    }
  | * => {
      selected := -1.0
    }
"#,
        )
        .unwrap_err();

    assert_eq!(error.kind_name(), "ActivationPatternGuardMustBePure");
    assert_eq!(calls.load(Ordering::SeqCst), 0);
    assert_eq!(plan_snapshot(&runtime), plan_before);
    assert_eq!(
        runtime.program.interpreter().symbols().borrow().snapshot(),
        symbols_before,
    );
    let registrations_after = runtime
        .program
        .interpreter()
        .plan()
        .pattern_activation_registrations()
        .to_vec();
    assert_eq!(registrations_after, registrations_before);
    assert!(
        !runtime
            .program
            .interpreter()
            .plan()
            .activation_registration_active()
    );
    assert!(
        !runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    assert!(
        runtime
            .program
            .interpreter()
            .symbols()
            .borrow()
            .get(hash_str("selected"))
            .is_none()
    );
}

#[test]
fn patterned_activation_snapshots_effects_before_commit_and_releases_after_success() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, "test://render/timer", "tick");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~state := 0.0
~> render-tick
| amount => {
    state = amount
    @out/line <- state
    @out/line <- state
  }
| ignored => {
    fallback := ignored
  }
"#,
        )
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);
    let register = register_node_for_symbol(&runtime, "state");
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 0.0);
    assert!(output.lines().is_empty());

    let first = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    let first_turn = only_reactive_turn(&first);
    assert_eq!(first_turn.register_commit.committed_nodes, vec![register]);
    assert!(first_turn.after_commit.pending_register_nodes.is_empty());
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0", "0"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);

    let equal = apply_f64_input(&mut runtime, "test://render/timer", "tick", 1.0);
    let equal_turn = only_reactive_turn(&equal);
    assert_eq!(equal_turn.register_commit.committed_nodes, vec![register]);
    assert!(equal_turn.after_commit.pending_register_nodes.is_empty());
    assert_eq!(f64_value(&symbol_value(&runtime, "state")), 1.0);
    assert_eq!(output.lines(), vec!["0", "0", "1", "1"]);
    assert_eq!(activation_plan_snapshot(&runtime), plan);
    assert_eq!(activation_send_count(&runtime), 2);
}
