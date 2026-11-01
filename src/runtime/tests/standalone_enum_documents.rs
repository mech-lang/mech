#![cfg(all(feature = "full_source", feature = "resident-routing-source"))]

use mech_core::{CanonicalNominalPath, NominalKey, SchemaBody, Value};
use mech_runtime::{
    CanonicalProgramBundle, ResidentDurabilityPolicy, RuntimeBuilder, SourceDocument,
};
use mech_syntax::document::{ParseConfig, Revision};

const SOURCE: &str = "<color> := :red | :green | :blue\nmy-color<color> := :red\n";

fn parsed(uri: &str, source: &str) -> SourceDocument {
    SourceDocument::parse_resolved(uri, Revision(0), source, ParseConfig::default()).unwrap()
}

fn execute(document: &SourceDocument) -> Value {
    RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .unwrap()
        .load_document_program(document, ResidentDurabilityPolicy::Volatile)
        .unwrap()
        .initial_value
        .to_value()
}

fn enum_key(value: &Value) -> NominalKey {
    let schemas = value.schemas().unwrap();
    let SchemaBody::Enum { key, variants } = schemas.get(value.schema()).unwrap().body() else {
        panic!("the typed enum must retain its declared schema")
    };
    let enumeration = value.enum_view().unwrap();
    assert_eq!(variants[enumeration.ordinal() as usize].name, "red");
    assert!(enumeration.payload().is_none());
    *key
}

#[test]
fn standalone_enums_agree_across_direct_compiler_runtime_and_bytecode() {
    let document = parsed("memory:colors", SOURCE).with_standalone_nominal_origin();
    let direct = document
        .canonical_frontend()
        .compile_document_with_catalog(&document.document(), mech_stdlib::source_catalog())
        .unwrap()
        .compile_artifact()
        .unwrap();
    let mut compiler = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build_compiler()
        .unwrap();
    let product = compiler.compile_document(&document).unwrap();
    assert_eq!(direct.revision(), product.artifact().revision());

    let accepted = execute(&document);
    let decoded = RuntimeBuilder::new()
        .function_catalog(mech_stdlib::source_catalog())
        .build()
        .unwrap()
        .load_bytecode_program(product.bytecode(), ResidentDurabilityPolicy::Volatile)
        .unwrap()
        .initial_value
        .to_value();
    assert_eq!(enum_key(&accepted), enum_key(&decoded));
    assert_eq!(accepted.schema_key(), decoded.schema_key());

    let bundle =
        CanonicalProgramBundle::from_product("memory:colors", &document, &product).unwrap();
    assert_eq!(
        bundle.root_nominal_origin.as_ref(),
        document.nominal_origin()
    );
    let restored = parsed("memory:restored-colors", &bundle.source)
        .with_nominal_origin(bundle.root_nominal_origin.unwrap());
    assert_eq!(enum_key(&execute(&restored)), enum_key(&accepted));
}

#[test]
fn standalone_enum_owners_are_distinct_and_survive_recompilation() {
    // Identical source and transport URI do not conflate independent documents.
    let first = parsed("memory:colors", SOURCE).with_standalone_nominal_origin();
    let second = parsed("memory:colors", SOURCE).with_standalone_nominal_origin();
    let first_key = enum_key(&execute(&first));
    assert_ne!(first_key, enum_key(&execute(&second)));
    assert_eq!(
        first_key,
        enum_key(&execute(&first.clone().with_standalone_nominal_origin()))
    );
    let edited = parsed("memory:renamed-colors", &format!("-- edited\n{SOURCE}"))
        .with_nominal_origin(first.nominal_origin().unwrap().clone());
    assert_eq!(first_key, enum_key(&execute(&edited)));
}

#[test]
fn standalone_admission_preserves_explicit_package_provenance() {
    let origin = CanonicalNominalPath::new(vec!["palette".into(), "colors".into()]).unwrap();
    let document = parsed("memory:colors", SOURCE)
        .with_nominal_origin(origin.clone())
        .with_nominal_package_id("palette@1.0.0");
    let admitted = document.clone().with_standalone_nominal_origin();
    assert_eq!(admitted.nominal_origin(), Some(&origin));
    assert_eq!(admitted.nominal_package_id(), Some("palette@1.0.0"));
    assert_eq!(enum_key(&execute(&admitted)), enum_key(&execute(&document)));

    // Parsing a resolver-owned source is not permission to invent its package.
    let unowned = parsed("memory:unresolved-package", SOURCE);
    let error = unowned
        .canonical_frontend()
        .compile_document(&unowned.document())
        .err()
        .unwrap();
    assert_eq!(error.code, "source-semantics/nominal-origin-required");
}
