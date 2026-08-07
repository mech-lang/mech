use std::collections::BTreeSet;

use mech_core::Value;
use mech_runtime::{RuntimeBuilder, RuntimeValueSnapshot};
use mech_wasm as _;
use serde::Deserialize;
use wasm_bindgen_test::*;

wasm_bindgen_test_configure!(run_in_browser);

const SOURCE_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/source-cases.json"
));

#[derive(Debug, Deserialize)]
struct SourceCorpus {
    schema: u32,
    cross_target: Vec<SourceCase>,
    #[allow(dead_code)]
    native_modules: Vec<SourceCase>,
}

#[derive(Debug, Deserialize)]
struct SourceCase {
    name: String,
    source: String,
    expected: ExpectedValue,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "kind")]
enum ExpectedValue {
    #[serde(rename = "f64")]
    F64 {
        value: f64,
        #[serde(default)]
        tolerance: Option<f64>,
    },
    #[serde(rename = "bool")]
    Bool { value: bool },
    #[serde(rename = "string")]
    String { value: String },
}

fn corpus() -> SourceCorpus {
    let corpus: SourceCorpus =
        serde_json::from_str(SOURCE_CASES).expect("shared source corpus must be valid JSON");
    assert_eq!(corpus.schema, 1, "unsupported shared source corpus schema");

    let mut names = BTreeSet::new();
    for case in &corpus.cross_target {
        assert!(
            names.insert(case.name.as_str()),
            "duplicate cross-target source case name `{}`",
            case.name
        );
    }

    corpus
}

fn dereference(value: Value) -> Value {
    match value {
        Value::MutableReference(reference) => dereference(reference.borrow().clone()),
        Value::Typed(value, _) => dereference(*value),
        value => value,
    }
}

fn assert_expected(case: &SourceCase, snapshot: RuntimeValueSnapshot) {
    let actual = dereference(snapshot.into_value());
    match (&case.expected, actual) {
        (
            ExpectedValue::F64 {
                value: expected,
                tolerance,
            },
            Value::F64(actual),
        ) => {
            let actual = *actual.borrow();
            let tolerance = tolerance.unwrap_or(0.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "source case `{}` expected f64 {expected} with tolerance {tolerance}, got {actual}",
                case.name
            );
        }
        (ExpectedValue::Bool { value: expected }, Value::Bool(actual)) => {
            assert_eq!(
                *actual.borrow(),
                *expected,
                "source case `{}` returned the wrong bool",
                case.name
            );
        }
        (ExpectedValue::String { value: expected }, Value::String(actual)) => {
            assert_eq!(
                &*actual.borrow(),
                expected,
                "source case `{}` returned the wrong string",
                case.name
            );
        }
        (expected, actual) => panic!(
            "source case `{}` expected {expected:?}, got {actual:?}",
            case.name
        ),
    }
}

#[wasm_bindgen_test]
fn cross_target_source_contract() {
    for case in &corpus().cross_target {
        let mut runtime = RuntimeBuilder::new().build().unwrap_or_else(|error| {
            panic!("failed to build runtime for `{}`: {error:?}", case.name)
        });
        let snapshot = runtime
            .run_string(&case.source)
            .unwrap_or_else(|error| panic!("source case `{}` failed: {error:?}", case.name));
        assert_expected(case, snapshot);
    }
}
