use std::collections::HashMap;
use std::sync::Arc;

use mech_core::{MechError, Ref, Value, hash_str};

use super::super::{MechRuntime, RuntimeBuilder};
use super::scheduling::{ActivationPlanSnapshot, activation_plan_snapshot};
use crate::runtime::test_support::{
    capabilities::{grant_host_call, grant_read, grant_read_to, grant_write},
    providers::{
        RecordingTestOutput, TEST_OUTPUT_BASE_URI, TestResourceProvider, sleep_host,
        test_provider_with, test_runtime, test_runtime_with_output,
    },
    values::{f64_value, plan_snapshot, source_value, symbol_value},
};
use crate::{
    ActivationScopeEffectWithRegisterUnsupported, ActorId, BasicCapability, BasicOperation,
    BasicResource, BasicSubject, CapabilityId, MessageId, MessageRecord, ObjectId, ResourceBudget,
    RuntimeAuthorityScope, RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue, RuntimeInvalidOperationError, RuntimeIsolatedActivationSendUnsupported,
};

const TEST_CLOCK_BASE_URI: &str = "test://clock/ticks";
const TEST_SIGNALS_BASE_URI: &str = "test://signals/inputs";

#[test]
fn packet_with_bound_and_unbound_sources_updates_bound_inputs() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    grant_read(&mut runtime, "test://clock/ticks", "value");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2",
        )
        .unwrap();
    let outcome = runtime
        .apply_host_input(
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
                    value: RuntimeHostInputValue::F64(5.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://clock/ticks", "missing").unwrap(),
                    value: RuntimeHostInputValue::F64(9.0),
                },
            ])
            .unwrap(),
        )
        .unwrap();

    assert_eq!(outcome.update_count, 2);
    assert_eq!(outcome.ignored_update_count, 1);
    assert_eq!(outcome.binding_count, 1);
    assert!(outcome.turn.is_some());
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
    assert_eq!(
        f64_value(&source_value(
            &runtime,
            &RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap()
        )),
        5.0
    );
}

#[test]
fn runtime_reactive_host_input_preflight_failure_mutates_nothing() {
    let provider = TestResourceProvider::new()
        .with_value(TEST_SIGNALS_BASE_URI, "a", Value::F64(Ref::new(1.0)))
        .with_value(TEST_SIGNALS_BASE_URI, "b", Value::F64(Ref::new(2.0)));
    let (mut runtime, output) = test_runtime_with_output(provider);
    grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "a");
    grant_read(&mut runtime, TEST_SIGNALS_BASE_URI, "b");
    grant_write(&mut runtime, TEST_OUTPUT_BASE_URI, "line");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@out := test://effects/output{:write(line)}\n@signals := test://signals/inputs{:read(a), :read(b)}\nsum := @signals/a + @signals/b\n@out/line <- sum").unwrap();
    let a_source = RuntimeHostInputSource::new(TEST_SIGNALS_BASE_URI, "a").unwrap();
    let b_source = RuntimeHostInputSource::new(TEST_SIGNALS_BASE_URI, "b").unwrap();
    let a_before = f64_value(&source_value(&runtime, &a_source));
    let b_before = f64_value(&source_value(&runtime, &b_source));
    let sum_before = f64_value(&symbol_value(&runtime, "sum"));
    let plan_before = plan_snapshot(&runtime);
    let lines_before = output.lines();
    assert!(
        !runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    let error = runtime
        .apply_host_input(
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: a_source.clone(),
                    value: RuntimeHostInputValue::F64(10.0),
                },
                RuntimeHostInputUpdate {
                    source: b_source.clone(),
                    value: RuntimeHostInputValue::String("bad".to_string()),
                },
            ])
            .unwrap(),
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("StableValueUpdateKindMismatch"));
    assert_eq!(f64_value(&source_value(&runtime, &a_source)), a_before);
    assert_eq!(f64_value(&source_value(&runtime, &b_source)), b_before);
    assert_eq!(f64_value(&symbol_value(&runtime, "sum")), sum_before);
    assert_eq!(plan_snapshot(&runtime), plan_before);
    assert!(
        !runtime
            .program
            .interpreter()
            .has_pending_reactive_registers()
    );
    assert_eq!(output.lines(), lines_before);
}

