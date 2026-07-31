use mech_syntax::document::parser::canonical::{
    CanonicalRuleOutcome, parse_canonical_phase_2f_rule_for_test,
};
use mech_syntax::document::parser::rules;
use mech_syntax::document::{
    DocumentId, ParseConfig, Revision, RuleId, SyntaxKind, SyntaxNode, TextRange, TextSize,
    TextSnapshot, reconstruct_source_range, validate_lossless_range,
};

fn source(text: &str) -> TextSnapshot {
    TextSnapshot::new(DocumentId(931), Revision(0), text).unwrap()
}

fn parse(
    text: &str,
    rule: RuleId,
) -> mech_syntax::document::parser::canonical::CanonicalSourceRuleSnapshot {
    parse_canonical_phase_2f_rule_for_test(source(text), rule, ParseConfig::default())
        .unwrap_or_else(|| panic!("{rule:?} is not a Phase 2F direct rule"))
}

fn find_node(root: &SyntaxNode, kind: SyntaxKind) -> Option<SyntaxNode> {
    if root.kind() == kind {
        return Some(root.clone());
    }
    root.children().find_map(|child| find_node(&child, kind))
}

fn assert_clean_prefix(rule: RuleId, input: &str, expected: &str, kind: Option<SyntaxKind>) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::Matched, "{input:?}");
    assert!(parsed.is_strictly_clean(), "{rule:?} on {input:?}");
    assert_eq!(parsed.source.text(parsed.consumed).unwrap(), expected);
    assert_eq!(
        reconstruct_source_range(&parsed.root, &parsed.source, parsed.consumed).unwrap(),
        expected,
    );
    validate_lossless_range(&parsed.root, &parsed.source, parsed.consumed).unwrap();
    if let Some(kind) = kind {
        assert!(find_node(&parsed.syntax(), kind).is_some(), "{input:?}");
    }
}

fn assert_no_match(rule: RuleId, input: &str) {
    let parsed = parse(input, rule);
    assert_eq!(parsed.outcome, CanonicalRuleOutcome::NoMatch, "{input:?}");
    assert_eq!(parsed.consumed, TextRange::empty(TextSize::ZERO));
    assert!(parsed.diagnostics.is_empty());
}

#[test]
fn source_import_direct_contracts_are_closed_and_lossless() {
    let cases = [
        (
            rules::SOURCE_IMPORT_TAIL,
            "dep",
            "dep",
            Some(SyntaxKind::SourceImportTail),
        ),
        (rules::SOURCE_PATH_COMPONENT_TOKEN, "a", "a", None),
        (
            rules::SOURCE_PATH_COMPONENT,
            "foo-1_bar.mec",
            "foo-1_bar.mec",
            Some(SyntaxKind::SourcePathComponent),
        ),
        (
            rules::SOURCE_MEC_PATH,
            "path/to/foo.mec",
            "path/to/foo.mec",
            Some(SyntaxKind::SourceMecPath),
        ),
        (rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX, "/*", "/*", None),
        (
            rules::RELATIVE_SOURCE_IMPORT_SPECIFIER,
            "../lib/foo.mec",
            "../lib/foo.mec",
            Some(SyntaxKind::RelativeSourceImportSpecifier),
        ),
        (
            rules::ABSOLUTE_SOURCE_IMPORT_SPECIFIER,
            "/lib/foo.mec",
            "/lib/foo.mec",
            Some(SyntaxKind::AbsoluteSourceImportSpecifier),
        ),
        (
            rules::BARE_SOURCE_IMPORT_SPECIFIER,
            "foo.mec",
            "foo.mec",
            Some(SyntaxKind::BareSourceImportSpecifier),
        ),
        (rules::URI_SCHEME_PART, "+", "+", None),
        (
            rules::SOURCE_IMPORT_URI_SCHEME,
            "git+ssh",
            "git+ssh",
            Some(SyntaxKind::SourceImportUriScheme),
        ),
        (
            rules::URI_SOURCE_IMPORT_SPECIFIER,
            "https://example.com/dep.mec",
            "https://example.com/dep.mec",
            Some(SyntaxKind::UriSourceImportSpecifier),
        ),
        (
            rules::SOURCE_IMPORT_SPECIFIER,
            "foo.mec://bar",
            "foo.mec://bar",
            Some(SyntaxKind::SourceImportSpecifier),
        ),
        (
            rules::IMPORT_DECLARATION,
            "+> dep.mec",
            "+> dep.mec",
            Some(SyntaxKind::ImportDeclaration),
        ),
    ];
    assert_eq!(cases.len(), 13);
    for (rule, input, expected, kind) in cases {
        assert_clean_prefix(rule, input, expected, kind);
    }
}

#[test]
fn mec_paths_are_maximal_lowercase_candidates_without_backtracking() {
    for (input, expected) in [
        (".mec", ".mec"),
        ("foo.mec", "foo.mec"),
        ("foo.mec/", "foo.mec"),
        ("foo.mec/*", "foo.mec"),
        ("foo.mec/bar.mec", "foo.mec/bar.mec"),
    ] {
        assert_clean_prefix(
            rules::SOURCE_MEC_PATH,
            input,
            expected,
            Some(SyntaxKind::SourceMecPath),
        );
    }
    for input in ["foo.MEC", "foo", "foo.mec/bar"] {
        assert_no_match(rules::SOURCE_MEC_PATH, input);
    }
}

#[test]
fn aggregate_uses_the_specified_first_success_order() {
    for (input, kind) in [
        ("./foo.mec", SyntaxKind::RelativeSourceImportSpecifier),
        ("../foo.mec", SyntaxKind::RelativeSourceImportSpecifier),
        ("/foo.mec", SyntaxKind::AbsoluteSourceImportSpecifier),
        ("foo.mec://bar", SyntaxKind::UriSourceImportSpecifier),
        ("foo.mec", SyntaxKind::BareSourceImportSpecifier),
    ] {
        let parsed = parse(input, rules::SOURCE_IMPORT_SPECIFIER);
        assert!(parsed.is_strictly_clean(), "{input:?}");
        assert!(find_node(&parsed.syntax(), kind).is_some(), "{input:?}");
    }
    assert_clean_prefix(
        rules::BARE_SOURCE_IMPORT_SPECIFIER,
        "./foo.mec",
        "./foo.mec",
        Some(SyntaxKind::BareSourceImportSpecifier),
    );
}

#[test]
fn uri_tails_retain_physical_source_and_stop_at_terminators() {
    assert_clean_prefix(
        rules::SOURCE_IMPORT_TAIL,
        "dep;rest",
        "dep",
        Some(SyntaxKind::SourceImportTail),
    );
    assert_clean_prefix(
        rules::SOURCE_IMPORT_TAIL,
        "dep\nrest",
        "dep",
        Some(SyntaxKind::SourceImportTail),
    );
    assert_no_match(rules::SOURCE_IMPORT_TAIL, "\n");
    assert_clean_prefix(
        rules::URI_SOURCE_IMPORT_SPECIFIER,
        "x://   ",
        "x://   ",
        Some(SyntaxKind::UriSourceImportSpecifier),
    );
}

#[test]
fn wildcard_suffix_is_always_an_accepted_optional_prefix() {
    for (input, expected) in [
        ("", ""),
        ("/", ""),
        ("/x", ""),
        ("/*", "/*"),
        ("/*/x", "/*"),
    ] {
        assert_clean_prefix(
            rules::SOURCE_MEC_PATH_WILDCARD_SUFFIX,
            input,
            expected,
            None,
        );
    }
}
