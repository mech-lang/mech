use std::cell::RefCell;
use std::sync::Arc;

use mech_core::{Ref, Value};
use mech_runtime::{
    BasicCapability, BasicOperation, BasicResource, BasicSubject, CapabilityId, MechRuntime,
    ModuleBuildOptions, PlannedPureHostFunction, RuntimeCallContext, RuntimeValueSnapshot,
};

thread_local! {
  static RETAINED_HOST_ARGUMENT: RefCell<Option<RuntimeValueSnapshot>> =
    const { RefCell::new(None) };
  static RETAINED_HOST_RESULT: RefCell<Option<RuntimeValueSnapshot>> =
    const { RefCell::new(None) };
}

fn f64_value(value: &Value) -> f64 {
    match value {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected f64 snapshot, got {other:?}"),
    }
}

fn mutate_f64(value: &Value, next: f64) {
    match value {
        Value::F64(value) => *value.borrow_mut() = next,
        other => panic!("expected mutable f64 snapshot, got {other:?}"),
    }
}

fn module_options() -> ModuleBuildOptions<'static> {
    ModuleBuildOptions::new("test", "v0.3", "native", &[], &[])
}

fn grant_host_call(runtime: &mut MechRuntime, name: &str) {
    let subject = runtime.runtime_context().unwrap().subject().to_string();
    runtime
        .grant_capability(Arc::new(BasicCapability::new(
            CapabilityId(90_001),
            &BasicSubject::new(subject),
            &BasicResource::new(format!("host:{name}")),
            [BasicOperation::new("call")],
        )))
        .unwrap();
}

#[test]
fn mutating_runtime_value_snapshot_does_not_change_runtime_output() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let snapshot = runtime.run_string("sealed-value := 41.0").unwrap();

    mutate_f64(&snapshot.to_value(), 99.0);

    assert_eq!(
        f64_value(
            &runtime
                .root_symbol_value("sealed-value")
                .unwrap()
                .to_value(),
        ),
        41.0,
    );
}

#[test]
fn mutating_module_result_snapshot_does_not_change_module_exports() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    let version = runtime
        .put_source_module(
            "sealed-module",
            "memory://sealed-module.mec",
            "value := 41.0\n<+ value\nvalue",
            module_options(),
        )
        .unwrap();
    let first = runtime.run_module(version).unwrap();

    mutate_f64(&first.exports["value"].to_value(), 99.0);
    mutate_f64(&first.result.to_value(), 100.0);

    let second = runtime.run_module(version).unwrap();
    assert_eq!(f64_value(&second.exports["value"].to_value()), 41.0,);
    assert_eq!(f64_value(&second.result.to_value()), 41.0);
}

#[test]
fn retained_host_snapshots_cannot_mutate_program_inputs_or_outputs() {
    RETAINED_HOST_ARGUMENT.with(|slot| slot.borrow_mut().take());
    RETAINED_HOST_RESULT.with(|slot| slot.borrow_mut().take());

    let function = PlannedPureHostFunction::new(
        "sealed/snapshot",
        |_context: &RuntimeCallContext, _arguments: &[RuntimeValueSnapshot]| {
            RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(5.0)))
        },
        |_context: &RuntimeCallContext, arguments: Vec<RuntimeValueSnapshot>| {
            RETAINED_HOST_ARGUMENT.with(|slot| {
                *slot.borrow_mut() = arguments.first().cloned();
            });
            let result = RuntimeValueSnapshot::try_capture(&Value::F64(Ref::new(5.0)))?;
            RETAINED_HOST_RESULT.with(|slot| {
                *slot.borrow_mut() = Some(result.clone());
            });
            Ok(result)
        },
    );
    let mut runtime = MechRuntime::builder()
        .host_function(function)
        .unwrap()
        .build()
        .unwrap();
    grant_host_call(&mut runtime, "sealed/snapshot");
    runtime
        .run_string("sealed-input := 1.0\nsealed-output := sealed/snapshot(sealed-input)")
        .unwrap();

    RETAINED_HOST_ARGUMENT.with(|slot| {
        mutate_f64(&slot.borrow().as_ref().unwrap().to_value(), 101.0);
    });
    RETAINED_HOST_RESULT.with(|slot| {
        mutate_f64(&slot.borrow().as_ref().unwrap().to_value(), 202.0);
    });

    assert_eq!(
        f64_value(
            &runtime
                .root_symbol_value("sealed-input")
                .unwrap()
                .to_value(),
        ),
        1.0,
    );
    assert_eq!(
        f64_value(
            &runtime
                .root_symbol_value("sealed-output")
                .unwrap()
                .to_value(),
        ),
        5.0,
    );
}

#[test]
fn runtime_value_snapshot_raw_access_is_sealed() {
    let tests = trybuild::TestCases::new();
    tests.compile_fail("tests/ui/sealed/runtime_value_snapshot_raw_access.rs");
}
