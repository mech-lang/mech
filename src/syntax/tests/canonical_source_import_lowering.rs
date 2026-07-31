use mech_core::TokenKind;
use mech_core::nodes::{ImportDeclaration, MechString};
use mech_syntax::document::ast::{ImportDeclarationSyntax, SourceImportSpecifierSyntax};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2f_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
    lower_legacy_import_declaration, lower_legacy_source_import_specifier,
};
use mech_syntax::{ParseString, graphemes, import_declaration};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(932), Revision(0), text).unwrap()
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn specifier(input: &str) -> SourceImportSpecifierSyntax {
    let parsed = parse_canonical_phase_2f_rule_for_test(
        source(input),
        rules::SOURCE_IMPORT_SPECIFIER,
        ParseConfig::default(),
    )
    .unwrap();
    assert!(parsed.is_strictly_clean(), "{input:?}");
    SourceImportSpecifierSyntax::cast(
        find_node(&parsed.syntax(), SyntaxKind::SourceImportSpecifier).unwrap(),
    )
    .unwrap()
}

fn declaration(input: &str) -> ImportDeclarationSyntax {
    let parsed = parse_canonical_phase_2f_rule_for_test(
        source(input),
        rules::IMPORT_DECLARATION,
        ParseConfig::default(),
    )
    .unwrap();
    ImportDeclarationSyntax::cast(
        find_node(&parsed.syntax(), SyntaxKind::ImportDeclaration).unwrap(),
    )
    .unwrap()
}

fn lowered_text(value: MechString) -> String {
    value.text.to_string()
}

fn legacy_import(input: &str) -> (ImportDeclaration, usize) {
    let graphemes = graphemes::init_source(input);
    let (remaining, value) = import_declaration(ParseString::new(&graphemes))
        .unwrap_or_else(|error| panic!("legacy parser rejected {input:?}: {error:?}"));
    assert!(remaining.error_log.is_empty(), "{input:?}");
    let remaining_text = remaining.rest();
    let physical_remaining = remaining_text
        .strip_suffix('\n')
        .expect("legacy source sentinel remains unconsumed");
    (value, input.len() - physical_remaining.len())
}

#[test]
fn file_and_uri_specifiers_lower_to_legacy_any_tokens() {
    for (input, expected) in [
        ("./dep.mec", "./dep.mec"),
        ("../lib/dep.mec/*", "../lib/dep.mec/*"),
        ("/lib/dep.mec", "/lib/dep.mec"),
        ("dep.mec", "dep.mec"),
        ("https://example.com/dep.mec", "https://example.com/dep.mec"),
    ] {
        let lowered = lower_legacy_source_import_specifier(&specifier(input)).unwrap();
        assert_eq!(lowered_text(lowered), expected, "{input:?}");
    }
}

#[test]
fn uri_tail_lowering_trims_only_compatibility_text_and_range() {
    for (input, expected) in [
        ("x://path   ", "x://path"),
        ("x://path\t", "x://path"),
        ("x://path\u{00a0}\u{2009}", "x://path"),
        ("x://   ", "x://"),
    ] {
        let lowered = lower_legacy_source_import_specifier(&specifier(input)).unwrap();
        assert_eq!(lowered.text.to_string(), expected, "{input:?}");
    }
}

#[test]
fn import_declaration_lowering_preserves_valid_semantics() {
    for (input, expected) in [
        ("+> dep.mec", "dep.mec"),
        ("+>\u{2009}dep.mec/*", "dep.mec/*"),
        ("+> https://x/dep   ", "https://x/dep"),
    ] {
        let lowered = lower_legacy_import_declaration(&declaration(input)).unwrap();
        assert_eq!(lowered.specifier.to_string(), expected, "{input:?}");
    }
}

#[test]
fn import_declaration_lowering_preserves_all_leading_whitespace_spellings() {
    for input in [
        "\n+> dep.mec",
        "\r+> dep.mec",
        "\r\n+> dep.mec",
        " \n\t+> dep.mec",
    ] {
        let parsed = parse_canonical_phase_2f_rule_for_test(
            source(input),
            rules::IMPORT_DECLARATION,
            ParseConfig::default(),
        )
        .unwrap();
        assert!(parsed.is_strictly_clean(), "{input:?}");
        let (legacy, legacy_consumed) = legacy_import(input);
        assert_eq!(parsed.consumed.end.to_usize(), legacy_consumed, "{input:?}");
        assert_eq!(
            parsed.source.byte_len().to_usize() - parsed.consumed.end.to_usize(),
            input.len() - legacy_consumed,
            "{input:?}"
        );
        let canonical = lower_legacy_import_declaration(
            &ImportDeclarationSyntax::cast(
                find_node(&parsed.syntax(), SyntaxKind::ImportDeclaration).unwrap(),
            )
            .unwrap(),
        )
        .unwrap();
        assert_eq!(canonical, legacy, "{input:?}");
        assert_eq!(canonical.specifier.text.kind, TokenKind::Any, "{input:?}");
        assert_eq!(
            canonical.specifier.text.src_range.start, legacy.specifier.text.src_range.start,
            "{input:?}"
        );
        assert_eq!(
            canonical.specifier.text.src_range.end, legacy.specifier.text.src_range.end,
            "{input:?}"
        );
    }
}
