use std::collections::BTreeSet;

use mech_core::LegacyValue;
use mech_engine::{MechProgram, MechProgramConfig};
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
    assert_eq!(corpus.cross_target.len(), 8);
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

        let mut program = MechProgram::with_function_catalog(
            MechProgramConfig::default(),
            mech_stdlib::source_catalog(),
        );
        let actual = program
            .run_string(&case.source)
            .unwrap_or_else(|error| panic!("source case `{}` failed: {error:?}", case.name));
        assert_expected(case, actual);
    }
}

fn dereference(value: LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => dereference(reference.borrow().clone()),
        LegacyValue::Typed(value, _) => dereference(*value),
        value => value,
    }
}

fn assert_expected(case: &SourceCase, actual: LegacyValue) {
    match (&case.expected, dereference(actual)) {
        (
            ExpectedValue::F64 {
                value: expected,
                tolerance,
            },
            LegacyValue::F64(actual),
        ) => {
            let actual = *actual.borrow();
            let tolerance = tolerance.unwrap_or(0.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "source case `{}` expected f64 {expected} with tolerance {tolerance}, got {actual}",
                case.name,
            );
        }
        (ExpectedValue::Bool { value: expected }, LegacyValue::Bool(actual)) => {
            assert_eq!(
                *actual.borrow(),
                *expected,
                "source case `{}` returned the wrong bool",
                case.name,
            );
        }
        (ExpectedValue::String { value: expected }, LegacyValue::String(actual)) => {
            assert_eq!(
                &*actual.borrow(),
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
