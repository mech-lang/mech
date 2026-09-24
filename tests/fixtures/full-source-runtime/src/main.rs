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
    if let Some(path) = std::env::var_os("MECH_BROWSER_BUNDLE_FIXTURES") {
        write_browser_bundle_fixtures(std::path::Path::new(&path));
    }
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
                .compile_canonical_root_with_options(
                    mech_runtime::SourceRequest::new("main.mec"),
                    options,
                )
                .unwrap(),
            false,
        ),
        (
            compiler
                .compile_canonical_interactive_root_with_options(
                    mech_runtime::SourceRequest::new("main.mec"),
                    options,
                )
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
            .compile_canonical_root_with_options(
                mech_runtime::SourceRequest::new("missing.mec"),
                options,
            )
            .is_err()
    );
    assert!(
        compiler
            .compile_canonical_root_with_options(
                mech_runtime::SourceRequest::new("main.mec"),
                options,
            )
            .is_ok()
    );
}

fn write_browser_bundle_fixtures(directory: &std::path::Path) {
    use mech_runtime::{
        BrowserDocumentPayload, InMemorySourceResolver, ResolvedSource, SourceDocument,
        SourceRequest,
    };
    use std::{collections::BTreeMap, sync::Arc};
    std::fs::create_dir_all(directory).unwrap();
    for (name, source, dependency) in [
        ("plain", "~answer := 0\nanswer += 2\nanswer\n", None),
        ("replacement", "~answer := 0\nanswer += 3\nanswer\n", None),
        ("capture", "answer := 41\nanswer\n", None),
        (
            "capture-fenced",
            "~~~mech\nanswer := 41\nanswer\n~~~\n",
            None,
        ),
        (
            "rich",
            "~answer := 0\nanswer += 2 -- **Count** [docs](https://mech-lang.org) `literal` {{answer + 99}}: {ans}, {ans + 1}.\nanswer\n\nA paragraph with *emphasis* and [documentation](https://mech-lang.org).\n\nAnother paragraph displays {answer}.\n\nanswer\n",
            None,
        ),
        (
            "rich-fenced",
            "~~~mech\n~answer := 0\nanswer += 2 -- **Count** [docs](https://mech-lang.org) `literal` {{answer + 99}}: {ans}, {ans + 1}.\nanswer\n~~~\n\nA paragraph with *emphasis* and [documentation](https://mech-lang.org).\n\nAnother paragraph displays {answer}.\n\nanswer\n",
            None,
        ),
        (
            "imported",
            "+> ./dep.mec\n~answer := 0\nanswer += dep/value\nanswer\n",
            Some("value := 2\n<+ value\n"),
        ),
    ] {
        let uri = "bundle:///document.mec";
        let document = SourceDocument::parse_resolved(
            uri,
            mech_syntax::document::Revision(0),
            Arc::<str>::from(source),
            Default::default(),
        )
        .unwrap();
        let mut resolver = InMemorySourceResolver::new();
        resolver
            .insert_source(
                uri,
                ResolvedSource::new(uri, uri, mech_core::MechSourceCode::String(source.into()))
                    .with_source_document(document.clone())
                    .unwrap()
                    .admit_canonical_document()
                    .unwrap(),
            )
            .unwrap();
        let mut sources = BTreeMap::from([("document.mec", source)]);
        if let Some(text) = dependency {
            let dependency_uri = "bundle:///dep.mec";
            let dependency_document = SourceDocument::parse_resolved(
                dependency_uri,
                mech_syntax::document::Revision(0),
                Arc::<str>::from(text),
                Default::default(),
            )
            .unwrap();
            resolver
                .insert_source(
                    dependency_uri,
                    ResolvedSource::new(
                        dependency_uri,
                        dependency_uri,
                        mech_core::MechSourceCode::String(text.into()),
                    )
                    .with_source_document(dependency_document)
                    .unwrap()
                    .admit_canonical_document()
                    .unwrap(),
                )
                .unwrap();
            resolver
                .insert_resolution(uri, "./dep.mec", dependency_uri)
                .unwrap();
            sources.insert("dep.mec", text);
        }
        let mut compiler = RuntimeBuilder::new()
            .function_catalog(mech_stdlib::source_catalog())
            .source_resolver(resolver)
            .build_compiler()
            .unwrap();
        let product = compiler
            .compile_canonical_interactive_root(SourceRequest::new(uri))
            .unwrap();
        let revision = product
            .artifact()
            .revision()
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        let encoded = BrowserDocumentPayload::new("document.mec", source)
            .unwrap()
            .with_presentation_output_ids(
                mech_runtime::canonical_document_presentation_output_ids(&document.document())
                    .unwrap(),
            )
            .encode()
            .unwrap();
        let html = mech_runtime::CanonicalDocumentRenderer
            .format_browser_html(&document.document())
            .unwrap();
        let value = serde_json::json!({"encoded": encoded, "revision": revision, "sources": sources, "html": html});
        std::fs::write(
            directory.join(format!("{name}.json")),
            serde_json::to_vec(&value).unwrap(),
        )
        .unwrap();
    }
}
