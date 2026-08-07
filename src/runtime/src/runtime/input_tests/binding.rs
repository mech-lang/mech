use mech_core::{Ref, Value};

use super::scheduling::{activation_send_count, apply_f64_input};
use crate::runtime::test_support::{
    capabilities::{grant_read, grant_read_to, grant_write_to},
    providers::{
        TEST_OUTPUT_BASE_URI, TestResourceProvider, test_provider_with, test_runtime,
        test_runtime_with_output,
    },
    values::{f64_value, source_cell, source_value, symbol_value},
};
use crate::{
    CapabilityRequest, RuntimeHostInput, RuntimeHostInputSource, RuntimeHostInputUpdate,
    RuntimeHostInputValue,
};

const TEST_TIMER_BASE_URI: &str = "test://timer/state";

#[test]
fn canonical_host_input_updates_live_context_read() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    grant_read(&mut runtime, "test://clock/ticks", "value");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();

    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 1.0);
    let canonical = RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap();
    let source_identity = source_cell(&runtime, &canonical);
    assert!(runtime.live_input_bindings.contains_key(&canonical));
    assert!(
        runtime
            .live_input_bindings
            .keys()
            .all(|source| !source.base_uri().contains("pulse") && !source.path().contains("pulse"))
    );

    let alias = RuntimeHostInputSource::new("test://pulse", "value").unwrap();
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            alias,
            RuntimeHostInputValue::F64(5.0),
        ))
        .unwrap();
    assert_eq!(outcome.update_count, 1);
    assert_eq!(outcome.ignored_update_count, 1);
    assert_eq!(outcome.binding_count, 0);
    assert!(outcome.turn.is_none());
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 1.0);

    runtime
        .ingress()
        .submit(RuntimeHostInput::single(
            canonical.clone(),
            RuntimeHostInputValue::F64(5.0),
        ))
        .unwrap();
    let outcomes = runtime.drain_host_inputs(1).unwrap();
    assert_eq!(outcomes.len(), 1);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 5.0);
    assert_eq!(source_cell(&runtime, &canonical), source_identity);
}

#[test]
fn partial_snapshot_updates_bound_fields_and_ignores_unbound_fields() {
    let provider = TestResourceProvider::new()
        .with_value(TEST_TIMER_BASE_URI, "tick", Value::F64(Ref::new(1.0)))
        .with_value(
            TEST_TIMER_BASE_URI,
            "delta-seconds",
            Value::F64(Ref::new(0.1)),
        );
    let mut runtime = test_runtime(provider);
    grant_read(&mut runtime, "test://timer/state", "tick");
    grant_read(&mut runtime, "test://timer/state", "delta-seconds");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@timer := test://timer/state{:read(tick), :read(delta-seconds)}\noutput := @timer/tick\nbound-delta := @timer/delta-seconds").unwrap();

    let outcome = runtime
        .apply_host_input(
            RuntimeHostInput::new(vec![
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "tick").unwrap(),
                    value: RuntimeHostInputValue::F64(10.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "elapsed-ms")
                        .unwrap(),
                    value: RuntimeHostInputValue::F64(1000.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "delta-ms").unwrap(),
                    value: RuntimeHostInputValue::F64(16.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "elapsed-seconds")
                        .unwrap(),
                    value: RuntimeHostInputValue::F64(1.0),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "delta-seconds")
                        .unwrap(),
                    value: RuntimeHostInputValue::F64(0.25),
                },
                RuntimeHostInputUpdate {
                    source: RuntimeHostInputSource::new("test://timer/state", "skipped-steps")
                        .unwrap(),
                    value: RuntimeHostInputValue::F64(0.0),
                },
            ])
            .unwrap(),
        )
        .unwrap();

    assert_eq!(outcome.update_count, 6);
    assert_eq!(outcome.ignored_update_count, 4);
    assert_eq!(outcome.binding_count, 2);
    assert!(outcome.turn.is_some());
    assert_eq!(
        f64_value(&source_value(
            &runtime,
            &RuntimeHostInputSource::new("test://timer/state", "tick").unwrap()
        )),
        10.0
    );
    assert_eq!(
        f64_value(&source_value(
            &runtime,
            &RuntimeHostInputSource::new("test://timer/state", "delta-seconds").unwrap()
        )),
        0.25
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}

#[test]
fn transactional_live_program_is_provisional_until_outer_commit() {
    let mut runtime = test_runtime(test_provider_with("test://clock/ticks", "value", 1.0));
    let mut context = runtime.runtime_context().unwrap();
    grant_read_to(
        &mut runtime,
        &context.subject,
        "test://clock/ticks",
        "value",
    );
    let transaction_id = runtime.begin_transaction(&mut context).unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := @pulse/value",
        )
        .unwrap();

    assert!(runtime.live_context_template.is_some());
    assert!(!runtime.live_input_bindings.is_empty());
    assert_eq!(runtime.program_transaction_owner, Some(transaction_id));
    assert!(runtime.program.root_symbol_value("output").is_ok());

    runtime
        .abort_runtime_transaction(&mut context, "discard live program")
        .unwrap();

    assert_eq!(context.transaction, None);
    assert!(runtime.live_context_template.is_none());
    assert!(runtime.live_input_bindings.is_empty());
    assert!(runtime.persistent_sends.is_empty());
    assert!(runtime.program.root_symbol_value("output").is_err());
}

#[test]
fn patterned_activation_send_preserves_custom_live_authority() {
    let provider = TestResourceProvider::new().with_value(
        "test://render/timer",
        "tick",
        Value::F64(Ref::new(0.0)),
    );
    let (mut runtime, output) = test_runtime_with_output(provider);
    let default_subject = runtime.runtime_context().unwrap().subject;
    let custom_subject = "task:patterned-activation-send-custom";
    grant_read_to(&mut runtime, custom_subject, "test://render/timer", "tick");
    grant_write_to(&mut runtime, custom_subject, TEST_OUTPUT_BASE_URI, "line");
    assert!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                &default_subject,
                "read",
                "test://render/timer/tick"
            ))
            .is_err()
    );
    assert!(
        runtime
            .check_capability(&CapabilityRequest::from_keys(
                &default_subject,
                "write",
                format!("{TEST_OUTPUT_BASE_URI}/line")
            ))
            .is_err()
    );
    let mut context = runtime
        .runtime_context()
        .unwrap()
        .with_subject(custom_subject);
    runtime
        .run_string_with_context(
            &mut context,
            r#"@tick := test://render/timer{:read(tick)}
@out := test://effects/output{:write(line)}
render-tick := @tick/tick
~> render-tick
| selected => {
    @out/line <- selected
  }
| * => {
    fallback := 0.0
  }
"#,
        )
        .unwrap();
    assert!(output.lines().is_empty());
    assert_eq!(activation_send_count(&runtime), 1);
    let outcome = apply_f64_input(&mut runtime, "test://render/timer", "tick", 9.0);
    assert!(outcome.turn.is_some());
    assert_eq!(output.lines(), vec!["9"]);
    assert_eq!(activation_send_count(&runtime), 1);
}
