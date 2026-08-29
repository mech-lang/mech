use std::collections::BTreeSet;

use mech_core::{Value, ValueData};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder};
use serde::Deserialize;

const SOURCE_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../architecture/function-system/source-cases.json"
));

#[derive(Debug, Deserialize)]
struct SourceCorpus {
    schema: u32,
    cross_target: Vec<SourceCase>,
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

fn main() {
    let corpus: SourceCorpus =
        serde_json::from_str(SOURCE_CASES).expect("shared source corpus must be valid JSON");
    assert_eq!(corpus.schema, 1, "unsupported shared source corpus schema");
    assert_eq!(corpus.cross_target.len(), 9);
    assert_eq!(corpus.native_modules.len(), 5);

    let mut names = BTreeSet::new();
    for case in corpus
        .cross_target
        .iter()
        .chain(corpus.native_modules.iter())
    {
        assert!(
            names.insert(case.name.as_str()),
            "duplicate shared source case name `{}`",
            case.name,
        );

        let product = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build_compiler()
            .expect("source compiler construction failed")
            .compile_source(&case.source)
            .unwrap_or_else(|error| panic!("source case `{}` failed: {error:?}", case.name));
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::runtime_catalog())
            .build()
            .expect("resident runtime construction failed");
        let actual = runtime
            .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)
            .unwrap_or_else(|error| {
                panic!("source case `{}` failed resident admission: {error:?}", case.name)
            })
            .initial_value
            .into_value();
        assert_expected(case, actual);
    }
}

fn assert_expected(case: &SourceCase, actual: Value) {
    match (&case.expected, actual.data()) {
        (
            ExpectedValue::F64 {
                value: expected,
                tolerance,
            },
            ValueData::F64(actual),
        ) => {
            let actual = actual.to_f64();
            let tolerance = tolerance.unwrap_or(0.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "source case `{}` expected f64 {expected} with tolerance {tolerance}, got {actual}",
                case.name,
            );
        }
        (ExpectedValue::Bool { value: expected }, ValueData::Bool(actual)) => {
            assert_eq!(
                *actual,
                *expected,
                "source case `{}` returned the wrong bool",
                case.name,
            );
        }
        (ExpectedValue::String { value: expected }, ValueData::String(actual)) => {
            assert_eq!(
                actual.as_ref(),
                expected,
                "source case `{}` returned the wrong string",
                case.name,
            );
        }
        (expected, actual) => panic!(
            "source case `{}` expected {expected:?}, got {actual:?}",
            case.name,
        ),
    }
}
