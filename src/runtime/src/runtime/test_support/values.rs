use mech_core::{Value, hash_str};

use super::super::MechRuntime;
use crate::{HostArgumentValue, RuntimeHostInputSource};

pub(crate) fn f64_value(value: &Value) -> f64 {
    match value {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected f64, got {other:?}"),
    }
}

pub(crate) fn bool_value(value: &Value) -> bool {
    match value {
        Value::Bool(value) => *value.borrow(),
        other => panic!("expected bool, got {other:?}"),
    }
}

pub(crate) fn string_value(value: &Value) -> String {
    match value {
        Value::String(value) => value.borrow().clone(),
        other => panic!("expected string, got {other:?}"),
    }
}

pub(crate) fn host_f64_argument(value: &impl HostArgumentValue) -> f64 {
    match value.host_argument_value() {
        Value::F64(value) => *value.borrow(),
        Value::MutableReference(value) => match &*value.borrow() {
            Value::F64(value) => *value.borrow(),
            other => panic!("expected f64 mutable reference, got {other:?}",),
        },
        other => panic!("expected f64 host argument, got {other:?}"),
    }
}

pub(crate) fn symbol_value(runtime: &MechRuntime, name: &str) -> Value {
    runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(hash_str(name))
        .unwrap_or_else(|| panic!("missing symbol {name}"))
        .borrow()
        .clone()
}

pub(crate) fn source_value(runtime: &MechRuntime, source: &RuntimeHostInputSource) -> Value {
    let input = runtime
        .live_input_bindings
        .get(source)
        .and_then(|inputs| inputs.first())
        .unwrap_or_else(|| {
            panic!(
                "missing binding for {} / {}",
                source.base_uri(),
                source.path(),
            )
        });
    runtime
        .program
        .interpreter()
        .symbols()
        .borrow()
        .get(input.symbol_id)
        .unwrap_or_else(|| panic!("missing symbol {}", input.symbol_id))
        .borrow()
        .clone()
}
