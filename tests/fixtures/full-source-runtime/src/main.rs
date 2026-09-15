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
    rooted_source_canary();
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
                panic!(
                    "source case `{}` failed resident admission: {error:?}",
                    case.name
                )
            })
            .initial_value
            .into_value();
        assert_expected(case, actual);
    }
    println!(
        "source product probe passed: {} catalog cases and rooted source canary",
        names.len()
    );
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
                *actual, *expected,
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

fn rooted_source_canary() {
    let mut resolver = mech_runtime::InMemorySourceResolver::new();
    resolver
        .insert_canonical_string("dep.mec", "value := 41.0\n<+ value\n")
        .unwrap();
    resolver
        .insert_canonical_string(
            "main.mec",
            "+> ./dep.mec\nanswer := dep/value + 1.0\nanswer\n",
        )
        .unwrap();
    resolver
        .insert_canonical_string("missing.mec", "+> ./absent.mec\nanswer := absent/value\n")
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .source_resolver(resolver)
        .build_compiler()
        .unwrap();
    let options = mech_runtime::ModuleBuildOptions::new(
        "source-fixture",
        "v0.4",
        "native",
        &["full_source"],
        &[],
    );
    for (product, interactive) in [
        (
            compiler
                .compile_root(mech_runtime::SourceRequest::new("main.mec"), options)
                .unwrap(),
            false,
        ),
        (
            compiler
                .compile_interactive_root(mech_runtime::SourceRequest::new("main.mec"), options)
                .unwrap(),
            true,
        ),
    ] {
        let mut runtime = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .build()
            .unwrap();
        runtime
            .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)
            .unwrap();
        let value = runtime
            .output_value(mech_core::OutputId::new(0))
            .unwrap()
            .unwrap();
        assert!(matches!(value.value().data(), ValueData::F64(value) if value.to_f64() == 42.0));
        assert_eq!(
            runtime.root_symbol_output_id("answer").is_some(),
            interactive
        );
        assert_eq!(product.source_dependencies().len(), 1);
    }
    assert!(
        compiler
            .compile_root(mech_runtime::SourceRequest::new("missing.mec"), options)
            .is_err()
    );
    assert!(
        compiler
            .compile_root(mech_runtime::SourceRequest::new("main.mec"), options)
            .is_ok()
    );
}