#[test]
fn live_turn_enforces_step_budget() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    let mut context = runtime
        .runtime_context()
        .unwrap()
        .with_budget(ResourceBudget::default().with_max_steps(1));
    grant_read_to(
        &mut runtime,
        &context.subject,
        "test://clock/ticks",
        "value",
    );
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2",
        )
        .unwrap();
    runtime
        .live_context_template
        .as_mut()
        .unwrap()
        .budget_limits
        .max_steps = Some(0);
    let error = format!(
        "{:?}",
        runtime
            .apply_host_input(RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
                RuntimeHostInputValue::F64(2.0)
            ))
            .unwrap_err()
    );
    assert!(error.contains("ResourceBudgetExceeded"), "{error}");

    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    let mut context = runtime
        .runtime_context()
        .unwrap()
        .with_budget(ResourceBudget::default().with_max_steps(1));
    grant_read_to(
        &mut runtime,
        &context.subject,
        "test://clock/ticks",
        "value",
    );
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2",
        )
        .unwrap();
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(2.0),
        ))
        .unwrap();
    assert!(outcome.turn.is_some());
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(3.0),
        ))
        .unwrap();
    assert!(outcome.turn.is_some());
}

#[test]
fn live_turn_enforces_duration_limit() {
    let mut runtime = RuntimeBuilder::new()
        .resource_provider(Box::new(test_provider_with(
            "test://clock/ticks",
            "value",
            1.0,
        )))
        .host_function(sleep_host("demo/live-sleep"))
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
    grant_host_call(&mut runtime, CapabilityId(44), "demo/live-sleep");
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\nslow := demo/live-sleep(@pulse/value)",
        )
        .unwrap();
    runtime.config.limits.max_turn_duration_ms = Some(1);
    let error = format!(
        "{:?}",
        runtime
            .apply_host_input(RuntimeHostInput::single(
                RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
                RuntimeHostInputValue::F64(2.0)
            ))
            .unwrap_err()
    );
    assert!(
        error.contains("turn_duration_ms") || error.contains("ResourceBudgetExceeded"),
        "{error}"
    );
    assert!(
        runtime
            .program()
            .interpreter()
            .symbols()
            .borrow()
            .contains(hash_str("slow"))
    );
    runtime.config.limits.max_turn_duration_ms = None;
    runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn changed_actor_message_rejects_live_context_reuse() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    let actor = ActorId(7);
    let state = ObjectId(9);
    let mut context_a = runtime
        .runtime_context()
        .unwrap()
        .with_subject("actor:7")
        .with_actor(actor);
    context_a.actor_state = Some(state);
    context_a.actor_message = Some(MessageRecord::new(
        MessageId(1),
        actor,
        "tick",
        b"a".to_vec(),
    ));
    grant_read_to(&mut runtime, "actor:7", "test://clock/ticks", "value");
    runtime
        .run_string_with_context(
            &mut context_a,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();

    let mut context_b = runtime
        .runtime_context()
        .unwrap()
        .with_subject("actor:7")
        .with_actor(actor);
    context_b.actor_state = Some(state);
    context_b.actor_message = Some(MessageRecord::new(
        MessageId(2),
        actor,
        "tick",
        b"b".to_vec(),
    ));
    let error = format!(
        "{:?}",
        runtime
            .run_string_with_context(
                &mut context_b,
                "@pulse := test://clock/ticks{:read(value)}\nother := @pulse/value"
            )
            .unwrap_err()
    );
    assert!(error.contains("RuntimeLiveContextMismatch"), "{error}");
}

