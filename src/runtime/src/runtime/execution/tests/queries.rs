use super::super::{MechRuntime, RuntimeConfig, Value};

fn f64_value(value: &Value) -> f64 {
    match value {
        Value::F64(value) => *value.borrow(),
        other => panic!("expected f64, got {other:?}"),
    }
}

#[test]
fn runtime_output_value_for_interpreter_returns_value_after_run_string() {
    let mut runtime = MechRuntime::new(RuntimeConfig::default()).unwrap();
    let source = "```mech
1
```";
    let _ = runtime.run_string(source).unwrap();
    let root_id = runtime.program().interpreter().id;
    let output_id = {
        let out_values = runtime.program().interpreter().out_values.borrow();
        *out_values
            .keys()
            .next()
            .expect("expected output value after run_string")
    };
    let output = runtime.output_value_for_interpreter(root_id, output_id);
    assert!(output.is_some());
}

#[test]
fn runtime_delegates_root_symbol_value() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("answer := 42.0").unwrap();
    assert_eq!(
        f64_value(runtime.root_symbol_value("answer").unwrap().as_value(),),
        42.0,
    );
}

#[test]
fn runtime_delegates_root_symbol_values() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("a := 1.0\nb := 2.0").unwrap();
    let rows = runtime.root_symbol_values(&["b", "a"]).unwrap();
    assert_eq!(rows[0].0, "b");
    assert_eq!(f64_value(rows[0].1.as_value()), 2.0);
    assert_eq!(rows[1].0, "a");
    assert_eq!(f64_value(rows[1].1.as_value()), 1.0);
}
