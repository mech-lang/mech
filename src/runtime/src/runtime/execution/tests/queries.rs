use super::super::{LegacyValue, MechRuntime, RuntimeConfig};
use mech_core::hash_str;
use mech_syntax::parser;

fn f64_value(value: &LegacyValue) -> f64 {
    match value {
        LegacyValue::F64(value) => *value.borrow(),
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
    let output = runtime
        .output_value_for_interpreter(root_id, output_id)
        .expect("output snapshot should succeed");
    assert!(output.is_some());
}

#[test]
fn runtime_run_tree_defers_inline_document_expression_in_the_formatter_root_namespace() {
    let tree = parser::parse("The document evaluates {answer} inline.\n\nanswer := 41").unwrap();
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .config(RuntimeConfig::default())
        .build()
        .unwrap();

    runtime.run_tree(&tree).unwrap();

    let root_id = runtime.program().interpreter().id;
    assert_ne!(root_id, 0, "the runtime root has a physical ID");
    let output_id = hash_str("inline-eval:0:0");
    let output = runtime
        .output_value_for_interpreter(root_id, output_id)
        .unwrap()
        .expect("expected inline document output");
    assert_eq!(f64_value(&output.to_value()), 41.0);
}

#[test]
fn runtime_delegates_root_symbol_value() {
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .build()
        .unwrap();
    runtime.run_string("answer := 42.0").unwrap();
    assert_eq!(
        f64_value(&runtime.root_symbol_value("answer").unwrap().to_value(),),
        42.0,
    );
}

#[test]
fn runtime_delegates_root_symbol_values() {
    let mut runtime = crate::runtime::test_support::providers::test_runtime_builder()
        .build()
        .unwrap();
    runtime.run_string("a := 1.0\nb := 2.0").unwrap();
    let rows = runtime.root_symbol_values(&["b", "a"]).unwrap();
    assert_eq!(rows[0].0, "b");
    assert_eq!(f64_value(&rows[0].1.to_value()), 2.0);
    assert_eq!(rows[1].0, "a");
    assert_eq!(f64_value(&rows[1].1.to_value()), 1.0);
}

#[test]
fn runtime_named_interpreter_lookup_uses_retained_document_metadata() {
    let mut runtime = MechRuntime::builder().build().unwrap();
    runtime.run_string("~~~mech:foo\nanswer := 7\n~~~").unwrap();

    let id = runtime.interpreter_id_by_name("foo").unwrap();
    assert_eq!(id, Some(mech_core::hash_str("foo")));
    assert!(runtime.has_interpreter(id.unwrap()));
    assert_eq!(runtime.interpreter_id_by_name("missing").unwrap(), None);
}