#[test]
fn live_context_capability_order_does_not_cause_mismatch() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    let subject = runtime.runtime_context().unwrap().subject;
    let read = grant_read_to(&mut runtime, &subject, "test://clock/ticks", "value");
    let unused = CapabilityId(2);
    let mut context_a = runtime
        .runtime_context()
        .unwrap()
        .with_capabilities(vec![read, unused]);
    let mut context_b = runtime
        .runtime_context()
        .unwrap()
        .with_capabilities(vec![unused, read]);
    runtime
        .run_string_with_context(
            &mut context_a,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();
    runtime
        .run_string_with_context(
            &mut context_b,
            "@pulse := test://clock/ticks{:read(value)}\nother := @pulse/value",
        )
        .unwrap();
}

#[test]
fn host_input_preparation_opens_no_transaction_or_budget_charge() {
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
    let used_steps = context.budget.used_steps;
    let event_sequence = runtime.event_sequence;

    let empty = RuntimeHostInput {
        updates: Vec::new(),
    };
    assert!(
        runtime
            .apply_host_input_with_context(&mut context, empty)
            .is_err()
    );
    assert_eq!(context.budget.used_steps, used_steps);
    assert_eq!(runtime.event_sequence, event_sequence);
    assert!(runtime.active_transactions.is_empty());

    let target = runtime.live_input_bindings[&source][0];
    let alias = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "alias").unwrap();
    runtime
        .live_input_bindings
        .insert(alias.clone(), vec![target]);
    let duplicate = RuntimeHostInput::new(vec![
        RuntimeHostInputUpdate {
            source: source.clone(),
            value: RuntimeHostInputValue::F64(2.0),
        },
        RuntimeHostInputUpdate {
            source: alias,
            value: RuntimeHostInputValue::F64(3.0),
        },
    ])
    .unwrap();
    let error = runtime
        .apply_host_input_with_context(&mut context, duplicate)
        .unwrap_err();
    assert_eq!(error.kind_name(), "ProgramInputDuplicateTarget");
    assert_eq!(context.budget.used_steps, used_steps);
    assert_eq!(runtime.event_sequence, event_sequence);
    assert!(runtime.active_transactions.is_empty());

    let unbound = RuntimeHostInput::single(
        RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing").unwrap(),
        RuntimeHostInputValue::F64(4.0),
    );
    let outcome = runtime
        .apply_host_input_with_context(&mut context, unbound)
        .unwrap();
    assert!(outcome.turn.is_none());
    assert_eq!(context.budget.used_steps, used_steps);
    assert_eq!(runtime.event_sequence, event_sequence);
    assert!(runtime.active_transactions.is_empty());
}

#[test]
fn unbound_host_input_without_a_live_program_is_a_noop() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let event_sequence = runtime.event_sequence;
    let input = RuntimeHostInput::single(
        RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "missing").unwrap(),
        RuntimeHostInputValue::F64(4.0),
    );

    let outcome = runtime.apply_host_input(input).unwrap();

    assert_eq!(outcome.update_count, 1);
    assert_eq!(outcome.ignored_update_count, 1);
    assert_eq!(outcome.binding_count, 0);
    assert!(outcome.turn.is_none());
    assert_eq!(runtime.event_sequence, event_sequence);
    assert!(runtime.active_transactions.is_empty());
    assert_eq!(runtime.program_transaction_owner, None);
}

#[test]
fn implicit_host_input_cannot_drive_an_explicit_program_owner() {
    let mut runtime = test_runtime(test_provider_with(TEST_CLOCK_BASE_URI, "value", 1.0));
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "value");
    let mut load_context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut load_context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value * 2",
        )
        .unwrap();
    let source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "value").unwrap();
    let mut owner_context = runtime.live_turn_context().unwrap();
    let owner = runtime.begin_transaction(&mut owner_context).unwrap();
    runtime.step_with_context(&mut owner_context, 0).unwrap();

    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            source.clone(),
            RuntimeHostInputValue::F64(5.0),
        ))
        .unwrap_err();
    assert_eq!(error.kind_name(), "RuntimeProgramBusy");
    assert_eq!(runtime.program_transaction_owner, Some(owner));
    assert_eq!(f64_value(&source_value(&runtime, &source)), 1.0);

    runtime
        .abort_runtime_transaction(&mut owner_context, "release input owner")
        .unwrap();
}

