use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

use mech_core::{LegacyValue, Ref, hash_str};

use crate::runtime::test_support::{
    capabilities::{grant_host_call, grant_read},
    providers::{
        TestResourceProvider, test_provider_with, test_runtime_builder, test_runtime_with_host,
    },
    values::{f64_value, host_f64_argument, source_cell, source_value, string_value, symbol_value},
};
use crate::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId,
    DeterministicHostFunction, PlannedPureHostFunction, RuntimeCallContext, RuntimeHostInput,
    RuntimeHostInputSource, RuntimeHostInputValue, RuntimeValueSnapshot,
};

const TEST_CLOCK_BASE_URI: &str = "test://clock/ticks";

fn snapshot(value: LegacyValue) -> RuntimeValueSnapshot {
    RuntimeValueSnapshot::try_capture(&value).expect("acyclic fixture")
}

#[test]
fn live_input_recomputes_runtime_host_function() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let host = PlannedPureHostFunction::new(
        "demo/live-plus-one",
        |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
            Ok(snapshot(LegacyValue::F64(Ref::new(
                host_f64_argument(&args[0]) + 1.0,
            ))))
        },
        move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
            host_calls.fetch_add(1, Ordering::SeqCst);
            Ok(snapshot(LegacyValue::F64(Ref::new(
                host_f64_argument(&args[0]) + 1.0,
            ))))
        },
    );
    let mut runtime =
        test_runtime_with_host(test_provider_with("test://clock/ticks", "value", 1.0), host);
    grant_read(&mut runtime, "test://clock/ticks", "value");
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(42),
            &BasicSubject::new(&runtime.runtime_context().unwrap().subject),
            &BasicResource::new("host:demo/live-plus-one"),
            [BasicOperation::new("call")],
        )))
        .unwrap();
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-plus-one(@pulse/value)").unwrap();
    let initial_calls = calls.load(Ordering::SeqCst);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);

    runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();

    assert!(calls.load(Ordering::SeqCst) > initial_calls);
    assert_eq!(
        f64_value(&source_value(
            &runtime,
            &RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap()
        )),
        9.0
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 10.0);
}

#[test]
fn runtime_reactive_host_input_executes_only_reachable_branch() {
    let provider = TestResourceProvider::new()
        .with_value(TEST_CLOCK_BASE_URI, "left", LegacyValue::F64(Ref::new(1.0)))
        .with_value(
            TEST_CLOCK_BASE_URI,
            "right",
            LegacyValue::F64(Ref::new(2.0)),
        );
    let left_calls = Arc::new(AtomicUsize::new(0));
    let right_calls = Arc::new(AtomicUsize::new(0));
    let left_host_calls = left_calls.clone();
    let right_host_calls = right_calls.clone();
    let mut runtime = test_runtime_builder()
        .resource_provider(Box::new(provider))
        .host_function(PlannedPureHostFunction::new(
            "demo/left-branch",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(LegacyValue::F64(Ref::new(
                    host_f64_argument(&args[0]) + 100.0,
                ))))
            },
            move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                left_host_calls.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(LegacyValue::F64(Ref::new(
                    host_f64_argument(&args[0]) + 100.0,
                ))))
            },
        ))
        .unwrap()
        .host_function(PlannedPureHostFunction::new(
            "demo/right-branch",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(LegacyValue::F64(Ref::new(
                    host_f64_argument(&args[0]) + 200.0,
                ))))
            },
            move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                right_host_calls.fetch_add(1, Ordering::SeqCst);
                Ok(snapshot(LegacyValue::F64(Ref::new(
                    host_f64_argument(&args[0]) + 200.0,
                ))))
            },
        ))
        .unwrap()
        .build()
        .unwrap();
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "left");
    grant_read(&mut runtime, TEST_CLOCK_BASE_URI, "right");
    grant_host_call(&mut runtime, CapabilityId(91), "demo/left-branch");
    grant_host_call(&mut runtime, CapabilityId(92), "demo/right-branch");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(left), :read(right)}\nleft-output := demo/left-branch(@pulse/left)\nright-output := demo/right-branch(@pulse/right)").unwrap();
    let left_calls_before = left_calls.load(Ordering::SeqCst);
    let right_calls_before = right_calls.load(Ordering::SeqCst);
    assert_eq!(f64_value(&symbol_value(&runtime, "left-output")), 101.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "right-output")), 202.0);
    let left_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "left").unwrap();
    let right_source = RuntimeHostInputSource::new(TEST_CLOCK_BASE_URI, "right").unwrap();
    let plan = runtime.program.interpreter().plan();
    let left_consumers = plan
        .borrow()
        .reactive_consumers_for(source_cell(&runtime, &left_source))
        .to_vec();
    let right_consumers = plan
        .borrow()
        .reactive_consumers_for(source_cell(&runtime, &right_source))
        .to_vec();
    drop(plan);
    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            left_source,
            RuntimeHostInputValue::F64(10.0),
        ))
        .unwrap();
    let program_turn = outcome.turn.as_ref().unwrap();
    assert_eq!(program_turn.interpreter_turns.len(), 1);
    let reactive_turn = &program_turn.interpreter_turns[0].turn;
    assert_eq!(left_calls.load(Ordering::SeqCst), left_calls_before + 1);
    assert_eq!(right_calls.load(Ordering::SeqCst), right_calls_before);
    assert!(reactive_turn.register_commit.staged_nodes.is_empty());
    assert!(reactive_turn.register_commit.committed_nodes.is_empty());
    assert!(reactive_turn.register_commit.dirty_cells.is_empty());
    assert!(
        left_consumers
            .iter()
            .any(|id| reactive_turn.before_commit.executed_nodes.contains(id))
    );
    assert!(
        !right_consumers
            .iter()
            .any(|id| reactive_turn.before_commit.executed_nodes.contains(id))
    );
    assert_eq!(f64_value(&symbol_value(&runtime, "left-output")), 110.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "right-output")), 202.0);
}

