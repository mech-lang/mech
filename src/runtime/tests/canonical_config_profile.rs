#![cfg(feature = "source")]

use mech_syntax::document::{DocumentId, ParseConfig, Revision, TextSnapshot};

// Run the maintained behavioral suite through the prepared canonical product
// interface. Expectations remain literal configuration outcomes, not AST parity.
fn parse_config_document(
    name: impl Into<String>,
    source: &str,
    options: mech_runtime::ConfigProfileOptions,
) -> mech_core::MResult<mech_runtime::MechConfigDocument> {
    let source = mech_runtime::resolver::SourceDocument::parse(
        TextSnapshot::new(DocumentId(834), Revision(0), source).unwrap(),
        ParseConfig::default(),
    );
    mech_runtime::compile_config_document(name, &source, options)
}

include!("config_profile.rs");

#[test]
fn selected_config_fences_reject_imports_and_exports() {
    for declaration in ["+> ./dependency.mec", "<+ config"] {
        let source = format!("~~~mech:config\n{declaration}\nconfig := {{:}}\n~~~\n");
        assert!(err_text(&source).contains("not allowed in Mech config"));
    }
}

#[test]
fn configuration_keeps_non_config_owners_out_of_evaluation() {
    let mut source = String::from("config := {runtime: {name: \"root\"}}\n\n");
    for fence in ["text", "mech", "mech:false", "mech:other"] {
        source.push_str(&format!(
            "~~~{fence}\nconfig := {{runtime: {{name: \"ignored\"}}}}\n~~~\n\n"
        ));
    }
    if cfg!(feature = "mika") {
        source.push_str("╭◉╮⸢config := {runtime: {name: \"local\"}}\n⸥\n");
    }
    assert_eq!(
        parse(&source).unwrap().runtime.name.as_deref(),
        Some("root")
    );
}

#[test]
fn retained_config_rejects_malformed_and_resource_limited_documents() {
    use mech_runtime::resolver::SourceDocument;
    use mech_syntax::document::ParseLimits;
    for (text, fuel) in [
        ("config := {:}\nx := [1,,2]\n", u64::MAX),
        ("config := {:}\n", 1),
    ] {
        let source = SourceDocument::parse(
            TextSnapshot::new(DocumentId(835), Revision(3), text).unwrap(),
            ParseConfig {
                limits: ParseLimits {
                    fuel,
                    ..ParseLimits::default()
                },
            },
        );
        assert!(!source.is_strictly_clean());
        let error = mech_runtime::compile_config_document(
            "limited.mcfg",
            &source,
            ConfigProfileOptions::default(),
        )
        .unwrap_err();
        assert!(error.kind_message().contains("complete canonical source"));
        let syntax = error
            .kind_as::<mech_runtime::InvalidConfigSyntax>()
            .unwrap();
        assert_eq!(syntax.source_name, "limited.mcfg");
        assert!(std::ptr::eq(syntax.source.snapshot(), source.snapshot()));
        assert_eq!(syntax.source.source().document(), DocumentId(835));
        assert_eq!(syntax.source.source().revision(), Revision(3));
        for diagnostic in source.snapshot().diagnostics.iter() {
            assert!(error.kind_message().contains(&diagnostic.message));
            assert!(
                diagnostic
                    .primary
                    .resolve(Revision(3), &syntax.source.snapshot().nodes)
                    .is_some()
            );
        }
    }
}

#[test]
fn finalized_stream_compiles_configuration_without_reparsing_or_replacing_its_revision() {
    use mech_runtime::resolver::SourceDocument;
    use mech_syntax::document::{DocumentStream, StreamProgress};
    let text = "  config := {runtime: {name: \"café\"}}\r\n";
    let mut stream = DocumentStream::new(DocumentId(836), ParseConfig::default());
    for chunk in ["  config := {runtime: {name: \"caf", "é\"}}\r\n"] {
        stream.append(chunk, u64::MAX).unwrap();
    }
    assert_eq!(stream.finish(u64::MAX).progress, StreamProgress::Finished);
    let snapshot = stream.materialize().unwrap();
    let work = stream.work();
    let source = SourceDocument::from_finished_stream(&mut stream).unwrap();
    let result = mech_runtime::compile_config_document(
        "retained.mcfg",
        &source,
        ConfigProfileOptions::default(),
    )
    .unwrap();
    assert_eq!(result.source_name, "retained.mcfg");
    assert_eq!(result.runtime.name.as_deref(), Some("café"));
    assert!(std::ptr::eq(source.snapshot(), snapshot.as_ref()));
    assert_eq!(source.source().to_contiguous_string(), text);
    assert_eq!(source.source().revision(), snapshot.revision);
    assert_eq!(stream.work(), work);
}
