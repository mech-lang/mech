use mech_syntax::document::ast::{ImportDeclarationSyntax, SourceImportSpecifierSyntax};
use mech_syntax::document::parser::canonical::parse_canonical_phase_2f_rule_for_test;
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    AstNode, DocumentId, ParseConfig, Revision, SyntaxKind, SyntaxNode, TextSnapshot,
};

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

#[test]
fn file_and_uri_specifiers_preserve_exact_canonical_text() {
    for (input, expected) in [
        ("./dep.mec", "./dep.mec"),
        ("../lib/dep.mec/*", "../lib/dep.mec/*"),
        ("/lib/dep.mec", "/lib/dep.mec"),
        ("dep.mec", "dep.mec"),
        ("https://example.com/dep.mec", "https://example.com/dep.mec"),
    ] {
        let selected = specifier(input).selected().unwrap();
        assert_eq!(selected.syntax().text().unwrap(), expected, "{input:?}");
    }
}

#[test]
fn uri_tail_excludes_only_trailing_spacing_from_the_specifier_range() {
    for (input, expected) in [
        ("x://path   ", "x://path"),
        ("x://path\t", "x://path"),
        ("x://path\u{00a0}\u{2009}", "x://path"),
        ("x://   ", "x://"),
    ] {
        let selected = specifier(input).selected().unwrap();
        assert_eq!(selected.syntax().text().unwrap(), expected, "{input:?}");
    }
}

#[test]
fn import_declaration_exposes_the_canonical_specifier() {
    for (input, expected) in [
        ("+> dep.mec", "dep.mec"),
        ("+>\u{2009}dep.mec/*", "dep.mec/*"),
        ("+> https://x/dep   ", "https://x/dep"),
    ] {
        let selected = declaration(input).specifier().unwrap().selected().unwrap();
        assert_eq!(selected.syntax().text().unwrap(), expected, "{input:?}");
    }
}

#[test]
fn import_declaration_preserves_all_leading_whitespace_spellings() {
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
        assert_eq!(parsed.consumed.end.to_usize(), input.len(), "{input:?}");
        let canonical = ImportDeclarationSyntax::cast(
            find_node(&parsed.syntax(), SyntaxKind::ImportDeclaration).unwrap(),
        )
        .unwrap();
        let selected = canonical.specifier().unwrap().selected().unwrap();
        assert_eq!(selected.syntax().text().unwrap(), "dep.mec", "{input:?}");
        assert_eq!(
            selected.syntax().range().end.to_usize(),
            input.len(),
            "{input:?}"
        );
    }
}