#[test]
fn live_host_string_output_recomputes_without_replacing_reference() {
    let host = DeterministicHostFunction::new(
        "demo/live-label",
        |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
            Ok(LegacyValue::String(Ref::new(format!(
                "tick:{}",
                host_f64_argument(&args[0])
            ))))
        },
        |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
            Ok(LegacyValue::String(Ref::new(format!(
                "tick:{}",
                host_f64_argument(&args[0])
            ))))
        },
    );
    let mut runtime =
        test_runtime_with_host(test_provider_with("test://clock/ticks", "value", 1.0), host);
    grant_read(&mut runtime, "test://clock/ticks", "value");
    grant_host_call(&mut runtime, CapabilityId(45), "demo/live-label");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-label(@pulse/value)",
        )
        .unwrap();
    let before = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("output"))
        .unwrap();
    let outer_pointer = before.as_ptr();
    let inner_pointer = match &*before.borrow() {
        LegacyValue::String(value) => value.as_ptr(),
        other => panic!("expected string, got {other:?}"),
    };
    assert_eq!(string_value(&before.borrow()), "tick:1");

    runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();

    let after = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("output"))
        .unwrap();
    assert_eq!(outer_pointer, after.as_ptr());
    match &*after.borrow() {
        LegacyValue::String(value) => {
            assert_eq!(inner_pointer, value.as_ptr());
            assert_eq!(&*value.borrow(), "tick:9");
        }
        other => panic!("expected string, got {other:?}"),
    }
}

#[test]
fn live_host_output_kind_change_preserves_previous_output() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let mut runtime = test_runtime_with_host(
        test_provider_with("test://clock/ticks", "value", 1.0),
        PlannedPureHostFunction::new(
            "demo/kind-change",
            |_context: &RuntimeCallContext, args: &[RuntimeValueSnapshot]| {
                Ok(snapshot(LegacyValue::F64(Ref::new(
                    host_f64_argument(&args[0]) + 1.0,
                ))))
            },
            move |_context: &RuntimeCallContext, args: Vec<RuntimeValueSnapshot>| {
                let invocation = host_calls.fetch_add(1, Ordering::SeqCst);
                if invocation == 0 {
                    return Ok(snapshot(LegacyValue::F64(Ref::new(
                        host_f64_argument(&args[0]) + 1.0,
                    ))));
                }
                Ok(snapshot(LegacyValue::String(Ref::new(
                    "bad-kind".to_string(),
                ))))
            },
        ),
    );
    grant_read(&mut runtime, "test://clock/ticks", "value");
    grant_host_call(&mut runtime, CapabilityId(46), "demo/kind-change");
    let mut context = runtime.runtime_context().unwrap();
    runtime.run_string_with_context(&mut context, "@pulse := test://clock/ticks{:read(value)}\nhost-result := demo/kind-change(@pulse/value)\noutput := host-result").unwrap();
    assert_eq!(f64_value(&symbol_value(&runtime, "host-result")), 2.0);
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
    let host_result = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("host-result"))
        .unwrap();
    let host_result_inner = match &*host_result.borrow() {
        LegacyValue::F64(value) => value.as_ptr(),
        other => panic!("expected f64 host result, got {other:?}"),
    };

    let error = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap_err();
    let rendered = format!("{error:?}");
    assert!(
        rendered.contains("StableValueUpdateContractViolation"),
        "{rendered}"
    );
    assert!(calls.load(Ordering::SeqCst) >= 1);
    let host_result = runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str("host-result"))
        .unwrap();
    match &*host_result.borrow() {
        LegacyValue::F64(value) => {
            assert_eq!(host_result_inner, value.as_ptr());
            assert_eq!(*value.borrow(), 2.0);
        }
        other => panic!("expected f64 host result, got {other:?}"),
    }
    assert_eq!(f64_value(&symbol_value(&runtime, "output")), 2.0);
    runtime.run_string("recovery := 1").unwrap();
}

#[test]
fn live_host_empty_output_can_recompute() {
    let calls = Arc::new(AtomicUsize::new(0));
    let host_calls = calls.clone();
    let mut runtime = test_runtime_with_host(
        test_provider_with("test://clock/ticks", "value", 1.0),
        PlannedPureHostFunction::new(
            "demo/live-empty",
            |_context: &RuntimeCallContext, _args: &[RuntimeValueSnapshot]| {
                Ok(RuntimeValueSnapshot::empty())
            },
            move |_context: &RuntimeCallContext, _args: Vec<RuntimeValueSnapshot>| {
                host_calls.fetch_add(1, Ordering::SeqCst);
                Ok(RuntimeValueSnapshot::empty())
            },
        ),
    );
    grant_read(&mut runtime, "test://clock/ticks", "value");
    grant_host_call(&mut runtime, CapabilityId(48), "demo/live-empty");
    let mut context = runtime.runtime_context().unwrap();
    runtime
        .run_string_with_context(
            &mut context,
            "@pulse := test://clock/ticks{:read(value)}\noutput := demo/live-empty(@pulse/value)",
        )
        .unwrap();
    assert_eq!(symbol_value(&runtime, "output"), LegacyValue::Empty);

    let outcome = runtime
        .apply_host_input(RuntimeHostInput::single(
            RuntimeHostInputSource::new("test://clock/ticks", "value").unwrap(),
            RuntimeHostInputValue::F64(9.0),
        ))
        .unwrap();
    assert!(outcome.turn.is_some());
    assert!(calls.load(Ordering::SeqCst) >= 1);
    assert_eq!(symbol_value(&runtime, "output"), LegacyValue::Empty);
}