#[test]
fn host_input_context_accepts_transaction_capabilities_but_rejects_drift() {
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
    runtime.begin_transaction(&mut context).unwrap();
    let capability_id = CapabilityId(910);
    let subject = context.subject.clone();
    runtime
        .grant_capability_with_context(
            &mut context,
            Arc::new(BasicCapability::new(
                capability_id,
                &BasicSubject::new(&subject),
                &BasicResource::new("db://host-input"),
                [BasicOperation::read()],
            )),
        )
        .unwrap();
    runtime
        .apply_host_input_with_context(
            &mut context,
            RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(2.0)),
        )
        .unwrap();
    runtime
        .revoke_capability_with_context(&mut context, capability_id)
        .unwrap();
    runtime
        .apply_host_input_with_context(
            &mut context,
            RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(3.0)),
        )
        .unwrap();
    runtime
        .abort_runtime_transaction(&mut context, "capability context test")
        .unwrap();

    let mut drift = runtime.live_turn_context().unwrap();
    drift.authority = RuntimeAuthorityScope::allow_list([CapabilityId(999)]);
    let error = runtime
        .apply_host_input_with_context(
            &mut drift,
            RuntimeHostInput::single(source.clone(), RuntimeHostInputValue::F64(4.0)),
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("RuntimeLiveContextMismatch"));

    let mut maxima = runtime.live_turn_context().unwrap();
    maxima.budget.max_steps = Some(1);
    let error = runtime
        .apply_host_input_with_context(
            &mut maxima,
            RuntimeHostInput::single(source, RuntimeHostInputValue::F64(5.0)),
        )
        .unwrap_err();
    assert!(format!("{error:?}").contains("RuntimeLiveContextMismatch"));
}

fn activation_rejection_runtime() -> (MechRuntime, RecordingTestOutput) {
    let (mut r, o) = test_runtime_with_output(TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    ));
    grant_read(&mut r, "test://render/timer", "tick");
    grant_write(&mut r, TEST_OUTPUT_BASE_URI, "line");
    (r, o)
}
fn assert_rejected_activation_left_no_state(
    r: &MechRuntime,
    o: &RecordingTestOutput,
    p: &ActivationPlanSnapshot,
    s: usize,
    b: &HashMap<RuntimeHostInputSource, Vec<mech_program::ProgramInputId>>,
) {
    assert_eq!(activation_plan_snapshot(r), *p);
    assert_eq!(r.persistent_sends.len(), s);
    assert_eq!(r.live_input_bindings, *b);
    assert!(o.lines().is_empty());
    assert!(
        !r.program
            .interpreter()
            .plan()
            .activation_registration_active()
    );
}

#[test]
fn activation_send_registration_is_atomic_on_fixed_elaboration_failure() {
    let (mut runtime, output) = activation_rejection_runtime();
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
render-tick := @tick/tick
"#,
        )
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);
    let sends = runtime.persistent_sends.len();
    let bindings = runtime.live_input_bindings.clone();

    let error = runtime
        .run_string(
            r#"@out := test://effects/output{:write(line)}
~> render-tick {
@out/line <- render-tick
registered-first := render-tick + 1.0
failure :=
  function-that-does-not-exist(registered-first)
}
"#,
        )
        .unwrap_err();

    assert!(
        error.kind_name().contains("Function"),
        "unexpected elaboration error: {error:?}"
    );
    assert_rejected_activation_left_no_state(&runtime, &output, &plan, sends, &bindings);
    assert!(
        !runtime
            .program
            .interpreter()
            .symbols()
            .borrow()
            .contains(hash_str("registered-first"))
    );
    assert!(
        !runtime
            .program
            .interpreter()
            .symbols()
            .borrow()
            .contains(hash_str("failure"))
    );
}

