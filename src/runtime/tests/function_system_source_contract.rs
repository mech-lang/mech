use std::collections::BTreeSet;

use mech_core::Value;
#[cfg(feature = "linked_stdlib")]
use mech_runtime::{InMemorySourceResolver, ModuleBuildOptions, SourceRequest};
use mech_runtime::{RuntimeBuilder, RuntimeValueSnapshot};
use serde::Deserialize;

const SOURCE_CASES: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../tests/architecture/function-system/source-cases.json"
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

fn corpus() -> SourceCorpus {
    let corpus: SourceCorpus =
        serde_json::from_str(SOURCE_CASES).expect("shared source corpus must be valid JSON");
    assert_eq!(corpus.schema, 1, "unsupported shared source corpus schema");

    let mut names = BTreeSet::new();
    for case in corpus
        .cross_target
        .iter()
        .chain(corpus.native_modules.iter())
    {
        assert!(
            names.insert(case.name.as_str()),
            "duplicate shared source case name `{}`",
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

#[test]
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

#[test]
#[cfg(feature = "linked_stdlib")]
fn native_module_source_contract() {
    for case in &corpus().native_modules {
        let root_specifier = format!("function-system-contract-{}.mec", case.name);
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_string(&root_specifier, &case.source)
            .unwrap_or_else(|error| {
                panic!("failed to register source for `{}`: {error:?}", case.name)
            });

        let mut runtime = RuntimeBuilder::new()
            .source_resolver(resolver)
            .build()
            .unwrap_or_else(|error| {
                panic!("failed to build runtime for `{}`: {error:?}", case.name)
            });

        runtime
            .resolve_and_run_root_module(
                SourceRequest::new(&root_specifier),
                ModuleBuildOptions::new(env!("CARGO_PKG_VERSION"), "v0.3", "native", &[], &[]),
            )
            .unwrap_or_else(|error| panic!("module source case `{}` failed: {error:?}", case.name));

        let snapshot = runtime.root_symbol_value("result").unwrap_or_else(|error| {
            panic!(
                "module source case `{}` did not define `result`: {error:?}",
                case.name
            )
        });
        assert_expected(case, snapshot);
    }
}
