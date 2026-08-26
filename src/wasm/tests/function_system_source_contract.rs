use std::collections::BTreeSet;

use mech_core::{LegacyValue, OperationId, RuntimeFunctionId};
use mech_runtime::{ResidentDurabilityPolicy, RuntimeBuilder, RuntimeValueSnapshot};
use mech_stdlib::source_catalog;
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

fn dereference(value: LegacyValue) -> LegacyValue {
    match value {
        LegacyValue::MutableReference(reference) => dereference(reference.borrow().clone()),
        LegacyValue::Typed(value, _) => dereference(*value),
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
            LegacyValue::F64(actual),
        ) => {
            let actual = *actual.borrow();
            let tolerance = tolerance.unwrap_or(0.0);
            assert!(
                (actual - expected).abs() <= tolerance,
                "source case `{}` expected f64 {expected} with tolerance {tolerance}, got {actual}",
                case.name
            );
        }
        (ExpectedValue::Bool { value: expected }, LegacyValue::Bool(actual)) => {
            assert_eq!(
                *actual.borrow(),
                *expected,
                "source case `{}` returned the wrong bool",
                case.name
            );
        }
        (ExpectedValue::String { value: expected }, LegacyValue::String(actual)) => {
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

fn assert_f64_snapshot(snapshot: RuntimeValueSnapshot, expected: f64) {
    let actual = dereference(snapshot.into_value());
    let LegacyValue::F64(actual) = actual else {
        panic!("expected f64 {expected}, got {actual:?}");
    };
    assert_eq!(*actual.borrow(), expected);
}

fn browser_runtime_builder() -> RuntimeBuilder {
    RuntimeBuilder::new().function_catalog(source_catalog())
}

#[wasm_bindgen_test]
fn cross_target_source_contract() {
    for case in &corpus().cross_target {
        let mut runtime = browser_runtime_builder().build().unwrap_or_else(|error| {
            panic!("failed to build runtime for `{}`: {error:?}", case.name)
        });
        let snapshot = runtime
            .load_source_program(&case.source, ResidentDurabilityPolicy::Volatile)
            .unwrap_or_else(|error| panic!("source case `{}` failed: {error:?}", case.name));
        assert_expected(case, snapshot.initial_value);
    }
}

#[wasm_bindgen_test]
fn enabled_standard_profile_is_fully_catalog_owned() {
    let catalog = source_catalog();

    assert!(catalog.specializer_count() > 1);
    assert!(catalog.intrinsic_specializer_count() > 0);
    assert!(catalog.runtime_factory_count() > 56);
    assert!(catalog.module_export("math", "add").is_none());
    for canonical_name in [
        "math/add",
        "compare/eq",
        "logic/and",
        "range/inclusive",
        "matrix/transpose",
        "set/union",
        "string/concat",
    ] {
        assert_eq!(
            catalog
                .specializer(OperationId::from_name(canonical_name))
                .unwrap()
                .canonical_name,
            canonical_name,
        );
    }
    assert!(
        catalog
            .runtime_entry(RuntimeFunctionId::from_name("AddSS<f64>"))
            .is_some()
    );
}

#[cfg(feature = "browser_compute")]
#[wasm_bindgen_test]
fn browser_compute_feature_closure_includes_ceil() {
    assert!(source_catalog().module_export("math", "ceil").is_some());
}

#[wasm_bindgen_test]
fn scalar_source_addition_uses_the_explicit_catalog() {
    let mut runtime = browser_runtime_builder()
        .build()
        .expect("standard WASM runtime must build");

    let snapshot = runtime
        .load_source_program("1.0 + 2.0", ResidentDurabilityPolicy::Volatile)
        .expect("scalar source addition must specialize through the catalog");

    assert_f64_snapshot(snapshot.initial_value, 3.0);
}