#[test]
fn patterned_activation_send_registration_is_atomic_on_elaboration_failure() {
    let (mut runtime, output) = activation_rejection_runtime();
    runtime
        .run_string(
            r#"@tick := test://render/timer{:read(tick)}
render-tick := @tick/tick
"#,
        )
        .unwrap();
    let plan = activation_plan_snapshot(&runtime);
    let sends = runtime.persistent_sends.len();
    let bindings = runtime.live_input_bindings.clone();
    let error = runtime
        .run_string(
            r#"@out := test://effects/output{:write(line)}
~> render-tick
| selected => {
    @out/line <- selected
  }
| * => {
    failure := function-that-does-not-exist(1.0)
  }
"#,
        )
        .unwrap_err();
    assert!(
        error.kind_name().contains("Function"),
        "unexpected elaboration error: {error:?}"
    );
    assert_rejected_activation_left_no_state(&runtime, &output, &plan, sends, &bindings);
}

#[test]
fn patterned_activation_send_rejects_isolated_registration() {
    let (mut runtime, output) = activation_rejection_runtime();
    let plan = activation_plan_snapshot(&runtime);
    let sends = runtime.persistent_sends.len();
    let bindings = runtime.live_input_bindings.clone();
    let mut context = runtime.runtime_context().unwrap();
    let error = runtime
        .run_string_with_isolated_registration_for_test(
            &mut context,
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
| selected, selected > 0.0 => {
    @out/line <- selected
  }
| * => {
    fallback := 0.0
  }
"#,
        )
        .unwrap_err();
    assert!(
        error
            .kind_as::<RuntimeIsolatedActivationSendUnsupported>()
            .is_some(),
        "unexpected error: {error:?}"
    );
    assert_rejected_activation_left_no_state(&runtime, &output, &plan, sends, &bindings);
}

fn rejected(source: &str, check: impl FnOnce(MechError)) {
    let (mut r, o) = activation_rejection_runtime();
    let p = activation_plan_snapshot(&r);
    let n = r.persistent_sends.len();
    let b = r.live_input_bindings.clone();
    check(r.run_string(source).unwrap_err());
    assert_rejected_activation_left_no_state(&r, &o, &p, n, &b);
}
#[test]
fn activation_send_rejects_local_register_mix() {
    rejected(
        r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~x := 0.0
~> render-tick {
x = x + 1.0
@out/line <- x
}
"#,
        |e| {
            assert!(
                e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>()
                    .is_some(),
                "unexpected error: {e:?}"
            )
        },
    );
}
#[test]
fn activation_send_rejects_local_op_assign_mix() {
    rejected(
        r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~x := 0.0
~> render-tick {
x += 1.0
@out/line <- x
}
"#,
        |e| {
            assert!(
                e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>()
                    .is_some(),
                "unexpected error: {e:?}"
            )
        },
    );
}
#[test]
fn activation_send_rejects_context_assignment_in_scope() {
    rejected(
        r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
x := 1.0
~> render-tick {
@out/line = x
@out/line <- x
}
"#,
        |e| {
            let k = e
                .kind_as::<RuntimeInvalidOperationError>()
                .expect("expected RuntimeInvalidOperationError");
            assert_eq!(k.operation, "direct_context_effect_placement");
            assert!(
                k.reason.contains("context assignment"),
                "unexpected placement reason: {}",
                k.reason
            );
            assert!(
                e.kind_as::<ActivationScopeEffectWithRegisterUnsupported>()
                    .is_none()
            );
        },
    );
}
#[test]
fn activation_send_rejects_isolated_registration() {
    let (mut r, o) = activation_rejection_runtime();
    let p = activation_plan_snapshot(&r);
    let n = r.persistent_sends.len();
    let b = r.live_input_bindings.clone();
    let mut c = r.runtime_context().unwrap();
    let e = r
        .run_string_with_isolated_registration_for_test(
            &mut c,
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick {
@out/line <- render-tick
}
"#,
        )
        .unwrap_err();
    assert!(
        e.kind_as::<RuntimeIsolatedActivationSendUnsupported>()
            .is_some(),
        "unexpected error: {e:?}"
    );
    assert_rejected_activation_left_no_state(&r, &o, &p, n, &b);
}
